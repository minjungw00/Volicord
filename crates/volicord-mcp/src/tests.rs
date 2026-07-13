use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    ffi::OsString,
    fs,
    io::{self, BufReader, Cursor, Write},
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
    classify_launch_origin, pending_user_action_from_response, percent_encode_query,
    run_stdio_with_env_marker, tool_execution_error_result, McpLaunchOrigin,
    MAX_MCP_COMPACT_MUTATION_RESULT_BYTES, MAX_MCP_ELICITATION_WIRE_BYTES,
    MAX_MCP_FULL_MUTATION_RESULT_BYTES, MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES,
};
use crate::{
    routing::McpStorageCapability,
    tool_registry::{
        canonical_tool_examples, mcp_tool_naming_style, mcp_tools_for_mode_and_storage,
        validate_tools_list_json_compatibility, validate_tools_list_schema_compatibility,
        CHECK_CLOSE_MISSING_FINAL_ACCEPTANCE_EXAMPLE_ID,
        GET_OPERATION_RESULT_FIRST_PAGE_EXAMPLE_ID, PREPARE_EVIDENCE_CAPTURE_CONNECTION_EXAMPLE_ID,
        PREPARE_EVIDENCE_CAPTURE_VERIFIED_COMMAND_EXAMPLE_ID,
        PREPARE_EVIDENCE_CAPTURE_VERIFIED_TOOL_EXAMPLE_ID, PREPARE_WRITE_SIMPLE_EXAMPLE_ID,
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
        REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID, STATUS_READ_ONLY_EXAMPLE_ID,
        UPDATE_SCOPE_KEEP_CURRENT_EXAMPLE_ID,
    },
};
use volicord_core::CoreBoundary;
use volicord_store::agent_connections::{
    add_connection_project, agent_connection_record, ensure_agent_connection,
    set_connection_enabled, AgentConnectionRegistration, ConnectionProjectRegistration,
    CONNECTION_MODE_READ_ONLY,
};
use volicord_store::bootstrap::{register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS};
use volicord_store::diagnostics::{diagnostics_db_path, read_diagnostic_session};
use volicord_store::guards::{
    list_unresolved_unrecorded_changes, upsert_guard_installation, GuardInstallationUpsert,
};
use volicord_store::session_watch::{
    latest_watch_baseline_for_connection, latest_watch_baseline_for_session,
};
use volicord_test_support::core_fixtures::{
    CoreFixture, ResolveUserActionFixture, UserActionFixture,
};
use volicord_types::{
    AgentConnectionMode, EvidenceTarget, OperationCategory, VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
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
    assert!(workflow_names.contains(&"volicord.request_user_action"));
    assert!(workflow_names.contains(&"volicord.reconcile_changes"));
    assert!(workflow_names.contains(&PREPARE_EVIDENCE_CAPTURE_TOOL_NAME));
    assert!(workflow_names.contains(&CHECK_CLOSE_TOOL_NAME));
    assert!(workflow_names.contains(&"volicord.close_task"));
    assert!(!workflow_names.contains(&"volicord.resolve_user_action"));
    assert_eq!(
        workflow_names.last().copied(),
        Some(LIST_PROJECTS_TOOL_NAME)
    );

    let read_only = mcp_tools_for_mode(AgentConnectionMode::ReadOnly);
    let read_only_names = tool_names(&read_only);
    assert!(!read_only_names.contains(&PREPARE_EVIDENCE_CAPTURE_TOOL_NAME));
    assert_eq!(
        read_only_names,
        vec![
            "volicord.status",
            GET_OPERATION_RESULT_TOOL_NAME,
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
            GET_OPERATION_RESULT_TOOL_NAME,
            REQUEST_USER_ACTION_TOOL_NAME,
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
        if !matches!(
            tool.name,
            STATUS_TOOL_NAME
                | GET_OPERATION_RESULT_TOOL_NAME
                | CHECK_CLOSE_TOOL_NAME
                | LIST_PROJECTS_TOOL_NAME
        ) {
            assert!(
                schema_has_definition(&tool.output_schema, "McpMutationResponseBudgetExceeded"),
                "{} output schema should cover compact response-budget failures",
                tool.name
            );
            assert!(
                schema_has_definition(&tool.output_schema, "McpMutationPostEffectFailure"),
                "{} output schema should cover post-effect adapter failures",
                tool.name
            );
        }

        let expected_annotations = match tool.name {
            STATUS_TOOL_NAME
            | GET_OPERATION_RESULT_TOOL_NAME
            | CHECK_CLOSE_TOOL_NAME
            | LIST_PROJECTS_TOOL_NAME => McpToolAnnotations {
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: true,
                open_world_hint: false,
            },
            PREPARE_EVIDENCE_CAPTURE_TOOL_NAME
            | PREPARE_WRITE_TOOL_NAME
            | STAGE_ARTIFACT_TOOL_NAME => McpToolAnnotations {
                read_only_hint: false,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
            },
            INTAKE_TOOL_NAME
            | UPDATE_SCOPE_TOOL_NAME
            | RECORD_RUN_TOOL_NAME
            | REQUEST_USER_ACTION_TOOL_NAME
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
fn request_user_action_output_schema_covers_compound_agent_safe_response() {
    let schema = tool_definition(REQUEST_USER_ACTION_TOOL_NAME).output_schema;

    assert!(schema_has_definition(&schema, "RequestUserActionResult"));
    assert!(schema_has_definition(
        &schema,
        "McpRequestUserActionResponse"
    ));
    assert!(schema_has_definition(
        &schema,
        "AgentSafeUserActionResolution"
    ));
    assert!(schema_has_definition(&schema, "AuthorityReceipt"));
    assert!(schema_has_definition(
        &schema,
        "McpRequestUserActionCompactResult"
    ));
    assert!(schema_has_definition(&schema, "McpMutationFullResponse"));
    assert!(schema_has_definition(&schema, "McpMutationSummaryResponse"));
    assert!(schema_has_definition(
        &schema,
        "McpMutationWorkflowResponse"
    ));
    assert!(schema_has_definition(
        &schema,
        "McpAuthoritativeRefreshFailure"
    ));
    assert!(schema_has_definition(
        &schema,
        "McpMutationResponseBudgetExceeded"
    ));
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

    let tool = tool_definition(REQUEST_USER_ACTION_TOOL_NAME);
    let decoded = decode_mcp_arguments_to_value(
        REQUEST_USER_ACTION_TOOL_NAME,
        canonical_example_value(
            REQUEST_USER_ACTION_TOOL_NAME,
            REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
        )?,
    )?;
    let create_schema = schema_variant_by_tag(&tool.input_schema, "operation", "create")
        .expect("request-user-action schema should expose its create variant");
    let create_required = root_required_fields(create_schema);
    for (field, pointer, expected) in [
        ("change_unit_id", "/request/change_unit_id", Value::Null),
        ("expires_at", "/request/expires_at", Value::Null),
    ] {
        assert!(
            !create_required.iter().any(|required| required == field),
            "{REQUEST_USER_ACTION_TOOL_NAME}{pointer} should be omittable"
        );
        assert_eq!(
            create_schema["properties"][field]["default"], expected,
            "{REQUEST_USER_ACTION_TOOL_NAME}{pointer} should advertise its exact omission default"
        );
        assert_eq!(
            decoded.pointer(pointer),
            Some(&expected),
            "{REQUEST_USER_ACTION_TOOL_NAME}{pointer} omission should decode to the advertised default"
        );
    }
    let choice_schema = &tool.input_schema["definitions"]["UserActionChoiceDraft"];
    assert_eq!(
        choice_schema["properties"]["options"]["default"],
        Value::Null
    );
    assert_eq!(decoded["request"]["action"]["options"], Value::Null);
    assert_eq!(decoded["request"]["action"]["affected_refs"], json!([]));
    assert_eq!(
        decoded["request"]["action"]["sensitive_action_scope"],
        Value::Null
    );

    for tool_name in [
        INTAKE_TOOL_NAME,
        UPDATE_SCOPE_TOOL_NAME,
        PREPARE_EVIDENCE_CAPTURE_TOOL_NAME,
        PREPARE_WRITE_TOOL_NAME,
        STAGE_ARTIFACT_TOOL_NAME,
        RECORD_RUN_TOOL_NAME,
        REQUEST_USER_ACTION_TOOL_NAME,
        RECONCILE_CHANGES_TOOL_NAME,
        CLOSE_TASK_TOOL_NAME,
    ] {
        let tool = tool_definition(tool_name);
        assert!(!root_required_fields(&tool.input_schema)
            .iter()
            .any(|field| field == "detail"));
        assert_eq!(
            tool.input_schema["properties"]["detail"]["default"],
            "summary"
        );
        let example = canonical_tool_examples(tool_name)
            .first()
            .expect("mutation tool should advertise an example");
        let decoded = decode_mcp_arguments_to_value(
            tool_name,
            serde_json::from_str(example.arguments_json)?,
        )?;
        let example_detail = if matches!(
            tool_name,
            PREPARE_WRITE_TOOL_NAME | STAGE_ARTIFACT_TOOL_NAME | RECONCILE_CHANGES_TOOL_NAME
        ) {
            "full"
        } else {
            "summary"
        };
        assert_eq!(decoded["detail"], example_detail);
    }

    assert_eq!(
        root_required_fields(&tool_definition(REQUEST_USER_ACTION_TOOL_NAME).input_schema)
            .into_iter()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["request".to_owned()])
    );
    Ok(())
}

#[test]
fn prepare_evidence_capture_arguments_map_strict_variants_and_omission_defaults(
) -> Result<(), Box<dyn Error>> {
    let cases = [
        (
            json!({
                "task_id": "task_capture_command",
                "change_unit_id": "cu_capture_command",
                "baseline_ref": "baseline_capture_command",
                "target": {
                    "target_kind": "acceptance_criterion",
                    "acceptance_criterion_id": "criterion_capture_command"
                },
                "capture": {
                    "capture_kind": "verified_command_execution",
                    "command_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "command_label": "Focused validation"
                }
            }),
            json!({
                "capture_kind": "verified_command_execution",
                "command_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "command_label": "Focused validation",
                "expected_exit_code": null
            }),
            "tool_name",
        ),
        (
            json!({
                "task_id": "task_capture_tool",
                "change_unit_id": "cu_capture_tool",
                "baseline_ref": "baseline_capture_tool",
                "target": {
                    "target_kind": "supplemental_claim",
                    "evidence_claim_id": "claim_capture_tool",
                    "statement": "The focused tool validation succeeds."
                },
                "capture": {
                    "capture_kind": "verified_tool_invocation",
                    "tool_name": "fixture.validator",
                    "tool_input_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                }
            }),
            json!({
                "capture_kind": "verified_tool_invocation",
                "tool_name": "fixture.validator",
                "tool_input_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "expected_success": null
            }),
            "command_label",
        ),
        (
            json!({
                "task_id": "task_capture_connection",
                "change_unit_id": "cu_capture_connection",
                "baseline_ref": "baseline_capture_connection",
                "target": {
                    "target_kind": "acceptance_criterion",
                    "acceptance_criterion_id": "criterion_capture_connection"
                },
                "capture": {
                    "capture_kind": "registered_connection_observation",
                    "source_selector": {"source_kind": "session_watcher"}
                }
            }),
            json!({
                "capture_kind": "registered_connection_observation",
                "source_selector": {"source_kind": "session_watcher"},
                "expected_complete": null
            }),
            "observation_input_sha256",
        ),
    ];

    for (arguments, expected_capture, foreign_field) in cases {
        crate::schema_validation::validate_mcp_tool_arguments(
            PREPARE_EVIDENCE_CAPTURE_TOOL_NAME,
            &arguments,
        )?;
        let decoded: McpPrepareEvidenceCaptureArguments =
            serde_json::from_value(arguments.clone())?;
        assert_eq!(decoded.detail, MutationDetailLevel::Summary);
        let core_capture: volicord_types::EvidenceCaptureSpec = decoded.capture.into();
        assert_eq!(serde_json::to_value(core_capture)?, expected_capture);

        let mut invalid = arguments;
        invalid["capture"][foreign_field] = json!("not allowed for this capture kind");
        assert!(crate::schema_validation::validate_mcp_tool_arguments(
            PREPARE_EVIDENCE_CAPTURE_TOOL_NAME,
            &invalid,
        )
        .is_err());
        assert!(serde_json::from_value::<McpPrepareEvidenceCaptureArguments>(invalid).is_err());
    }

    Ok(())
}

#[test]
fn prepare_evidence_capture_rejects_session_start_selector_before_core_without_effects(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-capture-session-start-selector")?;
    let adapter = adapter(&fixture)?;
    let before = fixture.counts()?;
    let mut arguments = canonical_example_value(
        PREPARE_EVIDENCE_CAPTURE_TOOL_NAME,
        PREPARE_EVIDENCE_CAPTURE_CONNECTION_EXAMPLE_ID,
    )?;
    arguments["capture"]["source_selector"]["event_kind"] = json!("session_start");

    let error = adapter
        .call_tool(PREPARE_EVIDENCE_CAPTURE_TOOL_NAME, arguments)
        .expect_err("session_start must fail before Core intent preparation");
    let response = structured_tool_error(PREPARE_EVIDENCE_CAPTURE_TOOL_NAME, &error);
    tool_error_issue(
        &response,
        "/capture/source_selector/event_kind",
        "MCP_ARGUMENT_ENUM_VALUE",
    );
    assert_eq!(
        fixture.counts()?,
        before,
        "invalid future-ineligible source selection must create no Core effects"
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
            REQUEST_USER_ACTION_TOOL_NAME,
            &["change_unit_id", "expires_at"][..],
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

    let schema = volicord_types::public_request_schema(REQUEST_USER_ACTION_TOOL_NAME)
        .expect("request-user-action public Core schema should exist");
    for field in ["affected_refs", "sensitive_action_scope"] {
        assert!(
            schema_requires_property(&schema, field),
            "{REQUEST_USER_ACTION_TOOL_NAME}.action.{field} should remain a required Core request member"
        );
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
        (
            GET_OPERATION_RESULT_TOOL_NAME,
            &[GET_OPERATION_RESULT_FIRST_PAGE_EXAMPLE_ID],
        ),
        (
            PREPARE_EVIDENCE_CAPTURE_TOOL_NAME,
            &[
                PREPARE_EVIDENCE_CAPTURE_VERIFIED_COMMAND_EXAMPLE_ID,
                PREPARE_EVIDENCE_CAPTURE_VERIFIED_TOOL_EXAMPLE_ID,
                PREPARE_EVIDENCE_CAPTURE_CONNECTION_EXAMPLE_ID,
            ],
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
            REQUEST_USER_ACTION_TOOL_NAME,
            &[
                REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
                "resume_user_action",
            ],
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
    assert_eq!(observation["source_kind"], "agent_report");
    assert_eq!(observation["assurance_level"], "cooperative_report");
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
fn request_user_action_invalid_options_report_option_id_shape() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-invalid-judgment-options")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        REQUEST_USER_ACTION_TOOL_NAME,
        REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
    )?;
    arguments["request"]["action"]["judgment_kind"] = json!("product_decision");
    arguments["request"]["action"]["options"] = json!([
        {
            "id": "accept",
            "label": "Accept",
            "description": "Record the user's selected option.",
            "consequence": "The option is recorded for this judgment.",
            "is_default": true
        }
    ]);

    let error = adapter
        .call_tool(REQUEST_USER_ACTION_TOOL_NAME, arguments)
        .expect_err("invalid options should fail before Core");
    let response = structured_tool_error(REQUEST_USER_ACTION_TOOL_NAME, &error);
    tool_error_issue(
        &response,
        "/request/action/options/0/option_id",
        "MCP_ARGUMENT_REQUIRED",
    );
    tool_error_issue(
        &response,
        "/request/action/options/0/id",
        "MCP_ARGUMENT_UNKNOWN",
    );
    Ok(())
}

#[test]
fn request_user_action_invalid_visible_risk_reports_expected_shape() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-invalid-judgment-visible-risk")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        REQUEST_USER_ACTION_TOOL_NAME,
        REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
    )?;
    arguments["request"]["action"]["context"]["visible_risks"] = json!(["plain risk text"]);

    let error = adapter
        .call_tool(REQUEST_USER_ACTION_TOOL_NAME, arguments)
        .expect_err("invalid visible risk should fail before Core");
    let response = structured_tool_error(REQUEST_USER_ACTION_TOOL_NAME, &error);
    tool_error_issue(
        &response,
        "/request/action/context/visible_risks/0",
        "MCP_ARGUMENT_TYPE_MISMATCH",
    );
    Ok(())
}

#[test]
fn request_user_action_operation_union_rejects_missing_invalid_and_mixed_shapes(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-user-action-operation-union")?;
    let adapter = adapter(&fixture)?;
    let before = fixture.counts()?;
    let create = canonical_example_value(
        REQUEST_USER_ACTION_TOOL_NAME,
        REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
    )?;

    let mut missing = create.clone();
    missing["request"]
        .as_object_mut()
        .expect("create request should be an object")
        .remove("operation");
    let mut invalid = create.clone();
    invalid["request"]["operation"] = json!("reopen");
    let mut mixed_create = create;
    mixed_create["request"]["user_action_request_id"] = json!("uar_mixed_create");
    let mixed_resume = json!({
        "request": {
            "operation": "resume",
            "user_action_request_id": "uar_mixed_resume",
            "task_id": "task_mixed_resume"
        }
    });

    for (arguments, path, code) in [
        (missing, "/request/operation", "MCP_ARGUMENT_REQUIRED"),
        (invalid, "/request/operation", "MCP_ARGUMENT_ENUM_VALUE"),
        (
            mixed_create,
            "/request/user_action_request_id",
            "MCP_ARGUMENT_UNKNOWN",
        ),
        (mixed_resume, "/request/task_id", "MCP_ARGUMENT_UNKNOWN"),
    ] {
        let error = adapter
            .call_tool(REQUEST_USER_ACTION_TOOL_NAME, arguments)
            .expect_err("invalid create/resume union shape should fail before Core");
        let response = structured_tool_error(REQUEST_USER_ACTION_TOOL_NAME, &error);
        tool_error_issue(&response, path, code);
    }
    assert_eq!(fixture.counts()?, before);
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
        REQUEST_USER_ACTION_TOOL_NAME,
        REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
    )?;
    arguments["request"]["expires_at"] = json!("not-a-timestamp");

    let error = adapter
        .call_tool(REQUEST_USER_ACTION_TOOL_NAME, arguments)
        .expect_err("invalid timestamp format should fail typed decoding");
    let response = structured_tool_error(REQUEST_USER_ACTION_TOOL_NAME, &error);

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
        REQUEST_USER_ACTION_TOOL_NAME,
        REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
    )?;
    arguments["request"]["expires_at"] = json!("not-a-timestamp");

    let error = adapter
        .call_tool(REQUEST_USER_ACTION_TOOL_NAME, arguments)
        .expect_err("typed argument decoding should precede storage preconditions");
    let response = structured_tool_error(REQUEST_USER_ACTION_TOOL_NAME, &error);

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
    assert!(names.contains(&GET_OPERATION_RESULT_TOOL_NAME));
    assert!(names.contains(&REQUEST_USER_ACTION_TOOL_NAME));
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
            GET_OPERATION_RESULT_TOOL_NAME,
            REQUEST_USER_ACTION_TOOL_NAME,
            CHECK_CLOSE_TOOL_NAME,
            LIST_PROJECTS_TOOL_NAME
        ]
    );
    assert!(responses[1].get("error").is_none());
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_readonly_storage_exposes_read_tools_and_user_action_resume() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readonly-exposes-read-tools")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

    let names = tool_names(&adapter.tools()?);

    assert!(names.contains(&STATUS_TOOL_NAME));
    assert!(names.contains(&GET_OPERATION_RESULT_TOOL_NAME));
    assert!(names.contains(&LIST_PROJECTS_TOOL_NAME));
    assert!(names.contains(&CHECK_CLOSE_TOOL_NAME));
    assert!(names.contains(&REQUEST_USER_ACTION_TOOL_NAME));
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

#[test]
fn stdio_operation_result_retrieval_is_exact_bounded_and_read_only_visible(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-operation-result-exact-page")?;
    let setup_adapter = adapter(&fixture)?;
    let committed = setup_adapter.call_tool(INTAKE_TOOL_NAME, intake_args(None))?;
    let operation_result_ref = committed
        .operation_result_ref
        .clone()
        .ok_or("committed agent-workflow result should expose a lookup ref")?;
    set_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;
    let read_only_adapter = adapter(&fixture)?;
    assert!(tool_names(&read_only_adapter.tools()?).contains(&GET_OPERATION_RESULT_TOOL_NAME));
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            GET_OPERATION_RESULT_TOOL_NAME,
            json!({ "operation_result_ref": operation_result_ref.clone() }),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(read_only_adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let result = &responses[1]["result"];
    let structured = &result["structuredContent"];
    assert_eq!(result["isError"], false);
    assert_eq!(structured["base"]["response_kind"], "result");
    assert_eq!(structured["start_offset_bytes"], 0);
    assert_eq!(structured["complete"], true);
    assert!(structured["next_cursor"].is_null());
    assert_eq!(structured["chunk_utf8"], committed.response_json);
    assert_eq!(structured["historical"], true);
    assert_eq!(structured["current_authority_refresh_required"], true);
    let primary_text = result["content"][0]["text"]
        .as_str()
        .ok_or("operation-result compatibility text should be present")?;
    assert!(primary_text.len() <= MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES);
    assert!(serde_json::from_str::<Value>(primary_text).is_err());
    assert!(!primary_text.contains(
        structured["chunk_utf8"]
            .as_str()
            .ok_or("chunk_utf8 should be a string")?
    ));
    assert!(serde_json::to_vec(result)?.len() <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);

    let stale_adapter = adapter(&fixture)?;
    set_connection_enabled(fixture.runtime_home_path(), fixture.connection_id(), false)?;
    let disabled = stale_adapter
        .call_tool(
            GET_OPERATION_RESULT_TOOL_NAME,
            json!({ "operation_result_ref": operation_result_ref }),
        )
        .expect_err("every result page should recheck current connection access");
    assert!(
        disabled.to_string().contains("disabled"),
        "unexpected disabled-connection error: {disabled}"
    );
    Ok(())
}

#[test]
fn stdio_budget_omission_reconstructs_exact_result_after_state_advance(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-operation-result-budget-chain")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, _) = create_task(&setup_adapter)?;
    let mut next_request_id = 10_u64;
    let mut call_stdio = |tool_name: &str, arguments: Value| -> Result<Value, Box<dyn Error>> {
        let initialize_id = next_request_id;
        let tool_id = next_request_id + 1;
        next_request_id += 2;
        let input = Cursor::new(json_lines(&[
            initialize_request(initialize_id, json!({})),
            initialized_notification(),
            tools_call(tool_id, tool_name, arguments),
        ])?);
        let mut output = Vec::new();
        run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;
        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 2);
        Ok(responses[1].clone())
    };

    let bounded_unicode_text =
        |label: &str, index: usize| format!("{label}-{index}:{}", "결과🙂".repeat(2_000));
    let omitted_exact_marker = "OMITTED_EXACT_OPERATION_RESULT_MARKER";
    let goal_summary = format!("{}:{omitted_exact_marker}", bounded_unicode_text("goal", 0));
    let scope_boundary = bounded_unicode_text("scope", 0);
    let non_goals = (0..6)
        .map(|index| bounded_unicode_text("non-goal", index))
        .collect::<Vec<_>>();
    let acceptance_criteria = (0..6)
        .map(|index| {
            json!({
                "acceptance_criterion_id": null,
                "statement": bounded_unicode_text("criterion", index),
                "evidence_requirement": "required"
            })
        })
        .collect::<Vec<_>>();
    let autonomy_boundary = bounded_unicode_text("autonomy", 0);
    let change_unit_summary = bounded_unicode_text("change-unit", 0);

    let omitted = call_stdio(
        UPDATE_SCOPE_TOOL_NAME,
        json!({
            "detail": "full",
            "task_id": task_id,
            "goal_summary": goal_summary,
            "scope_boundary": scope_boundary,
            "non_goals": non_goals,
            "acceptance_criteria": acceptance_criteria,
            "autonomy_boundary": autonomy_boundary,
            "change_unit": {
                "operation": "create_current",
                "scope_summary": change_unit_summary,
                "affected_paths": ["src/operation-result.rs"]
            }
        }),
    )?;
    let omitted_result = &omitted["result"];
    let omitted_structured = &omitted_result["structuredContent"];
    assert_eq!(omitted_result["isError"], false);
    assert_eq!(omitted_structured["code"], "MCP_RESPONSE_BUDGET_EXCEEDED");
    assert_eq!(omitted_structured["requested_detail"], "full");
    assert_eq!(omitted_structured["reached_core"], true);
    assert_eq!(omitted_structured["committed"], true);
    assert_eq!(omitted_structured["effect_applied"], true);
    assert_eq!(omitted_structured["response_projection_omitted"], true);
    assert_eq!(omitted_structured["status_read_required"], true);
    assert!(omitted_structured["method_result"].get("state").is_none());
    assert!(serde_json::to_vec(omitted_result)?.len() <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);
    assert!(!serde_json::to_string(&omitted)?.contains(omitted_exact_marker));

    let operation_result_ref_value = omitted_structured["operation_result_ref"].clone();
    let operation_result_ref: OperationResultRef =
        serde_json::from_value(operation_result_ref_value.clone())?;
    assert_eq!(operation_result_ref.source_method, MethodName::UpdateScope);
    assert_eq!(
        omitted_structured["authority_receipt"]["state_version"],
        operation_result_ref.committed_state_version
    );
    let stored = fixture
        .store()?
        .operation_result(
            operation_result_ref.source_method,
            &operation_result_ref.source_idempotency_key,
        )?
        .ok_or("budget-omitted exact result should remain in the replay row")?;
    assert_eq!(
        stored.response_size_bytes,
        operation_result_ref.response_size_bytes
    );
    assert_eq!(stored.response_sha256, operation_result_ref.response_sha256);
    assert!(stored.response_json.len() > MAX_MCP_FULL_MUTATION_RESULT_BYTES);
    assert!(stored.response_json.contains(omitted_exact_marker));

    let advanced = call_stdio(
        UPDATE_SCOPE_TOOL_NAME,
        json!({
            "task_id": task_id,
            "change_unit": { "operation": "keep_current" }
        }),
    )?;
    let advanced_structured = &advanced["result"]["structuredContent"];
    assert_eq!(advanced["result"]["isError"], false);
    assert!(advanced_structured.get("code").is_none());
    let advanced_state_version = advanced_structured["authority_receipt"]["state_version"]
        .as_u64()
        .ok_or("state-advance receipt should expose state_version")?;
    assert!(advanced_state_version > operation_result_ref.committed_state_version);
    let after_advance = fixture.counts()?;

    let mut cursor = None;
    let mut reconstructed = String::new();
    let mut pages = 0_usize;
    loop {
        let mut arguments = json!({
            "operation_result_ref": operation_result_ref_value.clone()
        });
        if let Some(next_cursor) = cursor.take() {
            arguments["cursor"] = Value::String(next_cursor);
        }
        let response = call_stdio(GET_OPERATION_RESULT_TOOL_NAME, arguments)?;
        let result = &response["result"];
        let page = &result["structuredContent"];
        assert_eq!(result["isError"], false);
        assert_eq!(page["base"]["response_kind"], "result");
        assert_eq!(page["base"]["effect_kind"], "read_only");
        assert_eq!(page["operation_result_ref"], operation_result_ref_value);
        assert_eq!(page["start_offset_bytes"], reconstructed.len() as u64);
        let chunk = page["chunk_utf8"]
            .as_str()
            .ok_or("operation-result page should contain UTF-8 text")?;
        assert!(chunk.len() <= volicord_types::MAX_OPERATION_RESULT_PAGE_BYTES);
        reconstructed.push_str(chunk);
        assert_eq!(page["end_offset_bytes"], reconstructed.len() as u64);
        assert_eq!(page["historical"], true);
        assert_eq!(page["current_authority_refresh_required"], true);
        assert!(serde_json::to_vec(result)?.len() <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);
        pages += 1;
        assert!(
            pages < 100,
            "bounded retrieval should make forward progress"
        );
        if page["complete"] == true {
            assert!(page["next_cursor"].is_null());
            break;
        }
        cursor = Some(
            page["next_cursor"]
                .as_str()
                .ok_or("incomplete operation-result page should expose a cursor")?
                .to_owned(),
        );
    }
    assert!(pages > 1);
    assert_eq!(reconstructed.as_bytes(), stored.response_json.as_bytes());

    let status = call_stdio(
        STATUS_TOOL_NAME,
        json!({ "detail": "summary", "task_id": task_id }),
    )?;
    let status_structured = &status["result"]["structuredContent"];
    assert_eq!(status["result"]["isError"], false);
    assert_eq!(
        status_structured["authority_receipt"]["state_version"],
        advanced_state_version
    );
    assert_eq!(fixture.counts()?, after_advance);
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
fn readonly_degraded_user_action_tool_rejects_create_but_allows_exact_resume(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readonly-user-action-resume")?;
    let adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&adapter)?;
    let created = adapter.call_tool(
        REQUEST_USER_ACTION_TOOL_NAME,
        product_action_args(&fixture, &task_id, state_version),
    )?;
    assert!(!created.replayed);
    let exact_origin = created.response_value.clone();
    let exact_operation_result_ref = created.operation_result_ref.clone();
    let user_action_request_id = created.response_value["user_action_request_ref"]["record_id"]
        .as_str()
        .ok_or("request-user-action result should identify its request")?
        .to_owned();
    let before_version = read_only_state_version(&fixture)?;
    let before_events = read_only_table_count(&fixture, "task_events")?;
    let before_invocations = read_only_table_count(&fixture, "tool_invocations")?;
    let before_requests = read_only_table_count(&fixture, "user_action_requests")?;
    let _guard = make_project_state_readonly(&fixture)?;

    assert!(tool_names(&adapter.tools()?).contains(&REQUEST_USER_ACTION_TOOL_NAME));
    let rejected_create = adapter.call_tool(
        REQUEST_USER_ACTION_TOOL_NAME,
        product_action_args(&fixture, &task_id, before_version),
    )?;
    assert_eq!(
        rejected_create.response_value["base"]["response_kind"],
        "rejected"
    );
    assert_eq!(
        rejected_create.response_value["errors"][0]["code"],
        "MCP_UNAVAILABLE"
    );

    let resumed = adapter.call_tool(
        REQUEST_USER_ACTION_TOOL_NAME,
        resume_user_action_args(&fixture, &user_action_request_id),
    )?;
    assert!(resumed.replayed);
    assert_eq!(resumed.response_value, exact_origin);
    assert_eq!(resumed.operation_result_ref, exact_operation_result_ref);
    assert_eq!(read_only_state_version(&fixture)?, before_version);
    assert_eq!(
        read_only_table_count(&fixture, "task_events")?,
        before_events
    );
    assert_eq!(
        read_only_table_count(&fixture, "tool_invocations")?,
        before_invocations
    );
    assert_eq!(
        read_only_table_count(&fixture, "user_action_requests")?,
        before_requests
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

    let cases = [
        (
            INTAKE_TOOL_NAME,
            json!({
                "plain_language_request": "Exercise read-only rejection.",
                "requested_mode": "work",
                "resume_policy": "create_new",
                "acceptance_policy": null,
                "lineage": null,
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
        ),
        (
            PREPARE_EVIDENCE_CAPTURE_TOOL_NAME,
            json!({
                "task_id": "task_read_only_capture",
                "change_unit_id": "cu_read_only_capture",
                "baseline_ref": "baseline_read_only_capture",
                "target": {
                    "target_kind": "acceptance_criterion",
                    "acceptance_criterion_id": "criterion_read_only_capture"
                },
                "capture": {
                    "capture_kind": "verified_command_execution",
                    "command_sha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                    "command_label": "Read-only rejection validation"
                }
            }),
        ),
    ];

    for (tool_name, arguments) in cases {
        let error = adapter
            .call_tool(tool_name, arguments)
            .expect_err("read_only should reject agent workflow calls");
        assert!(error.to_string().contains("mode read_only"));
        assert!(error.to_string().contains("agent_workflow"));
    }
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
            GET_OPERATION_RESULT_TOOL_NAME,
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
fn mutation_detail_shapes_compact_receipt_workflow_and_full_without_json_text_duplication(
) -> Result<(), Box<dyn Error>> {
    fn intake_result(prefix: &str, detail: Option<&str>) -> Result<Value, Box<dyn Error>> {
        let fixture = CoreFixture::new(prefix)?;
        let adapter = adapter(&fixture)?;
        let mut arguments = intake_args(None);
        if let Some(detail) = detail {
            arguments["detail"] = json!(detail);
        }
        let input = Cursor::new(json_lines(&[
            initialize_request(1, json!({})),
            initialized_notification(),
            tools_call(2, INTAKE_TOOL_NAME, arguments),
        ])?);
        let mut output = Vec::new();
        run_stdio(adapter, BufReader::new(input), &mut output)?;
        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 2);
        Ok(responses[1]["result"].clone())
    }

    let summary = intake_result("mcp-mutation-summary", None)?;
    assert_eq!(summary["isError"], false);
    let summary_keys = summary["structuredContent"]
        .as_object()
        .expect("summary receipt")
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        summary_keys,
        BTreeSet::from(["authority_receipt", "method_result", "operation_result_ref",])
    );
    assert!(summary["structuredContent"]["operation_result_ref"].is_object());
    assert_eq!(
        summary["structuredContent"]["method_result"]["effect_kind"],
        "core_committed"
    );
    assert!(summary["structuredContent"]["authority_receipt"]["state_version"].is_u64());
    let summary_text = summary["content"][0]["text"]
        .as_str()
        .expect("summary compatibility text");
    assert!(summary_text.contains("authority receipt"));
    assert!(summary_text.len() <= MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES);
    assert!(serde_json::from_str::<Value>(summary_text).is_err());
    assert!(serde_json::to_vec(&summary)?.len() <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);

    let workflow = intake_result("mcp-mutation-workflow", Some("workflow"))?;
    let workflow_keys = workflow["structuredContent"]
        .as_object()
        .expect("workflow receipt")
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        workflow_keys,
        BTreeSet::from([
            "authority_receipt",
            "method_result",
            "next_actions",
            "operation_result_ref",
        ])
    );
    assert!(workflow["structuredContent"]["next_actions"].is_array());
    assert!(serde_json::to_vec(&workflow)?.len() <= MAX_MCP_COMPACT_MUTATION_RESULT_BYTES);

    let full = intake_result("mcp-mutation-full", Some("full"))?;
    assert_eq!(
        full["structuredContent"]["method_result"]["base"]["response_kind"],
        "result"
    );
    assert!(full["structuredContent"]["method_result"]["state"].is_object());
    assert!(full["structuredContent"]["authority_receipt"]["state_version"].is_u64());
    let full_text = full["content"][0]["text"]
        .as_str()
        .expect("full compatibility text");
    assert!(full_text.len() <= MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES);
    assert!(serde_json::from_str::<Value>(full_text).is_err());
    assert!(serde_json::to_vec(&full)?.len() <= MAX_MCP_FULL_MUTATION_RESULT_BYTES);
    Ok(())
}

#[test]
fn default_compact_mutations_preserve_tool_essential_method_results() -> Result<(), Box<dyn Error>>
{
    fn call_default(
        fixture: &CoreFixture,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, Box<dyn Error>> {
        let input = Cursor::new(json_lines(&[
            initialize_request(1, json!({})),
            initialized_notification(),
            tools_call(2, tool_name, arguments),
        ])?);
        let mut output = Vec::new();
        run_stdio(adapter(fixture)?, BufReader::new(input), &mut output)?;
        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[1]["result"]["isError"], false);
        Ok(responses[1]["result"]["structuredContent"].clone())
    }

    let stage_fixture = CoreFixture::new("mcp-default-compact-stage")?;
    let (stage_task_id, _) = create_task(&adapter(&stage_fixture)?)?;
    let staged = call_default(
        &stage_fixture,
        STAGE_ARTIFACT_TOOL_NAME,
        json!({
            "task_id": stage_task_id,
            "display_name": "default-stage.log",
            "content_type": "text/plain",
            "redaction_state": "none",
            "safe_bytes_or_notice": "Default compact staging result."
        }),
    )?;
    assert_eq!(
        staged["method_result"]["effect"]["effect_kind"],
        "staging_created"
    );
    assert!(staged["method_result"]["staged_artifact_handle"]["handle_id"].is_string());
    assert!(staged["method_result"]["expires_at"].is_string());

    let capture_fixture = CoreFixture::new("mcp-default-compact-evidence-capture")?;
    let capture_git_dir = capture_fixture.product_repo_path().join(".git");
    fs::create_dir_all(&capture_git_dir)?;
    fs::write(capture_git_dir.join("HEAD"), "ref: refs/heads/main\n")?;
    let capture_adapter = adapter(&capture_fixture)?;
    let (capture_task_id, _) = create_task(&capture_adapter)?;
    let capture_scope = capture_adapter.call_tool(
        UPDATE_SCOPE_TOOL_NAME,
        json!({
            "task_id": capture_task_id,
            "goal_summary": null,
            "scope_update": null,
            "scope_boundary": null,
            "non_goals": null,
            "acceptance_criteria": null,
            "autonomy_boundary": null,
            "baseline_ref": "baseline_capture_compact",
            "change_unit": {
                "operation": "create_current",
                "scope_summary": "Prepare a registered evidence capture.",
                "affected_paths": []
            },
            "related_scope_decision_refs": []
        }),
    )?;
    let capture_change_unit_id = capture_scope.response_value["state"]["active_change_unit_ref"]
        ["record_id"]
        .as_str()
        .ok_or("scope response should expose the current Change Unit")?;
    let capture_criterion_id = capture_scope.response_value["state"]["acceptance_criteria"][0]
        ["acceptance_criterion_id"]
        .as_str()
        .ok_or("scope response should expose the acceptance criterion")?;
    let capture = call_default(
        &capture_fixture,
        PREPARE_EVIDENCE_CAPTURE_TOOL_NAME,
        json!({
            "task_id": capture_task_id,
            "change_unit_id": capture_change_unit_id,
            "baseline_ref": "baseline_capture_compact",
            "target": {
                "target_kind": "acceptance_criterion",
                "acceptance_criterion_id": capture_criterion_id
            },
            "capture": {
                "capture_kind": "verified_command_execution",
                "command_sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                "command_label": "Focused compact projection validation"
            }
        }),
    )?;
    assert_eq!(
        capture["method_result"]["effect"]["effect_kind"], "core_committed",
        "unexpected compact prepare_evidence_capture result: {capture:#}"
    );
    assert_eq!(
        capture["method_result"]["capture_intent_ref"]["record_kind"],
        "evidence_capture_intent"
    );
    assert_eq!(
        capture["method_result"]["capture_intent"]["capture"]["expected_exit_code"],
        0
    );
    assert!(capture["method_result"]["expires_at"].is_string());
    assert!(capture["authority_receipt"].is_object());
    assert_eq!(
        capture["operation_result_ref"]["source_method"],
        PREPARE_EVIDENCE_CAPTURE_TOOL_NAME
    );

    let record_fixture = CoreFixture::new("mcp-default-compact-record-run")?;
    let record_adapter = adapter(&record_fixture)?;
    let (record_task_id, _) = create_task(&record_adapter)?;
    let scope = record_adapter.call_tool(
        UPDATE_SCOPE_TOOL_NAME,
        json!({
            "task_id": record_task_id,
            "goal_summary": null,
            "scope_update": null,
            "scope_boundary": null,
            "non_goals": null,
            "acceptance_criteria": null,
            "autonomy_boundary": null,
            "baseline_ref": "baseline_record_compact",
            "change_unit": {
                "operation": "create_current",
                "scope_summary": "Record compact Run references.",
                "affected_paths": []
            },
            "related_scope_decision_refs": []
        }),
    )?;
    let change_unit_id = scope.response_value["state"]["active_change_unit_ref"]["record_id"]
        .as_str()
        .ok_or("scope response should expose the current Change Unit")?;
    let criterion_id = scope.response_value["state"]["acceptance_criteria"][0]
        ["acceptance_criterion_id"]
        .as_str()
        .ok_or("scope response should expose the acceptance criterion")?;
    let staged_for_run = record_adapter.call_tool(
        STAGE_ARTIFACT_TOOL_NAME,
        json!({
            "task_id": record_task_id,
            "display_name": "record-compact.log",
            "content_type": "text/plain",
            "redaction_state": "none",
            "safe_bytes_or_notice": "Evidence attachment for compact record_run refs."
        }),
    )?;
    let staged_handle = staged_for_run.response_value["staged_artifact_handle"].clone();
    let target = json!({
        "target_kind": "acceptance_criterion",
        "acceptance_criterion_id": criterion_id,
    });
    let recorded = call_default(
        &record_fixture,
        RECORD_RUN_TOOL_NAME,
        json!({
            "task_id": record_task_id,
            "change_unit_id": change_unit_id,
            "kind": "implementation",
            "baseline_ref": "baseline_record_compact",
            "summary": "Recorded compact follow-up references.",
            "observed_changes": {
                "changed_paths": [],
                "product_file_write_observed": false,
                "sensitive_categories": [],
                "baseline_ref": "baseline_record_compact"
            },
            "artifact_inputs": [{
                "artifact_input_id": "artifact_input_record_compact",
                "source_kind": "staged_artifact",
                "staged_artifact_handle": staged_handle,
                "existing_artifact_ref": null,
                "relation_hint": null,
                "evidence_target": target.clone(),
                "expected_sha256": null,
                "expected_size_bytes": null,
                "redaction_state": "none"
            }],
            "evidence_updates": [{
                "target": target.clone(),
                "coverage_state": "supported"
            }],
            "evidence_observations": [{
                "target": target,
                "source_kind": "agent_report",
                "assurance_level": "cooperative_report",
                "observed_at": "2026-07-13T00:00:00Z"
            }],
            "close_assessment": {
                "result_summary": "Recorded compact follow-up references.",
                "result_refs": [],
                "residual_risks": [],
                "sensitive_categories": [],
                "recovery_constraints": []
            }
        }),
    )?;
    let record_result = &recorded["method_result"];
    assert_eq!(
        record_result["effect"]["effect_kind"], "core_committed",
        "unexpected compact record_run result: {recorded:#}"
    );
    assert_eq!(record_result["run_ref"]["record_kind"], "run");
    assert!(record_result["run_ref"]["record_id"].is_string());
    assert_eq!(
        record_result["run_ref"]["project_id"],
        record_fixture.project_id()
    );
    assert_eq!(record_result["run_ref"]["task_id"], record_task_id);
    assert_eq!(
        record_result["run_ref"]["produced_at_state_version"],
        record_result["effect"]["state_version"]
    );
    assert_eq!(
        record_result["registered_artifact_refs"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        record_result["evidence_observation_refs"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        record_result["evidence_producer_refs"]
            .as_array()
            .map(Vec::len),
        Some(0),
        "a cooperative observation must not invent a producer ref"
    );
    assert_eq!(
        record_result["evidence_observation_refs"][0]["record_kind"],
        "evidence_observation"
    );
    assert_eq!(
        record_result["evidence_observation_refs"][0]["project_id"],
        record_fixture.project_id()
    );
    assert_eq!(
        record_result["evidence_observation_refs"][0]["task_id"],
        record_task_id
    );
    assert_eq!(
        record_result["evidence_observation_refs"][0]["produced_at_state_version"],
        record_result["effect"]["state_version"]
    );
    assert_eq!(
        record_result["registered_artifact_refs"][0]["created_by_run_ref"],
        record_result["run_ref"]
    );
    assert!(record_result["close_basis_anchor"]["close_basis_revision"].is_u64());
    assert!(record_result["close_basis_anchor"]["scope_revision"].is_u64());
    assert_eq!(
        record_result["close_basis_anchor"]["source_run_ref"],
        record_result["run_ref"]
    );
    assert_eq!(
        record_result["close_basis_anchor"]["evidence_summary_ref"]["record_kind"],
        "evidence_summary"
    );
    assert_eq!(
        record_result["close_basis_anchor"]["evidence_summary_ref"]["project_id"],
        record_fixture.project_id()
    );
    assert_eq!(
        record_result["close_basis_anchor"]["evidence_summary_ref"]["task_id"],
        record_task_id
    );
    assert_eq!(
        record_result["close_basis_anchor"]["evidence_summary_ref"]["produced_at_state_version"],
        record_result["effect"]["state_version"]
    );

    let prepare_fixture = CoreFixture::new("mcp-default-compact-prepare")?;
    let prepare_adapter = adapter(&prepare_fixture)?;
    let (prepare_task_id, _) = create_task(&prepare_adapter)?;
    let scope = prepare_adapter.call_tool(
        UPDATE_SCOPE_TOOL_NAME,
        json!({
            "task_id": prepare_task_id,
            "goal_summary": null,
            "scope_update": null,
            "scope_boundary": null,
            "non_goals": null,
            "acceptance_criteria": null,
            "autonomy_boundary": null,
            "baseline_ref": "baseline_fixture",
            "change_unit": {
                "operation": "create_current",
                "scope_summary": "Default compact write ticket.",
                "affected_paths": ["src/export.rs"]
            },
            "related_scope_decision_refs": []
        }),
    )?;
    assert_eq!(scope.response_value["base"]["response_kind"], "result");
    let prepared = call_default(
        &prepare_fixture,
        PREPARE_WRITE_TOOL_NAME,
        json!({
            "task_id": prepare_task_id,
            "change_unit_id": null,
            "intended_operation": "Update the export flow.",
            "intended_paths": ["src/export.rs"],
            "product_file_write_intended": true,
            "sensitive_categories": [],
            "baseline_ref": "baseline_fixture"
        }),
    )?;
    assert_eq!(
        prepared["method_result"]["decision"], "allowed",
        "unexpected prepare-write result: {prepared:#}"
    );
    assert_eq!(prepared["method_result"]["write_ticket_effect"], "issued");
    assert!(prepared["method_result"]["write_ticket_id"].is_string());
    assert_eq!(
        prepared["method_result"]["write_ticket"]["path_patterns"]["allowed"],
        json!(["src/export.rs"])
    );

    let reconcile_fixture = CoreFixture::new("mcp-default-compact-reconcile")?;
    let (reconcile_task_id, _) = create_task(&adapter(&reconcile_fixture)?)?;
    let reconciled = call_default(
        &reconcile_fixture,
        RECONCILE_CHANGES_TOOL_NAME,
        json!({"task_id": reconcile_task_id}),
    )?;
    assert!(reconciled["method_result"]["unresolved_changes"].is_array());
    assert!(reconciled["method_result"]["resolved_changes"].is_array());
    assert!(reconciled["method_result"]["rejected_resolution_requests"].is_array());
    Ok(())
}

#[test]
fn compact_close_mutation_receipt_refreshes_the_current_blocked_state() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-compact-terminal-close")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, _) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            CLOSE_TASK_TOOL_NAME,
            json!({
                "task_id": task_id,
                "intent": "cancel",
                "close_reason": "cancelled",
                "superseding_task_id": null,
                "user_note": null
            }),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let result = &responses[1]["result"];
    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["authority_receipt"]["close_state"],
        "blocked"
    );
    assert_eq!(
        result["structuredContent"]["authority_receipt"]["task_ref"]["record_id"],
        task_id
    );
    assert!(
        result["structuredContent"]["authority_receipt"]["state_version"]
            .as_u64()
            .is_some_and(|version| version >= 1)
    );
    assert!(serde_json::from_str::<Value>(
        result["content"][0]["text"]
            .as_str()
            .expect("compatibility text")
    )
    .is_err());
    Ok(())
}

#[test]
fn stdio_elicitation_accept_resolves_user_action_with_agent_safe_summary(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-accept")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_action",
            default_product_action_args(&fixture, &task_id, state_version),
        ),
        elicitation_accept("keep", Some("private-note-must-not-enter-agent-output")),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 3);
    assert_eq!(values[1]["method"], ELICITATION_CREATE_METHOD);
    assert_eq!(values[1]["id"], "elicit_user_action_1");
    assert_eq!(
        values[1]["params"]["requestedSchema"]["additionalProperties"],
        false
    );
    assert_eq!(
        values[1]["params"]["requestedSchema"]["properties"]["selected_option_id"]["enum"][0],
        "keep"
    );
    let elicitation_message = values[1]["params"]["message"]
        .as_str()
        .ok_or("elicitation message should be present")?;
    for expected in [
        "Keep focused behavior",
        "Record the user-owned product decision to keep the behavior.",
        "Only this focused judgment is resolved.",
        "is_default: true",
        "Change focused behavior",
        "is_default: false",
        "Note max characters: 1000",
    ] {
        assert!(
            elicitation_message.contains(expected),
            "elicitation must display {expected}"
        );
    }
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(response["effect"]["effect_kind"], "core_committed");
    assert_eq!(response["status"], "resolved");
    assert_eq!(response["resolution_summary"]["resolution_type"], "choice");
    assert_eq!(response["resolution_summary"]["selected_option_id"], "keep");
    assert_eq!(
        response["resolution_summary"]["selected_option_label"],
        "Keep focused behavior"
    );
    assert_eq!(
        response["resolution_summary"]["resolution_outcome"],
        "accepted"
    );
    assert!(response["derived_refs"].as_array().is_some_and(|refs| refs
        .iter()
        .any(|record_ref| { record_ref["record_kind"] == "project_continuity_record" })));
    assert!(response.get("note").is_none());
    assert!(!serde_json::to_string(&response)?.contains("private-note-must-not-enter-agent-output"));
    assert_eq!(
        stored_resolution_basis(&fixture, &task_id, &response)?,
        VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
    );
    let diagnostics = read_diagnostic_session(fixture.runtime_home_path(), None)?
        .expect("stdio tool call should create bounded diagnostics");
    assert_eq!(diagnostics.totals.tool_call_count, 1);
    assert_eq!(diagnostics.totals.core_reached_count, 1);
    assert_eq!(diagnostics.totals.core_committed_count, 1);
    assert_eq!(diagnostics.user_channel_counts["mcp_elicitation"], 1);
    assert!(diagnostics.fallback_counts.is_empty());
    let diagnostics_bytes = fs::read(diagnostics_db_path(fixture.runtime_home_path()))?;
    assert!(!String::from_utf8_lossy(&diagnostics_bytes)
        .contains("private-note-must-not-enter-agent-output"));
    Ok(())
}

