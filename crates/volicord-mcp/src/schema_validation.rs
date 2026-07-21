use crate::errors::{bound_mcp_tool_error_issue, McpAdapterError};
use crate::prelude::*;
use crate::tool_registry::{mcp_tool_input_schema, mcp_tool_output_schema};
use std::sync::OnceLock;

const MAX_SCHEMA_DEPTH: usize = 64;

#[derive(Debug, Default)]
struct ValidationIssues {
    issues: Vec<McpToolErrorIssue>,
    truncated: bool,
}

impl ValidationIssues {
    fn can_continue(&mut self) -> bool {
        if self.issues.len() < MAX_VALIDATION_ISSUES {
            true
        } else {
            self.truncated = true;
            false
        }
    }

    fn push(&mut self, issue: McpToolErrorIssue) {
        let (issue, issue_truncated) = bound_mcp_tool_error_issue(issue);
        self.truncated |= issue_truncated;
        if self.issues.contains(&issue) {
            return;
        }
        if self.issues.len() < MAX_VALIDATION_ISSUES {
            self.issues.push(issue);
        } else {
            self.truncated = true;
        }
    }
}

pub(crate) fn validate_mcp_tool_arguments(
    tool_name: &str,
    arguments: &Value,
) -> Result<(), McpAdapterError> {
    let schema = cached_tool_input_schemas()
        .get(tool_name)
        .ok_or_else(|| McpAdapterError::UnknownTool(tool_name.to_owned()))?;
    let mut validation = ValidationIssues::default();
    validate_schema_instance(schema, schema, arguments, "", 0, &mut validation);
    if validation.issues.is_empty() {
        Ok(())
    } else {
        Err(McpAdapterError::InvalidParams {
            tool_name: tool_name.to_owned(),
            issues: validation.issues,
            truncated: validation.truncated,
            source: None,
        })
    }
}

