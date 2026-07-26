use serde_json::{json, Map, Value};
use std::{
    any::Any,
    collections::BTreeSet,
    error::Error,
    fs,
    io::{BufReader, Cursor},
    panic::{catch_unwind, AssertUnwindSafe},
    path::PathBuf,
};
use volicord_mcp::{run_stdio, McpAdapter, McpConnectionContext};
use volicord_mcp_protocol::{
    JsonRpcBatching, McpProtocolProfile, McpRevisionStatus, ProtocolRegistry,
};
use volicord_test_support::core_fixtures::CoreFixture;
use volicord_types::tool_names::AgentToolId;
use volicord_types::tool_names::ToolVerificationRole;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[test]
fn every_production_profile_executes_the_wire_conformance_case() {
    let executed = run_production_profile_matrix(|profile| {
        exercise_production_profile(profile).map_err(|error| error.to_string())
    })
    .expect("all production MCP profiles should satisfy wire conformance");

    let expected = ProtocolRegistry::production()
        .oldest_to_newest()
        .map(|profile| profile.revision().as_str())
        .collect::<Vec<_>>();
    assert_eq!(executed, expected);
}

#[test]
fn profile_failure_reports_the_exact_revision() {
    let target = ProtocolRegistry::production()
        .oldest_to_newest()
        .find(|profile| profile.messages().json_rpc_batching() == JsonRpcBatching::Allowed)
        .expect("one production profile should exercise operation batching")
        .revision();

    let error = run_production_profile_matrix(|profile| {
        if profile.revision() == target {
            Err("injected profile-specific behavior failure".to_owned())
        } else {
            Ok(())
        }
    })
    .expect_err("an injected profile failure must fail the matrix");

    assert!(error.contains(target.as_str()), "{error}");
    assert!(error.contains("injected profile-specific behavior failure"));
}

#[test]
fn pre_release_profiles_are_outside_production_iteration() {
    assert!(ProtocolRegistry::production()
        .oldest_to_newest()
        .all(|profile| profile.status() == McpRevisionStatus::Released));
}

fn run_production_profile_matrix(
    mut case: impl FnMut(&'static McpProtocolProfile) -> Result<(), String>,
) -> Result<Vec<&'static str>, String> {
    let mut executed = Vec::new();
    let mut failures = Vec::new();

    for profile in ProtocolRegistry::production().oldest_to_newest() {
        let revision = profile.revision().as_str();
        executed.push(revision);
        match catch_unwind(AssertUnwindSafe(|| case(profile))) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => failures.push(format!("{revision}: {error}")),
            Err(payload) => failures.push(format!("{revision}: {}", panic_message(payload))),
        }
    }

    if failures.is_empty() {
        Ok(executed)
    } else {
        Err(format!(
            "MCP protocol conformance failed for revision(s): {}",
            failures.join("; ")
        ))
    }
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "profile case panicked without a string payload".to_owned())
}

fn exercise_production_profile(profile: &'static McpProtocolProfile) -> TestResult {
    let schema = pinned_schema(profile)?;
    exercise_standalone_initialize_and_eof(profile, &schema)?;
    exercise_lifecycle_tools_and_round_trip(profile, &schema)?;
    exercise_initialization_batch_rejection(profile)?;
    exercise_operation_batch_behavior(profile)?;
    Ok(())
}

fn exercise_standalone_initialize_and_eof(
    profile: &'static McpProtocolProfile,
    schema: &Value,
) -> TestResult {
    let fixture = CoreFixture::new(&format!("protocol-standalone-{}", profile.revision()))?;
    let responses = run_exchange(
        &fixture,
        &[initialize_request(1, profile.revision().as_str())],
    )?;

    assert_eq!(responses.len(), 1, "{}", profile.revision());
    assert_eq!(
        responses[0]["result"]["protocolVersion"],
        profile.revision().as_str()
    );
    assert_eq!(
        responses[0]["result"].get("instructions").is_some(),
        profile.messages().initialize_result_instructions(),
        "{} initialize instructions",
        profile.revision()
    );
    validate_definition(schema, "InitializeResult", &responses[0]["result"])
        .map_err(|error| format!("{} initialize result: {error}", profile.revision()))?;
    Ok(())
}

