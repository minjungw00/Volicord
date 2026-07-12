use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    ffi::OsString,
    fs,
    io::{BufReader, Cursor},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::adapter::StartupObservationResult;
use crate::local_http::{
    validate_bearer_token_text, validate_local_http_server_config, LOCAL_HTTP_CONTAINER_WARNING,
    LOCAL_HTTP_EXPOSURE_WARNING, LOCAL_HTTP_GENERATED_TOKEN_WARNING,
};
use crate::local_web_consent::{parse_urlencoded, single_param};
use crate::prelude::*;
use crate::stdio::{
    classify_launch_origin, pending_judgment_from_response, percent_encode_query,
    run_stdio_with_env_marker, tool_execution_error_result, McpLaunchOrigin,
};
use crate::{
    routing::McpStorageCapability,
    tool_registry::{
        canonical_tool_examples, mcp_tool_naming_style, mcp_tools_for_mode_and_storage,
        validate_tools_list_json_compatibility, validate_tools_list_schema_compatibility,
        CHECK_CLOSE_MISSING_FINAL_ACCEPTANCE_EXAMPLE_ID, PREPARE_WRITE_SIMPLE_EXAMPLE_ID,
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
        REQUEST_USER_JUDGMENT_FINAL_ACCEPTANCE_EXAMPLE_ID, STATUS_READ_ONLY_EXAMPLE_ID,
        UPDATE_SCOPE_KEEP_CURRENT_EXAMPLE_ID,
    },
};
use volicord_core::CoreBoundary;
use volicord_store::agent_connections::{
    add_connection_project, agent_connection_record, ensure_agent_connection,
    AgentConnectionRegistration, ConnectionProjectRegistration, CONNECTION_MODE_READ_ONLY,
};
use volicord_store::bootstrap::{register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS};
use volicord_store::guards::{
    list_unresolved_unrecorded_changes, upsert_guard_installation, GuardInstallationUpsert,
};
use volicord_store::session_watch::{
    latest_watch_baseline_for_connection, latest_watch_baseline_for_session,
};
use volicord_test_support::core_fixtures::CoreFixture;
use volicord_types::{
    AgentConnectionMode, OperationCategory, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};

use super::*;

#[test]
fn mcp_boundary_wraps_core_boundary() {
    assert_eq!(
        McpAdapterBoundary::new(CoreBoundary::new()).label(),
        "mcp-adapter"
    );
}

#[test]
fn tool_sets_follow_connection_mode_and_exclude_user_only_recording() {
    let workflow = mcp_tools_for_mode(AgentConnectionMode::Workflow);
    let workflow_names = tool_names(&workflow);
    assert_eq!(
        &workflow_names[..PUBLIC_METHOD_TOOL_NAMES.len()],
        PUBLIC_METHOD_TOOL_NAMES
    );
    assert!(workflow_names.contains(&"volicord.request_user_judgment"));
    assert!(workflow_names.contains(&"volicord.reconcile_changes"));
    assert!(workflow_names.contains(&CHECK_CLOSE_TOOL_NAME));
    assert!(workflow_names.contains(&"volicord.close_task"));
    assert!(!workflow_names.contains(&"volicord.record_user_judgment"));
    assert_eq!(
        workflow_names.last().copied(),
        Some(LIST_PROJECTS_TOOL_NAME)
    );

    let read_only = mcp_tools_for_mode(AgentConnectionMode::ReadOnly);
    let read_only_names = tool_names(&read_only);
    assert_eq!(
        read_only_names,
        vec![
            "volicord.status",
            CHECK_CLOSE_TOOL_NAME,
            LIST_PROJECTS_TOOL_NAME
        ]
    );
}

#[test]
fn mcp_visible_schemas_hide_envelope_and_metadata() {
    for tool in public_method_tools() {
        let properties = root_properties(&tool.input_schema);
        let required = root_required_fields(&tool.input_schema);
        assert!(
            properties.contains(&"project_selector".to_owned()),
            "{} should expose the public project selector",
            tool.name
        );
        assert!(
            !required.contains(&"project_selector".to_owned()),
            "{} should not require project selection for single-project connections",
            tool.name
        );
        for forbidden in [
            "envelope",
            "project_id",
            "request_id",
            "idempotency_key",
            "expected_state_version",
            "dry_run",
            "locale",
            "actor_source",
            "operation_category",
            "mode",
            "connection_id",
        ] {
            assert!(
                !properties.contains(&forbidden.to_owned()),
                "{} should not expose MCP-internal field {forbidden}",
                tool.name
            );
        }
        assert!(
            !schema_has_definition(&tool.input_schema, "ToolEnvelope"),
            "{} should not include the internal ToolEnvelope schema",
            tool.name
        );
    }
}

#[test]
fn mcp_tools_list_schema_is_client_compatible() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-tools-list-schema-compatible")?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        request(2, "tools/list", json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let tools = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools/list result should be an array");
    assert_tools_list_json_client_compatible(tools);
    assert_eq!(
        tool_names_from_list_response(&responses[1]).len(),
        tools.len()
    );
    Ok(())
}

#[test]
fn mcp_readonly_degraded_tools_have_valid_schemas() {
    let tools = mcp_tools_for_mode_and_storage(
        AgentConnectionMode::Workflow,
        McpStorageCapability::ReadOnly,
    );

    assert_eq!(
        tool_names(&tools),
        vec![
            STATUS_TOOL_NAME,
            CHECK_CLOSE_TOOL_NAME,
            LIST_PROJECTS_TOOL_NAME
        ]
    );
    assert_eq!(mcp_tool_naming_style(&tools), "dotted_namespace");
    assert_compatible_tool_definitions(&tools);
}

#[test]
fn mcp_workflow_tools_have_valid_schemas() {
    let tools = mcp_tools_for_mode_and_storage(
        AgentConnectionMode::Workflow,
        McpStorageCapability::ReadWrite,
    );
    let mut expected = PUBLIC_METHOD_TOOL_NAMES.to_vec();
    expected.push(LIST_PROJECTS_TOOL_NAME);

    assert_eq!(tool_names(&tools), expected);
    assert_eq!(mcp_tool_naming_style(&tools), "dotted_namespace");
    assert_compatible_tool_definitions(&tools);
}

#[test]
fn mcp_tools_publish_root_output_schemas_and_effect_specific_annotations() {
    for tool in mcp_tools_for_mode_and_storage(
        AgentConnectionMode::Workflow,
        McpStorageCapability::ReadWrite,
    ) {
        assert_eq!(
            tool.output_schema["type"], "object",
            "{} output schema should have an object root",
            tool.name
        );
        assert!(
            schema_has_definition(&tool.output_schema, "McpToolErrorResponse"),
            "{} output schema should cover structured adapter failures",
            tool.name
        );

        let expected_annotations = match tool.name {
            STATUS_TOOL_NAME | CHECK_CLOSE_TOOL_NAME | LIST_PROJECTS_TOOL_NAME => {
                McpToolAnnotations {
                    read_only_hint: true,
                    destructive_hint: false,
                    idempotent_hint: true,
                    open_world_hint: false,
                }
            }
            PREPARE_WRITE_TOOL_NAME
            | STAGE_ARTIFACT_TOOL_NAME
            | RECORD_RUN_TOOL_NAME
            | REQUEST_USER_JUDGMENT_TOOL_NAME => McpToolAnnotations {
                read_only_hint: false,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
            },
            INTAKE_TOOL_NAME
            | UPDATE_SCOPE_TOOL_NAME
            | RECONCILE_CHANGES_TOOL_NAME
            | CLOSE_TASK_TOOL_NAME => McpToolAnnotations {
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: false,
            },
            _ => panic!("missing expected MCP annotations for {}", tool.name),
        };
        assert_eq!(
            tool.annotations, expected_annotations,
            "{} annotations should match its effect boundary",
            tool.name
        );
    }
}

#[test]
fn request_user_judgment_output_schema_covers_elicited_recording_response() {
    let schema = tool_definition(REQUEST_USER_JUDGMENT_TOOL_NAME).output_schema;

    assert!(schema_has_definition(&schema, "RequestUserJudgmentResult"));
    assert!(schema_has_definition(&schema, "RecordUserJudgmentResult"));
}

#[test]
fn common_mcp_omissions_advertise_and_decode_exact_defaults() -> Result<(), Box<dyn Error>> {
    let cases = [
        (
            INTAKE_TOOL_NAME,
            "create_new",
            vec![
                ("initial_context_refs", json!([])),
                ("initial_source_refs", json!([])),
            ],
        ),
        (
            UPDATE_SCOPE_TOOL_NAME,
            UPDATE_SCOPE_KEEP_CURRENT_EXAMPLE_ID,
            vec![
                ("goal_summary", Value::Null),
                ("scope_update", Value::Null),
                ("scope_boundary", Value::Null),
                ("non_goals", Value::Null),
                ("acceptance_criteria", Value::Null),
                ("autonomy_boundary", Value::Null),
                ("baseline_ref", Value::Null),
                ("related_scope_decision_refs", json!([])),
            ],
        ),
        (
            PREPARE_WRITE_TOOL_NAME,
            PREPARE_WRITE_SIMPLE_EXAMPLE_ID,
            vec![
                ("task_id", Value::Null),
                ("change_unit_id", Value::Null),
                ("sensitive_categories", json!([])),
            ],
        ),
        (
            STAGE_ARTIFACT_TOOL_NAME,
            "stage_safe_text",
            vec![
                ("expected_sha256", Value::Null),
                ("expected_size_bytes", Value::Null),
                ("relation_hint", Value::Null),
            ],
        ),
        (
            RECORD_RUN_TOOL_NAME,
            RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
            vec![
                ("run_id", Value::Null),
                ("write_ticket_id", Value::Null),
                ("artifact_inputs", json!([])),
                ("evidence_updates", json!([])),
                ("evidence_observations", json!([])),
                ("close_assessment", Value::Null),
            ],
        ),
        (
            REQUEST_USER_JUDGMENT_TOOL_NAME,
            REQUEST_USER_JUDGMENT_FINAL_ACCEPTANCE_EXAMPLE_ID,
            vec![
                ("change_unit_id", Value::Null),
                ("sensitive_action_scope", Value::Null),
                ("options", Value::Null),
                ("affected_refs", json!([])),
                ("expires_at", Value::Null),
            ],
        ),
    ];

    for (tool_name, example_id, defaults) in cases {
        let tool = tool_definition(tool_name);
        let required = root_required_fields(&tool.input_schema);
        let decoded = decode_mcp_arguments_to_value(
            tool_name,
            canonical_example_value(tool_name, example_id)?,
        )?;
        for (field, expected) in defaults {
            assert!(
                !required.iter().any(|required| required == field),
                "{tool_name}.{field} should be omittable"
            );
            assert_eq!(
                tool.input_schema["properties"][field]["default"], expected,
                "{tool_name}.{field} should advertise its exact omission default"
            );
            assert_eq!(
                decoded[field], expected,
                "{tool_name}.{field} omission should decode to the advertised default"
            );
        }
    }

    assert_eq!(
        root_required_fields(&tool_definition(REQUEST_USER_JUDGMENT_TOOL_NAME).input_schema)
            .into_iter()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "context".to_owned(),
            "judgment_kind".to_owned(),
            "presentation".to_owned(),
            "question".to_owned(),
            "required_for".to_owned(),
            "task_id".to_owned(),
        ])
    );
    Ok(())
}

#[test]
fn mcp_omission_defaults_do_not_change_core_request_required_members() {
    let cases = [
        (
            INTAKE_TOOL_NAME,
            &["initial_context_refs", "initial_source_refs"][..],
        ),
        (
            UPDATE_SCOPE_TOOL_NAME,
            &[
                "goal_summary",
                "scope_update",
                "scope_boundary",
                "non_goals",
                "acceptance_criteria",
                "autonomy_boundary",
                "baseline_ref",
                "related_scope_decision_refs",
            ][..],
        ),
        (
            PREPARE_WRITE_TOOL_NAME,
            &["task_id", "change_unit_id", "sensitive_categories"][..],
        ),
        (
            STAGE_ARTIFACT_TOOL_NAME,
            &["expected_sha256", "expected_size_bytes", "relation_hint"][..],
        ),
        (
            RECORD_RUN_TOOL_NAME,
            &[
                "run_id",
                "write_ticket_id",
                "artifact_inputs",
                "evidence_updates",
                "evidence_observations",
                "close_assessment",
            ][..],
        ),
        (
            REQUEST_USER_JUDGMENT_TOOL_NAME,
            &["change_unit_id", "affected_refs", "expires_at"][..],
        ),
    ];

    for (method_name, fields) in cases {
        let schema = volicord_types::public_request_schema(method_name)
            .expect("public Core request schema should exist");
        let required = root_required_fields(&schema);
        for field in fields {
            assert!(
                required.iter().any(|required| required == field),
                "{method_name}.{field} should remain a required Core request member"
            );
        }
    }
}

#[test]
fn advertised_mcp_examples_cover_supported_branches_and_validate() -> Result<(), Box<dyn Error>> {
    let expected_branches: &[(&str, &[&str])] = &[
        (
            INTAKE_TOOL_NAME,
            &[
                "create_new",
                "resume_active",
                "supersede_active",
                "reject_if_active",
            ],
        ),
        (
            UPDATE_SCOPE_TOOL_NAME,
            &[
                UPDATE_SCOPE_KEEP_CURRENT_EXAMPLE_ID,
                "create_current_change_unit",
                "replace_current_change_unit",
            ],
        ),
        (
            STATUS_TOOL_NAME,
            &["summary_status", STATUS_READ_ONLY_EXAMPLE_ID, "full_status"],
        ),
        (PREPARE_WRITE_TOOL_NAME, &[PREPARE_WRITE_SIMPLE_EXAMPLE_ID]),
        (STAGE_ARTIFACT_TOOL_NAME, &["stage_safe_text"]),
        (
            RECORD_RUN_TOOL_NAME,
            &[
                RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
                "evidence_bearing_record_run",
            ],
        ),
        (
            REQUEST_USER_JUDGMENT_TOOL_NAME,
            &[REQUEST_USER_JUDGMENT_FINAL_ACCEPTANCE_EXAMPLE_ID],
        ),
        (RECONCILE_CHANGES_TOOL_NAME, &["reconcile_current_task"]),
        (
            CHECK_CLOSE_TOOL_NAME,
            &[CHECK_CLOSE_MISSING_FINAL_ACCEPTANCE_EXAMPLE_ID],
        ),
        (
            CLOSE_TASK_TOOL_NAME,
            &["close_complete", "close_cancel", "close_supersede"],
        ),
    ];

    for (tool_name, expected_ids) in expected_branches {
        let tool = tool_definition(tool_name);
        let canonical = canonical_tool_examples(tool_name);
        assert_eq!(
            canonical
                .iter()
                .map(|example| example.id)
                .collect::<Vec<_>>(),
            *expected_ids,
            "{tool_name} should advertise exactly the supported example branches"
        );
        assert!(tool.description.len() <= 160);
        assert!(!tool.description.contains("Required root fields"));
        assert!(!tool.description.contains("{\""));

        let advertised = tool.input_schema["examples"]
            .as_array()
            .unwrap_or_else(|| panic!("{tool_name} should advertise inputSchema.examples"));
        assert_eq!(advertised.len(), canonical.len());
        for (value, example) in advertised.iter().zip(canonical) {
            assert!(!example.description.is_empty());
            assert_eq!(
                value,
                &serde_json::from_str::<Value>(example.arguments_json)?,
                "{} should use its canonical example value",
                example.id
            );
            crate::schema_validation::validate_mcp_tool_arguments(tool_name, value)?;
            decode_mcp_arguments_to_value(tool_name, value.clone())?;
        }
    }
    Ok(())
}

