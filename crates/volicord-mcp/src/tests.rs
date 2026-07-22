use std::{
    collections::BTreeSet,
    error::Error,
    ffi::OsString,
    fs,
    io::{BufReader, Cursor},
    panic::{catch_unwind, AssertUnwindSafe},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::prelude::*;
use crate::stdio::{
    classify_launch_origin, handle_json_rpc_message, run_stdio_with_env_marker,
    tool_execution_error_result, ConnectionPhase, ConnectionState, McpLaunchOrigin,
    MAX_MCP_COMPACT_MUTATION_RESULT_BYTES, MAX_MCP_FULL_MUTATION_RESULT_BYTES,
    MAX_MCP_MUTATION_COMPATIBILITY_TEXT_BYTES,
};
use crate::{
    adapter::{AgentSessionCoordinates, ManagedAgentSessionBinding},
    routing::McpStorageCapability,
    tool_registry::{
        canonical_tool_examples, compact_runtime_schema, mcp_tool_naming_style,
        mcp_tools_for_mode_and_storage, mcp_tools_for_mode_and_storage_with_detail,
        validate_tools_list_json_compatibility, validate_tools_list_schema_compatibility,
        ToolSchemaDetail, CHECK_CLOSE_MISSING_FINAL_ACCEPTANCE_EXAMPLE_ID,
        GET_OPERATION_RESULT_FIRST_PAGE_EXAMPLE_ID, MAX_RUNTIME_TOOLS_LIST_BYTES,
        PREPARE_EVIDENCE_CAPTURE_VERIFIED_COMMAND_EXAMPLE_ID,
        PREPARE_EVIDENCE_CAPTURE_VERIFIED_TOOL_EXAMPLE_ID, PREPARE_WRITE_SIMPLE_EXAMPLE_ID,
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
        REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID, STATUS_READ_ONLY_EXAMPLE_ID,
        UPDATE_SCOPE_KEEP_CURRENT_EXAMPLE_ID,
    },
};
use volicord_core::CoreBoundary;
use volicord_mcp_protocol::ToolResultField;
use volicord_store::agent_connections::{
    add_connection_project, agent_connection_record, ensure_agent_connection,
    set_connection_enabled, AgentConnectionRegistration, ConnectionProjectRegistration,
    CONNECTION_MODE_READ_ONLY, HOST_KIND_CODEX,
};
use volicord_store::bootstrap::{register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS};
use volicord_store::diagnostic_findings::{
    diagnostic_findings_by_ids, diagnostic_occurrences_for_runtime_session,
};
use volicord_store::diagnostics::{
    diagnostics_db_path, read_diagnostic_session, read_workflow_metric_aggregates,
    WorkflowMetricAggregateRow,
};
use volicord_store::guards::{agent_session, upsert_guard_installation, GuardInstallationUpsert};
use volicord_store::operational_sessions::{latest_managed_runtime_session, mcp_runtime_session};
use volicord_store::sqlite::{open_registry_database_read_only, registry_db_path};
use volicord_test_support::core_fixtures::{
    artifact_input_for_handle, CoreFixture, ResolveUserActionFixture, UpdateScopeFixture,
    UserActionFixture,
};
use volicord_types::{
    AgentConnectionMode, ChangeUnitOperation, CloseAssessmentInput, OperationCategory,
    ResidualRiskInput, StagedArtifactHandle, CODEX_MANAGED_MCP_CLIENT_NAME,
};

fn conformance_profiles(
) -> impl ExactSizeIterator<Item = &'static volicord_mcp_protocol::McpProtocolProfile> {
    crate::volicord_conformance_covered_revisions()
        .iter()
        .map(|revision| {
            ProtocolRegistry::production()
                .profile(*revision)
                .expect("conformance-covered revision must have a production profile")
        })
}

use super::*;

const CODEX_TEST_SESSION_ID: &str = "fixture_codex_session";
const CODEX_TEST_THREAD_ID: &str = "fixture_codex_thread";
const CODEX_TEST_TURN_ID: &str = "fixture_codex_turn";
const CODEX_TEST_CLIENT_VERSION: &str = "test-codex-client";

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
    let expected = AgentToolId::ALL
        .iter()
        .map(|tool| tool.wire_name())
        .collect::<Vec<_>>();
    assert_eq!(workflow_names, expected);
    assert!(workflow_names.contains(&AgentToolId::REQUEST_USER_ACTION.wire_name()));
    assert!(workflow_names.contains(&AgentToolId::RECONCILE_CHANGES.wire_name()));
    assert!(workflow_names.contains(&AgentToolId::PREPARE_EVIDENCE_CAPTURE.wire_name()));
    assert!(workflow_names.contains(&AgentToolId::CHECK_CLOSE.wire_name()));
    assert!(workflow_names.contains(&AgentToolId::CLOSE_TASK.wire_name()));
    assert!(!workflow_names.contains(&MethodName::ResolveUserAction.as_str()));
    assert_eq!(
        workflow_names.last().copied(),
        Some(AgentToolId::LIST_PROJECTS.wire_name())
    );

    let read_only = mcp_tools_for_mode(AgentConnectionMode::ReadOnly);
    let read_only_names = tool_names(&read_only);
    assert!(!read_only_names.contains(&AgentToolId::PREPARE_EVIDENCE_CAPTURE.wire_name()));
    assert_eq!(
        read_only_names,
        vec![
            AgentToolId::STATUS.wire_name(),
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
            AgentToolId::CHECK_CLOSE.wire_name(),
            AgentToolId::LIST_PROJECTS.wire_name()
        ]
    );
}

#[test]
fn canonical_managed_host_round_trip_role_resolves_to_one_exposed_tool() {
    let designated = ToolVerificationRole::ManagedHostRoundTrip.tool();
    assert_eq!(designated, AgentToolId::LIST_PROJECTS);
    let tools = mcp_tools_for_mode(AgentConnectionMode::Workflow);
    assert_eq!(tools.iter().filter(|tool| tool.id == designated).count(), 1);
}

#[test]
fn mcp_visible_schemas_hide_envelope_and_metadata() {
    for tool in public_method_tools() {
        let properties = root_properties(&tool.input_schema);
        let required = root_required_fields(&tool.input_schema);
        assert!(
            properties.contains(&"project_selector".to_owned()),
            "{} should expose the public project selector",
            tool.id.wire_name()
        );
        assert!(
            !required.contains(&"project_selector".to_owned()),
            "{} should not require project selection for single-project connections",
            tool.id.wire_name()
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
                tool.id.wire_name()
            );
        }
        assert!(
            !schema_has_definition(&tool.input_schema, "ToolEnvelope"),
            "{} should not include the internal ToolEnvelope schema",
            tool.id.wire_name()
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
fn runtime_tools_list_is_compact_example_free_and_validation_equivalent() {
    for mode in [AgentConnectionMode::Workflow, AgentConnectionMode::ReadOnly] {
        for storage_capability in [
            McpStorageCapability::ReadWrite,
            McpStorageCapability::ReadOnly,
            McpStorageCapability::Unavailable,
            McpStorageCapability::Unknown,
        ] {
            let runtime = mcp_tools_for_mode_and_storage_with_detail(
                mode,
                storage_capability,
                ToolSchemaDetail::RuntimeCompact,
            );
            let documentation = mcp_tools_for_mode_and_storage_with_detail(
                mode,
                storage_capability,
                ToolSchemaDetail::Documentation,
            );

            assert_eq!(tool_names(&runtime), tool_names(&documentation));
            for (runtime_tool, documentation_tool) in runtime.iter().zip(&documentation) {
                assert_eq!(
                    runtime_tool.id.wire_name(),
                    documentation_tool.id.wire_name()
                );
                assert_eq!(runtime_tool.annotations, documentation_tool.annotations);
                assert_eq!(runtime_tool.output_schema, json!({ "type": "object" }));
                assert_eq!(documentation_tool.output_schema["type"], "object");

                assert_eq!(
                    root_properties(&runtime_tool.input_schema),
                    root_properties(&documentation_tool.input_schema),
                    "{} compact schema must preserve top-level properties",
                    runtime_tool.id.wire_name()
                );
                assert_eq!(
                    root_required_fields(&runtime_tool.input_schema),
                    root_required_fields(&documentation_tool.input_schema),
                    "{} compact schema must preserve top-level required fields",
                    runtime_tool.id.wire_name()
                );
                assert_eq!(
                    runtime_tool.input_schema.get("additionalProperties"),
                    documentation_tool.input_schema.get("additionalProperties"),
                    "{} compact schema must preserve the closed root",
                    runtime_tool.id.wire_name()
                );

                let mut documented_input = documentation_tool.input_schema.clone();
                documented_input
                    .as_object_mut()
                    .expect("tool input schema should be an object")
                    .remove("examples");
                strip_schema_presentation_for_test(&mut documented_input);
                assert_eq!(runtime_tool.input_schema, documented_input);
                assert!(
                    !json_member_exists(&runtime_tool.input_schema, "examples"),
                    "{} runtime input schema must not contain examples",
                    runtime_tool.id.wire_name()
                );
                assert_local_schema_refs_resolve(
                    &runtime_tool.input_schema,
                    runtime_tool.id.wire_name(),
                );
            }

            let payload = serde_json::to_vec(&json!({ "tools": runtime }))
                .expect("runtime tools/list result should serialize");
            assert!(
                payload.len() <= MAX_RUNTIME_TOOLS_LIST_BYTES,
                "{mode:?}/{storage_capability:?} runtime tools/list is {} bytes (limit {})",
                payload.len(),
                MAX_RUNTIME_TOOLS_LIST_BYTES
            );
        }
    }
}

#[test]
fn runtime_schema_compaction_preserves_data_properties_named_like_schema_keywords() {
    let mut schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Presentation-only root title",
        "type": "object",
        "properties": {
            "description": {
                "description": "Presentation-only field description",
                "$ref": "#/definitions/Text"
            },
            "definitions": {
                "type": "object",
                "properties": {
                    "default": { "$ref": "#/definitions/Text" }
                },
                "additionalProperties": false
            }
        },
        "required": ["description", "definitions"],
        "additionalProperties": false,
        "definitions": {
            "Text": {
                "title": "Presentation-only definition title",
                "type": "string",
                "minLength": 1
            },
            "Unused": { "type": "string" }
        }
    });

    compact_runtime_schema(&mut schema);

    let properties = schema["properties"]
        .as_object()
        .expect("compacted fixture should retain its properties");
    assert!(properties.contains_key("description"));
    assert!(properties.contains_key("definitions"));
    assert!(properties["definitions"]["properties"]
        .as_object()
        .is_some_and(|properties| properties.contains_key("default")));
    assert_eq!(schema["required"], json!(["description", "definitions"]));
    assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
    assert!(!json_member_exists(&schema, "title"));
    assert_local_schema_refs_resolve(&schema, "keyword-named-properties fixture");
    assert_eq!(
        schema["definitions"]
            .as_object()
            .expect("the shared definition should remain")
            .len(),
        1
    );
}

#[test]
fn stdio_workflow_metrics_record_exact_tools_list_method_outcomes_and_status_rereads(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-workflow-metrics")?;
    let private_marker = "private_prompt_marker_must_not_be_persisted";
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        request(2, "tools/list", json!({})),
        tools_call(
            3,
            AgentToolId::STATUS.wire_name(),
            json!({ "detail": "workflow" }),
        ),
        tools_call(
            4,
            AgentToolId::STATUS.wire_name(),
            json!({ "detail": "workflow" }),
        ),
        tools_call(
            5,
            AgentToolId::CHECK_CLOSE.wire_name(),
            json!({
                "private_marker": private_marker
            }),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio_with_env_marker(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
        |name| managed_codex_stdio_env(&fixture, name),
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 5);
    assert_eq!(responses[2]["result"]["isError"], false);
    assert_eq!(responses[3]["result"]["isError"], false);
    assert_eq!(responses[4]["result"]["isError"], true);
    let exact_tools_list_bytes = u64::try_from(serde_json::to_vec(&responses[1]["result"])?.len())?;
    let metrics =
        read_workflow_metric_aggregates(fixture.runtime_home_path(), fixture.project_id())?;

    let tools_list = workflow_metric_row(
        &metrics,
        WorkflowMetricKind::ToolsListSerializedBytes,
        None,
        Some(WorkflowMetricOutcome::Success),
    );
    assert_eq!(tools_list.sample_count, 1);
    assert_eq!(tools_list.host_kind.as_deref(), Some("codex"));
    assert_eq!(tools_list.value_total, exact_tools_list_bytes);
    assert_eq!(tools_list.value_min, exact_tools_list_bytes);
    assert_eq!(tools_list.value_max, exact_tools_list_bytes);

    let successful_status = workflow_metric_row(
        &metrics,
        WorkflowMetricKind::McpMethodCall,
        Some(MethodName::Status),
        Some(WorkflowMetricOutcome::Success),
    );
    assert_eq!(successful_status.sample_count, 2);
    assert_eq!(successful_status.value_total, 2);
    let invalid_check_close = workflow_metric_row(
        &metrics,
        WorkflowMetricKind::McpMethodCall,
        Some(MethodName::CheckClose),
        Some(WorkflowMetricOutcome::ValidationFailure),
    );
    assert_eq!(invalid_check_close.sample_count, 1);
    assert_eq!(invalid_check_close.value_total, 1);
    let status_reread = workflow_metric_row(
        &metrics,
        WorkflowMetricKind::StatusReread,
        None,
        Some(WorkflowMetricOutcome::Success),
    );
    assert_eq!(status_reread.sample_count, 1);
    assert_eq!(status_reread.value_total, 1);

    let diagnostics_bytes = fs::read(diagnostics_db_path(fixture.runtime_home_path()))?;
    assert!(!String::from_utf8_lossy(&diagnostics_bytes).contains(private_marker));
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
            AgentToolId::STATUS.wire_name(),
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            AgentToolId::CHECK_CLOSE.wire_name(),
            AgentToolId::LIST_PROJECTS.wire_name()
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
    let expected = AgentToolId::ALL
        .iter()
        .map(|tool| tool.wire_name())
        .collect::<Vec<_>>();

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
            tool.output_schema["type"],
            "object",
            "{} output schema should have an object root",
            tool.id.wire_name()
        );
        assert!(
            schema_has_definition(&tool.output_schema, "McpToolErrorResponse"),
            "{} output schema should cover structured adapter failures",
            tool.id.wire_name()
        );
        if !matches!(tool.id.category(), AgentToolCategory::ReadOnly) {
            assert!(
                schema_has_definition(&tool.output_schema, "McpMutationResponseBudgetExceeded"),
                "{} output schema should cover compact response-budget failures",
                tool.id.wire_name()
            );
            assert!(
                schema_has_definition(&tool.output_schema, "McpMutationPostEffectFailure"),
                "{} output schema should cover post-effect adapter failures",
                tool.id.wire_name()
            );
        }

        let expected_annotations = match tool.id.category() {
            AgentToolCategory::ReadOnly => CanonicalToolAnnotations {
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: true,
                open_world_hint: false,
            },
            AgentToolCategory::NonDestructiveMutation => CanonicalToolAnnotations {
                read_only_hint: false,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
            },
            AgentToolCategory::DestructiveMutation => CanonicalToolAnnotations {
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: false,
            },
        };
        assert_eq!(
            tool.annotations,
            expected_annotations,
            "{} annotations should match its effect boundary",
            tool.id.wire_name()
        );
    }
}

