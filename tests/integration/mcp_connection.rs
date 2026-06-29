#![forbid(unsafe_code)]

use std::error::Error;

use serde_json::{json, Value};
use volicord_core::{CoreService, InvocationContext};
use volicord_mcp::{
    mcp_tools_for_mode, McpAdapter, McpConnectionContext, PUBLIC_METHOD_TOOL_NAMES,
};
use volicord_store::{
    agent_connections::{
        add_connection_project, agent_connection_record, ensure_agent_connection,
        AgentConnectionRegistration, ConnectionProjectRegistration, CONNECTION_MODE_READ_ONLY,
        CONNECTION_MODE_WORKFLOW,
    },
    bootstrap::{register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS},
};
use volicord_test_support::core_fixtures::CoreFixture;
use volicord_types::{
    ActorSource, AgentConnectionMode, OperationCategory, ProjectId,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};

#[test]
fn workflow_tools_include_agent_workflow_and_read_tools_but_exclude_user_only() {
    let tools = mcp_tools_for_mode(AgentConnectionMode::Workflow);
    let names = tool_names(&tools);

    assert_eq!(
        &names[..PUBLIC_METHOD_TOOL_NAMES.len()],
        PUBLIC_METHOD_TOOL_NAMES
    );
    assert!(names.contains(&"volicord.intake"));
    assert!(names.contains(&"volicord.prepare_write"));
    assert!(names.contains(&"volicord.request_user_judgment"));
    assert!(names.contains(&"volicord.check_close"));
    assert!(names.contains(&"volicord.close_task"));
    assert!(names.contains(&"volicord.status"));
    assert!(names.contains(&"volicord.list_projects"));
    assert!(!names.contains(&"volicord.record_user_judgment"));
}

#[test]
fn read_only_tools_expose_only_read_operations_and_project_discovery() {
    let tools = mcp_tools_for_mode(AgentConnectionMode::ReadOnly);
    let names = tool_names(&tools);

    assert_eq!(
        names,
        vec![
            "volicord.status",
            "volicord.check_close",
            "volicord.list_projects"
        ]
    );
}

#[test]
fn connection_invocation_is_injected_and_single_project_is_auto_selected(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-integration-auto-select")?;
    let adapter = adapter(&fixture)?;

    let response = adapter.call_tool("volicord.status", json!({}))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    let verified = response
        .verified_invocation
        .expect("Core should verify injected connection invocation");
    assert_eq!(verified.project_id.as_str(), fixture.project_id());
    assert_eq!(
        verified.actor_source,
        ActorSource::agent_connection(fixture.connection_id())
    );
    assert_eq!(verified.operation_category, OperationCategory::Read);
    assert_eq!(
        verified.verification_basis,
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING
    );
    Ok(())
}

#[test]
fn workflow_mutation_generates_internal_request_metadata() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-integration-generated-metadata")?;
    let adapter = adapter(&fixture)?;
    let before = fixture.counts()?;

    let response = adapter.call_tool("volicord.intake", mcp_intake_args(None))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["state_version"], 1);
    let after = fixture.counts()?;
    assert_eq!(after.tool_invocations, before.tool_invocations + 1);
    Ok(())
}

#[test]
fn read_only_mode_allows_read_close_check_and_rejects_state_changing_close(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-integration-read-only-close")?;
    let task_id = create_task(&fixture, "read_only_close")?;
    set_connection_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;
    let adapter = adapter(&fixture)?;

    let check_response = adapter.call_tool(
        "volicord.check_close",
        json!({ "task_id": task_id.as_str() }),
    )?;
    assert_ne!(
        check_response.response_value["base"]["response_kind"], "rejected",
        "close check should reach Core in read_only mode"
    );
    let verified = check_response
        .verified_invocation
        .expect("close check should verify invocation");
    assert_eq!(verified.operation_category, OperationCategory::Read);

    let before = fixture.counts()?;
    let error = adapter
        .call_tool(
            "volicord.close_task",
            json!({
                "task_id": task_id,
                "intent": "complete",
                "close_reason": "completed_self_checked"
            }),
        )
        .expect_err("read_only should reject state-changing close intent before Core");

    assert!(error.to_string().contains("mode read_only"));
    assert!(error.to_string().contains("agent_workflow"));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn read_only_mode_rejects_agent_workflow_methods_before_core() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-integration-read-only-workflow")?;
    set_connection_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;
    let adapter = adapter(&fixture)?;
    let before = fixture.counts()?;

    let error = adapter
        .call_tool("volicord.intake", mcp_intake_args(None))
        .expect_err("read_only should reject agent workflow tools");

    assert!(error.to_string().contains("mode read_only"));
    assert!(error.to_string().contains("agent_workflow"));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn tool_listing_and_dispatch_use_current_connection_mode() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-integration-dynamic-mode")?;
    let adapter = adapter(&fixture)?;
    set_connection_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;

    let names = tool_names(&adapter.tools()?);
    assert_eq!(
        names,
        vec![
            "volicord.status",
            "volicord.check_close",
            "volicord.list_projects"
        ]
    );
    let error = adapter
        .call_tool("volicord.intake", mcp_intake_args(None))
        .expect_err("dispatch should use the current read_only mode");

    assert!(error.to_string().contains("mode read_only"));
    assert!(error.to_string().contains("agent_workflow"));
    Ok(())
}