#[test]
fn record_run_discovery_exposes_advisor_no_write_compatibility() -> Result<(), Box<dyn Error>> {
    let tool = tool_definition(RECORD_RUN_TOOL_NAME);
    for compatibility in [
        "advisor/shaping_update",
        "direct/direct",
        "work/shaping_update or implementation",
    ] {
        assert!(
            tool.description.contains(compatibility),
            "record_run description should expose {compatibility}"
        );
    }

    let arguments = canonical_example_value(
        RECORD_RUN_TOOL_NAME,
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
    )?;
    assert_eq!(arguments["kind"], "shaping_update");
    assert!(arguments["task_id"]
        .as_str()
        .is_some_and(|task_id| task_id.contains("advisor")));
    assert_eq!(
        arguments["observed_changes"]["product_file_write_observed"],
        false
    );
    assert_eq!(arguments["observed_changes"]["changed_paths"], json!([]));

    let decoded = decode_mcp_arguments_to_value(RECORD_RUN_TOOL_NAME, arguments)?;
    assert!(decoded["write_ticket_id"].is_null());
    Ok(())
}

#[test]
fn record_run_invalid_observed_changes_reports_expected_shape() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-invalid-record-run-observed-changes")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        RECORD_RUN_TOOL_NAME,
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
    )?;
    arguments["observed_changes"] = json!([]);

    let error = adapter
        .call_tool(RECORD_RUN_TOOL_NAME, arguments)
        .expect_err("invalid observed_changes should fail before Core");
    let response = structured_tool_error(RECORD_RUN_TOOL_NAME, &error);
    let issue = tool_error_issue(&response, "/observed_changes", "MCP_ARGUMENT_TYPE_MISMATCH");
    assert!(issue["message"]
        .as_str()
        .is_some_and(|message| { message.contains("object") && message.contains("array") }));
    Ok(())
}

#[test]
fn record_run_invalid_kind_reports_allowed_values() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-invalid-record-run-kind")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        RECORD_RUN_TOOL_NAME,
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
    )?;
    arguments["kind"] = json!("test");

    let error = adapter
        .call_tool(RECORD_RUN_TOOL_NAME, arguments)
        .expect_err("invalid kind should fail before Core");
    let response = structured_tool_error(RECORD_RUN_TOOL_NAME, &error);
    let issue = tool_error_issue(&response, "/kind", "MCP_ARGUMENT_ENUM_VALUE");
    let message = issue["message"].as_str().expect("enum issue message");
    assert!(message.contains("shaping_update"));
    assert!(message.contains("implementation"));
    assert!(message.contains("direct"));
    Ok(())
}

#[test]
fn record_run_artifact_input_source_uses_public_value_set() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-record-run-artifact-input-source")?;
    let adapter = adapter(&fixture)?;

    for unsupported in ["captured_artifact", "native_artifact"] {
        let before = fixture.counts()?;
        let mut arguments = canonical_example_value(
            RECORD_RUN_TOOL_NAME,
            RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
        )?;
        arguments["artifact_inputs"] = json!([{
            "artifact_input_id": "artifact_input_unsupported",
            "source_kind": unsupported,
            "staged_artifact_handle": null,
            "existing_artifact_ref": null,
            "relation_hint": null,
            "evidence_target": null,
            "expected_sha256": null,
            "expected_size_bytes": null,
            "redaction_state": null
        }]);

        let error = adapter
            .call_tool(RECORD_RUN_TOOL_NAME, arguments)
            .expect_err("unsupported artifact input source should fail before Core");
        let response = structured_tool_error(RECORD_RUN_TOOL_NAME, &error);
        let issue = tool_error_issue(
            &response,
            "/artifact_inputs/0/source_kind",
            "MCP_ARGUMENT_ENUM_VALUE",
        );
        let message = issue["message"].as_str().expect("enum issue message");
        assert!(message.contains(unsupported));
        assert!(message.contains("staged_artifact"));
        assert!(message.contains("existing_artifact"));
        assert_eq!(
            fixture.counts()?,
            before,
            "unsupported artifact input source should not create Core storage effects"
        );
    }
    Ok(())
}

#[test]
fn record_run_invalid_evidence_observation_reports_expected_shape() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-invalid-record-run-evidence-observation")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        RECORD_RUN_TOOL_NAME,
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
    )?;
    arguments["evidence_observations"] = json!([
        {
            "target": {
                "target_kind": "acceptance_criterion",
                "acceptance_criterion_id": "criterion_missing_fields_001"
            }
        }
    ]);

    let error = adapter
        .call_tool(RECORD_RUN_TOOL_NAME, arguments)
        .expect_err("invalid evidence observation should fail before Core");
    let response = structured_tool_error(RECORD_RUN_TOOL_NAME, &error);
    for field in ["source_kind", "assurance_level", "observed_at"] {
        tool_error_issue(
            &response,
            &format!("/evidence_observations/0/{field}"),
            "MCP_ARGUMENT_REQUIRED",
        );
    }
    Ok(())
}

#[test]
fn record_run_evidence_example_expands_nested_omission_defaults() -> Result<(), Box<dyn Error>> {
    let arguments = canonical_example_value(RECORD_RUN_TOOL_NAME, "evidence_bearing_record_run")?;
    crate::schema_validation::validate_mcp_tool_arguments(RECORD_RUN_TOOL_NAME, &arguments)?;
    let decoded = decode_mcp_arguments_to_value(RECORD_RUN_TOOL_NAME, arguments)?;

    let coverage = &decoded["evidence_updates"][0];
    assert_eq!(coverage["supporting_run_refs"], json!([]));
    assert_eq!(coverage["observation_refs"], json!([]));
    assert_eq!(coverage["supporting_artifact_refs"], json!([]));
    assert_eq!(coverage["gap_refs"], json!([]));
    assert!(coverage.get("provenance").is_none());

    let observation = &decoded["evidence_observations"][0];
    assert!(observation["observed_by_actor_source"].is_null());
    assert!(observation["tool_name"].is_null());
    assert!(observation["tool_invocation_id"].is_null());
    assert_eq!(observation["tool_metadata"], json!({}));
    assert_eq!(observation["input_refs"], json!([]));
    assert_eq!(observation["source_refs"], json!([]));
    assert_eq!(observation["output_artifact_refs"], json!([]));
    assert_eq!(observation["limitations"], json!([]));
    Ok(())
}

#[test]
fn record_run_nested_evidence_unknown_fields_fail_before_core() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-invalid-record-run-nested-evidence-field")?;
    let adapter = adapter(&fixture)?;
    let before = fixture.counts()?;
    let mut arguments =
        canonical_example_value(RECORD_RUN_TOOL_NAME, "evidence_bearing_record_run")?;
    arguments["evidence_updates"][0]["unsupported_ref"] = json!("not accepted");
    arguments["evidence_observations"][0]["unsupported_metadata"] = json!(true);

    let error = adapter
        .call_tool(RECORD_RUN_TOOL_NAME, arguments)
        .expect_err("unknown nested evidence fields should fail before Core");
    let response = structured_tool_error(RECORD_RUN_TOOL_NAME, &error);
    tool_error_issue(
        &response,
        "/evidence_updates/0/unsupported_ref",
        "MCP_ARGUMENT_UNKNOWN",
    );
    tool_error_issue(
        &response,
        "/evidence_observations/0/unsupported_metadata",
        "MCP_ARGUMENT_UNKNOWN",
    );
    assert_eq!(
        fixture.counts()?,
        before,
        "nested evidence validation failure should not create Core storage effects"
    );
    Ok(())
}

#[test]
fn record_run_unknown_root_field_reports_expected_arguments() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-invalid-record-run-root-field")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        RECORD_RUN_TOOL_NAME,
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
    )?;
    arguments["unexpected"] = json!("not accepted");

    let error = adapter
        .call_tool(RECORD_RUN_TOOL_NAME, arguments)
        .expect_err("unknown root field should fail before Core");
    let response = structured_tool_error(RECORD_RUN_TOOL_NAME, &error);
    tool_error_issue(&response, "/unexpected", "MCP_ARGUMENT_UNKNOWN");
    Ok(())
}

#[test]
fn request_user_judgment_invalid_options_report_option_id_shape() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-invalid-judgment-options")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        REQUEST_USER_JUDGMENT_TOOL_NAME,
        REQUEST_USER_JUDGMENT_FINAL_ACCEPTANCE_EXAMPLE_ID,
    )?;
    arguments["judgment_kind"] = json!("product_decision");
    arguments["options"] = json!([
        {
            "id": "accept",
            "label": "Accept",
            "description": "Record the user's selected option.",
            "consequence": "The option is recorded for this judgment.",
            "is_default": true
        }
    ]);

    let error = adapter
        .call_tool(REQUEST_USER_JUDGMENT_TOOL_NAME, arguments)
        .expect_err("invalid options should fail before Core");
    let response = structured_tool_error(REQUEST_USER_JUDGMENT_TOOL_NAME, &error);
    tool_error_issue(&response, "/options/0/option_id", "MCP_ARGUMENT_REQUIRED");
    tool_error_issue(&response, "/options/0/id", "MCP_ARGUMENT_UNKNOWN");
    Ok(())
}

#[test]
fn request_user_judgment_invalid_visible_risk_reports_expected_shape() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-invalid-judgment-visible-risk")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        REQUEST_USER_JUDGMENT_TOOL_NAME,
        REQUEST_USER_JUDGMENT_FINAL_ACCEPTANCE_EXAMPLE_ID,
    )?;
    arguments["context"]["visible_risks"] = json!(["plain risk text"]);

    let error = adapter
        .call_tool(REQUEST_USER_JUDGMENT_TOOL_NAME, arguments)
        .expect_err("invalid visible risk should fail before Core");
    let response = structured_tool_error(REQUEST_USER_JUDGMENT_TOOL_NAME, &error);
    tool_error_issue(
        &response,
        "/context/visible_risks/0",
        "MCP_ARGUMENT_TYPE_MISMATCH",
    );
    Ok(())
}

#[test]
fn known_tool_validation_aggregates_independent_issues_without_core_effects(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-aggregated-validation")?;
    let adapter = adapter(&fixture)?;
    let before = fixture.counts()?;
    let arguments = json!({
        "kind": "unsupported",
        "observed_changes": {
            "changed_paths": "not-an-array"
        },
        "unexpected": true
    });

    let error = adapter
        .call_tool(RECORD_RUN_TOOL_NAME, arguments)
        .expect_err("independent argument failures should be rejected together");
    let response = structured_tool_error(RECORD_RUN_TOOL_NAME, &error);

    for field in ["task_id", "change_unit_id", "baseline_ref", "summary"] {
        tool_error_issue(&response, &format!("/{field}"), "MCP_ARGUMENT_REQUIRED");
    }
    tool_error_issue(&response, "/unexpected", "MCP_ARGUMENT_UNKNOWN");
    tool_error_issue(&response, "/kind", "MCP_ARGUMENT_ENUM_VALUE");
    tool_error_issue(
        &response,
        "/observed_changes/changed_paths",
        "MCP_ARGUMENT_TYPE_MISMATCH",
    );
    for field in [
        "product_file_write_observed",
        "sensitive_categories",
        "baseline_ref",
    ] {
        tool_error_issue(
            &response,
            &format!("/observed_changes/{field}"),
            "MCP_ARGUMENT_REQUIRED",
        );
    }
    assert!(response["issues"]
        .as_array()
        .is_some_and(|issues| issues.len() > 8));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn nullable_object_union_prefers_matching_branch_and_keeps_nested_issues(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-nullable-object-validation")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        RECORD_RUN_TOOL_NAME,
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
    )?;
    arguments["close_assessment"] = json!({});

    let error = adapter
        .call_tool(RECORD_RUN_TOOL_NAME, arguments)
        .expect_err("empty close assessment should expose its nested missing fields");
    let response = structured_tool_error(RECORD_RUN_TOOL_NAME, &error);

    for field in [
        "result_summary",
        "result_refs",
        "residual_risks",
        "sensitive_categories",
        "recovery_constraints",
    ] {
        tool_error_issue(
            &response,
            &format!("/close_assessment/{field}"),
            "MCP_ARGUMENT_REQUIRED",
        );
    }
    assert!(response["issues"]
        .as_array()
        .expect("issues")
        .iter()
        .all(|issue| issue["path"] != "/close_assessment"
            || issue["code"] != "MCP_ARGUMENT_TYPE_MISMATCH"));
    Ok(())
}

#[test]
fn decoder_only_failure_is_one_structured_issue_without_core_effects() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-decoder-only-validation")?;
    let adapter = adapter(&fixture)?;
    let before = fixture.counts()?;
    let mut arguments = canonical_example_value(
        REQUEST_USER_JUDGMENT_TOOL_NAME,
        REQUEST_USER_JUDGMENT_FINAL_ACCEPTANCE_EXAMPLE_ID,
    )?;
    arguments["expires_at"] = json!("not-a-timestamp");

    let error = adapter
        .call_tool(REQUEST_USER_JUDGMENT_TOOL_NAME, arguments)
        .expect_err("invalid timestamp format should fail typed decoding");
    let response = structured_tool_error(REQUEST_USER_JUDGMENT_TOOL_NAME, &error);

    assert_eq!(response["issues"].as_array().map(Vec::len), Some(1));
    assert_eq!(response["reported_issue_count"], 1);
    assert_eq!(response["truncated"], false);
    tool_error_issue(&response, "", "MCP_ARGUMENT_DECODE_FAILED");
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[cfg(unix)]
#[test]
fn decoder_failure_precedes_readonly_storage_rejection() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-decoder-before-readonly-precondition")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;
    let before = fixture.counts()?;
    let mut arguments = canonical_example_value(
        REQUEST_USER_JUDGMENT_TOOL_NAME,
        REQUEST_USER_JUDGMENT_FINAL_ACCEPTANCE_EXAMPLE_ID,
    )?;
    arguments["expires_at"] = json!("not-a-timestamp");

    let error = adapter
        .call_tool(REQUEST_USER_JUDGMENT_TOOL_NAME, arguments)
        .expect_err("typed argument decoding should precede storage preconditions");
    let response = structured_tool_error(REQUEST_USER_JUDGMENT_TOOL_NAME, &error);

    assert_eq!(response["code"], "MCP_INVALID_ARGUMENTS");
    tool_error_issue(&response, "", "MCP_ARGUMENT_DECODE_FAILED");
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn mcp_minimal_smoke_tool_lists_hello() {
    let tools = vec![McpToolDefinition {
        name: "hello",
        description: "Minimal diagnostic smoke fixture.",
        input_schema: json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            },
            "required": ["message"],
            "additionalProperties": false
        }),
        annotations: McpToolAnnotations {
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: true,
            open_world_hint: false,
        },
    }];

    assert_eq!(tool_names(&tools), vec!["hello"]);
    assert_eq!(mcp_tool_naming_style(&tools), "plain");
    assert_compatible_tool_definitions(&tools);
}

