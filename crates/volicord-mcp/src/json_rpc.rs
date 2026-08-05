//! MCP adapter integration for wire-owned JSON-RPC envelopes.

use crate::diagnostics::JsonRpcDiagnostic;
use crate::errors::McpAdapterError;
use serde_json::Value;

pub(crate) use volicord_mcp_wire::json_rpc::{
    decode_json, invalid_params_response, invalid_request_response, json_rpc_error,
    notification_params_are_object_or_absent, parse_client_message, required_object_params,
    response_id, success_response, validate_optional_object_params, ClientMessage, JsonRpcFailure,
    JsonRpcFailureKind, JsonRpcNotification, JsonRpcRequest,
};

pub(crate) fn diagnostic_for_failure(failure: &JsonRpcFailure) -> JsonRpcDiagnostic {
    match failure.kind {
        JsonRpcFailureKind::InvalidRequest => JsonRpcDiagnostic::InvalidRequest,
        JsonRpcFailureKind::InvalidId => JsonRpcDiagnostic::InvalidId,
    }
}

pub(crate) fn json_rpc_error_for_adapter(id: Value, error: McpAdapterError) -> Value {
    let (code, message) = match error {
        McpAdapterError::UnknownTool(_) | McpAdapterError::InvalidParams { .. } => {
            (-32602, "Invalid params")
        }
        McpAdapterError::Protocol(_)
        | McpAdapterError::Environment(_)
        | McpAdapterError::Host(_)
        | McpAdapterError::ToolExecution { .. }
        | McpAdapterError::ToolOutputSchema { .. } => (-32602, "Invalid params"),
        McpAdapterError::MutationAdmission(_) => (-32000, "Runtime Home setup in progress"),
        McpAdapterError::SchemaContractFailure { .. }
        | McpAdapterError::InternalContractInconsistent { .. }
        | McpAdapterError::MutationAdmissionAcquisition { .. } => (-32603, "Internal error"),
        McpAdapterError::OperationalUnavailable { .. }
        | McpAdapterError::Core(_)
        | McpAdapterError::Json(_)
        | McpAdapterError::Io(_)
        | McpAdapterError::Store(_) => (-32603, "Internal error"),
    };
    json_rpc_error(id, code, message, Some(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{McpDiagnostic, McpToolCallDiagnostic};

    #[test]
    fn schema_contract_failure_is_internal_and_records_adapter_diagnostic() {
        let error = McpAdapterError::SchemaContractFailure {
            tool_name: "volicord.status".to_owned(),
        };
        assert!(matches!(
            McpDiagnostic::from(&error),
            McpDiagnostic::ToolCall(McpToolCallDiagnostic::AdapterExecutionError)
        ));

        let response = json_rpc_error_for_adapter(Value::from(7), error);
        assert_eq!(response["error"]["code"], -32603);
        assert_eq!(response["error"]["message"], "Internal error");
    }
}
