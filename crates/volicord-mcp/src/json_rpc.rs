//! JSON-RPC 2.0 envelope decoding and response construction.
//!
//! This module owns only JSON syntax, envelope shape, request identifiers, and
//! response envelopes. Lifecycle admission and tool execution remain outside
//! this boundary.

use crate::diagnostics::JsonRpcDiagnostic;
use crate::errors::McpAdapterError;
use serde_json::{json, Map, Value};

#[derive(Debug, PartialEq)]
pub(crate) enum ClientMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
}

#[derive(Debug, PartialEq)]
pub(crate) struct JsonRpcRequest {
    pub(crate) id: Value,
    pub(crate) method: String,
    pub(crate) params: Option<Value>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct JsonRpcNotification {
    pub(crate) method: String,
    pub(crate) params: Option<Value>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct JsonRpcFailure {
    pub(crate) id: Value,
    pub(crate) code: i64,
    pub(crate) message: &'static str,
    pub(crate) data: Option<String>,
    pub(crate) diagnostic: JsonRpcDiagnostic,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct JsonSyntaxFailure {
    pub(crate) detail: String,
}

pub(crate) fn decode_json(line: &str) -> Result<Value, JsonSyntaxFailure> {
    serde_json::from_str(line).map_err(|error| JsonSyntaxFailure {
        detail: format!(
            "invalid JSON at line {} column {}",
            error.line(),
            error.column()
        ),
    })
}

pub(crate) fn parse_client_message(message: Value) -> Result<ClientMessage, JsonRpcFailure> {
    let object = match message {
        Value::Object(object) => object,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
            return Err(invalid_request_failure(
                Value::Null,
                "message must be a JSON object",
                JsonRpcDiagnostic::InvalidRequest,
            ));
        }
    };

    let id = match object.get("id") {
        Some(value) => Some(valid_request_id(value)?),
        None => None,
    };
    let response_id = id.clone().unwrap_or(Value::Null);

    match object.get("jsonrpc") {
        Some(Value::String(version)) if version == "2.0" => (),
        _ => {
            return Err(invalid_request_failure(
                response_id,
                "jsonrpc must be exactly \"2.0\"",
                JsonRpcDiagnostic::InvalidRequest,
            ));
        }
    }

    let Some(Value::String(method)) = object.get("method") else {
        return Err(invalid_request_failure(
            response_id,
            "method must be a string",
            JsonRpcDiagnostic::InvalidRequest,
        ));
    };
    let params = object.get("params").cloned();

    if let Some(id) = id {
        Ok(ClientMessage::Request(JsonRpcRequest {
            id,
            method: method.clone(),
            params,
        }))
    } else {
        Ok(ClientMessage::Notification(JsonRpcNotification {
            method: method.clone(),
            params,
        }))
    }
}

pub(crate) fn valid_request_id(value: &Value) -> Result<Value, JsonRpcFailure> {
    match value {
        Value::String(_) => Ok(value.clone()),
        Value::Number(number) if number.is_i64() || number.is_u64() => Ok(value.clone()),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) | Value::Object(_) => {
            Err(invalid_request_failure(
                Value::Null,
                "id must be a string or integer",
                JsonRpcDiagnostic::InvalidId,
            ))
        }
    }
}

pub(crate) fn response_id(message: &Value) -> Option<Value> {
    message
        .as_object()
        .and_then(|object| object.get("id"))
        .cloned()
}

pub(crate) fn notification_params_are_object_or_absent(params: Option<&Value>) -> bool {
    matches!(params, None | Some(Value::Object(_)))
}

pub(crate) fn validate_optional_object_params(
    id: &Value,
    params: Option<Value>,
    method: &str,
) -> Result<(), Value> {
    match params {
        None | Some(Value::Object(_)) => Ok(()),
        Some(_) => Err(invalid_params_response(
            id,
            format!("{method} params must be an object"),
        )),
    }
}

pub(crate) fn required_object_params(
    id: &Value,
    params: Option<Value>,
    method: &str,
) -> Result<Map<String, Value>, Value> {
    match params {
        Some(Value::Object(object)) => Ok(object),
        None | Some(_) => Err(invalid_params_response(
            id,
            format!("{method} params must be an object"),
        )),
    }
}

pub(crate) fn success_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
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
        McpAdapterError::MutationAdmissionAcquisition { .. } => (-32603, "Internal error"),
        McpAdapterError::Core(_)
        | McpAdapterError::Json(_)
        | McpAdapterError::Io(_)
        | McpAdapterError::Store(_) => (-32603, "Internal error"),
    };
    json_rpc_error(id, code, message, Some(error.to_string()))
}

pub(crate) fn invalid_request_failure(
    id: Value,
    data: impl Into<String>,
    diagnostic: JsonRpcDiagnostic,
) -> JsonRpcFailure {
    JsonRpcFailure {
        id,
        code: -32600,
        message: "Invalid Request",
        data: Some(data.into()),
        diagnostic,
    }
}

pub(crate) fn invalid_request_response(id: &Value, data: impl Into<String>) -> Value {
    json_rpc_error(id.clone(), -32600, "Invalid Request", Some(data.into()))
}

pub(crate) fn invalid_params_response(id: &Value, data: impl Into<String>) -> Value {
    json_rpc_error(id.clone(), -32602, "Invalid params", Some(data.into()))
}

pub(crate) fn json_rpc_error(id: Value, code: i64, message: &str, data: Option<String>) -> Value {
    let mut error = json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    });
    if let Some(data) = data {
        error["error"]["data"] = Value::String(data);
    }
    error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_ids_preserve_strings_and_integers() {
        for id in [json!("request.alpha"), json!(0), json!(-7), json!(u64::MAX)] {
            let message = parse_client_message(json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "ping"
            }))
            .expect("valid request");
            let ClientMessage::Request(request) = message else {
                panic!("message with an id must be a request");
            };
            assert_eq!(request.id, id);
            assert_eq!(success_response(request.id.clone(), json!({}))["id"], id);
        }
    }

    #[test]
    fn non_integer_and_structured_request_ids_are_rejected_with_null_id() {
        for id in [json!(null), json!(true), json!(1.5), json!([]), json!({})] {
            let failure = parse_client_message(json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "ping"
            }))
            .expect_err("invalid request id");
            assert_eq!(failure.id, Value::Null);
            assert_eq!(failure.code, -32600);
            assert_eq!(failure.diagnostic, JsonRpcDiagnostic::InvalidId);
        }
    }

    #[test]
    fn notification_has_no_response_identifier() {
        let message = parse_client_message(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }))
        .expect("valid notification");
        assert!(matches!(message, ClientMessage::Notification(_)));
    }
}