#[test]
fn request_user_action_output_schema_covers_compound_agent_safe_response() {
    let schema = tool_definition(AgentToolId::REQUEST_USER_ACTION.wire_name()).output_schema;

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
            AgentToolId::INTAKE.wire_name(),
            "create_new",
            vec![
                ("initial_context_refs", json!([])),
                ("initial_source_refs", json!([])),
            ],
        ),
        (
            AgentToolId::UPDATE_SCOPE.wire_name(),
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
            AgentToolId::PREPARE_WRITE.wire_name(),
            PREPARE_WRITE_SIMPLE_EXAMPLE_ID,
            vec![
                ("task_id", Value::Null),
                ("change_unit_id", Value::Null),
                ("sensitive_categories", json!([])),
            ],
        ),
        (
            AgentToolId::STAGE_ARTIFACT.wire_name(),
            "stage_safe_text",
            vec![
                ("expected_sha256", Value::Null),
                ("expected_size_bytes", Value::Null),
                ("relation_hint", Value::Null),
            ],
        ),
        (
            AgentToolId::RECORD_RUN.wire_name(),
            RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
            vec![
                ("run_id", Value::Null),
                ("write_ticket_id", Value::Null),
                ("performed_operation", Value::Null),
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

    let tool = tool_definition(AgentToolId::REQUEST_USER_ACTION.wire_name());
    let decoded = decode_mcp_arguments_to_value(
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
        canonical_example_value(
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
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
            "{}{pointer} should be omittable",
            AgentToolId::REQUEST_USER_ACTION.wire_name()
        );
        assert_eq!(
            create_schema["properties"][field]["default"],
            expected,
            "{}{pointer} should advertise its exact omission default",
            AgentToolId::REQUEST_USER_ACTION.wire_name()
        );
        assert_eq!(
            decoded.pointer(pointer),
            Some(&expected),
            "{}{pointer} omission should decode to the advertised default",
            AgentToolId::REQUEST_USER_ACTION.wire_name()
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

    for tool in [
        AgentToolId::INTAKE,
        AgentToolId::UPDATE_SCOPE,
        AgentToolId::PREPARE_EVIDENCE_CAPTURE,
        AgentToolId::PREPARE_WRITE,
        AgentToolId::STAGE_ARTIFACT,
        AgentToolId::RECORD_RUN,
        AgentToolId::REQUEST_USER_ACTION,
        AgentToolId::RECONCILE_CHANGES,
        AgentToolId::CLOSE_TASK,
    ] {
        let tool_name = tool.wire_name();
        let tool = tool_definition(tool_name);
        assert!(!root_required_fields(&tool.input_schema)
            .iter()
            .any(|field| field == "detail"));
        assert_eq!(
            tool.input_schema["properties"]["detail"]["default"],
            "summary"
        );
        let example = canonical_tool_examples(tool.id)
            .first()
            .expect("mutation tool should advertise an example");
        let decoded = decode_mcp_arguments_to_value(
            tool_name,
            serde_json::from_str(example.arguments_json)?,
        )?;
        let example_detail = if matches!(
            tool.id,
            AgentToolId::PREPARE_WRITE
                | AgentToolId::STAGE_ARTIFACT
                | AgentToolId::RECONCILE_CHANGES
        ) {
            "full"
        } else {
            "summary"
        };
        assert_eq!(decoded["detail"], example_detail);
    }

    assert_eq!(
        root_required_fields(
            &tool_definition(AgentToolId::REQUEST_USER_ACTION.wire_name()).input_schema
        )
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
    ];

    for (arguments, expected_capture, foreign_field) in cases {
        crate::schema_validation::validate_mcp_tool_arguments(
            AgentToolId::PREPARE_EVIDENCE_CAPTURE.wire_name(),
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
            AgentToolId::PREPARE_EVIDENCE_CAPTURE.wire_name(),
            &invalid,
        )
        .is_err());
        assert!(serde_json::from_value::<McpPrepareEvidenceCaptureArguments>(invalid).is_err());
    }

    Ok(())
}

#[test]
fn mcp_omission_defaults_do_not_change_core_request_required_members() {
    let cases = [
        (
            AgentToolId::INTAKE.wire_name(),
            &["initial_context_refs", "initial_source_refs"][..],
        ),
        (
            AgentToolId::UPDATE_SCOPE.wire_name(),
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
            AgentToolId::PREPARE_WRITE.wire_name(),
            &["task_id", "change_unit_id", "sensitive_categories"][..],
        ),
        (
            AgentToolId::STAGE_ARTIFACT.wire_name(),
            &["expected_sha256", "expected_size_bytes", "relation_hint"][..],
        ),
        (
            AgentToolId::RECORD_RUN.wire_name(),
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
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
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

    let schema =
        volicord_types::public_request_schema(AgentToolId::REQUEST_USER_ACTION.wire_name())
            .expect("request-user-action public Core schema should exist");
    for field in ["affected_refs", "sensitive_action_scope"] {
        assert!(
            schema_requires_property(&schema, field),
            "{}.action.{field} should remain a required Core request member",
            AgentToolId::REQUEST_USER_ACTION.wire_name()
        );
    }
}

#[test]
fn advertised_mcp_examples_cover_supported_branches_and_validate() -> Result<(), Box<dyn Error>> {
    let expected_branches: &[(&str, &[&str])] = &[
        (
            AgentToolId::INTAKE.wire_name(),
            &[
                "create_new",
                "resume_active",
                "supersede_active",
                "reject_if_active",
            ],
        ),
        (
            AgentToolId::UPDATE_SCOPE.wire_name(),
            &[
                UPDATE_SCOPE_KEEP_CURRENT_EXAMPLE_ID,
                "create_current_change_unit",
                "replace_current_change_unit",
            ],
        ),
        (
            AgentToolId::STATUS.wire_name(),
            &["summary_status", STATUS_READ_ONLY_EXAMPLE_ID, "full_status"],
        ),
        (
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
            &[GET_OPERATION_RESULT_FIRST_PAGE_EXAMPLE_ID],
        ),
        (
            AgentToolId::PREPARE_EVIDENCE_CAPTURE.wire_name(),
            &[
                PREPARE_EVIDENCE_CAPTURE_VERIFIED_COMMAND_EXAMPLE_ID,
                PREPARE_EVIDENCE_CAPTURE_VERIFIED_TOOL_EXAMPLE_ID,
            ],
        ),
        (
            AgentToolId::PREPARE_WRITE.wire_name(),
            &[PREPARE_WRITE_SIMPLE_EXAMPLE_ID],
        ),
        (
            AgentToolId::STAGE_ARTIFACT.wire_name(),
            &["stage_safe_text"],
        ),
        (
            AgentToolId::RECORD_RUN.wire_name(),
            &[
                RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
                "evidence_bearing_record_run",
            ],
        ),
        (
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            &[
                REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
                "resume_user_action",
            ],
        ),
        (
            AgentToolId::RECONCILE_CHANGES.wire_name(),
            &["reconcile_current_task"],
        ),
        (
            AgentToolId::CHECK_CLOSE.wire_name(),
            &[CHECK_CLOSE_MISSING_FINAL_ACCEPTANCE_EXAMPLE_ID],
        ),
        (
            AgentToolId::CLOSE_TASK.wire_name(),
            &["close_complete", "close_cancel", "close_supersede"],
        ),
    ];

    for (tool_name, expected_ids) in expected_branches {
        let tool = tool_definition(tool_name);
        let canonical = canonical_tool_examples(tool.id);
        assert_eq!(
            canonical
                .iter()
                .map(|example| example.id)
                .collect::<Vec<_>>(),
            *expected_ids,
            "{tool_name} should advertise exactly the supported example branches"
        );
        assert!(
            tool.description.len() <= 160,
            "{tool_name} description is {} bytes",
            tool.description.len()
        );
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
    let tool = tool_definition(AgentToolId::RECORD_RUN.wire_name());
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
        AgentToolId::RECORD_RUN.wire_name(),
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

    let decoded = decode_mcp_arguments_to_value(AgentToolId::RECORD_RUN.wire_name(), arguments)?;
    assert!(decoded["write_ticket_id"].is_null());
    Ok(())
}

#[test]
fn record_run_invalid_observed_changes_reports_expected_shape() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-invalid-record-run-observed-changes")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        AgentToolId::RECORD_RUN.wire_name(),
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
    )?;
    arguments["observed_changes"] = json!([]);

    let error = adapter
        .call_tool(AgentToolId::RECORD_RUN.wire_name(), arguments)
        .expect_err("invalid observed_changes should fail before Core");
    let response = structured_tool_error(AgentToolId::RECORD_RUN.wire_name(), &error);
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
        AgentToolId::RECORD_RUN.wire_name(),
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
    )?;
    arguments["kind"] = json!("test");

    let error = adapter
        .call_tool(AgentToolId::RECORD_RUN.wire_name(), arguments)
        .expect_err("invalid kind should fail before Core");
    let response = structured_tool_error(AgentToolId::RECORD_RUN.wire_name(), &error);
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
            AgentToolId::RECORD_RUN.wire_name(),
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
            .call_tool(AgentToolId::RECORD_RUN.wire_name(), arguments)
            .expect_err("unsupported artifact input source should fail before Core");
        let response = structured_tool_error(AgentToolId::RECORD_RUN.wire_name(), &error);
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
        AgentToolId::RECORD_RUN.wire_name(),
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
        .call_tool(AgentToolId::RECORD_RUN.wire_name(), arguments)
        .expect_err("invalid evidence observation should fail before Core");
    let response = structured_tool_error(AgentToolId::RECORD_RUN.wire_name(), &error);
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
    let arguments = canonical_example_value(
        AgentToolId::RECORD_RUN.wire_name(),
        "evidence_bearing_record_run",
    )?;
    crate::schema_validation::validate_mcp_tool_arguments(
        AgentToolId::RECORD_RUN.wire_name(),
        &arguments,
    )?;
    let decoded = decode_mcp_arguments_to_value(AgentToolId::RECORD_RUN.wire_name(), arguments)?;

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
    let mut arguments = canonical_example_value(
        AgentToolId::RECORD_RUN.wire_name(),
        "evidence_bearing_record_run",
    )?;
    arguments["evidence_updates"][0]["unsupported_ref"] = json!("not accepted");
    arguments["evidence_observations"][0]["unsupported_metadata"] = json!(true);

    let error = adapter
        .call_tool(AgentToolId::RECORD_RUN.wire_name(), arguments)
        .expect_err("unknown nested evidence fields should fail before Core");
    let response = structured_tool_error(AgentToolId::RECORD_RUN.wire_name(), &error);
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
        AgentToolId::RECORD_RUN.wire_name(),
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
    )?;
    arguments["unexpected"] = json!("not accepted");

    let error = adapter
        .call_tool(AgentToolId::RECORD_RUN.wire_name(), arguments)
        .expect_err("unknown root field should fail before Core");
    let response = structured_tool_error(AgentToolId::RECORD_RUN.wire_name(), &error);
    tool_error_issue(&response, "/unexpected", "MCP_ARGUMENT_UNKNOWN");
    Ok(())
}

#[test]
fn request_user_action_invalid_options_report_option_id_shape() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-invalid-judgment-options")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
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
        .call_tool(AgentToolId::REQUEST_USER_ACTION.wire_name(), arguments)
        .expect_err("invalid options should fail before Core");
    let response = structured_tool_error(AgentToolId::REQUEST_USER_ACTION.wire_name(), &error);
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
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
        REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
    )?;
    arguments["request"]["action"]["context"]["visible_risks"] = json!(["plain risk text"]);

    let error = adapter
        .call_tool(AgentToolId::REQUEST_USER_ACTION.wire_name(), arguments)
        .expect_err("invalid visible risk should fail before Core");
    let response = structured_tool_error(AgentToolId::REQUEST_USER_ACTION.wire_name(), &error);
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
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
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
            .call_tool(AgentToolId::REQUEST_USER_ACTION.wire_name(), arguments)
            .expect_err("invalid create/resume union shape should fail before Core");
        let response = structured_tool_error(AgentToolId::REQUEST_USER_ACTION.wire_name(), &error);
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
        .call_tool(AgentToolId::RECORD_RUN.wire_name(), arguments)
        .expect_err("independent argument failures should be rejected together");
    let response = structured_tool_error(AgentToolId::RECORD_RUN.wire_name(), &error);

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
        AgentToolId::RECORD_RUN.wire_name(),
        RECORD_RUN_ADVISOR_NO_PRODUCT_WRITE_EXAMPLE_ID,
    )?;
    arguments["close_assessment"] = json!({});

    let error = adapter
        .call_tool(AgentToolId::RECORD_RUN.wire_name(), arguments)
        .expect_err("empty close assessment should expose its nested missing fields");
    let response = structured_tool_error(AgentToolId::RECORD_RUN.wire_name(), &error);

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
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
        REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
    )?;
    arguments["request"]["expires_at"] = json!("not-a-timestamp");

    let error = adapter
        .call_tool(AgentToolId::REQUEST_USER_ACTION.wire_name(), arguments)
        .expect_err("invalid timestamp format should fail typed decoding");
    let response = structured_tool_error(AgentToolId::REQUEST_USER_ACTION.wire_name(), &error);

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
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
        REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
    )?;
    arguments["request"]["expires_at"] = json!("not-a-timestamp");

    let error = adapter
        .call_tool(AgentToolId::REQUEST_USER_ACTION.wire_name(), arguments)
        .expect_err("typed argument decoding should precede storage preconditions");
    let response = structured_tool_error(AgentToolId::REQUEST_USER_ACTION.wire_name(), &error);

    assert_eq!(response["code"], "MCP_INVALID_ARGUMENTS");
    tool_error_issue(&response, "", "MCP_ARGUMENT_DECODE_FAILED");
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn mcp_minimal_smoke_definition_uses_canonical_identity() {
    let tools = vec![CanonicalToolDefinition {
        id: AgentToolId::STATUS,
        title: None,
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
        annotations: CanonicalToolAnnotations {
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: true,
            open_world_hint: false,
        },
        metadata: None,
    }];

    assert_eq!(tool_names(&tools), vec![AgentToolId::STATUS.wire_name()]);
    assert_eq!(mcp_tool_naming_style(&tools), "dotted_namespace");
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
    assert!(report.contains("effective_tool_mode: workflow"));
    assert!(report.contains("tools_list_schema_validation: passed"));
    assert!(report.contains("tool_naming_style: dotted_namespace"));
    Ok(())
}

#[test]
fn mcp_check_reports_readwrite_effective_tool_mode() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-check-readwrite-mode")?;

    let report = preflight_report_for_fixture(&fixture, Some(fixture.project_id()))?;

    assert_report_line(&report, "registry_read: passed");
    assert_report_line(&report, "project_state_read: passed");
    assert_report_line(&report, "project_state_write: passed");
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
    let before_invocations = read_only_table_count(&fixture, "tool_invocations")?;

    let report = preflight_report_for_fixture(&fixture, Some(fixture.project_id()))?;

    assert_report_line(&report, "project_state_write: passed");
    assert_eq!(read_only_state_version(&fixture)?, before_version);
    assert_eq!(
        read_only_table_count(&fixture, "agent_sessions")?,
        before_sessions
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
    assert!(names.contains(&AgentToolId::STATUS.wire_name()));
    assert!(names.contains(&AgentToolId::GET_OPERATION_RESULT.wire_name()));
    assert!(names.contains(&AgentToolId::REQUEST_USER_ACTION.wire_name()));
    assert!(names.contains(&AgentToolId::CHECK_CLOSE.wire_name()));
    assert!(names.contains(&AgentToolId::LIST_PROJECTS.wire_name()));
    assert!(!names.contains(&AgentToolId::INTAKE.wire_name()));
    Ok(())
}

