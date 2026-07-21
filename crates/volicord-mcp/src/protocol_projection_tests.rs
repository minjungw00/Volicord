use crate::prelude::*;
use crate::routing::McpStorageCapability;
use crate::schema_validation::validate_mcp_tool_output;
use crate::tool_registry::{
    mcp_tools_for_mode_and_storage_with_detail, CanonicalContent, CanonicalToolResult,
    ToolSchemaDetail,
};
use std::collections::BTreeSet;

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

fn pinned_schema(profile: &McpProtocolProfile) -> Value {
    let source = match profile.revision().as_str() {
        "2024-10-07" => SCHEMA_2024_10_07,
        "2024-11-05" => SCHEMA_2024_11_05,
        "2025-03-26" => SCHEMA_2025_03_26,
        "2025-06-18" => SCHEMA_2025_06_18,
        "2025-11-25" => SCHEMA_2025_11_25,
        revision => panic!("missing pinned production schema for {revision}"),
    };
    serde_json::from_str(source).expect("pinned MCP schema should parse")
}

fn definition<'a>(root: &'a Value, name: &str) -> &'a Value {
    root.get("definitions")
        .or_else(|| root.get("$defs"))
        .and_then(|definitions| definitions.get(name))
        .unwrap_or_else(|| panic!("missing pinned definition {name}"))
}

