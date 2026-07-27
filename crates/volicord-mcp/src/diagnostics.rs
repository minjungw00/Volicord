use crate::errors::{McpAdapterError, McpHostError};
use serde::Serialize;
use std::collections::BTreeSet;
use std::time::SystemTime;
use volicord_mcp_protocol::ProtocolRegistry;
use volicord_platform_fs::{PlatformDiagnosticClass, PlatformDiagnosticKind};
use volicord_types::diagnostics::{
    DiagnosticAction, DiagnosticCode, DiagnosticDomain, DiagnosticFactSource, DiagnosticFacts,
    DiagnosticFinding, DiagnosticFindingData, DiagnosticFindingId, DiagnosticSeverity,
    DiagnosticSource, DiagnosticStage, DiagnosticSubject,
};
use volicord_types::ids::{AgentConnectionId, AgentRuntimeSessionId};
use volicord_types::integration_revision::IntegrationRevision;
use volicord_types::values::UtcTimestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsonRpcDiagnostic {
    ParseError,
    InvalidRequest,
    InvalidId,
    UnknownMethod,
    MalformedResponse,
    FramingFailure,
    MessageSizeExceeded,
}

impl JsonRpcDiagnostic {
    const ALL: [Self; 7] = [
        Self::ParseError,
        Self::InvalidRequest,
        Self::InvalidId,
        Self::UnknownMethod,
        Self::MalformedResponse,
        Self::FramingFailure,
        Self::MessageSizeExceeded,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpLifecycleDiagnostic {
    InitializeRequired,
    DuplicateInitialize,
    InitializationBatchForbidden,
    InitializedNotificationMissing,
    InitializedNotificationInvalid,
    OperationBeforeReady,
    InvalidShutdownSequence,
}

impl McpLifecycleDiagnostic {
    const ALL: [Self; 7] = [
        Self::InitializeRequired,
        Self::DuplicateInitialize,
        Self::InitializationBatchForbidden,
        Self::InitializedNotificationMissing,
        Self::InitializedNotificationInvalid,
        Self::OperationBeforeReady,
        Self::InvalidShutdownSequence,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpProtocolDiagnostic {
    MalformedVersion,
    UnsupportedVersion,
    CapabilityShapeFailure,
    SchemaProjectionFailure,
}

impl McpProtocolDiagnostic {
    const ALL: [Self; 4] = [
        Self::MalformedVersion,
        Self::UnsupportedVersion,
        Self::CapabilityShapeFailure,
        Self::SchemaProjectionFailure,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum McpToolDiscoveryDiagnostic {
    ProtocolError,
    SchemaFailure,
    RequiredToolMissing,
    InvalidToolDefinitionProjection,
}

impl McpToolDiscoveryDiagnostic {
    const ALL: [Self; 4] = [
        Self::ProtocolError,
        Self::SchemaFailure,
        Self::RequiredToolMissing,
        Self::InvalidToolDefinitionProjection,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum McpToolCallDiagnostic {
    UnknownTool,
    InvalidArguments,
    OutputSchemaFailure,
    ResponseBudgetFailure,
    CoreExecutionError,
    AdapterExecutionError,
    SafeReadOnlyToolFailure,
    SessionCorrelationInvalid,
}

impl McpToolCallDiagnostic {
    const ALL: [Self; 8] = [
        Self::UnknownTool,
        Self::InvalidArguments,
        Self::OutputSchemaFailure,
        Self::ResponseBudgetFailure,
        Self::CoreExecutionError,
        Self::AdapterExecutionError,
        Self::SafeReadOnlyToolFailure,
        Self::SessionCorrelationInvalid,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpTransportDiagnostic {
    IoFailure,
}

impl McpTransportDiagnostic {
    const ALL: [Self; 1] = [Self::IoFailure];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum McpDiagnostic {
    Platform(PlatformDiagnosticKind),
    JsonRpc(JsonRpcDiagnostic),
    Lifecycle(McpLifecycleDiagnostic),
    Protocol(McpProtocolDiagnostic),
    ToolDiscovery(McpToolDiscoveryDiagnostic),
    ToolCall(McpToolCallDiagnostic),
    Host(McpHostError),
    Transport(McpTransportDiagnostic),
    Unexpected,
}

impl From<&McpAdapterError> for McpDiagnostic {
    fn from(error: &McpAdapterError) -> Self {
        match error {
            McpAdapterError::UnknownTool(_) => Self::ToolCall(McpToolCallDiagnostic::UnknownTool),
            McpAdapterError::InvalidParams { .. } => {
                Self::ToolCall(McpToolCallDiagnostic::InvalidArguments)
            }
            McpAdapterError::ToolExecution { .. } => {
                Self::ToolCall(McpToolCallDiagnostic::AdapterExecutionError)
            }
            McpAdapterError::ToolOutputSchema { .. } => {
                Self::ToolCall(McpToolCallDiagnostic::OutputSchemaFailure)
            }
            McpAdapterError::MutationAdmission(_)
            | McpAdapterError::MutationAdmissionAcquisition { .. } => {
                Self::ToolCall(McpToolCallDiagnostic::AdapterExecutionError)
            }
            McpAdapterError::OperationalUnavailable { reached_core, .. } => {
                if *reached_core {
                    Self::ToolCall(McpToolCallDiagnostic::CoreExecutionError)
                } else {
                    Self::ToolCall(McpToolCallDiagnostic::AdapterExecutionError)
                }
            }
            McpAdapterError::Core(_) => Self::ToolCall(McpToolCallDiagnostic::CoreExecutionError),
            McpAdapterError::Store(error) => error.platform_diagnostic().map_or(
                Self::ToolCall(McpToolCallDiagnostic::AdapterExecutionError),
                |diagnostic| Self::Platform(diagnostic.kind()),
            ),
            McpAdapterError::Environment(_) => {
                Self::ToolCall(McpToolCallDiagnostic::AdapterExecutionError)
            }
            McpAdapterError::Io(_) => Self::Transport(McpTransportDiagnostic::IoFailure),
            McpAdapterError::Json(_) => Self::JsonRpc(JsonRpcDiagnostic::MalformedResponse),
            McpAdapterError::Host(error) => Self::Host(*error),
            McpAdapterError::Protocol(_) => {
                Self::Protocol(McpProtocolDiagnostic::SchemaProjectionFailure)
            }
        }
    }
}

impl McpDiagnostic {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Platform(kind) => kind.code(),
            Self::JsonRpc(JsonRpcDiagnostic::ParseError) => "mcp.json_rpc.parse_error",
            Self::JsonRpc(JsonRpcDiagnostic::InvalidRequest) => "mcp.json_rpc.invalid_request",
            Self::JsonRpc(JsonRpcDiagnostic::InvalidId) => "mcp.json_rpc.invalid_id",
            Self::JsonRpc(JsonRpcDiagnostic::UnknownMethod) => "mcp.json_rpc.unknown_method",
            Self::JsonRpc(JsonRpcDiagnostic::MalformedResponse) => {
                "mcp.json_rpc.malformed_response"
            }
            Self::JsonRpc(JsonRpcDiagnostic::FramingFailure) => "mcp.json_rpc.framing_failure",
            Self::JsonRpc(JsonRpcDiagnostic::MessageSizeExceeded) => {
                "mcp.json_rpc.message_size_exceeded"
            }
            Self::Lifecycle(McpLifecycleDiagnostic::InitializeRequired) => {
                "mcp.lifecycle.initialize_required"
            }
            Self::Lifecycle(McpLifecycleDiagnostic::DuplicateInitialize) => {
                "mcp.lifecycle.duplicate_initialize"
            }
            Self::Lifecycle(McpLifecycleDiagnostic::InitializationBatchForbidden) => {
                "mcp.lifecycle.initialization_batch_forbidden"
            }
            Self::Lifecycle(McpLifecycleDiagnostic::InitializedNotificationMissing) => {
                "mcp.lifecycle.initialized_notification_missing"
            }
            Self::Lifecycle(McpLifecycleDiagnostic::InitializedNotificationInvalid) => {
                "mcp.lifecycle.initialized_notification_invalid"
            }
            Self::Lifecycle(McpLifecycleDiagnostic::OperationBeforeReady) => {
                "mcp.lifecycle.operation_before_ready"
            }
            Self::Lifecycle(McpLifecycleDiagnostic::InvalidShutdownSequence) => {
                "mcp.lifecycle.invalid_shutdown_sequence"
            }
            Self::Protocol(McpProtocolDiagnostic::MalformedVersion) => {
                "mcp.protocol.malformed_version"
            }
            Self::Protocol(McpProtocolDiagnostic::UnsupportedVersion) => {
                "mcp.protocol.unsupported_version"
            }
            Self::Protocol(McpProtocolDiagnostic::CapabilityShapeFailure) => {
                "mcp.protocol.capability_shape_invalid"
            }
            Self::Protocol(McpProtocolDiagnostic::SchemaProjectionFailure) => {
                "mcp.protocol.schema_projection_failed"
            }
            Self::ToolDiscovery(McpToolDiscoveryDiagnostic::ProtocolError) => {
                "mcp.tools.protocol_error"
            }
            Self::ToolDiscovery(McpToolDiscoveryDiagnostic::SchemaFailure) => {
                "mcp.tools.schema_failure"
            }
            Self::ToolDiscovery(McpToolDiscoveryDiagnostic::RequiredToolMissing) => {
                "mcp.tools.required_missing"
            }
            Self::ToolDiscovery(McpToolDiscoveryDiagnostic::InvalidToolDefinitionProjection) => {
                "mcp.tools.definition_projection_invalid"
            }
            Self::ToolCall(McpToolCallDiagnostic::UnknownTool) => "mcp.tool_call.unknown_tool",
            Self::ToolCall(McpToolCallDiagnostic::InvalidArguments) => {
                "mcp.tool_call.invalid_arguments"
            }
            Self::ToolCall(McpToolCallDiagnostic::OutputSchemaFailure) => {
                "mcp.tool_call.output_schema_failed"
            }
            Self::ToolCall(McpToolCallDiagnostic::ResponseBudgetFailure) => {
                "mcp.tool_call.response_budget_failed"
            }
            Self::ToolCall(McpToolCallDiagnostic::CoreExecutionError) => {
                "mcp.tool_call.core_execution_failed"
            }
            Self::ToolCall(McpToolCallDiagnostic::AdapterExecutionError) => {
                "mcp.tool_call.adapter_execution_failed"
            }
            Self::ToolCall(McpToolCallDiagnostic::SafeReadOnlyToolFailure) => {
                "mcp.tool_call.safe_read_only_failed"
            }
            Self::ToolCall(McpToolCallDiagnostic::SessionCorrelationInvalid) => {
                "mcp.tool_call.session_correlation_invalid"
            }
            Self::Host(McpHostError::MalformedNativeMetadata) => "host.codex.metadata_malformed",
            Self::Host(McpHostError::SessionThreadTurnInconsistent) => {
                "host.codex.session_thread_turn_inconsistent"
            }
            Self::Host(McpHostError::RegisteredSessionCorrelationMismatch) => {
                "host.codex.registered_session_correlation_mismatch"
            }
            Self::Transport(McpTransportDiagnostic::IoFailure) => "mcp.transport.io_failed",
            Self::Unexpected => volicord_types::diagnostics::INTERNAL_UNEXPECTED_FAILURE_CODE,
        }
    }

    pub(crate) const fn stage(self) -> &'static str {
        match self {
            Self::Platform(_) => "platform_observation",
            Self::JsonRpc(_) | Self::Transport(_) => "transport",
            Self::Lifecycle(McpLifecycleDiagnostic::InitializeRequired)
            | Self::Lifecycle(McpLifecycleDiagnostic::DuplicateInitialize)
            | Self::Protocol(_) => "initialize",
            Self::Lifecycle(McpLifecycleDiagnostic::InitializationBatchForbidden)
            | Self::Lifecycle(McpLifecycleDiagnostic::InitializedNotificationMissing)
            | Self::Lifecycle(McpLifecycleDiagnostic::InitializedNotificationInvalid)
            | Self::Lifecycle(McpLifecycleDiagnostic::OperationBeforeReady)
            | Self::Lifecycle(McpLifecycleDiagnostic::InvalidShutdownSequence) => "lifecycle",
            Self::ToolDiscovery(_) => "tools_list",
            Self::ToolCall(_) | Self::Host(_) => "tool_call",
            Self::Unexpected => "internal",
        }
    }

    pub(crate) const fn severity(self) -> DiagnosticSeverity {
        DiagnosticSeverity::Error
    }

    pub(crate) const fn safe_summary(self) -> &'static str {
        match self {
            Self::Platform(kind) => kind.summary(),
            Self::JsonRpc(JsonRpcDiagnostic::ParseError) => "JSON-RPC input was not valid JSON",
            Self::JsonRpc(JsonRpcDiagnostic::InvalidRequest) => {
                "JSON-RPC request shape was invalid"
            }
            Self::JsonRpc(JsonRpcDiagnostic::InvalidId) => "JSON-RPC request ID was invalid",
            Self::JsonRpc(JsonRpcDiagnostic::UnknownMethod) => "JSON-RPC method was not recognized",
            Self::JsonRpc(JsonRpcDiagnostic::MalformedResponse) => {
                "JSON-RPC response shape was invalid"
            }
            Self::JsonRpc(JsonRpcDiagnostic::FramingFailure) => {
                "JSON-RPC newline framing was invalid"
            }
            Self::JsonRpc(JsonRpcDiagnostic::MessageSizeExceeded) => {
                "JSON-RPC message exceeded its byte limit"
            }
            Self::Lifecycle(McpLifecycleDiagnostic::InitializeRequired) => {
                "initialize was required before the operation"
            }
            Self::Lifecycle(McpLifecycleDiagnostic::DuplicateInitialize) => {
                "initialize was requested more than once"
            }
            Self::Lifecycle(McpLifecycleDiagnostic::InitializationBatchForbidden) => {
                "initialization messages were included in a JSON-RPC batch"
            }
            Self::Lifecycle(McpLifecycleDiagnostic::InitializedNotificationMissing) => {
                "the required initialized notification was not observed"
            }
            Self::Lifecycle(McpLifecycleDiagnostic::InitializedNotificationInvalid) => {
                "the initialized notification was invalid"
            }
            Self::Lifecycle(McpLifecycleDiagnostic::OperationBeforeReady) => {
                "the operation was requested before the MCP session was ready"
            }
            Self::Lifecycle(McpLifecycleDiagnostic::InvalidShutdownSequence) => {
                "the MCP stream ended in an invalid lifecycle state"
            }
            Self::Protocol(McpProtocolDiagnostic::MalformedVersion) => {
                "the requested MCP protocol revision was malformed"
            }
            Self::Protocol(McpProtocolDiagnostic::UnsupportedVersion) => {
                "the requested MCP protocol revision was not production-supported"
            }
            Self::Protocol(McpProtocolDiagnostic::CapabilityShapeFailure) => {
                "initialize capabilities did not have the required shape"
            }
            Self::Protocol(McpProtocolDiagnostic::SchemaProjectionFailure) => {
                "revision-specific schema projection failed"
            }
            Self::ToolDiscovery(McpToolDiscoveryDiagnostic::ProtocolError) => {
                "tools/list returned a protocol error"
            }
            Self::ToolDiscovery(McpToolDiscoveryDiagnostic::SchemaFailure) => {
                "tools/list failed its revision-specific schema"
            }
            Self::ToolDiscovery(McpToolDiscoveryDiagnostic::RequiredToolMissing) => {
                "tools/list omitted a required tool"
            }
            Self::ToolDiscovery(McpToolDiscoveryDiagnostic::InvalidToolDefinitionProjection) => {
                "a projected tool definition was invalid"
            }
            Self::ToolCall(McpToolCallDiagnostic::UnknownTool) => {
                "tools/call named an unknown tool"
            }
            Self::ToolCall(McpToolCallDiagnostic::InvalidArguments) => {
                "tools/call arguments were invalid"
            }
            Self::ToolCall(McpToolCallDiagnostic::OutputSchemaFailure) => {
                "tool output failed its advertised schema"
            }
            Self::ToolCall(McpToolCallDiagnostic::ResponseBudgetFailure) => {
                "tool output exceeded its response budget"
            }
            Self::ToolCall(McpToolCallDiagnostic::CoreExecutionError) => {
                "Core tool execution failed"
            }
            Self::ToolCall(McpToolCallDiagnostic::AdapterExecutionError) => {
                "adapter tool execution failed"
            }
            Self::ToolCall(McpToolCallDiagnostic::SafeReadOnlyToolFailure) => {
                "the designated read-only verification tool call failed"
            }
            Self::ToolCall(McpToolCallDiagnostic::SessionCorrelationInvalid) => {
                "managed tool-call session correlation was invalid"
            }
            Self::Host(McpHostError::MalformedNativeMetadata) => {
                "Codex host-native metadata was malformed"
            }
            Self::Host(McpHostError::SessionThreadTurnInconsistent) => {
                "Codex session, thread, and turn metadata was inconsistent"
            }
            Self::Host(McpHostError::RegisteredSessionCorrelationMismatch) => {
                "Codex metadata did not match the registered session correlation"
            }
            Self::Transport(McpTransportDiagnostic::IoFailure) => {
                "managed stdio transport I/O failed"
            }
            Self::Unexpected => "an unexpected internal MCP failure occurred",
        }
    }

    const fn recommended_action(self) -> (&'static str, &'static str) {
        match self {
            Self::Platform(kind) => match kind.class() {
                PlatformDiagnosticClass::Unsupported => (
                    "action.platform.use_supported_environment",
                    "Use a supported Volicord platform and release target",
                ),
                PlatformDiagnosticClass::Unavailable => (
                    "action.platform.repair_observation_access",
                    "Restore access to the required local platform observations",
                ),
            },
            Self::Protocol(
                McpProtocolDiagnostic::MalformedVersion | McpProtocolDiagnostic::UnsupportedVersion,
            ) => (
                "action.mcp.use_supported_protocol_revision",
                "Configure the MCP peer to request one supported protocol revision",
            ),
            Self::ToolDiscovery(_) => (
                "action.mcp.restore_required_tools",
                "Restore the required tools/list projection for the selected revision",
            ),
            Self::ToolCall(McpToolCallDiagnostic::SafeReadOnlyToolFailure) => (
                "action.mcp.repair_read_only_tool",
                "Repair the designated read-only tool call for the selected revision",
            ),
            Self::Host(_) => (
                "action.host.repair_session_correlation",
                "Repair the managed host session correlation and reconnect",
            ),
            Self::Transport(_) => (
                "action.mcp.repair_stdio_transport",
                "Repair the managed MCP stdio transport and reconnect",
            ),
            _ => (
                "action.mcp.repair_protocol_exchange",
                "Repair the typed MCP protocol failure and reconnect",
            ),
        }
    }
}

/// Returns the current machine-readable diagnostic codes owned by the MCP
/// adapter registry.
pub fn diagnostic_codes() -> BTreeSet<String> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(
        PlatformDiagnosticKind::ALL
            .into_iter()
            .map(McpDiagnostic::Platform),
    );
    diagnostics.extend(
        JsonRpcDiagnostic::ALL
            .into_iter()
            .map(McpDiagnostic::JsonRpc),
    );
    diagnostics.extend(
        McpLifecycleDiagnostic::ALL
            .into_iter()
            .map(McpDiagnostic::Lifecycle),
    );
    diagnostics.extend(
        McpProtocolDiagnostic::ALL
            .into_iter()
            .map(McpDiagnostic::Protocol),
    );
    diagnostics.extend(
        McpToolDiscoveryDiagnostic::ALL
            .into_iter()
            .map(McpDiagnostic::ToolDiscovery),
    );
    diagnostics.extend(
        McpToolCallDiagnostic::ALL
            .into_iter()
            .map(McpDiagnostic::ToolCall),
    );
    diagnostics.extend(McpHostError::ALL.into_iter().map(McpDiagnostic::Host));
    diagnostics.extend(
        McpTransportDiagnostic::ALL
            .into_iter()
            .map(McpDiagnostic::Transport),
    );
    diagnostics.push(McpDiagnostic::Unexpected);
    diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.code().to_owned())
        .collect()
}

#[derive(Debug, Clone)]
pub(crate) struct McpDiagnosticContext {
    pub(crate) observed_at: UtcTimestamp,
    pub(crate) connection_id: Option<String>,
    pub(crate) integration_revision: Option<String>,
    pub(crate) runtime_session_id: Option<String>,
    pub(crate) requested_revision: Option<String>,
    pub(crate) selected_revision: Option<String>,
    pub(crate) negotiated_revision: Option<String>,
    pub(crate) supported_revisions: Vec<String>,
    pub(crate) attempted_client_name: Option<String>,
    pub(crate) attempted_client_version: Option<String>,
    pub(crate) json_rpc_error_code: Option<i64>,
    pub(crate) safe_error_data: Option<String>,
    pub(crate) tool_name: Option<String>,
    pub(crate) missing_tools: Vec<String>,
}

#[derive(Serialize)]
struct McpDiagnosticFacts<'a> {
    summary: &'static str,
    requested_revision: &'a Option<String>,
    selected_revision: &'a Option<String>,
    negotiated_revision: &'a Option<String>,
    production_supported_revisions: &'a [String],
    attempted_client_name: &'a Option<String>,
    attempted_client_version: &'a Option<String>,
    json_rpc_error_code: Option<i64>,
    safe_error_data: &'a Option<String>,
    runtime_session_id: &'a Option<String>,
    tool_name: &'a Option<String>,
    missing_tools: &'a [String],
}

impl DiagnosticFactSource for McpDiagnosticFacts<'_> {}

pub(crate) fn finding_for_diagnostic(
    diagnostic: McpDiagnostic,
    finding_id: impl Into<String>,
    context: McpDiagnosticContext,
) -> Result<DiagnosticFinding, volicord_types::diagnostics::DiagnosticError> {
    let data = data_for_diagnostic(diagnostic, &context)?;
    Ok(data.to_read_projection(
        DiagnosticFindingId::parse(finding_id)?,
        context.runtime_session_id.map(AgentRuntimeSessionId::new),
    ))
}

pub(crate) fn data_for_diagnostic(
    diagnostic: McpDiagnostic,
    context: &McpDiagnosticContext,
) -> Result<DiagnosticFindingData, volicord_types::diagnostics::DiagnosticError> {
    let facts = DiagnosticFacts::project(&McpDiagnosticFacts {
        summary: diagnostic.safe_summary(),
        requested_revision: &context.requested_revision,
        selected_revision: &context.selected_revision,
        negotiated_revision: &context.negotiated_revision,
        production_supported_revisions: &context.supported_revisions,
        attempted_client_name: &context.attempted_client_name,
        attempted_client_version: &context.attempted_client_version,
        json_rpc_error_code: context.json_rpc_error_code,
        safe_error_data: &context.safe_error_data,
        runtime_session_id: &context.runtime_session_id,
        tool_name: &context.tool_name,
        missing_tools: &context.missing_tools,
    })?;
    let subject_reference = context
        .runtime_session_id
        .as_deref()
        .or(context.connection_id.as_deref())
        .unwrap_or("mcp_startup");
    let subject_kind = if context.runtime_session_id.is_some() {
        "runtime_session"
    } else if context.connection_id.is_some() {
        "connection"
    } else {
        "operation"
    };
    let (action_code, action_summary) = diagnostic.recommended_action();
    let mut data = DiagnosticFindingData::try_new(
        DiagnosticCode::parse(diagnostic.code())?,
        DiagnosticDomain::parse(match diagnostic {
            McpDiagnostic::Platform(_) => "platform",
            McpDiagnostic::Host(_) => "host",
            McpDiagnostic::Unexpected => "internal",
            _ => "mcp",
        })?,
        DiagnosticStage::parse(diagnostic.stage())?,
        diagnostic.severity(),
        DiagnosticSource::parse("mcp_stdio")?,
        DiagnosticSubject::try_new(subject_kind, subject_reference)?,
        facts,
        context.observed_at.clone(),
    )?
    .with_actions(vec![DiagnosticAction::try_new(
        DiagnosticCode::parse(action_code)?,
        action_summary,
    )?])?;
    if let Some(connection_id) = &context.connection_id {
        data = data.with_connection_id(AgentConnectionId::new(connection_id.clone()))?;
    }
    if let Some(revision) = &context.integration_revision {
        data = data.with_integration_revision(
            IntegrationRevision::parse(revision.clone())
                .map_err(|_| diagnostic_model_error("invalid integration revision"))?,
        );
    }
    Ok(data)
}

fn diagnostic_model_error(message: &str) -> volicord_types::diagnostics::DiagnosticError {
    DiagnosticCode::parse(message).expect_err("plain text is not a namespaced diagnostic code")
}

pub(crate) fn production_supported_revisions() -> Vec<String> {
    ProtocolRegistry::production()
        .oldest_to_newest()
        .map(|profile| profile.revision().as_str().to_owned())
        .collect()
}

pub fn bootstrap_diagnostic_envelope(error: &McpAdapterError) -> String {
    bootstrap_envelope_for_diagnostic(
        McpDiagnostic::from(error),
        UtcTimestamp::from_datetime(chrono::DateTime::<chrono::Utc>::from(SystemTime::now())),
    )
    .unwrap_or_else(|_| {
        bootstrap_envelope_for_diagnostic(
            McpDiagnostic::Unexpected,
            UtcTimestamp::parse("1970-01-01T00:00:00Z")
                .expect("fixed bootstrap fallback timestamp is canonical"),
        )
        .expect("fixed unexpected MCP bootstrap finding is bounded and valid")
    })
}

fn bootstrap_envelope_for_diagnostic(
    diagnostic: McpDiagnostic,
    observed_at: UtcTimestamp,
) -> Result<String, volicord_types::diagnostics::DiagnosticError> {
    let finding = finding_for_diagnostic(
        diagnostic,
        "finding.mcp.bootstrap",
        McpDiagnosticContext {
            observed_at,
            connection_id: None,
            integration_revision: None,
            runtime_session_id: None,
            requested_revision: None,
            selected_revision: None,
            negotiated_revision: None,
            supported_revisions: production_supported_revisions(),
            attempted_client_name: None,
            attempted_client_version: None,
            json_rpc_error_code: None,
            safe_error_data: None,
            tool_name: None,
            missing_tools: Vec::new(),
        },
    )?;
    volicord_types::diagnostics::format_bootstrap_diagnostic_envelope(&finding)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mcp_mapping_family_has_stable_namespaced_codes() {
        let cases = [
            McpDiagnostic::Platform(PlatformDiagnosticKind::UnsupportedOperatingSystem),
            McpDiagnostic::JsonRpc(JsonRpcDiagnostic::ParseError),
            McpDiagnostic::JsonRpc(JsonRpcDiagnostic::InvalidRequest),
            McpDiagnostic::JsonRpc(JsonRpcDiagnostic::InvalidId),
            McpDiagnostic::JsonRpc(JsonRpcDiagnostic::UnknownMethod),
            McpDiagnostic::JsonRpc(JsonRpcDiagnostic::MalformedResponse),
            McpDiagnostic::JsonRpc(JsonRpcDiagnostic::FramingFailure),
            McpDiagnostic::JsonRpc(JsonRpcDiagnostic::MessageSizeExceeded),
            McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::InitializeRequired),
            McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::DuplicateInitialize),
            McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::InitializationBatchForbidden),
            McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::InitializedNotificationMissing),
            McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::InitializedNotificationInvalid),
            McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::OperationBeforeReady),
            McpDiagnostic::Lifecycle(McpLifecycleDiagnostic::InvalidShutdownSequence),
            McpDiagnostic::Protocol(McpProtocolDiagnostic::MalformedVersion),
            McpDiagnostic::Protocol(McpProtocolDiagnostic::UnsupportedVersion),
            McpDiagnostic::Protocol(McpProtocolDiagnostic::CapabilityShapeFailure),
            McpDiagnostic::Protocol(McpProtocolDiagnostic::SchemaProjectionFailure),
            McpDiagnostic::ToolDiscovery(McpToolDiscoveryDiagnostic::ProtocolError),
            McpDiagnostic::ToolDiscovery(McpToolDiscoveryDiagnostic::SchemaFailure),
            McpDiagnostic::ToolDiscovery(McpToolDiscoveryDiagnostic::RequiredToolMissing),
            McpDiagnostic::ToolDiscovery(
                McpToolDiscoveryDiagnostic::InvalidToolDefinitionProjection,
            ),
            McpDiagnostic::ToolCall(McpToolCallDiagnostic::UnknownTool),
            McpDiagnostic::ToolCall(McpToolCallDiagnostic::InvalidArguments),
            McpDiagnostic::ToolCall(McpToolCallDiagnostic::OutputSchemaFailure),
            McpDiagnostic::ToolCall(McpToolCallDiagnostic::ResponseBudgetFailure),
            McpDiagnostic::ToolCall(McpToolCallDiagnostic::CoreExecutionError),
            McpDiagnostic::ToolCall(McpToolCallDiagnostic::AdapterExecutionError),
            McpDiagnostic::ToolCall(McpToolCallDiagnostic::SafeReadOnlyToolFailure),
            McpDiagnostic::ToolCall(McpToolCallDiagnostic::SessionCorrelationInvalid),
            McpDiagnostic::Host(McpHostError::MalformedNativeMetadata),
            McpDiagnostic::Host(McpHostError::SessionThreadTurnInconsistent),
            McpDiagnostic::Host(McpHostError::RegisteredSessionCorrelationMismatch),
            McpDiagnostic::Transport(McpTransportDiagnostic::IoFailure),
            McpDiagnostic::Unexpected,
        ];
        for diagnostic in cases {
            DiagnosticCode::parse(diagnostic.code()).expect("stable diagnostic code");
        }
    }

    #[test]
    fn codex_2025_06_18_failure_facts_identify_unsupported_version_without_prose() {
        let finding = finding_for_diagnostic(
            McpDiagnostic::Protocol(McpProtocolDiagnostic::UnsupportedVersion),
            "finding.runtime_test.unsupported",
            McpDiagnosticContext {
                observed_at: UtcTimestamp::parse("2026-07-22T01:02:03Z").unwrap(),
                connection_id: Some("connection_test".to_owned()),
                integration_revision: None,
                runtime_session_id: Some("runtime_test".to_owned()),
                requested_revision: Some("2025-06-18".to_owned()),
                selected_revision: Some("2025-11-25".to_owned()),
                negotiated_revision: None,
                supported_revisions: production_supported_revisions(),
                attempted_client_name: Some("codex-mcp-client".to_owned()),
                attempted_client_version: Some("0.108.0".to_owned()),
                json_rpc_error_code: Some(-32601),
                safe_error_data: Some("unsupported requested revision".to_owned()),
                tool_name: None,
                missing_tools: Vec::new(),
            },
        )
        .unwrap();
        let facts = finding.facts().data();
        assert_eq!(finding.code().as_str(), "mcp.protocol.unsupported_version");
        assert_eq!(facts["requested_revision"], "2025-06-18");
        assert_eq!(facts["selected_revision"], "2025-11-25");
        assert!(facts["negotiated_revision"].is_null());
        assert!(facts["production_supported_revisions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "2025-06-18"));
    }

    #[test]
    fn bootstrap_failure_is_one_bounded_shared_finding_envelope() {
        let envelope = bootstrap_diagnostic_envelope(&McpAdapterError::Host(
            McpHostError::MalformedNativeMetadata,
        ));
        assert!(envelope.starts_with("VOLICORD_DIAGNOSTIC_V1 {"));
        assert!(
            envelope.len() <= volicord_types::diagnostics::MAX_BOOTSTRAP_DIAGNOSTIC_ENVELOPE_BYTES
        );
        let finding =
            volicord_types::diagnostics::parse_bootstrap_diagnostic_envelope(&envelope).unwrap();
        assert_eq!(finding.code().as_str(), "host.codex.metadata_malformed");
    }

    #[test]
    fn platform_store_failure_renders_the_same_code_and_typed_action_in_mcp() {
        let error =
            McpAdapterError::Store(volicord_store::StoreError::PlatformEnvironmentUnavailable {
                diagnostic: volicord_platform_fs::PlatformDiagnostic::new(
                    PlatformDiagnosticKind::FilesystemObservationFailure,
                    "filesystem observation failed",
                ),
            });

        let envelope = bootstrap_diagnostic_envelope(&error);
        let finding =
            volicord_types::diagnostics::parse_bootstrap_diagnostic_envelope(&envelope).unwrap();

        assert_eq!(
            finding.code().as_str(),
            "platform.filesystem.observation_failed"
        );
        assert_eq!(
            finding.actions()[0].code().as_str(),
            "action.platform.repair_observation_access"
        );
        assert_eq!(finding.domain().as_str(), "platform");
        assert_eq!(finding.stage().as_str(), "platform_observation");

        let mut value = serde_json::to_value(&finding).expect("finding JSON");
        assert_eq!(
            value
                .as_object()
                .expect("finding object")
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "actions",
                "causes",
                "code",
                "domain",
                "facts",
                "id",
                "observed_at",
                "severity",
                "source",
                "stage",
                "subject",
            ]
        );
        value.as_object_mut().expect("finding object").insert(
            "unexpected_identity".to_owned(),
            serde_json::json!("platform"),
        );
        assert!(
            serde_json::from_value::<DiagnosticFinding>(value).is_err(),
            "the current diagnostic schema must reject unknown fields"
        );
    }
}