#[test]
fn mcp_launch_origin_classifies_verification_managed_manual_and_invalid() {
    fn classified(markers: &[(&str, String)], host_kind: Option<&str>) -> McpLaunchOrigin {
        classify_launch_origin(
            |name| {
                markers
                    .iter()
                    .find(|(marker, _)| *marker == name)
                    .map(|(_, value)| OsString::from(value))
            },
            "conn_alpha",
            host_kind,
        )
    }

    fn markers(values: &[(&'static str, &str)]) -> Vec<(&'static str, String)> {
        values
            .iter()
            .map(|(name, value)| (*name, (*value).to_owned()))
            .collect()
    }

    assert_eq!(McpLaunchOrigin::Unknown.as_str(), "unknown");
    assert_eq!(
        classify_launch_origin(
            |name| (name == "VOLICORD_MCP_VERIFICATION").then(|| OsString::from("1")),
            "conn_alpha",
            Some("codex"),
        ),
        McpLaunchOrigin::CliVerification
    );
    assert_eq!(
        classify_launch_origin(|_| None, "conn_alpha", Some("codex")),
        McpLaunchOrigin::ManualCli
    );
    assert_eq!(
        classify_launch_origin(
            |name| (name == "CODEX_THREAD_ID").then(|| OsString::from("ambient_thread")),
            "conn_alpha",
            Some("codex"),
        ),
        McpLaunchOrigin::ManualCli,
        "an ambient host-native marker is not managed launch correlation evidence"
    );
    let valid_codex = markers(&[
        ("VOLICORD_MCP_LAUNCH", "managed_host"),
        ("VOLICORD_MCP_HOST", "codex"),
        ("VOLICORD_MCP_CONNECTION_ID", "conn_alpha"),
    ]);
    assert_eq!(
        classified(&valid_codex, Some("codex")),
        McpLaunchOrigin::ManagedHost
    );
    let invalid_cases = vec![
        (
            "wrong launch",
            markers(&[
                ("VOLICORD_MCP_LAUNCH", "manual"),
                ("VOLICORD_MCP_HOST", "codex"),
                ("VOLICORD_MCP_CONNECTION_ID", "conn_alpha"),
                ("CODEX_THREAD_ID", "thread_alpha"),
            ]),
            Some("codex"),
        ),
        (
            "wrong host",
            markers(&[
                ("VOLICORD_MCP_LAUNCH", "managed_host"),
                ("VOLICORD_MCP_HOST", "unsupported"),
                ("VOLICORD_MCP_CONNECTION_ID", "conn_alpha"),
                ("CODEX_THREAD_ID", "thread_alpha"),
            ]),
            Some("codex"),
        ),
        (
            "wrong connection",
            markers(&[
                ("VOLICORD_MCP_LAUNCH", "managed_host"),
                ("VOLICORD_MCP_HOST", "codex"),
                ("VOLICORD_MCP_CONNECTION_ID", "conn_beta"),
                ("CODEX_THREAD_ID", "thread_alpha"),
            ]),
            Some("codex"),
        ),
    ];

    for (label, marker_set, host_kind) in invalid_cases {
        assert_eq!(
            classified(&marker_set, host_kind),
            McpLaunchOrigin::InvalidManagedMarker,
            "{label}"
        );
    }
    for ambient in ["", "thread alpha", &"a".repeat(257)] {
        let mut with_ambient = valid_codex.clone();
        with_ambient.push(("CODEX_THREAD_ID", ambient.to_owned()));
        assert_eq!(
            classified(&with_ambient, Some("codex")),
            McpLaunchOrigin::ManagedHost,
            "ambient CODEX_THREAD_ID is not a binding input"
        );
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_managed_marker_is_invalid_instead_of_manual() {
    use std::os::unix::ffi::OsStringExt;

    assert_eq!(
        classify_launch_origin(
            |name| {
                (name == "VOLICORD_MCP_LAUNCH").then(|| OsString::from_vec(vec![0xff, 0xfe]))
            },
            "conn_alpha",
            Some("codex"),
        ),
        McpLaunchOrigin::InvalidManagedMarker
    );
}

#[test]
fn managed_codex_launch_stays_effect_free_until_exact_call_binding() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-managed-binding")?;
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
            "CODEX_THREAD_ID" => Some(OsString::from("ambient_not_binding")),
            _ => None,
        },
    )?;

    assert!(output.is_empty());
    assert_eq!(read_only_table_count(&fixture, "agent_sessions")?, 0);
    assert!(read_diagnostic_session(fixture.runtime_home_path(), None)?.is_none());
    Ok(())
}

#[test]
fn managed_codex_tools_list_buffers_metrics_until_call_binding() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-managed-tools-list-binding")?;
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
            _ => None,
        },
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    assert!(responses[1]["result"]["tools"].is_array());
    assert_eq!(read_only_table_count(&fixture, "agent_sessions")?, 0);
    assert!(read_diagnostic_session(fixture.runtime_home_path(), None)?.is_none());
    Ok(())
}

#[test]
fn managed_stdio_records_authoritative_protocol_milestones_with_future_client_data(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-authoritative-runtime-milestones")?;
    let mut initialize = initialize_request(1, json!({}));
    initialize["params"]["clientInfo"]["name"] = json!("future-cooperative-client");
    initialize["params"]["clientInfo"]["version"] = json!("999.0-preview+custom");
    let input = Cursor::new(json_lines(&[
        initialize,
        initialized_notification(),
        initialized_notification(),
        request(2, "tools/list", json!({})),
        tools_call_with_codex_metadata(
            3,
            AgentToolId::LIST_PROJECTS.wire_name(),
            json!({}),
            CODEX_TEST_SESSION_ID,
            CODEX_TEST_THREAD_ID,
            CODEX_TEST_TURN_ID,
        ),
    ])?);
    let mut output = Vec::new();
    run_stdio_with_env_marker(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
        |name| match name {
            "VOLICORD_MCP_LAUNCH" => Some(OsString::from("managed_host")),
            "VOLICORD_MCP_HOST" => Some(OsString::from("codex")),
            "VOLICORD_MCP_CONNECTION_ID" => Some(OsString::from(fixture.connection_id())),
            _ => None,
        },
    )?;
    assert_eq!(stdio_responses(&output)?.len(), 3);
    let registry = open_registry_database_read_only(registry_db_path(fixture.runtime_home_path()))?;
    let runtime_session_id = registry.query_row(
        "SELECT runtime_session_id
           FROM mcp_runtime_sessions
          WHERE connection_internal_id = ?1 AND session_source = 'managed_host'
          ORDER BY process_started_at DESC, runtime_session_id DESC
          LIMIT 1",
        [fixture.connection_id()],
        |row| row.get::<_, String>(0),
    )?;
    let runtime = mcp_runtime_session(fixture.runtime_home_path(), &runtime_session_id)?
        .expect("authoritative runtime session");
    assert_eq!(
        runtime.attempted_client_name.as_deref(),
        Some("future-cooperative-client")
    );
    assert_eq!(
        runtime.attempted_client_version.as_deref(),
        Some("999.0-preview+custom")
    );
    assert_eq!(
        runtime.requested_protocol_version.as_deref(),
        Some("2025-11-25")
    );
    assert_eq!(
        runtime.selected_protocol_version.as_deref(),
        Some("2025-11-25")
    );
    assert_eq!(
        runtime.negotiated_protocol_version.as_deref(),
        Some("2025-11-25")
    );
    assert!(runtime.initialize_completed_at.is_some());
    assert!(runtime.initialized_notification_at.is_some());
    assert_eq!(runtime.required_tools_present, Some(true));
    assert_eq!(
        runtime.verification_tool_name.as_deref(),
        Some(AgentToolId::LIST_PROJECTS.wire_name())
    );
    assert!(runtime.verification_tool_observed_at.is_some());
    assert!(runtime.graceful_close_at.is_some());
    assert!(runtime.terminal_finding_id.is_none());
    Ok(())
}

#[test]
fn successful_non_designated_read_only_tools_do_not_record_round_trip_evidence(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-nondesignated-read-only-round-trip")?;
    let setup_adapter = adapter(&fixture)?;
    let committed = setup_adapter.call_tool(AgentToolId::INTAKE.wire_name(), intake_args(None))?;
    let task_id = committed.response_value["task_ref"]["record_id"]
        .as_str()
        .ok_or("intake task id")?
        .to_owned();
    let operation_result_ref = committed
        .operation_result_ref
        .ok_or("intake operation result ref")?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        request(2, "tools/list", json!({})),
        tools_call_with_codex_metadata(
            3,
            AgentToolId::STATUS.wire_name(),
            json!({ "detail": "workflow" }),
            CODEX_TEST_SESSION_ID,
            CODEX_TEST_THREAD_ID,
            "fixture_codex_turn_status",
        ),
        tools_call_with_codex_metadata(
            4,
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
            json!({ "operation_result_ref": operation_result_ref }),
            CODEX_TEST_SESSION_ID,
            CODEX_TEST_THREAD_ID,
            "fixture_codex_turn_result",
        ),
        tools_call_with_codex_metadata(
            5,
            AgentToolId::CHECK_CLOSE.wire_name(),
            json!({ "task_id": task_id }),
            CODEX_TEST_SESSION_ID,
            CODEX_TEST_THREAD_ID,
            "fixture_codex_turn_close",
        ),
    ])?);
    let mut output = Vec::new();
    run_stdio_with_env_marker(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
        |name| managed_codex_stdio_env(&fixture, name),
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 5);
    for response in &responses[2..] {
        assert_eq!(response["result"]["isError"], false, "{response}");
    }
    let runtime =
        latest_managed_runtime_session(fixture.runtime_home_path(), fixture.connection_id())?
            .ok_or("managed runtime")?;
    assert!(runtime.verification_tool_name.is_none());
    assert!(runtime.verification_tool_observed_at.is_none());
    Ok(())
}

#[test]
fn failed_designated_tool_call_does_not_record_round_trip_evidence() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-failed-designated-round-trip")?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        request(2, "tools/list", json!({})),
        tools_call_with_codex_metadata(
            3,
            AgentToolId::LIST_PROJECTS.wire_name(),
            json!({ "unexpected": true }),
            CODEX_TEST_SESSION_ID,
            CODEX_TEST_THREAD_ID,
            CODEX_TEST_TURN_ID,
        ),
    ])?);
    let mut output = Vec::new();
    run_stdio_with_env_marker(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
        |name| managed_codex_stdio_env(&fixture, name),
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[2]["result"]["isError"], true);
    let runtime =
        latest_managed_runtime_session(fixture.runtime_home_path(), fixture.connection_id())?
            .ok_or("managed runtime")?;
    assert!(runtime.verification_tool_name.is_none());
    assert!(runtime.verification_tool_observed_at.is_none());
    Ok(())
}

#[test]
fn failed_initialize_retains_attempted_client_and_requested_revision() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-failed-initialize-attempt")?;
    let mut initialize = initialize_request(1, json!("invalid-capabilities"));
    initialize["params"]["protocolVersion"] = json!("2099-01-01");
    initialize["params"]["clientInfo"]["name"] = json!("future-client");
    initialize["params"]["clientInfo"]["version"] = json!("2099.7");
    let mut output = Vec::new();
    run_stdio_with_env_marker(
        project_bound_adapter(&fixture)?,
        BufReader::new(Cursor::new(json_lines(&[initialize])?)),
        &mut output,
        |name| match name {
            "VOLICORD_MCP_LAUNCH" => Some(OsString::from("managed_host")),
            "VOLICORD_MCP_HOST" => Some(OsString::from("codex")),
            "VOLICORD_MCP_CONNECTION_ID" => Some(OsString::from(fixture.connection_id())),
            _ => None,
        },
    )?;
    assert!(stdio_responses(&output)?[0]["error"].is_object());
    let registry = open_registry_database_read_only(registry_db_path(fixture.runtime_home_path()))?;
    let runtime_session_id = registry.query_row(
        "SELECT runtime_session_id FROM mcp_runtime_sessions
          WHERE connection_internal_id = ?1
          ORDER BY process_started_at DESC, runtime_session_id DESC LIMIT 1",
        [fixture.connection_id()],
        |row| row.get::<_, String>(0),
    )?;
    let runtime = mcp_runtime_session(fixture.runtime_home_path(), &runtime_session_id)?
        .expect("failed initialize runtime session");
    assert_eq!(
        runtime.attempted_client_name.as_deref(),
        Some("future-client")
    );
    assert_eq!(runtime.attempted_client_version.as_deref(), Some("2099.7"));
    assert_eq!(
        runtime.requested_protocol_version.as_deref(),
        Some("2099-01-01")
    );
    assert!(runtime.selected_protocol_version.is_none());
    assert!(runtime.negotiated_protocol_version.is_none());
    assert!(runtime.initialize_completed_at.is_none());
    let terminal_id = runtime.terminal_finding_id.ok_or("terminal finding")?;
    let terminal = diagnostic_findings_by_ids(
        fixture.runtime_home_path(),
        &[volicord_types::DiagnosticFindingId::parse(terminal_id)?],
    )?
    .into_iter()
    .next()
    .ok_or("persisted terminal finding")?;
    assert_eq!(
        terminal.code().as_str(),
        "mcp.protocol.capability_shape_invalid"
    );
    assert_eq!(
        terminal.runtime_session_id().map(|value| value.as_str()),
        Some(runtime_session_id.as_str())
    );
    Ok(())
}

#[test]
fn managed_stdio_tool_call_records_bounded_metrics() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-managed-tool-call-metrics")?;
    let adapter = project_bound_adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        request(2, "tools/list", json!({})),
        initialized_notification(),
        tools_call(3, "volicord.status", json!({ "detail": "workflow" })),
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
            _ => None,
        },
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 3);
    let status = volicord_response_from_tool(&responses[2])?;
    assert_eq!(status["base"]["response_kind"], "result");
    let exact_tools_list_bytes = u64::try_from(serde_json::to_vec(&responses[1]["result"])?.len())?;
    let metrics =
        read_workflow_metric_aggregates(fixture.runtime_home_path(), fixture.project_id())?;
    let tools_list = workflow_metric_row(
        &metrics,
        WorkflowMetricKind::ToolsListSerializedBytes,
        None,
        Some(WorkflowMetricOutcome::Success),
    );
    assert_eq!(tools_list.sample_count, 1);
    assert_eq!(tools_list.host_kind.as_deref(), Some("codex"));
    assert_eq!(tools_list.value_total, exact_tools_list_bytes);
    let status_call = workflow_metric_row(
        &metrics,
        WorkflowMetricKind::McpMethodCall,
        Some(MethodName::Status),
        Some(WorkflowMetricOutcome::Success),
    );
    assert_eq!(status_call.sample_count, 1);
    assert_eq!(status_call.value_total, 1);
    Ok(())
}