#[test]
fn dot_free_aliases_are_not_exposed_by_default() {
    for tools in [
        mcp_tools_for_mode_and_storage(
            AgentConnectionMode::Workflow,
            McpStorageCapability::ReadWrite,
        ),
        mcp_tools_for_mode_and_storage(
            AgentConnectionMode::Workflow,
            McpStorageCapability::ReadOnly,
        ),
        mcp_tools_for_mode_and_storage(
            AgentConnectionMode::ReadOnly,
            McpStorageCapability::ReadWrite,
        ),
        mcp_tools_for_mode_and_storage(
            AgentConnectionMode::Workflow,
            McpStorageCapability::Unavailable,
        ),
    ] {
        let names = tool_names(&tools);
        assert_eq!(mcp_tool_naming_style(&tools), "dotted_namespace");
        assert!(
            names.iter().all(|name| name.starts_with("volicord.")),
            "normal tool surface should stay in the volicord dotted namespace: {names:?}"
        );
        assert!(
            !names.iter().any(|name| name.contains("volicord_")),
            "normal tool surface should not expose dot-free aliases: {names:?}"
        );
    }
}

#[test]
fn generated_bearer_token_is_visible_ascii_hex() -> Result<(), Box<dyn Error>> {
    let token = generate_bearer_token()?;

    assert_eq!(token.len(), 64);
    assert!(validate_bearer_token_text(&token).is_ok());
    assert!(token
        .chars()
        .all(|character| matches!(character, '0'..='9' | 'a'..='f')));
    assert!(!token.chars().any(char::is_whitespace));
    Ok(())
}

#[test]
fn generated_bearer_tokens_are_unique_in_small_sample() -> Result<(), Box<dyn Error>> {
    let mut tokens = BTreeSet::new();
    for _ in 0..8 {
        let token = generate_bearer_token()?;
        assert!(
            tokens.insert(token),
            "generated bearer token repeated in a small sanity sample"
        );
    }
    Ok(())
}

#[test]
fn local_http_startup_warnings_keep_generated_tokens_and_container_binds_local() {
    assert!(LOCAL_HTTP_EXPOSURE_WARNING.contains("host loopback"));
    assert!(LOCAL_HTTP_EXPOSURE_WARNING.contains("do not expose"));
    assert!(LOCAL_HTTP_CONTAINER_WARNING.contains("Docker host-loopback publishing"));
    assert!(LOCAL_HTTP_CONTAINER_WARNING.contains("public interfaces"));
    assert!(LOCAL_HTTP_GENERATED_TOKEN_WARNING.contains("local secret"));
    assert!(LOCAL_HTTP_GENERATED_TOKEN_WARNING.contains("Docker host-loopback boundary"));
}

#[test]
fn connection_context_resolves_and_preflight_reports_allowed_project() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-context")?;

    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
    assert_eq!(
        context.connection_internal_id.as_str(),
        fixture.connection_id()
    );
    assert_eq!(context.mode, AgentConnectionMode::Workflow);

    let report = preflight_check(
        |name| {
            if name == "VOLICORD_HOME" {
                Some(fixture.runtime_home_path().as_os_str().to_owned())
            } else {
                None
            }
        },
        fixture.runtime_home_path(),
        fixture.connection_id(),
        None,
    )?;
    assert!(report.contains(&format!("connection_id: {}", fixture.connection_id())));
    assert!(report.contains("mode: workflow"));
    assert!(report.contains("allowed_projects: 1"));
    assert!(report.contains("available_projects: 1"));
    assert!(report.contains("registry_read: passed"));
    assert!(report.contains("project_state_read: passed"));
    assert!(report.contains("project_state_write: passed"));
    assert!(report.contains("startup_observation: recordable"));
    assert!(report.contains("effective_tool_mode: workflow"));
    assert!(report.contains("tools_list_schema_validation: passed"));
    assert!(report.contains("tool_naming_style: dotted_namespace"));
    assert!(report.contains("watcher_status: pending_mcp_start"));
    assert!(report.contains("watcher_coverage_basis: mcp_start"));
    Ok(())
}

#[test]
fn mcp_check_reports_readwrite_effective_tool_mode() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-check-readwrite-mode")?;

    let report = preflight_report_for_fixture(&fixture, Some(fixture.project_id()))?;

    assert_report_line(&report, "registry_read: passed");
    assert_report_line(&report, "project_state_read: passed");
    assert_report_line(&report, "project_state_write: passed");
    assert_report_line(&report, "startup_observation: recordable");
    assert_report_line(&report, "effective_tool_mode: workflow");
    assert_report_line(&report, "tools_list_schema_validation: passed");
    assert_report_line(&report, "tool_naming_style: dotted_namespace");
    assert_report_line(&report, "project[0].state_read: passed");
    assert_report_line(&report, "project[0].state_write: passed");
    Ok(())
}

#[test]
fn mcp_check_does_not_mutate_project_state() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-check-no-mutate")?;
    let before_version = read_only_state_version(&fixture)?;
    let before_sessions = read_only_table_count(&fixture, "agent_sessions")?;
    let before_baselines = read_only_table_count(&fixture, "session_watch_baselines")?;
    let before_invocations = read_only_table_count(&fixture, "tool_invocations")?;

    let report = preflight_report_for_fixture(&fixture, Some(fixture.project_id()))?;

    assert_report_line(&report, "project_state_write: passed");
    assert_eq!(read_only_state_version(&fixture)?, before_version);
    assert_eq!(
        read_only_table_count(&fixture, "agent_sessions")?,
        before_sessions
    );
    assert_eq!(
        read_only_table_count(&fixture, "session_watch_baselines")?,
        before_baselines
    );
    assert_eq!(
        read_only_table_count(&fixture, "tool_invocations")?,
        before_invocations
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_check_reports_readonly_project_state() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-check-readonly-state")?;
    let _guard = make_project_state_readonly(&fixture)?;

    let report = preflight_report_for_fixture(&fixture, Some(fixture.project_id()))?;

    assert_report_line(&report, "registry_read: passed");
    assert_report_line(&report, "project_state_read: passed");
    assert_report_line(&report, "project_state_write: readonly");
    assert_report_line(
        &report,
        "startup_observation: best_effort_skipped_if_readonly",
    );
    assert_report_line(&report, "effective_tool_mode: read_only_degraded");
    assert_report_line(&report, "tools_list_schema_validation: passed");
    assert_report_line(&report, "tool_naming_style: dotted_namespace");
    assert_report_line(&report, "project[0].state_read: passed");
    assert_report_line(&report, "project[0].state_write: readonly");
    assert!(!report.contains("attempt to write a readonly database"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_check_reports_readonly_degraded_tool_mode() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-check-readonly-tool-mode")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

    let report = preflight_report_for_fixture(&fixture, Some(fixture.project_id()))?;
    let names = tool_names(&adapter.tools()?);

    assert_report_line(&report, "effective_tool_mode: read_only_degraded");
    assert!(names.contains(&STATUS_TOOL_NAME));
    assert!(names.contains(&CHECK_CLOSE_TOOL_NAME));
    assert!(names.contains(&LIST_PROJECTS_TOOL_NAME));
    assert!(!names.contains(&INTAKE_TOOL_NAME));
    Ok(())
}

#[test]
fn direct_startup_watch_records_legacy_observation_without_managed_lifecycle(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-direct-startup-watch")?;
    let adapter = adapter(&fixture)?;

    assert_eq!(
        adapter.startup_session_watch_observation_best_effort("session_direct_startup"),
        StartupObservationResult::Recorded
    );

    let baseline = latest_watch_baseline_for_connection(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
    )?
    .expect("direct startup watch should create a watch baseline");
    assert_eq!(baseline.status, "active");
    let metadata: Value = serde_json::from_str(&baseline.metadata_json)?;
    assert_eq!(metadata["coverage_basis"], "mcp_start");
    assert!(metadata.get("launch_origin").is_none());
    assert!(metadata.get("lifecycle_events").is_none());
    assert!(metadata.get("partial_coverage_warning").is_none());
    assert_eq!(
        metadata["scan_summary"]["not_full_filesystem_monitoring"],
        true
    );
    assert_eq!(metadata["scan_summary"]["follows_symlinks"], false);
    Ok(())
}

#[test]
fn mcp_launch_origin_classifies_verification_managed_manual_and_invalid() {
    assert_eq!(McpLaunchOrigin::Unknown.as_str(), "unknown");
    assert_eq!(
        classify_launch_origin(
            |name| (name == "VOLICORD_MCP_VERIFICATION").then(|| OsString::from("1")),
            "conn_alpha",
            Some("project_alpha"),
        ),
        McpLaunchOrigin::CliVerification
    );
    assert_eq!(
        classify_launch_origin(
            |name| match name {
                "VOLICORD_MCP_LAUNCH" => Some(OsString::from("managed_host")),
                "VOLICORD_MCP_HOST" => Some(OsString::from("codex")),
                "VOLICORD_MCP_CONNECTION_ID" => Some(OsString::from("conn_alpha")),
                "VOLICORD_MCP_PROJECT_ID" => Some(OsString::from("project_alpha")),
                _ => None,
            },
            "conn_alpha",
            Some("project_alpha"),
        ),
        McpLaunchOrigin::ManagedHost
    );
    assert_eq!(
        classify_launch_origin(|_| None, "conn_alpha", Some("project_alpha")),
        McpLaunchOrigin::ManualCli
    );
    assert_eq!(
        classify_launch_origin(
            |name| match name {
                "VOLICORD_MCP_LAUNCH" => Some(OsString::from("managed_host")),
                "VOLICORD_MCP_HOST" => Some(OsString::from("codex")),
                "VOLICORD_MCP_CONNECTION_ID" => Some(OsString::from("conn_beta")),
                "VOLICORD_MCP_PROJECT_ID" => Some(OsString::from("project_alpha")),
                _ => None,
            },
            "conn_alpha",
            Some("project_alpha"),
        ),
        McpLaunchOrigin::InvalidManagedMarker
    );
}

#[test]
fn managed_stdio_launch_records_host_runtime_observation() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-managed-watch")?;
    let adapter = project_bound_adapter(&fixture)?;
    let input = Cursor::new(Vec::<u8>::new());
    let mut output = Vec::new();

    run_stdio_with_env_marker(
        adapter,
        BufReader::new(input),
        &mut output,
        |name| match name {
            "VOLICORD_MCP_LAUNCH" => Some(OsString::from("managed_host")),
            "VOLICORD_MCP_HOST" => Some(OsString::from("codex")),
            "VOLICORD_MCP_CONNECTION_ID" => Some(OsString::from(fixture.connection_id())),
            "VOLICORD_MCP_PROJECT_ID" => Some(OsString::from(fixture.project_id())),
            _ => None,
        },
    )?;

    assert!(output.is_empty());
    let baseline = latest_watch_baseline_for_connection(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
    )?
    .expect("managed stdio startup should create a watch baseline");
    let metadata: Value = serde_json::from_str(&baseline.metadata_json)?;
    assert_eq!(metadata["launch_origin"], "managed_host");
    assert_eq!(metadata["host_kind"], "codex");
    assert_eq!(metadata["connection_id"], fixture.connection_id());
    assert_eq!(metadata["project_id"], fixture.project_id());
    let startup = lifecycle_event(&metadata, "managed_host_startup");
    assert_eq!(startup["launch_origin"], "managed_host");
    assert_eq!(startup["host_kind"], "codex");
    assert_eq!(startup["connection_id"], fixture.connection_id());
    assert_eq!(startup["project_id"], fixture.project_id());
    assert_eq!(startup["storage_capability"], "read_write");
    assert_eq!(startup["effective_tool_mode"], "workflow");
    assert!(startup["timestamp"].is_string());
    Ok(())
}

#[test]
fn managed_stdio_tools_list_records_lifecycle_observation() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-managed-tools-list-watch")?;
    let adapter = project_bound_adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        request(2, "tools/list", json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio_with_env_marker(
        adapter,
        BufReader::new(input),
        &mut output,
        |name| match name {
            "VOLICORD_MCP_LAUNCH" => Some(OsString::from("managed_host")),
            "VOLICORD_MCP_HOST" => Some(OsString::from("codex")),
            "VOLICORD_MCP_CONNECTION_ID" => Some(OsString::from(fixture.connection_id())),
            "VOLICORD_MCP_PROJECT_ID" => Some(OsString::from(fixture.project_id())),
            _ => None,
        },
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    assert!(responses[1]["result"]["tools"].is_array());
    let baseline = latest_watch_baseline_for_connection(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
    )?
    .expect("managed tools/list should update the lifecycle baseline");
    let metadata: Value = serde_json::from_str(&baseline.metadata_json)?;
    assert_eq!(
        lifecycle_event_names(&metadata),
        vec![
            "managed_host_startup",
            "managed_host_initialize_response",
            "managed_host_tools_list",
        ]
    );
    let tools_list = lifecycle_event(&metadata, "managed_host_tools_list");
    assert_eq!(tools_list["storage_capability"], "read_write");
    assert_eq!(tools_list["effective_tool_mode"], "workflow");
    Ok(())
}

#[test]
fn managed_stdio_tool_call_records_lifecycle_observation() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-managed-tool-call-watch")?;
    let adapter = project_bound_adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(2, "volicord.status", json!({ "detail": "workflow" })),
    ])?);
    let mut output = Vec::new();

    run_stdio_with_env_marker(
        adapter,
        BufReader::new(input),
        &mut output,
        |name| match name {
            "VOLICORD_MCP_LAUNCH" => Some(OsString::from("managed_host")),
            "VOLICORD_MCP_HOST" => Some(OsString::from("codex")),
            "VOLICORD_MCP_CONNECTION_ID" => Some(OsString::from(fixture.connection_id())),
            "VOLICORD_MCP_PROJECT_ID" => Some(OsString::from(fixture.project_id())),
            _ => None,
        },
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    let status = volicord_response_from_tool(&responses[1])?;
    assert_eq!(status["base"]["response_kind"], "result");
    let baseline = latest_watch_baseline_for_connection(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
    )?
    .expect("managed tools/call should update the lifecycle baseline");
    let metadata: Value = serde_json::from_str(&baseline.metadata_json)?;
    assert!(lifecycle_event_names(&metadata).contains(&"managed_host_tool_call".to_owned()));
    assert!(
        lifecycle_event_names(&metadata).contains(&"managed_host_tool_call_completed".to_owned())
    );
    let tool_call = lifecycle_event(&metadata, "managed_host_tool_call");
    assert_eq!(tool_call["tool_name"], "volicord.status");
    assert_eq!(tool_call["storage_capability"], "read_write");
    assert_eq!(tool_call["effective_tool_mode"], "workflow");
    Ok(())
}

