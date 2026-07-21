use serde_json::Value;
use volicord_mcp_protocol::McpProtocolRevision;

const SCHEMA_2024_10_07: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/conformance/mcp-spec/2024-10-07/schema.json"
));
const SCHEMA_2024_11_05: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/conformance/mcp-spec/2024-11-05/schema.json"
));
const SCHEMA_2025_03_26: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/conformance/mcp-spec/2025-03-26/schema.json"
));
const SCHEMA_2025_06_18: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/conformance/mcp-spec/2025-06-18/schema.json"
));
const SCHEMA_2025_11_25: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/conformance/mcp-spec/2025-11-25/schema.json"
));

pub(super) fn validate_definition(
    revision: McpProtocolRevision,
    name: &str,
    instance: &Value,
) -> Result<(), String> {
    let root: Value = serde_json::from_str(schema_source(revision))
        .map_err(|error| format!("pinned MCP schema could not be parsed: {error}"))?;
    let definition = root
        .get("definitions")
        .or_else(|| root.get("$defs"))
        .and_then(|definitions| definitions.get(name))
        .ok_or_else(|| format!("pinned MCP schema is missing definition {name}"))?;
    validate_schema(&root, definition, instance, 0)
}

fn schema_source(revision: McpProtocolRevision) -> &'static str {
    match revision.as_str() {
        "2024-10-07" => SCHEMA_2024_10_07,
        "2024-11-05" => SCHEMA_2024_11_05,
        "2025-03-26" => SCHEMA_2025_03_26,
        "2025-06-18" => SCHEMA_2025_06_18,
        "2025-11-25" => SCHEMA_2025_11_25,
        _ => {
            unreachable!("the discover release candidate is not an initialize probe")
        }
    }
}

fn validate_schema(
    root: &Value,
    schema: &Value,
    instance: &Value,
    depth: usize,
) -> Result<(), String> {
    if depth > 128 {
        return Err("pinned schema recursion limit exceeded".to_owned());
    }
    let Some(schema) = schema.as_object() else {
        return Ok(());
    };
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        let target = reference
            .strip_prefix('#')
            .and_then(|pointer| root.pointer(pointer))
            .ok_or_else(|| format!("unresolved pinned schema reference {reference}"))?;
        return validate_schema(root, target, instance, depth + 1);
    }
    if let Some(expected) = schema.get("const") {
        if instance != expected {
            return Err(format!("expected const {expected}, got {instance}"));
        }
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        if !values.contains(instance) {
            return Err(format!("value {instance} is outside pinned enum"));
        }
    }
    if let Some(branches) = schema.get("allOf").and_then(Value::as_array) {
        for branch in branches {
            validate_schema(root, branch, instance, depth + 1)?;
        }
    }
    for keyword in ["anyOf", "oneOf"] {
        if let Some(branches) = schema.get(keyword).and_then(Value::as_array) {
            if !branches
                .iter()
                .any(|branch| validate_schema(root, branch, instance, depth + 1).is_ok())
            {
                return Err(format!("value {instance} matches no {keyword} branch"));
            }
        }
    }
    if let Some(expected_type) = schema.get("type") {
        let matches = match expected_type {
            Value::String(expected) => instance_matches_type(instance, expected),
            Value::Array(expected) => expected
                .iter()
                .filter_map(Value::as_str)
                .any(|expected| instance_matches_type(instance, expected)),
            _ => true,
        };
        if !matches {
            return Err(format!("value {instance} has the wrong pinned type"));
        }
    }
    if let Some(object) = instance.as_object() {
        let properties = schema.get("properties").and_then(Value::as_object);
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for field in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(field) {
                    return Err(format!("required field {field} is absent"));
                }
            }
        }
        for (field, value) in object {
            if let Some(field_schema) = properties.and_then(|properties| properties.get(field)) {
                validate_schema(root, field_schema, value, depth + 1)?;
            } else if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                return Err(format!("unsupported field {field}"));
            } else if let Some(additional) = schema
                .get("additionalProperties")
                .filter(|additional| additional.is_object())
            {
                validate_schema(root, additional, value, depth + 1)?;
            }
        }
    }
    if let Some(array) = instance.as_array() {
        if let Some(minimum) = schema.get("minItems").and_then(Value::as_u64) {
            if array.len() < minimum as usize {
                return Err(format!("array has fewer than {minimum} items"));
            }
        }
        if let Some(items) = schema.get("items") {
            for item in array {
                validate_schema(root, items, item, depth + 1)?;
            }
        }
    }
    Ok(())
}

fn instance_matches_type(instance: &Value, expected: &str) -> bool {
    match expected {
        "null" => instance.is_null(),
        "boolean" => instance.is_boolean(),
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "number" => instance.is_number(),
        "integer" => instance
            .as_number()
            .is_some_and(|number| number.is_i64() || number.is_u64()),
        "string" => instance.is_string(),
        _ => true,
    }
}
