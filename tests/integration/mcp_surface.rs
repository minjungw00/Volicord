use std::{
    collections::BTreeSet,
    error::Error,
    fs,
    io::{BufReader, Cursor},
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use serde_json::{json, Value};
use volicord_core::{AdapterSessionBinding, CoreService, InvocationContext};
use volicord_mcp::{
    public_method_tools, run_stdio, McpAdapter, McpIntegrationContext, ADAPTER_UTILITY_TOOL_NAMES,
    PUBLIC_METHOD_TOOL_NAMES,
};
use volicord_store::{
    agent_integrations::{
        add_integration_project, register_agent_integration, AgentIntegrationRegistration,
        IntegrationProjectRegistration,
    },
    bootstrap::{
        list_projects, register_project, register_surface, ProjectRegistration,
        SurfaceRegistration, ACTIVE_PROJECT_STATUS,
    },
    core_pipeline::{CoreProjectStore, StorageEffectCounts},
    sqlite::registry_db_path,
};
use volicord_test_support::core_fixtures::{
    answer_payload, artifact_input_for_handle, supported_evidence_update, CloseTaskFixture,
    CoreFixture, RecordJudgmentFixture, UpdateScopeFixture, UserJudgmentFixture,
    DEFAULT_PRODUCT_PATH,
};
use volicord_types::{
    AccessClass, ChangeUnitOperation, CloseAssessmentInput, CloseIntent, CloseReason, JudgmentKind,
    ProjectId, ResidualRiskInput, StagedArtifactHandle, StatusInclude, SurfaceId,
    SurfaceInstanceId, SurfaceInteractionRole, WriteAuthorizationId,
    VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};

static NEXT_INTEGRATION_SUFFIX: AtomicUsize = AtomicUsize::new(0);

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
    let adapter = adapter(&fixture);
    let input = Cursor::new(
        br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"harness-integration-test","version":"0.0.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
"#
        .to_vec(),
    );
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    let response = &responses[1];
    let names = response["result"]["tools"]
        .as_array()
        .expect("tools should be an array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();
    assert_eq!(
        &names[..PUBLIC_METHOD_TOOL_NAMES.len()],
        PUBLIC_METHOD_TOOL_NAMES
    );
    assert_eq!(
        &names[PUBLIC_METHOD_TOOL_NAMES.len()..],
        ADAPTER_UTILITY_TOOL_NAMES
    );
    assert_eq!(names.len(), 10);
    Ok(())
}

#[test]
fn stdio_rejected_lifecycle_and_notification_tool_calls_have_no_storage_effect(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_stdio_protocol_no_effect")?;
    let adapter = adapter(&fixture);
    let before = fixture.counts()?;
    let mutating_arguments = mcp_arguments(fixture.intake_request(
        "req_stdio_no_effect",
        "idem_stdio_no_effect",
        false,
        Some(0),
    ))?;
    let input = stdio_input(&[
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "harness.intake",
                "arguments": mutating_arguments.clone()
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {
                    "name": "harness-integration-test",
                    "version": "0.0.0"
                }
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "harness.intake",
                "arguments": mutating_arguments.clone()
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": []
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": null
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "params": {}
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "harness.intake",
                "arguments": mutating_arguments
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/unknown",
            "params": {}
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/list",
            "params": {}
        }),
    ])?;
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 4);
    assert_eq!(responses[0]["id"], 1);
    assert_eq!(responses[0]["error"]["code"], -32600);
    assert_eq!(responses[1]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(responses[2]["id"], 3);
    assert_eq!(responses[2]["error"]["code"], -32600);
    assert!(responses[3]["result"]["tools"].is_array());
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn mcp_tool_schemas_are_closed_top_level_objects() {
    for tool in public_method_tools() {
        assert_eq!(
            tool.input_schema["type"], "object",
            "{} schema should be a top-level object",
            tool.name
        );
        assert_eq!(
            tool.input_schema["additionalProperties"], false,
            "{} schema should reject additional top-level properties",
            tool.name
        );
        for forbidden in [
            "surface_id",
            "verified_surface_context",
            "access_class",
            "capability_profile",
            "verification_basis",
        ] {
            assert!(
                !schema_has_property(&tool.input_schema, forbidden),
                "{} schema should not expose request-level authority field {forbidden}",
                tool.name
            );
        }
        let envelope_required = envelope_required_fields(&tool.input_schema)
            .expect("tool schema should contain ToolEnvelope schema");
        assert!(
            schema_has_property(&tool.input_schema, "project_id"),
            "{} schema should expose envelope.project_id",
            tool.name
        );
        assert!(
            !envelope_required.contains(&"project_id".to_owned()),
            "{} schema should not require envelope.project_id",
            tool.name
        );
        assert!(
            !envelope_required.contains(&"surface_id".to_owned()),
            "{} schema should not require envelope.surface_id",
            tool.name
        );
    }
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

    let adapter = adapter(&fixture);
    let params = mcp_arguments(fixture.stage_artifact_request(
        "req_mcp_stage",
        None,
        false,
        Some(1),
        &task_id,
    ))?;

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
fn bound_session_rejects_different_request_project_without_effect() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_project_binding")?;
    let other_project_id = "project_other_binding";
    register_extra_project_surface(
        &fixture,
        other_project_id,
        fixture.surface_id(),
        "surface_instance_other_project",
    )?;
    let adapter = adapter(&fixture);
    let mut request = fixture.status_request("req_project_mismatch", None);
    request.envelope.project_id = ProjectId::new(other_project_id);
    let before_bound = fixture.counts()?;
    let before_other = counts_for_project(&fixture, other_project_id)?;

    let error = adapter
        .call_tool("harness.status", mcp_arguments(request)?)
        .expect_err("ungranted project should fail before Core");

    assert_tool_execution_error(&error, "not allowed");
    assert_eq!(fixture.counts()?, before_bound);
    assert_eq!(
        counts_for_project(&fixture, other_project_id)?,
        before_other
    );
    Ok(())
}

#[test]
fn bound_session_rejects_caller_supplied_surface_id_without_effect() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_surface_binding")?;
    register_surface(
        fixture.runtime_home_path(),
        SurfaceRegistration {
            project_id: fixture.project_id().to_owned(),
            surface_id: "surface_other_binding".to_owned(),
            surface_instance_id: "surface_instance_other_binding".to_owned(),
            surface_kind: "local_test".to_owned(),
            interaction_role: SurfaceInteractionRole::Agent,
            display_name: Some("Other binding surface".to_owned()),
            capability_profile_json: default_capability_profile().to_string(),
            local_access_json: local_access_without(&[]).to_string(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    let adapter = adapter(&fixture);

    for (case, supplied_surface_id) in [
        ("matching", fixture.surface_id()),
        ("mismatching", "surface_other_binding"),
    ] {
        let request = fixture.status_request(&format!("req_surface_forbidden_{case}"), None);
        let mut params = mcp_arguments(request)?;
        params["envelope"]["surface_id"] = json!(supplied_surface_id);
        let before = fixture.counts()?;

        let error = adapter
            .call_tool("harness.status", params)
            .expect_err("caller-supplied surface_id should fail before Core");

        assert_tool_execution_error(&error, "surface_id");
        assert_eq!(fixture.counts()?, before);
    }
    Ok(())
}

#[test]
fn same_surface_instance_id_in_another_project_does_not_permit_access() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp_same_instance_other_project")?;
    let other_project_id = "project_same_instance";
    register_extra_project_surface(
        &fixture,
        other_project_id,
        fixture.surface_id(),
        fixture.surface_instance_id(),
    )?;
    let adapter = adapter(&fixture);
    let mut request = fixture.status_request("req_same_instance_other_project", None);
    request.envelope.project_id = ProjectId::new(other_project_id);
    let before_bound = fixture.counts()?;
    let before_other = counts_for_project(&fixture, other_project_id)?;

    let error = adapter
        .call_tool("harness.status", mcp_arguments(request)?)
        .expect_err("ungranted project should fail before Core");

    assert_tool_execution_error(&error, "not allowed");
    assert_eq!(fixture.counts()?, before_bound);
    assert_eq!(
        counts_for_project(&fixture, other_project_id)?,
        before_other
    );
    Ok(())
}

#[test]
fn same_surface_id_in_another_project_does_not_permit_access() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_same_surface_other_project")?;
    let other_project_id = "project_same_surface";
    register_extra_project_surface(
        &fixture,
        other_project_id,
        fixture.surface_id(),
        "surface_instance_same_surface_other_project",
    )?;
    let adapter = adapter(&fixture);
    let mut request = fixture.status_request("req_same_surface_other_project", None);
    request.envelope.project_id = ProjectId::new(other_project_id);
    let before_bound = fixture.counts()?;
    let before_other = counts_for_project(&fixture, other_project_id)?;

    let error = adapter
        .call_tool("harness.status", mcp_arguments(request)?)
        .expect_err("ungranted project should fail before Core");

    assert_tool_execution_error(&error, "not allowed");
    assert_eq!(fixture.counts()?, before_bound);
    assert_eq!(
        counts_for_project(&fixture, other_project_id)?,
        before_other
    );
    Ok(())
}

#[test]
fn deleted_bound_surface_fails_later_calls_closed_without_effect() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_deleted_bound_surface")?;
    let surface_id = "surface_deleted_binding";
    let surface_instance_id = "surface_instance_deleted_binding";
    register_surface(
        fixture.runtime_home_path(),
        SurfaceRegistration {
            project_id: fixture.project_id().to_owned(),
            surface_id: surface_id.to_owned(),
            surface_instance_id: surface_instance_id.to_owned(),
            surface_kind: "local_test".to_owned(),
            interaction_role: SurfaceInteractionRole::Agent,
            display_name: Some("Deleted binding surface".to_owned()),
            capability_profile_json: default_capability_profile().to_string(),
            local_access_json: local_access_without(&[]).to_string(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    let adapter = adapter_for_surface(
        &fixture,
        fixture.project_id(),
        surface_id,
        surface_instance_id,
    );
    fixture.conn()?.execute(
        "DELETE FROM surfaces
          WHERE project_id = ?1
            AND surface_id = ?2
            AND surface_instance_id = ?3",
        rusqlite::params![fixture.project_id(), surface_id, surface_instance_id],
    )?;
    let before = fixture.counts()?;
    let request = fixture.status_request("req_deleted_bound_surface", None);

    let error = adapter
        .call_tool("harness.status", mcp_arguments(request)?)
        .expect_err("deleted integration surface should fail before Core");

    assert!(matches!(
        error,
        volicord_mcp::McpAdapterError::ToolExecution { .. }
    ));
    assert!(error.to_string().contains("surface instance"));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn invalid_integration_project_registration_blocks_core_execution_and_listing(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_invalid_integration_registration")?;
    let integration_id = next_integration_id();
    set_surface_role(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.surface_id(),
        fixture.surface_instance_id(),
        SurfaceInteractionRole::Agent,
    )?;
    register_agent_integration(
        fixture.runtime_home_path(),
        AgentIntegrationRegistration {
            integration_id: integration_id.clone(),
            interaction_role: "agent".to_owned(),
            surface_id: fixture.surface_id().to_owned(),
            surface_instance_id: fixture.surface_instance_id().to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    add_integration_project(
        fixture.runtime_home_path(),
        IntegrationProjectRegistration {
            integration_id: integration_id.clone(),
            project_id: fixture.project_id().to_owned(),
        },
    )?;
    let context = McpIntegrationContext::resolve(fixture.runtime_home_path(), &integration_id)?
        .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
    let adapter = McpAdapter::new(fixture.runtime_home_path(), context);
    replace_project_repo_root(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.runtime_home_path(),
    )?;

    let list_error = list_projects(fixture.runtime_home_path())
        .expect_err("invalid registration should reject operational project listing");
    assert!(list_error.to_string().contains("same_path"));
    let startup_error =
        McpIntegrationContext::resolve(fixture.runtime_home_path(), &integration_id)
            .expect_err("invalid registration should reject integration startup");
    assert!(startup_error.to_string().contains("same_path"));

    let error = adapter
        .call_tool(
            "harness.status",
            mcp_arguments(fixture.status_request("req_invalid_integration_registration", None))?,
        )
        .expect_err("invalid integration project should fail before Core");
    assert!(error.to_string().contains("same_path"));
    Ok(())
}

#[test]
fn exact_idempotency_replay_succeeds_inside_bound_session() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_bound_replay")?;
    let adapter = adapter(&fixture);
    let request = fixture.intake_request("req_bound_replay", "idem_bound_replay", false, Some(0));

    let first = adapter.call_tool("harness.intake", mcp_arguments(request.clone())?)?;
    let after_first = fixture.counts()?;
    let second = adapter.call_tool("harness.intake", mcp_arguments(request)?)?;

    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(fixture.counts()?, after_first);
    Ok(())
}

#[test]
fn one_mcp_session_with_baseline_workflow_surface_runs_full_access_workflow(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_baseline_workflow")?;
    let adapter = adapter(&fixture);

    let status = adapter.call_tool(
        "harness.status",
        mcp_arguments(fixture.status_request("req_mcp_full_status", None))?,
    )?;
    assert_eq!(status.response_value["base"]["response_kind"], "result");
    assert_eq!(
        status
            .verified_surface
            .as_ref()
            .expect("status verified surface")
            .access_class,
        AccessClass::ReadStatus
    );

    let intake = adapter.call_tool(
        "harness.intake",
        mcp_arguments(fixture.intake_request(
            "req_mcp_full_intake",
            "idem_mcp_full_intake",
            false,
            Some(0),
        ))?,
    )?;
    assert_eq!(intake.response_value["base"]["response_kind"], "result");
    assert_eq!(
        intake
            .verified_surface
            .as_ref()
            .expect("intake verified surface")
            .access_class,
        AccessClass::CoreMutation
    );
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id")
        .to_owned();

    let scope = adapter.call_tool(
        "harness.update_scope",
        mcp_arguments(fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_mcp_full_scope",
            idempotency_key: "idem_mcp_full_scope",
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "Baseline MCP workflow scope.",
        }))?,
    )?;
    assert_eq!(scope.response_value["base"]["response_kind"], "result");
    let change_unit_id = scope.response_value["change_unit_ref"]["record_id"]
        .as_str()
        .expect("update_scope response should carry current Change Unit")
        .to_owned();

    let prepare = adapter.call_tool(
        "harness.prepare_write",
        mcp_arguments(fixture.prepare_write_request(
            "req_mcp_full_prepare",
            "idem_mcp_full_prepare",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ))?,
    )?;
    assert_eq!(prepare.response_value["decision"], "allowed");
    let write_authorization_id = prepare.response_value["write_authorization_ref"]["record_id"]
        .as_str()
        .expect("write authorization id")
        .to_owned();
    assert_eq!(
        prepare
            .verified_surface
            .as_ref()
            .expect("prepare verified surface")
            .access_class,
        AccessClass::WriteAuthorization
    );

    let product_path = fixture.product_repo_path().join(DEFAULT_PRODUCT_PATH);
    if let Some(parent) = product_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &product_path,
        "fixture product write observed by record_run\n",
    )?;

    let stage = adapter.call_tool(
        "harness.stage_artifact",
        mcp_arguments(fixture.stage_artifact_request(
            "req_mcp_full_stage",
            None,
            false,
            Some(3),
            &task_id,
        ))?,
    )?;
    assert_eq!(stage.response_value["base"]["response_kind"], "result");
    assert_eq!(
        stage.response_value["staged_artifact_handle"]["created_by_surface_id"],
        fixture.surface_id()
    );
    assert_eq!(
        stage.response_value["staged_artifact_handle"]["created_by_surface_instance_id"],
        fixture.surface_instance_id()
    );
    let handle: StagedArtifactHandle =
        serde_json::from_value(stage.response_value["staged_artifact_handle"].clone())?;

    let mut run_request = fixture.record_run_request(
        "req_mcp_full_run",
        "idem_mcp_full_run",
        false,
        Some(3),
        &task_id,
        &change_unit_id,
    );
    run_request.artifact_inputs = vec![artifact_input_for_handle(
        "artifact_input_mcp_full",
        handle,
        Some("workflow_trace"),
        Some("MCP workflow trace recorded."),
    )];
    run_request.evidence_updates = vec![supported_evidence_update("MCP workflow trace recorded.")];
    run_request.observed_changes.product_file_write_observed = true;
    run_request.observed_changes.changed_paths = vec![DEFAULT_PRODUCT_PATH.to_owned()];
    run_request.write_authorization_id =
        Some(WriteAuthorizationId::new(&write_authorization_id)).into();
    run_request.close_assessment = Some(CloseAssessmentInput {
        result_summary: "MCP workflow trace recorded.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: vec![ResidualRiskInput {
            summary: "Manual MCP workflow verification remains visible.".to_owned(),
            consequence: "The user must accept the remaining manual verification risk.".to_owned(),
            acceptance_required: true,
            source_refs: Vec::new(),
        }],
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let run = adapter.call_tool("harness.record_run", mcp_arguments(run_request)?)?;
    assert_eq!(run.response_value["base"]["response_kind"], "result");
    let risk_id = run.response_value["current_close_basis"]["residual_risks"][0]["risk_id"]
        .as_str()
        .expect("generated risk id should be present")
        .to_owned();
    assert_eq!(
        fixture.write_authorization_status(&write_authorization_id)?,
        "consumed"
    );
    assert_eq!(
        run.verified_surface
            .as_ref()
            .expect("run verified surface")
            .access_class,
        AccessClass::RunRecording
    );

    let before_status = fixture.counts()?;
    let status = adapter.call_tool(
        "harness.status",
        mcp_arguments(fixture.status_request("req_mcp_full_status_after_run", Some(&task_id)))?,
    )?;
    assert_eq!(status.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(
        status.response_value["current_close_basis"],
        run.response_value["current_close_basis"]
    );
    assert_eq!(
        status.response_value["current_close_basis"]["residual_risks"][0]["risk_id"],
        risk_id
    );
    assert_eq!(
        status.response_value["evidence_summary"]["coverage_items"][0]["claim"],
        "MCP workflow trace recorded."
    );
    assert_eq!(
        status.response_value["guarantee_display"]["level"],
        "cooperative"
    );
    assert_ne!(
        status.response_value["guarantee_display"]["level"],
        "detective"
    );
    assert_eq!(fixture.counts()?, before_status);

    let close_check = adapter.call_tool(
        "harness.close_task",
        mcp_arguments(fixture.close_task_request(CloseTaskFixture {
            request_id: "req_mcp_full_close_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }))?,
    )?;
    assert_eq!(
        close_check.response_value["base"]["effect_kind"],
        "read_only"
    );
    assert_eq!(
        status.response_value["close_blockers"],
        close_check.response_value["blockers"]
    );
    assert_eq!(fixture.counts()?, before_status);
    assert_eq!(
        close_check
            .verified_surface
            .as_ref()
            .expect("close check verified surface")
            .access_class,
        AccessClass::ReadStatus
    );

    let risk_judgment = adapter.call_tool(
        "harness.request_user_judgment",
        mcp_arguments(fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_mcp_full_risk",
            idempotency_key: "idem_mcp_full_risk",
            dry_run: false,
            expected_state_version: Some(4),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::ResidualRiskAcceptance,
        }))?,
    )?;
    assert_eq!(
        risk_judgment.response_value["base"]["response_kind"],
        "result"
    );
    let risk_judgment_id = risk_judgment.response_value["user_judgment_ref"]["record_id"]
        .as_str()
        .expect("risk judgment id")
        .to_owned();
    assert_eq!(
        risk_judgment
            .verified_surface
            .as_ref()
            .expect("judgment request verified surface")
            .access_class,
        AccessClass::CoreMutation
    );
    assert_eq!(fixture.user_judgment_status(&risk_judgment_id)?, "pending");
    Ok(())
}

#[test]
fn capability_profile_text_cannot_override_registered_agent_role_for_authority(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_role_capability")?;
    let adapter = adapter(&fixture);

    let intake = adapter.call_tool(
        "harness.intake",
        mcp_arguments(fixture.intake_request(
            "req_mcp_role_task",
            "idem_mcp_role_task",
            false,
            Some(0),
        ))?,
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id")
        .to_owned();
    let scope = adapter.call_tool(
        "harness.update_scope",
        mcp_arguments(fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_mcp_role_scope",
            idempotency_key: "idem_mcp_role_scope",
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "MCP role derivation scope.",
        }))?,
    )?;
    assert_eq!(scope.response_value["base"]["response_kind"], "result");
    let change_unit_id = fixture
        .current_change_unit_id(&task_id)?
        .expect("Change Unit should be current");
    let mut run_request = fixture.record_run_request(
        "req_mcp_role_run",
        "idem_mcp_role_run",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    run_request.evidence_updates = vec![supported_evidence_update("MCP role basis recorded.")];
    run_request.close_assessment = Some(CloseAssessmentInput {
        result_summary: "MCP role basis recorded.".to_owned(),
        result_refs: Vec::new(),
        residual_risks: Vec::new(),
        sensitive_categories: Vec::new(),
        recovery_constraints: Vec::new(),
    })
    .into();
    let run = adapter.call_tool("harness.record_run", mcp_arguments(run_request)?)?;
    assert_eq!(run.response_value["base"]["response_kind"], "result");
    let final_judgment = adapter.call_tool(
        "harness.request_user_judgment",
        mcp_arguments(fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_mcp_role_final",
            idempotency_key: "idem_mcp_role_final",
            dry_run: false,
            expected_state_version: Some(3),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::FinalAcceptance,
        }))?,
    )?;
    let judgment_id = final_judgment.response_value["user_judgment_ref"]["record_id"]
        .as_str()
        .expect("judgment id")
        .to_owned();
    fixture.conn()?.execute(
        "UPDATE surfaces
            SET interaction_role = 'agent'
          WHERE project_id = ?1
            AND surface_id = ?2
            AND surface_instance_id = ?3",
        rusqlite::params![
            fixture.project_id(),
            fixture.surface_id(),
            fixture.surface_instance_id()
        ],
    )?;
    fixture.set_surface_capability(json!({
        "access_class": "core_mutation",
        "supported_access_classes": ["core_mutation"],
        "interaction_role": "user_interaction"
    }))?;
    let before = fixture.counts()?;

    let record = adapter.call_tool(
        "harness.record_user_judgment",
        mcp_arguments(fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_mcp_role_final_record",
            idempotency_key: "idem_mcp_role_final_record",
            expected_state_version: Some(4),
            task_id: &task_id,
            user_judgment_id: &judgment_id,
            judgment_kind: JudgmentKind::FinalAcceptance,
            answer: answer_payload(JudgmentKind::FinalAcceptance),
        }))?,
    )?;

    assert_rejected_code(&record.response_value, "LOCAL_ACCESS_MISMATCH");
    assert_eq!(
        record.response_value["errors"][0]["details"]["field"],
        "surfaces.interaction_role"
    );
    assert_eq!(fixture.user_judgment_status(&judgment_id)?, "pending");
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn missing_run_recording_grant_blocks_only_record_run() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_missing_run")?;
    fixture.set_surface_local_access(json!({
        "authorized_access_classes": [
            "read_status",
            "core_mutation",
            "write_authorization",
            "artifact_registration"
        ],
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    }))?;
    let adapter = adapter(&fixture);

    let intake = adapter.call_tool(
        "harness.intake",
        mcp_arguments(fixture.intake_request(
            "req_missing_run_intake",
            "idem_missing_run_intake",
            false,
            Some(0),
        ))?,
    )?;
    assert_eq!(intake.response_value["base"]["response_kind"], "result");
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id")
        .to_owned();

    let scope = adapter.call_tool(
        "harness.update_scope",
        mcp_arguments(fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_missing_run_scope",
            idempotency_key: "idem_missing_run_scope",
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "Scope remains mutable without run_recording.",
        }))?,
    )?;
    assert_eq!(scope.response_value["base"]["response_kind"], "result");
    let change_unit_id = fixture
        .current_change_unit_id(&task_id)?
        .expect("Change Unit should be current");

    let prepare = adapter.call_tool(
        "harness.prepare_write",
        mcp_arguments(fixture.prepare_write_request(
            "req_missing_run_prepare",
            "idem_missing_run_prepare",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ))?,
    )?;
    assert_eq!(prepare.response_value["base"]["response_kind"], "result");

    let stage = adapter.call_tool(
        "harness.stage_artifact",
        mcp_arguments(fixture.stage_artifact_request(
            "req_missing_run_stage",
            None,
            false,
            Some(3),
            &task_id,
        ))?,
    )?;
    assert_eq!(stage.response_value["base"]["response_kind"], "result");

    let before_run = fixture.counts()?;
    let error = adapter
        .call_tool(
            "harness.record_run",
            mcp_arguments(fixture.record_run_request(
                "req_missing_run_record",
                "idem_missing_run_record",
                false,
                Some(3),
                &task_id,
                &change_unit_id,
            ))?,
        )
        .expect_err("missing run_recording grant should fail before Core");
    assert_tool_execution_error(&error, "run_recording");
    assert_eq!(fixture.counts()?, before_run);
    Ok(())
}