#[test]
fn manual_stdio_launch_does_not_create_host_runtime_observation() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-manual-watch-skip")?;
    let adapter = project_bound_adapter(&fixture)?;
    let input = Cursor::new(Vec::<u8>::new());
    let mut output = Vec::new();

    run_stdio_with_env_marker(adapter, BufReader::new(input), &mut output, |_| None)?;

    assert!(output.is_empty());
    assert!(latest_watch_baseline_for_connection(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
    )?
    .is_none());
    Ok(())
}

#[test]
fn invalid_managed_marker_launch_does_not_create_host_runtime_observation(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-invalid-marker-watch-skip")?;
    let adapter = project_bound_adapter(&fixture)?;
    let input = Cursor::new(Vec::<u8>::new());
    let mut output = Vec::new();

    run_stdio_with_env_marker(
        adapter,
        BufReader::new(input),
        &mut output,
        |name| match name {
            "VOLICORD_MCP_LAUNCH" => Some(OsString::from("managed_host")),
            "VOLICORD_MCP_HOST" => Some(OsString::from("codex")),
            "VOLICORD_MCP_CONNECTION_ID" => Some(OsString::from("conn_wrong")),
            "VOLICORD_MCP_PROJECT_ID" => Some(OsString::from(fixture.project_id())),
            _ => None,
        },
    )?;

    assert!(output.is_empty());
    assert!(latest_watch_baseline_for_connection(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
    )?
    .is_none());
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_workflow_connection_degrades_tool_list_when_storage_readonly() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-readonly-tools-list")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

    let report = preflight_check(
        |name| {
            if name == "VOLICORD_HOME" {
                Some(fixture.runtime_home_path().as_os_str().to_owned())
            } else {
                None
            }
        },
        fixture.runtime_home_path(),
        fixture.connection_id(),
        Some(fixture.project_id()),
    )?;
    assert!(report.contains("available_projects: 1"));
    assert!(report.contains("project[0].available: true"));

    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        request(2, "tools/list", json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    assert_eq!(
        responses[0]["result"]["protocolVersion"],
        json!(SUPPORTED_PROTOCOL_VERSION)
    );
    let names = tool_names_from_list_response(&responses[1]);
    assert_eq!(
        names,
        vec![
            STATUS_TOOL_NAME,
            CHECK_CLOSE_TOOL_NAME,
            LIST_PROJECTS_TOOL_NAME
        ]
    );
    assert!(responses[1].get("error").is_none());
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_readonly_storage_exposes_status_and_list_projects() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readonly-exposes-read-tools")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

    let names = tool_names(&adapter.tools()?);

    assert!(names.contains(&STATUS_TOOL_NAME));
    assert!(names.contains(&LIST_PROJECTS_TOOL_NAME));
    assert!(names.contains(&CHECK_CLOSE_TOOL_NAME));
    assert!(!names.contains(&INTAKE_TOOL_NAME));
    assert!(!names.contains(&RECORD_RUN_TOOL_NAME));
    assert!(!names.contains(&CLOSE_TASK_TOOL_NAME));
    Ok(())
}

#[test]
fn mcp_readwrite_storage_exposes_workflow_tools() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readwrite-exposes-workflow")?;
    let adapter = adapter(&fixture)?;

    let mut expected = PUBLIC_METHOD_TOOL_NAMES.to_vec();
    expected.push(LIST_PROJECTS_TOOL_NAME);

    assert_eq!(tool_names(&adapter.tools()?), expected);
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_status_succeeds_with_readonly_storage() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readonly-status")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

    let response = adapter.call_tool_for_session(
        STATUS_TOOL_NAME,
        json!({ "detail": "workflow" }),
        Some("session_readonly_status"),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(response.response_value["base"]["effect_kind"], "read_only");
    assert_eq!(
        response.response_value["status_summary"],
        "No current Task is selected."
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_status_does_not_advance_state_version() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readonly-status-version")?;
    let before_version = read_only_state_version(&fixture)?;
    let before_sessions = read_only_table_count(&fixture, "agent_sessions")?;
    let before_baselines = read_only_table_count(&fixture, "session_watch_baselines")?;
    let before_invocations = read_only_table_count(&fixture, "tool_invocations")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

    let response = adapter.call_tool_for_session(
        STATUS_TOOL_NAME,
        json!({ "detail": "full" }),
        Some("session_readonly_status_no_write"),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(read_only_state_version(&fixture)?, before_version);
    assert_eq!(
        read_only_table_count(&fixture, "agent_sessions")?,
        before_sessions
    );
    assert_eq!(
        read_only_table_count(&fixture, "session_watch_baselines")?,
        before_baselines
    );
    assert_eq!(
        read_only_table_count(&fixture, "tool_invocations")?,
        before_invocations
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_write_tool_returns_unavailable_when_storage_readonly() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readonly-write-reject")?;
    let before_version = read_only_state_version(&fixture)?;
    let before_events = read_only_table_count(&fixture, "task_events")?;
    let before_invocations = read_only_table_count(&fixture, "tool_invocations")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

    let response = adapter.call_tool_for_session(
        INTAKE_TOOL_NAME,
        intake_args(None),
        Some("session_readonly_write_reject"),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "rejected");
    assert_eq!(
        response.response_value["errors"][0]["code"],
        "MCP_UNAVAILABLE"
    );
    assert_eq!(
        response.response_value["errors"][0]["message"],
        "Volicord project state is not writable in the current MCP host environment."
    );
    assert_eq!(
        response.response_value["errors"][0]["details"]["storage_capability"],
        "read_only"
    );
    assert_eq!(read_only_state_version(&fixture)?, before_version);
    assert_eq!(
        read_only_table_count(&fixture, "task_events")?,
        before_events
    );
    assert_eq!(
        read_only_table_count(&fixture, "tool_invocations")?,
        before_invocations
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_startup_observation_write_failure_is_nonfatal() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readonly-startup-observation")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

    let result = adapter.startup_session_watch_observation_best_effort("session_readonly_startup");

    assert!(
        matches!(
            result,
            StartupObservationResult::SkippedReadonlyStorage
                | StartupObservationResult::FailedButNonfatal { .. }
        ),
        "readonly startup observation should be degraded, got {result:?}"
    );
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        request(2, "tools/list", json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    assert!(responses[1]["result"]["tools"].is_array());
    Ok(())
}

#[test]
fn mcp_verification_probe_does_not_create_host_runtime_observation() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-verification-watch-skip")?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(Vec::<u8>::new());
    let mut output = Vec::new();

    run_stdio_with_env_marker(adapter, BufReader::new(input), &mut output, |name| {
        (name == "VOLICORD_MCP_VERIFICATION").then(|| OsString::from("1"))
    })?;

    assert!(output.is_empty());
    let conn = fixture.conn()?;
    let agent_sessions: i64 = conn.query_row(
        "SELECT COUNT(*) FROM agent_sessions WHERE project_id = ?1",
        [fixture.project_id()],
        |row| row.get(0),
    )?;
    let watch_baselines: i64 = conn.query_row(
        "SELECT COUNT(*) FROM session_watch_baselines WHERE project_id = ?1",
        [fixture.project_id()],
        |row| row.get(0),
    )?;
    assert_eq!(agent_sessions, 0);
    assert_eq!(watch_baselines, 0);
    Ok(())
}

#[test]
fn adapter_auto_selects_single_project_and_injects_connection_invocation(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-auto-select")?;
    let adapter = adapter(&fixture)?;

    let response = adapter.call_tool("volicord.status", json!({}))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    let verified = response
        .verified_invocation
        .expect("Core should verify adapter invocation");
    assert_eq!(verified.project_id.as_str(), fixture.project_id());
    assert_eq!(verified.actor_source.to_string(), fixture.actor_source());
    assert_eq!(verified.operation_category, OperationCategory::Read);
    Ok(())
}

#[test]
fn multi_project_session_reports_pending_project_selection() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-watch-pending")?;
    add_allowed_project(&fixture, "project_watch_pending_other")?;
    let adapter = adapter(&fixture)?;

    let result =
        adapter.call_adapter_tool(LIST_PROJECTS_TOOL_NAME, json!({}), Some("session_pending"))?;

    assert_eq!(result["watcher_status"], "pending_project_selection");
    assert!(result["watcher_baseline_created_at"].is_null());
    assert!(result["watcher_coverage_start_at"].is_null());
    assert!(result["watcher_coverage_basis"].is_null());
    assert!(result["watcher_partial_coverage_warning"]
        .as_str()
        .unwrap_or_default()
        .contains("project_selector"));
    Ok(())
}

#[test]
fn first_project_selection_creates_partial_coverage_baseline() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-watch-first-selection")?;
    add_allowed_project(&fixture, "project_watch_first_selection_other")?;
    let adapter = adapter(&fixture)?;
    let session_id = "session_first_project_selection";

    let response = adapter.call_tool_for_session(
        "volicord.status",
        json!({ "project_selector": fixture.project_id() }),
        Some(session_id),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    let baseline = latest_watch_baseline_for_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        session_id,
    )?
    .expect("first explicit project selection should create a baseline");
    let metadata: Value = serde_json::from_str(&baseline.metadata_json)?;
    assert_eq!(metadata["coverage_basis"], "first_project_selection");
    assert!(metadata["partial_coverage_warning"]
        .as_str()
        .unwrap_or_default()
        .contains("project selection"));
    Ok(())
}

#[test]
fn project_bound_early_edit_is_detected_on_first_close_attempt() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-watch-early-edit")?;
    let adapter = adapter(&fixture)?;
    let (task_id, _) = create_task(&adapter)?;
    let session_id = "session_project_bound_early_edit";
    assert_eq!(
        adapter.startup_session_watch_observation_best_effort(session_id),
        StartupObservationResult::Recorded
    );
    write_product_file(&fixture, "src/early.txt", "changed before first method\n")?;

    let response = adapter.call_tool_for_session(
        CLOSE_TASK_TOOL_NAME,
        json!({
            "task_id": task_id,
            "intent": "complete",
            "close_reason": "completed_self_checked"
        }),
        Some(session_id),
    )?;

    assert_eq!(
        response.response_value["guard_health"]["session_watch_coverage_basis"],
        "mcp_start"
    );
    assert_eq!(
        response.response_value["guard_health"]["session_watch_partial_coverage_warning"],
        Value::Null
    );
    assert_eq!(
        response.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        1
    );
    let changes = list_unresolved_unrecorded_changes(
        fixture.runtime_home_path(),
        fixture.project_id(),
        Some(fixture.connection_id()),
    )?;
    assert_eq!(changes.len(), 1);
    assert!(!changes[0]
        .detection_json
        .contains("changed before first method"));
    Ok(())
}

#[test]
fn edit_before_project_selection_is_reported_outside_coverage() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-watch-before-selection")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, _) = create_task(&setup_adapter)?;
    add_allowed_project(&fixture, "project_watch_before_selection_other")?;
    let adapter = adapter(&fixture)?;
    let session_id = "session_before_project_selection";
    write_product_file(&fixture, "src/before-selection.txt", "before selection\n")?;

    let response = adapter.call_tool_for_session(
        CHECK_CLOSE_TOOL_NAME,
        json!({
            "project_selector": fixture.project_id(),
            "task_id": task_id
        }),
        Some(session_id),
    )?;

    assert_eq!(
        response.response_value["guard_health"]["session_watch_coverage_basis"],
        "first_project_selection"
    );
    assert!(
        response.response_value["guard_health"]["session_watch_partial_coverage_warning"]
            .as_str()
            .unwrap_or_default()
            .contains("project selection")
    );
    assert_eq!(
        response.response_value["guard_health"]["unresolved_unrecorded_change_count"],
        0
    );
    let changes = list_unresolved_unrecorded_changes(
        fixture.runtime_home_path(),
        fixture.project_id(),
        Some(fixture.connection_id()),
    )?;
    assert!(changes.is_empty());
    Ok(())
}

#[test]
fn read_only_mode_rejects_agent_workflow_calls_before_core() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-read-only")?;
    set_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;
    let adapter = adapter(&fixture)?;
    let before = fixture.counts()?;

    let error = adapter
        .call_tool(
            "volicord.intake",
            json!({
                "plain_language_request": "Exercise read-only rejection.",
                "requested_mode": "work",
                "resume_policy": "create_new",
                "initial_scope": {
                    "boundary": "Read-only rejection.",
                    "non_goals": [],
                    "acceptance_criteria": [{
                        "statement": "No Core mutation occurs.",
                        "evidence_requirement": "required"
                    }]
                },
                "initial_context_refs": []
            }),
        )
        .expect_err("read_only should reject agent workflow calls");

    assert!(error.to_string().contains("mode read_only"));
    assert!(error.to_string().contains("agent_workflow"));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn mcp_tools_list_is_available_after_initialize() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-list-after-init")?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        request(2, "tools/list", json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    assert_eq!(
        responses[0]["result"]["protocolVersion"],
        json!(SUPPORTED_PROTOCOL_VERSION)
    );
    let names = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools should be an array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();
    let mut expected = PUBLIC_METHOD_TOOL_NAMES.to_vec();
    expected.push(LIST_PROJECTS_TOOL_NAME);
    assert_eq!(names, expected);
    Ok(())
}

#[test]
fn mcp_tools_list_remains_available_after_initialized_notification() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-mode")?;
    set_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(
        br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"volicord-unit-test","version":"0.0.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
"#
        .to_vec(),
    );
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    let names = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools should be an array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "volicord.status",
            CHECK_CLOSE_TOOL_NAME,
            LIST_PROJECTS_TOOL_NAME
        ]
    );
    Ok(())
}

