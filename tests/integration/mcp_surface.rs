use std::{
    collections::BTreeSet,
    error::Error,
    io::{BufReader, Cursor},
};

use harness_core::{CoreService, InvocationContext};
use harness_mcp::{
    public_method_tools, run_stdio, McpAdapter, McpSessionContext, PUBLIC_METHOD_TOOL_NAMES,
};
use harness_test_support::core_fixtures::CoreFixture;
use harness_types::{AccessClass, SurfaceInstanceId};
use serde_json::{json, Value};

#[test]
fn mcp_exposes_exactly_the_documented_public_methods() {
    let tools = public_method_tools();
    let names = tools.iter().map(|tool| tool.name).collect::<Vec<_>>();
    let unique_names = names.iter().copied().collect::<BTreeSet<_>>();

    assert_eq!(names, PUBLIC_METHOD_TOOL_NAMES);
    assert_eq!(tools.len(), 9);
    assert_eq!(unique_names.len(), 9);
}

#[test]
fn stdio_tools_list_exposes_exactly_the_public_method_set() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_tools")?;
    let adapter = adapter(&fixture, AccessClass::ReadStatus);
    let input = Cursor::new(
        br#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
"#
        .to_vec(),
    );
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let response: Value = serde_json::from_slice(&output)?;
    let names = response["result"]["tools"]
        .as_array()
        .expect("tools should be an array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();
    assert_eq!(names, PUBLIC_METHOD_TOOL_NAMES);
    Ok(())
}

#[test]
fn adapter_uses_session_surface_context_for_artifact_provenance() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_surface")?;
    let core = CoreService::new(fixture.runtime_home_path());
    let intake = core.intake(
        fixture.intake_request("req_mcp_task", "idem_mcp_task", false, Some(0)),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();

    let adapter = adapter(&fixture, AccessClass::ArtifactRegistration);
    let mut params = serde_json::to_value(fixture.stage_artifact_request(
        "req_mcp_stage",
        None,
        false,
        Some(1),
        &task_id,
    ))?;
    params["surface_instance_id"] = json!("forged_surface_instance");
    params["access_class"] = json!("core_mutation");

    let response = adapter.call_tool("harness.stage_artifact", params)?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(
        response.response_value["staged_artifact_handle"]["created_by_surface_id"],
        fixture.surface_id()
    );
    assert_eq!(
        response.response_value["staged_artifact_handle"]["created_by_surface_instance_id"],
        fixture.surface_instance_id()
    );
    assert_eq!(fixture.counts()?.state_version, 1);
    assert_eq!(fixture.counts()?.artifact_staging, 1);
    Ok(())
}

#[test]
fn adapter_does_not_expand_access_class_for_method_calls() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_access")?;
    let adapter = adapter(&fixture, AccessClass::CoreMutation);
    let response = adapter.call_tool(
        "harness.status",
        serde_json::to_value(fixture.status_request("req_status_wrong_access", None))?,
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "CAPABILITY_INSUFFICIENT"
    );
    Ok(())
}

fn adapter(fixture: &CoreFixture, access_class: AccessClass) -> McpAdapter {
    McpAdapter::new(
        fixture.runtime_home_path(),
        McpSessionContext::new(access_class)
            .with_surface_instance_id(SurfaceInstanceId::new(fixture.surface_instance_id()))
            .with_verification_basis("integration_fixture"),
    )
}

fn invocation(fixture: &CoreFixture, access_class: AccessClass) -> InvocationContext {
    InvocationContext {
        surface_instance_id: Some(SurfaceInstanceId::new(fixture.surface_instance_id())),
        access_class,
        verification_basis: "integration_fixture".to_owned(),
    }
}