#[test]
fn elicitation_wire_budget_accepts_exact_line_and_falls_back_at_next_byte_without_partial_form(
) -> Result<(), Box<dyn Error>> {
    fn set_first_consequence(response: &mut PipelineResponse, consequence: String) {
        response.response_value["user_action_request"]["body"]["options"][0]["consequence"] =
            json!(consequence);
        response.response_value["inbox_item"]["form"]["choices"][0]["consequence"] = response
            .response_value["user_action_request"]["body"]["options"][0]["consequence"]
            .clone();
    }

    let fixture = CoreFixture::new("mcp-elicitation-whole-line-boundary")?;
    let (_task_id, pending_response) = create_pending_product_action(&fixture)?;
    let mut base_response = pending_response.clone();
    set_first_consequence(&mut base_response, "x".to_owned());
    let base_pending = pending_user_action_from_response(&base_response)?
        .ok_or("boundary response should remain pending")?;
    let base_request =
        crate::stdio::elicitation_create_request("elicit_user_action_1", &base_pending)?
            .ok_or("base request should fit")?;
    let base_wire_bytes = serde_json::to_vec(&base_request)?.len() + 1;
    assert!(base_wire_bytes < MAX_MCP_ELICITATION_WIRE_BYTES);
    let exact_consequence_len = 1 + (MAX_MCP_ELICITATION_WIRE_BYTES - base_wire_bytes);

    let mut exact_response = pending_response.clone();
    set_first_consequence(&mut exact_response, "x".repeat(exact_consequence_len));
    let exact_pending = pending_user_action_from_response(&exact_response)?
        .ok_or("exact-fit response should remain pending")?;
    exact_pending.inbox_item.form.validate_canonical_size()?;
    assert_eq!(
        exact_pending.request.body.capture_form()?,
        exact_pending.inbox_item.form
    );
    let exact_request =
        crate::stdio::elicitation_create_request("elicit_user_action_1", &exact_pending)?
            .ok_or("the exact whole-line boundary must fit")?;
    assert_eq!(
        serde_json::to_vec(&exact_request)?.len() + 1,
        MAX_MCP_ELICITATION_WIRE_BYTES,
        "the budget includes the complete serialized JSON object and trailing LF"
    );

    let mut over_response = pending_response;
    set_first_consequence(&mut over_response, "x".repeat(exact_consequence_len + 1));
    let over_pending = pending_user_action_from_response(&over_response)?
        .ok_or("one-byte-over response should remain pending")?;
    over_pending.inbox_item.form.validate_canonical_size()?;
    assert!(
        crate::stdio::elicitation_create_request("elicit_user_action_1", &over_pending,)?.is_none()
    );

    let mut input_lines = BufReader::new(Cursor::new(Vec::<u8>::new())).lines();
    let mut wire_output = Vec::new();
    let mut request_sequence = 1;
    let output = crate::stdio::user_action_tool_output(
        &adapter(&fixture)?,
        over_response,
        true,
        true,
        &mut request_sequence,
        &mut input_lines,
        &mut wire_output,
    )?;
    assert!(
        wire_output.is_empty(),
        "an oversized elicitation must not send any partial JSON line"
    );
    let result = crate::stdio::tool_call_result_from_output(output);
    assert!(result["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry["text"].as_str())
        .any(|text| text.contains("complete elicitation request exceeds the 32768-byte wire budget; no partial form was sent")));
    Ok(())
}

#[test]
fn stdio_elicitation_evidence_observation_preserves_canonical_candidates_and_redacts_user_summary(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-evidence-observation")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, _) = create_task(&setup_adapter)?;
    let baseline_ref = "baseline_mcp_evidence_observation";
    let scope = setup_adapter.call_tool(
        UPDATE_SCOPE_TOOL_NAME,
        json!({
            "task_id": task_id,
            "goal_summary": null,
            "scope_update": null,
            "scope_boundary": "Observe stored evidence candidates through a User Channel.",
            "non_goals": [],
            "acceptance_criteria": [
                {
                    "acceptance_criterion_id": null,
                    "statement": "The first stored target remains available.",
                    "evidence_requirement": "required"
                },
                {
                    "acceptance_criterion_id": null,
                    "statement": "The selected stored target is supported by exact bytes.",
                    "evidence_requirement": "required"
                }
            ],
            "autonomy_boundary": null,
            "baseline_ref": baseline_ref,
            "change_unit": {
                "operation": "create_current",
                "scope_summary": "Exercise the evidence-observation User Channel.",
                "affected_paths": []
            },
            "related_scope_decision_refs": []
        }),
    )?;
    let change_unit_id = scope.response_value["state"]["active_change_unit_ref"]["record_id"]
        .as_str()
        .ok_or("scope response should expose the current Change Unit")?
        .to_owned();
    let criteria = scope.response_value["state"]["acceptance_criteria"]
        .as_array()
        .ok_or("scope response should expose acceptance criteria")?;
    assert_eq!(criteria.len(), 2);
    let target_candidates = criteria
        .iter()
        .map(|criterion| {
            json!({
                "target_kind": "acceptance_criterion",
                "acceptance_criterion_id": criterion["acceptance_criterion_id"]
            })
        })
        .collect::<Vec<_>>();
    let target_selectors = target_candidates
        .iter()
        .map(|target| {
            serde_json::from_value::<volicord_types::EvidenceTarget>(target.clone()).map(|target| {
                match target {
                    EvidenceTarget::AcceptanceCriterion {
                        acceptance_criterion_id,
                    } => format!("--criterion {acceptance_criterion_id}"),
                    EvidenceTarget::SupplementalClaim {
                        evidence_claim_id, ..
                    } => format!("--claim {evidence_claim_id}"),
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut staged_handles = Vec::new();
    for (display_name, bytes) in [
        (
            "observation-candidate-a.txt",
            "First exact evidence candidate bytes.",
        ),
        (
            "observation-candidate-b.txt",
            "Selected exact evidence candidate bytes.",
        ),
    ] {
        let staged = setup_adapter.call_tool(
            STAGE_ARTIFACT_TOOL_NAME,
            json!({
                "task_id": task_id,
                "display_name": display_name,
                "content_type": "text/plain",
                "redaction_state": "none",
                "safe_bytes_or_notice": bytes
            }),
        )?;
        staged_handles.push(staged.response_value["staged_artifact_handle"].clone());
    }
    let recorded = setup_adapter.call_tool(
        RECORD_RUN_TOOL_NAME,
        json!({
            "task_id": task_id,
            "change_unit_id": change_unit_id,
            "kind": "implementation",
            "baseline_ref": baseline_ref,
            "summary": "Register exact artifacts for a user-owned observation.",
            "observed_changes": {
                "changed_paths": [],
                "product_file_write_observed": false,
                "sensitive_categories": [],
                "baseline_ref": baseline_ref
            },
            "artifact_inputs": [
                {
                    "artifact_input_id": "artifact_input_observation_candidate_a",
                    "source_kind": "staged_artifact",
                    "staged_artifact_handle": staged_handles[0],
                    "existing_artifact_ref": null,
                    "relation_hint": "user_observation_candidate",
                    "evidence_target": target_candidates[0],
                    "expected_sha256": null,
                    "expected_size_bytes": null,
                    "redaction_state": "none"
                },
                {
                    "artifact_input_id": "artifact_input_observation_candidate_b",
                    "source_kind": "staged_artifact",
                    "staged_artifact_handle": staged_handles[1],
                    "existing_artifact_ref": null,
                    "relation_hint": "user_observation_candidate",
                    "evidence_target": target_candidates[1],
                    "expected_sha256": null,
                    "expected_size_bytes": null,
                    "redaction_state": "none"
                }
            ],
            "evidence_updates": [],
            "evidence_observations": [],
            "close_assessment": null
        }),
    )?;
    let registered_artifacts = recorded.response_value["registered_artifacts"]
        .as_array()
        .ok_or("record_run should expose registered artifacts")?
        .clone();
    assert_eq!(registered_artifacts.len(), 2);
    let artifact_candidate_ids = registered_artifacts
        .iter()
        .map(|artifact| {
            artifact["artifact_id"]
                .as_str()
                .ok_or("registered artifact should expose artifact_id")
                .map(str::to_owned)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut canonical_artifact_candidate_ids = artifact_candidate_ids.clone();
    canonical_artifact_candidate_ids.sort();
    let selected_target = target_candidates[1].clone();
    let selected_target_selector = target_selectors[1].clone();
    let selected_artifact_id = artifact_candidate_ids[1].clone();
    let private_summary = "private-user-observation-summary-must-not-enter-agent-output";
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            REQUEST_USER_ACTION_TOOL_NAME,
            evidence_observation_action_args(
                &task_id,
                &change_unit_id,
                target_candidates.clone(),
                artifact_candidate_ids.clone(),
            ),
        ),
        elicitation_accept_observation(
            &selected_target_selector,
            std::slice::from_ref(&selected_artifact_id),
            "supported",
            private_summary,
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 3);
    assert_eq!(values[1]["method"], ELICITATION_CREATE_METHOD);
    let requested_schema = &values[1]["params"]["requestedSchema"];
    assert_eq!(requested_schema["additionalProperties"], false);
    assert_eq!(
        requested_schema["properties"]["selected_target"]["enum"],
        json!(target_selectors)
    );
    assert_eq!(
        requested_schema["properties"]["selected_artifact_ids"]["items"]["enum"],
        json!(canonical_artifact_candidate_ids)
    );
    assert_eq!(
        requested_schema["properties"]["relevance_status"]["enum"],
        json!(["supported", "contradicted"])
    );
    let message = values[1]["params"]["message"]
        .as_str()
        .ok_or("elicitation message should be present")?;
    for target in &target_candidates {
        let target: EvidenceTarget = serde_json::from_value(target.clone())?;
        match target {
            EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id,
            } => assert!(message.contains(acceptance_criterion_id.as_str())),
            EvidenceTarget::SupplementalClaim {
                evidence_claim_id,
                statement,
            } => {
                assert!(message.contains(evidence_claim_id.as_str()));
                assert!(message.contains(&statement));
            }
        }
    }
    for artifact in &registered_artifacts {
        for field in [
            "artifact_id",
            "display_name",
            "sha256",
            "size_bytes",
            "storage_ref",
            "created_by_actor_source",
        ] {
            let value = &artifact[field];
            let displayed = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            assert!(
                message.contains(&displayed),
                "elicitation must display artifact {field} metadata"
            );
        }
    }
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(response["effect"]["effect_kind"], "core_committed");
    assert_eq!(response["status"], "resolved");
    assert_eq!(
        response["resolution_summary"]["resolution_type"],
        "evidence_observation"
    );
    assert_eq!(response["resolution_summary"]["target"], selected_target);
    assert_eq!(
        response["resolution_summary"]["relevance_status"],
        "supported"
    );
    assert_eq!(
        response["resolution_summary"]["artifact_refs"],
        json!([registered_artifacts[1].clone()])
    );
    assert_eq!(
        response["user_action_resolution_ref"]["produced_at_state_version"],
        response["current_projection_state_version"]
    );
    assert!(
        response["current_projection_state_version"]
            .as_u64()
            .zip(response["effect"]["state_version"].as_u64())
            .is_some_and(|(current, origin)| current > origin),
        "resolution projection must remain distinct from the originating request result"
    );
    assert_eq!(response["derived_refs"], json!([]));
    assert!(response["resolution_summary"].get("summary").is_none());
    assert!(!String::from_utf8_lossy(&output).contains(private_summary));

    let store = CoreProjectStore::open(
        fixture.runtime_home_path(),
        &ProjectId::new(fixture.project_id()),
    )?;
    let now = volicord_types::UtcTimestamp::parse(&user_action_channel_current_timestamp(
        fixture.runtime_home_path(),
        fixture.project_id(),
    )?)?;
    let records =
        store.user_action_records_for_task(&volicord_types::TaskId::new(&task_id), &now)?;
    assert_eq!(
        records.len(),
        1,
        "host capture must not duplicate the request"
    );
    let record = &records[0];
    assert_eq!(record.status, UserActionStatus::Resolved);
    let resolution = record
        .resolution
        .as_ref()
        .ok_or("stored evidence observation should be resolved")?;
    assert_eq!(resolution.resolved_by_actor_source, "local_user");
    assert_eq!(
        resolution.channel_kind,
        volicord_types::UserActionChannelKind::McpElicitation
    );
    assert_eq!(
        resolution.resolved_verification_basis,
        VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
    );
    assert_eq!(
        resolution.user_action_resolution_id,
        response["user_action_resolution_ref"]["record_id"]
    );
    let stored_body: volicord_types::UserActionResolutionBody =
        serde_json::from_str(&resolution.resolution_json)?;
    let volicord_types::UserActionResolutionBody::EvidenceObservation { observation } = stored_body
    else {
        return Err("stored resolution should be an evidence observation".into());
    };
    assert_eq!(serde_json::to_value(&observation.target)?, selected_target);
    assert_eq!(
        observation.relevance_status,
        EvidenceRelevanceStatus::Supported
    );
    assert_eq!(observation.summary, private_summary);
    assert_eq!(observation.output_artifact_refs.len(), 1);
    assert_eq!(
        observation.output_artifact_refs[0],
        serde_json::from_value(registered_artifacts[1].clone())?
    );
    let diagnostics_bytes = fs::read(diagnostics_db_path(fixture.runtime_home_path()))?;
    assert!(!String::from_utf8_lossy(&diagnostics_bytes).contains(private_summary));
    Ok(())
}

#[test]
fn stdio_elicitation_accept_workflow_preserves_resolution_derived_refs(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-workflow-derived-refs")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let mut arguments = default_product_action_args(&fixture, &task_id, state_version);
    arguments["detail"] = json!("workflow");
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(2, REQUEST_USER_ACTION_TOOL_NAME, arguments),
        elicitation_accept("keep", None),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    let response = values
        .iter()
        .find(|value| value["id"] == 2)
        .ok_or("tools/call response should be present")?;
    let structured = &response["result"]["structuredContent"];
    assert!(structured["next_actions"].is_array());
    assert!(structured["method_result"]["derived_refs"]
        .as_array()
        .is_some_and(|refs| refs
            .iter()
            .any(|record_ref| { record_ref["record_kind"] == "project_continuity_record" })));
    Ok(())
}

#[test]
fn stdio_full_elicitation_result_does_not_expose_private_user_note() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-full-private-note")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let mut arguments = default_product_action_args(&fixture, &task_id, state_version);
    arguments["detail"] = json!("full");
    let private_note = "private-user-note-must-not-enter-agent-connection-output";
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(2, REQUEST_USER_ACTION_TOOL_NAME, arguments),
        elicitation_accept("keep", Some(private_note)),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    let response = values
        .iter()
        .find(|value| value["id"] == 2)
        .ok_or("tools/call response should be present")?;
    assert_eq!(response["result"]["isError"], false);
    assert!(!serde_json::to_string(response)?.contains(private_note));
    let structured = &response["result"]["structuredContent"];
    assert_eq!(
        structured["operation_result_ref"]["source_method"],
        REQUEST_USER_ACTION_TOOL_NAME
    );
    assert!(
        structured["method_result"]["user_channel_resolution"]["resolution_summary"]
            .get("note")
            .is_none()
    );
    assert_eq!(
        structured["method_result"]["agent_workflow_result"]["user_action_request"]["status"],
        "pending"
    );
    assert!(structured["method_result"]["derived_refs"]
        .as_array()
        .is_some_and(|refs| refs
            .iter()
            .any(|record_ref| { record_ref["record_kind"] == "project_continuity_record" })));

    let pending_page = adapter(&fixture)?.call_tool(
        GET_OPERATION_RESULT_TOOL_NAME,
        json!({
            "operation_result_ref": structured["operation_result_ref"].clone()
        }),
    )?;
    let pending_exact = pending_page.response_value["chunk_utf8"]
        .as_str()
        .ok_or("retrieved pending response should include chunk_utf8")?;
    assert!(!pending_exact.contains(private_note));
    let pending_value: Value = serde_json::from_str(pending_exact)?;
    assert_eq!(pending_value["user_action_request"]["status"], "pending");
    Ok(())
}

#[test]
fn elicitation_write_failure_returns_nonretryable_post_effect_result() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-elicitation-write-post-effect")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            REQUEST_USER_ACTION_TOOL_NAME,
            default_product_action_args(&fixture, &task_id, state_version),
        ),
        elicitation_accept("keep", None),
    ])?);
    let mut writer = FailElicitationRequestWriter::default();

    run_stdio(adapter(&fixture)?, BufReader::new(input), &mut writer)?;

    assert!(writer.failed_elicitation);
    let values = stdio_responses(&writer.output)?;
    let response = values
        .iter()
        .find(|value| value["id"] == 2)
        .ok_or("tools/call response should remain available after elicitation write failure")?;
    let structured = &response["result"]["structuredContent"];
    assert_eq!(response["result"]["isError"], false);
    assert_eq!(structured["code"], "MCP_POST_EFFECT_ADAPTER_FAILED");
    assert_eq!(structured["retryable"], false);
    assert_eq!(structured["reached_core"], true);
    assert_eq!(structured["committed"], true);
    assert_eq!(structured["effect_kind"], "core_committed");
    assert_eq!(structured["effect_applied"], true);
    assert!(structured["effect_anchor"]
        .as_str()
        .is_some_and(|anchor| anchor.starts_with("authority_event:")));
    assert_eq!(
        structured["operation_result_ref"]["source_method"],
        REQUEST_USER_ACTION_TOOL_NAME
    );
    assert!(
        structured["method_result"]["user_action_request_ref"]["record_id"]
            .as_str()
            .is_some_and(|record_id| !record_id.trim().is_empty())
    );
    assert_eq!(
        structured["method_result"]["user_action_request"]["status"],
        "pending"
    );
    assert_eq!(
        structured["authority_receipt"]["task_ref"]["record_id"],
        task_id
    );
    assert_eq!(structured["authoritative_refresh_succeeded"], true);
    assert_eq!(structured["response_projection_omitted"], true);
    assert_eq!(structured["status_read_required"], true);
    assert_eq!(structured["completion_claim_withheld"], true);
    Ok(())
}

