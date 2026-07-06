use crate::prelude::*;
use crate::routing::{
    effective_tool_mode_for_mode_and_storage, McpEffectiveToolMode, McpStorageCapability,
};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct McpToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

pub fn public_method_tools() -> Vec<McpToolDefinition> {
    method_tools(PUBLIC_METHOD_TOOL_NAMES)
}

/// Returns adapter utility tool definitions.
pub fn adapter_utility_tools() -> Vec<McpToolDefinition> {
    ADAPTER_UTILITY_TOOL_NAMES
        .iter()
        .map(|name| McpToolDefinition {
            name,
            description: tool_description(name),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        })
        .collect()
}

/// Returns workflow-mode MCP-visible tools.
pub fn mcp_tools() -> Vec<McpToolDefinition> {
    mcp_tools_for_mode(AgentConnectionMode::Workflow)
}

/// Returns MCP-visible tools for the supplied Agent Connection mode.
pub fn mcp_tools_for_mode(mode: AgentConnectionMode) -> Vec<McpToolDefinition> {
    let mut tools = match mode {
        AgentConnectionMode::ReadOnly => method_tools(READ_ONLY_METHOD_TOOL_NAMES),
        AgentConnectionMode::Workflow => public_method_tools(),
    };
    tools.extend(adapter_utility_tools());
    tools
}

/// Returns MCP-visible tools for the effective connection and storage capability.
pub fn mcp_tools_for_mode_and_storage(
    mode: AgentConnectionMode,
    storage_capability: McpStorageCapability,
) -> Vec<McpToolDefinition> {
    let mut tools = match effective_tool_mode_for_mode_and_storage(mode, storage_capability) {
        McpEffectiveToolMode::Unavailable => Vec::new(),
        McpEffectiveToolMode::ReadOnly | McpEffectiveToolMode::ReadOnlyDegraded => {
            method_tools(READ_ONLY_METHOD_TOOL_NAMES)
        }
        McpEffectiveToolMode::Workflow => public_method_tools(),
    };
    tools.extend(adapter_utility_tools());
    tools
}

pub(crate) fn tools_list_schema_validation_status(tools: &[McpToolDefinition]) -> &'static str {
    if validate_tools_list_schema_compatibility(tools).is_ok() {
        "passed"
    } else {
        "failed"
    }
}

pub(crate) fn mcp_tool_naming_style(tools: &[McpToolDefinition]) -> &'static str {
    if tools.is_empty() {
        return "empty";
    }
    if tools.iter().all(|tool| tool.name.contains('.')) {
        "dotted_namespace"
    } else if tools.iter().all(|tool| !tool.name.contains('.')) {
        "plain"
    } else {
        "mixed"
    }
}

pub(crate) fn validate_tools_list_schema_compatibility(
    tools: &[McpToolDefinition],
) -> Result<(), Vec<String>> {
    let values = tools
        .iter()
        .map(|tool| serde_json::to_value(tool).expect("tool definition should serialize"))
        .collect::<Vec<_>>();
    validate_tools_list_json_compatibility(&values)
}

