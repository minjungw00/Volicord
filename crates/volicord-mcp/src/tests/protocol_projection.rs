use super::*;

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
        Some(AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name())
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
            AgentToolId::LIST_PROJECTS.wire_name(),
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
            AgentToolId::GUARD_PROBE.wire_name(),
            AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
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
fn runtime_tools_list_is_bounded_and_semantically_descriptor_owned() {
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
                assert_eq!(runtime_tool.output_schema["type"], "object");
                assert_eq!(documentation_tool.output_schema["type"], "object");
                assert_eq!(
                    runtime_tool.output_schema["required"],
                    json!(["result_type"]),
                    "{} compact output must require its semantic discriminator",
                    runtime_tool.id.wire_name()
                );
                assert!(
                    runtime_tool.output_schema["properties"]["result_type"]["enum"]
                        .as_array()
                        .is_some_and(|variants| !variants.is_empty())
                );
                assert!(
                    runtime_tool.output_schema["properties"]["result_type"]["description"]
                        .as_str()
                        .is_some_and(|description| !description.is_empty())
                );

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

                assert_runtime_variant_semantics_are_documented(
                    &runtime_tool.input_schema,
                    &documentation_tool.input_schema,
                    runtime_tool.id.wire_name(),
                );
                assert!(
                    runtime_tool.input_schema["description"]
                        .as_str()
                        .is_some_and(|description| description.contains("Semantic type")),
                    "{} runtime input schema must carry its semantic summary",
                    runtime_tool.id.wire_name()
                );
                let advertised_examples = runtime_tool.input_schema["examples"]
                    .as_array()
                    .map_or(0, Vec::len);
                assert!(advertised_examples <= 1);
                if runtime_tool.id == AgentToolId::RECORD_SHAPING_CHECKPOINT {
                    let properties = runtime_tool.input_schema["properties"]
                        .as_object()
                        .expect("checkpoint runtime schema properties");
                    let descriptions =
                        json_values_for_key(&runtime_tool.input_schema, "description");
                    assert!(descriptions.iter().any(|description| {
                        description.as_str().is_some_and(|description| {
                            description.contains("create_initial")
                                && description.contains("replace_current")
                        })
                    }));
                    let semantic_types =
                        json_values_for_key(&runtime_tool.input_schema, "x-volicord-semantic-type");
                    for semantic_type in [
                        "BaselineRef",
                        "TaskId",
                        "RequestHash",
                        "SourceRef::repository_file",
                    ] {
                        assert!(
                            semantic_types
                                .iter()
                                .any(|value| { value.as_str() == Some(semantic_type) }),
                            "runtime schema must preserve `{semantic_type}`"
                        );
                    }
                    assert!(schema_accepts_json_null(&properties["baseline_ref"]));
                    let gap_kinds = json_values_for_key(&properties["gaps"]["items"], "enum")
                        .into_iter()
                        .filter_map(Value::as_array)
                        .flatten()
                        .filter_map(Value::as_str)
                        .collect::<BTreeSet<_>>();
                    for gap_kind in [
                        "implementation_boundary_missing",
                        "user_product_decision_required",
                        "user_technical_decision_required",
                        "user_scope_decision_required",
                    ] {
                        assert!(
                            gap_kinds.contains(gap_kind),
                            "runtime ShapingGapInput must advertise `{gap_kind}`"
                        );
                    }
                    for (schema, discriminator, expected) in [
                        (
                            &properties["checkpoint_operation"],
                            "operation",
                            ["create_initial", "replace_current"].as_slice(),
                        ),
                        (
                            &properties["source_refs"]["items"],
                            "source_kind",
                            [
                                "repository_file",
                                "git_commit",
                                "git_diff",
                                "command",
                                "external_uri",
                                "user_context",
                            ]
                            .as_slice(),
                        ),
                    ] {
                        let advertised = json_values_for_key(schema, "enum")
                            .into_iter()
                            .filter_map(Value::as_array)
                            .flatten()
                            .filter_map(Value::as_str)
                            .collect::<BTreeSet<_>>();
                        for value in expected {
                            assert!(
                                advertised.contains(value),
                                "runtime `{discriminator}` must advertise `{value}`"
                            );
                        }
                        let meanings = json_values_for_key(schema, "x-volicord-variant-meaning");
                        assert_eq!(meanings.len(), expected.len());
                        assert!(meanings.iter().all(|meaning| meaning
                            .as_str()
                            .is_some_and(|meaning| !meaning.trim().is_empty())));
                    }
                }
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

fn schema_accepts_json_null(schema: &Value) -> bool {
    json_values_for_key(schema, "type")
        .into_iter()
        .any(|value| match value {
            Value::String(value_type) => value_type == "null",
            Value::Array(value_types) => value_types.iter().any(|value_type| value_type == "null"),
            _ => false,
        })
}

fn assert_runtime_variant_semantics_are_documented(
    runtime: &Value,
    documentation: &Value,
    tool_name: &str,
) {
    let documented_values = json_values_for_key(documentation, "enum")
        .into_iter()
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    assert_runtime_variant_nodes(runtime, &documented_values, tool_name);
}

fn assert_runtime_variant_nodes(
    value: &Value,
    documented_values: &BTreeSet<&str>,
    tool_name: &str,
) {
    match value {
        Value::Object(object) => {
            if let Some(meaning) = object
                .get("x-volicord-variant-meaning")
                .and_then(Value::as_str)
            {
                assert!(
                    !meaning.is_empty(),
                    "{tool_name} has an empty variant meaning"
                );
                let discriminator = object
                    .get("enum")
                    .and_then(Value::as_array)
                    .filter(|values| values.len() == 1)
                    .and_then(|values| values[0].as_str())
                    .unwrap_or_else(|| {
                        panic!("{tool_name} runtime variant meaning lacks one discriminator")
                    });
                assert!(
                    documented_values.contains(discriminator),
                    "{tool_name} runtime discriminator `{discriminator}` is absent from documentation"
                );
                assert!(
                    object
                        .get("x-volicord-semantic-type")
                        .and_then(Value::as_str)
                        .is_some_and(|semantic_type| !semantic_type.is_empty()),
                    "{tool_name} runtime discriminator `{discriminator}` lacks its semantic type"
                );
            }
            for child in object.values() {
                assert_runtime_variant_nodes(child, documented_values, tool_name);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_runtime_variant_nodes(item, documented_values, tool_name);
            }
        }
        _ => {}
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
            AgentToolId::LIST_PROJECTS.wire_name(),
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
            AgentToolId::GUARD_PROBE.wire_name(),
            AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
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
            schema_variant_by_tag(&tool.output_schema, "result_type", "adapter_error").is_some(),
            "{} output schema should cover structured adapter failures",
            tool.id.wire_name()
        );
        if matches!(tool.id.owner(), AgentToolOwner::CoreMethod(_))
            && !matches!(tool.id.category(), AgentToolCategory::ReadOnly)
        {
            assert!(
                schema_variant_by_tag(
                    &tool.output_schema,
                    "result_type",
                    "response_budget_exceeded"
                )
                .is_some(),
                "{} output schema should cover compact response-budget failures",
                tool.id.wire_name()
            );
            assert!(
                schema_variant_by_tag(&tool.output_schema, "result_type", "post_effect_failure")
                    .is_some(),
                "{} output schema should cover post-effect adapter failures",
                tool.id.wire_name()
            );
        }

        let mut expected_annotations = match tool.id.category() {
            AgentToolCategory::ReadOnly => McpToolAnnotations {
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: true,
                open_world_hint: false,
            },
            AgentToolCategory::NonDestructiveMutation => McpToolAnnotations {
                read_only_hint: false,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
            },
            AgentToolCategory::DestructiveMutation => McpToolAnnotations {
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: false,
            },
        };
        if tool.id.is_idempotent() {
            expected_annotations.idempotent_hint = true;
        }
        assert_eq!(
            tool.annotations,
            expected_annotations,
            "{} annotations should match its effect boundary",
            tool.id.wire_name()
        );
    }
}