#[test]
fn mcp_tools_call_requires_initialized_notification() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-call-requires-ready")?;
    let before = fixture.counts()?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        request(2, "tools/list", json!({})),
        tools_call(3, "volicord.intake", intake_args(None)),
        initialized_notification(),
        tools_call(4, "volicord.status", json!({ "detail": "workflow" })),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 4);
    assert!(responses[1]["result"]["tools"].is_array());
    assert_eq!(responses[2]["error"]["code"], -32600);
    assert!(responses[2]["error"]["data"]
        .as_str()
        .unwrap_or_default()
        .contains("notifications/initialized"));
    let status = volicord_response_from_tool(&responses[3])?;
    assert_eq!(status["base"]["response_kind"], "result");
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn stdio_aggregated_validation_error_is_structured_and_has_no_core_effects(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-aggregated-validation")?;
    let before = fixture.counts()?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            RECORD_RUN_TOOL_NAME,
            json!({
                "kind": "unsupported",
                "observed_changes": {},
                "unexpected": true
            }),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let error = structured_error_result(&responses[1]["result"]);
    assert_eq!(error["code"], "MCP_INVALID_ARGUMENTS");
    assert_eq!(error["tool_name"], RECORD_RUN_TOOL_NAME);
    assert_eq!(error["retryable"], true);
    tool_error_issue(&error, "/task_id", "MCP_ARGUMENT_REQUIRED");
    tool_error_issue(&error, "/unexpected", "MCP_ARGUMENT_UNKNOWN");
    tool_error_issue(&error, "/kind", "MCP_ARGUMENT_ENUM_VALUE");
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn known_tool_validation_error_bounds_issue_fields_and_complete_result(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-bounded-aggregate-validation")?;
    let before = fixture.counts()?;
    let adapter = adapter(&fixture)?;
    let mut arguments = Map::new();
    arguments.insert("kind".to_owned(), Value::String("x".repeat(16 * 1024)));
    for index in 0..(MAX_VALIDATION_ISSUES * 3) {
        arguments.insert(
            format!("unexpected_{index}_{}", "\0".repeat(1024)),
            Value::Bool(true),
        );
    }

    let error = adapter
        .call_tool(RECORD_RUN_TOOL_NAME, Value::Object(arguments))
        .expect_err("pathological invalid arguments should be rejected");
    let result = tool_execution_error_result(RECORD_RUN_TOOL_NAME, &error);
    let response = structured_error_result(&result);
    let issues = response["issues"].as_array().expect("issues");

    assert!(!issues.is_empty());
    assert!(
        issues.len() < MAX_VALIDATION_ISSUES,
        "escape-heavy issues should exercise the whole-result byte cap"
    );
    assert_eq!(response["reported_issue_count"], issues.len());
    assert_eq!(response["truncated"], true);
    for issue in issues {
        assert!(issue["path"].as_str().expect("issue path").len() <= MAX_MCP_TOOL_ISSUE_PATH_BYTES);
        assert!(
            issue["message"].as_str().expect("issue message").len()
                <= MAX_MCP_TOOL_ISSUE_MESSAGE_BYTES
        );
    }
    assert!(
        serde_json::to_vec(&result)?.len() <= MAX_MCP_TOOL_ERROR_RESULT_BYTES,
        "complete CallToolResult should honor the compact JSON byte limit"
    );
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn stdio_adapter_precondition_error_uses_requested_tool_and_structured_flags(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-adapter-precondition")?;
    add_allowed_project(&fixture, "project_stdio_precondition_other")?;
    let before = fixture.counts()?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(2, STATUS_TOOL_NAME, json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let error = structured_error_result(&responses[1]["result"]);
    assert_eq!(error["code"], "MCP_ADAPTER_PRECONDITION_FAILED");
    assert_eq!(error["tool_name"], STATUS_TOOL_NAME);
    assert_eq!(error["retryable"], false);
    assert_eq!(error["reported_issue_count"], 1);
    assert_eq!(error["truncated"], false);
    tool_error_issue(
        &error,
        "/project_selector",
        "MCP_ADAPTER_PRECONDITION_FAILED",
    );
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn stdio_elicitation_accept_records_user_judgment() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-accept")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            product_judgment_args(&fixture, &task_id, state_version),
        ),
        elicitation_accept("keep", None),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 3);
    assert_eq!(values[1]["method"], ELICITATION_CREATE_METHOD);
    assert_eq!(values[1]["id"], "elicit_user_judgment_1");
    assert_eq!(
        values[1]["params"]["requestedSchema"]["properties"]["selected_option_id"]["enum"][0],
        "keep"
    );
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(response["base"]["response_kind"], "result");
    assert_eq!(response["user_judgment"]["status"], "resolved");
    assert_eq!(
        response["user_judgment"]["resolution"]["resolved_by_actor_source"],
        "local_user"
    );
    assert_eq!(
        response["user_judgment"]["resolution"]["selected_option_id"],
        "keep"
    );
    assert_eq!(
        stored_resolution_basis(&fixture, &task_id, &response)?,
        VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
    );
    Ok(())
}

#[test]
fn stdio_elicitation_decline_records_rejected_authority_judgment() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-decline")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            authority_judgment_args(&fixture, &task_id, state_version),
        ),
        elicitation_action("decline"),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values[1]["method"], ELICITATION_CREATE_METHOD);
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(response["user_judgment"]["status"], "resolved");
    assert_eq!(
        response["user_judgment"]["resolution"]["selected_option_id"],
        "reject"
    );
    assert_eq!(
        response["user_judgment"]["resolution"]["resolution_outcome"],
        "rejected"
    );
    assert_eq!(
        stored_resolution_basis(&fixture, &task_id, &response)?,
        VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
    );
    Ok(())
}

#[test]
fn stdio_elicitation_accept_can_record_deferred_judgment() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-defer")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            authority_judgment_args(&fixture, &task_id, state_version),
        ),
        elicitation_accept("defer", Some("Not enough context yet.")),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(response["user_judgment"]["status"], "resolved");
    assert_eq!(
        response["user_judgment"]["resolution"]["selected_option_id"],
        "defer"
    );
    assert_eq!(
        response["user_judgment"]["resolution"]["resolution_outcome"],
        "deferred"
    );
    assert_eq!(
        response["user_judgment"]["resolution"]["note"],
        "Not enough context yet."
    );
    Ok(())
}

#[test]
fn stdio_elicitation_cancel_leaves_judgment_pending() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-cancel")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            product_judgment_args(&fixture, &task_id, state_version),
        ),
        elicitation_action("cancel"),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(response["user_judgment"]["status"], "pending");
    assert!(values[2]["result"]["content"][1]["text"]
        .as_str()
        .expect("extra text")
        .contains("remains pending"));
    let record = stored_judgment_record(&fixture, &task_id, &response)?;
    assert_eq!(record.status, "pending");
    assert!(record.resolved_verification_basis.is_none());
    Ok(())
}

#[test]
fn stdio_elicitation_invalid_response_leaves_judgment_pending() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-invalid")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            product_judgment_args(&fixture, &task_id, state_version),
        ),
        elicitation_accept("not_an_option", None),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(response["user_judgment"]["status"], "pending");
    assert!(values[2]["result"]["content"][1]["text"]
        .as_str()
        .expect("extra text")
        .contains("unknown option_id"));
    let record = stored_judgment_record(&fixture, &task_id, &response)?;
    assert_eq!(record.status, "pending");
    Ok(())
}

#[test]
fn stdio_without_elicitation_capability_returns_cli_recovery_when_prompt_capture_unavailable(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-unavailable")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            product_judgment_args(&fixture, &task_id, state_version),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 2);
    let response = volicord_response_from_tool(&values[1])?;
    assert_eq!(response["user_judgment"]["status"], "pending");
    assert_eq!(
        response["inbox_item"]["preferred_capture_path"]["kind"],
        "cli"
    );
    let availability = &response["inbox_item"]["answer_path_availability"];
    assert_eq!(
        channel_path(availability, "mcp_elicitation")["available"],
        false
    );
    assert_eq!(
        channel_path(availability, "local_web_consent")["available"],
        false
    );
    assert_eq!(channel_path(availability, "cli")["available"], true);
    assert!(response["inbox_item"]["preferred_capture_path"]["command"]
        .as_str()
        .expect("CLI fallback command should be present")
        .contains("volicord inbox answer"));
    let fallback = values[1]["result"]["content"][1]["text"]
        .as_str()
        .expect("fallback text");
    assert!(fallback.contains("Host prompt input is unavailable"));
    assert!(fallback.contains("CLI inbox path"));
    assert!(fallback.contains("volicord inbox answer"));
    assert!(!fallback.contains("Volicord: answer J-1 1 #"));
    Ok(())
}

#[test]
fn stdio_without_elicitation_capability_returns_chat_capture_when_configured(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-chat-capture")?;
    install_prompt_capture_guard(&fixture)?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            product_judgment_args(&fixture, &task_id, state_version),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 2);
    let response = volicord_response_from_tool(&values[1])?;
    assert_eq!(response["user_judgment"]["status"], "pending");
    assert_eq!(
        response["inbox_item"]["preferred_capture_path"]["kind"],
        "prompt_capture"
    );
    let availability = &response["inbox_item"]["answer_path_availability"];
    assert_eq!(
        channel_path(availability, "mcp_elicitation")["available"],
        false
    );
    assert_eq!(
        channel_path(availability, "prompt_capture")["available"],
        true
    );
    assert_eq!(channel_path(availability, "cli")["available"], true);
    let fallback = values[1]["result"]["content"][1]["text"]
        .as_str()
        .expect("fallback text");
    assert!(fallback.contains("Host prompt input is unavailable"));
    assert!(fallback.contains("Volicord: answer J-1 1 #"));
    assert!(fallback.contains("Volicord: note J-1 \"text\" #"));
    Ok(())
}

#[test]
fn stdio_without_elicitation_uses_local_web_consent_when_prompt_capture_unavailable(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-fallback")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter_with_local_web_consent(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_judgment",
            product_judgment_args(&fixture, &task_id, state_version),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 2);
    let response = volicord_response_from_tool(&values[1])?;
    assert_eq!(response["user_judgment"]["status"], "pending");
    assert_eq!(
        response["inbox_item"]["preferred_capture_path"]["kind"],
        "local_web_consent"
    );
    let availability = &response["inbox_item"]["answer_path_availability"];
    assert_eq!(
        channel_path(availability, "mcp_elicitation")["available"],
        false
    );
    assert_eq!(
        channel_path(availability, "local_web_consent")["available"],
        true
    );
    assert_eq!(channel_path(availability, "cli")["available"], true);
    assert!(response["inbox_item"]["preferred_capture_path"]["url"]
        .as_str()
        .expect("local web URL should be present")
        .starts_with(&format!(
            "{}{}?project=",
            consent_base_url(),
            LOCAL_WEB_CONSENT_PATH
        )));
    assert!(response["inbox_item"]["fallbacks"]
        .as_array()
        .expect("inbox fallbacks should be an array")
        .iter()
        .any(|fallback| fallback["kind"] == "cli"
            && fallback["command"]
                .as_str()
                .is_some_and(|command| command.contains("volicord inbox answer"))));
    let fallback = values[1]["result"]["content"][1]["text"]
        .as_str()
        .expect("fallback text");
    assert!(fallback.contains("local Volicord consent link"));
    assert!(!fallback.contains("volicord user judgment answer"));

    let state: Value = serde_json::from_str(
        values[1]["result"]["content"][2]["text"]
            .as_str()
            .expect("structured fallback text"),
    )?;
    let state = &state["volicord_fallback"];
    assert_eq!(state["kind"], "local_web_consent");
    assert_eq!(state["project_id"], fixture.project_id());
    assert_eq!(state["connection_id"], fixture.connection_id());
    assert_eq!(
        state["capture_basis"],
        VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB
    );
    let url = state["url"].as_str().expect("fallback URL");
    assert!(url.starts_with(&format!(
        "{}{}?project=",
        consent_base_url(),
        LOCAL_WEB_CONSENT_PATH
    )));
    let token = token_from_consent_url(url)?;
    let now =
        local_web_consent_current_timestamp(fixture.runtime_home_path(), fixture.project_id())?;
    let validation = validate_local_web_consent_token(
        fixture.runtime_home_path(),
        LocalWebConsentTokenCheck {
            token,
            expected_project_id: fixture.project_id().to_owned(),
            expected_connection_internal_id: fixture.connection_id().to_owned(),
            now,
        },
    )?;
    assert!(matches!(
        validation,
        LocalWebConsentTokenValidation::Valid(_)
    ));
    Ok(())
}

#[test]
fn local_web_consent_get_renders_pending_judgment_page() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-get")?;
    let (_task_id, response) = create_pending_product_judgment(&fixture)?;
    let token = "1111111111111111111111111111111111111111111111111111111111111111";
    create_consent_token_for_response(&fixture, &response, token, 60)?;
    let mut server = consent_server(&fixture)?;

    let response = server.handle_request(consent_get_request(&consent_target(
        fixture.project_id(),
        token,
    )));

    assert_eq!(response.status, 200);
    assert_local_web_consent_security_headers(&response);
    let body = http_body_text(&response)?;
    assert!(body.contains("Record user-owned judgment"));
    assert!(body.contains("This page records one user-owned judgment"));
    assert!(body.contains("The agent cannot record this judgment on your behalf."));
    assert!(body.contains("does not prove correctness"));
    assert!(body.contains("test sufficiency"));
    assert!(body.contains("deployment success"));
    assert!(body.contains("review completion"));
    assert!(body.contains("Choose the focused User Channel test outcome."));
    assert!(body.contains(fixture.project_id()));
    assert!(body.contains(&fixture.product_repo_path().display().to_string()));
    assert!(body.contains(fixture.connection_id()));
    assert!(body.contains("Judgment id"));
    assert!(body.contains("Token expires"));
    assert!(body.contains("Fallback CLI command"));
    assert!(body.contains("volicord inbox answer"));
    assert!(body.contains("Available choices"));
    assert!(body.contains("Option ID: <code>keep</code>"));
    assert!(body.contains("Meaning: Only this focused judgment is resolved."));
    assert!(!body.contains("Runtime Home"));
    Ok(())
}

