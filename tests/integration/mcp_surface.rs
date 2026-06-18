use std::{
    collections::BTreeSet,
    error::Error,
    ffi::OsString,
    io::{BufReader, Cursor},
};

use harness_core::{CoreService, InvocationContext};
use harness_mcp::{
    public_method_tools, run_stdio, McpAdapter, McpSessionContext, PUBLIC_METHOD_TOOL_NAMES,
};
use harness_store::bootstrap::{register_surface, SurfaceRegistration};
use harness_test_support::core_fixtures::{
    answer_payload, artifact_input_for_handle, CloseTaskFixture, CoreFixture,
    RecordJudgmentFixture, UpdateScopeFixture, UserJudgmentFixture,
};
use harness_types::{
    AccessClass, ChangeUnitOperation, CloseIntent, JudgmentKind, StagedArtifactHandle, SurfaceId,
    SurfaceInstanceId, VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};
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
    let adapter = adapter(&fixture);
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
            "verified_surface_context",
            "access_class",
            "capability_profile",
            "verification_basis",
        ] {
            assert!(
                tool.input_schema["properties"].get(forbidden).is_none(),
                "{} schema should not expose request-level authority field {forbidden}",
                tool.name
            );
        }
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
    let params = serde_json::to_value(fixture.stage_artifact_request(
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
fn one_mcp_session_with_baseline_workflow_surface_runs_full_access_workflow(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_baseline_workflow")?;
    let adapter = adapter(&fixture);

    let status = adapter.call_tool(
        "harness.status",
        serde_json::to_value(fixture.status_request("req_mcp_full_status", None))?,
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
        serde_json::to_value(fixture.intake_request(
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
        serde_json::to_value(fixture.update_scope_request(UpdateScopeFixture {
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
    let change_unit_id = fixture
        .current_change_unit_id(&task_id)?
        .expect("Change Unit should be current");

    let prepare = adapter.call_tool(
        "harness.prepare_write",
        serde_json::to_value(fixture.prepare_write_request(
            "req_mcp_full_prepare",
            "idem_mcp_full_prepare",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ))?,
    )?;
    assert_eq!(prepare.response_value["decision"], "allowed");
    assert_eq!(
        prepare
            .verified_surface
            .as_ref()
            .expect("prepare verified surface")
            .access_class,
        AccessClass::WriteAuthorization
    );

    let stage = adapter.call_tool(
        "harness.stage_artifact",
        serde_json::to_value(fixture.stage_artifact_request(
            "req_mcp_full_stage",
            None,
            false,
            Some(3),
            &task_id,
        ))?,
    )?;
    assert_eq!(stage.response_value["base"]["response_kind"], "result");
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
    let run = adapter.call_tool("harness.record_run", serde_json::to_value(run_request)?)?;
    assert_eq!(run.response_value["base"]["response_kind"], "result");
    assert_eq!(
        run.verified_surface
            .as_ref()
            .expect("run verified surface")
            .access_class,
        AccessClass::RunRecording
    );

    let judgment = adapter.call_tool(
        "harness.request_user_judgment",
        serde_json::to_value(fixture.user_judgment_request(UserJudgmentFixture {
            request_id: "req_mcp_full_judgment",
            idempotency_key: "idem_mcp_full_judgment",
            dry_run: false,
            expected_state_version: Some(4),
            task_id: &task_id,
            change_unit_id: Some(&change_unit_id),
            judgment_kind: JudgmentKind::TechnicalDecision,
        }))?,
    )?;
    assert_eq!(judgment.response_value["base"]["response_kind"], "result");
    let judgment_id = judgment.response_value["user_judgment_ref"]["record_id"]
        .as_str()
        .expect("judgment id")
        .to_owned();

    let recorded = adapter.call_tool(
        "harness.record_user_judgment",
        serde_json::to_value(fixture.record_judgment_request(RecordJudgmentFixture {
            request_id: "req_mcp_full_judgment_record",
            idempotency_key: "idem_mcp_full_judgment_record",
            expected_state_version: Some(5),
            task_id: &task_id,
            user_judgment_id: &judgment_id,
            judgment_kind: JudgmentKind::TechnicalDecision,
            answer: answer_payload(JudgmentKind::TechnicalDecision),
        }))?,
    )?;
    assert_eq!(recorded.response_value["base"]["response_kind"], "result");

    let close_check = adapter.call_tool(
        "harness.close_task",
        serde_json::to_value(fixture.close_task_request(CloseTaskFixture {
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
        close_check
            .verified_surface
            .as_ref()
            .expect("close check verified surface")
            .access_class,
        AccessClass::ReadStatus
    );
    Ok(())
}

#[test]
fn missing_run_recording_grant_blocks_only_record_run() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_missing_run")?;
    fixture.set_surface_local_access(json!({
        "access_class": "read_status",
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
        serde_json::to_value(fixture.intake_request(
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
        serde_json::to_value(fixture.update_scope_request(UpdateScopeFixture {
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
        serde_json::to_value(fixture.prepare_write_request(
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
        serde_json::to_value(fixture.stage_artifact_request(
            "req_missing_run_stage",
            None,
            false,
            Some(3),
            &task_id,
        ))?,
    )?;
    assert_eq!(stage.response_value["base"]["response_kind"], "result");

    let run = adapter.call_tool(
        "harness.record_run",
        serde_json::to_value(fixture.record_run_request(
            "req_missing_run_record",
            "idem_missing_run_record",
            false,
            Some(3),
            &task_id,
            &change_unit_id,
        ))?,
    )?;
    assert_rejected_code(&run.response_value, "LOCAL_ACCESS_MISMATCH");
    assert!(run.verified_surface.is_none());
    Ok(())
}

#[test]
fn missing_write_authorization_grant_blocks_prepare_write() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_missing_write")?;
    fixture.set_surface_local_access(json!({
        "access_class": "read_status",
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
        serde_json::to_value(fixture.intake_request(
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
        serde_json::to_value(fixture.update_scope_request(UpdateScopeFixture {
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

    let prepare = adapter.call_tool(
        "harness.prepare_write",
        serde_json::to_value(fixture.prepare_write_request(
            "req_missing_write_prepare",
            "idem_missing_write_prepare",
            Some(2),
            Some(&task_id),
            Some(&change_unit_id),
        ))?,
    )?;
    assert_rejected_code(&prepare.response_value, "LOCAL_ACCESS_MISMATCH");
    assert!(prepare.verified_surface.is_none());
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
        "access_class": "read_status",
        "authorized_access_classes": ["read_status"],
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    }))?;
    let adapter = adapter(&fixture);
    let check = adapter.call_tool(
        "harness.close_task",
        serde_json::to_value(fixture.close_task_request(CloseTaskFixture {
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

    let mutating_without_core = adapter.call_tool(
        "harness.close_task",
        serde_json::to_value(fixture.close_task_request(CloseTaskFixture {
            request_id: "req_close_intent_complete_no_core",
            idempotency_key: None,
            dry_run: true,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(harness_types::CloseReason::CompletedSelfChecked),
            superseding_task_id: None,
        }))?,
    )?;
    assert_rejected_code(
        &mutating_without_core.response_value,
        "LOCAL_ACCESS_MISMATCH",
    );

    fixture.set_surface_local_access(json!({
        "access_class": "core_mutation",
        "authorized_access_classes": ["core_mutation"],
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    }))?;
    let mutating_with_core = adapter.call_tool(
        "harness.close_task",
        serde_json::to_value(fixture.close_task_request(CloseTaskFixture {
            request_id: "req_close_intent_complete_core",
            idempotency_key: None,
            dry_run: true,
            expected_state_version: None,
            task_id: &task_id,
            intent: CloseIntent::Complete,
            close_reason: Some(harness_types::CloseReason::CompletedSelfChecked),
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
fn mcp_environment_access_class_and_basis_do_not_override_derived_context(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_env_no_elevate")?;
    fixture.set_surface_local_access(json!({
        "access_class": "read_status",
        "authorized_access_classes": ["read_status"],
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    }))?;
    let session = McpSessionContext::from_env(|name| match name {
        "HARNESS_ACCESS_CLASS" => Some(OsString::from("core_mutation")),
        "HARNESS_SURFACE_INSTANCE_ID" => Some(OsString::from(fixture.surface_instance_id())),
        "HARNESS_VERIFICATION_BASIS" => Some(OsString::from("integration_env")),
        _ => None,
    })?;
    let adapter = McpAdapter::new(fixture.runtime_home_path(), session);
    let before = fixture.counts()?;

    let response = adapter.call_tool(
        "harness.status",
        serde_json::to_value(fixture.status_request("req_env_no_elevate", None))?,
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    let verified = response
        .verified_surface
        .as_ref()
        .expect("status should use method-derived read access");
    assert_eq!(verified.access_class, AccessClass::ReadStatus);
    assert_eq!(
        verified.verification_basis,
        "local_admin_registration:mcp_stdio_surface_binding"
    );
    assert!(!verified.verification_basis.contains("integration_env"));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn mcp_environment_basis_does_not_alter_newly_stored_trusted_basis() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_env_basis_storage")?;
    let session = McpSessionContext::from_env(|name| match name {
        "HARNESS_ACCESS_CLASS" => Some(OsString::from("read_status")),
        "HARNESS_SURFACE_INSTANCE_ID" => Some(OsString::from(fixture.surface_instance_id())),
        "HARNESS_VERIFICATION_BASIS" => Some(OsString::from("caller_env_basis")),
        _ => None,
    })?;
    let adapter = McpAdapter::new(fixture.runtime_home_path(), session);

    let intake = adapter.call_tool(
        "harness.intake",
        serde_json::to_value(fixture.intake_request(
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
        serde_json::to_value(fixture.update_scope_request(UpdateScopeFixture {
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
        serde_json::to_value(fixture.prepare_write_request(
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
        "local_admin_registration:mcp_stdio_surface_binding"
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
        "local_admin_registration:mcp_stdio_surface_binding"
    );
    assert!(!authorization_metadata
        .to_string()
        .contains("caller_env_basis"));
    assert!(!replay_basis.contains("caller_env_basis"));
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
        let mut params = serde_json::to_value(fixture.stage_artifact_request(
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
            harness_mcp::McpAdapterError::InvalidParams { .. }
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
fn stdio_invalid_params_returns_protocol_error_without_storage_effect() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp_stdio_invalid")?;
    let adapter = adapter(&fixture);
    let before = fixture.counts()?;
    let input = Cursor::new(
        br#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"harness.status","arguments":{"envelope":{"project_id":"project_fixture","task_id":null,"actor_kind":"agent","surface_id":"surface_fixture","request_id":"req_stdio_invalid","idempotency_key":null,"expected_state_version":null,"dry_run":false,"locale":"en-US"},"include":{"task":true,"pending_user_judgments":true,"write_authority":true,"evidence":true,"close":true,"guarantees":true},"access_class":"core_mutation"}}}
"#
        .to_vec(),
    );
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let response: Value = serde_json::from_slice(&output)?;
    assert_eq!(response["error"]["code"], -32602);
    assert_eq!(response["error"]["message"], "Invalid params");
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn mcp_session_derives_access_per_method_call() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp_access")?;
    let adapter = adapter(&fixture);
    let response = adapter.call_tool(
        "harness.status",
        serde_json::to_value(fixture.status_request("req_status_derived", None))?,
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

    let first = adapter(&fixture).call_tool(
        "harness.prepare_write",
        serde_json::to_value(request.clone())?,
    )?;
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
        "access_class": "core_mutation",
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
        "access_class": "core_mutation",
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
        "access_class": "read_status",
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
fn unknown_surface_instance_is_rejected() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("unknown_instance")?;
    let core = CoreService::new(fixture.runtime_home_path());

    let response = core.status(
        fixture.status_request("req_unknown_instance", None),
        InvocationContext {
            surface_instance_id: Some(SurfaceInstanceId::new("missing_surface_instance")),
            requested_access_class: AccessClass::ReadStatus,
            invocation_binding_basis: VERIFICATION_BASIS_TEST_FIXTURE_BINDING.to_owned(),
        },
    )?;

    assert_rejected_code(&response.response_value, "LOCAL_ACCESS_MISMATCH");
    assert!(response.verified_surface.is_none());
    Ok(())
}

#[test]
fn ambiguous_surface_id_without_usable_default_is_rejected() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("ambiguous_surface")?;
    for surface_instance_id in [
        "surface_instance_ambiguous_a",
        "surface_instance_ambiguous_b",
    ] {
        register_surface(
            fixture.runtime_home_path(),
            SurfaceRegistration {
                project_id: fixture.project_id().to_owned(),
                surface_id: "surface_ambiguous".to_owned(),
                surface_instance_id: surface_instance_id.to_owned(),
                surface_kind: "local_test".to_owned(),
                display_name: None,
                capability_profile_json: json!({}).to_string(),
                local_access_json: json!({
                    "access_class": "read_status",
                    "authorized_access_classes": ["read_status"],
                    "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
                })
                .to_string(),
                metadata_json: "{}".to_owned(),
            },
        )?;
    }
    let core = CoreService::new(fixture.runtime_home_path());
    let mut request = fixture.status_request("req_ambiguous_surface", None);
    request.envelope.surface_id = SurfaceId::new("surface_ambiguous");

    let response = core.status(
        request,
        InvocationContext {
            surface_instance_id: None,
            requested_access_class: AccessClass::ReadStatus,
            invocation_binding_basis: VERIFICATION_BASIS_TEST_FIXTURE_BINDING.to_owned(),
        },
    )?;

    assert_rejected_code(&response.response_value, "LOCAL_ACCESS_MISMATCH");
    assert!(response.verified_surface.is_none());
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

    assert_rejected_code(&response.response_value, "LOCAL_ACCESS_MISMATCH");
    assert!(response.verified_surface.is_none());
    Ok(())
}

#[test]
fn legacy_single_access_class_grant_remains_readable() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("legacy_grant")?;
    fixture.set_surface_local_access(json!({
        "access_class": "read_status"
    }))?;
    let core = CoreService::new(fixture.runtime_home_path());

    let response = core.status(
        fixture.status_request("req_legacy_grant", None),
        invocation(&fixture, AccessClass::ReadStatus),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    let verified = response
        .verified_surface
        .as_ref()
        .expect("legacy grant should create verified surface context");
    assert_eq!(verified.access_class, AccessClass::ReadStatus);
    assert_eq!(
        verified.verification_basis,
        "local_admin_registration:test_fixture_binding"
    );
    Ok(())
}

fn adapter(fixture: &CoreFixture) -> McpAdapter {
    McpAdapter::new(
        fixture.runtime_home_path(),
        McpSessionContext::new()
            .with_surface_instance_id(SurfaceInstanceId::new(fixture.surface_instance_id()))
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING),
    )
}

fn invocation(fixture: &CoreFixture, access_class: AccessClass) -> InvocationContext {
    InvocationContext {
        surface_instance_id: Some(SurfaceInstanceId::new(fixture.surface_instance_id())),
        requested_access_class: access_class,
        invocation_binding_basis: VERIFICATION_BASIS_TEST_FIXTURE_BINDING.to_owned(),
    }
}

fn assert_rejected_code(response: &Value, code: &str) {
    assert_eq!(response["base"]["response_kind"], "rejected");
    assert_eq!(response["errors"][0]["code"], code);
}