#[test]
fn mcp_core_method_output_schemas_follow_exact_response_families() {
    use volicord_types::methods::{MethodResponseBranch, PUBLIC_METHOD_CONTRACTS};

    for contract in PUBLIC_METHOD_CONTRACTS {
        let Ok(tool_id) = AgentToolId::from_wire_name(contract.method().as_str()) else {
            continue;
        };
        if !matches!(tool_id.owner(), AgentToolOwner::CoreMethod(_)) {
            continue;
        }

        let schema = tool_definition(tool_id.wire_name()).output_schema;
        assert_eq!(
            schema_has_definition(&schema, "ToolDryRunResponse"),
            contract.supports_response_branch(MethodResponseBranch::DryRun),
            "{} MCP output schema disagrees with its canonical response family",
            contract.method().as_str()
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
    assert!(schema_has_definition(&schema, "McpUserActionResolution"));
    assert!(schema_has_definition(&schema, "AuthorityReceipt"));
    assert!(schema_has_definition(
        &schema,
        "McpRequestUserActionCompactResult"
    ));
    for variant in [
        "full",
        "summary",
        "workflow",
        "refresh_failure",
        "response_budget_exceeded",
    ] {
        assert!(
            schema_variant_by_tag(&schema, "result_type", variant).is_some(),
            "request-user-action output must expose `{variant}`"
        );
    }
}

#[test]
fn common_mcp_defaults_distinguish_optional_and_required_nullable() -> Result<(), Box<dyn Error>> {
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
            vec![("sensitive_categories", json!([]))],
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
    ];

    for (tool_name, example_id, defaults) in cases {
        let tool = tool_definition(tool_name);
        let required = root_required_fields(&tool.input_schema);
        let example = canonical_example_value(tool_name, example_id)?;
        let decoded = decode_mcp_arguments_to_value(tool_name, example.clone())?;
        for (field, expected) in defaults {
            if expected.is_null() {
                assert!(
                    required.iter().any(|required| required == field),
                    "{tool_name}.{field} should be required-nullable"
                );
                assert!(
                    tool.input_schema["properties"][field]
                        .get("default")
                        .is_none(),
                    "{tool_name}.{field} must not advertise omission as null: {}",
                    tool.input_schema["properties"][field]
                );
                let mut omitted = example.clone();
                omitted
                    .as_object_mut()
                    .expect("canonical arguments must be an object")
                    .remove(field);
                assert!(
                    crate::schema_validation::validate_mcp_tool_arguments(tool_name, &omitted)
                        .is_err()
                );
            } else {
                assert!(
                    !required.iter().any(|required| required == field),
                    "{tool_name}.{field} should be omittable"
                );
                assert_eq!(
                    tool.input_schema["properties"][field]["default"], expected,
                    "{tool_name}.{field} should advertise its exact omission default"
                );
            }
            assert_eq!(
                decoded[field], expected,
                "{tool_name}.{field} example should carry the declared value"
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
            create_required.iter().any(|required| required == field),
            "{}{pointer} should be required-nullable",
            AgentToolId::REQUEST_USER_ACTION.wire_name()
        );
        assert!(
            create_schema["properties"][field].get("default").is_none(),
            "{}{pointer} must not advertise omission as null",
            AgentToolId::REQUEST_USER_ACTION.wire_name()
        );
        assert_eq!(
            decoded.pointer(pointer),
            Some(&expected),
            "{}{pointer} canonical example should carry explicit null",
            AgentToolId::REQUEST_USER_ACTION.wire_name()
        );
    }
    let choice_schema = schema_variant_by_tag(&tool.input_schema, "action_type", "choice")
        .expect("request-user-action schema should retain the choice action branch");
    let choice_required = root_required_fields(choice_schema);
    assert!(
        choice_required.iter().any(|field| field == "options"),
        "choice required: {choice_required:?}; schema: {choice_schema}"
    );
    assert!(choice_required
        .iter()
        .any(|field| field == "sensitive_action_scope"));
    assert!(choice_schema["properties"]["options"]
        .get("default")
        .is_none());
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
        let decoded = decode_mcp_arguments_to_value(tool_name, example.value().clone())?;
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
fn prepare_evidence_capture_arguments_map_strict_variants_and_required_nulls(
) -> Result<(), Box<dyn Error>> {
    let cases = [
        (
            json!({
                "action_form_ref": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
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
                    "command_label": "Focused validation",
                    "expected_exit_code": null
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
                "action_form_ref": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
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
                    "tool_input_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "expected_success": null
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
        let core_capture: volicord_types::schema::EvidenceCaptureSpec = decoded.capture.into_core();
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
fn mcp_input_projection_preserves_core_request_required_members() {
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
        let schema = volicord_types::methods::public_request_schema(method_name)
            .expect("public Core request schema should exist");
        let required = root_required_fields(&schema);
        for field in fields {
            assert!(
                required.iter().any(|required| required == field),
                "{method_name}.{field} should remain a required Core request member"
            );
        }
    }

    let schema = volicord_types::methods::public_request_schema(
        AgentToolId::REQUEST_USER_ACTION.wire_name(),
    )
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
            &[RECORD_RUN_EVIDENCE_BEARING_EXAMPLE_ID],
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
                .map(|example| example.id())
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
            assert!(!example.description().is_empty());
            assert_eq!(
                value,
                example.value(),
                "{} should use its canonical example value",
                example.id()
            );
            crate::schema_validation::validate_mcp_tool_arguments(tool_name, value)?;
            decode_mcp_arguments_to_value(tool_name, value.clone())?;
        }
    }
    Ok(())
}

#[test]
fn record_run_discovery_exposes_execution_only_compatibility() -> Result<(), Box<dyn Error>> {
    let tool = tool_definition(AgentToolId::RECORD_RUN.wire_name());
    for compatibility in ["direct/direct", "work/implementation"] {
        assert!(
            tool.description.contains(compatibility),
            "record_run description should expose {compatibility}"
        );
    }
    assert!(!tool.description.contains("advisor"));

    let arguments = canonical_example_value(
        AgentToolId::RECORD_RUN.wire_name(),
        RECORD_RUN_EVIDENCE_BEARING_EXAMPLE_ID,
    )?;
    assert_eq!(arguments["kind"], "implementation");
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
        RECORD_RUN_EVIDENCE_BEARING_EXAMPLE_ID,
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
fn checkpoint_discriminator_errors_are_branch_local_and_project_one_summary(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-checkpoint-discriminator")?;
    let adapter = adapter(&fixture)?;
    let (task_id, _) = create_task(&adapter)?;
    let action_form_ref = current_action_form_ref(&adapter, &task_id)?;
    let before = fixture.counts()?;
    let error = adapter
        .call_tool(
            AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
            json!({
                "action_form_ref": action_form_ref,
                "task_id": task_id,
                "checkpoint_operation": {"operation": "create"},
                "scope_revision": 0,
                "baseline_ref": null
            }),
        )
        .expect_err("invalid discriminator must fail before Core");
    let response =
        structured_tool_error(AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(), &error);

    assert_eq!(response["code"], "ACTION_FORM_ARGUMENT_MISMATCH");
    assert_eq!(response["selected_variant"], "create_initial");
    assert_eq!(response["authoritative_context"]["context_loaded"], true);
    assert_eq!(
        response["authoritative_context"]["action_form_catalog"]["forms"][0]["method"],
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name()
    );
    assert!(response["retry_contract"]["invalid_paths"]
        .as_array()
        .is_some_and(|paths| paths.contains(&json!("/checkpoint_operation/operation"))));
    assert_eq!(response["failure"]["reached_core"], false);
    assert_eq!(response["failure"]["core_state_unchanged"], true);
    assert!(response["canonical_example"].is_null());
    assert_eq!(
        response["action_form_argument_mismatches"][0]["path"],
        "/checkpoint_operation/operation"
    );
    assert_eq!(
        response["action_form_argument_mismatches"][0]["expected_value"],
        "create_initial"
    );
    assert_eq!(
        response["action_form_argument_mismatches"][0]["received_value"],
        "create"
    );
    assert_eq!(
        response["issues"][0]["path"],
        "/checkpoint_operation/operation"
    );
    assert_eq!(
        response["issues"].as_array().map(Vec::len),
        Some(1),
        "one altered fixed discriminator must produce one mismatch: {}",
        response["issues"]
    );
    let issue = tool_error_issue(
        &response,
        "/checkpoint_operation/operation",
        "ACTION_FORM_ARGUMENT_MISMATCH",
    );
    assert!(issue["allowed_values"]
        .as_array()
        .is_some_and(Vec::is_empty));
    assert!(response["issues"]
        .as_array()
        .expect("issues")
        .iter()
        .all(|issue| issue["path"] != "/baseline_ref"));
    assert_eq!(fixture.counts()?, before);

    let retry = &response["retry_contract"];
    let current_form = &response["authoritative_context"]["action_form_catalog"]["forms"][0];
    assert_eq!(retry["action_form_ref"], current_form["form_ref"]);
    assert_eq!(
        retry["fixed_arguments"]["checkpoint_operation"]["operation"],
        "create_initial"
    );
    assert_eq!(retry["fixed_arguments"]["scope_revision"], 0);
    assert_eq!(retry["fixed_arguments"]["baseline_ref"], Value::Null);
    assert_eq!(retry["corrected_retry_allowed"], true);

    let corrected = adapter.call_tool(
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
        json!({
            "action_form_ref": retry["action_form_ref"],
            "task_id": task_id,
            "checkpoint_operation": {"operation": "create_initial"},
            "scope_revision": 0,
            "baseline_ref": null,
            "summary": "The corrected retry records one bounded structural checkpoint.",
            "implementation_boundary": "Only the current bounded preparation is in scope.",
            "gaps": [{
                "gap_kind": "baseline_missing",
                "summary": "The planning-only repository has no baseline yet.",
                "affected_refs": [],
                "user_action": null
            }],
            "source_refs": [],
            "evidence_refs": []
        }),
    )?;
    assert_eq!(corrected.response_value["base"]["response_kind"], "result");
    assert_eq!(fixture.counts()?.state_version, before.state_version + 1);
    Ok(())
}

#[test]
fn selected_checkpoint_branch_owns_required_fields_and_canonical_example(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-checkpoint-selected-branch")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
        "replace_current",
    )?;
    arguments["checkpoint_operation"]
        .as_object_mut()
        .expect("checkpoint operation")
        .remove("expected_current_checkpoint_id");

    let error = adapter
        .call_tool(
            AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
            arguments,
        )
        .expect_err("selected branch missing field must fail before Core");
    let response =
        structured_tool_error(AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(), &error);

    assert_eq!(response["selected_variant"], "replace_current");
    assert_eq!(
        response["canonical_example"]["checkpoint_operation"]["operation"],
        "replace_current"
    );
    tool_error_issue(
        &response,
        "/checkpoint_operation/expected_current_checkpoint_id",
        "MCP_ARGUMENT_REQUIRED",
    );
    assert!(response["issues"]
        .as_array()
        .expect("issues")
        .iter()
        .all(|issue| !issue["path"]
            .as_str()
            .is_some_and(|path| { path.contains("create_initial") })));
    Ok(())
}

#[test]
fn nested_shaping_call_errors_stay_inside_the_selected_local_branch() -> Result<(), Box<dyn Error>>
{
    for (label, example_id, parent_path, removed_field, expected_path) in [
        (
            "source-ref",
            "repository_file_source_ref",
            "/source_refs/0/source",
            "repository_path",
            "/source_refs/0/source/repository_path",
        ),
        (
            "shaping-gap",
            "product_decision_gap",
            "/gaps/0",
            "summary",
            "/gaps/0/summary",
        ),
        (
            "user-action",
            "product_decision_gap",
            "/gaps/0/user_action/action",
            "judgment_kind",
            "/gaps/0/user_action/action/judgment_kind",
        ),
    ] {
        let fixture = CoreFixture::new(&format!("mcp-nested-{label}"))?;
        let adapter = adapter(&fixture)?;
        let (task_id, _) = create_task(&adapter)?;
        let action_form_ref = current_action_form_ref(&adapter, &task_id)?;
        let mut arguments = canonical_example_value(
            AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
            example_id,
        )?;
        arguments["task_id"] = json!(task_id);
        arguments["action_form_ref"] = json!(action_form_ref);
        arguments["scope_revision"] = json!(0);
        arguments["baseline_ref"] = Value::Null;
        arguments
            .pointer_mut(parent_path)
            .and_then(Value::as_object_mut)
            .unwrap_or_else(|| panic!("{label} selected branch object"))
            .remove(removed_field);
        let before = fixture.counts()?;

        let error = adapter
            .call_tool(
                AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
                arguments,
            )
            .expect_err("malformed nested branch must fail before Core");
        let response =
            structured_tool_error(AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(), &error);

        assert_eq!(
            response["issues"].as_array().map(Vec::len),
            Some(1),
            "{label}: {}",
            response["issues"]
        );
        tool_error_issue(&response, expected_path, "MCP_ARGUMENT_REQUIRED");
        assert_eq!(response["failure"]["reached_core"], false, "{label}");
        assert_eq!(fixture.counts()?, before, "{label}");
    }
    Ok(())
}

#[test]
fn valid_nested_shaping_calls_cover_repository_source_and_decision_gaps(
) -> Result<(), Box<dyn Error>> {
    for example_id in [
        "repository_file_source_ref",
        "product_decision_gap",
        "technical_decision_gap",
        "scope_decision_gap",
    ] {
        let fixture = CoreFixture::new(&format!("mcp-valid-{example_id}"))?;
        let adapter = adapter(&fixture)?;
        let (task_id, _) = create_task(&adapter)?;
        let action_form_ref = current_action_form_ref(&adapter, &task_id)?;
        let mut arguments = canonical_example_value(
            AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
            example_id,
        )?;
        arguments["task_id"] = json!(task_id);
        arguments["action_form_ref"] = json!(action_form_ref);
        arguments["scope_revision"] = json!(0);
        arguments["baseline_ref"] = Value::Null;
        if example_id == "repository_file_source_ref" {
            arguments["gaps"] = json!([{
                "gap_kind": "baseline_missing",
                "summary": "The planning-only repository has no current baseline.",
                "affected_refs": [],
                "user_action": null
            }]);
        }

        let response = adapter.call_tool(
            AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
            arguments,
        )?;

        assert_eq!(response.response_value["base"]["response_kind"], "result");
        assert_eq!(
            response.response_value["shaping_checkpoint"]["source_refs"]
                .as_array()
                .map(Vec::len),
            Some(usize::from(example_id == "repository_file_source_ref")),
            "{example_id}"
        );
        assert_eq!(
            response.response_value["created_user_action_request_refs"]
                .as_array()
                .map(Vec::len),
            Some(usize::from(example_id != "repository_file_source_ref")),
            "{example_id}"
        );
    }
    Ok(())
}

#[test]
fn record_run_invalid_kind_reports_allowed_values() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-invalid-record-run-kind")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        AgentToolId::RECORD_RUN.wire_name(),
        RECORD_RUN_EVIDENCE_BEARING_EXAMPLE_ID,
    )?;
    arguments["kind"] = json!("test");

    let error = adapter
        .call_tool(AgentToolId::RECORD_RUN.wire_name(), arguments)
        .expect_err("invalid kind should fail before Core");
    let response = structured_tool_error(AgentToolId::RECORD_RUN.wire_name(), &error);
    let issue = tool_error_issue(&response, "/kind", "MCP_ARGUMENT_ENUM_VALUE");
    let message = issue["message"].as_str().expect("enum issue message");
    assert!(message.contains("implementation"));
    assert!(message.contains("direct"));
    assert_eq!(issue["expected_semantic_type"], "RunKind");
    assert_eq!(issue["allowed_values"], json!(["implementation", "direct"]));
    assert!(response["canonical_example"].is_object());
    assert_eq!(response["retryable"], true);
    assert_eq!(response["reached_core"], false);
    assert_eq!(response["committed"], false);
    Ok(())
}

#[test]
fn intake_malformed_state_record_ref_reports_semantic_owner() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-invalid-intake-state-record-ref")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(AgentToolId::INTAKE.wire_name(), "create_new")?;
    arguments["initial_context_refs"] = json!(["this is prose, not an authority ref"]);

    let error = adapter
        .call_tool(AgentToolId::INTAKE.wire_name(), arguments)
        .expect_err("malformed StateRecordRef should fail before Core");
    let response = structured_tool_error(AgentToolId::INTAKE.wire_name(), &error);
    let issue = tool_error_issue(
        &response,
        "/initial_context_refs/0",
        "MCP_ARGUMENT_TYPE_MISMATCH",
    );
    assert_eq!(issue["expected_semantic_type"], "StateRecordRef");
    let hint = issue["owner_hint"].as_str().expect("field owner hint");
    assert!(hint.contains("StateRecordRef"));
    assert!(hint.contains("plain_language_request"));
    assert!(response["canonical_example"].is_object());
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
            RECORD_RUN_EVIDENCE_BEARING_EXAMPLE_ID,
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
        RECORD_RUN_EVIDENCE_BEARING_EXAMPLE_ID,
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
fn record_run_evidence_example_expands_collection_defaults_and_required_nulls(
) -> Result<(), Box<dyn Error>> {
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
        RECORD_RUN_EVIDENCE_BEARING_EXAMPLE_ID,
    )?;
    arguments["unexpected"] = json!("not accepted");

    let error = adapter
        .call_tool(AgentToolId::RECORD_RUN.wire_name(), arguments)
        .expect_err("unknown root field should fail before Core");
    let response = structured_tool_error(AgentToolId::RECORD_RUN.wire_name(), &error);
    let issue = tool_error_issue(&response, "/unexpected", "MCP_ARGUMENT_UNKNOWN");
    assert_eq!(issue["allowed_values"], json!([]));
    assert!(issue["owner_hint"].is_string());
    assert_eq!(
        issue
            .as_object()
            .expect("issue object")
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "path",
            "code",
            "message",
            "expected_semantic_type",
            "allowed_values",
            "owner_hint",
        ])
    );
    assert!(response["canonical_example"].is_object());
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
        annotations: McpToolAnnotations {
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
        BTreeSet::from([
            "authority_receipt",
            "method_result",
            "operation_result_ref",
            "presentation",
            "result_type",
        ])
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
            "operation_result_ref",
            "presentation",
            "result_type",
            "workflow",
        ])
    );
    assert!(workflow["structuredContent"]["workflow"].is_object());
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
        assert_eq!(
            responses[1]["result"]["isError"], false,
            "unexpected compact mutation result: {:#}",
            responses[1]
        );
        Ok(responses[1]["result"]["structuredContent"].clone())
    }

    let stage_fixture = CoreFixture::new("mcp-default-compact-stage")?;
    let stage_adapter = adapter(&stage_fixture)?;
    let (stage_task_id, _, _) = create_implementation_task(&stage_fixture)?;
    let stage_action_form_ref =
        action_form_ref_for_method(&stage_adapter, &stage_task_id, MethodName::StageArtifact)?;
    let staged = call_default(
        &stage_fixture,
        AgentToolId::STAGE_ARTIFACT.wire_name(),
        json!({
            "action_form_ref": stage_action_form_ref,
            "task_id": stage_task_id,
            "display_name": "default-stage.log",
            "content_type": "text/plain",
            "redaction_state": "none",
            "safe_bytes_or_notice": "Default compact staging result.",
            "expected_sha256": null,
            "expected_size_bytes": null,
            "relation_hint": null
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
    let (capture_task_id, capture_change_unit_id, _) =
        create_implementation_task(&capture_fixture)?;
    let capture_scope_arguments = bind_action_form_arguments(
        &capture_adapter,
        &capture_task_id,
        MethodName::UpdateScope,
        json!({
            "task_id": capture_task_id,
            "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
            "goal_summary": null,
            "scope_update": null,
            "scope_boundary": null,
            "non_goals": null,
            "acceptance_criteria": null,
            "autonomy_boundary": null,
            "change_unit": {"operation": "keep_current"}
        }),
    )?;
    capture_adapter.call_tool(
        AgentToolId::UPDATE_SCOPE.wire_name(),
        capture_scope_arguments,
    )?;
    let capture_status = capture_adapter.call_tool(
        AgentToolId::STATUS.wire_name(),
        json!({"task_id": capture_task_id, "detail": "full"}),
    )?;
    let capture_criterion_id = capture_status.response_value["active_task"]["acceptance_criteria"]
        [0]["acceptance_criterion_id"]
        .as_str()
        .ok_or("status should expose the acceptance criterion")?;
    let capture_action_form_ref = action_form_ref_for_method(
        &capture_adapter,
        &capture_task_id,
        MethodName::PrepareEvidenceCapture,
    )?;
    let capture = call_default(
        &capture_fixture,
        AgentToolId::PREPARE_EVIDENCE_CAPTURE.wire_name(),
        json!({
            "action_form_ref": capture_action_form_ref,
            "task_id": capture_task_id,
            "change_unit_id": capture_change_unit_id,
            "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
            "target": {
                "target_kind": "acceptance_criterion",
                "acceptance_criterion_id": capture_criterion_id
            },
            "capture": {
                "capture_kind": "verified_command_execution",
                "command_sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                "command_label": "Focused compact projection validation",
                "expected_exit_code": null
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
    let (record_task_id, change_unit_id, _) = create_implementation_task(&record_fixture)?;
    let record_status = record_adapter.call_tool(
        AgentToolId::STATUS.wire_name(),
        json!({"task_id": record_task_id, "detail": "full"}),
    )?;
    let criterion_id = record_status.response_value["active_task"]["acceptance_criteria"][0]
        ["acceptance_criterion_id"]
        .as_str()
        .ok_or("status should expose the acceptance criterion")?;
    let stage_for_run_action_form_ref =
        action_form_ref_for_method(&record_adapter, &record_task_id, MethodName::StageArtifact)?;
    let staged_for_run = record_adapter.call_tool(
        AgentToolId::STAGE_ARTIFACT.wire_name(),
        json!({
            "action_form_ref": stage_for_run_action_form_ref,
            "task_id": record_task_id,
            "display_name": "record-compact.log",
            "content_type": "text/plain",
            "redaction_state": "none",
            "safe_bytes_or_notice": "Evidence attachment for compact record_run refs.",
            "expected_sha256": null,
            "expected_size_bytes": null,
            "relation_hint": null
        }),
    )?;
    let staged_handle = staged_for_run.response_value["staged_artifact_handle"].clone();
    let target = json!({
        "target_kind": "acceptance_criterion",
        "acceptance_criterion_id": criterion_id,
    });
    let record_run_action_form_ref =
        action_form_ref_for_method(&record_adapter, &record_task_id, MethodName::RecordRun)?;
    let recorded = call_default(
        &record_fixture,
        AgentToolId::RECORD_RUN.wire_name(),
        json!({
            "action_form_ref": record_run_action_form_ref,
            "task_id": record_task_id,
            "change_unit_id": change_unit_id,
            "kind": "implementation",
            "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
            "run_id": null,
            "write_ticket_id": null,
            "performed_operation": null,
            "summary": "Recorded compact follow-up references.",
            "observed_changes": {
                "changed_paths": [],
                "product_file_write_observed": false,
                "sensitive_categories": [],
                "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF
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
                "observed_at": "2026-07-13T00:00:00Z",
                "observed_by_actor_source": null,
                "tool_name": null,
                "tool_invocation_id": null
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
    let (prepare_task_id, prepare_change_unit_id, _) =
        create_implementation_task(&prepare_fixture)?;
    let prepare_write_action_form_ref =
        action_form_ref_for_method(&prepare_adapter, &prepare_task_id, MethodName::PrepareWrite)?;
    let prepared = call_default(
        &prepare_fixture,
        AgentToolId::PREPARE_WRITE.wire_name(),
        json!({
            "action_form_ref": prepare_write_action_form_ref,
            "task_id": prepare_task_id,
            "change_unit_id": prepare_change_unit_id,
            "intended_operation": "Update the export flow.",
            "intended_paths": ["src/export.rs"],
            "product_file_write_intended": true,
            "sensitive_categories": [],
            "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF
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
    let reconcile_adapter = adapter(&reconcile_fixture)?;
    let (reconcile_task_id, _, _) = create_implementation_task(&reconcile_fixture)?;
    let reconcile_action_form_ref = action_form_ref_for_method(
        &reconcile_adapter,
        &reconcile_task_id,
        MethodName::ReconcileChanges,
    )?;
    let reconciled = call_default(
        &reconcile_fixture,
        AgentToolId::RECONCILE_CHANGES.wire_name(),
        json!({
            "action_form_ref": reconcile_action_form_ref,
            "task_id": reconcile_task_id
        }),
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
    let (task_id, _, _) = create_implementation_task(&fixture)?;
    let close_task_action_form_ref =
        action_form_ref_for_method(&setup_adapter, &task_id, MethodName::CloseTask)?;
    let adapter = adapter(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::CLOSE_TASK.wire_name(),
            json!({
                "action_form_ref": close_task_action_form_ref,
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