#[test]
fn missing_write_authorization_grant_blocks_prepare_write() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_missing_write")?;
    fixture.set_surface_local_access(json!({
        "authorized_access_classes": [
            "read_status",
            "core_mutation",
            "artifact_registration",
            "run_recording"
        ],
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    }))?;
    let adapter = adapter(&fixture);

    let intake = adapter.call_tool(
        "harness.intake",
        mcp_arguments(fixture.intake_request(
            "req_missing_write_intake",
            "idem_missing_write_intake",
            false,
            Some(0),
        ))?,
    )?;
    assert_eq!(intake.response_value["base"]["response_kind"], "result");
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id")
        .to_owned();

    let scope = adapter.call_tool(
        "harness.update_scope",
        mcp_arguments(fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_missing_write_scope",
            idempotency_key: "idem_missing_write_scope",
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "Scope remains mutable without write_authorization.",
        }))?,
    )?;
    assert_eq!(scope.response_value["base"]["response_kind"], "result");
    let change_unit_id = fixture
        .current_change_unit_id(&task_id)?
        .expect("Change Unit should be current");

    let before_prepare = fixture.counts()?;
    let error = adapter
        .call_tool(
            "harness.prepare_write",
            mcp_arguments(fixture.prepare_write_request(
                "req_missing_write_prepare",
                "idem_missing_write_prepare",
                Some(2),
                Some(&task_id),
                Some(&change_unit_id),
            ))?,
        )
        .expect_err("missing write_authorization grant should fail before Core");
    assert_tool_execution_error(&error, "write_authorization");
    assert_eq!(fixture.counts()?, before_prepare);
    Ok(())
}