#[test]
fn local_web_consent_post_records_user_owned_answer() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-post")?;
    let (task_id, pending_response) = create_pending_product_judgment(&fixture)?;
    let token = "2222222222222222222222222222222222222222222222222222222222222222";
    create_consent_token_for_response(&fixture, &pending_response, token, 60)?;
    let mut server = consent_server(&fixture)?;

    let response = server.handle_request(consent_post_request(
        Some(consent_base_url()),
        &format!(
            "project={}&token={}&selected_option_id=keep&note=Browser+answer",
            percent_encode_query(fixture.project_id()),
            token
        ),
    ));

    assert_eq!(response.status, 200);
    assert_local_web_consent_security_headers(&response);
    let body = http_body_text(&response)?;
    assert!(body.contains("Answer recorded"));
    assert!(body.contains("user-owned judgment"));
    assert!(body.contains("does not prove correctness"));
    let pending_value = pending_response.response_value;
    let record = stored_judgment_record(&fixture, &task_id, &pending_value)?;
    assert_eq!(record.status, "resolved");
    assert_eq!(
        record.resolved_by_actor_source.as_deref(),
        Some("local_user")
    );
    assert_eq!(
        record.resolved_verification_basis.as_deref(),
        Some(VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB)
    );
    Ok(())
}

#[test]
fn local_web_consent_rejects_origin_mismatch_without_consuming_token() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-local-web-origin")?;
    let (task_id, pending_response) = create_pending_product_judgment(&fixture)?;
    let token = "9999999999999999999999999999999999999999999999999999999999999999";
    create_consent_token_for_response(&fixture, &pending_response, token, 60)?;
    let mut server = consent_server(&fixture)?;
    let form_body = format!(
        "project={}&token={}&selected_option_id=keep",
        percent_encode_query(fixture.project_id()),
        token
    );

    let rejected = server.handle_request(consent_post_request(
        Some("http://example.invalid"),
        &form_body,
    ));

    assert_eq!(rejected.status, 403);
    assert_local_web_consent_security_headers(&rejected);
    assert!(http_body_text(&rejected)?.contains("ORIGIN_NOT_ALLOWED"));

    let valid = server.handle_request(consent_post_request(Some(consent_base_url()), &form_body));
    assert_eq!(valid.status, 200);
    assert_local_web_consent_security_headers(&valid);
    assert!(http_body_text(&valid)?.contains("Answer recorded"));
    let pending_value = pending_response.response_value;
    let record = stored_judgment_record(&fixture, &task_id, &pending_value)?;
    assert_eq!(record.status, "resolved");
    Ok(())
}

#[test]
fn local_web_consent_validation_failure_leaves_token_reusable() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-validation-retry")?;
    let (task_id, pending_response) = create_pending_product_judgment(&fixture)?;
    let token = "8888888888888888888888888888888888888888888888888888888888888888";
    create_consent_token_for_response(&fixture, &pending_response, token, 60)?;
    let mut server = consent_server(&fixture)?;

    let invalid = server.handle_request(consent_post_request(
        Some(consent_base_url()),
        &format!(
            "project={}&token={}&selected_option_id=missing",
            percent_encode_query(fixture.project_id()),
            token
        ),
    ));
    assert_eq!(invalid.status, 400);
    assert_local_web_consent_security_headers(&invalid);
    assert!(http_body_text(&invalid)?.contains("INVALID_SELECTION"));

    let valid = server.handle_request(consent_post_request(
        Some(consent_base_url()),
        &format!(
            "project={}&token={}&selected_option_id=keep",
            percent_encode_query(fixture.project_id()),
            token
        ),
    ));

    assert_eq!(valid.status, 200);
    assert_local_web_consent_security_headers(&valid);
    assert!(http_body_text(&valid)?.contains("Answer recorded"));
    let pending_value = pending_response.response_value;
    let record = stored_judgment_record(&fixture, &task_id, &pending_value)?;
    assert_eq!(record.status, "resolved");
    Ok(())
}

#[test]
fn local_web_consent_rejects_invalid_token() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-invalid")?;
    let mut server = consent_server(&fixture)?;

    let response = server.handle_request(consent_get_request(&consent_target(
        fixture.project_id(),
        "invalid-token",
    )));

    assert_eq!(response.status, 404);
    assert_local_web_consent_security_headers(&response);
    assert!(http_body_text(&response)?.contains("INVALID_TOKEN"));
    Ok(())
}

#[test]
fn local_web_consent_rejects_expired_token() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-expired")?;
    let (_task_id, pending_response) = create_pending_product_judgment(&fixture)?;
    let token = "3333333333333333333333333333333333333333333333333333333333333333";
    create_consent_token_for_response(&fixture, &pending_response, token, 60)?;
    volicord_store::local_consent::expire_local_web_consent_tokens(
        fixture.runtime_home_path(),
        fixture.project_id(),
        "2999-01-01T00:00:00.000Z",
    )?;
    let mut server = consent_server(&fixture)?;

    let response = server.handle_request(consent_get_request(&consent_target(
        fixture.project_id(),
        token,
    )));

    assert_eq!(response.status, 410);
    assert_local_web_consent_security_headers(&response);
    assert!(http_body_text(&response)?.contains("TOKEN_EXPIRED"));
    Ok(())
}

#[test]
fn local_web_consent_rejects_replay() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-replay")?;
    let (_task_id, pending_response) = create_pending_product_judgment(&fixture)?;
    let token = "4444444444444444444444444444444444444444444444444444444444444444";
    create_consent_token_for_response(&fixture, &pending_response, token, 60)?;
    let mut server = consent_server(&fixture)?;
    let form_body = format!(
        "project={}&token={}&selected_option_id=keep",
        percent_encode_query(fixture.project_id()),
        token
    );

    let first = server.handle_request(consent_post_request(Some(consent_base_url()), &form_body));
    let replay = server.handle_request(consent_post_request(Some(consent_base_url()), &form_body));

    assert_eq!(first.status, 200);
    assert_eq!(replay.status, 409);
    assert_local_web_consent_security_headers(&first);
    assert_local_web_consent_security_headers(&replay);
    assert!(http_body_text(&replay)?.contains("TOKEN_CONSUMED"));
    Ok(())
}

#[test]
fn local_web_consent_rejects_wrong_project_and_connection() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-context")?;
    let (_task_id, pending_response) = create_pending_product_judgment(&fixture)?;
    let token = "5555555555555555555555555555555555555555555555555555555555555555";
    create_consent_token_for_response(&fixture, &pending_response, token, 60)?;

    let mut server = consent_server(&fixture)?;
    let wrong_project =
        server.handle_request(consent_get_request(&consent_target("project_other", token)));
    assert_eq!(wrong_project.status, 403);
    assert_local_web_consent_security_headers(&wrong_project);
    assert!(http_body_text(&wrong_project)?.contains("WRONG_PROJECT"));

    let mut wrong_connection_server =
        consent_server_for_connection(&fixture, "conn_mcp_local_web_other")?;
    let wrong_connection = wrong_connection_server.handle_request(consent_get_request(
        &consent_target(fixture.project_id(), token),
    ));
    assert_eq!(wrong_connection.status, 403);
    assert_local_web_consent_security_headers(&wrong_connection);
    assert!(http_body_text(&wrong_connection)?.contains("WRONG_CONNECTION"));
    Ok(())
}

#[test]
fn local_http_rejects_missing_bearer_auth() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-http-auth")?;
    let mut server = http_server(&fixture, Vec::new(), Vec::new())?;

    let response = server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        None,
        None,
        None,
        initialize_request(1, json!({})),
    )?);

    assert_eq!(response.status, 401);
    assert_eq!(http_json(&response)["error"]["code"], "AUTH_REQUIRED");
    assert_diagnostic_disclosure(&http_json(&response));
    assert_eq!(http_header(&response, "WWW-Authenticate"), Some("Bearer"));

    let unauthenticated_health = server.handle_request(http_request(
        "GET",
        "/healthz",
        None,
        None,
        None,
        Value::Null,
    )?);
    assert_eq!(unauthenticated_health.status, 401);
    assert_eq!(
        http_json(&unauthenticated_health)["error"]["code"],
        "AUTH_REQUIRED"
    );

    let health = server.handle_request(http_request(
        "GET",
        "/healthz",
        Some("test_token"),
        None,
        None,
        Value::Null,
    )?);
    assert_eq!(health.status, 200);
    assert_eq!(http_json(&health)["status"], "ok");
    assert_diagnostic_disclosure(&http_json(&health));
    let health_body = serde_json::to_string(&http_json(&health))?;
    assert!(!health_body.contains("test_token"));
    assert!(!health_body.contains(fixture.connection_id()));
    assert!(!health_body.contains(fixture.project_id()));
    assert!(!health_body.contains(&fixture.runtime_home_path().display().to_string()));
    Ok(())
}

#[test]
fn local_http_rejects_origin_unless_explicitly_allowed() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-http-origin")?;
    let mut server = http_server(&fixture, Vec::new(), Vec::new())?;

    let rejected = server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        Some("https://example.invalid"),
        None,
        initialize_request(1, json!({})),
    )?);

    assert_eq!(rejected.status, 403);
    assert_eq!(http_json(&rejected)["error"]["code"], "ORIGIN_NOT_ALLOWED");
    assert_eq!(http_header(&rejected, "Access-Control-Allow-Origin"), None);

    let denied_preflight = server.handle_request(http_request(
        "OPTIONS",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        None,
        Some("https://example.invalid"),
        None,
        Value::Null,
    )?);
    assert_eq!(denied_preflight.status, 403);
    assert_eq!(
        http_json(&denied_preflight)["error"]["code"],
        "ORIGIN_NOT_ALLOWED"
    );

    let mut allowed_server = http_server(
        &fixture,
        Vec::new(),
        vec!["https://allowed.example".to_owned()],
    )?;
    let allowed = allowed_server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        Some("https://allowed.example"),
        None,
        initialize_request(2, json!({})),
    )?);
    assert_eq!(allowed.status, 200);
    assert_eq!(
        http_header(&allowed, "Access-Control-Allow-Origin"),
        Some("https://allowed.example")
    );
    Ok(())
}

#[test]
fn local_http_rejects_unsupported_mcp_endpoint_methods() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-http-method")?;
    let mut server = http_server(&fixture, Vec::new(), Vec::new())?;

    let rejected = server.handle_request(http_request(
        "PUT",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        None,
        None,
        Value::Null,
    )?);

    assert_eq!(rejected.status, 405);
    assert_eq!(http_json(&rejected)["error"]["code"], "METHOD_NOT_ALLOWED");
    assert_eq!(
        http_header(&rejected, "Allow"),
        Some("POST, GET, DELETE, OPTIONS")
    );
    Ok(())
}

#[test]
fn local_http_rejects_nonlocal_listen_addresses() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-http-listen")?;

    for listen_addr in ["0.0.0.0:8765", "[::]:8765", "192.0.2.10:8765"] {
        let mut config = http_config(&fixture, Vec::new(), Vec::new());
        config.listen_addr = listen_addr.parse()?;
        let error = validate_local_http_server_config(&config)
            .expect_err("nonlocal listen address should be rejected");
        assert!(
            error.to_string().contains("NONLOCAL_LISTEN_REJECTED"),
            "unexpected error for {listen_addr}: {error}"
        );
    }

    for listen_addr in ["127.0.0.1:0", "[::1]:0"] {
        let mut config = http_config(&fixture, Vec::new(), Vec::new());
        config.listen_addr = listen_addr.parse()?;
        validate_local_http_server_config(&config)?;
    }
    Ok(())
}

#[test]
fn local_http_container_listen_scope_allows_only_container_wildcard() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-http-container-listen")?;

    for listen_addr in ["0.0.0.0:8765", "[::]:8765"] {
        let mut config = http_config(&fixture, Vec::new(), Vec::new());
        config.listen_addr = listen_addr.parse()?;
        config.listen_scope = LocalHttpListenScope::ContainerPublishedHostLoopback;
        validate_local_http_server_config(&config)?;
    }

    for listen_addr in ["127.0.0.1:8765", "192.0.2.10:8765", "0.0.0.0:0"] {
        let mut config = http_config(&fixture, Vec::new(), Vec::new());
        config.listen_addr = listen_addr.parse()?;
        config.listen_scope = LocalHttpListenScope::ContainerPublishedHostLoopback;
        let error = validate_local_http_server_config(&config)
            .expect_err("unsupported container listen address should be rejected");
        assert!(
            error.to_string().contains("CONTAINER_LISTEN_REJECTED"),
            "unexpected error for {listen_addr}: {error}"
        );
    }
    Ok(())
}

#[test]
fn project_bound_http_initialize_creates_baseline_before_tool_handling(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-http-startup-watch")?;
    let mut server = http_server(
        &fixture,
        vec![ProjectId::new(fixture.project_id())],
        Vec::new(),
    )?;

    let initialize = server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        None,
        None,
        initialize_request(1, json!({})),
    )?);

    assert_eq!(initialize.status, 200);
    assert!(http_header(&initialize, "Mcp-Session-Id").is_some());
    let baseline = latest_watch_baseline_for_connection(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
    )?
    .expect("HTTP initialize should create a watch baseline");
    let metadata: Value = serde_json::from_str(&baseline.metadata_json)?;
    assert_eq!(metadata["coverage_basis"], "mcp_start");
    Ok(())
}

#[test]
fn local_http_project_allowlist_narrows_connection_projects() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-http-project-allowlist")?;
    let outside_project_id = "project_http_allowed_by_connection";
    add_allowed_project(&fixture, outside_project_id)?;
    let mut server = http_server(
        &fixture,
        vec![ProjectId::new(fixture.project_id())],
        Vec::new(),
    )?;

    let initialize = server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        None,
        None,
        initialize_request(1, json!({})),
    )?);
    assert_eq!(initialize.status, 200);
    let session_id = http_header(&initialize, "Mcp-Session-Id")
        .expect("initialize should create session")
        .to_owned();

    let initialized = server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        None,
        Some(&session_id),
        initialized_notification(),
    )?);
    assert_eq!(initialized.status, 202);

    let listed = server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        None,
        Some(&session_id),
        tools_call(2, LIST_PROJECTS_TOOL_NAME, json!({})),
    )?);
    assert_eq!(listed.status, 200);
    let listed_tool = volicord_response_from_tool(&http_json(&listed))?;
    let projects = listed_tool["projects"]
        .as_array()
        .expect("projects should be listed");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["project_selector"], fixture.project_id());

    let rejected = server.handle_request(http_request(
        "POST",
        LOCAL_HTTP_MCP_ENDPOINT_PATH,
        Some("test_token"),
        None,
        Some(&session_id),
        tools_call(
            3,
            "volicord.status",
            json!({
                "detail": "workflow",
                "project_selector": outside_project_id
            }),
        ),
    )?);
    assert_eq!(rejected.status, 200);
    let rejected_json = http_json(&rejected);
    let error = structured_error_result(&rejected_json["result"]);
    assert_eq!(error["code"], "MCP_ADAPTER_PRECONDITION_FAILED");
    assert_eq!(error["tool_name"], STATUS_TOOL_NAME);
    let issue = tool_error_issue(
        &error,
        "/project_selector",
        "MCP_ADAPTER_PRECONDITION_FAILED",
    );
    assert!(issue["message"]
        .as_str()
        .is_some_and(|message| message.contains("outside this HTTP serve project allowlist")));
    Ok(())
}