fn validate_definition(root: &Value, name: &str, instance: &Value) -> Result<(), String> {
    validate_schema(root, definition(root, name), instance, 0)
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

fn canonical_runtime_tools() -> Vec<crate::CanonicalToolDefinition> {
    mcp_tools_for_mode_and_storage_with_detail(
        AgentConnectionMode::Workflow,
        McpStorageCapability::ReadWrite,
        ToolSchemaDetail::RuntimeCompact,
    )
}

#[test]
fn every_production_tools_list_is_a_projection_of_one_canonical_registry() {
    let canonical = canonical_runtime_tools();
    let canonical_names = canonical.iter().map(|tool| tool.name).collect::<Vec<_>>();
    let expected_names = PUBLIC_METHOD_TOOL_NAMES
        .iter()
        .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(canonical_names, expected_names);

    for profile in ProtocolRegistry::production().oldest_to_newest() {
        let schema = pinned_schema(profile);
        let tools = canonical
            .iter()
            .map(|tool| tool.project(profile).into_value())
            .collect::<Vec<_>>();
        let result = json!({ "tools": tools });
        validate_definition(&schema, "ListToolsResult", &result)
            .unwrap_or_else(|error| panic!("{} tools/list: {error}", profile.revision()));

        let permitted = profile
            .schema()
            .tool_definition_fields()
            .iter()
            .map(|field| field.as_str())
            .collect::<BTreeSet<_>>();
        for (canonical_tool, projected) in canonical.iter().zip(
            result["tools"]
                .as_array()
                .expect("projected tools should be an array"),
        ) {
            validate_definition(&schema, "Tool", projected).unwrap_or_else(|error| {
                panic!(
                    "{} tool {}: {error}",
                    profile.revision(),
                    canonical_tool.name
                )
            });
            let fields = projected
                .as_object()
                .expect("projected tool should be an object")
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            assert!(fields.is_subset(&permitted));
            assert_eq!(projected["name"], canonical_tool.name);
            assert!(projected.get("description").is_some());
            assert!(projected.get("inputSchema").is_some());
            assert_eq!(
                projected.get("annotations").is_some(),
                profile.tools().annotations()
            );
            assert_eq!(
                projected.get("outputSchema").is_some(),
                profile.tools().output_schema()
            );
            assert!(projected.get("title").is_none());
            assert!(projected.get("_meta").is_none());
        }
        let projected_names = result["tools"]
            .as_array()
            .expect("projected tools")
            .iter()
            .map(|tool| tool["name"].as_str().expect("projected tool name"))
            .collect::<Vec<_>>();
        assert_eq!(projected_names, canonical_names);
    }
}

#[test]
fn every_production_call_tool_result_uses_its_pinned_wire_shape() {
    let success_body = json!({
        "base": {
            "response_kind": "result",
            "effect_kind": "none",
            "dry_run": false,
            "state_version": 7,
            "events": []
        },
        "value": "ok"
    });
    let error_body = json!({
        "code": "MCP_INVALID_ARGUMENTS",
        "tool_name": STATUS_TOOL_NAME,
        "retryable": true,
        "reached_core": false,
        "committed": false,
        "reported_issue_count": 1,
        "truncated": false,
        "issues": [{
            "path": "/detail",
            "code": "MCP_ARGUMENT_ENUM_VALUE",
            "message": "Unsupported detail value."
        }]
    });

    for profile in ProtocolRegistry::production().oldest_to_newest() {
        for (is_error, body) in [(false, &success_body), (true, &error_body)] {
            let canonical = CanonicalToolResult {
                metadata: None,
                content: vec![CanonicalContent::Text("bounded compatibility".to_owned())],
                structured_content: body.clone(),
                is_error,
            };
            if profile.tools().structured_content() {
                validate_mcp_tool_output(STATUS_TOOL_NAME, body)
                    .expect("structured output should match the advertised runtime schema");
            }
            let projected = canonical
                .project(profile)
                .expect("canonical result should project")
                .into_value();
            let schema = pinned_schema(profile);
            validate_definition(&schema, "CallToolResult", &projected)
                .unwrap_or_else(|error| panic!("{} tools/call: {error}", profile.revision()));

            let permitted = profile
                .schema()
                .tool_result_fields()
                .iter()
                .map(|field| field.as_str())
                .collect::<BTreeSet<_>>();
            let fields = projected
                .as_object()
                .expect("projected result should be an object")
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            assert!(fields.is_subset(&permitted));
            assert_eq!(authoritative_result(&projected), *body);
            assert_eq!(
                projected.get("structuredContent").is_some(),
                profile.tools().structured_content()
            );
            if permitted.contains("isError") {
                assert_eq!(projected["isError"], is_error);
            } else {
                assert!(projected.get("isError").is_none());
            }
        }
    }
}

#[test]
fn bounded_budget_recovery_shape_remains_bounded_for_every_revision() {
    let recovery = CanonicalToolResult {
        metadata: None,
        content: vec![CanonicalContent::Text(
            "r".repeat(crate::stdio::MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES),
        )],
        structured_content: json!({
            "code": "MCP_RESPONSE_BUDGET_EXCEEDED",
            "tool_name": "intake",
            "requested_detail": "full",
            "retryable": false,
            "reached_core": true,
            "committed": true,
            "effect_applied": true,
            "response_projection_omitted": true,
            "status_read_required": true,
            "completion_claim_withheld": true
        }),
        is_error: false,
    };

    for profile in ProtocolRegistry::production().oldest_to_newest() {
        let projected = recovery
            .project(profile)
            .expect("recovery result should project")
            .into_value();
        assert!(
            serde_json::to_vec(&projected)
                .expect("recovery result should serialize")
                .len()
                <= crate::stdio::MAX_MCP_COMPACT_MUTATION_RESULT_BYTES,
            "{} recovery projection exceeded the compact budget",
            profile.revision()
        );
        validate_definition(&pinned_schema(profile), "CallToolResult", &projected)
            .unwrap_or_else(|error| panic!("{} recovery: {error}", profile.revision()));
    }
}

fn authoritative_result(result: &Value) -> Value {
    if let Some(structured) = result.get("structuredContent") {
        return structured.clone();
    }
    if let Some(tool_result) = result.get("toolResult") {
        return tool_result.clone();
    }
    let text = result
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .expect("content-only result should carry authoritative JSON text");
    serde_json::from_str(text).expect("authoritative content text should be JSON")
}