#[test]
fn stdio_diagnostics_count_validation_retry_without_storing_request_content(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-diagnostics-validation-retry")?;
    let before = fixture.counts()?;
    let adapter = adapter(&fixture)?;
    let sensitive_sentinel = "diagnostic-request-secret-and-file-/private/example.txt";
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            STATUS_TOOL_NAME,
            json!({"unexpected_private_value": sensitive_sentinel}),
        ),
        tools_call(3, STATUS_TOOL_NAME, json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[1]["result"]["isError"], true);
    assert_eq!(responses[2]["result"]["isError"], false);
    let diagnostics =
        read_diagnostic_session(fixture.runtime_home_path(), None)?.expect("diagnostics session");
    let status = diagnostics
        .tools
        .iter()
        .find(|tool| tool.tool_name == STATUS_TOOL_NAME)
        .expect("status metrics");
    assert_eq!(status.call_count, 2);
    assert_eq!(status.validation_failures, 1);
    assert_eq!(status.retries_after_validation_failure, 1);
    assert_eq!(status.core_reached_count, 1);
    assert_eq!(fixture.counts()?, before);
    let diagnostics_bytes = fs::read(diagnostics_db_path(fixture.runtime_home_path()))?;
    assert!(!String::from_utf8_lossy(&diagnostics_bytes).contains(sensitive_sentinel));
    Ok(())
}

