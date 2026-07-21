#![forbid(unsafe_code)]

use std::{env, ffi::OsStr, fs, path::Path};

use serde_json::{json, Map, Value};
use volicord_mcp::{mcp_tools_for_mode, CanonicalToolDefinition};
use volicord_types::{
    canonical_json_sha256, canonical_json_string, public_request_schema, AgentConnectionMode,
    CHECK_CLOSE_TOOL_NAME, CLOSE_TASK_TOOL_NAME, GET_OPERATION_RESULT_TOOL_NAME, INTAKE_TOOL_NAME,
    PREPARE_EVIDENCE_CAPTURE_TOOL_NAME, PREPARE_WRITE_TOOL_NAME, RECONCILE_CHANGES_TOOL_NAME,
    RECORD_RUN_TOOL_NAME, REQUEST_USER_ACTION_TOOL_NAME, RESOLVE_USER_ACTION_TOOL_NAME,
    STAGE_ARTIFACT_TOOL_NAME, STATUS_TOOL_NAME, UPDATE_SCOPE_TOOL_NAME,
};

const UPDATE_ENV: &str = "VOLICORD_UPDATE_CONTRACT_SNAPSHOTS";
const API_SCHEMA_SNAPSHOT: &str = "snapshots/api_request_schema_contract.json";
const MCP_WORKFLOW_SNAPSHOT: &str = "snapshots/mcp_workflow_tools_contract.json";
const MCP_READ_ONLY_SNAPSHOT: &str = "snapshots/mcp_read_only_tools_contract.json";
const SNAPSHOT_UPDATE_COMMAND: &str = concat!(
    "VOLICORD_UPDATE_CONTRACT_SNAPSHOTS=1 ",
    "cargo test -p volicord-integration-tests --test public_contract_snapshots"
);

const PUBLIC_API_METHOD_NAMES: &[&str] = &[
    INTAKE_TOOL_NAME,
    UPDATE_SCOPE_TOOL_NAME,
    STATUS_TOOL_NAME,
    GET_OPERATION_RESULT_TOOL_NAME,
    CHECK_CLOSE_TOOL_NAME,
    PREPARE_EVIDENCE_CAPTURE_TOOL_NAME,
    PREPARE_WRITE_TOOL_NAME,
    STAGE_ARTIFACT_TOOL_NAME,
    RECORD_RUN_TOOL_NAME,
    REQUEST_USER_ACTION_TOOL_NAME,
    RESOLVE_USER_ACTION_TOOL_NAME,
    RECONCILE_CHANGES_TOOL_NAME,
    CLOSE_TASK_TOOL_NAME,
];

#[test]
fn generated_api_schema_contract_snapshot_matches_sources() {
    assert_snapshot(
        API_SCHEMA_SNAPSHOT,
        api_schema_contract_snapshot(),
        "generated API request schema contract snapshot",
    );
}

#[test]
fn generated_mcp_workflow_tool_contract_snapshot_matches_sources() {
    assert_snapshot(
        MCP_WORKFLOW_SNAPSHOT,
        mcp_tools_contract_snapshot(
            "mcp_workflow_tools",
            AgentConnectionMode::Workflow,
            &[
                "crates/volicord-types/src/ids.rs",
                "crates/volicord-types/src/methods.rs",
                "crates/volicord-types/src/schema.rs",
                "crates/volicord-types/src/tool_names.rs",
                "crates/volicord-types/src/values.rs",
                "crates/volicord-mcp/src/routing.rs",
                "crates/volicord-mcp/src/tool_registry.rs",
            ],
        ),
        "generated workflow MCP tool contract snapshot",
    );
}

#[test]
fn generated_mcp_read_only_tool_contract_snapshot_matches_sources() {
    assert_snapshot(
        MCP_READ_ONLY_SNAPSHOT,
        mcp_tools_contract_snapshot(
            "mcp_read_only_tools",
            AgentConnectionMode::ReadOnly,
            &[
                "crates/volicord-types/src/ids.rs",
                "crates/volicord-types/src/methods.rs",
                "crates/volicord-types/src/schema.rs",
                "crates/volicord-types/src/tool_names.rs",
                "crates/volicord-types/src/values.rs",
                "crates/volicord-mcp/src/routing.rs",
                "crates/volicord-mcp/src/tool_registry.rs",
            ],
        ),
        "generated read-only MCP tool contract snapshot",
    );
}