#[test]
fn managed_codex_new_client_version_uses_protocol_and_call_binding() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-managed-new-codex-version")?;
    let observed_version = "0.145.0";
    let input = Cursor::new(json_lines(&[
        initialize_request_with_client_info(
            1,
            json!({}),
            CODEX_MANAGED_MCP_CLIENT_NAME,
            observed_version,
        ),
        initialized_notification(),
        tools_call_with_codex_metadata(
            2,
            AgentToolId::STATUS.wire_name(),
            json!({"detail":"workflow"}),
            CODEX_TEST_SESSION_ID,
            CODEX_TEST_THREAD_ID,
            CODEX_TEST_TURN_ID,
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio_with_env_marker(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
        |name| managed_codex_stdio_env(&fixture, name),
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[1]["result"]["isError"], false);
    Ok(())
}

#[test]
fn managed_codex_binding_allows_new_turn_and_rejects_session_or_thread_rebind(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-codex-binding-immutable")?;
    let native_session_id = "native.session.root";
    let native_thread_id = "native.thread.root";
    let capability_sentinel = "capability.must.not.persist";
    let initialize_sentinel = "initialize.payload.must.not.persist";
    let client_extension_sentinel = "client.extension.must.not.persist";
    let tool_payload_sentinel = "tool.payload.must.not.persist";
    let mut initialize = initialize_request(1, json!({}));
    initialize["params"]["capabilities"]["future_capability"] = json!(capability_sentinel);
    initialize["params"]["future_initialize_field"] = json!(initialize_sentinel);
    initialize["params"]["clientInfo"]["future_client_field"] = json!(client_extension_sentinel);
    let mut first_call = tools_call_with_codex_metadata(
        2,
        "volicord.status",
        json!({"detail":"workflow"}),
        native_session_id,
        native_thread_id,
        "turn.one",
    );
    first_call["params"]["_meta"]["future_tool_payload"] = json!(tool_payload_sentinel);
    let input = Cursor::new(json_lines(&[
        initialize,
        initialized_notification(),
        first_call,
        tools_call_with_codex_metadata(
            3,
            "volicord.status",
            json!({"detail":"workflow"}),
            native_session_id,
            native_thread_id,
            "turn.two",
        ),
        tools_call_with_codex_metadata(
            4,
            "volicord.status",
            json!({"detail":"workflow"}),
            native_session_id,
            "native.thread.other",
            "turn.three",
        ),
        tools_call_with_codex_metadata(
            5,
            "volicord.status",
            json!({"detail":"workflow"}),
            "native.session.other",
            native_thread_id,
            "turn.four",
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio_with_env_marker(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
        |name| managed_codex_stdio_env(&fixture, name),
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 5);
    assert!(responses[1]["result"].is_object());
    assert!(responses[2]["result"].is_object());
    assert_eq!(responses[3]["error"]["code"], -32602);
    assert_eq!(responses[4]["error"]["code"], -32602);
    let diagnostic = read_diagnostic_session(fixture.runtime_home_path(), None)?
        .expect("first exact call must bind the managed runtime diagnostic session");
    assert_eq!(diagnostic.totals.tool_call_count, 2);
    let persisted = serde_json::to_string(&diagnostic)?;
    for raw in [
        native_session_id,
        native_thread_id,
        "turn.one",
        "turn.two",
        "native.thread.other",
        "native.session.other",
        capability_sentinel,
        initialize_sentinel,
        client_extension_sentinel,
        tool_payload_sentinel,
    ] {
        assert!(
            !persisted.contains(raw),
            "raw host-native session correlation metadata leaked: {raw}"
        );
    }
    Ok(())
}

#[test]
fn invalid_codex_call_metadata_has_zero_durable_or_core_effect() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-invalid-marker-watch-skip")?;
    let adapter = project_bound_adapter(&fixture)?;
    let before_state_version = read_only_state_version(&fixture)?;
    let before_agent_sessions = read_only_table_count(&fixture, "agent_sessions")?;
    let before_tool_invocations = read_only_table_count(&fixture, "tool_invocations")?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        request(
            2,
            "tools/call",
            json!({
                "name": AgentToolId::LIST_PROJECTS.wire_name(),
                "arguments": {},
                "_meta": {
                    "threadId": "thread invalid marker",
                    "x-codex-turn-metadata": {
                        "session_id": CODEX_TEST_SESSION_ID,
                        "thread_id": CODEX_TEST_THREAD_ID,
                        "turn_id": CODEX_TEST_TURN_ID
                    }
                }
            }),
        ),
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
            "CODEX_THREAD_ID" => Some(OsString::from("ambient ignored value")),
            _ => None,
        },
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[1]["error"]["code"], -32602);
    assert!(!serde_json::to_string(&responses)?.contains("thread invalid marker"));
    assert_eq!(read_only_state_version(&fixture)?, before_state_version);
    assert_eq!(
        read_only_table_count(&fixture, "agent_sessions")?,
        before_agent_sessions
    );
    assert_eq!(
        read_only_table_count(&fixture, "tool_invocations")?,
        before_tool_invocations
    );
    assert!(read_diagnostic_session(fixture.runtime_home_path(), None)?.is_none());
    let runtime =
        latest_managed_runtime_session(fixture.runtime_home_path(), fixture.connection_id())?
            .ok_or("managed runtime for malformed host metadata")?;
    assert!(runtime.verification_tool_name.is_none());
    assert!(runtime.verification_tool_observed_at.is_none());
    let findings = diagnostic_occurrences_for_runtime_session(
        fixture.runtime_home_path(),
        &runtime.runtime_session_id,
    )?;
    assert!(findings
        .iter()
        .any(|finding| finding.data().code().as_str() == "host.codex.metadata_malformed"));
    let persisted_findings = serde_json::to_string(
        &findings
            .iter()
            .map(|finding| finding.to_diagnostic_finding())
            .collect::<Vec<_>>(),
    )?;
    assert!(!persisted_findings.contains("thread invalid marker"));
    assert!(!persisted_findings.contains(CODEX_TEST_SESSION_ID));
    assert!(!persisted_findings.contains(CODEX_TEST_THREAD_ID));
    Ok(())
}

#[test]
fn invalid_tool_shapes_do_not_bind_and_a_later_exact_codex_call_recovers(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-codex-prebinding-validation-order")?;
    let rejected_session_id = "native.session.rejected-before-binding";
    let accepted_session_id = "native.session.accepted-after-recovery";
    let mut non_object_arguments = tools_call_with_codex_metadata(
        3,
        "volicord.status",
        json!({}),
        rejected_session_id,
        "native.thread.rejected-before-binding",
        "turn.rejected.arguments",
    );
    non_object_arguments["params"]["arguments"] = json!([]);
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call_with_codex_metadata(
            2,
            "volicord.unknown",
            json!({}),
            rejected_session_id,
            "native.thread.rejected-before-binding",
            "turn.rejected.tool",
        ),
        non_object_arguments,
        request(
            4,
            "tools/call",
            json!({"name":"volicord.status","arguments":{}}),
        ),
        tools_call_with_codex_metadata(
            5,
            "volicord.status",
            json!({"detail":"workflow"}),
            accepted_session_id,
            "native.thread.accepted-after-recovery",
            "turn.accepted",
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio_with_env_marker(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
        |name| managed_codex_stdio_env(&fixture, name),
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 5);
    for response in &responses[1..4] {
        assert_eq!(response["error"]["code"], -32602);
    }
    assert!(responses[4]["result"].is_object());

    let rejected = current_project_agent_session_coordinates(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
        None,
        rejected_session_id,
    )?
    .session_id;
    assert!(
        agent_session(fixture.runtime_home_path(), fixture.project_id(), &rejected,)?.is_none()
    );

    let accepted = current_project_agent_session_coordinates(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
        None,
        accepted_session_id,
    )?
    .session_id;
    assert!(
        agent_session(fixture.runtime_home_path(), fixture.project_id(), &accepted,)?.is_some()
    );
    assert!(read_diagnostic_session(fixture.runtime_home_path(), None)?.is_some());
    let serialized = serde_json::to_string(&responses)?;
    assert!(!serialized.contains(rejected_session_id));
    assert!(!serialized.contains(accepted_session_id));
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
        json!(ProtocolRegistry::production()
            .preferred_server_profile()
            .revision()
            .as_str())
    );
    let names = tool_names_from_list_response(&responses[1]);
    assert_eq!(
        names,
        vec![
            AgentToolId::STATUS.wire_name(),
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            AgentToolId::CHECK_CLOSE.wire_name(),
            AgentToolId::LIST_PROJECTS.wire_name()
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

    assert!(names.contains(&AgentToolId::STATUS.wire_name()));
    assert!(names.contains(&AgentToolId::GET_OPERATION_RESULT.wire_name()));
    assert!(names.contains(&AgentToolId::LIST_PROJECTS.wire_name()));
    assert!(names.contains(&AgentToolId::CHECK_CLOSE.wire_name()));
    assert!(names.contains(&AgentToolId::REQUEST_USER_ACTION.wire_name()));
    assert!(!names.contains(&AgentToolId::INTAKE.wire_name()));
    assert!(!names.contains(&AgentToolId::RECORD_RUN.wire_name()));
    assert!(!names.contains(&AgentToolId::CLOSE_TASK.wire_name()));
    Ok(())
}

#[test]
fn mcp_readwrite_storage_exposes_workflow_tools() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readwrite-exposes-workflow")?;
    let adapter = adapter(&fixture)?;

    let expected = AgentToolId::ALL
        .iter()
        .map(|tool| tool.wire_name())
        .collect::<Vec<_>>();

    assert_eq!(tool_names(&adapter.tools()?), expected);
    Ok(())
}

#[cfg(unix)]
#[test]
fn mcp_status_succeeds_with_readonly_storage() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readonly-status")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

    let response = adapter.call_tool(
        AgentToolId::STATUS.wire_name(),
        json!({ "detail": "workflow" }),
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
    let before_invocations = read_only_table_count(&fixture, "tool_invocations")?;
    let adapter = adapter(&fixture)?;
    let before_sessions = read_only_table_count(&fixture, "agent_sessions")?;
    let _guard = make_project_state_readonly(&fixture)?;

    let response =
        adapter.call_tool(AgentToolId::STATUS.wire_name(), json!({ "detail": "full" }))?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(read_only_state_version(&fixture)?, before_version);
    assert_eq!(
        read_only_table_count(&fixture, "agent_sessions")?,
        before_sessions
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
    let committed = setup_adapter.call_tool(AgentToolId::INTAKE.wire_name(), intake_args(None))?;
    let operation_result_ref = committed
        .operation_result_ref
        .clone()
        .ok_or("committed agent-workflow result should expose a lookup ref")?;
    set_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;
    let read_only_adapter = adapter(&fixture)?;
    assert!(tool_names(&read_only_adapter.tools()?)
        .contains(&AgentToolId::GET_OPERATION_RESULT.wire_name()));
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
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
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
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
        AgentToolId::UPDATE_SCOPE.wire_name(),
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
        AgentToolId::UPDATE_SCOPE.wire_name(),
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
        let response = call_stdio(AgentToolId::GET_OPERATION_RESULT.wire_name(), arguments)?;
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
        AgentToolId::STATUS.wire_name(),
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
    let before_events = read_only_table_count(&fixture, "authority_events")?;
    let before_invocations = read_only_table_count(&fixture, "tool_invocations")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;

    let response = adapter.call_tool(AgentToolId::INTAKE.wire_name(), intake_args(None))?;

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
        read_only_table_count(&fixture, "authority_events")?,
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
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
        product_action_args(&fixture, &task_id, state_version),
    )?;
    assert!(!created.replayed);
    let exact_origin = created.response_value.clone();
    let exact_operation_result_ref = created.operation_result_ref.clone();
    let user_action_request_id = created.response_value["user_action_request_summary"]
        ["user_action_request_id"]
        .as_str()
        .ok_or("request-user-action result should identify its request")?
        .to_owned();
    let before_version = read_only_state_version(&fixture)?;
    let before_events = read_only_table_count(&fixture, "authority_events")?;
    let before_invocations = read_only_table_count(&fixture, "tool_invocations")?;
    let before_requests = read_only_table_count(&fixture, "user_action_requests")?;
    let _guard = make_project_state_readonly(&fixture)?;

    assert!(tool_names(&adapter.tools()?).contains(&AgentToolId::REQUEST_USER_ACTION.wire_name()));
    let rejected_create = adapter.call_tool(
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
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
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
        resume_user_action_args(&fixture, &user_action_request_id),
    )?;
    assert!(resumed.replayed);
    assert_eq!(resumed.response_value, exact_origin);
    assert_eq!(resumed.operation_result_ref, exact_operation_result_ref);
    assert_eq!(read_only_state_version(&fixture)?, before_version);
    assert_eq!(
        read_only_table_count(&fixture, "authority_events")?,
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
fn read_only_mode_rejects_agent_workflow_calls_before_core() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-read-only")?;
    set_mode(&fixture, CONNECTION_MODE_READ_ONLY)?;
    let adapter = adapter(&fixture)?;
    let before = fixture.counts()?;

    let cases = [
        (
            AgentToolId::INTAKE.wire_name(),
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
            AgentToolId::PREPARE_EVIDENCE_CAPTURE.wire_name(),
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
fn every_conformance_covered_revision_executes_the_transport_and_eof_matrix(
) -> Result<(), Box<dyn Error>> {
    for (index, profile) in conformance_profiles().enumerate() {
        let revision = profile.revision().as_str();
        let fixture = CoreFixture::new(&format!("mcp-negotiate-production-{index}"))?;
        let connection_adapter = adapter(&fixture)?;
        let capabilities = json!({"experimental": {"fixture": true}});
        let mut state = ConnectionState::default();

        let initialize = handle_json_rpc_message(
            &connection_adapter,
            &mut state,
            initialize_request_for_protocol(
                1,
                capabilities.clone(),
                "profile-test-client",
                "1.2.3",
                revision,
            ),
        )?
        .expect("initialize request should return a response");
        assert_eq!(initialize["result"]["protocolVersion"], revision);
        assert_eq!(initialize["result"]["capabilities"], json!({"tools": {}}));
        assert_eq!(
            initialize["result"].get("instructions").is_some(),
            profile.messages().initialize_result_instructions()
        );

        let selected = state
            .mcp_session
            .as_ref()
            .expect("valid initialize should select a session profile");
        assert_eq!(selected.requested_protocol_version, revision);
        assert_eq!(selected.selected_profile, profile);
        assert_eq!(selected.outcome, McpNegotiationOutcome::ExactMatch);
        assert_eq!(
            Value::Object(selected.client_capabilities.clone()),
            capabilities
        );
        assert_eq!(selected.attempted_client_name, "profile-test-client");
        assert_eq!(selected.attempted_client_version, "1.2.3");
        assert!(!selected.initialized_notification_completed);
        assert_eq!(state.phase, ConnectionPhase::AwaitingInitialized);

        let premature_tools = handle_json_rpc_message(
            &connection_adapter,
            &mut state,
            tools_call(2, AgentToolId::STATUS.wire_name(), json!({})),
        )?
        .expect("premature tools/call should return an error");
        assert_eq!(premature_tools["error"]["code"], -32600);

        assert!(handle_json_rpc_message(
            &connection_adapter,
            &mut state,
            initialized_notification()
        )?
        .is_none());
        assert_eq!(state.phase, ConnectionPhase::Ready);
        assert!(
            state
                .mcp_session
                .as_ref()
                .expect("selected session remains active")
                .initialized_notification_completed
        );

        let input = Cursor::new(json_lines(&[
            initialize_request_for_protocol(
                10,
                json!({}),
                "recording-test-client",
                "4.5.6",
                revision,
            ),
            initialized_notification(),
            request(11, "ping", json!({})),
            request(12, "tools/list", json!({})),
            tools_call(13, AgentToolId::LIST_PROJECTS.wire_name(), json!({})),
        ])?);
        let mut output = Vec::new();
        run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;
        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 4);
        assert_eq!(responses[0]["result"]["protocolVersion"], revision);
        assert_eq!(responses[1]["result"], json!({}));
        assert!(responses[2]["result"]["tools"].is_array());
        let list_projects = projected_authoritative_tool_result(&responses[3]["result"])?;
        assert!(list_projects["projects"]
            .as_array()
            .is_some_and(|projects| !projects.is_empty()));

        let registry =
            open_registry_database_read_only(registry_db_path(fixture.runtime_home_path()))?;
        let runtime_session_id = registry.query_row(
            "SELECT runtime_session_id
               FROM mcp_runtime_sessions
              WHERE connection_internal_id = ?1
              ORDER BY process_started_at DESC, runtime_session_id DESC
              LIMIT 1",
            [fixture.connection_id()],
            |row| row.get::<_, String>(0),
        )?;
        let recorded = mcp_runtime_session(fixture.runtime_home_path(), &runtime_session_id)?
            .expect("runtime session should be recorded");
        assert_eq!(
            recorded.negotiated_protocol_version.as_deref(),
            Some(revision)
        );
        assert!(recorded.initialized_notification_at.is_some());
        assert!(recorded.tools_list_observed_at.is_some());
        assert_eq!(recorded.required_tools_present, Some(true));
        assert!(recorded.verification_tool_name.is_none());
        assert!(recorded.verification_tool_observed_at.is_none());
        assert!(recorded.graceful_close_at.is_some());
        assert!(recorded.terminal_finding_id.is_none());
    }
    Ok(())
}

#[test]
fn every_conformance_covered_revision_projects_tools_results_and_request_failures(
) -> Result<(), Box<dyn Error>> {
    for (index, profile) in conformance_profiles().enumerate() {
        let fixture = CoreFixture::new(&format!("mcp-revision-call-shape-{index}"))?;
        let connection_adapter = adapter(&fixture)?;
        let mut state = ConnectionState::default();

        let initialize = handle_json_rpc_message(
            &connection_adapter,
            &mut state,
            initialize_request_for_protocol(
                1,
                json!({}),
                "revision-call-client",
                "1.0",
                profile.revision().as_str(),
            ),
        )?
        .expect("initialize response");
        assert_eq!(
            initialize["result"]["protocolVersion"],
            profile.revision().as_str()
        );
        assert!(handle_json_rpc_message(
            &connection_adapter,
            &mut state,
            initialized_notification()
        )?
        .is_none());

        let ping = handle_json_rpc_message(
            &connection_adapter,
            &mut state,
            request(2, "ping", json!({})),
        )?
        .expect("ping response");
        assert_eq!(ping["result"], json!({}));

        let tools_list = handle_json_rpc_message(
            &connection_adapter,
            &mut state,
            request(3, "tools/list", json!({})),
        )?
        .expect("tools/list response");
        let tools = tools_list["result"]["tools"]
            .as_array()
            .expect("tools/list result array");
        let tool_names = tools
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect::<BTreeSet<_>>();
        for required in AgentToolId::ALL.map(AgentToolId::wire_name) {
            assert!(
                tool_names.contains(required),
                "{} omitted required tool {required}",
                profile.revision()
            );
        }
        for tool in tools {
            let object = tool.as_object().expect("tool definition object");
            for required in ["name", "description", "inputSchema"] {
                assert!(object.contains_key(required), "missing {required}: {tool}");
            }
            assert_eq!(
                object.contains_key("annotations"),
                profile.tools().annotations(),
                "{} annotations projection",
                profile.revision()
            );
            assert_eq!(
                object.contains_key("outputSchema"),
                profile.tools().output_schema(),
                "{} outputSchema projection",
                profile.revision()
            );
            for absent in ["title", "_meta", "execution", "icons"] {
                assert!(!object.contains_key(absent), "fabricated {absent}: {tool}");
            }
        }

        let success = handle_json_rpc_message(
            &connection_adapter,
            &mut state,
            tools_call(4, AgentToolId::LIST_PROJECTS.wire_name(), json!({})),
        )?
        .expect("list-projects response");
        let success_body = projected_authoritative_tool_result(&success["result"])?;
        assert!(success_body["projects"].is_array());
        assert_eq!(
            success["result"].get("structuredContent").is_some(),
            profile.tools().structured_content(),
            "{} structured-content projection",
            profile.revision()
        );
        assert_eq!(
            success["result"].get("toolResult").is_some(),
            profile
                .schema()
                .tool_result_fields()
                .contains(&ToolResultField::ToolResult),
            "{} legacy toolResult projection",
            profile.revision()
        );

        let invalid_arguments = handle_json_rpc_message(
            &connection_adapter,
            &mut state,
            tools_call(
                5,
                AgentToolId::RECORD_RUN.wire_name(),
                json!({ "kind": "unsupported", "unexpected": true }),
            ),
        )?
        .expect("known-tool validation response");
        let invalid_body = projected_authoritative_tool_result(&invalid_arguments["result"])?;
        assert_eq!(invalid_body["code"], "MCP_INVALID_ARGUMENTS");
        assert_eq!(
            invalid_body["tool_name"],
            AgentToolId::RECORD_RUN.wire_name()
        );

        let invalid_params = handle_json_rpc_message(
            &connection_adapter,
            &mut state,
            request(6, "tools/list", json!([])),
        )?
        .expect("invalid tools/list params response");
        assert_eq!(invalid_params["error"]["code"], -32602);

        let invalid_request =
            handle_json_rpc_message(&connection_adapter, &mut state, json!(true))?
                .expect("invalid JSON-RPC request response");
        assert_eq!(invalid_request["error"]["code"], -32600);

        let unknown_method = handle_json_rpc_message(
            &connection_adapter,
            &mut state,
            request(7, "volicord/not-a-method", json!({})),
        )?
        .expect("unknown-method response");
        assert_eq!(unknown_method["error"]["code"], -32601);

        let execution_error = handle_json_rpc_message(
            &connection_adapter,
            &mut state,
            tools_call(
                8,
                AgentToolId::STATUS.wire_name(),
                json!({"project_selector": "project_that_is_not_registered"}),
            ),
        )?
        .expect("tool execution error response");
        let execution_body = projected_authoritative_tool_result(&execution_error["result"])?;
        assert_eq!(execution_body["code"], "MCP_ADAPTER_PRECONDITION_FAILED");
        assert_eq!(execution_body["tool_name"], AgentToolId::STATUS.wire_name());

        let unknown_tool = handle_json_rpc_message(
            &connection_adapter,
            &mut state,
            tools_call(9, "volicord.unknown", json!({})),
        )?
        .expect("unknown-tool response");
        assert_eq!(unknown_tool["error"]["code"], -32602);
        assert!(unknown_tool["result"].is_null());
    }
    Ok(())
}

#[test]
fn property_arbitrary_json_rpc_values_never_panic() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-arbitrary-json-rpc-property")?;
    let connection_adapter = adapter(&fixture)?;
    for seed in 0_u64..2_048 {
        let message = generated_json_rpc_value(seed, 0);
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut state = ConnectionState::default();
            let _ = handle_json_rpc_message(&connection_adapter, &mut state, message);
        }));
        assert!(result.is_ok(), "JSON-RPC input seed {seed} panicked");
    }
    Ok(())
}

#[test]
fn initialization_batches_are_rejected_without_selecting_a_profile_for_every_covered_revision(
) -> Result<(), Box<dyn Error>> {
    for (index, profile) in conformance_profiles().enumerate() {
        let fixture = CoreFixture::new(&format!("mcp-initialization-batch-{index}"))?;
        let connection_adapter = adapter(&fixture)?;
        let mut state = ConnectionState::default();
        let batch = json!([
            initialize_request_for_protocol(
                1,
                json!({}),
                "revision-batch-client",
                "1.0",
                profile.revision().as_str(),
            ),
            initialized_notification(),
            request(2, "tools/list", json!({})),
        ]);

        let response = handle_json_rpc_message(&connection_adapter, &mut state, batch.clone())?
            .expect("initialization batch should return one rejection");
        assert_eq!(response["error"]["code"], -32600);
        assert_eq!(state.phase, ConnectionPhase::AwaitingInitialize);
        assert!(state.mcp_session.is_none());

        let mut output = Vec::new();
        run_stdio(
            connection_adapter,
            BufReader::new(Cursor::new(json_lines(&[batch])?)),
            &mut output,
        )?;
        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0]["error"]["code"], -32600);

        let runtime_session_id = latest_runtime_session_id(&fixture)?;
        let runtime = mcp_runtime_session(fixture.runtime_home_path(), &runtime_session_id)?
            .expect("initialization-batch runtime session");
        assert!(runtime.attempted_client_name.is_none());
        assert!(runtime.attempted_client_version.is_none());
        assert!(runtime.requested_protocol_version.is_none());
        assert!(runtime.selected_protocol_version.is_none());
        assert!(runtime.negotiated_protocol_version.is_none());
        assert!(runtime.initialize_completed_at.is_none());
        assert!(runtime.initialized_notification_at.is_none());
        assert!(runtime.tools_list_observed_at.is_none());
        assert!(diagnostic_occurrences_for_runtime_session(
            fixture.runtime_home_path(),
            &runtime_session_id,
        )?
        .iter()
        .any(|finding| {
            finding.data().code().as_str() == "mcp.lifecycle.initialization_batch_forbidden"
        }));
    }
    Ok(())
}

#[test]
fn ready_2025_03_26_session_batches_operations_in_order_and_omits_notifications(
) -> Result<(), Box<dyn Error>> {
    let batching_revisions = conformance_profiles()
        .filter(|profile| profile.messages().json_rpc_batching() == JsonRpcBatching::Allowed)
        .map(|profile| profile.revision().as_str())
        .collect::<Vec<_>>();
    assert_eq!(batching_revisions, vec!["2025-03-26"]);

    let fixture = CoreFixture::new("mcp-ready-operation-batch")?;
    let operation_batch = json!([
        request(20, "ping", json!({})),
        notification(
            "notifications/progress",
            json!({"progressToken": "fixture", "progress": 1})
        ),
        request(21, "tools/list", json!({})),
    ]);
    let mut output = Vec::new();
    run_stdio(
        adapter(&fixture)?,
        BufReader::new(Cursor::new(json_lines(&[
            initialize_request_for_protocol(
                1,
                json!({}),
                "operation-batch-client",
                "1.0",
                "2025-03-26",
            ),
            initialized_notification(),
            operation_batch,
        ])?)),
        &mut output,
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-03-26");
    let batch_responses = responses[1]
        .as_array()
        .expect("ready operation batch response array");
    assert_eq!(
        batch_responses
            .iter()
            .map(|response| response["id"].as_u64().expect("response id"))
            .collect::<Vec<_>>(),
        vec![20, 21]
    );
    assert_eq!(batch_responses[0]["result"], json!({}));
    assert!(batch_responses[1]["result"]["tools"].is_array());

    let runtime_session_id = latest_runtime_session_id(&fixture)?;
    let runtime = mcp_runtime_session(fixture.runtime_home_path(), &runtime_session_id)?
        .expect("valid operation-batch runtime session");
    assert_eq!(
        runtime.selected_protocol_version.as_deref(),
        Some("2025-03-26")
    );
    assert_eq!(
        runtime.negotiated_protocol_version.as_deref(),
        Some("2025-03-26")
    );
    assert!(runtime.initialized_notification_at.is_some());
    assert!(runtime.tools_list_observed_at.is_some());
    assert!(runtime.graceful_close_at.is_some());
    assert!(diagnostic_occurrences_for_runtime_session(
        fixture.runtime_home_path(),
        &runtime_session_id,
    )?
    .is_empty());
    Ok(())
}

#[test]
fn non_batching_profiles_reject_ready_operation_batches_without_tool_observations(
) -> Result<(), Box<dyn Error>> {
    for (index, profile) in conformance_profiles()
        .filter(|profile| profile.messages().json_rpc_batching() == JsonRpcBatching::Disallowed)
        .enumerate()
    {
        let fixture = CoreFixture::new(&format!("mcp-non-batching-profile-{index}"))?;
        let revision = profile.revision().as_str();
        let mut output = Vec::new();
        run_stdio(
            adapter(&fixture)?,
            BufReader::new(Cursor::new(json_lines(&[
                initialize_request_for_protocol(
                    1,
                    json!({}),
                    "non-batching-client",
                    "1.0",
                    revision,
                ),
                initialized_notification(),
                json!([
                    request(2, "ping", json!({})),
                    request(3, "tools/list", json!({})),
                ]),
            ])?)),
            &mut output,
        )?;

        let responses = stdio_responses(&output)?;
        assert_eq!(responses.len(), 2, "{revision}");
        assert_eq!(responses[0]["result"]["protocolVersion"], revision);
        assert_eq!(responses[1]["error"]["code"], -32600);

        let runtime_session_id = latest_runtime_session_id(&fixture)?;
        let runtime = mcp_runtime_session(fixture.runtime_home_path(), &runtime_session_id)?
            .expect("non-batching profile runtime session");
        assert_eq!(runtime.selected_protocol_version.as_deref(), Some(revision));
        assert_eq!(
            runtime.negotiated_protocol_version.as_deref(),
            Some(revision)
        );
        assert!(runtime.initialized_notification_at.is_some());
        assert!(runtime.tools_list_observed_at.is_none());
        assert!(runtime.graceful_close_at.is_some());
        let findings = diagnostic_occurrences_for_runtime_session(
            fixture.runtime_home_path(),
            &runtime_session_id,
        )?;
        assert!(findings
            .iter()
            .any(|finding| finding.data().code().as_str() == "mcp.json_rpc.invalid_request"));
        assert!(!findings.iter().any(|finding| {
            finding.data().code().as_str() == "mcp.lifecycle.initialization_batch_forbidden"
        }));
    }
    Ok(())
}

#[test]
fn operation_batches_before_ready_do_not_advance_lifecycle() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-operation-batch-before-ready")?;
    let connection_adapter = adapter(&fixture)?;
    let mut state = ConnectionState::default();

    let rejected_before_initialize = handle_json_rpc_message(
        &connection_adapter,
        &mut state,
        json!([
            request(90, "ping", json!({})),
            request(91, "tools/list", json!({})),
        ]),
    )?
    .expect("pre-initialize operation batch rejection");
    assert_eq!(rejected_before_initialize["error"]["code"], -32600);
    assert_eq!(state.phase, ConnectionPhase::AwaitingInitialize);
    assert!(state.mcp_session.is_none());

    let initialize = handle_json_rpc_message(
        &connection_adapter,
        &mut state,
        initialize_request_for_protocol(
            1,
            json!({}),
            "pre-ready-batch-client",
            "1.0",
            "2025-03-26",
        ),
    )?
    .expect("standalone initialize response");
    assert_eq!(initialize["result"]["protocolVersion"], "2025-03-26");
    let selected_before = state.mcp_session.clone();

    let rejected = handle_json_rpc_message(
        &connection_adapter,
        &mut state,
        json!([
            request(2, "ping", json!({})),
            request(3, "tools/list", json!({})),
        ]),
    )?
    .expect("pre-ready operation batch rejection");
    assert_eq!(rejected["error"]["code"], -32600);
    assert_eq!(state.phase, ConnectionPhase::AwaitingInitialized);
    assert_eq!(state.mcp_session, selected_before);

    let rejected_initialized_batch = handle_json_rpc_message(
        &connection_adapter,
        &mut state,
        json!([initialized_notification()]),
    )?
    .expect("initialized notification batch rejection");
    assert_eq!(rejected_initialized_batch["error"]["code"], -32600);
    assert_eq!(state.phase, ConnectionPhase::AwaitingInitialized);
    assert_eq!(state.mcp_session, selected_before);

    assert!(
        handle_json_rpc_message(&connection_adapter, &mut state, initialized_notification(),)?
            .is_none()
    );
    assert_eq!(state.phase, ConnectionPhase::Ready);
    Ok(())
}

#[test]
fn ready_batch_with_initialize_is_rejected_before_a_sibling_operation() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-ready-batch-duplicate-initialize")?;
    let mut output = Vec::new();
    run_stdio(
        adapter(&fixture)?,
        BufReader::new(Cursor::new(json_lines(&[
            initialize_request_for_protocol(
                1,
                json!({}),
                "ready-batch-client",
                "1.0",
                "2025-03-26",
            ),
            initialized_notification(),
            json!([
                request(2, "tools/list", json!({})),
                initialize_request_for_protocol(
                    3,
                    json!({}),
                    "ready-batch-client",
                    "1.0",
                    "2025-03-26",
                ),
            ]),
            request(4, "ping", json!({})),
        ])?)),
        &mut output,
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(responses[1]["error"]["code"], -32600);
    assert_eq!(responses[2]["id"], 4);
    assert_eq!(responses[2]["result"], json!({}));

    let runtime_session_id = latest_runtime_session_id(&fixture)?;
    let runtime = mcp_runtime_session(fixture.runtime_home_path(), &runtime_session_id)?
        .expect("ready duplicate-initialize batch runtime session");
    assert_eq!(
        runtime.selected_protocol_version.as_deref(),
        Some("2025-03-26")
    );
    assert_eq!(
        runtime.negotiated_protocol_version.as_deref(),
        Some("2025-03-26")
    );
    assert!(runtime.tools_list_observed_at.is_none());
    let findings = diagnostic_occurrences_for_runtime_session(
        fixture.runtime_home_path(),
        &runtime_session_id,
    )?;
    assert!(findings.iter().any(|finding| {
        finding.data().code().as_str() == "mcp.lifecycle.initialization_batch_forbidden"
    }));
    assert!(!findings
        .iter()
        .any(|finding| finding.data().code().as_str() == "mcp.lifecycle.duplicate_initialize"));
    Ok(())
}

#[test]
fn empty_batch_remains_a_json_rpc_invalid_request() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-empty-batch")?;
    let connection_adapter = adapter(&fixture)?;
    let mut state = ConnectionState::default();

    let response = handle_json_rpc_message(&connection_adapter, &mut state, json!([]))?
        .expect("empty batch rejection");
    assert_eq!(response["error"]["code"], -32600);
    assert_eq!(
        response["error"]["data"],
        "JSON-RPC batch must not be empty"
    );
    assert_eq!(state.phase, ConnectionPhase::AwaitingInitialize);
    assert!(state.mcp_session.is_none());
    Ok(())
}

#[test]
fn codex_compatible_2025_06_18_initialize_succeeds() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-negotiate-codex-2025-06-18")?;
    let input = Cursor::new(json_lines(&[initialize_request_for_protocol(
        1,
        json!({}),
        CODEX_MANAGED_MCP_CLIENT_NAME,
        CODEX_TEST_CLIENT_VERSION,
        "2025-06-18",
    )])?);
    let mut output = Vec::new();

    run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-06-18");
    Ok(())
}