#[test]
fn stdio_diagnostics_never_store_unknown_caller_tool_names() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-diagnostics-unknown-tool-private")?;
    let adapter = adapter(&fixture)?;
    let sensitive_tool_name = "token=abc123-private-tool-name";
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(2, sensitive_tool_name, json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses[1]["error"]["code"], -32602);
    let diagnostics =
        read_diagnostic_session(fixture.runtime_home_path(), None)?.expect("diagnostics session");
    assert!(diagnostics
        .tools
        .iter()
        .all(|tool| tool.tool_name != sensitive_tool_name));
    let diagnostics_bytes = fs::read(diagnostics_db_path(fixture.runtime_home_path()))?;
    assert!(!String::from_utf8_lossy(&diagnostics_bytes).contains(sensitive_tool_name));
    Ok(())
}

#[test]
fn corrupt_diagnostics_store_is_nonfatal_to_mcp_core_result() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-diagnostics-corrupt-nonfatal")?;
    fs::write(
        diagnostics_db_path(fixture.runtime_home_path()),
        b"not a sqlite diagnostics database",
    )?;
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
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[1]["result"]["isError"], false);
    let response = volicord_response_from_tool(&responses[1])?;
    assert_eq!(response["base"]["response_kind"], "result");
    assert_eq!(response["base"]["effect_kind"], "read_only");
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn stdio_elicitation_decline_resolves_stored_reject_choice() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-decline")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_action",
            authority_action_args(&fixture, &task_id, state_version),
        ),
        elicitation_action("decline"),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values[1]["method"], ELICITATION_CREATE_METHOD);
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(
        response["user_channel_resolution"]["resolution_summary"]["resolution_type"],
        "choice"
    );
    assert_eq!(
        response["user_channel_resolution"]["resolution_summary"]["selected_option_id"],
        "reject"
    );
    assert_eq!(
        response["user_channel_resolution"]["resolution_summary"]["resolution_outcome"],
        "rejected"
    );
    assert_eq!(
        stored_resolution_basis(&fixture, &task_id, &response)?,
        VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL
    );
    Ok(())
}

