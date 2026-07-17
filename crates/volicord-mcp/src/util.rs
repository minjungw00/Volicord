use crate::errors::McpAdapterError;
use crate::prelude::*;

pub(crate) fn current_dir_environment_error(error: io::Error) -> McpAdapterError {
    McpAdapterError::Environment(format!("failed to read current directory: {error}"))
}

pub(crate) fn process_env_var(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

pub(crate) fn optional_string_field(
    object: &Map<String, Value>,
    field: &'static str,
    tool_name: &str,
) -> Result<Option<String>, McpAdapterError> {
    match object.get(field) {
        None => Ok(None),
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(Some(value.clone())),
        Some(_) => Err(McpAdapterError::ToolExecution {
            tool_name: tool_name.to_owned(),
            message: format!("{field} must be a non-empty string when supplied"),
        }),
    }
}

pub(crate) fn reject_internal_mcp_argument_fields(
    object: &Map<String, Value>,
    tool_name: &str,
) -> Result<(), McpAdapterError> {
    for field in [
        "envelope",
        "project_id",
        "request_id",
        "idempotency_key",
        "expected_state_version",
        "dry_run",
        "locale",
        "actor_source",
        "operation_category",
        "mode",
        "connection_id",
    ] {
        if object.contains_key(field) {
            return Err(McpAdapterError::ToolExecution {
                tool_name: tool_name.to_owned(),
                message: format!("{field} is supplied by the bound MCP connection and must not be included in MCP tool arguments"),
            });
        }
    }
    Ok(())
}

pub(crate) fn generated_metadata_id(prefix: &str, connection_id: &str, tool_name: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let sequence = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!(
        "{prefix}_{}_{}_{}_{}",
        sanitize_metadata_component(connection_id),
        sanitize_metadata_component(tool_name),
        nanos,
        sequence
    )
}

pub(crate) fn sanitize_metadata_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn validate_identifier_text(
    field: &'static str,
    value: &str,
) -> Result<(), McpAdapterError> {
    if value.trim().is_empty() {
        return Err(McpAdapterError::Environment(format!(
            "{field} must not be empty"
        )));
    }
    if value.contains('\0') {
        return Err(McpAdapterError::Environment(format!(
            "{field} must not contain NUL bytes"
        )));
    }
    Ok(())
}