pub(crate) fn validate_tools_list_json_compatibility(tools: &[Value]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut names = BTreeSet::new();

    for (index, tool) in tools.iter().enumerate() {
        let Some(object) = tool.as_object() else {
            errors.push(format!("tool[{index}] is not an object"));
            continue;
        };

        let Some(name) = object.get("name").and_then(Value::as_str) else {
            errors.push(format!("tool[{index}].name is not a string"));
            continue;
        };
        if !valid_mcp_tool_name(name) {
            errors.push(format!("tool `{name}` has an MCP-incompatible name"));
        }
        if !names.insert(name.to_owned()) {
            errors.push(format!("tool `{name}` is duplicated"));
        }
        if object
            .get("description")
            .is_none_or(|description| description.as_str().is_none_or(|text| text.is_empty()))
        {
            errors.push(format!("tool `{name}` description is missing or empty"));
        }

        let Some(input_schema) = object.get("inputSchema") else {
            errors.push(format!("tool `{name}` is missing inputSchema"));
            continue;
        };
        validate_input_schema(name, input_schema, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub(crate) fn method_tools<const N: usize>(names: [&'static str; N]) -> Vec<McpToolDefinition> {
    names
        .iter()
        .map(|name| McpToolDefinition {
            name,
            description: tool_description(name),
            input_schema: mcp_request_schema(name).expect("MCP tool schema should exist"),
        })
        .collect()
}

pub(crate) fn tool_description(name: &str) -> &'static str {
    match name {
        INTAKE_TOOL_NAME => "Start, resume, supersede, or reject an ordinary user work loop.",
        UPDATE_SCOPE_TOOL_NAME => "Update current Task scope and Change Unit state.",
        STATUS_TOOL_NAME => "Read the current Core status view.",
        PREPARE_WRITE_TOOL_NAME => "Check one proposed product-file write against Core state.",
        STAGE_ARTIFACT_TOOL_NAME => {
            "Prepare an Evidence attachment input; staging alone is not recorded Evidence."
        }
        RECORD_RUN_TOOL_NAME => {
            "Record Evidence for a run, observation, or result, linking attachments when supplied."
        }
        REQUEST_USER_JUDGMENT_TOOL_NAME => "Create one pending focused user-owned judgment.",
        RECONCILE_CHANGES_TOOL_NAME => {
            "Reconcile unresolved unrecorded Product Repository changes."
        }
        CHECK_CLOSE_TOOL_NAME => "Check Close Status for a selected Task.",
        CLOSE_TASK_TOOL_NAME => "Perform a selected Task close path.",
        LIST_PROJECTS_TOOL_NAME => "List projects explicitly allowed for this MCP connection.",
        _ => "Unsupported Volicord method.",
    }
}

fn valid_mcp_tool_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn validate_input_schema(tool_name: &str, schema: &Value, errors: &mut Vec<String>) {
    let Some(object) = schema.as_object() else {
        errors.push(format!("tool `{tool_name}` inputSchema is not an object"));
        return;
    };

    match object.get("type") {
        Some(Value::String(schema_type)) if schema_type == "object" => {}
        Some(_) => errors.push(format!(
            "tool `{tool_name}` inputSchema root type is not object"
        )),
        None => errors.push(format!(
            "tool `{tool_name}` inputSchema root type is missing"
        )),
    }

    validate_schema_fragment(tool_name, "inputSchema", schema, errors);
}

fn validate_schema_fragment(tool_name: &str, path: &str, schema: &Value, errors: &mut Vec<String>) {
    let Some(object) = schema.as_object() else {
        errors.push(format!("tool `{tool_name}` {path} schema is not an object"));
        return;
    };

    for keyword in [
        "patternProperties",
        "unevaluatedProperties",
        "dependentSchemas",
        "$dynamicRef",
        "contains",
    ] {
        if object.contains_key(keyword) {
            errors.push(format!(
                "tool `{tool_name}` {path} uses unsupported schema keyword `{keyword}`"
            ));
        }
    }

    if let Some(schema_uri) = object.get("$schema") {
        if schema_uri.as_str().is_none_or(|uri| uri.is_empty()) {
            errors.push(format!("tool `{tool_name}` {path} has invalid $schema"));
        }
    }
    if let Some(reference) = object.get("$ref") {
        match reference.as_str() {
            Some(value) if value.starts_with("#/") => {}
            _ => errors.push(format!("tool `{tool_name}` {path} uses a non-local $ref")),
        }
    }
    if let Some(schema_type) = object.get("type") {
        validate_schema_type(tool_name, path, schema_type, errors);
    }
    if let Some(enum_values) = object.get("enum") {
        if enum_values
            .as_array()
            .is_none_or(|values| values.is_empty())
        {
            errors.push(format!(
                "tool `{tool_name}` {path} enum is not a non-empty array"
            ));
        }
    }
    if let Some(format) = object.get("format") {
        if format.as_str().is_none_or(|value| value.is_empty()) {
            errors.push(format!("tool `{tool_name}` {path} format is not a string"));
        }
    }
    if let Some(required) = object.get("required") {
        validate_required_fields(tool_name, path, object, required, errors);
    }
    if let Some(properties) = object.get("properties") {
        validate_properties(tool_name, path, properties, errors);
    }
    if let Some(items) = object.get("items") {
        validate_items(tool_name, path, items, errors);
    }
    if let Some(additional_properties) = object.get("additionalProperties") {
        validate_additional_properties(tool_name, path, additional_properties, errors);
    }
    for definitions_key in ["definitions", "$defs"] {
        if let Some(definitions) = object.get(definitions_key) {
            validate_definitions(tool_name, path, definitions_key, definitions, errors);
        }
    }
}

fn validate_schema_type(
    tool_name: &str,
    path: &str,
    schema_type: &Value,
    errors: &mut Vec<String>,
) {
    match schema_type {
        Value::String(value) if valid_json_schema_type(value) => {}
        Value::Array(values)
            if !values.is_empty()
                && values
                    .iter()
                    .all(|value| value.as_str().is_some_and(valid_json_schema_type)) => {}
        _ => errors.push(format!("tool `{tool_name}` {path} has invalid type")),
    }
}

fn valid_json_schema_type(value: &str) -> bool {
    matches!(
        value,
        "null" | "boolean" | "object" | "array" | "number" | "string" | "integer"
    )
}

fn validate_required_fields(
    tool_name: &str,
    path: &str,
    object: &Map<String, Value>,
    required: &Value,
    errors: &mut Vec<String>,
) {
    let Some(required_values) = required.as_array() else {
        errors.push(format!(
            "tool `{tool_name}` {path} required is not an array"
        ));
        return;
    };
    let properties = object.get("properties").and_then(Value::as_object);
    let mut seen = BTreeSet::new();
    for value in required_values {
        let Some(field) = value.as_str() else {
            errors.push(format!(
                "tool `{tool_name}` {path} required contains a non-string value"
            ));
            continue;
        };
        if !seen.insert(field.to_owned()) {
            errors.push(format!(
                "tool `{tool_name}` {path} required duplicates `{field}`"
            ));
        }
        if !properties.is_some_and(|properties| properties.contains_key(field)) {
            errors.push(format!(
                "tool `{tool_name}` {path} requires unknown property `{field}`"
            ));
        }
    }
}

fn validate_properties(tool_name: &str, path: &str, properties: &Value, errors: &mut Vec<String>) {
    let Some(properties) = properties.as_object() else {
        errors.push(format!(
            "tool `{tool_name}` {path} properties is not an object"
        ));
        return;
    };
    for (property_name, property_schema) in properties {
        if property_name.is_empty() {
            errors.push(format!(
                "tool `{tool_name}` {path} has an empty property name"
            ));
        }
        validate_schema_fragment(
            tool_name,
            &format!("{path}.properties.{property_name}"),
            property_schema,
            errors,
        );
    }
}

fn validate_items(tool_name: &str, path: &str, items: &Value, errors: &mut Vec<String>) {
    if let Some(item_schema) = items.as_object() {
        validate_schema_fragment(
            tool_name,
            &format!("{path}.items"),
            &Value::Object(item_schema.clone()),
            errors,
        );
    } else if let Some(item_schemas) = items.as_array() {
        for (index, item_schema) in item_schemas.iter().enumerate() {
            validate_schema_fragment(
                tool_name,
                &format!("{path}.items[{index}]"),
                item_schema,
                errors,
            );
        }
    } else {
        errors.push(format!("tool `{tool_name}` {path} items is not a schema"));
    }
}

fn validate_additional_properties(
    tool_name: &str,
    path: &str,
    additional_properties: &Value,
    errors: &mut Vec<String>,
) {
    if additional_properties.is_boolean() {
        return;
    }
    if additional_properties.is_object() {
        validate_schema_fragment(
            tool_name,
            &format!("{path}.additionalProperties"),
            additional_properties,
            errors,
        );
        return;
    }
    errors.push(format!(
        "tool `{tool_name}` {path} additionalProperties is not boolean or schema"
    ));
}

fn validate_definitions(
    tool_name: &str,
    path: &str,
    definitions_key: &str,
    definitions: &Value,
    errors: &mut Vec<String>,
) {
    let Some(definitions) = definitions.as_object() else {
        errors.push(format!(
            "tool `{tool_name}` {path}.{definitions_key} is not an object"
        ));
        return;
    };
    for (definition_name, definition_schema) in definitions {
        validate_schema_fragment(
            tool_name,
            &format!("{path}.{definitions_key}.{definition_name}"),
            definition_schema,
            errors,
        );
    }
}