#[test]
fn stdio_elicitation_accept_can_resolve_deferred_choice() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-defer")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_action",
            authority_action_args(&fixture, &task_id, state_version),
        ),
        elicitation_accept("defer", Some("Not enough context yet.")),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(
        response["user_channel_resolution"]["resolution_summary"]["resolution_type"],
        "choice"
    );
    assert_eq!(
        response["user_channel_resolution"]["resolution_summary"]["selected_option_id"],
        "defer"
    );
    assert_eq!(
        response["user_channel_resolution"]["resolution_summary"]["resolution_outcome"],
        "deferred"
    );
    assert!(response["user_channel_resolution"]["resolution_summary"]
        .get("note")
        .is_none());
    let stored = stored_action_record(&fixture, &task_id, &response)?;
    let stored_resolution: Value = serde_json::from_str(
        &stored
            .resolution
            .as_ref()
            .ok_or("stored deferred action should include a resolution")?
            .resolution_json,
    )?;
    assert_eq!(stored_resolution["note"], "Not enough context yet.");
    Ok(())
}

#[test]
fn stdio_elicitation_cancel_leaves_user_action_pending() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-cancel")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_action",
            product_action_args(&fixture, &task_id, state_version),
        ),
        elicitation_action("cancel"),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(
        response["agent_workflow_result"]["user_action_request"]["status"],
        "pending"
    );
    assert!(response["user_channel_resolution"].is_null());
    assert!(values[2]["result"]["content"][1]["text"]
        .as_str()
        .expect("extra text")
        .contains("current status is pending"));
    assert_pending_user_action_resume_guidance(&values[2], &response, &fixture)?;
    let record = stored_action_record(&fixture, &task_id, &response)?;
    assert_eq!(record.status, UserActionStatus::Pending);
    assert!(record.resolution.is_none());
    Ok(())
}

#[test]
fn stdio_elicitation_invalid_response_leaves_user_action_pending() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-invalid")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            2,
            "volicord.request_user_action",
            product_action_args(&fixture, &task_id, state_version),
        ),
        elicitation_accept("not_an_option", None),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    let response = volicord_response_from_tool(&values[2])?;
    assert_eq!(
        response["agent_workflow_result"]["user_action_request"]["status"],
        "pending"
    );
    assert!(values[2]["result"]["content"][1]["text"]
        .as_str()
        .expect("extra text")
        .contains("not a stored choice"));
    assert_pending_user_action_resume_guidance(&values[2], &response, &fixture)?;
    let record = stored_action_record(&fixture, &task_id, &response)?;
    assert_eq!(record.status, UserActionStatus::Pending);
    Ok(())
}

#[test]
fn stdio_elicitation_rejects_unknown_mixed_and_null_choice_fields_without_effect(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-closed-choice-content")?;
    let (task_id, pending_response) = create_pending_product_action(&fixture)?;
    let before = user_action_side_effect_snapshot(&fixture)?;
    assert_eq!(before.1, 0);
    let cases = [
        (
            "unknown",
            json!({"selected_option_id":"keep","unexpected":"discard-me"}),
            "unsupported field `unexpected`",
        ),
        (
            "mixed",
            json!({"selected_option_id":"keep","summary":"cross-variant"}),
            "unsupported field `summary`",
        ),
        (
            "null-note",
            json!({"selected_option_id":"keep","note":null}),
            "content.note must be a string when supplied",
        ),
    ];

    for (case, content, expected_error) in cases {
        let (request, result) =
            invoke_pending_elicitation(&fixture, pending_response.clone(), content)?;
        assert_eq!(
            request["params"]["requestedSchema"]["additionalProperties"], false,
            "choice schema must remain closed for {case}"
        );
        assert!(
            result["content"][1]["text"]
                .as_str()
                .is_some_and(|text| text.contains(expected_error)),
            "missing rejection reason for {case}: {result}"
        );
        assert_eq!(
            user_action_side_effect_snapshot(&fixture)?,
            before,
            "invalid choice content must have no effect for {case}"
        );
    }

    let record = stored_action_record(&fixture, &task_id, &pending_response.response_value)?;
    assert_eq!(record.status, UserActionStatus::Pending);
    assert!(record.resolution.is_none());
    Ok(())
}

#[test]
fn stdio_elicitation_rejects_cross_variant_evidence_fields_without_effect(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-elicitation-closed-evidence-content")?;
    let pending = create_pending_evidence_observation_action(&fixture)?;
    let before = user_action_side_effect_snapshot(&fixture)?;
    let target = match &pending.target_candidates[0] {
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } => format!("--criterion {acceptance_criterion_id}"),
        EvidenceTarget::SupplementalClaim {
            evidence_claim_id, ..
        } => format!("--claim {evidence_claim_id}"),
    };
    let artifact_id = pending.registered_artifacts[0]
        .artifact_id
        .as_str()
        .to_owned();
    let (request, result) = invoke_pending_elicitation(
        &fixture,
        pending.response.clone(),
        json!({
            "selected_target":target,
            "selected_artifact_ids":[artifact_id],
            "relevance_status":"supported",
            "summary":"bounded observation",
            "selected_option_id":"keep"
        }),
    )?;

    assert_eq!(
        request["params"]["requestedSchema"]["additionalProperties"],
        false
    );
    assert!(result["content"][1]["text"]
        .as_str()
        .is_some_and(|text| text.contains("unsupported field `selected_option_id`")));
    assert_eq!(user_action_side_effect_snapshot(&fixture)?, before);
    let record =
        stored_action_record(&fixture, &pending.task_id, &pending.response.response_value)?;
    assert_eq!(record.status, UserActionStatus::Pending);
    assert!(record.resolution.is_none());
    Ok(())
}

#[test]
fn sensitive_complete_presentation_skips_elicitation_and_prompt_capture_without_effect(
) -> Result<(), Box<dyn Error>> {
    const TARGET_MARKER: &str = "SENSITIVE_TARGET_PRESENTATION_MARKER";
    const ARTIFACT_MARKER: &str = "SENSITIVE_ARTIFACT_PRESENTATION_MARKER";

    for (case, target_marker, artifact_marker, marker) in [
        ("target_only", Some(TARGET_MARKER), None, TARGET_MARKER),
        (
            "artifact_only",
            None,
            Some(ARTIFACT_MARKER),
            ARTIFACT_MARKER,
        ),
    ] {
        let fixture = CoreFixture::new(&format!("mcp-sensitive-presentation-{case}"))?;
        let pending = create_pending_evidence_observation_action_with_sensitive_markers(
            &fixture,
            target_marker,
            artifact_marker,
        )?;
        let before = user_action_side_effect_snapshot(&fixture)?;
        let mut response = pending.response;
        response.response_value["inbox_item"]["preferred_capture_path"] = json!({
            "kind": "prompt_capture",
            "label": "Host chat prompt capture",
            "available": true,
            "command": null,
            "url": null,
            "capture_basis": "user_prompt_submit_hook",
            "expires_at": null,
            "detail": "Present the exact request-bound form in chat."
        });
        let parsed = pending_user_action_from_response(&response)?
            .ok_or("sensitive presentation should remain a valid pending action")?;
        assert!(!crate::stdio::agent_facing_user_action_input_allowed(
            &parsed
        ));
        let expected_agent_workflow_result = serde_json::to_vec(&response.response_value)?;

        for client_supports_elicitation in [true, false] {
            let mut lines = BufReader::new(Cursor::new(Vec::<u8>::new())).lines();
            let mut writer = Vec::new();
            let mut request_sequence = 1;
            let output = crate::stdio::user_action_tool_output(
                &adapter(&fixture)?,
                response.clone(),
                true,
                client_supports_elicitation,
                &mut request_sequence,
                &mut lines,
                &mut writer,
            )?;

            assert!(writer.is_empty(), "{case}: no elicitation/create wire");
            assert_eq!(request_sequence, 1, "{case}: no elicitation id");
            let result = crate::stdio::tool_call_result_from_output(output);
            assert_eq!(result["structuredContent"]["current_status"], "pending");
            assert_eq!(
                serde_json::to_vec(&result["structuredContent"]["agent_workflow_result"])?,
                expected_agent_workflow_result,
                "{case}: immutable historical result bytes must not be rewritten"
            );
            assert!(result["structuredContent"]["agent_workflow_result"]
                .to_string()
                .contains(marker));
            let fallback_texts = result["content"]
                .as_array()
                .expect("tool content")
                .iter()
                .skip(1)
                .filter_map(|entry| entry["text"].as_str())
                .collect::<Vec<_>>();
            assert!(fallback_texts
                .iter()
                .any(|text| text.contains("requires a user-only channel")));
            assert!(fallback_texts
                .iter()
                .any(|text| text.contains("CLI inbox path")));
            assert!(fallback_texts.iter().all(|text| !text.contains(marker)));
            assert!(fallback_texts
                .iter()
                .all(|text| !text.contains("Exact request-bound command template")));
            assert_eq!(user_action_side_effect_snapshot(&fixture)?, before);
        }
    }
    Ok(())
}

#[test]
fn sensitive_presentation_uses_local_web_and_full_form_stays_on_user_only_page(
) -> Result<(), Box<dyn Error>> {
    const TARGET_MARKER: &str = "LOCAL_WEB_SENSITIVE_TARGET_MARKER";
    const ARTIFACT_MARKER: &str = "LOCAL_WEB_SENSITIVE_ARTIFACT_MARKER";

    let fixture = CoreFixture::new("mcp-sensitive-presentation-local-web")?;
    let pending = create_pending_evidence_observation_action_with_sensitive_markers(
        &fixture,
        Some(TARGET_MARKER),
        Some(ARTIFACT_MARKER),
    )?;
    let expected_agent_workflow_result = serde_json::to_vec(&pending.response.response_value)?;
    let before = fixture.counts()?;
    let mut lines = BufReader::new(Cursor::new(Vec::<u8>::new())).lines();
    let mut writer = Vec::new();
    let mut request_sequence = 1;
    let output = crate::stdio::user_action_tool_output(
        &adapter_with_local_web_consent(&fixture)?,
        pending.response.clone(),
        true,
        true,
        &mut request_sequence,
        &mut lines,
        &mut writer,
    )?;

    assert!(writer.is_empty());
    assert_eq!(request_sequence, 1);
    let result = crate::stdio::tool_call_result_from_output(output);
    assert_eq!(
        serde_json::to_vec(&result["structuredContent"]["agent_workflow_result"])?,
        expected_agent_workflow_result
    );
    let fallback_texts = result["content"]
        .as_array()
        .expect("tool content")
        .iter()
        .skip(1)
        .filter_map(|entry| entry["text"].as_str())
        .collect::<Vec<_>>();
    assert!(fallback_texts
        .iter()
        .all(|text| !text.contains(TARGET_MARKER) && !text.contains(ARTIFACT_MARKER)));
    assert!(fallback_texts
        .iter()
        .any(|text| text.contains("requires a user-only channel")));
    let fallback_state = fallback_texts
        .iter()
        .filter_map(|text| serde_json::from_str::<Value>(text).ok())
        .find(|value| value["volicord_fallback"]["kind"] == "local_web_consent")
        .ok_or("sensitive presentation should retain local-web fallback")?;
    let url = fallback_state["volicord_fallback"]["url"]
        .as_str()
        .ok_or("local-web fallback should include a URL")?;
    let target = url
        .strip_prefix(consent_base_url())
        .ok_or("fallback URL should use the local consent base URL")?;
    let mut server = consent_server(&fixture)?;
    let get = server.handle_request(consent_get_request(target));
    assert_eq!(get.status, 200);
    assert_local_web_consent_security_headers(&get);
    let body = http_body_text(&get)?;
    assert!(body.contains(TARGET_MARKER));
    assert!(body.contains(ARTIFACT_MARKER));
    assert!(body.contains("Supplemental claim"));
    for artifact in &pending.registered_artifacts {
        assert!(body.contains(artifact.artifact_id.as_str()));
        assert!(body.contains(&artifact.display_name));
    }
    assert_eq!(fixture.counts()?, before);
    let record =
        stored_action_record(&fixture, &pending.task_id, &pending.response.response_value)?;
    assert_eq!(record.status, UserActionStatus::Pending);
    assert!(record.resolution.is_none());
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
            "volicord.request_user_action",
            product_action_args(&fixture, &task_id, state_version),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 2);
    assert_eq!(
        values[1]["result"]["structuredContent"]["operation_result_ref"]["source_method"],
        REQUEST_USER_ACTION_TOOL_NAME
    );
    let response = volicord_response_from_tool(&values[1])?;
    let workflow = &response["agent_workflow_result"];
    assert_eq!(workflow["user_action_request"]["status"], "pending");
    assert_eq!(
        workflow["inbox_item"]["preferred_capture_path"]["kind"],
        "cli"
    );
    let availability = &workflow["inbox_item"]["answer_path_availability"];
    assert_eq!(
        channel_path(availability, "mcp_elicitation")["available"],
        false
    );
    assert_eq!(
        channel_path(availability, "local_web_consent")["available"],
        false
    );
    assert_eq!(channel_path(availability, "cli")["available"], true);
    assert!(workflow["inbox_item"]["preferred_capture_path"]["command"]
        .as_str()
        .expect("CLI fallback command should be present")
        .contains("volicord inbox resolve"));
    let fallback = values[1]["result"]["content"][1]["text"]
        .as_str()
        .expect("fallback text");
    assert!(fallback.contains("Host prompt input is unavailable"));
    assert!(fallback.contains("CLI inbox path"));
    assert!(fallback.contains("volicord inbox resolve"));
    assert!(fallback.contains("nested `request.operation=resume`"));
    let state: Value = serde_json::from_str(
        values[1]["result"]["content"][2]["text"]
            .as_str()
            .expect("structured fallback text"),
    )?;
    let state = &state["volicord_fallback"];
    assert_eq!(state["kind"], "cli_recovery");
    assert_eq!(state["resume"]["tool_name"], REQUEST_USER_ACTION_TOOL_NAME);
    assert_eq!(state["resume"]["creates_new_request"], false);
    assert_eq!(
        state["resume"]["arguments"]["project_selector"],
        fixture.project_id()
    );
    assert_eq!(
        state["resume"]["arguments"]["request"]["operation"],
        "resume"
    );
    assert_eq!(
        state["resume"]["arguments"]["request"]["user_action_request_id"],
        workflow["user_action_request"]["user_action_request_id"]
    );
    let diagnostics = read_diagnostic_session(fixture.runtime_home_path(), None)?
        .expect("CLI fallback should create bounded diagnostics");
    assert_eq!(diagnostics.fallback_counts["cli_inbox"], 1);
    Ok(())
}

#[test]
fn stdio_does_not_reconstruct_prompt_capture_when_core_prefers_cli() -> Result<(), Box<dyn Error>> {
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
            "volicord.request_user_action",
            product_action_args(&fixture, &task_id, state_version),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 2);
    assert_eq!(
        values[1]["result"]["structuredContent"]["operation_result_ref"]["source_method"],
        REQUEST_USER_ACTION_TOOL_NAME
    );
    let response = volicord_response_from_tool(&values[1])?;
    let workflow = &response["agent_workflow_result"];
    assert_eq!(workflow["user_action_request"]["status"], "pending");
    assert_eq!(
        workflow["inbox_item"]["preferred_capture_path"]["kind"],
        "cli"
    );
    let availability = &workflow["inbox_item"]["answer_path_availability"];
    assert_eq!(
        channel_path(availability, "mcp_elicitation")["available"],
        false
    );
    assert_eq!(
        channel_path(availability, "prompt_capture")["available"],
        false
    );
    assert_eq!(channel_path(availability, "cli")["available"], true);
    let fallback_texts = values[1]["result"]["content"]
        .as_array()
        .expect("tool content")
        .iter()
        .filter_map(|content| content["text"].as_str())
        .collect::<Vec<_>>();
    assert!(fallback_texts
        .iter()
        .any(|text| text.contains("Host prompt input is unavailable")));
    assert!(fallback_texts
        .iter()
        .any(|text| text.contains("CLI inbox path")));
    let diagnostics = read_diagnostic_session(fixture.runtime_home_path(), None)?
        .expect("CLI fallback should create bounded diagnostics");
    assert_eq!(diagnostics.fallback_counts["cli_inbox"], 1);
    Ok(())
}

#[test]
fn stdio_without_elicitation_uses_local_web_consent_when_prompt_capture_unavailable(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-fallback")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let adapter = adapter_with_local_web_consent(&fixture)?;
    let now =
        user_action_channel_current_timestamp(fixture.runtime_home_path(), fixture.project_id())?;
    let now = volicord_types::UtcTimestamp::parse(&now)?;
    let request_expires_at = volicord_types::UtcTimestamp::from_datetime(
        *now.as_datetime() + std::time::Duration::from_secs(120),
    );
    let mut arguments = product_action_args(&fixture, &task_id, state_version);
    arguments["request"]["expires_at"] = json!(request_expires_at);
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(2, "volicord.request_user_action", arguments),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 2);
    assert_eq!(
        values[1]["result"]["structuredContent"]["operation_result_ref"]["source_method"],
        REQUEST_USER_ACTION_TOOL_NAME
    );
    let response = volicord_response_from_tool(&values[1])?;
    let workflow = &response["agent_workflow_result"];
    assert_eq!(workflow["user_action_request"]["status"], "pending");
    assert_eq!(
        workflow["inbox_item"]["preferred_capture_path"]["kind"],
        "local_web_consent"
    );
    let availability = &workflow["inbox_item"]["answer_path_availability"];
    assert_eq!(
        channel_path(availability, "mcp_elicitation")["available"],
        false
    );
    assert_eq!(
        channel_path(availability, "local_web_consent")["available"],
        true
    );
    assert_eq!(channel_path(availability, "cli")["available"], true);
    let fallback = values[1]["result"]["content"][1]["text"]
        .as_str()
        .expect("fallback text");
    assert!(fallback.contains("local Volicord consent link"));
    assert!(fallback.contains("nested `request.operation=resume`"));

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
    assert!(state.get("ttl_seconds").is_none());
    assert_eq!(
        state["expires_at"],
        workflow["user_action_request"]["expires_at"]
    );
    assert_eq!(state["resume"]["tool_name"], REQUEST_USER_ACTION_TOOL_NAME);
    assert_eq!(state["resume"]["creates_new_request"], false);
    assert_eq!(
        state["resume"]["arguments"]["project_selector"],
        fixture.project_id()
    );
    assert_eq!(
        state["resume"]["arguments"]["request"]["operation"],
        "resume"
    );
    assert_eq!(
        state["resume"]["arguments"]["request"]["user_action_request_id"],
        workflow["user_action_request"]["user_action_request_id"]
    );
    let url = state["url"].as_str().expect("fallback URL");
    assert!(url.starts_with(&format!(
        "{}{}?project=",
        consent_base_url(),
        LOCAL_WEB_CONSENT_PATH
    )));
    let token = token_from_consent_url(url)?;
    let now =
        user_action_channel_current_timestamp(fixture.runtime_home_path(), fixture.project_id())?;
    let validation = validate_user_action_channel_token(
        fixture.runtime_home_path(),
        UserActionChannelTokenCheck {
            token,
            expected_project_id: fixture.project_id().to_owned(),
            expected_connection_internal_id: fixture.connection_id().to_owned(),
            now,
        },
    )?;
    let UserActionChannelTokenValidation::Valid(record) = validation else {
        return Err("local-web token should remain valid".into());
    };
    let token_expires_at = volicord_types::UtcTimestamp::parse(&record.expires_at)?;
    assert!(token_expires_at <= request_expires_at);
    let diagnostics = read_diagnostic_session(fixture.runtime_home_path(), None)?
        .expect("local web fallback should create bounded diagnostics");
    assert_eq!(diagnostics.fallback_counts["local_web_consent"], 1);
    Ok(())
}

