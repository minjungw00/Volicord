#![forbid(unsafe_code)]

use std::error::Error;

use serde_json::{json, Value};
use volicord_mcp::{mcp_tools_for_mode, McpAdapter, McpConnectionContext};
use volicord_store::{
    agent_connections::{
        add_connection_project, ConnectionProjectRegistration, CONNECTION_MODE_READ_ONLY,
        CONNECTION_MODE_WORKFLOW,
    },
    bootstrap::{register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS},
};
use volicord_test_support::{core_fixtures::CoreFixture, transition_test_connection_mode};
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::{AgentConnectionMode, MethodName};

#[test]
fn workflow_tools_include_agent_workflow_and_read_tools_but_exclude_user_only() {
    let tools = mcp_tools_for_mode(AgentConnectionMode::Workflow);
    let names = tool_names(&tools);
    let expected = AgentToolId::ALL
        .iter()
        .map(|tool| tool.wire_name())
        .collect::<Vec<_>>();

    assert_eq!(names, expected);
    assert!(AgentToolId::ALL.contains(&AgentToolId::RECONCILE_CHANGES));
    assert!(names.contains(&AgentToolId::INTAKE.wire_name()));
    assert!(names.contains(&AgentToolId::PREPARE_WRITE.wire_name()));
    assert!(names.contains(&AgentToolId::REQUEST_USER_ACTION.wire_name()));
    assert!(names.contains(&AgentToolId::RECONCILE_CHANGES.wire_name()));
    assert!(names.contains(&AgentToolId::CLOSE_TASK.wire_name()));
    assert!(!names.contains(&MethodName::ResolveUserAction.as_str()));
}

#[test]
fn read_only_tools_expose_only_read_operations_and_project_discovery() {
    let tools = mcp_tools_for_mode(AgentConnectionMode::ReadOnly);
    let names = tool_names(&tools);
    let expected = AgentToolId::ALL
        .iter()
        .copied()
        .filter(|tool| tool.available_in(AgentConnectionMode::ReadOnly))
        .map(AgentToolId::wire_name)
        .collect::<Vec<_>>();

    assert_eq!(names, expected);
    for mutation_tool in [
        AgentToolId::INTAKE.wire_name(),
        AgentToolId::PREPARE_WRITE.wire_name(),
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
        AgentToolId::RECONCILE_CHANGES.wire_name(),
        AgentToolId::CLOSE_TASK.wire_name(),
    ] {
        assert!(!names.contains(&mutation_tool));
    }
}

#[test]
fn production_adapter_rejects_missing_current_managed_session_before_core(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-integration-session-required")?;
    let adapter = adapter(&fixture)?;
    let before = fixture.counts()?;

    let error = adapter
        .call_tool("volicord.status", json!({}))
        .expect_err("a production adapter must require a current managed Agent Session");

    assert!(error.to_string().contains("agent_session_missing"));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn public_mcp_arguments_reject_internal_envelope_and_invocation_fields(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-integration-internal-args")?;
    let adapter = adapter(&fixture)?;

    for (field, value) in [
        ("envelope", json!({ "project_id": fixture.project_id() })),
        ("project_id", json!(fixture.project_id())),
        ("request_id", json!("req_forged")),
        ("idempotency_key", json!("idem_forged")),
        ("expected_state_version", json!(0)),
        ("dry_run", json!(true)),
        ("locale", json!("en-US")),
        ("actor_source", json!("agent_connection:forged")),
        ("operation_category", json!("agent_workflow")),
        ("mode", json!("workflow")),
        ("connection_id", json!("forged_connection")),
    ] {
        let before = fixture.counts()?;
        let mut args = json!({ "detail": "workflow" });
        args[field] = value;

        let error = match adapter.call_tool("volicord.status", args) {
            Ok(_) => panic!("{field} should be rejected before Core"),
            Err(error) => error,
        };

        assert!(
            error.to_string().contains(field),
            "error for {field} should name the rejected field: {error}"
        );
        assert_eq!(
            fixture.counts()?,
            before,
            "{field} rejection should not create Core storage effects"
        );
    }
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
            AgentToolId::STATUS.wire_name(),
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
            AgentToolId::CHECK_CLOSE.wire_name(),
            AgentToolId::LIST_PROJECTS.wire_name(),
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
            AgentToolId::GUARD_PROBE.wire_name(),
            AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
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
fn user_only_action_resolution_is_not_available_to_agent_mcp() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-integration-user-only")?;
    let adapter = adapter(&fixture)?;

    assert!(!adapter
        .tools()?
        .iter()
        .any(|tool| tool.id.wire_name() == MethodName::ResolveUserAction.as_str()));
    let error = adapter
        .call_tool(MethodName::ResolveUserAction.as_str(), json!({}))
        .expect_err("agent MCP must not expose user-only action resolution");
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
fn explicit_allowed_project_still_requires_current_managed_session() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-integration-explicit-project")?;
    let second_project_id = "project_mcp_second";
    add_project(&fixture, second_project_id, true)?;
    let adapter = adapter(&fixture)?;

    let error = adapter
        .call_tool(
            "volicord.status",
            json!({ "project_selector": second_project_id }),
        )
        .expect_err("an allowed selector must not bypass managed Agent Session authority");

    assert!(error.to_string().contains("agent_session_missing"));
    Ok(())
}

fn adapter(fixture: &CoreFixture) -> Result<McpAdapter, Box<dyn Error>> {
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
    Ok(McpAdapter::new(fixture.runtime_home_path(), context))
}

fn set_connection_mode(fixture: &CoreFixture, mode: &str) -> Result<(), Box<dyn Error>> {
    transition_test_connection_mode(
        fixture.runtime_home_path(),
        &fixture.product_repo_path(),
        fixture.project_id(),
        fixture.connection_id(),
        mode,
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
        &fixture.mutation_context()?,
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
            &fixture.mutation_context()?,
            ConnectionProjectRegistration {
                connection_internal_id: fixture.connection_id().to_owned(),
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
        "acceptance_policy": null,
        "lineage": null,
        "initial_scope": {
            "boundary": "Initial test scope.",
            "non_goals": ["Changing unrelated flows."],
            "acceptance_criteria": [{
                "statement": "The test export flow is represented.",
                "evidence_requirement": "required"
            }]
        },
        "initial_context_refs": []
    });
    if let Some(project_selector) = project_selector {
        args["project_selector"] = json!(project_selector);
    }
    args
}

fn tool_names(tools: &[volicord_mcp::CanonicalToolDefinition]) -> Vec<&'static str> {
    tools
        .iter()
        .map(|tool| tool.id.wire_name())
        .collect::<Vec<_>>()
}

#[test]
fn workflow_mode_constant_stays_available_for_fixture_updates() {
    assert_eq!(CONNECTION_MODE_WORKFLOW, "workflow");
}
