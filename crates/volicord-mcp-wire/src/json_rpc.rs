//! JSON-RPC 2.0 MCP envelope decoding and response construction.

use serde_json::{json, Map, Value};

#[derive(Debug, PartialEq)]
pub enum ClientMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
}

#[derive(Debug, PartialEq)]
pub struct JsonRpcRequest {
    pub id: Value,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, PartialEq)]
pub struct JsonRpcNotification {
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonRpcFailureKind {
    InvalidRequest,
    InvalidId,
}

#[derive(Debug, PartialEq)]
pub struct JsonRpcFailure {
    pub id: Value,
    pub code: i64,
    pub message: &'static str,
    pub data: Option<String>,
    pub kind: JsonRpcFailureKind,
}

#[derive(Debug, PartialEq, Eq)]
pub struct JsonSyntaxFailure {
    pub detail: String,
}

pub fn decode_json(line: &str) -> Result<Value, JsonSyntaxFailure> {
    serde_json::from_str(line).map_err(|error| JsonSyntaxFailure {
        detail: format!(
            "invalid JSON at line {} column {}",
            error.line(),
            error.column()
        ),
    })
}

pub fn parse_client_message(message: Value) -> Result<ClientMessage, JsonRpcFailure> {
    let object = match message {
        Value::Object(object) => object,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
            return Err(invalid_request_failure(
                Value::Null,
                "message must be a JSON object",
                JsonRpcFailureKind::InvalidRequest,
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
                JsonRpcFailureKind::InvalidRequest,
            ));
        }
    }

    let Some(Value::String(method)) = object.get("method") else {
        return Err(invalid_request_failure(
            response_id,
            "method must be a string",
            JsonRpcFailureKind::InvalidRequest,
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

pub fn valid_request_id(value: &Value) -> Result<Value, JsonRpcFailure> {
    match value {
        Value::String(_) => Ok(value.clone()),
        Value::Number(number) if number.is_i64() || number.is_u64() => Ok(value.clone()),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) | Value::Object(_) => {
            Err(invalid_request_failure(
                Value::Null,
                "id must be a string or integer",
                JsonRpcFailureKind::InvalidId,
            ))
        }
    }
}

pub fn response_id(message: &Value) -> Option<Value> {
    message
        .as_object()
        .and_then(|object| object.get("id"))
        .cloned()
}

pub fn notification_params_are_object_or_absent(params: Option<&Value>) -> bool {
    matches!(params, None | Some(Value::Object(_)))
}

pub fn validate_optional_object_params(
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

pub fn required_object_params(
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

pub fn success_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

pub fn invalid_request_failure(
    id: Value,
    data: impl Into<String>,
    kind: JsonRpcFailureKind,
) -> JsonRpcFailure {
    JsonRpcFailure {
        id,
        code: -32600,
        message: "Invalid Request",
        data: Some(data.into()),
        kind,
    }
}

pub fn invalid_request_response(id: &Value, data: impl Into<String>) -> Value {
    json_rpc_error(id.clone(), -32600, "Invalid Request", Some(data.into()))
}

pub fn invalid_params_response(id: &Value, data: impl Into<String>) -> Value {
    json_rpc_error(id.clone(), -32602, "Invalid params", Some(data.into()))
}

pub fn json_rpc_error(id: Value, code: i64, message: &str, data: Option<String>) -> Value {
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
