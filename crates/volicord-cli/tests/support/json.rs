use std::{collections::BTreeMap, error::Error};

use serde_json::{json, Value};

pub(crate) fn request(id: u64, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
}

pub(crate) fn initialize_request(id: u64) -> Value {
    initialize_request_with_capabilities(id, json!({}))
}

pub(crate) fn initialize_request_with_capabilities(id: u64, capabilities: Value) -> Value {
    request(
        id,
        "initialize",
        json!({
            "protocolVersion": "2025-11-25",
            "capabilities": capabilities,
            "clientInfo": {
                "name": "volicord-binary-test",
                "version": "0.0.0"
            }
        }),
    )
}

pub(crate) fn initialized_notification() -> Value {
    initialized_notification_with_params(json!({}))
}

pub(crate) fn initialized_notification_with_params(params: Value) -> Value {
    notification("notifications/initialized", params)
}

pub(crate) fn notification(method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    })
}

pub(crate) fn tools_call(id: u64, name: &str, arguments: Value) -> Value {
    request(
        id,
        "tools/call",
        json!({
            "name": name,
            "arguments": arguments
        }),
    )
}

pub(crate) fn tools_list_messages(
    initialize_id: u64,
    tools_list_id: u64,
) -> Result<String, serde_json::Error> {
    json_lines(&[
        initialize_request(initialize_id),
        request(tools_list_id, "tools/list", json!({})),
    ])
}

pub(crate) fn json_lines(messages: &[Value]) -> Result<String, serde_json::Error> {
    let mut output = String::new();
    for message in messages {
        output.push_str(&serde_json::to_string(message)?);
        output.push('\n');
    }
    Ok(output)
}

pub(crate) fn json_rpc_values(output: &[u8]) -> Result<Vec<Value>, Box<dyn Error>> {
    let text = std::str::from_utf8(output)?;
    let mut values = Vec::new();
    for (line_number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("invalid JSON on output line {}: {error}", line_number + 1))?;
        assert_eq!(value["jsonrpc"], "2.0");
        values.push(value);
    }
    Ok(values)
}

pub(crate) fn responses_by_id(output: &[u8]) -> Result<BTreeMap<u64, Value>, Box<dyn Error>> {
    let text = std::str::from_utf8(output)?;
    let mut responses = BTreeMap::new();
    for (line_number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("invalid JSON on output line {}: {error}", line_number + 1))?;
        assert_eq!(value["jsonrpc"], "2.0");
        let id = value["id"]
            .as_u64()
            .ok_or_else(|| format!("missing numeric id on output line {}", line_number + 1))?;
        assert!(
            responses.insert(id, value).is_none(),
            "duplicate JSON-RPC response id {id}"
        );
    }
    Ok(responses)
}

pub(crate) fn volicord_response(response: &Value) -> Result<Value, Box<dyn Error>> {
    assert_eq!(response["result"]["isError"], json!(false));
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("tools/call response should contain text content")?;
    let parsed: Value = serde_json::from_str(text)?;
    assert_eq!(response["result"]["structuredContent"], parsed);
    Ok(parsed)
}

pub(crate) fn adapter_tool_response(response: &Value) -> Result<Value, Box<dyn Error>> {
    assert_eq!(response["result"]["isError"], json!(false));
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("adapter tools/call response should contain text content")?;
    let parsed: Value = serde_json::from_str(text)?;
    assert_eq!(response["result"]["structuredContent"], parsed);
    Ok(parsed)
}

pub(crate) fn record_id(value: &Value) -> Result<String, Box<dyn Error>> {
    value["record_id"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "record_id should be present".into())
}
