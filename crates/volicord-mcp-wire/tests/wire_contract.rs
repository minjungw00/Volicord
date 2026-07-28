use serde_json::{json, Value};
use volicord_mcp_wire::json_rpc::{
    parse_client_message, success_response, ClientMessage, JsonRpcFailureKind,
};
use volicord_mcp_wire::{
    mcp_request_schema, mcp_response_schema, McpMutationEffectSummary,
    McpMutationStructuredContent, McpOperationalErrorCode, McpOperationalFailure,
    McpOperationalOperation, McpOperationalResource, McpReadOnlyToolStructuredContent,
    McpStatusArguments,
};
use volicord_types::methods::{
    public_request_schema, public_response_schema, IntakeResponse, StatusResponse,
};
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::MethodName;

#[test]
fn operational_failure_round_trips_the_exact_wire_shape() {
    let failure = McpOperationalFailure {
        code: McpOperationalErrorCode::Unavailable,
        tool_name: MethodName::Status,
        operation: McpOperationalOperation::StoreAccess,
        resource: McpOperationalResource::ProjectStore,
        retryable: true,
        reached_core: false,
        committed: false,
    };
    let expected = json!({
        "code": "MCP_UNAVAILABLE",
        "tool_name": "volicord.status",
        "operation": "store_access",
        "resource": "project_store",
        "retryable": true,
        "reached_core": false,
        "committed": false
    });

    assert_eq!(serde_json::to_value(&failure).expect("serialize"), expected);
    assert_eq!(
        serde_json::from_value::<McpOperationalFailure>(expected.clone()).expect("deserialize"),
        failure
    );

    let mut unknown = expected;
    unknown["unexpected"] = json!(true);
    assert!(serde_json::from_value::<McpOperationalFailure>(unknown).is_err());
}

#[test]
fn json_rpc_envelopes_preserve_ids_and_reject_non_integer_numbers() {
    for id in [json!("request.alpha"), json!(0), json!(-7), json!(u64::MAX)] {
        let message = parse_client_message(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "ping"
        }))
        .expect("valid request");
        let ClientMessage::Request(request) = message else {
            panic!("message with an id must be a request");
        };
        assert_eq!(request.id, id);
        assert_eq!(success_response(request.id.clone(), json!({}))["id"], id);
    }

    let failure = parse_client_message(json!({
        "jsonrpc": "2.0",
        "id": 1.5,
        "method": "ping"
    }))
    .expect_err("fractional IDs are invalid");
    assert_eq!(failure.id, Value::Null);
    assert_eq!(failure.kind, JsonRpcFailureKind::InvalidId);
}

#[test]
fn wire_owner_generates_mcp_schemas_while_public_schemas_stay_neutral() {
    let request = mcp_request_schema(AgentToolId::STATUS).expect("status MCP request schema");
    let response = mcp_response_schema(AgentToolId::STATUS).expect("status MCP response schema");
    let arguments: McpStatusArguments =
        serde_json::from_value(json!({})).expect("default MCP status arguments");

    assert!(arguments.continuity_page.is_none());
    assert_eq!(request["type"], "object");
    assert_eq!(response["type"], "object");
    let response_text = serde_json::to_string(&response).expect("response schema");
    assert!(response_text.contains("McpOperationalFailure"));
    assert!(response_text.contains("MCP_UNAVAILABLE"));

    for method in MethodName::ALL {
        for public in [
            public_request_schema(method.as_str()).expect("public request schema"),
            public_response_schema(method.as_str()).expect("public response schema"),
        ] {
            let public_text = serde_json::to_string(&public).expect("public schema");
            assert!(
                !public_text.contains("\"Mcp"),
                "{} public schema contains MCP-only structures",
                method.as_str()
            );
            assert!(!public_text.contains("MCP_UNAVAILABLE"));
            assert!(!public_text.contains("structuredContent"));
            assert!(!public_text.contains("\"jsonrpc\""));
        }
    }
}

#[test]
fn mcp_structured_output_rejects_malformed_public_response_branches() {
    let rejection = json!({
        "base": {
            "response_kind": "rejected",
            "effect_kind": "no_effect",
            "dry_run": false,
            "state_version": 7,
            "disclosure": {
                "guarantee_class": "authority_record",
                "guarantees": [],
                "non_guarantees": []
            },
            "events": []
        },
        "errors": [{
            "category": "rejected",
            "code": "VALIDATION_FAILED",
            "message": "request validation failed",
            "retryable": false,
            "details": null
        }]
    });
    let preview = json!({
        "base": {
            "response_kind": "dry_run",
            "effect_kind": "no_effect",
            "dry_run": true,
            "state_version": 7,
            "disclosure": {
                "guarantee_class": "authority_record",
                "guarantees": [],
                "non_guarantees": []
            },
            "events": []
        },
        "dry_run_summary": {
            "planned_effects": [],
            "would_blockers": [],
            "would_errors": [],
            "next_actions": [],
            "diagnostics": []
        }
    });

    type MutationOutput = McpMutationStructuredContent<IntakeResponse, McpMutationEffectSummary>;
    type ReadOnlyOutput = McpReadOnlyToolStructuredContent<StatusResponse>;

    serde_json::from_value::<MutationOutput>(rejection.clone())
        .expect("an exact rejection should decode through MCP structured output");
    serde_json::from_value::<MutationOutput>(preview.clone())
        .expect("an exact preview should decode through MCP structured output");

    let mut malformed = Vec::new();
    for (pointer, value) in [
        ("/base/response_kind", json!("result")),
        ("/base/effect_kind", json!("core_committed")),
    ] {
        let mut value_to_reject = rejection.clone();
        *value_to_reject
            .pointer_mut(pointer)
            .expect("rejection field should exist") = value;
        malformed.push(value_to_reject);
    }
    for (pointer, value) in [
        ("/base/response_kind", json!("rejected")),
        ("/base/effect_kind", json!("read_only")),
        ("/base/dry_run", json!(false)),
    ] {
        let mut value_to_reject = preview.clone();
        *value_to_reject
            .pointer_mut(pointer)
            .expect("preview field should exist") = value;
        malformed.push(value_to_reject);
    }

    let mut unknown_base = rejection.clone();
    unknown_base["base"]["unknown"] = json!(true);
    malformed.push(unknown_base);
    let mut result_rejection_hybrid = rejection.clone();
    result_rejection_hybrid["task_ref"] = Value::Null;
    malformed.push(result_rejection_hybrid);
    let mut preview_rejection_hybrid = rejection.clone();
    preview_rejection_hybrid["dry_run_summary"] = preview["dry_run_summary"].clone();
    malformed.push(preview_rejection_hybrid);

    for value in malformed {
        assert!(
            serde_json::from_value::<MutationOutput>(value.clone()).is_err(),
            "mutation MCP output accepted malformed branch: {value}"
        );
        assert!(
            serde_json::from_value::<ReadOnlyOutput>(value.clone()).is_err(),
            "read-only MCP output accepted malformed branch: {value}"
        );
    }
}