#[test]
fn removed_read_status_grant_blocks_read_methods_only() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_missing_read")?;
    let core = CoreService::new(fixture.runtime_home_path());
    let intake = core.intake(
        fixture.intake_request(
            "req_missing_read_task",
            "idem_missing_read_task",
            false,
            Some(0),
        ),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id")
        .to_owned();
    core.update_scope(
        fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_missing_read_scope",
            idempotency_key: "idem_missing_read_scope",
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "Read-status grant reduction scope.",
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    fixture.set_surface_capability(default_capability_profile())?;
    fixture.set_surface_local_access(local_access_without(&["read_status"]))?;
    let adapter = adapter(&fixture);
    let before_read = fixture.counts()?;

    let status = adapter
        .call_tool(
            "harness.status",
            mcp_arguments(fixture.status_request("req_missing_read_status", Some(&task_id)))?,
        )
        .expect_err("missing read_status grant should fail before Core");
    assert_tool_execution_error(&status, "read_status");
    let close_check = adapter
        .call_tool(
            "harness.close_task",
            mcp_arguments(fixture.close_task_request(CloseTaskFixture {
                request_id: "req_missing_read_close_check",
                idempotency_key: None,
                dry_run: false,
                expected_state_version: None,
                task_id: &task_id,
                intent: CloseIntent::Check,
                close_reason: None,
                superseding_task_id: None,
            }))?,
        )
        .expect_err("missing read_status grant should fail before Core");
    assert_tool_execution_error(&close_check, "read_status");
    assert_eq!(fixture.counts()?, before_read);

    let mutation = adapter.call_tool(
        "harness.intake",
        mcp_arguments(fixture.intake_request(
            "req_missing_read_mutation",
            "idem_missing_read_mutation",
            false,
            Some(before_read.state_version),
        ))?,
    )?;
    assert_eq!(mutation.response_value["base"]["response_kind"], "result");
    assert_eq!(
        mutation
            .verified_surface
            .as_ref()
            .expect("core grant should remain usable")
            .access_class,
        AccessClass::CoreMutation
    );
    Ok(())
}

#[test]
fn removed_core_mutation_grant_blocks_mutating_core_methods_only() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_missing_core")?;
    let core = CoreService::new(fixture.runtime_home_path());
    let intake = core.intake(
        fixture.intake_request(
            "req_missing_core_task",
            "idem_missing_core_task",
            false,
            Some(0),
        ),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id")
        .to_owned();
    core.update_scope(
        fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_missing_core_scope",
            idempotency_key: "idem_missing_core_scope",
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "Core-mutation grant reduction scope.",
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let change_unit_id = fixture
        .current_change_unit_id(&task_id)?
        .expect("Change Unit should be current");
    fixture.set_surface_capability(default_capability_profile())?;
    fixture.set_surface_local_access(local_access_without(&["core_mutation"]))?;
    let adapter = adapter(&fixture);

    for (tool_name, params) in [
        (
            "harness.intake",
            mcp_arguments(fixture.intake_request(
                "req_missing_core_intake",
                "idem_missing_core_intake",
                false,
                Some(2),
            ))?,
        ),
        (
            "harness.update_scope",
            mcp_arguments(fixture.update_scope_request(UpdateScopeFixture {
                request_id: "req_missing_core_update",
                idempotency_key: "idem_missing_core_update",
                dry_run: false,
                expected_state_version: Some(2),
                task_id: &task_id,
                operation: ChangeUnitOperation::KeepCurrent,
                scope_summary: "This should not update without core_mutation.",
            }))?,
        ),
        (
            "harness.request_user_judgment",
            mcp_arguments(fixture.user_judgment_request(UserJudgmentFixture {
                request_id: "req_missing_core_judgment",
                idempotency_key: "idem_missing_core_judgment",
                dry_run: false,
                expected_state_version: Some(2),
                task_id: &task_id,
                change_unit_id: Some(&change_unit_id),
                judgment_kind: JudgmentKind::TechnicalDecision,
            }))?,
        ),
        (
            "harness.close_task",
            mcp_arguments(fixture.close_task_request(CloseTaskFixture {
                request_id: "req_missing_core_close",
                idempotency_key: Some("idem_missing_core_close"),
                dry_run: false,
                expected_state_version: Some(2),
                task_id: &task_id,
                intent: CloseIntent::Complete,
                close_reason: Some(CloseReason::CompletedSelfChecked),
                superseding_task_id: None,
            }))?,
        ),
    ] {
        let before = fixture.counts()?;
        let error = adapter
            .call_tool(tool_name, params)
            .expect_err("missing core_mutation grant should fail before Core");
        assert_tool_execution_error(&error, "core_mutation");
        assert_eq!(
            fixture.counts()?,
            before,
            "{tool_name} should have no effect"
        );
    }

    let before_prepare = fixture.counts()?;
    let prepare = adapter.call_tool(
        "harness.prepare_write",
        mcp_arguments(fixture.prepare_write_request(
            "req_missing_core_prepare",
            "idem_missing_core_prepare",
            Some(before_prepare.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        ))?,
    )?;
    assert_eq!(prepare.response_value["decision"], "allowed");
    assert_eq!(
        prepare
            .verified_surface
            .as_ref()
            .expect("write_authorization grant should remain usable")
            .access_class,
        AccessClass::WriteAuthorization
    );
    Ok(())
}

#[test]
fn removed_artifact_registration_grant_blocks_stage_only() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_missing_artifact")?;
    let core = CoreService::new(fixture.runtime_home_path());
    let intake = core.intake(
        fixture.intake_request(
            "req_missing_artifact_task",
            "idem_missing_artifact_task",
            false,
            Some(0),
        ),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id")
        .to_owned();
    core.update_scope(
        fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_missing_artifact_scope",
            idempotency_key: "idem_missing_artifact_scope",
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "Artifact grant reduction scope.",
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let change_unit_id = fixture
        .current_change_unit_id(&task_id)?
        .expect("Change Unit should be current");
    fixture.set_surface_capability(default_capability_profile())?;
    fixture.set_surface_local_access(local_access_without(&["artifact_registration"]))?;
    let adapter = adapter(&fixture);
    let before_stage = fixture.counts()?;

    let stage = adapter
        .call_tool(
            "harness.stage_artifact",
            mcp_arguments(fixture.stage_artifact_request(
                "req_missing_artifact_stage",
                None,
                false,
                Some(before_stage.state_version),
                &task_id,
            ))?,
        )
        .expect_err("missing artifact_registration grant should fail before Core");
    assert_tool_execution_error(&stage, "artifact_registration");
    assert_eq!(fixture.counts()?, before_stage);

    let run = adapter.call_tool(
        "harness.record_run",
        mcp_arguments(fixture.record_run_request(
            "req_missing_artifact_run",
            "idem_missing_artifact_run",
            false,
            Some(before_stage.state_version),
            &task_id,
            &change_unit_id,
        ))?,
    )?;
    assert_eq!(run.response_value["base"]["response_kind"], "result");
    assert_eq!(
        run.verified_surface
            .as_ref()
            .expect("run_recording grant should remain usable")
            .access_class,
        AccessClass::RunRecording
    );
    Ok(())
}

#[test]
fn close_task_access_derives_from_typed_intent() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_close_intent_access")?;
    let core = CoreService::new(fixture.runtime_home_path());
    let intake = core.intake(
        fixture.intake_request(
            "req_close_intent_task",
            "idem_close_intent_task",
            false,
            Some(0),
        ),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id")
        .to_owned();

    fixture.set_surface_local_access(json!({
        "authorized_access_classes": ["read_status"],
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    }))?;
    let adapter = adapter(&fixture);
    let check = adapter.call_tool(
        "harness.close_task",
        mcp_arguments(fixture.close_task_request(CloseTaskFixture {
            request_id: "req_close_intent_check",
            idempotency_key: None,
            dry_run: false,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Check,
            close_reason: None,
            superseding_task_id: None,
        }))?,
    )?;
    assert_eq!(check.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(
        check
            .verified_surface
            .as_ref()
            .expect("check verified surface")
            .access_class,
        AccessClass::ReadStatus
    );

    let mutating_without_core = adapter
        .call_tool(
            "harness.close_task",
            mcp_arguments(fixture.close_task_request(CloseTaskFixture {
                request_id: "req_close_intent_complete_no_core",
                idempotency_key: None,
                dry_run: true,
                expected_state_version: None,
                task_id: &task_id,
                intent: CloseIntent::Complete,
                close_reason: Some(volicord_types::CloseReason::CompletedSelfChecked),
                superseding_task_id: None,
            }))?,
        )
        .expect_err("missing core_mutation grant should fail before Core");
    assert_tool_execution_error(&mutating_without_core, "core_mutation");

    fixture.set_surface_local_access(json!({
        "authorized_access_classes": ["core_mutation"],
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    }))?;
    let mutating_with_core = adapter.call_tool(
        "harness.close_task",
        mcp_arguments(fixture.close_task_request(CloseTaskFixture {
            request_id: "req_close_intent_complete_core",
            idempotency_key: None,
            dry_run: true,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(volicord_types::CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }))?,
    )?;
    assert_eq!(
        mutating_with_core.response_value["base"]["response_kind"],
        "dry_run"
    );
    assert_eq!(
        mutating_with_core
            .verified_surface
            .as_ref()
            .expect("mutating close verified surface")
            .access_class,
        AccessClass::CoreMutation
    );
    Ok(())
}

#[test]
fn integration_access_class_derives_from_method_not_caller_fields() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_env_no_elevate")?;
    fixture.set_surface_local_access(json!({
        "authorized_access_classes": ["read_status"],
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    }))?;
    let adapter = adapter(&fixture);
    let before = fixture.counts()?;

    let response = adapter.call_tool(
        "harness.status",
        mcp_arguments(fixture.status_request("req_env_no_elevate", None))?,
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    let verified = response
        .verified_surface
        .as_ref()
        .expect("status should use method-derived read access");
    assert_eq!(verified.access_class, AccessClass::ReadStatus);
    assert_eq!(
        verified.verification_basis,
        "local_admin_registration:test_fixture_binding"
    );
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn integration_binding_basis_is_used_for_newly_stored_trusted_basis() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp_env_basis_storage")?;
    let adapter = adapter(&fixture);

    let intake = adapter.call_tool(
        "harness.intake",
        mcp_arguments(fixture.intake_request(
            "req_env_basis_intake",
            "idem_env_basis_intake",
            false,
            Some(0),
        ))?,
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id")
        .to_owned();
    adapter.call_tool(
        "harness.update_scope",
        mcp_arguments(fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_env_basis_scope",
            idempotency_key: "idem_env_basis_scope",
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "Environment basis must not persist.",
        }))?,
    )?;
    let change_unit_id = fixture
        .current_change_unit_id(&task_id)?
        .expect("Change Unit should be current");

    let prepare = adapter.call_tool(
        "harness.prepare_write",
        mcp_arguments(fixture.prepare_write_request(
            "req_env_basis_prepare",
            "idem_env_basis_prepare",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ))?,
    )?;
    assert_eq!(prepare.response_value["decision"], "allowed");
    let write_authorization_id = prepare.response_value["write_authorization_ref"]["record_id"]
        .as_str()
        .expect("write authorization id");

    let conn = fixture.conn()?;
    let authorization_metadata: String = conn.query_row(
        "SELECT metadata_json
           FROM write_authorizations
          WHERE project_id = ?1
            AND write_authorization_id = ?2",
        rusqlite::params![fixture.project_id(), write_authorization_id],
        |row| row.get(0),
    )?;
    let authorization_metadata: Value = serde_json::from_str(&authorization_metadata)?;
    assert_eq!(
        authorization_metadata["verification_basis"],
        "local_admin_registration:test_fixture_binding"
    );

    let replay_basis: String = conn.query_row(
        "SELECT verification_basis
           FROM tool_invocations
          WHERE project_id = ?1
            AND idempotency_key = 'idem_env_basis_prepare'",
        rusqlite::params![fixture.project_id()],
        |row| row.get(0),
    )?;
    assert_eq!(
        replay_basis,
        "local_admin_registration:test_fixture_binding"
    );
    Ok(())
}

#[test]
fn invalid_mcp_authority_fields_are_rejected_before_core() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_invalid_fields")?;
    let core = CoreService::new(fixture.runtime_home_path());
    let intake = core.intake(
        fixture.intake_request("req_invalid_task", "idem_invalid_task", false, Some(0)),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    let adapter = adapter(&fixture);

    for (field_path, forged_value) in [
        ("envelope.verified", json!(true)),
        (
            "envelope.surface_instance_id",
            json!("surface_instance_forged"),
        ),
        ("verified_surface_context", json!({ "verified": true })),
        ("access_class", json!("core_mutation")),
        (
            "capability_profile",
            json!({ "artifact_registration": true }),
        ),
        ("verification_basis", json!("caller_basis")),
    ] {
        let mut params = mcp_arguments(fixture.stage_artifact_request(
            &format!("req_invalid_{}", field_path.replace('.', "_")),
            None,
            false,
            Some(1),
            &task_id,
        ))?;
        if let Some(field) = field_path.strip_prefix("envelope.") {
            params["envelope"][field] = forged_value;
        } else {
            params[field_path] = forged_value;
        }
        let before = fixture.counts()?;

        let error = adapter
            .call_tool("harness.stage_artifact", params)
            .expect_err("invalid request params should fail before Core");

        assert!(matches!(
            error,
            volicord_mcp::McpAdapterError::InvalidParams { .. }
        ));
        assert_eq!(
            fixture.counts()?,
            before,
            "{field_path} should create no storage effect"
        );
    }

    Ok(())
}

#[test]
fn stdio_invalid_known_tool_arguments_return_tool_error_without_storage_effect(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_stdio_invalid")?;
    let adapter = adapter(&fixture);
    let before = fixture.counts()?;
    let input = Cursor::new(
        br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"harness-integration-test","version":"0.0.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"harness.status","arguments":{"envelope":{"project_id":"project_fixture","task_id":null,"actor_kind":"agent","request_id":"req_stdio_invalid","idempotency_key":null,"expected_state_version":null,"dry_run":false,"locale":"en-US"},"include":{"task":true,"pending_user_judgments":true,"write_authority":true,"evidence":true,"close":true,"guarantees":true},"access_class":"core_mutation"}}}
"#
        .to_vec(),
    );
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    let response = &responses[1];
    assert!(response.get("error").is_none());
    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("tool error should include text");
    assert!(text.contains("Invalid arguments for harness.status"));
    assert!(!text.contains("McpAdapterError"));
    assert!(!text.contains("state.sqlite"));
    assert!(!text.contains(fixture.runtime_home_path().to_string_lossy().as_ref()));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn mcp_session_derives_access_per_method_call() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_access")?;
    let adapter = adapter(&fixture);
    let response = adapter.call_tool(
        "harness.status",
        mcp_arguments(fixture.status_request("req_status_derived", None))?,
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    Ok(())
}

#[test]
fn mcp_replay_rejects_different_session_access_class_without_stored_response(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_replay_context")?;
    let core = CoreService::new(fixture.runtime_home_path());
    let intake = core.intake(
        fixture.intake_request(
            "req_mcp_replay_task",
            "idem_mcp_replay_task",
            false,
            Some(0),
        ),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task ref should be present")
        .to_owned();
    core.update_scope(
        fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_mcp_replay_scope",
            idempotency_key: "idem_mcp_replay_scope",
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "MCP replay context scope.",
        }),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let change_unit_id = fixture
        .current_change_unit_id(&task_id)?
        .expect("Change Unit should be current");
    let request = fixture.prepare_write_request(
        "req_mcp_prepare_replay",
        "idem_mcp_prepare_replay",
        Some(2),
        Some(&task_id),
        Some(&change_unit_id),
    );

    let first =
        adapter(&fixture).call_tool("harness.prepare_write", mcp_arguments(request.clone())?)?;
    let after_first = fixture.counts()?;
    let write_authorization_id = first.response_value["write_authorization_ref"]["record_id"]
        .as_str()
        .expect("prepare_write should return an authorization id")
        .to_owned();

    let mismatch = core.prepare_write(request, invocation(&fixture, AccessClass::CoreMutation))?;

    assert_rejected_code(&mismatch.response_value, "LOCAL_ACCESS_MISMATCH");
    assert!(!mismatch.response_json.contains(&write_authorization_id));
    assert_eq!(fixture.counts()?, after_first);
    Ok(())
}

#[test]
fn registered_core_mutation_grant_rejects_requested_write_authorization(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("grant_reject")?;
    fixture.set_surface_local_access(json!({
        "authorized_access_classes": ["core_mutation"],
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    }))?;
    let core = CoreService::new(fixture.runtime_home_path());

    let response = core.prepare_write(
        fixture.prepare_write_request("req_grant_reject", "idem_grant_reject", Some(0), None, None),
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;

    assert_rejected_code(&response.response_value, "LOCAL_ACCESS_MISMATCH");
    assert!(response.verified_surface.is_none());
    Ok(())
}

#[test]
fn capability_profile_cannot_override_registered_local_grant() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("capability_no_grant")?;
    fixture.set_surface_local_access(json!({
        "authorized_access_classes": ["core_mutation"],
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    }))?;
    fixture.set_surface_capability(json!({
        "access_class": "write_authorization",
        "supported_access_classes": ["write_authorization"],
        "write_authorization": true
    }))?;
    let core = CoreService::new(fixture.runtime_home_path());

    let response = core.prepare_write(
        fixture.prepare_write_request("req_cap_no_grant", "idem_cap_no_grant", Some(0), None, None),
        invocation(&fixture, AccessClass::WriteAuthorization),
    )?;

    assert_rejected_code(&response.response_value, "LOCAL_ACCESS_MISMATCH");
    assert!(response.verified_surface.is_none());
    Ok(())
}

#[test]
fn matching_registered_grant_and_requested_access_succeeds() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("grant_match")?;
    fixture.set_surface_local_access(json!({
        "authorized_access_classes": ["read_status"],
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    }))?;
    let core = CoreService::new(fixture.runtime_home_path());

    let response = core.status(
        fixture.status_request("req_grant_match", None),
        invocation(&fixture, AccessClass::ReadStatus),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    let verified = response
        .verified_surface
        .as_ref()
        .expect("matching grant should create verified surface context");
    assert_eq!(verified.access_class, AccessClass::ReadStatus);
    assert_eq!(
        verified.verification_basis,
        "local_admin_registration:test_fixture_binding"
    );
    Ok(())
}

#[test]
fn mcp_and_direct_status_omit_same_excluded_projection_fields() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_status_omitted")?;
    let core = CoreService::new(fixture.runtime_home_path());
    let intake = core.intake(
        fixture.intake_request(
            "req_status_omit_task",
            "idem_status_omit_task",
            false,
            Some(0),
        ),
        invocation(&fixture, AccessClass::CoreMutation),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id")
        .to_owned();
    let mut request = fixture.status_request("req_status_omit_direct", Some(&task_id));
    request.include = StatusInclude {
        task: true,
        pending_user_judgments: false,
        write_authority: false,
        evidence: false,
        close: false,
        guarantees: false,
    };
    let before = fixture.counts()?;

    let direct = core.status(
        request.clone(),
        invocation(&fixture, AccessClass::ReadStatus),
    )?;
    let mcp = adapter(&fixture).call_tool("harness.status", mcp_arguments(request)?)?;

    assert_eq!(direct.response_value, mcp.response_value);
    for field in [
        "evidence_summary",
        "close_state",
        "current_close_basis",
        "risk_acceptance_coverage",
        "close_blockers",
        "guarantee_display",
    ] {
        assert!(direct.response_value.get(field).is_none());
    }
    assert!(direct.response_value["active_task"]
        .get("close_blockers")
        .is_none());
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn unknown_surface_instance_is_rejected() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("unknown_instance")?;
    let core = CoreService::new(fixture.runtime_home_path());

    let response = core.status(
        fixture.status_request("req_unknown_instance", None),
        InvocationContext {
            binding: AdapterSessionBinding::new(
                ProjectId::new(fixture.project_id()),
                SurfaceId::new(fixture.surface_id()),
                SurfaceInstanceId::new("missing_surface_instance"),
                VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
            ),
            requested_access_class: AccessClass::ReadStatus,
        },
    )?;

    assert_rejected_code(&response.response_value, "LOCAL_ACCESS_MISMATCH");
    assert!(response.verified_surface.is_none());
    Ok(())
}

#[test]
fn direct_core_rejects_envelope_surface_mismatch() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("core_surface_binding")?;
    let core = CoreService::new(fixture.runtime_home_path());
    let mut request = fixture.status_request("req_core_surface_mismatch", None);
    request.envelope.surface_id = SurfaceId::new("surface_unbound");

    let response = core.status(request, invocation(&fixture, AccessClass::ReadStatus))?;

    assert_rejected_code(&response.response_value, "LOCAL_ACCESS_MISMATCH");
    assert_rejected_field(&response.response_value, "envelope.surface_id");
    assert!(response.verified_surface.is_none());
    Ok(())
}

#[test]
fn direct_core_rejects_envelope_project_mismatch_without_opening_target(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("core_project_binding")?;
    let core = CoreService::new(fixture.runtime_home_path());
    let mut request = fixture.status_request("req_core_project_mismatch", None);
    request.envelope.project_id = ProjectId::new("project_unbound");
    let before = fixture.counts()?;

    let response = core.status(request, invocation(&fixture, AccessClass::ReadStatus))?;

    assert_rejected_code(&response.response_value, "LOCAL_ACCESS_MISMATCH");
    assert_rejected_field(&response.response_value, "envelope.project_id");
    assert!(response.verified_surface.is_none());
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn malformed_local_access_document_fails_closed() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("malformed_grant")?;
    fixture.set_surface_local_access(json!({
        "authorized_access_classes": [],
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    }))?;
    let core = CoreService::new(fixture.runtime_home_path());

    let response = core.status(
        fixture.status_request("req_malformed_grant", None),
        invocation(&fixture, AccessClass::ReadStatus),
    )?;

    assert_rejected_code(&response.response_value, "MCP_UNAVAILABLE");
    assert!(response.verified_surface.is_none());
    Ok(())
}

#[test]
fn obsolete_single_access_class_grant_fails_closed() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("obsolete_grant")?;
    fixture.set_surface_local_access(json!({
        "access_class": "read_status"
    }))?;
    let core = CoreService::new(fixture.runtime_home_path());

    let response = core.status(
        fixture.status_request("req_obsolete_grant", None),
        invocation(&fixture, AccessClass::ReadStatus),
    )?;

    assert_rejected_code(&response.response_value, "MCP_UNAVAILABLE");
    assert!(response.verified_surface.is_none());
    Ok(())
}

#[test]
fn replay_surface_foreign_key_is_physical_restrictive_and_requires_identity(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("replay_fk")?;
    register_surface(
        fixture.runtime_home_path(),
        SurfaceRegistration {
            project_id: fixture.project_id().to_owned(),
            surface_id: "surface_replay_fk".to_owned(),
            surface_instance_id: "surface_instance_replay_fk".to_owned(),
            surface_kind: "local_test".to_owned(),
            interaction_role: SurfaceInteractionRole::Agent,
            display_name: Some("Replay FK surface".to_owned()),
            capability_profile_json: default_capability_profile().to_string(),
            local_access_json: local_access_without(&[]).to_string(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    let conn = fixture.conn()?;
    assert_replay_surface_foreign_key(&conn)?;

    conn.execute(
        "INSERT INTO tool_invocations (
            project_id,
            tool_name,
            idempotency_key,
            request_hash,
            basis_state_version,
            committed_state_version,
            surface_id,
            surface_instance_id,
            access_class,
            verification_basis,
            response_json,
            created_at
        )
        VALUES (
            ?1,
            'harness.status',
            'idem_verified_replay_fk',
            'sha256:verified-replay-fk',
            0,
            1,
            'surface_replay_fk',
            'surface_instance_replay_fk',
            'read_status',
            'local_admin_registration:mcp_stdio_surface_binding',
            '{\"stored\":\"verified\"}',
            '2026-01-01T00:00:00.000Z'
        )",
        rusqlite::params![fixture.project_id()],
    )?;

    let restrictive_delete = conn.execute(
        "DELETE FROM surfaces
          WHERE project_id = ?1
            AND surface_id = 'surface_replay_fk'
            AND surface_instance_id = 'surface_instance_replay_fk'",
        rusqlite::params![fixture.project_id()],
    );
    assert!(
        restrictive_delete.is_err(),
        "verified replay rows should restrict deleting their registered surface"
    );

    let dangling_verified = conn.execute(
        "INSERT INTO tool_invocations (
            project_id,
            tool_name,
            idempotency_key,
            request_hash,
            basis_state_version,
            committed_state_version,
            surface_id,
            surface_instance_id,
            access_class,
            verification_basis,
            response_json,
            created_at
        )
        VALUES (
            ?1,
            'harness.status',
            'idem_dangling_verified_replay',
            'sha256:dangling-replay',
            0,
            1,
            'missing_surface',
            'missing_surface_instance',
            'read_status',
            'local_admin_registration:mcp_stdio_surface_binding',
            '{\"stored\":\"dangling\"}',
            '2026-01-01T00:00:00.000Z'
        )",
        rusqlite::params![fixture.project_id()],
    );
    assert!(
        dangling_verified.is_err(),
        "dangling verified replay rows should fail physical FK insertion"
    );

    let missing_identity = conn.execute(
        "INSERT INTO tool_invocations (
            project_id,
            tool_name,
            idempotency_key,
            request_hash,
            basis_state_version,
            committed_state_version,
            response_json,
            created_at
        )
        VALUES (
            ?1,
            'harness.intake',
            'idem_missing_identity_replay_fk',
            'sha256:missing-identity-replay-fk',
            0,
            1,
            '{\"stored\":\"legacy\"}',
            '2026-01-01T00:00:00.000Z'
        )",
        rusqlite::params![fixture.project_id()],
    );
    assert!(
        missing_identity.is_err(),
        "replay rows without surface identity should fail insertion"
    );
    drop(conn);

    let conn = fixture.conn()?;
    assert_integrity_check_clean(&conn)?;
    assert_foreign_key_check_clean(&conn)?;
    Ok(())
}

fn adapter(fixture: &CoreFixture) -> McpAdapter {
    adapter_for_surface(
        fixture,
        fixture.project_id(),
        fixture.surface_id(),
        fixture.surface_instance_id(),
    )
}

fn adapter_for_surface(
    fixture: &CoreFixture,
    project_id: &str,
    surface_id: &str,
    surface_instance_id: &str,
) -> McpAdapter {
    let integration_id = next_integration_id();
    set_surface_role(
        fixture.runtime_home_path(),
        project_id,
        surface_id,
        surface_instance_id,
        SurfaceInteractionRole::Agent,
    )
    .expect("adapter surface should be made agent-bound");
    register_agent_integration(
        fixture.runtime_home_path(),
        AgentIntegrationRegistration {
            integration_id: integration_id.clone(),
            interaction_role: "agent".to_owned(),
            surface_id: surface_id.to_owned(),
            surface_instance_id: surface_instance_id.to_owned(),
            metadata_json: "{}".to_owned(),
        },
    )
    .expect("integration registration should succeed");
    add_integration_project(
        fixture.runtime_home_path(),
        IntegrationProjectRegistration {
            integration_id: integration_id.clone(),
            project_id: project_id.to_owned(),
        },
    )
    .expect("integration project membership should succeed");
    let context = McpIntegrationContext::resolve(fixture.runtime_home_path(), &integration_id)
        .expect("integration context should resolve")
        .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
    McpAdapter::new(fixture.runtime_home_path(), context)
}

fn next_integration_id() -> String {
    let suffix = NEXT_INTEGRATION_SUFFIX.fetch_add(1, Ordering::Relaxed);
    format!("agent_mcp_surface_{suffix}")
}

fn set_surface_role(
    runtime_home: &Path,
    project_id: &str,
    surface_id: &str,
    surface_instance_id: &str,
    interaction_role: SurfaceInteractionRole,
) -> Result<(), Box<dyn Error>> {
    let path = runtime_home
        .join("projects")
        .join(project_id)
        .join("state.sqlite");
    let conn = rusqlite::Connection::open(path)?;
    conn.execute(
        "UPDATE surfaces
            SET interaction_role = ?4
          WHERE project_id = ?1
            AND surface_id = ?2
            AND surface_instance_id = ?3",
        rusqlite::params![
            project_id,
            surface_id,
            surface_instance_id,
            interaction_role.as_str()
        ],
    )?;
    Ok(())
}

fn stdio_responses(output: &[u8]) -> Result<Vec<Value>, Box<dyn Error>> {
    let text = std::str::from_utf8(output)?;
    let mut responses = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        responses.push(serde_json::from_str(line)?);
    }
    Ok(responses)
}

fn stdio_input(messages: &[Value]) -> Result<Cursor<Vec<u8>>, serde_json::Error> {
    let mut input = Vec::new();
    for message in messages {
        serde_json::to_writer(&mut input, message)?;
        input.push(b'\n');
    }
    Ok(Cursor::new(input))
}

fn register_extra_project_surface(
    fixture: &CoreFixture,
    project_id: &str,
    surface_id: &str,
    surface_instance_id: &str,
) -> Result<(), Box<dyn Error>> {
    register_extra_project_surface_with_role(
        fixture,
        project_id,
        surface_id,
        surface_instance_id,
        SurfaceInteractionRole::Agent,
    )
}

fn register_extra_project_surface_with_role(
    fixture: &CoreFixture,
    project_id: &str,
    surface_id: &str,
    surface_instance_id: &str,
    interaction_role: SurfaceInteractionRole,
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
    register_surface(
        fixture.runtime_home_path(),
        SurfaceRegistration {
            project_id: project_id.to_owned(),
            surface_id: surface_id.to_owned(),
            surface_instance_id: surface_instance_id.to_owned(),
            surface_kind: "local_test".to_owned(),
            interaction_role,
            display_name: Some(format!("Extra project surface {surface_instance_id}")),
            capability_profile_json: default_capability_profile().to_string(),
            local_access_json: local_access_without(&[]).to_string(),
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(())
}

fn counts_for_project(
    fixture: &CoreFixture,
    project_id: &str,
) -> Result<StorageEffectCounts, Box<dyn Error>> {
    Ok(
        CoreProjectStore::open(fixture.runtime_home_path(), &ProjectId::new(project_id))?
            .effect_counts()?,
    )
}

fn replace_project_repo_root(
    runtime_home: &Path,
    project_id: &str,
    repo_root: &Path,
) -> Result<(), Box<dyn Error>> {
    let conn = rusqlite::Connection::open(registry_db_path(runtime_home))?;
    conn.execute(
        "UPDATE projects SET repo_root = ?2 WHERE project_id = ?1",
        rusqlite::params![project_id, repo_root.to_string_lossy().as_ref()],
    )?;
    Ok(())
}

fn local_access_without(removed: &[&str]) -> Value {
    let authorized_access_classes = [
        "read_status",
        "core_mutation",
        "write_authorization",
        "run_recording",
        "artifact_registration",
    ]
    .into_iter()
    .filter(|access_class| !removed.contains(access_class))
    .collect::<Vec<_>>();
    json!({
        "authorized_access_classes": authorized_access_classes,
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    })
}

fn default_capability_profile() -> Value {
    json!({
        "supported_access_classes": [
            "read_status",
            "core_mutation",
            "write_authorization",
            "run_recording",
            "artifact_registration"
        ],
        "write_authorization": true,
        "manual_artifact_attachment_supported": true
    })
}

fn invocation(fixture: &CoreFixture, access_class: AccessClass) -> InvocationContext {
    InvocationContext {
        binding: AdapterSessionBinding::new(
            ProjectId::new(fixture.project_id()),
            SurfaceId::new(fixture.surface_id()),
            SurfaceInstanceId::new(fixture.surface_instance_id()),
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        ),
        requested_access_class: access_class,
    }
}

fn mcp_arguments<T: serde::Serialize>(request: T) -> Result<Value, serde_json::Error> {
    let mut params = serde_json::to_value(request)?;
    if let Some(envelope) = params.get_mut("envelope").and_then(Value::as_object_mut) {
        envelope.remove("surface_id");
    }
    Ok(params)
}

fn assert_rejected_code(response: &Value, code: &str) {
    assert_eq!(response["base"]["response_kind"], "rejected");
    assert_eq!(response["errors"][0]["code"], code);
}

fn assert_rejected_field(response: &Value, field: &str) {
    assert_eq!(response["errors"][0]["details"]["field"], field);
}

fn assert_tool_execution_error(error: &volicord_mcp::McpAdapterError, needle: &str) {
    assert!(matches!(
        error,
        volicord_mcp::McpAdapterError::ToolExecution { .. }
    ));
    assert!(
        error.to_string().contains(needle),
        "expected `{}` to contain `{needle}`",
        error
    );
}

fn schema_has_property(schema: &Value, property_name: &str) -> bool {
    match schema {
        Value::Object(object) => {
            object
                .get("properties")
                .and_then(Value::as_object)
                .is_some_and(|properties| properties.contains_key(property_name))
                || object
                    .values()
                    .any(|child| schema_has_property(child, property_name))
        }
        Value::Array(items) => items
            .iter()
            .any(|child| schema_has_property(child, property_name)),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => false,
    }
}

fn envelope_required_fields(schema: &Value) -> Option<Vec<String>> {
    match schema {
        Value::Object(object) => {
            if is_tool_envelope_schema(object) {
                return object
                    .get("required")
                    .and_then(Value::as_array)
                    .map(|required| {
                        required
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect::<Vec<_>>()
                    });
            }
            object.values().find_map(envelope_required_fields)
        }
        Value::Array(items) => items.iter().find_map(envelope_required_fields),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
    }
}

fn is_tool_envelope_schema(object: &serde_json::Map<String, Value>) -> bool {
    object
        .get("properties")
        .and_then(Value::as_object)
        .is_some_and(|properties| {
            [
                "project_id",
                "task_id",
                "request_id",
                "actor_kind",
                "idempotency_key",
                "expected_state_version",
                "dry_run",
                "locale",
            ]
            .iter()
            .all(|field| properties.contains_key(*field))
        })
}

fn assert_replay_surface_foreign_key(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("PRAGMA foreign_key_list(tool_invocations)")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(6)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut matching_ids = rows
        .iter()
        .filter(|(_, _, table, _, _, on_delete)| table == "surfaces" && on_delete == "RESTRICT")
        .map(|(id, _, _, _, _, _)| *id)
        .collect::<BTreeSet<_>>();
    assert_eq!(matching_ids.len(), 1);
    let id = matching_ids.pop_first().expect("matching FK id");
    let mut columns = rows
        .iter()
        .filter(|(candidate_id, _, table, _, _, on_delete)| {
            *candidate_id == id && table == "surfaces" && on_delete == "RESTRICT"
        })
        .cloned()
        .collect::<Vec<_>>();
    columns.sort_by_key(|(_, seq, _, _, _, _)| *seq);
    let actual = columns
        .iter()
        .map(|(_, _, _, from, to, _)| (from.as_str(), to.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            ("project_id", "project_id"),
            ("surface_id", "surface_id"),
            ("surface_instance_id", "surface_instance_id"),
        ]
    );
    Ok(())
}

fn assert_integrity_check_clean(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let result: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    assert_eq!(result, "ok");
    Ok(())
}

fn assert_foreign_key_check_clean(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("PRAGMA foreign_key_check")?;
    let mut rows = stmt.query([])?;
    assert!(rows.next()?.is_none());
    Ok(())
}