#[test]
fn stdio_pending_resume_with_local_web_is_read_only_exact_replay() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-pending-resume-local-web-read-only")?;
    let (_task_id, pending_response) = create_pending_product_action(&fixture)?;
    assert!(!pending_response.replayed);
    let exact_origin_bytes = serde_json::to_vec(&pending_response.response_value)?;
    let origin_operation_result_ref = serde_json::to_value(
        pending_response
            .operation_result_ref
            .as_ref()
            .ok_or("created response should have an operation-result ref")?,
    )?;
    let user_action_request_id = pending_response.response_value["user_action_request_ref"]
        ["record_id"]
        .as_str()
        .ok_or("created response should identify the user-action request")?
        .to_owned();
    let storage_snapshot = || -> Result<(i64, String), Box<dyn Error>> {
        Ok(fixture.conn()?.query_row(
            "SELECT (SELECT COUNT(*) FROM user_action_channel_tokens), updated_at
               FROM project_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?)
    };
    let before_counts = fixture.counts()?;
    let before_storage = storage_snapshot()?;
    assert_eq!(before_storage.0, 0);

    let resume_input = Cursor::new(json_lines(&[
        initialize_request(3, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            4,
            REQUEST_USER_ACTION_TOOL_NAME,
            resume_user_action_args(&fixture, &user_action_request_id),
        ),
    ])?);
    let mut resume_output = Vec::new();
    run_stdio(
        adapter_with_local_web_consent(&fixture)?,
        BufReader::new(resume_input),
        &mut resume_output,
    )?;

    let resume_values = stdio_responses(&resume_output)?;
    assert_eq!(resume_values.len(), 2);
    assert!(resume_values
        .iter()
        .all(|value| value["method"] != "elicitation/create"));
    assert_eq!(
        resume_values[1]["result"]["content"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    let resumed = volicord_response_from_tool(&resume_values[1])?;
    assert_eq!(
        serde_json::to_vec(&resumed["agent_workflow_result"])?,
        exact_origin_bytes
    );
    assert_eq!(resumed["agent_workflow_result_replayed"], true);
    assert_eq!(resumed["current_status"], "pending");
    assert!(resumed["user_channel_resolution_ref"].is_null());
    assert!(resumed["user_channel_resolution"].is_null());
    assert_eq!(
        resume_values[1]["result"]["structuredContent"]["operation_result_ref"],
        origin_operation_result_ref
    );
    assert_eq!(fixture.counts()?, before_counts);
    assert_eq!(storage_snapshot()?, before_storage);
    Ok(())
}

#[test]
fn stdio_rejects_tampered_noncanonical_and_oversized_pending_forms_before_capture(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-pending-form-fail-closed")?;
    let (_task_id, pending_response) = create_pending_product_action(&fixture)?;
    let before = user_action_side_effect_snapshot(&fixture)?;
    assert_eq!(before.1, 0);

    let mut tampered = pending_response.clone();
    tampered.response_value["inbox_item"]["form"]["choices"][0]["label"] = json!("Tampered label");
    let mut noncanonical = pending_response.clone();
    noncanonical.response_value["inbox_item"]["form"]["note_max_chars"] = json!(17);
    let mut oversized = pending_response.clone();
    oversized.response_value["inbox_item"]["form"]["choices"][0]["description"] =
        json!("x".repeat(volicord_types::USER_ACTION_FORM_MAX_BYTES + 1));

    for (case, response) in [
        ("tampered", tampered),
        ("noncanonical", noncanonical),
        ("oversized", oversized),
    ] {
        assert!(
            pending_user_action_from_response(&response).is_err(),
            "{case} form must fail closed before entering a User Channel"
        );
        let mut lines = BufReader::new(Cursor::new(Vec::<u8>::new())).lines();
        let mut writer = Vec::new();
        let mut request_sequence = 1;
        let error = crate::stdio::user_action_tool_output(
            &adapter_with_local_web_consent(&fixture)?,
            response,
            true,
            true,
            &mut request_sequence,
            &mut lines,
            &mut writer,
        )
        .expect_err("invalid pending form must report post-effect adapter failure");
        assert!(matches!(
            error,
            McpAdapterError::Protocol(_) | McpAdapterError::Json(_)
        ));
        assert!(writer.is_empty(), "{case} form must not elicit");
        assert_eq!(request_sequence, 1, "{case} form must not allocate an id");
        assert_eq!(
            user_action_side_effect_snapshot(&fixture)?,
            before,
            "{case} form must not create a token, resolution, or project effect"
        );
    }
    Ok(())
}

#[test]
fn stdio_resume_replays_exact_origin_and_rereads_cross_channel_resolution(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-user-action-cross-channel-resume")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;

    let create_input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            REQUEST_USER_ACTION_TOOL_NAME,
            product_action_args(&fixture, &task_id, state_version),
        ),
    ])?);
    let mut create_output = Vec::new();
    run_stdio(
        adapter(&fixture)?,
        BufReader::new(create_input),
        &mut create_output,
    )?;

    let create_values = stdio_responses(&create_output)?;
    assert_eq!(create_values.len(), 2);
    let created = volicord_response_from_tool(&create_values[1])?;
    assert_eq!(created["current_status"], "pending");
    assert_eq!(created["agent_workflow_result_replayed"], false);
    let exact_origin = created["agent_workflow_result"].clone();
    let exact_origin_bytes = serde_json::to_vec(&exact_origin)?;
    let origin_operation_result_ref =
        create_values[1]["result"]["structuredContent"]["operation_result_ref"].clone();
    let user_action_request_id = exact_origin["user_action_request_ref"]["record_id"]
        .as_str()
        .ok_or("created response should identify the user-action request")?
        .to_owned();
    let after_create = fixture.counts()?;

    let core = CoreService::new(fixture.runtime_home_path());
    let resolved = core.resolve_user_action(
        fixture.resolve_user_action_request(ResolveUserActionFixture {
            request_id: "req_mcp_cross_channel_resolution",
            task_id: &task_id,
            user_action_request_id: &user_action_request_id,
            channel_submission_id: "submission_mcp_cross_channel_resolution",
            resolution: UserActionResolutionInput::Choice {
                selected_option_id: volicord_types::UserActionOptionId::new("keep"),
                note: Some("This private user note must not enter the MCP projection.".to_owned())
                    .into(),
            },
        }),
        InvocationContext::new(
            ProjectId::new(fixture.project_id()),
            ActorSource::LocalUser,
            OperationCategory::UserOnly,
            VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL,
        ),
    )?;
    assert_eq!(resolved.response_value["base"]["response_kind"], "result");
    let historical_derived_refs = resolved.response_value["derived_refs"].clone();
    assert!(historical_derived_refs
        .as_array()
        .is_some_and(|refs| !refs.is_empty()));
    let historical_resolution_ref = resolved.response_value["user_action_resolution_ref"].clone();
    let resolution_state_version = resolved.response_value["base"]["state_version"]
        .as_u64()
        .ok_or("resolution should report its committed state version")?;
    let after_resolution = fixture.counts()?;
    assert_eq!(
        after_resolution.user_action_requests,
        after_create.user_action_requests
    );
    assert_eq!(
        after_resolution.user_action_resolutions,
        after_create.user_action_resolutions + 1
    );

    let unrelated = core.request_user_action(
        fixture.user_action_request(UserActionFixture {
            request_id: "req_mcp_cross_channel_unrelated_action",
            idempotency_key: "idem_mcp_cross_channel_unrelated_action",
            dry_run: false,
            expected_state_version: Some(resolution_state_version),
            task_id: &task_id,
            change_unit_id: None,
            judgment_kind: volicord_types::JudgmentKind::TechnicalDecision,
        }),
        InvocationContext::new(
            ProjectId::new(fixture.project_id()),
            ActorSource::agent_connection(fixture.connection_id()),
            OperationCategory::AgentWorkflow,
            VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
        ),
    )?;
    assert_eq!(unrelated.response_value["base"]["response_kind"], "result");
    let before_resume = fixture.counts()?;
    assert_eq!(before_resume.state_version, resolution_state_version + 1);
    assert_eq!(
        before_resume.user_action_requests,
        after_resolution.user_action_requests + 1
    );

    let wrong_connection_id = "conn_mcp_cross_channel_wrong";
    let wrong_adapter = adapter_for_additional_connection(&fixture, wrong_connection_id)?;
    let wrong_error = wrong_adapter
        .call_tool(
            REQUEST_USER_ACTION_TOOL_NAME,
            resume_user_action_args(&fixture, &user_action_request_id),
        )
        .expect_err("another Agent Connection must not resume the originating result");
    assert!(matches!(wrong_error, McpAdapterError::ToolExecution { .. }));
    assert_eq!(fixture.counts()?, before_resume);

    let resume_input = Cursor::new(json_lines(&[
        initialize_request(3, json!({ "elicitation": {} })),
        initialized_notification(),
        tools_call(
            4,
            REQUEST_USER_ACTION_TOOL_NAME,
            resume_user_action_args(&fixture, &user_action_request_id),
        ),
    ])?);
    let mut resume_output = Vec::new();
    run_stdio(
        adapter(&fixture)?,
        BufReader::new(resume_input),
        &mut resume_output,
    )?;

    let resume_values = stdio_responses(&resume_output)?;
    assert_eq!(resume_values.len(), 2);
    assert!(resume_values
        .iter()
        .all(|value| value["method"] != "elicitation/create"));
    let resumed = volicord_response_from_tool(&resume_values[1])?;
    assert_eq!(
        serde_json::to_vec(&resumed["agent_workflow_result"])?,
        exact_origin_bytes
    );
    assert_eq!(resumed["agent_workflow_result_replayed"], true);
    assert_eq!(resumed["current_status"], "resolved");
    assert_eq!(
        resumed["current_projection_state_version"],
        before_resume.state_version
    );
    assert!(resumed["current_projection_observed_at"].is_string());
    assert_eq!(
        resumed["user_channel_resolution_ref"],
        historical_resolution_ref
    );
    assert_eq!(resumed["derived_refs"], historical_derived_refs);
    assert_eq!(
        resumed["user_channel_resolution"]["resolution_summary"]["selected_option_id"],
        "keep"
    );
    assert!(resumed["user_channel_resolution"]
        .to_string()
        .find("private user note")
        .is_none());
    assert_eq!(
        resume_values[1]["result"]["structuredContent"]["operation_result_ref"],
        origin_operation_result_ref
    );
    assert_eq!(fixture.counts()?, before_resume);
    Ok(())
}

#[test]
fn local_web_consent_get_renders_pending_user_action_page() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-get")?;
    let (_task_id, response) = create_pending_product_action(&fixture)?;
    let token = "1111111111111111111111111111111111111111111111111111111111111111";
    create_consent_token_for_response(&fixture, &response, token)?;
    let mut server = consent_server(&fixture)?;

    let response = server.handle_request(consent_get_request(&consent_target(
        fixture.project_id(),
        token,
    )));

    assert_eq!(response.status, 200);
    assert_local_web_consent_security_headers(&response);
    let body = http_body_text(&response)?;
    assert!(body.contains("Resolve user action"));
    assert!(body.contains("This page records one user-owned action"));
    assert!(body.contains("The agent cannot resolve it on your behalf."));
    assert!(body.contains("does not prove correctness"));
    assert!(body.contains("test sufficiency"));
    assert!(body.contains("deployment success"));
    assert!(body.contains("review completion"));
    assert!(body.contains("Choose the focused User Channel test outcome."));
    assert!(body.contains(fixture.project_id()));
    assert!(body.contains(&fixture.product_repo_path().display().to_string()));
    assert!(body.contains(fixture.connection_id()));
    assert!(body.contains("User-action request id"));
    assert!(body.contains("Token expires"));
    assert!(body.contains("Fallback CLI command"));
    assert!(body.contains("volicord inbox resolve"));
    assert!(body.contains("Available choices"));
    assert!(body.contains("Choice ID: <code>keep</code>"));
    assert!(body.contains("Consequence: Only this focused judgment is resolved."));
    assert!(!body.contains("Runtime Home"));
    Ok(())
}

#[test]
fn local_web_consent_get_and_post_reject_shortened_token_window_without_effects(
) -> Result<(), Box<dyn Error>> {
    type TokenSnapshot = (String, String, Option<String>, Option<String>);

    let fixture = CoreFixture::new("mcp-local-web-shortened-token-window")?;
    let (_task_id, pending_response) = create_pending_product_action(&fixture)?;
    let token = "1212121212121212121212121212121212121212121212121212121212121212";
    create_consent_token_for_response(&fixture, &pending_response, token)?;
    let token_hash = volicord_store::user_action_channel::user_action_channel_token_hash(token)?;
    fixture.conn()?.execute(
        "UPDATE user_action_channel_tokens
            SET expires_at = strftime('%Y-%m-%dT%H:%M:%SZ', created_at, '+599 seconds')
          WHERE project_id = ?1 AND token_hash = ?2",
        (fixture.project_id(), token_hash.as_str()),
    )?;
    let token_snapshot = || -> Result<TokenSnapshot, Box<dyn Error>> {
        Ok(fixture.conn()?.query_row(
            "SELECT status, expires_at, consumed_at, completed_at
                   FROM user_action_channel_tokens
                  WHERE project_id = ?1 AND token_hash = ?2",
            (fixture.project_id(), token_hash.as_str()),
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?)
    };
    let before_effects = user_action_side_effect_snapshot(&fixture)?;
    let before_token = token_snapshot()?;
    let mut server = consent_server(&fixture)?;

    let get = server.handle_request(consent_get_request(&consent_target(
        fixture.project_id(),
        token,
    )));
    assert_eq!(get.status, 500);
    assert_local_web_consent_security_headers(&get);
    assert!(http_body_text(&get)?.contains("STORE_UNAVAILABLE"));
    assert_eq!(user_action_side_effect_snapshot(&fixture)?, before_effects);
    assert_eq!(token_snapshot()?, before_token);

    let post = server.handle_request(consent_post_request(
        Some(consent_base_url()),
        &format!(
            "project={}&token={}&selected_option_id=keep",
            percent_encode_query(fixture.project_id()),
            token
        ),
    ));
    assert_eq!(post.status, 500);
    assert_local_web_consent_security_headers(&post);
    assert!(http_body_text(&post)?.contains("STORE_UNAVAILABLE"));
    assert_eq!(user_action_side_effect_snapshot(&fixture)?, before_effects);
    assert_eq!(token_snapshot()?, before_token);
    Ok(())
}

#[test]
fn local_web_consent_post_resolves_user_owned_action() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-post")?;
    let (task_id, pending_response) = create_pending_product_action(&fixture)?;
    let token = "2222222222222222222222222222222222222222222222222222222222222222";
    create_consent_token_for_response(&fixture, &pending_response, token)?;
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
    assert!(body.contains("Resolution recorded"));
    assert!(body.contains("resolved user action"));
    assert!(body.contains("does not prove correctness"));
    let pending_value = pending_response.response_value;
    let record = stored_action_record(&fixture, &task_id, &pending_value)?;
    assert_eq!(record.status, UserActionStatus::Resolved);
    let resolution = record.resolution.expect("resolution should be stored");
    assert_eq!(resolution.resolved_by_actor_source, "local_user");
    assert_eq!(
        resolution.resolved_verification_basis,
        VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB
    );
    Ok(())
}

#[test]
fn local_web_consent_rejects_unknown_mixed_and_malformed_fields_without_effect(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-closed-choice-fields")?;
    let (task_id, pending_response) = create_pending_product_action(&fixture)?;
    let token = "2323232323232323232323232323232323232323232323232323232323232323";
    create_consent_token_for_response(&fixture, &pending_response, token)?;
    let before = user_action_side_effect_snapshot(&fixture)?;
    assert_eq!(before.1, 1);
    let mut server = consent_server(&fixture)?;
    let canonical_prefix = format!(
        "project={}&token={}&selected_option_id=keep",
        percent_encode_query(fixture.project_id()),
        token
    );
    let cases = [
        (
            "unknown",
            format!("{canonical_prefix}&unexpected=discard-me"),
            "INVALID_SELECTION",
        ),
        (
            "mixed",
            format!("{canonical_prefix}&summary=cross-variant"),
            "INVALID_SELECTION",
        ),
        (
            "malformed-extra-name",
            format!("{canonical_prefix}&%ZZ=discard-me"),
            "FORM_ENCODING_INVALID",
        ),
        (
            "malformed-extra-value",
            format!("{canonical_prefix}&unexpected=%ZZ"),
            "FORM_ENCODING_INVALID",
        ),
    ];

    for (case, body, expected_code) in cases {
        let response = server.handle_request(consent_post_request(Some(consent_base_url()), &body));
        assert_eq!(response.status, 400, "unexpected status for {case}");
        assert_local_web_consent_security_headers(&response);
        assert!(
            http_body_text(&response)?.contains(expected_code),
            "missing error code for {case}"
        );
        assert_eq!(
            user_action_side_effect_snapshot(&fixture)?,
            before,
            "invalid local-web fields must not consume a token, resolve, or change project state for {case}"
        );
    }

    let now =
        user_action_channel_current_timestamp(fixture.runtime_home_path(), fixture.project_id())?;
    assert!(matches!(
        validate_user_action_channel_token(
            fixture.runtime_home_path(),
            UserActionChannelTokenCheck {
                token: token.to_owned(),
                expected_project_id: fixture.project_id().to_owned(),
                expected_connection_internal_id: fixture.connection_id().to_owned(),
                now,
            },
        )?,
        UserActionChannelTokenValidation::Valid(_)
    ));
    let record = stored_action_record(&fixture, &task_id, &pending_response.response_value)?;
    assert_eq!(record.status, UserActionStatus::Pending);
    assert!(record.resolution.is_none());
    Ok(())
}