pub(crate) fn validate_mcp_tool_output(
    tool_name: &str,
    output: &Value,
) -> Result<(), McpAdapterError> {
    let schema = mcp_tool_output_schema(tool_name)
        .ok_or_else(|| McpAdapterError::UnknownTool(tool_name.to_owned()))?;
    let mut validation = ValidationIssues::default();
    validate_schema_instance(&schema, &schema, output, "", 0, &mut validation);
    if validation.issues.is_empty() {
        Ok(())
    } else {
        Err(McpAdapterError::ToolOutputSchema {
            tool_name: tool_name.to_owned(),
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
    issues: &mut ValidationIssues,
) {
    if !issues.can_continue() {
        return;
    }
    if depth >= MAX_SCHEMA_DEPTH {
        issues.truncated = true;
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
            if !issues.can_continue() {
                break;
            }
            validate_schema_instance(root_schema, branch, instance, path, depth + 1, issues);
        }
    }
    for keyword in ["anyOf", "oneOf"] {
        if let Some(branches) = object.get(keyword).and_then(Value::as_array) {
            if !issues.can_continue() {
                break;
            }
            validate_union(root_schema, branches, instance, path, depth + 1, issues);
        }
    }

    if let Some(expected_types) = schema_type_names(object.get("type")) {
        if !expected_types
            .iter()
            .any(|expected| instance_matches_type(instance, expected))
        {
            issues.push(McpToolErrorIssue {
                path: path.to_owned(),
                code: McpToolIssueCode::ArgumentTypeMismatch,
                message: format!(
                    "Expected {}, but received {}.",
                    expected_types.join(" or "),
                    instance_type_name(instance)
                ),
            });
            return;
        }
    }

    if let Some(allowed) = object.get("enum").and_then(Value::as_array) {
        if !allowed.contains(instance) {
            let (allowed, allowed_truncated) = compact_enum_values(allowed);
            let (received, received_truncated) = compact_json_preview(instance);
            issues.truncated |= allowed_truncated || received_truncated;
            issues.push(McpToolErrorIssue {
                path: path.to_owned(),
                code: McpToolIssueCode::ArgumentEnumValue,
                message: format!("Expected one of [{allowed}], but received {}.", received),
            });
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
                if !issues.can_continue() {
                    break;
                }
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
    issues: &mut ValidationIssues,
) {
    let mut best: Option<(u8, ValidationIssues)> = None;
    for branch in branches {
        // Each branch receives its own issue budget. Reaching the cap in an
        // invalid branch must not prevent a later valid branch from matching.
        let mut branch_issues = ValidationIssues::default();
        validate_schema_instance(
            root_schema,
            branch,
            instance,
            path,
            depth,
            &mut branch_issues,
        );
        if branch_issues.issues.is_empty() {
            return;
        }
        let compatibility = schema_instance_compatibility(root_schema, branch, instance, depth);
        if best
            .as_ref()
            .is_none_or(|(current_compatibility, current)| {
                compatibility > *current_compatibility
                    || (compatibility == *current_compatibility
                        && branch_issues.issues.len() < current.issues.len())
            })
        {
            best = Some((compatibility, branch_issues));
        }
    }
    if let Some((_, best)) = best {
        issues.truncated |= best.truncated;
        for issue in best.issues {
            if !issues.can_continue() {
                break;
            }
            issues.push(issue);
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
    issues: &mut ValidationIssues,
) {
    let properties = schema.get("properties").and_then(Value::as_object);

    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        for field in required.iter().filter_map(Value::as_str) {
            if !issues.can_continue() {
                break;
            }
            if !instance.contains_key(field) {
                issues.push(McpToolErrorIssue {
                    path: pointer_child(path, field),
                    code: McpToolIssueCode::ArgumentRequired,
                    message: format!("Required argument `{field}` is missing."),
                });
            }
        }
    }

    for (field, value) in instance {
        if !issues.can_continue() {
            break;
        }
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
            Some(Value::Bool(false)) => {
                let (field_preview, field_truncated) = text_preview(field, 128);
                issues.truncated |= field_truncated;
                issues.push(McpToolErrorIssue {
                    path: pointer_child(path, field),
                    code: McpToolIssueCode::ArgumentUnknown,
                    message: format!("Unknown argument `{}` is not allowed.", field_preview),
                })
            }
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

fn compact_enum_values(values: &[Value]) -> (String, bool) {
    let mut preview = String::new();
    let mut truncated = false;
    for value in values {
        let (value, value_truncated) = compact_json_preview(value);
        truncated |= value_truncated;
        let separator = if preview.is_empty() { "" } else { ", " };
        if preview.len() + separator.len() + value.len() > 256 {
            preview.push_str(", ...");
            truncated = true;
            break;
        }
        preview.push_str(separator);
        preview.push_str(&value);
    }
    (preview, truncated)
}

fn compact_json_preview(value: &Value) -> (String, bool) {
    match value {
        Value::String(value) => {
            let (preview, truncated) = text_preview(value, 128);
            (
                serde_json::to_string(&preview)
                    .unwrap_or_else(|_| "\"<unserializable>\"".to_owned()),
                truncated,
            )
        }
        Value::Array(values) => (format!("<array with {} items>", values.len()), true),
        Value::Object(values) => (format!("<object with {} fields>", values.len()), true),
        _ => (
            serde_json::to_string(value).unwrap_or_else(|_| "<unserializable>".to_owned()),
            false,
        ),
    }
}

fn text_preview(value: &str, max_bytes: usize) -> (String, bool) {
    if value.len() <= max_bytes {
        return (value.to_owned(), false);
    }
    let mut end = max_bytes.saturating_sub(3);
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    (format!("{}...", &value[..end]), true)
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
        let mut issues = ValidationIssues::default();

        validate_schema_instance(
            &schema,
            &schema,
            &json!({ "value": "x" }),
            "",
            0,
            &mut issues,
        );

        assert!(issues.issues.is_empty());
        assert!(!issues.truncated);
    }

    #[test]
    fn aggregate_validation_stops_at_the_reported_issue_cap() {
        let schema = json!({
            "type": "array",
            "items": { "type": "string" }
        });
        let instance = Value::Array(
            (0..(MAX_VALIDATION_ISSUES + 20))
                .map(|value| json!(value))
                .collect(),
        );
        let mut issues = ValidationIssues::default();

        validate_schema_instance(&schema, &schema, &instance, "", 0, &mut issues);

        assert_eq!(issues.issues.len(), MAX_VALIDATION_ISSUES);
        assert!(issues.truncated);
        assert_eq!(
            issues.issues.last().map(|issue| issue.path.as_str()),
            Some("/31")
        );
    }

    #[test]
    fn capped_invalid_union_branch_does_not_hide_a_later_valid_branch() {
        let required = (0..(MAX_VALIDATION_ISSUES + 10))
            .map(|index| Value::String(format!("missing_{index}")))
            .collect::<Vec<_>>();
        let schema = json!({
            "anyOf": [
                {
                    "type": "object",
                    "required": required,
                    "additionalProperties": true
                },
                {
                    "type": "object",
                    "properties": {
                        "accepted": { "type": "boolean" }
                    },
                    "required": ["accepted"],
                    "additionalProperties": false
                }
            ]
        });
        let mut issues = ValidationIssues::default();

        validate_schema_instance(
            &schema,
            &schema,
            &json!({ "accepted": true }),
            "",
            0,
            &mut issues,
        );

        assert!(issues.issues.is_empty());
        assert!(!issues.truncated);
    }
}