#[test]
fn user_only_record_judgment_is_not_available_to_agent_mcp() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-integration-user-only")?;
    let adapter = adapter(&fixture)?;

    assert!(!adapter
        .tools()?
        .iter()
        .any(|tool| tool.name == "volicord.record_user_judgment"));
    let error = adapter
        .call_tool("volicord.record_user_judgment", json!({}))
        .expect_err("agent MCP must not expose user-only judgment recording");
    assert!(error.to_string().contains("unknown MCP tool"));
    Ok(())
}

#[test]
fn multiple_allowed_projects_require_explicit_project_selector() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-integration-ambiguous")?;
    add_project(&fixture, "project_mcp_allowed_b", true)?;
    let adapter = adapter(&fixture)?;

    let error = adapter
        .call_tool("volicord.status", json!({}))
        .expect_err("multiple allowed projects without project_selector should be ambiguous");

    assert!(error.to_string().contains("ambiguous"));
    assert!(error.to_string().contains("project_selector is required"));
    Ok(())
}

#[test]
fn explicit_project_outside_allowlist_is_rejected_before_core() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-integration-outside-allowlist")?;
    let outside_project_id = "project_mcp_outside";
    add_project(&fixture, outside_project_id, false)?;
    let adapter = adapter(&fixture)?;
    let params = mcp_intake_args(Some(outside_project_id));
    let before = fixture.counts()?;

    let error = adapter
        .call_tool("volicord.intake", params)
        .expect_err("out-of-allowlist project should be rejected before Core");

    assert!(error
        .to_string()
        .contains("outside this connection project allowlist"));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn explicit_allowed_project_routes_to_that_project() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-integration-explicit-project")?;
    let second_project_id = "project_mcp_second";
    add_project(&fixture, second_project_id, true)?;
    let adapter = adapter(&fixture)?;

    let response = adapter.call_tool(
        "volicord.status",
        json!({ "project_selector": second_project_id }),
    )?;

    let verified = response
        .verified_invocation
        .expect("explicit allowed project should reach Core");
    assert_eq!(verified.project_id.as_str(), second_project_id);
    assert_eq!(verified.operation_category, OperationCategory::Read);
    Ok(())
}

fn adapter(fixture: &CoreFixture) -> Result<McpAdapter, Box<dyn Error>> {
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
    Ok(McpAdapter::new(fixture.runtime_home_path(), context))
}

fn create_task(fixture: &CoreFixture, suffix: &str) -> Result<String, Box<dyn Error>> {
    let response = CoreService::new(fixture.runtime_home_path()).intake(
        fixture.intake_request(
            &format!("req_mcp_{suffix}_task"),
            &format!("idem_mcp_{suffix}_task"),
            false,
            Some(0),
        ),
        invocation(fixture, OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(response.response_value["base"]["response_kind"], "result");
    Ok(response.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id should be present")
        .to_owned())
}

fn invocation(fixture: &CoreFixture, operation_category: OperationCategory) -> InvocationContext {
    InvocationContext::new(
        ProjectId::new(fixture.project_id()),
        ActorSource::agent_connection(fixture.connection_id()),
        operation_category,
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
    )
}

fn set_connection_mode(fixture: &CoreFixture, mode: &str) -> Result<(), Box<dyn Error>> {
    let existing = agent_connection_record(fixture.runtime_home_path(), fixture.connection_id())?
        .expect("fixture connection should exist");
    ensure_agent_connection(
        fixture.runtime_home_path(),
        AgentConnectionRegistration {
            connection_id: existing.connection_id,
            host_kind: existing.host_kind,
            intent: existing.intent,
            host_scope: existing.host_scope,
            server_name: existing.server_name,
            config_target: existing.config_target,
            mode: mode.to_owned(),
            enabled: existing.enabled,
            managed_fingerprint: existing.managed_fingerprint,
            last_verified_status: existing.last_verified_status,
            last_verification_report_json: existing.last_verification_report_json,
            last_user_actions_json: existing.last_user_actions_json,
            metadata_json: existing.metadata_json,
        },
    )?;
    Ok(())
}

fn add_project(
    fixture: &CoreFixture,
    project_id: &str,
    allow_connection: bool,
) -> Result<(), Box<dyn Error>> {
    let repo_root = fixture.create_product_repo(format!("repo-{project_id}"))?;
    register_project(
        fixture.runtime_home_path(),
        ProjectRegistration {
            project_id: project_id.to_owned(),
            repo_root,
            project_home: None,
            status: ACTIVE_PROJECT_STATUS.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    if allow_connection {
        add_connection_project(
            fixture.runtime_home_path(),
            ConnectionProjectRegistration {
                connection_id: fixture.connection_id().to_owned(),
                project_id: project_id.to_owned(),
            },
        )?;
    }
    Ok(())
}

fn mcp_intake_args(project_selector: Option<&str>) -> Value {
    let mut args = json!({
        "plain_language_request": "Create a test export flow.",
        "requested_mode": "work",
        "resume_policy": "create_new",
        "initial_scope": {
            "boundary": "Initial test scope.",
            "non_goals": ["Changing unrelated flows."],
            "acceptance_criteria": ["The test export flow is represented."]
        },
        "initial_context_refs": []
    });
    if let Some(project_selector) = project_selector {
        args["project_selector"] = json!(project_selector);
    }
    args
}

fn tool_names(tools: &[volicord_mcp::McpToolDefinition]) -> Vec<&'static str> {
    tools.iter().map(|tool| tool.name).collect::<Vec<_>>()
}

#[test]
fn workflow_mode_constant_stays_available_for_fixture_updates() {
    assert_eq!(CONNECTION_MODE_WORKFLOW, "workflow");
}