fn exercise_lifecycle_tools_and_round_trip(
    profile: &'static McpProtocolProfile,
    schema: &Value,
) -> TestResult {
    let fixture = CoreFixture::new(&format!("protocol-tools-{}", profile.revision()))?;
    let verification_tool = ToolVerificationRole::ManagedHostRoundTrip.tool();
    let responses = run_exchange(
        &fixture,
        &[
            tools_call(1, verification_tool.wire_name(), json!({})),
            initialize_request(2, profile.revision().as_str()),
            tools_call(3, verification_tool.wire_name(), json!({})),
            notification("notifications/initialized", json!({})),
            request(4, "tools/list", json!({})),
            tools_call(5, verification_tool.wire_name(), json!({})),
            initialize_request(6, profile.revision().as_str()),
        ],
    )?;

    assert_eq!(responses.len(), 6, "{}", profile.revision());
    assert_eq!(responses[0]["error"]["code"], -32600);
    assert_eq!(
        responses[1]["result"]["protocolVersion"],
        profile.revision().as_str()
    );
    assert_eq!(responses[2]["error"]["code"], -32600);
    assert_eq!(responses[5]["error"]["code"], -32600);

    let tools_result = &responses[3]["result"];
    validate_definition(schema, "ListToolsResult", tools_result)
        .map_err(|error| format!("{} tools/list: {error}", profile.revision()))?;
    let tools = tools_result["tools"]
        .as_array()
        .expect("tools/list should return an array");
    let names = tools
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<BTreeSet<_>>();
    for required in AgentToolId::ALL.map(AgentToolId::wire_name) {
        assert!(
            names.contains(required),
            "{} omitted required tool {required}",
            profile.revision()
        );
    }
    assert!(names.contains(verification_tool.wire_name()));

    for tool in tools {
        validate_definition(schema, "Tool", tool).map_err(|error| {
            format!(
                "{} tool {}: {error}",
                profile.revision(),
                tool["name"].as_str().unwrap_or("<missing>")
            )
        })?;
        let object = tool.as_object().expect("tool definition object");
        assert_eq!(
            object.contains_key("annotations"),
            profile.tools().annotations(),
            "{} annotations projection",
            profile.revision()
        );
        assert_eq!(
            object.contains_key("outputSchema"),
            profile.tools().output_schema(),
            "{} output schema projection",
            profile.revision()
        );
        for absent in ["title", "_meta", "execution", "icons"] {
            assert!(
                !object.contains_key(absent),
                "{} fabricated {absent}",
                profile.revision()
            );
        }
    }

    let call_result = &responses[4]["result"];
    validate_definition(schema, "CallToolResult", call_result)
        .map_err(|error| format!("{} tools/call: {error}", profile.revision()))?;
    assert_eq!(
        call_result.get("structuredContent").is_some(),
        profile.tools().structured_content(),
        "{} structured result projection",
        profile.revision()
    );
    assert_eq!(
        call_result.get("toolResult").is_some(),
        profile
            .schema()
            .tool_result_fields()
            .iter()
            .any(|field| field.as_str() == "toolResult"),
        "{} legacy result projection",
        profile.revision()
    );
    let authoritative = authoritative_tool_result(call_result)?;
    assert!(authoritative["projects"].is_array());
    Ok(())
}

fn exercise_initialization_batch_rejection(profile: &'static McpProtocolProfile) -> TestResult {
    let fixture = CoreFixture::new(&format!("protocol-init-batch-{}", profile.revision()))?;
    let batch = json!([
        initialize_request(1, profile.revision().as_str()),
        notification("notifications/initialized", json!({})),
        request(2, "tools/list", json!({}))
    ]);
    let responses = run_exchange(&fixture, &[batch])?;

    assert_eq!(responses.len(), 1, "{}", profile.revision());
    assert_eq!(responses[0]["error"]["code"], -32600);
    Ok(())
}

fn exercise_operation_batch_behavior(profile: &'static McpProtocolProfile) -> TestResult {
    let fixture = CoreFixture::new(&format!("protocol-op-batch-{}", profile.revision()))?;
    let operation_batch = json!([
        request(2, "ping", json!({})),
        notification(
            "notifications/progress",
            json!({"progressToken": "conformance", "progress": 1})
        ),
        request(3, "tools/list", json!({}))
    ]);
    let responses = run_exchange(
        &fixture,
        &[
            initialize_request(1, profile.revision().as_str()),
            notification("notifications/initialized", json!({})),
            operation_batch,
        ],
    )?;

    assert_eq!(responses.len(), 2, "{}", profile.revision());
    match profile.messages().json_rpc_batching() {
        JsonRpcBatching::Allowed => {
            let batch = responses[1]
                .as_array()
                .expect("batch-enabled profile should return a response batch");
            assert_eq!(batch.len(), 2);
            assert_eq!(batch[0]["id"], 2);
            assert_eq!(batch[0]["result"], json!({}));
            assert_eq!(batch[1]["id"], 3);
            assert!(batch[1]["result"]["tools"].is_array());
        }
        JsonRpcBatching::Disallowed => {
            assert_eq!(responses[1]["error"]["code"], -32600);
        }
    }
    Ok(())
}

fn run_exchange(fixture: &CoreFixture, messages: &[Value]) -> TestResult<Vec<Value>> {
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
    let adapter = McpAdapter::new(fixture.runtime_home_path(), context);
    let mut input = Vec::new();
    for message in messages {
        serde_json::to_writer(&mut input, message)?;
        input.push(b'\n');
    }
    let mut output = Vec::new();
    run_stdio(adapter, BufReader::new(Cursor::new(input)), &mut output)?;

    std::str::from_utf8(&output)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(Into::into))
        .collect()
}

fn initialize_request(id: u64, revision: &str) -> Value {
    request(
        id,
        "initialize",
        json!({
            "protocolVersion": revision,
            "capabilities": {},
            "clientInfo": {
                "name": "volicord-protocol-conformance",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

fn request(id: u64, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
}

fn notification(method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    })
}

fn tools_call(id: u64, name: &str, arguments: Value) -> Value {
    request(
        id,
        "tools/call",
        json!({
            "name": name,
            "arguments": arguments
        }),
    )
}

fn authoritative_tool_result(result: &Value) -> TestResult<Value> {
    if let Some(structured) = result.get("structuredContent") {
        return Ok(structured.clone());
    }
    if let Some(tool_result) = result.get("toolResult") {
        return Ok(tool_result.clone());
    }
    let text = result
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .ok_or("content-only result should carry authoritative JSON text")?;
    Ok(serde_json::from_str(text)?)
}

fn pinned_schema(profile: &'static McpProtocolProfile) -> TestResult<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/mcp-spec")
        .join(profile.revision().as_str())
        .join("schema.json");
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
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
        validate_object(root, schema, object, depth)?;
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

fn validate_object(
    root: &Value,
    schema: &Map<String, Value>,
    object: &Map<String, Value>,
    depth: usize,
) -> Result<(), String> {
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