fn adapter(fixture: &CoreFixture) -> Result<McpAdapter, Box<dyn Error>> {
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
    Ok(McpAdapter::new(fixture.runtime_home_path(), context))
}

fn project_bound_adapter(fixture: &CoreFixture) -> Result<McpAdapter, Box<dyn Error>> {
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?
            .with_project_allowlist(vec![ProjectId::new(fixture.project_id())])
            .with_invocation_binding_basis(VERIFICATION_BASIS_TEST_FIXTURE_BINDING);
    Ok(McpAdapter::new(fixture.runtime_home_path(), context))
}

fn adapter_with_local_web_consent(fixture: &CoreFixture) -> Result<McpAdapter, Box<dyn Error>> {
    Ok(
        adapter(fixture)?.with_local_web_consent(LocalWebConsentContext {
            base_url: consent_base_url().to_owned(),
        }),
    )
}

fn consent_base_url() -> &'static str {
    "http://127.0.0.1:39000"
}

fn install_prompt_capture_guard(fixture: &CoreFixture) -> Result<(), Box<dyn Error>> {
    upsert_guard_installation(
        fixture.runtime_home_path(),
        GuardInstallationUpsert {
            guard_installation_id: "guard_installation_mcp_prompt_capture".to_owned(),
            connection_internal_id: fixture.connection_id().to_owned(),
            project_id: Some(fixture.project_id().to_owned()),
            host_kind: "prompt_capture_test_host".to_owned(),
            guard_mode: IntegrationProfile::Detective.as_str().to_owned(),
            host_capability_json: json!({
                "schema": "volicord-host-hook-capability-v1",
                "policy_hash": "sha256:mcp-prompt-capture",
                "host_capabilities": {
                    "user_prompt_submit_hook": true
                },
                "required_hook_phases": [
                    "session_start_hook",
                    "pre_tool_hook",
                    "post_tool_hook",
                    "user_prompt_submit_hook",
                    "stop_hook"
                ],
                "missing_required_hooks": [],
                "prompt_capture": true
            })
            .to_string(),
            installation_status: "configured".to_owned(),
            installed_at: Some("2026-06-30T00:00:00Z".to_owned()),
            last_checked_at: "2026-06-30T00:00:00Z".to_owned(),
            first_seen_at: None,
            last_seen_at: None,
            last_seen_phase: None,
            observed_host_kind: None,
            observed_policy_hash: None,
            observed_binary_version: None,
            metadata_json: "{}".to_owned(),
        },
    )?;
    Ok(())
}

fn set_mode(fixture: &CoreFixture, mode: &str) -> Result<(), Box<dyn Error>> {
    let existing = agent_connection_record(fixture.runtime_home_path(), fixture.connection_id())?
        .expect("fixture connection should exist");
    ensure_agent_connection(
        fixture.runtime_home_path(),
        AgentConnectionRegistration {
            connection_internal_id: existing.connection_internal_id,
            host_kind: existing.host_kind,
            intent: existing.intent,
            host_scope: existing.host_scope,
            server_name: existing.server_name,
            config_target: existing.config_target,
            mode: mode.to_owned(),
            enabled: existing.enabled,
            managed_fingerprint: existing.managed_fingerprint,
            last_verification_status: existing.last_verification_status,
            last_verification_report_json: existing.last_verification_report_json,
            last_user_actions_json: existing.last_user_actions_json,
            metadata_json: existing.metadata_json,
        },
    )?;
    Ok(())
}

fn http_config(
    fixture: &CoreFixture,
    project_allowlist: Vec<ProjectId>,
    allowed_origins: Vec<String>,
) -> LocalHttpServerConfig {
    LocalHttpServerConfig {
        runtime_home: fixture.runtime_home_path().to_path_buf(),
        connection_id: fixture.connection_id().to_owned(),
        listen_addr: "127.0.0.1:0".parse().expect("valid test listen"),
        listen_scope: LocalHttpListenScope::NativeLoopback,
        bearer_token: "test_token".to_owned(),
        token_source: LocalHttpTokenSource::Supplied,
        project_allowlist,
        allowed_origins,
    }
}

fn http_server(
    fixture: &CoreFixture,
    project_allowlist: Vec<ProjectId>,
    allowed_origins: Vec<String>,
) -> Result<LocalHttpServer, Box<dyn Error>> {
    let config = http_config(fixture, project_allowlist.clone(), allowed_origins);
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?
            .with_invocation_binding_basis(VERIFICATION_BASIS_MCP_LOCAL_HTTP_CONNECTION_BINDING)
            .with_project_allowlist(project_allowlist);
    Ok(LocalHttpServer::new(
        McpAdapter::new(fixture.runtime_home_path(), context),
        config,
    ))
}

fn consent_server(fixture: &CoreFixture) -> Result<LocalHttpServer, Box<dyn Error>> {
    consent_server_with_context(
        fixture,
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?
            .with_invocation_binding_basis(VERIFICATION_BASIS_MCP_LOCAL_HTTP_CONNECTION_BINDING),
    )
}

fn consent_server_for_connection(
    fixture: &CoreFixture,
    connection_id: &str,
) -> Result<LocalHttpServer, Box<dyn Error>> {
    let existing = agent_connection_record(fixture.runtime_home_path(), fixture.connection_id())?
        .expect("fixture connection should exist");
    ensure_agent_connection(
        fixture.runtime_home_path(),
        AgentConnectionRegistration {
            connection_internal_id: connection_id.to_owned(),
            host_kind: existing.host_kind,
            intent: existing.intent,
            host_scope: existing.host_scope,
            server_name: existing.server_name,
            config_target: format!("{}_other", existing.config_target),
            mode: existing.mode,
            enabled: existing.enabled,
            managed_fingerprint: format!("{}_other", existing.managed_fingerprint),
            last_verification_status: existing.last_verification_status,
            last_verification_report_json: existing.last_verification_report_json,
            last_user_actions_json: existing.last_user_actions_json,
            metadata_json: existing.metadata_json,
        },
    )?;
    add_connection_project(
        fixture.runtime_home_path(),
        ConnectionProjectRegistration {
            connection_internal_id: connection_id.to_owned(),
            project_id: fixture.project_id().to_owned(),
        },
    )?;
    consent_server_with_context(
        fixture,
        McpConnectionContext::resolve(fixture.runtime_home_path(), connection_id)?
            .with_invocation_binding_basis(VERIFICATION_BASIS_MCP_LOCAL_HTTP_CONNECTION_BINDING),
    )
}

fn consent_server_with_context(
    fixture: &CoreFixture,
    context: McpConnectionContext,
) -> Result<LocalHttpServer, Box<dyn Error>> {
    Ok(LocalHttpServer::new(
        McpAdapter::new(fixture.runtime_home_path(), context).with_local_web_consent(
            LocalWebConsentContext {
                base_url: consent_base_url().to_owned(),
            },
        ),
        http_config(fixture, Vec::new(), Vec::new()),
    ))
}

fn http_request(
    method: &str,
    target: &str,
    token: Option<&str>,
    origin: Option<&str>,
    session_id: Option<&str>,
    body: Value,
) -> Result<HttpRequest, serde_json::Error> {
    let mut headers = BTreeMap::new();
    headers.insert(
        "accept".to_owned(),
        "application/json, text/event-stream".to_owned(),
    );
    headers.insert("content-type".to_owned(), "application/json".to_owned());
    if let Some(token) = token {
        headers.insert("authorization".to_owned(), format!("Bearer {token}"));
    }
    if let Some(origin) = origin {
        headers.insert("origin".to_owned(), origin.to_owned());
    }
    if let Some(session_id) = session_id {
        headers.insert("mcp-session-id".to_owned(), session_id.to_owned());
    }
    Ok(HttpRequest {
        method: method.to_owned(),
        target: target.to_owned(),
        headers,
        body: serde_json::to_vec(&body)?,
    })
}

fn consent_get_request(target: &str) -> HttpRequest {
    HttpRequest {
        method: "GET".to_owned(),
        target: target.to_owned(),
        headers: BTreeMap::new(),
        body: Vec::new(),
    }
}

fn consent_post_request(origin: Option<&str>, body: &str) -> HttpRequest {
    let mut headers = BTreeMap::new();
    headers.insert(
        "content-type".to_owned(),
        "application/x-www-form-urlencoded".to_owned(),
    );
    if let Some(origin) = origin {
        headers.insert("origin".to_owned(), origin.to_owned());
    }
    HttpRequest {
        method: "POST".to_owned(),
        target: LOCAL_WEB_CONSENT_PATH.to_owned(),
        headers,
        body: body.as_bytes().to_vec(),
    }
}

fn http_json(response: &HttpResponse) -> Value {
    serde_json::from_slice(&response.body).expect("HTTP body should be JSON")
}

fn assert_diagnostic_disclosure(value: &Value) {
    let disclosure = value
        .get("disclosure")
        .expect("HTTP status or error should include disclosure");
    assert_eq!(disclosure["guarantee_class"], "detective_observation");
    let values = disclosure["non_guarantees"]
        .as_array()
        .expect("disclosure should include non_guarantees");
    for expected in [
        "NotOsSandbox",
        "NotActorAttributionProof",
        "NotNetworkIsolation",
    ] {
        assert!(
            values.iter().any(|value| value.as_str() == Some(expected)),
            "missing non-guarantee {expected}: {disclosure}"
        );
    }
}

fn http_body_text(response: &HttpResponse) -> Result<String, Box<dyn Error>> {
    Ok(std::str::from_utf8(&response.body)?.to_owned())
}