#[test]
fn snapshot_updates_require_explicit_enable_value() {
    assert!(!snapshot_updates_enabled(None));
    assert!(!snapshot_updates_enabled(Some(OsStr::new(""))));
    assert!(!snapshot_updates_enabled(Some(OsStr::new("0"))));
    assert!(!snapshot_updates_enabled(Some(OsStr::new("true"))));
    assert!(snapshot_updates_enabled(Some(OsStr::new("1"))));
}

fn api_schema_contract_snapshot() -> Value {
    let schemas = PUBLIC_API_METHOD_NAMES
        .iter()
        .map(|method_name| {
            let schema = public_request_schema(method_name)
                .unwrap_or_else(|| panic!("missing public request schema for {method_name}"));
            ((*method_name).to_owned(), schema_projection(&schema))
        })
        .collect::<Map<_, _>>();

    json!({
        "_generated": generated_metadata(
            "api_request_schemas",
            &[
                "crates/volicord-types/src/ids.rs",
                "crates/volicord-types/src/methods.rs",
                "crates/volicord-types/src/schema.rs",
                "crates/volicord-types/src/tool_names.rs",
                "crates/volicord-types/src/values.rs",
            ]
        ),
        "method_order": PUBLIC_API_METHOD_NAMES,
        "schemas": schemas
    })
}

fn mcp_tools_contract_snapshot(
    contract: &str,
    mode: AgentConnectionMode,
    source_paths: &[&str],
) -> Value {
    let tools = mcp_tools_for_mode(mode)
        .iter()
        .map(tool_projection)
        .collect::<Vec<_>>();

    json!({
        "_generated": generated_metadata(contract, source_paths),
        "connection_mode": mode.as_str(),
        "tools": tools
    })
}

fn tool_projection(tool: &CanonicalToolDefinition) -> Value {
    json!({
        "name": tool.name,
        "description": tool.description,
        "input_schema": schema_projection(&tool.input_schema),
        "output_schema": schema_projection(&tool.output_schema),
        "annotations": tool.annotations
    })
}

fn schema_projection(schema: &Value) -> Value {
    json!({
        "sha256": canonical_json_sha256(schema)
            .expect("schema should hash")
            .as_str(),
        "root_required": string_array_field(schema, "required"),
        "root_properties": object_keys(schema.get("properties")),
        "definitions": object_keys(schema.get("definitions"))
    })
}

fn generated_metadata(contract: &str, source_paths: &[&str]) -> Value {
    json!({
        "notice": "Generated public contract snapshot. Do not edit by hand.",
        "contract": contract,
        "source_paths": source_paths,
        "update_command": SNAPSHOT_UPDATE_COMMAND,
        "check_command": "cargo test -p volicord-integration-tests --test public_contract_snapshots"
    })
}

fn string_array_field(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    item.as_str()
                        .unwrap_or_else(|| panic!("{field} entries should be strings"))
                        .to_owned()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn object_keys(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_object)
        .map(|object| object.keys().cloned().collect())
        .unwrap_or_default()
}

fn assert_snapshot(relative_path: &str, actual: Value, label: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    let actual = snapshot_text(&actual);

    if snapshot_updates_enabled(env::var_os(UPDATE_ENV).as_deref()) {
        fs::write(&path, actual)
            .unwrap_or_else(|error| panic!("failed to update {}: {error}", path.display()));
        return;
    }

    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    assert_eq!(
        expected, actual,
        "{label} drifted; regenerate with `{SNAPSHOT_UPDATE_COMMAND}`"
    );
}

fn snapshot_updates_enabled(value: Option<&OsStr>) -> bool {
    value == Some(OsStr::new("1"))
}

fn snapshot_text(value: &Value) -> String {
    let canonical = canonical_json_string(value).expect("snapshot should serialize");
    let parsed: Value = serde_json::from_str(&canonical).expect("canonical snapshot should parse");
    format!(
        "{}\n",
        serde_json::to_string_pretty(&parsed).expect("snapshot should pretty-print")
    )
}