#[test]
fn local_web_consent_evidence_observation_uses_canonical_form_and_consumes_token_atomically(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-evidence-observation")?;
    let pending = create_pending_evidence_observation_action(&fixture)?;
    let token = "abababababababababababababababababababababababababababababababab";
    create_consent_token_for_response(&fixture, &pending.response, token)?;
    let pending_action = pending_user_action_from_response(&pending.response)?
        .ok_or("response should include a pending evidence-observation action")?;
    let expected_form_digest = canonical_json_bare_sha256(&pending_action.inbox_item.form)?;
    let now =
        user_action_channel_current_timestamp(fixture.runtime_home_path(), fixture.project_id())?;
    let validation = validate_user_action_channel_token(
        fixture.runtime_home_path(),
        UserActionChannelTokenCheck {
            token: token.to_owned(),
            expected_project_id: fixture.project_id().to_owned(),
            expected_connection_internal_id: fixture.connection_id().to_owned(),
            now,
        },
    )?;
    let UserActionChannelTokenValidation::Valid(token_record) = validation else {
        return Err("local-web token should be valid before capture".into());
    };
    assert_eq!(
        serde_json::from_str::<Value>(&token_record.created_metadata_json)?["form_digest"],
        expected_form_digest
    );
    let before = fixture.counts()?;
    let mut server = consent_server(&fixture)?;

    let get = server.handle_request(consent_get_request(&consent_target(
        fixture.project_id(),
        token,
    )));
    assert_eq!(get.status, 200);
    assert_local_web_consent_security_headers(&get);
    let get_body = http_body_text(&get)?;
    for expected in [
        "Evidence target",
        "Observed artifacts",
        "Relevance",
        "supported",
        "contradicted",
        "name=\"summary\"",
    ] {
        assert!(
            get_body.contains(expected),
            "missing canonical field {expected}"
        );
    }
    for target in &pending.target_candidates {
        let volicord_types::EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } = target
        else {
            return Err("fixture should use acceptance-criterion targets".into());
        };
        assert!(get_body.contains(acceptance_criterion_id.as_str()));
    }
    for artifact in &pending.registered_artifacts {
        assert!(get_body.contains(artifact.artifact_id.as_str()));
        assert!(get_body.contains(&artifact.display_name));
        let metadata = serde_json::to_value(artifact)?;
        for field in ["sha256", "storage_ref", "created_by_actor_source"] {
            let value = metadata[field]
                .as_str()
                .ok_or("registered artifact should expose exact presentation metadata")?;
            assert!(
                get_body.contains(value),
                "local-web form must display artifact {field} metadata"
            );
        }
        assert!(get_body.contains(&metadata["size_bytes"].to_string()));
    }

    let selected_target = pending.target_candidates[1].clone();
    let selected_target_selector = match &selected_target {
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } => format!("--criterion {acceptance_criterion_id}"),
        EvidenceTarget::SupplementalClaim {
            evidence_claim_id, ..
        } => format!("--claim {evidence_claim_id}"),
    };
    let selected_artifact = pending.registered_artifacts[1].clone();
    let private_summary =
        "private-local-web-evidence-summary-must-not-enter-browser-or-agent-output";
    let post_body = format!(
        "project={}&token={}&selected_target={}&selected_artifact_ids={}&relevance_status=supported&summary={}",
        percent_encode_query(fixture.project_id()),
        token,
        percent_encode_query(&selected_target_selector),
        percent_encode_query(selected_artifact.artifact_id.as_str()),
        percent_encode_query(private_summary),
    );
    let before_invalid = user_action_side_effect_snapshot(&fixture)?;
    let mixed = server.handle_request(consent_post_request(
        Some(consent_base_url()),
        &format!("{post_body}&selected_option_id=keep"),
    ));
    assert_eq!(mixed.status, 400);
    assert_local_web_consent_security_headers(&mixed);
    assert!(http_body_text(&mixed)?.contains("INVALID_SELECTION"));
    assert_eq!(user_action_side_effect_snapshot(&fixture)?, before_invalid);
    let now =
        user_action_channel_current_timestamp(fixture.runtime_home_path(), fixture.project_id())?;
    assert!(matches!(
        validate_user_action_channel_token(
            fixture.runtime_home_path(),
            UserActionChannelTokenCheck {
                token: token.to_owned(),
                expected_project_id: fixture.project_id().to_owned(),
                expected_connection_internal_id: fixture.connection_id().to_owned(),
                now,
            },
        )?,
        UserActionChannelTokenValidation::Valid(_)
    ));
    let post = server.handle_request(consent_post_request(Some(consent_base_url()), &post_body));
    assert_eq!(post.status, 200);
    assert_local_web_consent_security_headers(&post);
    let post_response_body = http_body_text(&post)?;
    assert!(post_response_body.contains("Resolution recorded"));
    assert!(!post_response_body.contains(private_summary));

    let record =
        stored_action_record(&fixture, &pending.task_id, &pending.response.response_value)?;
    assert_eq!(record.status, UserActionStatus::Resolved);
    let resolution = record
        .resolution
        .as_ref()
        .ok_or("local-web observation resolution should be stored")?;
    assert_eq!(resolution.resolved_by_actor_source, "local_user");
    assert_eq!(
        resolution.channel_kind,
        volicord_types::UserActionChannelKind::LocalWebConsent
    );
    assert_eq!(
        resolution.resolved_verification_basis,
        VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB
    );
    let stored_body: volicord_types::UserActionResolutionBody =
        serde_json::from_str(&resolution.resolution_json)?;
    let volicord_types::UserActionResolutionBody::EvidenceObservation { observation } = stored_body
    else {
        return Err("stored local-web resolution should be an evidence observation".into());
    };
    assert_eq!(observation.target, selected_target);
    assert_eq!(
        observation.relevance_status,
        EvidenceRelevanceStatus::Supported
    );
    assert_eq!(observation.summary, private_summary);
    assert_eq!(
        observation.output_artifact_refs,
        vec![selected_artifact.clone()],
        "the stored resolution must preserve the exact historical artifact ref"
    );

    let request_id = record.request.user_action_request_id.clone();
    let projection = CoreService::new(fixture.runtime_home_path())
        .current_user_action_projection(
            &ProjectId::new(fixture.project_id()),
            &volicord_types::UserActionRequestId::new(&request_id),
        )?
        .ok_or("resolved action should have a current projection")?;
    assert_eq!(projection.status, UserActionStatus::Resolved);
    let safe_resolution = projection
        .user_action_resolution
        .as_ref()
        .ok_or("resolved action should have an agent projection")?;
    assert_eq!(
        safe_resolution.channel_kind,
        volicord_types::UserActionChannelKind::LocalWebConsent
    );
    let volicord_types::McpUserActionResolutionSummary::EvidenceObservation {
        target,
        artifact_refs,
        relevance_status,
    } = &safe_resolution.resolution_summary
    else {
        return Err("agent projection should preserve observation semantics".into());
    };
    assert_eq!(target, &selected_target);
    assert_eq!(artifact_refs, &vec![selected_artifact]);
    assert_eq!(*relevance_status, EvidenceRelevanceStatus::Supported);
    assert!(!format!("{projection:?}").contains(private_summary));

    let now =
        user_action_channel_current_timestamp(fixture.runtime_home_path(), fixture.project_id())?;
    let consumed = validate_user_action_channel_token(
        fixture.runtime_home_path(),
        UserActionChannelTokenCheck {
            token: token.to_owned(),
            expected_project_id: fixture.project_id().to_owned(),
            expected_connection_internal_id: fixture.connection_id().to_owned(),
            now,
        },
    )?;
    let UserActionChannelTokenValidation::Rejected(UserActionChannelTokenRejection::Consumed(
        consumed,
    )) = consumed
    else {
        return Err("successful local-web resolution should atomically consume its token".into());
    };
    assert_eq!(consumed.status, "consumed");
    assert!(consumed.consumed_at.is_some());
    assert!(consumed.completed_at.is_some());
    assert!(!consumed.completion_metadata_json.is_empty());

    let after = fixture.counts()?;
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.user_action_requests, before.user_action_requests);
    assert_eq!(
        after.user_action_resolutions,
        before.user_action_resolutions + 1
    );
    let replay = server.handle_request(consent_post_request(Some(consent_base_url()), &post_body));
    assert_eq!(replay.status, 200);
    assert!(http_body_text(&replay)?.contains("Resolution recorded"));
    assert_eq!(fixture.counts()?, after);
    Ok(())
}

#[test]
fn local_web_consent_form_digest_failures_are_closed_and_leave_tokens_unconsumed(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-form-digest-fail-closed")?;
    let (task_id, pending_response) = create_pending_product_action(&fixture)?;
    let canonical_action = pending_user_action_from_response(&pending_response)?
        .ok_or("response should include a canonical pending action")?;
    let exact_form_digest = canonical_json_bare_sha256(&canonical_action.inbox_item.form)?;
    let cases = [
        (
            "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
            json!({
                "fallback_kind": "local_web_consent",
                "endpoint": LOCAL_WEB_CONSENT_PATH
            }),
        ),
        (
            "dededededededededededededededededededededededededededededededede",
            json!({
                "fallback_kind": "local_web_consent",
                "endpoint": LOCAL_WEB_CONSENT_PATH,
                "form_digest": exact_form_digest,
                "unexpected": true
            }),
        ),
        (
            "efefefefefefefefefefefefefefefefefefefefefefefefefefefefefefefef",
            json!({
                "fallback_kind": "local_web_consent",
                "endpoint": LOCAL_WEB_CONSENT_PATH,
                "form_digest": 7
            }),
        ),
        (
            "fafafafafafafafafafafafafafafafafafafafafafafafafafafafafafafafa",
            json!({
                "fallback_kind": "local_web_consent",
                "endpoint": LOCAL_WEB_CONSENT_PATH,
                "form_digest": "0".repeat(64)
            }),
        ),
    ];
    let before = fixture.counts()?;
    let mut server = consent_server(&fixture)?;

    for (token, metadata) in cases {
        create_consent_token_for_response(&fixture, &pending_response, token)?;
        overwrite_consent_token_created_metadata(&fixture, token, &metadata)?;

        let get = server.handle_request(consent_get_request(&consent_target(
            fixture.project_id(),
            token,
        )));
        assert_eq!(get.status, 409);
        assert_local_web_consent_security_headers(&get);
        assert!(http_body_text(&get)?.contains("TOKEN_FORM_MISMATCH"));

        let post = server.handle_request(consent_post_request(
            Some(consent_base_url()),
            &format!(
                "project={}&token={}&selected_option_id=keep",
                percent_encode_query(fixture.project_id()),
                token
            ),
        ));
        assert_eq!(post.status, 409);
        assert_local_web_consent_security_headers(&post);
        assert!(http_body_text(&post)?.contains("TOKEN_FORM_MISMATCH"));
        assert_eq!(fixture.counts()?, before);

        let now = user_action_channel_current_timestamp(
            fixture.runtime_home_path(),
            fixture.project_id(),
        )?;
        let validation = validate_user_action_channel_token(
            fixture.runtime_home_path(),
            UserActionChannelTokenCheck {
                token: token.to_owned(),
                expected_project_id: fixture.project_id().to_owned(),
                expected_connection_internal_id: fixture.connection_id().to_owned(),
                now,
            },
        )?;
        let UserActionChannelTokenValidation::Valid(record) = validation else {
            return Err("a form-digest failure must leave its token pending".into());
        };
        assert_eq!(record.status, "pending");
        assert!(record.consumed_at.is_none());
        assert!(record.completed_at.is_none());
    }

    let record = stored_action_record(&fixture, &task_id, &pending_response.response_value)?;
    assert_eq!(record.status, UserActionStatus::Pending);
    assert!(record.resolution.is_none());
    Ok(())
}

#[test]
fn local_web_consent_rejects_origin_mismatch_without_consuming_token() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-local-web-origin")?;
    let (task_id, pending_response) = create_pending_product_action(&fixture)?;
    let token = "9999999999999999999999999999999999999999999999999999999999999999";
    create_consent_token_for_response(&fixture, &pending_response, token)?;
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
    assert!(http_body_text(&valid)?.contains("Resolution recorded"));
    let pending_value = pending_response.response_value;
    let record = stored_action_record(&fixture, &task_id, &pending_value)?;
    assert_eq!(record.status, UserActionStatus::Resolved);
    Ok(())
}

#[test]
fn local_web_consent_validation_failure_leaves_token_reusable() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-validation-retry")?;
    let (task_id, pending_response) = create_pending_product_action(&fixture)?;
    let token = "8888888888888888888888888888888888888888888888888888888888888888";
    create_consent_token_for_response(&fixture, &pending_response, token)?;
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
    assert!(http_body_text(&valid)?.contains("Resolution recorded"));
    let pending_value = pending_response.response_value;
    let record = stored_action_record(&fixture, &task_id, &pending_value)?;
    assert_eq!(record.status, UserActionStatus::Resolved);
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
    let (_task_id, pending_response) = create_pending_product_action(&fixture)?;
    let token = "3333333333333333333333333333333333333333333333333333333333333333";
    create_consent_token_for_response(&fixture, &pending_response, token)?;
    volicord_store::user_action_channel::expire_user_action_channel_tokens(
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
fn local_web_consent_duplicate_post_replays_safe_completion() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-local-web-replay")?;
    let (task_id, pending_response) = create_pending_product_action(&fixture)?;
    let action = pending_user_action_from_response(&pending_response)?
        .ok_or("response should include a pending user action")?;
    let request_id = action.request.user_action_request_id.as_str().to_owned();
    let token = "4444444444444444444444444444444444444444444444444444444444444444";
    create_consent_token_for_response(&fixture, &pending_response, token)?;
    let mut server = consent_server(&fixture)?;
    let form_body = format!(
        "project={}&token={}&selected_option_id=keep",
        percent_encode_query(fixture.project_id()),
        token
    );

    let first = server.handle_request(consent_post_request(Some(consent_base_url()), &form_body));
    assert_eq!(first.status, 200);
    set_user_action_basis_status(&fixture, &request_id, "stale")?;
    let stale = stored_action_record(&fixture, &task_id, &pending_response.response_value)?;
    assert_eq!(stale.status, UserActionStatus::Stale);
    let after_first = fixture.counts()?;
    let replay = server.handle_request(consent_post_request(Some(consent_base_url()), &form_body));

    assert_eq!(replay.status, 200);
    assert_local_web_consent_security_headers(&first);
    assert_local_web_consent_security_headers(&replay);
    assert!(http_body_text(&replay)?.contains("Resolution recorded"));
    assert_eq!(fixture.counts()?, after_first);
    Ok(())
}

#[test]
fn local_web_consent_hides_wrong_project_and_rejects_wrong_connection() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-local-web-context")?;
    let (_task_id, pending_response) = create_pending_product_action(&fixture)?;
    let token = "5555555555555555555555555555555555555555555555555555555555555555";
    create_consent_token_for_response(&fixture, &pending_response, token)?;

    let mut server = consent_server(&fixture)?;
    let wrong_project =
        server.handle_request(consent_get_request(&consent_target("project_other", token)));
    assert_eq!(wrong_project.status, 404);
    assert_local_web_consent_security_headers(&wrong_project);
    assert!(http_body_text(&wrong_project)?.contains("INVALID_TOKEN"));

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