fn http_header<'a>(response: &'a HttpResponse, name: &str) -> Option<&'a str> {
    response
        .headers
        .iter()
        .find(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn assert_local_web_consent_security_headers(response: &HttpResponse) {
    assert_eq!(http_header(response, "Cache-Control"), Some("no-store"));
    assert_eq!(
        http_header(response, "Referrer-Policy"),
        Some("no-referrer")
    );
    assert_eq!(
        http_header(response, "X-Content-Type-Options"),
        Some("nosniff")
    );
    let csp = http_header(response, "Content-Security-Policy")
        .expect("local web consent response should include CSP");
    assert!(csp.contains("default-src 'none'"));
    assert!(csp.contains("form-action 'self'"));
    assert!(csp.contains("frame-ancestors 'none'"));
    assert!(csp.contains("script-src 'none'"));
}

fn add_allowed_project(fixture: &CoreFixture, project_id: &str) -> Result<(), Box<dyn Error>> {
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
    add_connection_project(
        fixture.runtime_home_path(),
        ConnectionProjectRegistration {
            connection_internal_id: fixture.connection_id().to_owned(),
            project_id: project_id.to_owned(),
        },
    )?;
    Ok(())
}

fn create_pending_product_judgment(
    fixture: &CoreFixture,
) -> Result<(String, PipelineResponse), Box<dyn Error>> {
    let setup_adapter = adapter(fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let response = setup_adapter.call_tool(
        "volicord.request_user_judgment",
        product_judgment_args(fixture, &task_id, state_version),
    )?;
    Ok((task_id, response))
}

fn create_consent_token_for_response(
    fixture: &CoreFixture,
    response: &PipelineResponse,
    token: &str,
    ttl_seconds: u64,
) -> Result<(), Box<dyn Error>> {
    let judgment = pending_judgment_from_response(response)
        .ok_or("response should include a pending user judgment")?;
    create_local_web_consent_token(
        fixture.runtime_home_path(),
        LocalWebConsentTokenCreate {
            token: token.to_owned(),
            project_id: judgment.project_id.as_str().to_owned(),
            connection_internal_id: fixture.connection_id().to_owned(),
            judgment_id: judgment.judgment_id.as_str().to_owned(),
            capture_basis: VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB.to_owned(),
            ttl_seconds,
            created_metadata_json: json!({ "test": "local_web_consent" }).to_string(),
        },
    )?;
    Ok(())
}

fn consent_target(project_id: &str, token: &str) -> String {
    format!(
        "{}?project={}&token={}",
        LOCAL_WEB_CONSENT_PATH,
        percent_encode_query(project_id),
        percent_encode_query(token)
    )
}

fn token_from_consent_url(url: &str) -> Result<String, Box<dyn Error>> {
    let query = url
        .split_once('?')
        .map(|(_, query)| query)
        .ok_or("consent URL should include a query string")?;
    let fields = parse_urlencoded(query);
    Ok(single_param(&fields, "token")
        .ok_or("consent URL should include exactly one token")?
        .to_owned())
}

fn create_task(adapter: &McpAdapter) -> Result<(String, u64), Box<dyn Error>> {
    let response = adapter.call_tool(
        "volicord.intake",
        json!({
            "plain_language_request": "Create a task for User Channel tests.",
            "requested_mode": "work",
            "resume_policy": "create_new",
            "initial_scope": {
                "boundary": "User Channel test task.",
                "non_goals": ["Changing unrelated behavior."],
                "acceptance_criteria": [{
                    "statement": "A pending judgment can be requested.",
                    "evidence_requirement": "required"
                }]
            },
            "initial_context_refs": []
        }),
    )?;
    let task_id = response.response_value["task_ref"]["record_id"]
        .as_str()
        .expect("task id")
        .to_owned();
    let state_version = response.response_value["base"]["state_version"]
        .as_u64()
        .expect("state version");
    Ok((task_id, state_version))
}

fn write_product_file(
    fixture: &CoreFixture,
    path: &str,
    contents: &str,
) -> Result<(), Box<dyn Error>> {
    let absolute = fixture.product_repo_path().join(path);
    if let Some(parent) = absolute.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(absolute, contents)?;
    Ok(())
}

#[cfg(unix)]
struct ReadOnlyProjectStateGuard {
    state_db_path: std::path::PathBuf,
    state_dir: std::path::PathBuf,
    old_state_mode: u32,
    old_dir_mode: u32,
}

#[cfg(unix)]
impl Drop for ReadOnlyProjectStateGuard {
    fn drop(&mut self) {
        let _ = fs::set_permissions(
            &self.state_dir,
            fs::Permissions::from_mode(self.old_dir_mode),
        );
        let _ = fs::set_permissions(
            &self.state_db_path,
            fs::Permissions::from_mode(self.old_state_mode),
        );
    }
}

#[cfg(unix)]
fn make_project_state_readonly(
    fixture: &CoreFixture,
) -> Result<ReadOnlyProjectStateGuard, Box<dyn Error>> {
    let state_db_path = fixture
        .runtime_home_path()
        .join("projects")
        .join(fixture.project_id())
        .join("state.sqlite");
    let state_dir = state_db_path
        .parent()
        .expect("project state database should have a parent directory")
        .to_path_buf();
    let old_state_mode = fs::metadata(&state_db_path)?.permissions().mode();
    let old_dir_mode = fs::metadata(&state_dir)?.permissions().mode();

    fs::set_permissions(
        &state_db_path,
        fs::Permissions::from_mode(old_state_mode & !0o222),
    )?;
    fs::set_permissions(
        &state_dir,
        fs::Permissions::from_mode(old_dir_mode & !0o222),
    )?;

    Ok(ReadOnlyProjectStateGuard {
        state_db_path,
        state_dir,
        old_state_mode,
        old_dir_mode,
    })
}

fn initialize_request(id: u64, capabilities: Value) -> Value {
    request(
        id,
        "initialize",
        json!({
            "protocolVersion": SUPPORTED_PROTOCOL_VERSION,
            "capabilities": capabilities,
            "clientInfo": {
                "name": "volicord-unit-test",
                "version": "0.0.0"
            }
        }),
    )
}

fn initialized_notification() -> Value {
    notification("notifications/initialized", json!({}))
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

fn intake_args(project_selector: Option<&str>) -> Value {
    let mut arguments = json!({
        "plain_language_request": "Exercise MCP lifecycle gating.",
        "requested_mode": "work",
        "resume_policy": "create_new",
        "initial_scope": {
            "boundary": "MCP lifecycle gating test.",
            "non_goals": ["Changing Core method behavior."],
            "acceptance_criteria": [{
                "statement": "tools/call is gated until notifications/initialized.",
                "evidence_requirement": "required"
            }]
        },
        "initial_context_refs": []
    });
    if let Some(project_selector) = project_selector {
        arguments["project_selector"] = json!(project_selector);
    }
    arguments
}

fn product_judgment_args(fixture: &CoreFixture, task_id: &str, state_version: u64) -> Value {
    judgment_args(
        fixture,
        task_id,
        state_version,
        "product_decision",
        json!([
            {
                "option_id": "keep",
                "label": "Keep focused behavior",
                "description": "Record the user-owned product decision to keep the behavior.",
                "consequence": "Only this focused judgment is resolved.",
                "is_default": true
            },
            {
                "option_id": "change",
                "label": "Change focused behavior",
                "description": "Record the user-owned product decision to change the behavior.",
                "consequence": "Only this focused judgment is resolved with the alternate option.",
                "is_default": false
            }
        ]),
        json!(["close_complete"]),
    )
}

fn authority_judgment_args(fixture: &CoreFixture, task_id: &str, state_version: u64) -> Value {
    judgment_args(
        fixture,
        task_id,
        state_version,
        "scope_decision",
        Value::Null,
        json!(["scope_update"]),
    )
}

fn judgment_args(
    fixture: &CoreFixture,
    task_id: &str,
    state_version: u64,
    judgment_kind: &str,
    options: Value,
    required_for: Value,
) -> Value {
    json!({
        "task_id": task_id,
        "change_unit_id": null,
        "judgment_kind": judgment_kind,
        "presentation": "short",
        "question": "Choose the focused User Channel test outcome.",
        "options": options,
        "context": {
            "summary": "A focused test judgment needs a user-owned answer.",
            "related_refs": [],
            "artifact_refs": [],
            "visible_risks": [],
            "constraints": ["The answer covers only this pending judgment."]
        },
        "affected_refs": [
            {
                "record_kind": "task",
                "record_id": task_id,
                "project_id": fixture.project_id(),
                "task_id": task_id,
                "produced_at_state_version": state_version
            }
        ],
        "required_for": required_for,
        "expires_at": null
    })
}

fn elicitation_accept(selected_option_id: &str, note: Option<&str>) -> Value {
    let mut content = json!({
        "selected_option_id": selected_option_id
    });
    if let Some(note) = note {
        content["note"] = json!(note);
    }
    json!({
        "jsonrpc": "2.0",
        "id": "elicit_user_judgment_1",
        "result": {
            "action": "accept",
            "content": content
        }
    })
}

fn elicitation_action(action: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": "elicit_user_judgment_1",
        "result": {
            "action": action
        }
    })
}

fn json_lines(messages: &[Value]) -> Result<Vec<u8>, serde_json::Error> {
    let mut output = Vec::new();
    for message in messages {
        serde_json::to_writer(&mut output, message)?;
        output.push(b'\n');
    }
    Ok(output)
}

fn volicord_response_from_tool(response: &Value) -> Result<Value, Box<dyn Error>> {
    assert_eq!(response["result"]["isError"], json!(false));
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("tools/call response should include text content")?;
    let parsed: Value = serde_json::from_str(text)?;
    assert_eq!(
        response["result"]["structuredContent"], parsed,
        "structuredContent should equal the compatibility JSON text"
    );
    Ok(parsed)
}

fn channel_path<'a>(availability: &'a Value, kind: &str) -> &'a Value {
    let paths = availability["paths"]
        .as_array()
        .expect("user channel availability paths should be an array");
    paths
        .iter()
        .find(|path| path["kind"] == kind)
        .unwrap_or_else(|| panic!("expected user channel path {kind}, got {paths:?}"))
}

fn stored_resolution_basis(
    fixture: &CoreFixture,
    task_id: &str,
    response: &Value,
) -> Result<String, Box<dyn Error>> {
    let record = stored_judgment_record(fixture, task_id, response)?;
    record
        .resolved_verification_basis
        .ok_or_else(|| "stored judgment should have a resolution basis".into())
}

fn stored_judgment_record(
    fixture: &CoreFixture,
    task_id: &str,
    response: &Value,
) -> Result<volicord_store::core_pipeline::UserJudgmentRecord, Box<dyn Error>> {
    let judgment_id = response["user_judgment_ref"]["record_id"]
        .as_str()
        .ok_or("response should include user_judgment_ref.record_id")?;
    let store = CoreProjectStore::open(
        fixture.runtime_home_path(),
        &ProjectId::new(fixture.project_id()),
    )?;
    let record = store
        .user_judgment_records_for_task(&volicord_types::TaskId::new(task_id))?
        .into_iter()
        .find(|record| record.judgment_id == judgment_id)
        .ok_or("stored judgment record should exist")?;
    Ok(record)
}

fn tool_names(tools: &[McpToolDefinition]) -> Vec<&'static str> {
    tools.iter().map(|tool| tool.name).collect::<Vec<_>>()
}

fn tool_definition(tool_name: &str) -> McpToolDefinition {
    mcp_tools_for_mode_and_storage(
        AgentConnectionMode::Workflow,
        McpStorageCapability::ReadWrite,
    )
    .into_iter()
    .find(|tool| tool.name == tool_name)
    .unwrap_or_else(|| panic!("missing tool definition for {tool_name}"))
}

fn canonical_example(
    tool_name: &str,
    example_id: &str,
) -> &'static crate::tool_registry::McpToolExample {
    canonical_tool_examples(tool_name)
        .iter()
        .find(|example| example.id == example_id)
        .unwrap_or_else(|| panic!("missing canonical example {example_id} for {tool_name}"))
}

fn canonical_example_value(tool_name: &str, example_id: &str) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(
        canonical_example(tool_name, example_id).arguments_json,
    )?)
}

fn decode_mcp_arguments_to_value(
    tool_name: &str,
    value: Value,
) -> Result<Value, serde_json::Error> {
    match tool_name {
        INTAKE_TOOL_NAME => {
            serde_json::to_value(serde_json::from_value::<McpIntakeArguments>(value)?)
        }
        UPDATE_SCOPE_TOOL_NAME => {
            serde_json::to_value(serde_json::from_value::<McpUpdateScopeArguments>(value)?)
        }
        STATUS_TOOL_NAME => {
            serde_json::to_value(serde_json::from_value::<McpStatusArguments>(value)?)
        }
        PREPARE_WRITE_TOOL_NAME => {
            serde_json::to_value(serde_json::from_value::<McpPrepareWriteArguments>(value)?)
        }
        STAGE_ARTIFACT_TOOL_NAME => {
            serde_json::to_value(serde_json::from_value::<McpStageArtifactArguments>(value)?)
        }
        RECORD_RUN_TOOL_NAME => {
            serde_json::to_value(serde_json::from_value::<McpRecordRunArguments>(value)?)
        }
        REQUEST_USER_JUDGMENT_TOOL_NAME => serde_json::to_value(serde_json::from_value::<
            McpRequestUserJudgmentArguments,
        >(value)?),
        RECONCILE_CHANGES_TOOL_NAME => serde_json::to_value(serde_json::from_value::<
            McpReconcileChangesArguments,
        >(value)?),
        CHECK_CLOSE_TOOL_NAME => {
            serde_json::to_value(serde_json::from_value::<McpCheckCloseArguments>(value)?)
        }
        CLOSE_TASK_TOOL_NAME => {
            serde_json::to_value(serde_json::from_value::<McpCloseTaskArguments>(value)?)
        }
        other => panic!("unsupported MCP tool example decoder: {other}"),
    }
}

fn structured_tool_error(tool_name: &str, error: &McpAdapterError) -> Value {
    let result = tool_execution_error_result(tool_name, error);
    let parsed = structured_error_result(&result);
    assert_eq!(parsed["tool_name"], tool_name);
    match error {
        McpAdapterError::InvalidParams { .. } => {
            assert_eq!(parsed["code"], "MCP_INVALID_ARGUMENTS");
            assert_eq!(parsed["retryable"], true);
        }
        McpAdapterError::ToolExecution { .. } => {
            assert_eq!(parsed["code"], "MCP_ADAPTER_PRECONDITION_FAILED");
            assert_eq!(parsed["retryable"], false);
        }
        _ => {}
    }
    parsed
}

fn structured_error_result(result: &Value) -> Value {
    assert_eq!(result["isError"], true);
    assert!(
        serde_json::to_vec(result)
            .expect("tool error result should serialize")
            .len()
            <= MAX_MCP_TOOL_ERROR_RESULT_BYTES
    );
    let parsed: Value = serde_json::from_str(
        result["content"][0]["text"]
            .as_str()
            .expect("tool error compatibility text"),
    )
    .expect("tool error compatibility text should be JSON");
    assert_eq!(result["structuredContent"], parsed);
    serde_json::from_value::<McpToolErrorResponse>(parsed.clone())
        .expect("structured tool error should match its advertised response type");
    assert_eq!(parsed["reached_core"], false);
    assert_eq!(parsed["committed"], false);
    assert_eq!(
        parsed["reported_issue_count"].as_u64(),
        parsed["issues"]
            .as_array()
            .map(|issues| issues.len() as u64)
    );
    assert!(parsed["truncated"].is_boolean());
    parsed
}

fn tool_error_issue<'a>(response: &'a Value, path: &str, code: &str) -> &'a Value {
    response["issues"]
        .as_array()
        .expect("tool error issues should be an array")
        .iter()
        .find(|issue| issue["path"] == path && issue["code"] == code)
        .unwrap_or_else(|| panic!("missing issue {code} at {path}: {response}"))
}

fn tool_names_from_list_response(response: &Value) -> Vec<&str> {
    response["result"]["tools"]
        .as_array()
        .expect("tools should be an array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>()
}

fn assert_compatible_tool_definitions(tools: &[McpToolDefinition]) {
    if let Err(errors) = validate_tools_list_schema_compatibility(tools) {
        panic!(
            "MCP tool definitions should be client-compatible:\n{}",
            errors.join("\n")
        );
    }
}

fn assert_tools_list_json_client_compatible(tools: &[Value]) {
    if let Err(errors) = validate_tools_list_json_compatibility(tools) {
        panic!(
            "MCP tools/list response should be client-compatible:\n{}",
            errors.join("\n")
        );
    }
}

fn lifecycle_event_names(metadata: &Value) -> Vec<String> {
    metadata["lifecycle_events"]
        .as_array()
        .expect("lifecycle_events should be an array")
        .iter()
        .filter_map(|event| event["lifecycle_event"].as_str().map(str::to_owned))
        .collect()
}

fn lifecycle_event<'a>(metadata: &'a Value, lifecycle_event: &str) -> &'a Value {
    metadata["lifecycle_events"]
        .as_array()
        .expect("lifecycle_events should be an array")
        .iter()
        .find(|event| event["lifecycle_event"] == lifecycle_event)
        .unwrap_or_else(|| panic!("missing lifecycle event {lifecycle_event}: {metadata}"))
}

fn preflight_report_for_fixture(
    fixture: &CoreFixture,
    project_id: Option<&str>,
) -> Result<String, Box<dyn Error>> {
    Ok(preflight_check(
        |name| {
            if name == "VOLICORD_HOME" {
                Some(fixture.runtime_home_path().as_os_str().to_owned())
            } else {
                None
            }
        },
        fixture.runtime_home_path(),
        fixture.connection_id(),
        project_id,
    )?)
}

fn assert_report_line(report: &str, expected: &str) {
    assert!(
        report.lines().any(|line| line == expected),
        "missing report line `{expected}` in:\n{report}"
    );
}

fn read_only_state_version(fixture: &CoreFixture) -> Result<u64, Box<dyn Error>> {
    let state_db_path = fixture
        .runtime_home_path()
        .join("projects")
        .join(fixture.project_id())
        .join("state.sqlite");
    let conn = open_project_state_database_read_only(state_db_path)?;
    Ok(conn.query_row(
        "SELECT state_version FROM project_state WHERE project_id = ?1",
        [fixture.project_id()],
        |row| row.get(0),
    )?)
}

fn read_only_table_count(fixture: &CoreFixture, table: &str) -> Result<i64, Box<dyn Error>> {
    let state_db_path = fixture
        .runtime_home_path()
        .join("projects")
        .join(fixture.project_id())
        .join("state.sqlite");
    let conn = open_project_state_database_read_only(state_db_path)?;
    let sql = format!(
        "SELECT COUNT(*) FROM \"{}\" WHERE project_id = ?1",
        table.replace('"', "\"\"")
    );
    Ok(conn.query_row(&sql, [fixture.project_id()], |row| row.get(0))?)
}

fn root_properties(schema: &Value) -> Vec<String> {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|properties| properties.keys().cloned().collect())
        .unwrap_or_default()
}

fn root_required_fields(schema: &Value) -> Vec<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|required| {
            required
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn schema_has_definition(schema: &Value, name: &str) -> bool {
    schema
        .get("definitions")
        .and_then(Value::as_object)
        .is_some_and(|definitions| definitions.contains_key(name))
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

#[test]
fn workflow_public_tool_names_are_unique() {
    let unique = PUBLIC_METHOD_TOOL_NAMES
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), PUBLIC_METHOD_TOOL_NAMES.len());
}