#[test]
fn unsupported_initialize_revision_receives_preferred_server_counter_offer(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-negotiate-counter-offer")?;
    let adapter = adapter(&fixture)?;
    let mut state = ConnectionState::default();
    let response = handle_json_rpc_message(
        &adapter,
        &mut state,
        initialize_request_for_protocol(1, json!({}), "counter-offer-client", "1.0", "2025-01-01"),
    )?
    .expect("well-formed unsupported revision should receive initialize result");
    let preferred = ProtocolRegistry::production().preferred_server_profile();

    assert_eq!(
        response["result"]["protocolVersion"],
        preferred.revision().as_str()
    );
    let selected = state.mcp_session.as_ref().expect("counter-offer session");
    assert_eq!(selected.requested_protocol_version, "2025-01-01");
    assert_eq!(selected.selected_profile, preferred);
    assert_eq!(selected.outcome, McpNegotiationOutcome::ServerCounterOffer);
    assert!(!selected.initialized_notification_completed);
    assert!(handle_json_rpc_message(&adapter, &mut state, initialized_notification())?.is_none());
    assert!(
        state
            .mcp_session
            .as_ref()
            .expect("counter-offer session remains active")
            .initialized_notification_completed
    );
    Ok(())
}

#[test]
fn malformed_initialize_fields_are_invalid_params_without_session_mutation(
) -> Result<(), Box<dyn Error>> {
    let valid_client = json!({"name": "valid-client", "version": "1.0"});
    let cases = [
        json!({"capabilities": {}, "clientInfo": valid_client}),
        json!({"protocolVersion": 1, "capabilities": {}, "clientInfo": valid_client}),
        json!({"protocolVersion": "2025-11-25", "clientInfo": valid_client}),
        json!({"protocolVersion": "2025-11-25", "capabilities": null, "clientInfo": valid_client}),
        json!({"protocolVersion": "2025-11-25", "capabilities": {}}),
        json!({"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": null}),
        json!({"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"version": "1.0"}}),
        json!({"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": 7, "version": "1.0"}}),
        json!({"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "valid-client"}}),
        json!({"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "valid-client", "version": false}}),
    ];

    for (index, params) in cases.into_iter().enumerate() {
        let fixture = CoreFixture::new(&format!("mcp-malformed-initialize-{index}"))?;
        let adapter = adapter(&fixture)?;
        let mut state = ConnectionState::default();
        let response =
            handle_json_rpc_message(&adapter, &mut state, request(1, "initialize", params))?
                .expect("malformed initialize should return an error");

        assert_eq!(response["error"]["code"], -32602, "case {index}");
        assert!(response["error"]["data"]
            .as_str()
            .is_some_and(|data| data.len() <= 512));
        assert_eq!(state.phase, ConnectionPhase::AwaitingInitialize);
        assert!(state.mcp_session.is_none());
    }
    Ok(())
}

#[test]
fn lifecycle_rejects_tools_before_initialize_and_a_second_initialize() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-negotiate-lifecycle-order")?;
    let adapter = adapter(&fixture)?;
    let mut state = ConnectionState::default();

    for message in [
        request(1, "tools/list", json!({})),
        tools_call(2, AgentToolId::STATUS.wire_name(), json!({})),
    ] {
        let response = handle_json_rpc_message(&adapter, &mut state, message)?
            .expect("pre-initialize request should return an error");
        assert_eq!(response["error"]["code"], -32600);
    }
    let initialized =
        handle_json_rpc_message(&adapter, &mut state, initialize_request(3, json!({})))?
            .expect("first initialize should return a result");
    assert!(initialized["result"].is_object());
    let repeated = handle_json_rpc_message(&adapter, &mut state, initialize_request(4, json!({})))?
        .expect("second initialize should return an error");
    assert_eq!(repeated["error"]["code"], -32600);
    Ok(())
}

#[test]
fn discover_generation_protocol_is_not_accepted_as_initialize() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-negotiate-discover-generation")?;
    let adapter = adapter(&fixture)?;
    let mut state = ConnectionState::default();
    let response = handle_json_rpc_message(
        &adapter,
        &mut state,
        initialize_request_for_protocol(1, json!({}), "future-client", "1.0", "2026-07-28"),
    )?
    .expect("generation mismatch should return an error");

    assert_eq!(response["error"]["code"], -32601);
    assert!(response["error"]["data"]
        .as_str()
        .is_some_and(|data| data.contains("does not use the initialize handshake")));
    assert_eq!(state.phase, ConnectionPhase::AwaitingInitialize);
    assert!(state.mcp_session.is_none());
    Ok(())
}

#[test]
fn initialize_client_info_enforces_utf8_byte_bounds_before_connection_state_mutation(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-initialize-client-info-bound")?;
    let before = fixture.counts()?;
    let at_limit_name = format!(" {} ", "n".repeat(254));
    let at_limit_version = "가".repeat(85) + "a";
    assert_eq!(at_limit_name.len(), 256);
    assert_eq!(at_limit_version.len(), 256);

    let mut output = Vec::new();
    run_stdio(
        adapter(&fixture)?,
        BufReader::new(Cursor::new(json_lines(&[
            initialize_request_with_client_info(1, json!({}), &at_limit_name, &at_limit_version),
        ])?)),
        &mut output,
    )?;
    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 1);
    assert!(responses[0]["result"].is_object());

    let invalid_values = [
        String::new(),
        " \t\u{2003}".to_owned(),
        "line\nbreak".to_owned(),
        "x".repeat(257),
        "가".repeat(86),
    ];
    for (index, invalid) in invalid_values.iter().enumerate() {
        for invalid_name in [true, false] {
            let (name, version) = if invalid_name {
                (invalid.as_str(), "valid-version")
            } else {
                ("valid-name", invalid.as_str())
            };
            let fallback_id = 100 + (index as u64 * 2) + u64::from(invalid_name);
            let input = json_lines(&[
                initialize_request_with_client_info(1, json!({}), name, version),
                initialize_request_with_client_info(
                    fallback_id,
                    json!({}),
                    "valid-name",
                    "valid-version",
                ),
            ])?;
            let mut output = Vec::new();
            run_stdio(
                adapter(&fixture)?,
                BufReader::new(Cursor::new(input)),
                &mut output,
            )?;
            let responses = stdio_responses(&output)?;
            assert_eq!(responses.len(), 2);
            assert_eq!(responses[0]["error"]["code"], -32602);
            assert!(responses[1]["result"].is_object());
            assert_eq!(responses[1]["id"], fallback_id);
        }
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
        json!(ProtocolRegistry::production()
            .preferred_server_profile()
            .revision()
            .as_str())
    );
    let names = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools should be an array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();
    let expected = AgentToolId::ALL
        .iter()
        .map(|tool| tool.wire_name())
        .collect::<Vec<_>>();
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
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
            AgentToolId::CHECK_CLOSE.wire_name(),
            AgentToolId::LIST_PROJECTS.wire_name()
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
            AgentToolId::RECORD_RUN.wire_name(),
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
    assert_eq!(error["tool_name"], AgentToolId::RECORD_RUN.wire_name());
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
        .call_tool(
            AgentToolId::RECORD_RUN.wire_name(),
            Value::Object(arguments),
        )
        .expect_err("pathological invalid arguments should be rejected");
    let result = tool_execution_error_result(AgentToolId::RECORD_RUN.wire_name(), &error);
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
        tools_call(2, AgentToolId::STATUS.wire_name(), json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let error = structured_error_result(&responses[1]["result"]);
    assert_eq!(error["code"], "MCP_ADAPTER_PRECONDITION_FAILED");
    assert_eq!(error["tool_name"], AgentToolId::STATUS.wire_name());
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
fn project_bound_stdio_rejects_a_guessed_repository_name_as_project_selector(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-stdio-guessed-project-selector")?;
    let before = fixture.counts()?;
    let adapter = project_bound_adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::STATUS.wire_name(),
            json!({
                "detail": "workflow",
                "project_selector": "product-repo"
            }),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let error = structured_error_result(&responses[1]["result"]);
    assert_eq!(error["code"], "MCP_ADAPTER_PRECONDITION_FAILED");
    assert_eq!(error["tool_name"], AgentToolId::STATUS.wire_name());
    let issue = tool_error_issue(
        &error,
        "/project_selector",
        "MCP_ADAPTER_PRECONDITION_FAILED",
    );
    let message = issue["message"].as_str().expect("routing issue message");
    assert!(message.contains("outside this MCP transport project allowlist"));
    assert!(!message.contains("HTTP serve"));
    assert!(message.contains(&format!("Use {}", AgentToolId::LIST_PROJECTS.wire_name())));
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
            tools_call(2, AgentToolId::INTAKE.wire_name(), arguments),
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
        AgentToolId::STAGE_ARTIFACT.wire_name(),
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
        AgentToolId::UPDATE_SCOPE.wire_name(),
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
        AgentToolId::PREPARE_EVIDENCE_CAPTURE.wire_name(),
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
        AgentToolId::PREPARE_EVIDENCE_CAPTURE.wire_name()
    );

    let record_fixture = CoreFixture::new("mcp-default-compact-record-run")?;
    let record_adapter = adapter(&record_fixture)?;
    let (record_task_id, _) = create_task(&record_adapter)?;
    let scope = record_adapter.call_tool(
        AgentToolId::UPDATE_SCOPE.wire_name(),
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
        AgentToolId::STAGE_ARTIFACT.wire_name(),
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
        AgentToolId::RECORD_RUN.wire_name(),
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
        AgentToolId::UPDATE_SCOPE.wire_name(),
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
        AgentToolId::PREPARE_WRITE.wire_name(),
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
        AgentToolId::RECONCILE_CHANGES.wire_name(),
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
            AgentToolId::CLOSE_TASK.wire_name(),
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
fn stdio_diagnostics_count_validation_retry_without_storing_request_content(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-diagnostics-validation-retry")?;
    let before = fixture.counts()?;
    let sensitive_sentinel = "diagnostic-request-secret-and-file-/private/example.txt";
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::STATUS.wire_name(),
            json!({"unexpected_private_value": sensitive_sentinel}),
        ),
        tools_call(3, AgentToolId::STATUS.wire_name(), json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio_with_env_marker(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
        |name| managed_codex_stdio_env(&fixture, name),
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[1]["result"]["isError"], true);
    assert_eq!(responses[2]["result"]["isError"], false);
    let diagnostics =
        read_diagnostic_session(fixture.runtime_home_path(), None)?.expect("diagnostics session");
    let status = diagnostics
        .tools
        .iter()
        .find(|tool| tool.tool_name == AgentToolId::STATUS.wire_name())
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
    let sensitive_tool_name = "token=abc123-private-tool-name";
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(2, sensitive_tool_name, json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio_with_env_marker(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
        |name| managed_codex_stdio_env(&fixture, name),
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses[1]["error"]["code"], -32602);
    assert!(
        read_diagnostic_session(fixture.runtime_home_path(), None)?.is_none(),
        "untrusted tool metadata must not bind or create a managed diagnostics session"
    );
    assert!(
        !diagnostics_db_path(fixture.runtime_home_path()).exists(),
        "rejected untrusted metadata must not create the diagnostics store"
    );
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
        request(2, "tools/list", json!({})),
        tools_call(3, AgentToolId::STATUS.wire_name(), json!({})),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 3);
    assert!(responses[1]["result"]["tools"].is_array());
    assert_eq!(responses[2]["result"]["isError"], false);
    let response = volicord_response_from_tool(&responses[2])?;
    assert_eq!(response["base"]["response_kind"], "result");
    assert_eq!(response["base"]["effect_kind"], "read_only");
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn corrupt_diagnostics_store_is_nonfatal_to_managed_codex_binding() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-managed-diagnostics-corrupt-nonfatal")?;
    fs::write(
        diagnostics_db_path(fixture.runtime_home_path()),
        b"not a sqlite diagnostics database",
    )?;
    let native_session_id = "native.session.corrupt-diagnostics";
    let native_thread_id = "native.thread.corrupt-diagnostics";
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call_with_codex_metadata(
            2,
            AgentToolId::STATUS.wire_name(),
            json!({}),
            native_session_id,
            native_thread_id,
            "turn.one",
        ),
        tools_call_with_codex_metadata(
            3,
            AgentToolId::STATUS.wire_name(),
            json!({}),
            native_session_id,
            native_thread_id,
            "turn.two",
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio_with_env_marker(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
        |name| managed_codex_stdio_env(&fixture, name),
    )?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 3);
    assert_eq!(
        responses[1]["result"]["isError"], false,
        "first response: {:?}",
        responses[1]
    );
    assert_eq!(
        responses[2]["result"]["isError"], false,
        "second response: {:?}",
        responses[2]
    );
    let serialized = serde_json::to_string(&responses)?;
    assert!(!serialized.contains(native_session_id));
    assert!(!serialized.contains(native_thread_id));
    Ok(())
}

#[test]
fn stdio_pending_user_action_returns_cli_inbox_recovery() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-cli-inbox-recovery")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
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

    run_stdio_with_env_marker(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
        |name| managed_codex_stdio_env(&fixture, name),
    )?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 2);
    assert_eq!(
        values[1]["result"]["structuredContent"]["operation_result_ref"]["source_method"],
        AgentToolId::REQUEST_USER_ACTION.wire_name()
    );
    let response = volicord_response_from_tool(&values[1])?;
    let workflow = &response["agent_workflow_result"];
    let summary = &workflow["user_action_request_summary"];
    assert_eq!(summary["status"], "pending");
    assert_eq!(summary["next_actor"], "user");
    assert!(summary["user_action_request_id"]
        .as_str()
        .is_some_and(|request_id| !request_id.is_empty()));
    assert!(workflow.get("inbox_item").is_none());
    assert!(workflow.get("user_action_request").is_none());
    let fallback = values[1]["result"]["content"][1]["text"]
        .as_str()
        .expect("fallback text");
    assert!(fallback.contains("pending UserAction requires the user"));
    assert!(fallback.contains("`volicord inbox`"));
    assert!(!fallback.contains("volicord inbox resolve"));
    assert!(!fallback.contains("request.operation=resume"));
    assert!(values[1]["result"].get("_meta").is_none());
    let diagnostics = read_diagnostic_session(fixture.runtime_home_path(), None)?
        .expect("CLI fallback should create bounded diagnostics");
    assert_eq!(diagnostics.fallback_counts["cli_inbox"], 1);
    Ok(())
}

#[test]
fn stdio_record_guard_uses_the_cli_inbox_without_projecting_the_private_form(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-record-guard-cli-inbox")?;
    install_record_guard(&fixture)?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
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

    run_stdio_with_env_marker(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
        |name| managed_codex_stdio_env(&fixture, name),
    )?;

    let values = stdio_responses(&output)?;
    assert_eq!(values.len(), 2);
    assert_eq!(
        values[1]["result"]["structuredContent"]["operation_result_ref"]["source_method"],
        AgentToolId::REQUEST_USER_ACTION.wire_name()
    );
    let response = volicord_response_from_tool(&values[1])?;
    let workflow = &response["agent_workflow_result"];
    assert_eq!(workflow["user_action_request_summary"]["status"], "pending");
    assert_eq!(
        workflow["user_action_request_summary"]["next_actor"],
        "user"
    );
    assert!(workflow.get("inbox_item").is_none());
    assert!(workflow.get("user_action_request").is_none());
    let fallback_texts = values[1]["result"]["content"]
        .as_array()
        .expect("tool content")
        .iter()
        .filter_map(|content| content["text"].as_str())
        .collect::<Vec<_>>();
    assert!(fallback_texts
        .iter()
        .any(|text| text.contains("pending UserAction requires the user")));
    assert!(fallback_texts
        .iter()
        .any(|text| text.contains("`volicord inbox`")));
    assert!(fallback_texts
        .iter()
        .all(|text| !text.contains("prompt_capture") && !text.contains("volicord inbox resolve")));
    assert!(values[1]["result"].get("_meta").is_none());
    let diagnostics = read_diagnostic_session(fixture.runtime_home_path(), None)?
        .expect("CLI fallback should create bounded diagnostics");
    assert_eq!(diagnostics.fallback_counts["cli_inbox"], 1);
    Ok(())
}

#[test]
fn request_user_action_agent_projection_is_only_the_exact_pending_user_summary(
) -> Result<(), Box<dyn Error>> {
    const QUESTION_MARKER: &str = "MODEL_VISIBLE_USER_ACTION_QUESTION_MUST_NOT_ESCAPE";
    const OPTION_MARKER: &str = "MODEL_VISIBLE_USER_ACTION_OPTION_MUST_NOT_ESCAPE";
    const CONTEXT_MARKER: &str = "MODEL_VISIBLE_USER_ACTION_CONTEXT_MUST_NOT_ESCAPE";

    let fixture = CoreFixture::new("mcp-agent-user-action-summary")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let mut arguments = product_action_args(&fixture, &task_id, state_version);
    arguments["request"]["action"]["question"] = json!(QUESTION_MARKER);
    arguments["request"]["action"]["context"]["summary"] = json!(CONTEXT_MARKER);
    arguments["request"]["action"]["options"][0]["label"] = json!(OPTION_MARKER);
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(2, AgentToolId::REQUEST_USER_ACTION.wire_name(), arguments),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;

    let values = stdio_responses(&output)?;
    let tool_result = &values[1]["result"];
    assert_eq!(tool_result["isError"], false);
    let model_visible = json!({
        "content": tool_result["content"].clone(),
        "structuredContent": tool_result["structuredContent"].clone()
    });
    let model_visible_text = serde_json::to_string(&model_visible)?;
    let mut violations = Vec::new();

    for marker in [QUESTION_MARKER, OPTION_MARKER, CONTEXT_MARKER] {
        if model_visible_text.contains(marker) {
            violations.push(format!("agent projection exposed private marker {marker}"));
        }
    }
    for forbidden_key in [
        "user_action_request",
        "user_action_request_ref",
        "inbox_item",
        "request_ref",
        "question",
        "options",
        "form",
        "preferred_capture_path",
        "command",
        "url",
        "token",
    ] {
        if !json_values_for_key(&model_visible, forbidden_key).is_empty() {
            violations.push(format!(
                "agent projection exposed forbidden field {forbidden_key}"
            ));
        }
    }

    let summaries = json_values_for_key(&model_visible, "user_action_request_summary");
    if summaries.is_empty() {
        violations.push("agent projection omitted user_action_request_summary".to_owned());
    }
    for summary in summaries {
        let Some(summary) = summary.as_object() else {
            violations.push("user_action_request_summary was not an object".to_owned());
            continue;
        };
        let actual_keys = summary.keys().map(String::as_str).collect::<BTreeSet<_>>();
        let expected_keys = ["next_actor", "status", "user_action_request_id"]
            .into_iter()
            .collect::<BTreeSet<_>>();
        if actual_keys != expected_keys {
            violations.push(format!(
                "user_action_request_summary keys were {actual_keys:?}, expected {expected_keys:?}"
            ));
        }
        if summary.get("status") != Some(&json!("pending")) {
            violations.push("user_action_request_summary.status was not pending".to_owned());
        }
        if summary.get("next_actor") != Some(&json!("user")) {
            violations.push("user_action_request_summary.next_actor was not user".to_owned());
        }
        if summary
            .get("user_action_request_id")
            .and_then(Value::as_str)
            .is_none_or(|request_id| request_id.is_empty())
        {
            violations.push(
                "user_action_request_summary.user_action_request_id was not a non-empty string"
                    .to_owned(),
            );
        }
    }

    assert!(
        violations.is_empty(),
        "unsafe request_user_action agent projection:\n{}",
        violations.join("\n")
    );
    Ok(())
}

#[test]
fn all_eight_user_action_kinds_preserve_the_cli_inbox_boundary() -> Result<(), Box<dyn Error>> {
    let cases = [
        McpUserActionLeakageCase::choice(
            "product_decision",
            &["close_complete"],
            McpUserActionCloseBasis::None,
            false,
        ),
        McpUserActionLeakageCase::choice(
            "technical_decision",
            &["close_complete"],
            McpUserActionCloseBasis::None,
            false,
        ),
        McpUserActionLeakageCase::choice(
            "scope_decision",
            &["scope_update"],
            McpUserActionCloseBasis::None,
            false,
        ),
        McpUserActionLeakageCase::choice(
            "sensitive_approval",
            &["prepare_write", "close_complete"],
            McpUserActionCloseBasis::None,
            true,
        ),
        McpUserActionLeakageCase::choice(
            "final_acceptance",
            &["close_complete"],
            McpUserActionCloseBasis::NoResidualRisks,
            false,
        ),
        McpUserActionLeakageCase::choice(
            "residual_risk_acceptance",
            &["close_complete"],
            McpUserActionCloseBasis::VisibleResidualRisk,
            false,
        ),
        McpUserActionLeakageCase::choice(
            "cancellation",
            &["close_cancel"],
            McpUserActionCloseBasis::None,
            false,
        ),
        McpUserActionLeakageCase::evidence_observation(),
    ];

    for case in cases {
        let fixture = CoreFixture::new(&format!("mcp-user-action-leakage-{}", case.name))?;
        let prepared = prepare_mcp_user_action_leakage_case(&fixture, case)?;
        let input = Cursor::new(json_lines(&[
            initialize_request(1, json!({})),
            initialized_notification(),
            tools_call(
                2,
                AgentToolId::REQUEST_USER_ACTION.wire_name(),
                prepared.arguments,
            ),
        ])?);
        let mut output = Vec::new();

        run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;

        let values = stdio_responses(&output)?;
        assert_eq!(values.len(), 2, "{}: unexpected MCP exchange", case.name);
        let tool_result = &values[1]["result"];
        assert_eq!(
            tool_result["isError"], false,
            "{}: {tool_result}",
            case.name
        );
        let response = volicord_response_from_tool(&values[1])?;
        let summary = &response["agent_workflow_result"]["user_action_request_summary"];
        let summary_keys = summary
            .as_object()
            .unwrap_or_else(|| panic!("{}: pending summary must be an object", case.name))
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            summary_keys,
            BTreeSet::from(["next_actor", "status", "user_action_request_id"]),
            "{}: pending summary must use the exact model-visible three-field shape",
            case.name
        );
        assert_eq!(summary["status"], "pending", "{}", case.name);
        assert_eq!(summary["next_actor"], "user", "{}", case.name);
        assert!(
            summary["user_action_request_id"]
                .as_str()
                .is_some_and(|request_id| !request_id.is_empty()),
            "{}: pending summary must identify the request",
            case.name
        );

        let model_visible = json!({
            "content": tool_result["content"].clone(),
            "structuredContent": tool_result["structuredContent"].clone(),
        });
        for forbidden_key in [
            "user_action_request",
            "user_action_request_ref",
            "request_ref",
            "inbox_item",
            "question",
            "options",
            "context",
            "context_summary",
            "form",
            "preferred_capture_path",
            "answer_path_availability",
            "user_channel_availability",
            "fallbacks",
            "command",
            "url",
            "token",
            "verification_code",
            "sensitive_action_scope",
        ] {
            assert!(
                json_values_for_key(&model_visible, forbidden_key).is_empty(),
                "{}: model-visible result exposed forbidden key {forbidden_key}",
                case.name
            );
        }
        let model_visible_text = serde_json::to_string(&model_visible)?;
        for forbidden_text in prepared.private_markers.iter().map(String::as_str).chain([
            "http://",
            "/consent?",
            "token=",
        ]) {
            assert!(
                !model_visible_text.contains(forbidden_text),
                "{}: model-visible result exposed forbidden text {forbidden_text:?}",
                case.name
            );
        }

        assert!(tool_result.get("_meta").is_none(), "{}", case.name);
        assert!(
            tool_result["content"]
                .as_array()
                .is_some_and(|content| content.iter().any(|item| item["text"]
                    .as_str()
                    .is_some_and(|text| text.contains("`volicord inbox`")))),
            "{}",
            case.name
        );

        let record = stored_action_record(&fixture, &prepared.task_id, &response)?;
        assert_eq!(
            serde_json::to_value(record.request.action_kind)?,
            json!(case.name),
            "{}: fixture must exercise the intended action kind",
            case.name
        );
        assert!(
            record.resolution.is_none(),
            "{}: handoff delivery must not resolve the action",
            case.name
        );
        assert_eq!(
            fixture.counts()?.user_action_resolutions,
            0,
            "{}: handoff delivery must create no resolution row",
            case.name
        );
    }
    Ok(())
}

#[test]
fn stdio_rejects_tampered_summaries_and_noncanonical_full_form_before_delivery(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-pending-form-fail-closed")?;
    let (_task_id, pending_response) = create_pending_product_action(&fixture)?;
    let before = user_action_side_effect_snapshot(&fixture)?;

    let mut mismatched_id = pending_response.clone();
    mismatched_id.response_value["user_action_request_summary"]["user_action_request_id"] =
        json!("uar_not_in_the_trusted_projection");
    let mut invalid_summary = pending_response.clone();
    invalid_summary.response_value["user_action_request_summary"]["next_actor"] = json!("agent");
    let mut noncanonical_full_form = pending_response.clone();
    noncanonical_full_form.response_value["inbox_item"] = json!({
        "form": {"question": "noncanonical model-visible form must not be trusted"}
    });

    for (case, response) in [
        ("mismatched_id", mismatched_id),
        ("invalid_summary", invalid_summary),
        ("noncanonical_full_form", noncanonical_full_form),
    ] {
        let error = crate::stdio::user_action_tool_output(&adapter(&fixture)?, response)
            .expect_err("untrusted public pending data must fail before delivery");
        assert!(matches!(
            error,
            McpAdapterError::Protocol(_) | McpAdapterError::Json(_)
        ));
        assert_eq!(
            user_action_side_effect_snapshot(&fixture)?,
            before,
            "{case} must not create a token, resolution, or project effect"
        );
    }
    Ok(())
}

#[test]
fn stdio_resume_replays_exact_origin_after_cli_inbox_resolution() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-user-action-cli-inbox-resume")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;

    let create_input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
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
    let user_action_request_id = exact_origin["user_action_request_summary"]
        ["user_action_request_id"]
        .as_str()
        .ok_or("created response should identify the user-action request")?
        .to_owned();
    let after_create = fixture.counts()?;

    let core = CoreService::new(fixture.runtime_home_path());
    let resolved = core.resolve_user_action(
        fixture.resolve_user_action_request(ResolveUserActionFixture {
            request_id: "req_cli_inbox_resolution",
            task_id: &task_id,
            user_action_request_id: &user_action_request_id,
            channel_submission_id: "submission_cli_inbox_resolution",
            resolution: volicord_types::UserActionResolutionInput::Choice {
                selected_option_id: volicord_types::UserActionOptionId::new("keep"),
                note: Some("This private user note must not enter the MCP projection.".to_owned())
                    .into(),
            },
        }),
        InvocationContext::new(
            ProjectId::new(fixture.project_id()),
            ActorSource::LocalUser,
            OperationCategory::UserOnly,
            volicord_types::VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
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
        test_agent_invocation(&fixture, OperationCategory::AgentWorkflow),
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
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            resume_user_action_args(&fixture, &user_action_request_id),
        )
        .expect_err("another Agent Connection must not resume the originating result");
    assert!(matches!(wrong_error, McpAdapterError::ToolExecution { .. }));
    assert_eq!(fixture.counts()?, before_resume);

    let resume_input = Cursor::new(json_lines(&[
        initialize_request(3, json!({})),
        initialized_notification(),
        tools_call(
            4,
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
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

fn adapter(fixture: &CoreFixture) -> Result<McpAdapter, Box<dyn Error>> {
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
    let guard = guard_health_record(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
    )?;
    let session = volicord_test_support::seed_test_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
        guard
            .guard_installation
            .as_ref()
            .map(|installation| installation.guard_installation_id.as_str()),
    )?;
    Ok(
        McpAdapter::new(fixture.runtime_home_path(), context).with_managed_agent_session_binding(
            ManagedAgentSessionBinding {
                runtime_session_id: session.runtime_session_id.as_str().to_owned(),
                host_session_id: session.host_session_id,
                host_thread_id: session.host_thread_id,
                host_turn_id: session.host_turn_id,
            },
        ),
    )
}

fn test_agent_invocation(
    fixture: &CoreFixture,
    operation_category: OperationCategory,
) -> InvocationContext {
    let guard = guard_health_record(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
    )
    .expect("guard authority fixture must load");
    let session = volicord_test_support::seed_test_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        fixture.connection_id(),
        guard
            .guard_installation
            .as_ref()
            .map(|installation| installation.guard_installation_id.as_str()),
    )
    .expect("managed Agent Session fixture must seed");
    let validated = CoreService::new(fixture.runtime_home_path())
        .validate_agent_session(
            AgentConnectionId::new(fixture.connection_id()),
            ProjectId::new(fixture.project_id()),
            session.runtime_session_id,
            session.project_session_id,
            operation_category,
        )
        .expect("managed Agent Session fixture must validate");
    InvocationContext::new(
        ProjectId::new(fixture.project_id()),
        ActorSource::agent_connection(fixture.connection_id()),
        operation_category,
        "",
    )
    .with_validated_agent_session(validated)
}

#[test]
fn project_tool_rejects_missing_managed_session_coordinates() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-managed-agent-session-required")?;
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
    let adapter = McpAdapter::new(fixture.runtime_home_path(), context);

    let error = adapter
        .call_tool(
            AgentToolId::STATUS.wire_name(),
            json!({"detail": "workflow"}),
        )
        .expect_err("project tools require current managed session coordinates");
    assert!(error.to_string().contains("agent_session_missing"));
    Ok(())
}

#[test]
fn invented_session_coordinates_do_not_authorize_or_insert_a_project_session(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-invented-session-not-authority")?;
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?;
    let adapter = McpAdapter::new(fixture.runtime_home_path(), context);
    let before_sessions = read_only_table_count(&fixture, "agent_sessions")?;

    let error = adapter
        .call_tool_for_session(
            AgentToolId::STATUS,
            json!({"detail": "workflow"}),
            Some(AgentSessionCoordinates {
                runtime_session_id: "mcp_invented_runtime",
                project_session_id: "agent_invented_session",
            }),
        )
        .expect_err("caller-invented coordinates must not establish session authority");

    assert!(error
        .to_string()
        .contains("agent_runtime_session_not_current"));
    assert_eq!(
        read_only_table_count(&fixture, "agent_sessions")?,
        before_sessions
    );
    Ok(())
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
    let context = McpConnectionContext::resolve(fixture.runtime_home_path(), connection_id)?;
    let session = volicord_test_support::seed_test_agent_session(
        fixture.runtime_home_path(),
        fixture.project_id(),
        connection_id,
        None,
    )?;
    Ok(
        McpAdapter::new(fixture.runtime_home_path(), context).with_managed_agent_session_binding(
            ManagedAgentSessionBinding {
                runtime_session_id: session.runtime_session_id.as_str().to_owned(),
                host_session_id: session.host_session_id,
                host_thread_id: session.host_thread_id,
                host_turn_id: session.host_turn_id,
            },
        ),
    )
}

fn project_bound_adapter(fixture: &CoreFixture) -> Result<McpAdapter, Box<dyn Error>> {
    let context =
        McpConnectionContext::resolve(fixture.runtime_home_path(), fixture.connection_id())?
            .with_project_allowlist(vec![ProjectId::new(fixture.project_id())]);
    Ok(McpAdapter::new(fixture.runtime_home_path(), context))
}

fn managed_codex_stdio_env(fixture: &CoreFixture, name: &str) -> Option<OsString> {
    match name {
        "VOLICORD_MCP_LAUNCH" => Some(OsString::from("managed_host")),
        "VOLICORD_MCP_HOST" => Some(OsString::from(HOST_KIND_CODEX)),
        "VOLICORD_MCP_CONNECTION_ID" => Some(OsString::from(fixture.connection_id())),
        _ => None,
    }
}

fn install_record_guard(fixture: &CoreFixture) -> Result<(), Box<dyn Error>> {
    let repo_root = fixture.product_repo_path();
    let guard_installation_id = "guard_installation_mcp_record";
    let policy_hash = "sha256:2222222222222222222222222222222222222222222222222222222222222222";
    upsert_guard_installation(
        fixture.runtime_home_path(),
        GuardInstallationUpsert {
            guard_installation_id: guard_installation_id.to_owned(),
            connection_internal_id: fixture.connection_id().to_owned(),
            project_id: fixture.project_id().to_owned(),
            manifest_json: volicord_test_support::test_guard_manifest_json(
                fixture.runtime_home_path(),
                &repo_root,
                fixture.project_id(),
                fixture.connection_id(),
                guard_installation_id,
                policy_hash,
            ),
        },
    )?;
    Ok(())
}

fn set_mode(fixture: &CoreFixture, mode: &str) -> Result<(), Box<dyn Error>> {
    volicord_test_support::transition_test_connection_mode(
        fixture.runtime_home_path(),
        &fixture.product_repo_path(),
        fixture.project_id(),
        fixture.connection_id(),
        mode,
    )?;
    Ok(())
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

fn user_action_side_effect_snapshot(
    fixture: &CoreFixture,
) -> Result<(volicord_store::core_pipeline::StorageEffectCounts, String), Box<dyn Error>> {
    let counts = fixture.counts()?;
    let project_updated_at =
        fixture
            .conn()?
            .query_row("SELECT updated_at FROM project_state", [], |row| row.get(0))?;
    Ok((counts, project_updated_at))
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
    initialize_request_with_client_info(
        id,
        capabilities,
        CODEX_MANAGED_MCP_CLIENT_NAME,
        CODEX_TEST_CLIENT_VERSION,
    )
}

fn initialize_request_with_client_info(
    id: u64,
    capabilities: Value,
    client_name: &str,
    client_version: &str,
) -> Value {
    initialize_request_for_protocol(
        id,
        capabilities,
        client_name,
        client_version,
        ProtocolRegistry::production()
            .preferred_server_profile()
            .revision()
            .as_str(),
    )
}

fn initialize_request_for_protocol(
    id: u64,
    capabilities: Value,
    client_name: &str,
    client_version: &str,
    protocol_version: &str,
) -> Value {
    request(
        id,
        "initialize",
        json!({
            "protocolVersion": protocol_version,
            "capabilities": capabilities,
            "clientInfo": {
                "name": client_name,
                "version": client_version
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
    tools_call_with_codex_metadata(
        id,
        name,
        arguments,
        CODEX_TEST_SESSION_ID,
        CODEX_TEST_THREAD_ID,
        CODEX_TEST_TURN_ID,
    )
}

fn tools_call_with_codex_metadata(
    id: u64,
    name: &str,
    arguments: Value,
    session_id: &str,
    thread_id: &str,
    turn_id: &str,
) -> Value {
    request(
        id,
        "tools/call",
        json!({
            "name": name,
            "arguments": arguments,
            "_meta": {
                "threadId": thread_id,
                "x-codex-turn-metadata": {
                    "session_id": session_id,
                    "thread_id": thread_id,
                    "turn_id": turn_id
                }
            }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpUserActionCloseBasis {
    None,
    NoResidualRisks,
    VisibleResidualRisk,
}

#[derive(Debug, Clone, Copy)]
enum McpUserActionLeakageCaseKind {
    Choice {
        required_for: &'static [&'static str],
        close_basis: McpUserActionCloseBasis,
        sensitive: bool,
    },
    EvidenceObservation,
}

#[derive(Debug, Clone, Copy)]
struct McpUserActionLeakageCase {
    name: &'static str,
    kind: McpUserActionLeakageCaseKind,
}

impl McpUserActionLeakageCase {
    const fn choice(
        name: &'static str,
        required_for: &'static [&'static str],
        close_basis: McpUserActionCloseBasis,
        sensitive: bool,
    ) -> Self {
        Self {
            name,
            kind: McpUserActionLeakageCaseKind::Choice {
                required_for,
                close_basis,
                sensitive,
            },
        }
    }

    const fn evidence_observation() -> Self {
        Self {
            name: "evidence_observation",
            kind: McpUserActionLeakageCaseKind::EvidenceObservation,
        }
    }
}

struct PreparedMcpUserActionLeakageCase {
    task_id: String,
    arguments: Value,
    private_markers: Vec<String>,
}

fn prepare_mcp_user_action_leakage_case(
    fixture: &CoreFixture,
    case: McpUserActionLeakageCase,
) -> Result<PreparedMcpUserActionLeakageCase, Box<dyn Error>> {
    let core = CoreService::new(fixture.runtime_home_path());
    let invocation = || test_agent_invocation(fixture, OperationCategory::AgentWorkflow);
    let intake = core.intake(
        fixture.intake_request(
            &format!("req_mcp_user_action_{}_task", case.name),
            &format!("idem_mcp_user_action_{}_task", case.name),
            false,
            Some(0),
        ),
        invocation(),
    )?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .ok_or("intake response should expose the Task")?
        .to_owned();
    let scope_request_id = format!("req_mcp_user_action_{}_scope", case.name);
    let scope_idempotency_key = format!("idem_mcp_user_action_{}_scope", case.name);
    let scope = core.update_scope(
        fixture.update_scope_request(UpdateScopeFixture {
            request_id: &scope_request_id,
            idempotency_key: &scope_idempotency_key,
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "Exercise the UserAction adapter boundary.",
        }),
        invocation(),
    )?;
    let change_unit_id = scope.response_value["change_unit_ref"]["record_id"]
        .as_str()
        .ok_or("scope response should expose the current Change Unit")?
        .to_owned();
    let criterion_id = scope.response_value["state"]["acceptance_criteria"][0]
        ["acceptance_criterion_id"]
        .as_str()
        .ok_or("scope response should expose the acceptance criterion")?
        .to_owned();
    let mut state_version = scope.response_value["base"]["state_version"]
        .as_u64()
        .ok_or("scope response should expose state_version")?;
    let mut registered_artifact_id = None;
    if let McpUserActionLeakageCaseKind::Choice { close_basis, .. } = case.kind {
        if close_basis != McpUserActionCloseBasis::None {
            let request_id = format!("req_mcp_user_action_{}_run", case.name);
            let idempotency_key = format!("idem_mcp_user_action_{}_run", case.name);
            let mut request = fixture.record_run_request(
                &request_id,
                &idempotency_key,
                false,
                Some(state_version),
                &task_id,
                &change_unit_id,
            );
            let residual_risks = if close_basis == McpUserActionCloseBasis::VisibleResidualRisk {
                vec![ResidualRiskInput {
                    summary: "A visible fixture risk remains.".to_owned(),
                    consequence: "The user must decide whether this fixture risk is acceptable."
                        .to_owned(),
                    acceptance_required: true,
                    source_refs: Vec::new(),
                }]
            } else {
                Vec::new()
            };
            request.close_assessment = Some(CloseAssessmentInput {
                result_summary: "Current close evidence is available.".to_owned(),
                result_refs: Vec::new(),
                residual_risks,
                sensitive_categories: Vec::new(),
                recovery_constraints: Vec::new(),
            })
            .into();
            let recorded = core.record_run(request, invocation())?;
            state_version = recorded.response_value["base"]["state_version"]
                .as_u64()
                .ok_or("record_run response should expose state_version")?;
        }
    }
    if matches!(case.kind, McpUserActionLeakageCaseKind::EvidenceObservation) {
        let staged_request_id = format!("req_mcp_user_action_{}_stage", case.name);
        let staged_idempotency_key = format!("idem_mcp_user_action_{}_stage", case.name);
        let staged = core.stage_artifact(
            fixture.stage_artifact_request(
                &staged_request_id,
                Some(&staged_idempotency_key),
                false,
                Some(state_version),
                &task_id,
            ),
            invocation(),
        )?;
        let handle: StagedArtifactHandle =
            serde_json::from_value(staged.response_value["staged_artifact_handle"].clone())?;
        let request_id = format!("req_mcp_user_action_{}_run", case.name);
        let idempotency_key = format!("idem_mcp_user_action_{}_run", case.name);
        let mut request = fixture.record_run_request(
            &request_id,
            &idempotency_key,
            false,
            Some(state_version),
            &task_id,
            &change_unit_id,
        );
        request.artifact_inputs = vec![artifact_input_for_handle(
            "artifact_input_mcp_user_action_evidence_observation",
            handle,
            Some("user_action_candidate"),
            None,
        )];
        let recorded = core.record_run(request, invocation())?;
        state_version = recorded.response_value["base"]["state_version"]
            .as_u64()
            .ok_or("record_run response should expose state_version")?;
        registered_artifact_id = recorded.response_value["registered_artifacts"][0]["artifact_id"]
            .as_str()
            .map(str::to_owned);
    }

    let question_marker = format!("PRIVATE_{}_QUESTION_MUST_NOT_ESCAPE", case.name);
    let context_marker = format!("PRIVATE_{}_CONTEXT_MUST_NOT_ESCAPE", case.name);
    let option_marker = format!("PRIVATE_{}_OPTION_MUST_NOT_ESCAPE", case.name);
    let mut arguments = match case.kind {
        McpUserActionLeakageCaseKind::Choice {
            required_for,
            sensitive,
            ..
        } => {
            let options = if matches!(case.name, "product_decision" | "technical_decision") {
                json!([
                    {
                        "option_id": "keep",
                        "label": option_marker,
                        "description": "Keep the focused fixture behavior.",
                        "consequence": "Only this fixture action is resolved.",
                        "is_default": true
                    },
                    {
                        "option_id": "change",
                        "label": "Change the focused fixture behavior",
                        "description": "Change only the focused fixture behavior.",
                        "consequence": "Only this fixture action is resolved differently.",
                        "is_default": false
                    }
                ])
            } else {
                Value::Null
            };
            let mut arguments = action_args(
                fixture,
                &task_id,
                state_version,
                case.name,
                options,
                json!(required_for),
            );
            arguments["request"]["change_unit_id"] = json!(change_unit_id);
            arguments["request"]["action"]["question"] = json!(question_marker);
            arguments["request"]["action"]["context"]["summary"] = json!(context_marker);
            if sensitive {
                arguments["request"]["action"]["sensitive_action_scope"] = json!({
                    "action_kind": "mcp_user_action_leakage_fixture",
                    "description": "Authorize only the named fixture-sensitive step.",
                    "intended_paths": ["src/fixture.rs"],
                    "sensitive_categories": ["network"],
                    "command_or_tool_summary": "Run one local fixture command.",
                    "network_or_host_summary": "No remote host is authorized.",
                    "secret_or_credential_summary": null,
                    "capability_claim": "This fixture approval is not a write ticket.",
                    "expires_at": null
                });
            }
            arguments
        }
        McpUserActionLeakageCaseKind::EvidenceObservation => {
            let artifact_id = registered_artifact_id
                .ok_or("evidence-observation setup must register an artifact")?;
            let mut arguments = evidence_observation_action_args(
                &task_id,
                &change_unit_id,
                vec![json!({
                    "target_kind": "acceptance_criterion",
                    "acceptance_criterion_id": criterion_id
                })],
                vec![artifact_id],
            );
            arguments["detail"] = json!("full");
            arguments["request"]["action"]["question"] = json!(question_marker);
            arguments["request"]["action"]["context_summary"] = json!(context_marker);
            arguments
        }
    };
    arguments["project_selector"] = json!(fixture.project_id());

    Ok(PreparedMcpUserActionLeakageCase {
        task_id,
        arguments,
        private_markers: vec![question_marker, context_marker, option_marker],
    })
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

fn json_lines(messages: &[Value]) -> Result<Vec<u8>, serde_json::Error> {
    let mut output = Vec::new();
    for message in messages {
        serde_json::to_writer(&mut output, message)?;
        output.push(b'\n');
    }
    Ok(output)
}

fn generated_json_rpc_value(seed: u64, depth: usize) -> Value {
    if depth >= 4 {
        return match seed % 5 {
            0 => Value::Null,
            1 => Value::Bool(seed & 1 == 0),
            2 => json!(seed as i64 - 1_024),
            3 => json!(format!("value-{seed}")),
            _ => json!([seed, seed.wrapping_mul(17)]),
        };
    }

    match seed % 9 {
        0 => Value::Null,
        1 => Value::Bool(seed & 1 == 0),
        2 => json!(seed as i64 - 1_024),
        3 => json!(format!("json-rpc-{seed}")),
        4 => Value::Array(
            (0..(seed as usize % 4))
                .map(|index| {
                    generated_json_rpc_value(
                        seed.wrapping_mul(31).wrapping_add(index as u64),
                        depth + 1,
                    )
                })
                .collect(),
        ),
        5 => json!({
            "jsonrpc": if seed & 1 == 0 { "2.0" } else { "1.0" },
            "id": generated_json_rpc_value(seed.wrapping_add(1), depth + 1),
            "method": generated_json_rpc_value(seed.wrapping_add(2), depth + 1),
            "params": generated_json_rpc_value(seed.wrapping_add(3), depth + 1),
        }),
        6 => json!({
            "jsonrpc": "2.0",
            "id": seed,
            "method": "initialize",
            "params": generated_json_rpc_value(seed.wrapping_mul(7), depth + 1),
        }),
        7 => json!({
            "jsonrpc": "2.0",
            "id": seed,
            "method": "tools/call",
            "params": {
                "name": generated_json_rpc_value(seed.wrapping_add(5), depth + 1),
                "arguments": generated_json_rpc_value(seed.wrapping_add(6), depth + 1),
            },
        }),
        _ => json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": generated_json_rpc_value(seed.wrapping_add(9), depth + 1),
        }),
    }
}

fn projected_authoritative_tool_result(result: &Value) -> Result<Value, Box<dyn Error>> {
    if let Some(structured) = result.get("structuredContent") {
        return Ok(structured.clone());
    }
    if let Some(tool_result) = result.get("toolResult") {
        return Ok(tool_result.clone());
    }
    let text = result
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .ok_or("content-only tools/call result should carry authoritative JSON text")?;
    Ok(serde_json::from_str(text)?)
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

fn json_values_for_key<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    fn collect<'a>(value: &'a Value, key: &str, values: &mut Vec<&'a Value>) {
        match value {
            Value::Object(object) => {
                if let Some(value) = object.get(key) {
                    values.push(value);
                }
                for value in object.values() {
                    collect(value, key, values);
                }
            }
            Value::Array(array) => {
                for value in array {
                    collect(value, key, values);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }

    let mut values = Vec::new();
    collect(value, key, &mut values);
    values
}

fn stored_action_record(
    fixture: &CoreFixture,
    task_id: &str,
    response: &Value,
) -> Result<volicord_store::core_pipeline::EffectiveUserActionRecord, Box<dyn Error>> {
    let request_id = response
        .pointer("/agent_workflow_result/user_action_request_summary/user_action_request_id")
        .or_else(|| response.pointer("/user_action_request_summary/user_action_request_id"))
        .and_then(Value::as_str)
        .ok_or("response should include user_action_request_summary.user_action_request_id")?;
    let store = CoreProjectStore::open(
        fixture.runtime_home_path(),
        &ProjectId::new(fixture.project_id()),
    )?;
    let now = store.current_timestamp()?;
    let now = volicord_types::UtcTimestamp::parse(&now)?;
    let record = store
        .user_action_records_for_task(&volicord_types::TaskId::new(task_id), &now)?
        .into_iter()
        .find(|record| record.request.user_action_request_id == request_id)
        .ok_or("stored user-action record should exist")?;
    Ok(record)
}

fn tool_names(tools: &[CanonicalToolDefinition]) -> Vec<&'static str> {
    tools
        .iter()
        .map(|tool| tool.id.wire_name())
        .collect::<Vec<_>>()
}

fn tool_definition(tool_name: &str) -> CanonicalToolDefinition {
    mcp_tools_for_mode_and_storage(
        AgentConnectionMode::Workflow,
        McpStorageCapability::ReadWrite,
    )
    .into_iter()
    .find(|tool| tool.id.wire_name() == tool_name)
    .unwrap_or_else(|| panic!("missing tool definition for {tool_name}"))
}

fn canonical_example(
    tool_name: &str,
    example_id: &str,
) -> &'static crate::tool_registry::McpToolExample {
    canonical_tool_examples(
        AgentToolId::from_wire_name(tool_name)
            .unwrap_or_else(|_| panic!("unknown canonical tool {tool_name}")),
    )
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
    match AgentToolId::from_wire_name(tool_name)
        .unwrap_or_else(|_| panic!("unsupported MCP tool example decoder: {tool_name}"))
    {
        AgentToolId::INTAKE => {
            serde_json::to_value(serde_json::from_value::<McpIntakeArguments>(value)?)
        }
        AgentToolId::UPDATE_SCOPE => {
            serde_json::to_value(serde_json::from_value::<McpUpdateScopeArguments>(value)?)
        }
        AgentToolId::STATUS => {
            serde_json::to_value(serde_json::from_value::<McpStatusArguments>(value)?)
        }
        AgentToolId::GET_OPERATION_RESULT => serde_json::to_value(serde_json::from_value::<
            McpGetOperationResultArguments,
        >(value)?),
        AgentToolId::PREPARE_EVIDENCE_CAPTURE => serde_json::to_value(serde_json::from_value::<
            McpPrepareEvidenceCaptureArguments,
        >(value)?),
        AgentToolId::PREPARE_WRITE => {
            serde_json::to_value(serde_json::from_value::<McpPrepareWriteArguments>(value)?)
        }
        AgentToolId::STAGE_ARTIFACT => {
            serde_json::to_value(serde_json::from_value::<McpStageArtifactArguments>(value)?)
        }
        AgentToolId::RECORD_RUN => {
            serde_json::to_value(serde_json::from_value::<McpRecordRunArguments>(value)?)
        }
        AgentToolId::REQUEST_USER_ACTION => serde_json::to_value(serde_json::from_value::<
            McpRequestUserActionArguments,
        >(value)?),
        AgentToolId::RECONCILE_CHANGES => serde_json::to_value(serde_json::from_value::<
            McpReconcileChangesArguments,
        >(value)?),
        AgentToolId::CHECK_CLOSE => {
            serde_json::to_value(serde_json::from_value::<McpCheckCloseArguments>(value)?)
        }
        AgentToolId::CLOSE_TASK => {
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

fn assert_compatible_tool_definitions(tools: &[CanonicalToolDefinition]) {
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

fn json_member_exists(value: &Value, member: &str) -> bool {
    match value {
        Value::Object(object) => {
            object.contains_key(member)
                || object
                    .values()
                    .any(|child| json_member_exists(child, member))
        }
        Value::Array(items) => items.iter().any(|child| json_member_exists(child, member)),
        _ => false,
    }
}

fn workflow_metric_row(
    rows: &[WorkflowMetricAggregateRow],
    metric_kind: WorkflowMetricKind,
    method_name: Option<MethodName>,
    outcome: Option<WorkflowMetricOutcome>,
) -> &WorkflowMetricAggregateRow {
    rows.iter()
        .find(|row| {
            row.metric_kind == metric_kind.as_str()
                && row.method_name.as_deref() == method_name.map(MethodName::as_str)
                && row.outcome.as_deref() == outcome.map(WorkflowMetricOutcome::as_str)
        })
        .unwrap_or_else(|| {
            panic!(
                "missing workflow metric row kind={} method={:?} outcome={:?}; rows={rows:?}",
                metric_kind.as_str(),
                method_name.map(MethodName::as_str),
                outcome.map(WorkflowMetricOutcome::as_str),
            )
        })
}

fn assert_local_schema_refs_resolve(schema: &Value, tool_name: &str) {
    let definitions = schema.get("definitions").and_then(Value::as_object);
    assert_schema_value_refs_resolve(schema, definitions, tool_name);
}

fn assert_schema_value_refs_resolve(
    value: &Value,
    definitions: Option<&Map<String, Value>>,
    tool_name: &str,
) {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                let name = reference.strip_prefix("#/definitions/").unwrap_or_else(|| {
                    panic!("{tool_name} has a non-local runtime ref {reference}")
                });
                assert!(
                    definitions.is_some_and(|definitions| definitions.contains_key(name)),
                    "{tool_name} has an unresolved runtime ref {reference}"
                );
            }
            for child in object.values() {
                assert_schema_value_refs_resolve(child, definitions, tool_name);
            }
        }
        Value::Array(items) => {
            for child in items {
                assert_schema_value_refs_resolve(child, definitions, tool_name);
            }
        }
        _ => {}
    }
}

fn strip_schema_presentation_for_test(value: &mut Value) {
    compact_runtime_schema(value);
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

fn latest_runtime_session_id(fixture: &CoreFixture) -> Result<String, Box<dyn Error>> {
    let registry = open_registry_database_read_only(registry_db_path(fixture.runtime_home_path()))?;
    Ok(registry.query_row(
        "SELECT runtime_session_id
           FROM mcp_runtime_sessions
          WHERE connection_internal_id = ?1
          ORDER BY process_started_at DESC, runtime_session_id DESC
          LIMIT 1",
        [fixture.connection_id()],
        |row| row.get(0),
    )?)
}

#[test]
fn canonical_agent_tool_names_are_unique() {
    let unique = AgentToolId::ALL
        .iter()
        .map(|tool| tool.wire_name())
        .collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), AgentToolId::ALL.len());
}