fn adapter_for_additional_connection(
    fixture: &CoreFixture,
    connection_id: &str,
) -> Result<McpAdapter, Box<dyn Error>> {
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
            config_target: format!("{}_additional", existing.config_target),
            mode: existing.mode,
            enabled: existing.enabled,
            managed_fingerprint: format!("{}_additional", existing.managed_fingerprint),
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
    let context = McpConnectionContext::resolve(fixture.runtime_home_path(), connection_id)?
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

fn create_pending_product_action(
    fixture: &CoreFixture,
) -> Result<(String, PipelineResponse), Box<dyn Error>> {
    let setup_adapter = adapter(fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let response = setup_adapter.call_tool(
        "volicord.request_user_action",
        product_action_args(fixture, &task_id, state_version),
    )?;
    Ok((task_id, response))
}

fn invoke_pending_elicitation(
    fixture: &CoreFixture,
    pending_response: PipelineResponse,
    content: Value,
) -> Result<(Value, Value), Box<dyn Error>> {
    let input = Cursor::new(json_lines(&[json!({
        "jsonrpc":"2.0",
        "id":"elicit_user_action_1",
        "result":{
            "action":"accept",
            "content":content
        }
    })])?);
    let mut lines = BufReader::new(input).lines();
    let mut request_bytes = Vec::new();
    let mut request_sequence = 1;
    let output = crate::stdio::user_action_tool_output(
        &adapter(fixture)?,
        pending_response,
        true,
        true,
        &mut request_sequence,
        &mut lines,
        &mut request_bytes,
    )?;
    let requests = stdio_responses(&request_bytes)?;
    let request = requests
        .into_iter()
        .next()
        .ok_or("elicitation request should be emitted")?;
    Ok((request, crate::stdio::tool_call_result_from_output(output)))
}

fn user_action_side_effect_snapshot(
    fixture: &CoreFixture,
) -> Result<
    (
        volicord_store::core_pipeline::StorageEffectCounts,
        i64,
        String,
    ),
    Box<dyn Error>,
> {
    let counts = fixture.counts()?;
    let (tokens, project_updated_at) = fixture.conn()?.query_row(
        "SELECT (SELECT COUNT(*) FROM user_action_channel_tokens), updated_at
           FROM project_state",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok((counts, tokens, project_updated_at))
}

struct PendingEvidenceObservationAction {
    task_id: String,
    response: PipelineResponse,
    target_candidates: Vec<EvidenceTarget>,
    registered_artifacts: Vec<volicord_types::ArtifactRef>,
}

fn create_pending_evidence_observation_action(
    fixture: &CoreFixture,
) -> Result<PendingEvidenceObservationAction, Box<dyn Error>> {
    create_pending_evidence_observation_action_with_sensitive_markers(fixture, None, None)
}

fn create_pending_evidence_observation_action_with_sensitive_markers(
    fixture: &CoreFixture,
    target_marker: Option<&str>,
    artifact_marker: Option<&str>,
) -> Result<PendingEvidenceObservationAction, Box<dyn Error>> {
    let setup_adapter = adapter(fixture)?;
    let (task_id, _) = create_task(&setup_adapter)?;
    let baseline_ref = "baseline_local_web_evidence_observation";
    let scope = setup_adapter.call_tool(
        UPDATE_SCOPE_TOOL_NAME,
        json!({
            "task_id": task_id,
            "goal_summary": null,
            "scope_update": null,
            "scope_boundary": "Observe stored evidence candidates through local consent.",
            "non_goals": [],
            "acceptance_criteria": [
                {
                    "acceptance_criterion_id": null,
                    "statement": "The first local-web target remains available.",
                    "evidence_requirement": "required"
                },
                {
                    "acceptance_criterion_id": null,
                    "statement": "The selected local-web target is supported by exact bytes.",
                    "evidence_requirement": "required"
                }
            ],
            "autonomy_boundary": null,
            "baseline_ref": baseline_ref,
            "change_unit": {
                "operation": "create_current",
                "scope_summary": "Exercise local-web evidence observation.",
                "affected_paths": []
            },
            "related_scope_decision_refs": []
        }),
    )?;
    let change_unit_id = scope.response_value["state"]["active_change_unit_ref"]["record_id"]
        .as_str()
        .ok_or("scope response should expose the current Change Unit")?
        .to_owned();
    let target_candidates = if let Some(marker) = target_marker {
        vec![
            EvidenceTarget::SupplementalClaim {
                evidence_claim_id: volicord_types::EvidenceClaimId::new(
                    "claim_sensitive_presentation_a",
                ),
                statement: format!(
                    "An API key must be handled only through a user-only channel: {marker}"
                ),
            },
            EvidenceTarget::SupplementalClaim {
                evidence_claim_id: volicord_types::EvidenceClaimId::new(
                    "claim_sensitive_presentation_b",
                ),
                statement: "Ordinary secondary evidence claim.".to_owned(),
            },
        ]
    } else {
        scope.response_value["state"]["acceptance_criteria"]
            .as_array()
            .ok_or("scope response should expose acceptance criteria")?
            .iter()
            .map(|criterion| {
                criterion["acceptance_criterion_id"]
                    .as_str()
                    .ok_or("criterion should expose its id")
                    .map(|id| EvidenceTarget::AcceptanceCriterion {
                        acceptance_criterion_id: volicord_types::AcceptanceCriterionId::new(id),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    assert_eq!(target_candidates.len(), 2);

    let mut staged_handles = Vec::new();
    let display_names = if let Some(marker) = artifact_marker {
        [
            format!("credential-material-{marker}.txt"),
            "local-web-candidate-b.txt".to_owned(),
        ]
    } else {
        [
            "local-web-candidate-a.txt".to_owned(),
            "local-web-candidate-b.txt".to_owned(),
        ]
    };
    for (display_name, bytes) in display_names.into_iter().zip([
        "First local-web evidence candidate bytes.",
        "Selected local-web evidence candidate bytes.",
    ]) {
        let staged = setup_adapter.call_tool(
            STAGE_ARTIFACT_TOOL_NAME,
            json!({
                "task_id": task_id,
                "display_name": display_name,
                "content_type": "text/plain",
                "redaction_state": "none",
                "safe_bytes_or_notice": bytes
            }),
        )?;
        staged_handles.push(staged.response_value["staged_artifact_handle"].clone());
    }
    let recorded = setup_adapter.call_tool(
        RECORD_RUN_TOOL_NAME,
        json!({
            "task_id": task_id,
            "change_unit_id": change_unit_id,
            "kind": "implementation",
            "baseline_ref": baseline_ref,
            "summary": "Register local-web evidence-observation candidates.",
            "observed_changes": {
                "changed_paths": [],
                "product_file_write_observed": false,
                "sensitive_categories": [],
                "baseline_ref": baseline_ref
            },
            "artifact_inputs": [
                {
                    "artifact_input_id": "artifact_input_local_web_candidate_a",
                    "source_kind": "staged_artifact",
                    "staged_artifact_handle": staged_handles[0],
                    "existing_artifact_ref": null,
                    "relation_hint": "local_web_user_observation_candidate",
                    "evidence_target": target_candidates[0],
                    "expected_sha256": null,
                    "expected_size_bytes": null,
                    "redaction_state": "none"
                },
                {
                    "artifact_input_id": "artifact_input_local_web_candidate_b",
                    "source_kind": "staged_artifact",
                    "staged_artifact_handle": staged_handles[1],
                    "existing_artifact_ref": null,
                    "relation_hint": "local_web_user_observation_candidate",
                    "evidence_target": target_candidates[1],
                    "expected_sha256": null,
                    "expected_size_bytes": null,
                    "redaction_state": "none"
                }
            ],
            "evidence_updates": [],
            "evidence_observations": [],
            "close_assessment": null
        }),
    )?;
    let registered_artifacts = recorded.response_value["registered_artifacts"]
        .as_array()
        .ok_or("record_run should expose registered artifacts")?
        .iter()
        .cloned()
        .map(serde_json::from_value)
        .collect::<Result<Vec<volicord_types::ArtifactRef>, _>>()?;
    assert_eq!(registered_artifacts.len(), 2);
    let target_values = target_candidates
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()?;
    let artifact_candidate_ids = registered_artifacts
        .iter()
        .map(|artifact| artifact.artifact_id.as_str().to_owned())
        .collect::<Vec<_>>();
    let response = setup_adapter.call_tool(
        REQUEST_USER_ACTION_TOOL_NAME,
        evidence_observation_action_args(
            &task_id,
            &change_unit_id,
            target_values,
            artifact_candidate_ids,
        ),
    )?;
    Ok(PendingEvidenceObservationAction {
        task_id,
        response,
        target_candidates,
        registered_artifacts,
    })
}

fn create_consent_token_for_response(
    fixture: &CoreFixture,
    response: &PipelineResponse,
    token: &str,
) -> Result<(), Box<dyn Error>> {
    let action = pending_user_action_from_response(response)?
        .ok_or("response should include a pending user action")?;
    let form_digest = canonical_json_bare_sha256(&action.inbox_item.form)?;
    create_user_action_channel_token(
        fixture.runtime_home_path(),
        UserActionChannelTokenCreate {
            token: token.to_owned(),
            project_id: action.request.project_id.as_str().to_owned(),
            channel_kind: UserActionChannelKind::LocalWebConsent,
            connection_internal_id: fixture.connection_id().to_owned(),
            user_action_request_id: action.request.user_action_request_id.as_str().to_owned(),
            capture_basis: VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB.to_owned(),
            created_metadata_json: json!({
                "fallback_kind": "local_web_consent",
                "endpoint": LOCAL_WEB_CONSENT_PATH,
                "form_digest": form_digest
            })
            .to_string(),
        },
    )?;
    Ok(())
}

fn overwrite_consent_token_created_metadata(
    fixture: &CoreFixture,
    token: &str,
    metadata: &Value,
) -> Result<(), Box<dyn Error>> {
    let token_hash = volicord_store::user_action_channel::user_action_channel_token_hash(token)?;
    let changed = fixture.conn()?.execute(
        "UPDATE user_action_channel_tokens
            SET created_metadata_json = ?3
          WHERE project_id = ?1
            AND token_hash = ?2",
        (fixture.project_id(), token_hash, metadata.to_string()),
    )?;
    assert_eq!(changed, 1);
    Ok(())
}

fn set_user_action_basis_status(
    fixture: &CoreFixture,
    user_action_request_id: &str,
    basis_status: &str,
) -> Result<(), Box<dyn Error>> {
    let basis_json: String = fixture.conn()?.query_row(
        "SELECT basis_json
           FROM user_action_requests
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        (fixture.project_id(), user_action_request_id),
        |row| row.get(0),
    )?;
    let mut basis: Value = serde_json::from_str(&basis_json)?;
    basis["coordinates"]["compatibility_status"] = json!(basis_status);
    fixture.conn()?.execute(
        "UPDATE user_action_requests
            SET basis_status = ?3,
                basis_json = ?4
          WHERE project_id = ?1
            AND user_action_request_id = ?2",
        (
            fixture.project_id(),
            user_action_request_id,
            basis_status,
            basis.to_string(),
        ),
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
    let fields = parse_urlencoded(query)?;
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
            "acceptance_policy": null,
            "lineage": null,
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
        "acceptance_policy": null,
        "lineage": null,
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

fn product_action_args(fixture: &CoreFixture, task_id: &str, state_version: u64) -> Value {
    action_args(
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

fn evidence_observation_action_args(
    task_id: &str,
    change_unit_id: &str,
    target_candidates: Vec<Value>,
    artifact_candidate_ids: Vec<String>,
) -> Value {
    json!({
        "request": {
            "operation": "create",
            "task_id": task_id,
            "change_unit_id": change_unit_id,
            "action": {
                "action_type": "evidence_observation",
                "question": "Does an exact stored artifact support the selected target?",
                "context_summary": "Inspect stored candidate bytes and record a user-owned observation.",
                "target_candidates": target_candidates,
                "artifact_candidate_ids": artifact_candidate_ids
            },
            "required_for": ["record_run"],
            "expires_at": null
        }
    })
}

fn resume_user_action_args(fixture: &CoreFixture, user_action_request_id: &str) -> Value {
    json!({
        "project_selector": fixture.project_id(),
        "detail": "full",
        "request": {
            "operation": "resume",
            "user_action_request_id": user_action_request_id
        }
    })
}

fn default_product_action_args(fixture: &CoreFixture, task_id: &str, state_version: u64) -> Value {
    let mut arguments = product_action_args(fixture, task_id, state_version);
    arguments
        .as_object_mut()
        .expect("judgment arguments should be an object")
        .remove("detail");
    arguments
}

#[derive(Default)]
struct FailElicitationRequestWriter {
    output: Vec<u8>,
    pending_line: Vec<u8>,
    failed_elicitation: bool,
}

impl Write for FailElicitationRequestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes == b"\n" {
            let is_elicitation = self
                .pending_line
                .windows(b"elicitation/create".len())
                .any(|window| window == b"elicitation/create");
            if is_elicitation && !self.failed_elicitation {
                self.failed_elicitation = true;
                self.pending_line.clear();
                return Err(io::Error::other(
                    "fixture rejected the elicitation server request",
                ));
            }
            self.output.append(&mut self.pending_line);
            self.output.push(b'\n');
            return Ok(1);
        }
        self.pending_line.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.pending_line.is_empty() {
            self.output.append(&mut self.pending_line);
        }
        Ok(())
    }
}

fn authority_action_args(fixture: &CoreFixture, task_id: &str, state_version: u64) -> Value {
    action_args(
        fixture,
        task_id,
        state_version,
        "scope_decision",
        Value::Null,
        json!(["scope_update"]),
    )
}

fn action_args(
    fixture: &CoreFixture,
    task_id: &str,
    state_version: u64,
    judgment_kind: &str,
    options: Value,
    required_for: Value,
) -> Value {
    json!({
        "detail": "full",
        "request": {
            "operation": "create",
            "task_id": task_id,
            "change_unit_id": null,
            "action": {
                "action_type": "choice",
                "judgment_kind": judgment_kind,
                "presentation": "short",
                "question": "Choose the focused User Channel test outcome.",
                "options": options,
                "context": {
                    "summary": "A focused test user action needs a user-owned answer.",
                    "related_refs": [],
                    "artifact_refs": [],
                    "visible_risks": [],
                    "constraints": ["The answer covers only this pending user action."]
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
                "sensitive_action_scope": null
            },
            "required_for": required_for,
            "expires_at": null
        }
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
        "id": "elicit_user_action_1",
        "result": {
            "action": "accept",
            "content": content
        }
    })
}

fn elicitation_accept_observation(
    selected_target: &str,
    selected_artifact_ids: &[String],
    relevance_status: &str,
    summary: &str,
) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": "elicit_user_action_1",
        "result": {
            "action": "accept",
            "content": {
                "selected_target": selected_target,
                "selected_artifact_ids": selected_artifact_ids,
                "relevance_status": relevance_status,
                "summary": summary
            }
        }
    })
}

fn elicitation_action(action: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": "elicit_user_action_1",
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
    let structured = response["result"]
        .get("structuredContent")
        .filter(|value| value.is_object())
        .cloned()
        .ok_or("tools/call response should include structured content")?;
    Ok(structured
        .get("method_result")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or(structured))
}

fn assert_pending_user_action_resume_guidance(
    tool_response: &Value,
    response: &Value,
    fixture: &CoreFixture,
) -> Result<(), Box<dyn Error>> {
    let request_id =
        &response["agent_workflow_result"]["user_action_request"]["user_action_request_id"];
    let content = tool_response["result"]["content"]
        .as_array()
        .ok_or("tool response content should be an array")?;
    assert!(content.iter().any(|item| {
        item["text"]
            .as_str()
            .is_some_and(|text| text.contains("nested `request.operation=resume`"))
    }));
    let state = content
        .iter()
        .filter_map(|item| item["text"].as_str())
        .filter_map(|text| serde_json::from_str::<Value>(text).ok())
        .find(|value| value["volicord_fallback"]["kind"] == "resume_pending_user_action")
        .ok_or("pending response should include structured resume guidance")?;
    let state = &state["volicord_fallback"];
    assert_eq!(state["user_action_request_id"], *request_id);
    assert_eq!(state["resume"]["tool_name"], REQUEST_USER_ACTION_TOOL_NAME);
    assert_eq!(state["resume"]["creates_new_request"], false);
    assert_eq!(
        state["resume"]["arguments"]["project_selector"],
        fixture.project_id()
    );
    assert_eq!(
        state["resume"]["arguments"]["request"]["operation"],
        "resume"
    );
    assert_eq!(
        state["resume"]["arguments"]["request"]["user_action_request_id"],
        *request_id
    );
    Ok(())
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
    let record = stored_action_record(fixture, task_id, response)?;
    record
        .resolution
        .map(|resolution| resolution.resolved_verification_basis)
        .ok_or_else(|| "stored user action should have a resolution basis".into())
}

fn stored_action_record(
    fixture: &CoreFixture,
    task_id: &str,
    response: &Value,
) -> Result<volicord_store::core_pipeline::EffectiveUserActionRecord, Box<dyn Error>> {
    let request_id = response
        .pointer("/agent_workflow_result/user_action_request_ref/record_id")
        .or_else(|| response.pointer("/user_action_request_ref/record_id"))
        .and_then(Value::as_str)
        .ok_or("response should include user_action_request_ref.record_id")?;
    let store = CoreProjectStore::open(
        fixture.runtime_home_path(),
        &ProjectId::new(fixture.project_id()),
    )?;
    let now =
        user_action_channel_current_timestamp(fixture.runtime_home_path(), fixture.project_id())?;
    let now = volicord_types::UtcTimestamp::parse(&now)?;
    let record = store
        .user_action_records_for_task(&volicord_types::TaskId::new(task_id), &now)?
        .into_iter()
        .find(|record| record.request.user_action_request_id == request_id)
        .ok_or("stored user-action record should exist")?;
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
        GET_OPERATION_RESULT_TOOL_NAME => serde_json::to_value(serde_json::from_value::<
            McpGetOperationResultArguments,
        >(value)?),
        PREPARE_EVIDENCE_CAPTURE_TOOL_NAME => serde_json::to_value(serde_json::from_value::<
            McpPrepareEvidenceCaptureArguments,
        >(value)?),
        PREPARE_WRITE_TOOL_NAME => {
            serde_json::to_value(serde_json::from_value::<McpPrepareWriteArguments>(value)?)
        }
        STAGE_ARTIFACT_TOOL_NAME => {
            serde_json::to_value(serde_json::from_value::<McpStageArtifactArguments>(value)?)
        }
        RECORD_RUN_TOOL_NAME => {
            serde_json::to_value(serde_json::from_value::<McpRecordRunArguments>(value)?)
        }
        REQUEST_USER_ACTION_TOOL_NAME => serde_json::to_value(serde_json::from_value::<
            McpRequestUserActionArguments,
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
        .is_some_and(|definitions| {
            definitions.keys().any(|definition| {
                definition == name || definition.starts_with(&format!("{name}_for_"))
            })
        })
}

fn schema_variant_by_tag<'a>(schema: &'a Value, tag: &str, value: &str) -> Option<&'a Value> {
    let tag_schema = schema
        .get("properties")
        .and_then(|properties| properties.get(tag));
    let matches_tag = tag_schema.is_some_and(|tag_schema| {
        tag_schema.get("const").and_then(Value::as_str) == Some(value)
            || tag_schema
                .get("enum")
                .and_then(Value::as_array)
                .is_some_and(|values| values.iter().any(|candidate| candidate == value))
    });
    if matches_tag {
        return Some(schema);
    }
    match schema {
        Value::Array(values) => values
            .iter()
            .find_map(|schema| schema_variant_by_tag(schema, tag, value)),
        Value::Object(object) => object
            .values()
            .find_map(|schema| schema_variant_by_tag(schema, tag, value)),
        _ => None,
    }
}

fn schema_requires_property(schema: &Value, field: &str) -> bool {
    if schema
        .get("properties")
        .and_then(Value::as_object)
        .is_some_and(|properties| properties.contains_key(field))
        && root_required_fields(schema)
            .iter()
            .any(|required| required == field)
    {
        return true;
    }
    match schema {
        Value::Array(values) => values
            .iter()
            .any(|value| schema_requires_property(value, field)),
        Value::Object(object) => object
            .values()
            .any(|value| schema_requires_property(value, field)),
        _ => false,
    }
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
