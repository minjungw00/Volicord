use crate::errors::McpAdapterError;
use crate::prelude::*;
use crate::tool_registry::mcp_tool_input_schema;
use std::sync::OnceLock;

const MAX_SCHEMA_DEPTH: usize = 64;

pub(crate) fn validate_mcp_tool_arguments(
    tool_name: &str,
    arguments: &Value,
) -> Result<(), McpAdapterError> {
    let schema = cached_tool_input_schemas()
        .get(tool_name)
        .ok_or_else(|| McpAdapterError::UnknownTool(tool_name.to_owned()))?;
    let mut issues = Vec::new();
    validate_schema_instance(schema, schema, arguments, "", 0, &mut issues);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(McpAdapterError::InvalidParams {
            tool_name: tool_name.to_owned(),
            issues,
            source: None,
        })
    }
}

fn cached_tool_input_schemas() -> &'static HashMap<&'static str, Value> {
    static SCHEMAS: OnceLock<HashMap<&'static str, Value>> = OnceLock::new();
    SCHEMAS.get_or_init(|| {
        PUBLIC_METHOD_TOOL_NAMES
            .iter()
            .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
            .copied()
            .map(|tool_name| {
                (
                    tool_name,
                    mcp_tool_input_schema(tool_name)
                        .expect("known MCP tool input schema should exist"),
                )
            })
            .collect()
    })
}

fn validate_schema_instance(
    root_schema: &Value,
    schema: &Value,
    instance: &Value,
    path: &str,
    depth: usize,
    issues: &mut Vec<McpToolErrorIssue>,
) {
    if depth >= MAX_SCHEMA_DEPTH {
        return;
    }
    let Some(object) = schema.as_object() else {
        return;
    };

    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        if let Some(target) = resolve_local_reference(root_schema, reference) {
            validate_schema_instance(root_schema, target, instance, path, depth + 1, issues);
        }
        return;
    }

    if let Some(branches) = object.get("allOf").and_then(Value::as_array) {
        for branch in branches {
            validate_schema_instance(root_schema, branch, instance, path, depth + 1, issues);
        }
    }
    for keyword in ["anyOf", "oneOf"] {
        if let Some(branches) = object.get(keyword).and_then(Value::as_array) {
            validate_union(root_schema, branches, instance, path, depth + 1, issues);
        }
    }

    if let Some(expected_types) = schema_type_names(object.get("type")) {
        if !expected_types
            .iter()
            .any(|expected| instance_matches_type(instance, expected))
        {
            push_issue(
                issues,
                McpToolErrorIssue {
                    path: path.to_owned(),
                    code: McpToolIssueCode::ArgumentTypeMismatch,
                    message: format!(
                        "Expected {}, but received {}.",
                        expected_types.join(" or "),
                        instance_type_name(instance)
                    ),
                },
            );
            return;
        }
    }

    if let Some(allowed) = object.get("enum").and_then(Value::as_array) {
        if !allowed.contains(instance) {
            let allowed = allowed
                .iter()
                .map(compact_json)
                .collect::<Vec<_>>()
                .join(", ");
            push_issue(
                issues,
                McpToolErrorIssue {
                    path: path.to_owned(),
                    code: McpToolIssueCode::ArgumentEnumValue,
                    message: format!(
                        "Expected one of [{allowed}], but received {}.",
                        compact_json(instance)
                    ),
                },
            );
            return;
        }
    }

    if let Some(instance_object) = instance.as_object() {
        validate_object(
            root_schema,
            object,
            instance_object,
            path,
            depth + 1,
            issues,
        );
    }
    if let Some(instance_array) = instance.as_array() {
        if let Some(item_schema) = object.get("items").filter(|items| items.is_object()) {
            for (index, item) in instance_array.iter().enumerate() {
                validate_schema_instance(
                    root_schema,
                    item_schema,
                    item,
                    &pointer_child(path, &index.to_string()),
                    depth + 1,
                    issues,
                );
            }
        }
    }
}

fn validate_union(
    root_schema: &Value,
    branches: &[Value],
    instance: &Value,
    path: &str,
    depth: usize,
    issues: &mut Vec<McpToolErrorIssue>,
) {
    let mut best: Option<(u8, Vec<McpToolErrorIssue>)> = None;
    for branch in branches {
        let mut branch_issues = Vec::new();
        validate_schema_instance(
            root_schema,
            branch,
            instance,
            path,
            depth,
            &mut branch_issues,
        );
        if branch_issues.is_empty() {
            return;
        }
        let compatibility = schema_instance_compatibility(root_schema, branch, instance, depth);
        if best
            .as_ref()
            .is_none_or(|(current_compatibility, current)| {
                compatibility > *current_compatibility
                    || (compatibility == *current_compatibility
                        && branch_issues.len() < current.len())
            })
        {
            best = Some((compatibility, branch_issues));
        }
    }
    if let Some((_, best)) = best {
        for issue in best {
            push_issue(issues, issue);
        }
    }
}

fn schema_instance_compatibility(
    root_schema: &Value,
    schema: &Value,
    instance: &Value,
    depth: usize,
) -> u8 {
    if depth >= MAX_SCHEMA_DEPTH {
        return 1;
    }
    let Some(object) = schema.as_object() else {
        return 1;
    };
    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        return resolve_local_reference(root_schema, reference).map_or(1, |target| {
            schema_instance_compatibility(root_schema, target, instance, depth + 1)
        });
    }
    if let Some(expected_types) = schema_type_names(object.get("type")) {
        return u8::from(
            expected_types
                .iter()
                .any(|expected| instance_matches_type(instance, expected)),
        ) * 2;
    }
    if let Some(allowed) = object.get("enum").and_then(Value::as_array) {
        return u8::from(allowed.contains(instance)) * 2;
    }
    for keyword in ["anyOf", "oneOf"] {
        if let Some(branches) = object.get(keyword).and_then(Value::as_array) {
            return branches
                .iter()
                .map(|branch| {
                    schema_instance_compatibility(root_schema, branch, instance, depth + 1)
                })
                .max()
                .unwrap_or(1);
        }
    }
    if object.contains_key("properties") || object.contains_key("required") {
        return u8::from(instance.is_object()) * 2;
    }
    if object.contains_key("items") {
        return u8::from(instance.is_array()) * 2;
    }
    1
}

fn validate_object(
    root_schema: &Value,
    schema: &Map<String, Value>,
    instance: &Map<String, Value>,
    path: &str,
    depth: usize,
    issues: &mut Vec<McpToolErrorIssue>,
) {
    let properties = schema.get("properties").and_then(Value::as_object);

    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        for field in required.iter().filter_map(Value::as_str) {
            if !instance.contains_key(field) {
                push_issue(
                    issues,
                    McpToolErrorIssue {
                        path: pointer_child(path, field),
                        code: McpToolIssueCode::ArgumentRequired,
                        message: format!("Required argument `{field}` is missing."),
                    },
                );
            }
        }
    }

    for (field, value) in instance {
        if let Some(property_schema) = properties.and_then(|properties| properties.get(field)) {
            validate_schema_instance(
                root_schema,
                property_schema,
                value,
                &pointer_child(path, field),
                depth,
                issues,
            );
            continue;
        }

        match schema.get("additionalProperties") {
            Some(Value::Bool(false)) => push_issue(
                issues,
                McpToolErrorIssue {
                    path: pointer_child(path, field),
                    code: McpToolIssueCode::ArgumentUnknown,
                    message: format!("Unknown argument `{field}` is not allowed."),
                },
            ),
            Some(additional_schema) if additional_schema.is_object() => validate_schema_instance(
                root_schema,
                additional_schema,
                value,
                &pointer_child(path, field),
                depth,
                issues,
            ),
            _ => {}
        }
    }
}

fn resolve_local_reference<'a>(root_schema: &'a Value, reference: &str) -> Option<&'a Value> {
    reference
        .strip_prefix('#')
        .and_then(|pointer| (!pointer.is_empty()).then_some(pointer))
        .and_then(|pointer| root_schema.pointer(pointer))
}

fn schema_type_names(schema_type: Option<&Value>) -> Option<Vec<&str>> {
    match schema_type? {
        Value::String(value) => Some(vec![value.as_str()]),
        Value::Array(values) => Some(values.iter().filter_map(Value::as_str).collect()),
        _ => None,
    }
}

fn instance_matches_type(instance: &Value, expected: &str) -> bool {
    match expected {
        "null" => instance.is_null(),
        "boolean" => instance.is_boolean(),
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "number" => instance.is_number(),
        "string" => instance.is_string(),
        "integer" => instance
            .as_number()
            .is_some_and(|number| number.is_i64() || number.is_u64()),
        _ => true,
    }
}

fn instance_type_name(instance: &Value) -> &'static str {
    match instance {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(number) if number.is_i64() || number.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn pointer_child(path: &str, segment: &str) -> String {
    let escaped = segment.replace('~', "~0").replace('/', "~1");
    format!("{path}/{escaped}")
}

fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "<unserializable>".to_owned())
}

fn push_issue(issues: &mut Vec<McpToolErrorIssue>, issue: McpToolErrorIssue) {
    if !issues.contains(&issue) {
        issues.push(issue);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_schema_keywords_do_not_reject_supported_values() {
        let schema = json!({
            "type": "object",
            "properties": {
                "value": {
                    "type": "string",
                    "minLength": 10,
                    "pattern": "^required-pattern$"
                }
            },
            "required": ["value"],
            "additionalProperties": false
        });
        let mut issues = Vec::new();

        validate_schema_instance(
            &schema,
            &schema,
            &json!({ "value": "x" }),
            "",
            0,
            &mut issues,
        );

        assert!(issues.is_empty());
    }
}
