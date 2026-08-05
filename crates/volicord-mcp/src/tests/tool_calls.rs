use super::*;
use volicord_types::methods::{public_method_contract, WorkflowActionAdmissionClass};

fn assert_workflow_catalog_matches_allowed_actions(workflow: &Value) {
    let allowed_task_bound = workflow["allowed_actions"]
        .as_array()
        .expect("allowed_actions")
        .iter()
        .filter_map(|method| {
            let method = serde_json::from_value::<MethodName>(method.clone()).expect("method");
            (public_method_contract(method).workflow_action_admission()
                == WorkflowActionAdmissionClass::TaskStateBound)
                .then(|| method.as_str().to_owned())
        })
        .collect::<BTreeSet<_>>();
    let actions = workflow["action_catalog"]["actions"]
        .as_array()
        .expect("action catalog actions");
    let catalog_methods = actions
        .iter()
        .map(|action| {
            action["method"]
                .as_str()
                .expect("catalog method")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(catalog_methods, allowed_task_bound, "{workflow:#}");
    let catalog_keys = actions
        .iter()
        .map(|action| {
            (
                action["method"].as_str().expect("catalog method"),
                action["semantic_variant"]
                    .as_str()
                    .expect("catalog semantic variant"),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(catalog_keys.len(), actions.len(), "{workflow:#}");

    let required_method = workflow["action_catalog"]["required_method"].as_str();
    assert_eq!(
        actions.iter().any(|action| action["role"] == "required"),
        required_method.is_some(),
        "{workflow:#}"
    );
    for action in actions {
        assert_eq!(
            action["role"] == "required",
            action["method"].as_str() == required_method,
            "{workflow:#}"
        );
        assert_eq!(
            action["expected_state_version"], workflow["expected_state_version"],
            "{workflow:#}"
        );
    }
}
use volicord_test_support::TestRuntimeHomeSetup;

#[test]
fn admitted_tool_routing_rejects_a_different_runtime_home_context() -> Result<(), Box<dyn Error>> {
    let routed = CoreFixture::new("mcp-routing-runtime-home")?;
    let different = CoreFixture::new("mcp-context-runtime-home")?;
    let adapter = adapter(&routed)?;
    let different_context = different.mutation_context()?;
    let before = different.counts()?;

    let error = adapter
        .tools_for_context(&different_context)
        .expect_err("MCP routing home A must reject admitted context B");

    assert!(matches!(error, McpAdapterError::Environment(_)));
    assert!(error
        .to_string()
        .contains("runtime_home_mutation_context_mismatch"));
    assert_eq!(different.counts()?, before);
    Ok(())
}

#[test]
fn mcp_mutation_uses_the_admitted_lexical_runtime_home() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-mutation-runtime-home-alias")?;
    let runtime_home = fixture.runtime_home_path();
    let alias = runtime_home
        .parent()
        .expect("fixture Runtime Home has a parent")
        .join(".")
        .join(
            runtime_home
                .file_name()
                .expect("fixture Runtime Home has a file name"),
        );
    let adapter = adapter_at_runtime_home(&fixture, &alias)?;
    let before = fixture.counts()?;

    let committed = adapter.call_tool(AgentToolId::INTAKE.wire_name(), intake_args(None))?;

    assert_eq!(committed.response_value["base"]["response_kind"], "result");
    assert_eq!(fixture.counts()?.state_version, before.state_version + 1);
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
fn checkpoint_action_form_rejects_omission_and_stale_refs_before_core() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-checkpoint-action-form-admission")?;
    let adapter = adapter(&fixture)?;
    let (task_id, _) = create_task(&adapter)?;
    let before = fixture.counts()?;
    let omitted = adapter
        .call_tool(
            AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
            json!({
                "task_id": task_id,
                "checkpoint_operation": {"operation": "create_initial"},
                "scope_revision": 0,
                "baseline_ref": null,
                "summary": "A structural boundary remains current.",
                "implementation_boundary": null,
                "gaps": [{
                    "gap_kind": "implementation_boundary_missing",
                    "summary": "Define the implementation boundary.",
                    "affected_refs": [],
                    "user_action": null
                }],
                "source_refs": [],
                "evidence_refs": []
            }),
        )
        .expect_err("current state-bound progression must reject an omitted form ref");
    let omitted = structured_error_result(&tool_execution_error_result(
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
        &omitted,
    ));
    assert_eq!(omitted["code"], "MCP_ACTION_FORM_STALE");
    assert_eq!(omitted["issues"][0]["path"], "/action_form_ref");
    assert_eq!(omitted["reached_core"], false);
    assert_eq!(omitted["committed"], false);
    assert_eq!(omitted["authoritative_context"]["context_loaded"], true);
    assert_eq!(
        omitted["authoritative_context"]["baseline_ref"],
        Value::Null
    );
    assert_eq!(omitted["failure"]["checkpoint_recorded"], false);
    assert_eq!(omitted["failure"]["user_action_created"], false);
    assert_eq!(omitted["failure"]["product_repository_changed"], false);
    assert_eq!(omitted["failure"]["repair_required"], false);
    assert_eq!(fixture.counts()?, before);

    let current_ref = omitted["workflow_admission"]["called_method_form"]["form_ref"]
        .as_str()
        .ok_or("stale-form response should expose the current form")?
        .to_owned();
    assert_eq!(current_action_form_ref(&adapter, &task_id)?, current_ref);
    let committed = adapter.call_tool(
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
        json!({
            "action_form_ref": current_ref,
            "task_id": task_id,
            "checkpoint_operation": {"operation": "create_initial"},
            "scope_revision": 0,
            "baseline_ref": null,
            "summary": "A structural boundary remains current.",
            "implementation_boundary": null,
            "gaps": [{
                "gap_kind": "implementation_boundary_missing",
                "summary": "Define the implementation boundary.",
                "affected_refs": [],
                "user_action": null
            }],
            "source_refs": [],
            "evidence_refs": []
        }),
    )?;
    let checkpoint_id = committed.response_value["shaping_checkpoint"]["shaping_checkpoint_id"]
        .as_str()
        .ok_or("checkpoint should be recorded")?;
    let before_stale = fixture.counts()?;
    let stale = adapter
        .call_tool(
            AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
            json!({
                "action_form_ref": current_ref,
                "task_id": task_id,
                "checkpoint_operation": {
                    "operation": "replace_current",
                    "expected_current_checkpoint_id": checkpoint_id,
                    "retired_non_authorizing_request_refs": [],
                    "carry_forward_application_refs": [],
                    "stale_authority_actions": []
                },
                "scope_revision": 0,
                "baseline_ref": null,
                "summary": "The replacement closes the structural gap.",
                "implementation_boundary": "Only the current bounded path.",
                "gaps": [],
                "source_refs": [],
                "evidence_refs": []
            }),
        )
        .expect_err("checkpoint creation must invalidate the prior state-bound form");
    let stale = structured_error_result(&tool_execution_error_result(
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
        &stale,
    ));
    assert_eq!(stale["code"], "MCP_ACTION_FORM_STALE");
    assert_ne!(
        stale["workflow_admission"]["called_method_form"]["form_ref"],
        current_ref
    );
    assert_eq!(fixture.counts()?, before_stale);
    Ok(())
}

#[test]
fn workflow_catalog_admission_is_method_specific_and_pre_core() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-workflow-catalog-admission")?;
    let adapter = adapter(&fixture)?;
    let (task_id, change_unit_id, checkpoint_id, ready_state_version) =
        create_ready_for_implementation_task(&fixture)?;
    let advance_form_ref = action_form_ref_for_method(&adapter, &task_id, MethodName::AdvanceTask)?;
    let before_checkpoint_attempt = fixture.counts()?;
    let before_checkpoint_count = read_only_table_count(&fixture, "shaping_checkpoints")?;
    let before_unrecorded_change_count = read_only_table_count(&fixture, "unrecorded_changes")?;
    let before_product_repository_entries = fs::read_dir(fixture.product_repo_path())?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let rejected_checkpoint = adapter
        .call_tool(
            AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
            json!({
                "action_form_ref": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "task_id": task_id,
                "checkpoint_operation": {
                    "operation": "replace_current",
                    "expected_current_checkpoint_id": checkpoint_id,
                    "retired_non_authorizing_request_refs": [],
                    "carry_forward_application_refs": [],
                    "stale_authority_actions": []
                },
                "scope_revision": 1,
                "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
                "summary": "This otherwise valid replacement is not current.",
                "implementation_boundary": "Keep the current bounded implementation.",
                "gaps": [],
                "source_refs": [],
                "evidence_refs": []
            }),
        )
        .expect_err("ready_for_implementation must reject checkpoint replacement before Core");
    let rejected_checkpoint = structured_tool_error(
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
        &rejected_checkpoint,
    );
    assert_eq!(rejected_checkpoint["code"], "WORKFLOW_ACTION_NOT_ALLOWED");
    assert_eq!(rejected_checkpoint["reached_core"], false);
    assert_eq!(rejected_checkpoint["committed"], false);
    assert_eq!(
        rejected_checkpoint["workflow_admission"]["current_workflow_kind"],
        "ready_for_implementation"
    );
    assert_eq!(
        rejected_checkpoint["workflow_admission"]["called_method"],
        "volicord.record_shaping_checkpoint"
    );
    assert_eq!(
        rejected_checkpoint["workflow_admission"]["required_method"],
        "volicord.advance_task"
    );
    assert_eq!(
        rejected_checkpoint["workflow_admission"]["allowed_methods"],
        json!(["volicord.advance_task"])
    );
    assert_eq!(
        rejected_checkpoint["workflow_admission"]["required_method_forms"][0]["form_ref"],
        advance_form_ref
    );
    assert_eq!(
        rejected_checkpoint["workflow_admission"]["required_method_forms"][0]
            ["selected_semantic_variant"],
        "advance_task"
    );
    for advisor_field in [
        "finalize_advice",
        "result_summary",
        "residual_risks",
        "recovery_constraints",
    ] {
        assert!(
            json_values_for_key(&rejected_checkpoint, advisor_field).is_empty(),
            "workflow admission rejection must not fabricate `{advisor_field}`"
        );
    }
    assert_eq!(fixture.counts()?, before_checkpoint_attempt);
    assert_eq!(
        read_only_table_count(&fixture, "shaping_checkpoints")?,
        before_checkpoint_count
    );
    assert_eq!(
        read_only_table_count(&fixture, "unrecorded_changes")?,
        before_unrecorded_change_count
    );
    assert_eq!(
        fs::read_dir(fixture.product_repo_path())?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<Result<BTreeSet<_>, _>>()?,
        before_product_repository_entries
    );
    let status = adapter.call_tool(
        AgentToolId::STATUS.wire_name(),
        json!({"task_id": task_id, "detail": "workflow"}),
    )?;
    assert_eq!(
        status.response_value["base"]["state_version"],
        ready_state_version
    );
    assert_eq!(
        status.response_value["active_task"]["workflow"]["kind"],
        "ready_for_implementation"
    );
    assert_workflow_catalog_matches_allowed_actions(
        &status.response_value["active_task"]["workflow"],
    );

    adapter.call_tool(
        AgentToolId::ADVANCE_TASK.wire_name(),
        json!({
            "action_form_ref": advance_form_ref,
            "task_id": task_id,
            "shaping_checkpoint_id": checkpoint_id,
            "change_unit_id": change_unit_id,
            "scope_revision": 1,
            "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
            "user_action_resolution_ids": []
        }),
    )?;
    let implementation_status = adapter.call_tool(
        AgentToolId::STATUS.wire_name(),
        json!({"task_id": task_id, "detail": "workflow"}),
    )?;
    assert_workflow_catalog_matches_allowed_actions(
        &implementation_status.response_value["active_task"]["workflow"],
    );
    let update_form_ref = action_form_ref_for_method(&adapter, &task_id, MethodName::UpdateScope)?;
    let prepare_evidence_form_ref =
        action_form_ref_for_method(&adapter, &task_id, MethodName::PrepareEvidenceCapture)?;
    let prepare_form_ref =
        action_form_ref_for_method(&adapter, &task_id, MethodName::PrepareWrite)?;
    let stage_artifact_form_ref =
        action_form_ref_for_method(&adapter, &task_id, MethodName::StageArtifact)?;
    let record_form_ref = action_form_ref_for_method(&adapter, &task_id, MethodName::RecordRun)?;
    let request_user_action_form_ref =
        action_form_ref_for_method(&adapter, &task_id, MethodName::RequestUserAction)?;
    let reconcile_changes_form_ref =
        action_form_ref_for_method(&adapter, &task_id, MethodName::ReconcileChanges)?;
    let check_close_form_ref =
        action_form_ref_for_method(&adapter, &task_id, MethodName::CheckClose)?;
    let close_task_form_ref =
        action_form_ref_for_method(&adapter, &task_id, MethodName::CloseTask)?;
    assert_eq!(
        [
            update_form_ref.as_str(),
            prepare_evidence_form_ref.as_str(),
            prepare_form_ref.as_str(),
            stage_artifact_form_ref.as_str(),
            record_form_ref.as_str(),
            request_user_action_form_ref.as_str(),
            reconcile_changes_form_ref.as_str(),
            check_close_form_ref.as_str(),
            close_task_form_ref.as_str(),
        ]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len(),
        9
    );

    let pairwise_forms = [
        (MethodName::UpdateScope, update_form_ref.as_str()),
        (MethodName::PrepareWrite, prepare_form_ref.as_str()),
        (MethodName::RecordRun, record_form_ref.as_str()),
        (MethodName::CheckClose, check_close_form_ref.as_str()),
    ];
    for (called_method, called_form_ref) in pairwise_forms {
        let called_tool = AgentToolId::from_method(called_method)
            .expect("every pairwise method is an Agent tool");
        for (provided_method, provided_form_ref) in pairwise_forms {
            if called_method == provided_method {
                continue;
            }
            let before = fixture.counts()?;
            let error = adapter
                .call_tool(
                    called_tool.wire_name(),
                    json!({
                        "action_form_ref": provided_form_ref,
                        "task_id": task_id,
                    }),
                )
                .expect_err("one method's form must not authorize another method");
            let error = structured_tool_error(called_tool.wire_name(), &error);
            assert_eq!(error["code"], "MCP_ACTION_FORM_STALE");
            assert_eq!(error["reached_core"], false);
            assert_eq!(error["committed"], false);
            assert_eq!(
                error["workflow_admission"]["called_method"],
                called_method.as_str()
            );
            if called_method == MethodName::UpdateScope {
                assert!(error["workflow_admission"]["called_method_form"].is_null());
                assert_eq!(
                    error["workflow_admission"]["valid_called_method_form_refs"]
                        .as_array()
                        .expect("update-scope form refs")
                        .len(),
                    2
                );
            } else {
                assert_eq!(
                    error["workflow_admission"]["called_method_form"]["form_ref"],
                    called_form_ref
                );
            }
            assert_ne!(provided_form_ref, called_form_ref);
            assert_eq!(fixture.counts()?, before);
        }
    }

    let assert_stale_without_effect = |arguments: Value| -> Result<(), Box<dyn Error>> {
        let before = fixture.counts()?;
        let error = adapter
            .call_tool(AgentToolId::UPDATE_SCOPE.wire_name(), arguments)
            .expect_err("non-current update_scope form must reject before Core");
        let error = structured_tool_error(AgentToolId::UPDATE_SCOPE.wire_name(), &error);
        assert_eq!(error["code"], "MCP_ACTION_FORM_STALE");
        assert_eq!(error["reached_core"], false);
        assert_eq!(error["committed"], false);
        assert_eq!(
            error["workflow_admission"]["called_method_form"]["form_ref"],
            update_form_ref
        );
        assert_eq!(fixture.counts()?, before);
        Ok(())
    };
    assert_stale_without_effect(json!({
        "task_id": task_id,
        "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
        "change_unit": {"operation": "keep_current"}
    }))?;
    assert_stale_without_effect(json!({
        "action_form_ref": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "task_id": task_id,
        "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
        "change_unit": {"operation": "keep_current"}
    }))?;
    assert_stale_without_effect(json!({
        "action_form_ref": advance_form_ref,
        "task_id": task_id,
        "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
        "change_unit": {"operation": "keep_current"}
    }))?;

    let before_cross_method = fixture.counts()?;
    let cross_method = adapter
        .call_tool(
            AgentToolId::PREPARE_WRITE.wire_name(),
            json!({
                "action_form_ref": update_form_ref,
                "task_id": task_id,
                "change_unit_id": change_unit_id,
                "intended_operation": "cross-method admission test",
                "intended_paths": [volicord_test_support::core_fixtures::DEFAULT_PRODUCT_PATH],
                "product_file_write_intended": true,
                "sensitive_categories": [],
                "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF
            }),
        )
        .expect_err("an update_scope form must not authorize prepare_write");
    let cross_method = structured_tool_error(AgentToolId::PREPARE_WRITE.wire_name(), &cross_method);
    assert_eq!(cross_method["code"], "MCP_ACTION_FORM_STALE");
    assert_eq!(
        cross_method["workflow_admission"]["called_method_form"]["form_ref"],
        prepare_form_ref
    );
    assert_eq!(fixture.counts()?, before_cross_method);

    let foreign_fixture = CoreFixture::new("mcp-workflow-catalog-foreign-form")?;
    let foreign_adapter = super::support::adapter(&foreign_fixture)?;
    let (foreign_task_id, _, _) = create_implementation_task(&foreign_fixture)?;
    let foreign_form_ref =
        action_form_ref_for_method(&foreign_adapter, &foreign_task_id, MethodName::UpdateScope)?;
    assert_stale_without_effect(json!({
        "action_form_ref": foreign_form_ref,
        "task_id": task_id,
        "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
        "change_unit": {"operation": "keep_current"}
    }))?;
    Ok(())
}

#[test]
fn checkpoint_fixed_basis_mismatch_is_pre_core_and_reports_typed_null() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-checkpoint-null-basis-mismatch")?;
    let adapter = adapter(&fixture)?;
    let (task_id, _) = create_task(&adapter)?;
    let action_form_ref = current_action_form_ref(&adapter, &task_id)?;
    let before = fixture.counts()?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
            json!({
                "detail": "workflow",
                "action_form_ref": action_form_ref,
                "task_id": task_id,
                "checkpoint_operation": {"operation": "create_initial"},
                "scope_revision": 0,
                "baseline_ref": "0123456789012345678901234567890123456789",
                "summary": "This request has a mismatched authority basis.",
                "implementation_boundary": "No mutation should occur.",
                "gaps": [],
                "source_refs": [],
                "evidence_refs": []
            }),
        ),
    ])?);
    let mut output = Vec::new();
    run_stdio(adapter, BufReader::new(input), &mut output)?;
    let responses = stdio_responses(&output)?;
    let structured = &responses[1]["result"]["structuredContent"];
    assert_eq!(structured["result_type"], "adapter_error");
    assert_eq!(structured["code"], "ACTION_FORM_ARGUMENT_MISMATCH");
    assert_eq!(
        structured["action_form_argument_mismatches"][0]["path"],
        "/baseline_ref"
    );
    assert_eq!(
        structured["action_form_argument_mismatches"][0]["expected_value"],
        Value::Null
    );
    assert!(structured["action_form_argument_mismatches"][0]["received_value"].is_string());
    assert_eq!(
        structured["action_form_argument_mismatches"][0]["state_change_applied"],
        false
    );
    assert_eq!(structured["failure"]["reached_core"], false);
    assert_eq!(structured["failure"]["checkpoint_recorded"], false);
    assert_eq!(structured["failure"]["user_action_created"], false);
    assert_eq!(structured["failure"]["core_state_unchanged"], true);
    assert_eq!(structured["failure"]["current_baseline_valid"], true);
    assert_eq!(structured["failure"]["repair_required"], false);
    assert_eq!(
        structured["action_form_argument_mismatches"][0]["current_method_form"]["form_ref"],
        action_form_ref
    );
    assert_eq!(
        structured["retry_contract"]["action_form_ref"],
        action_form_ref
    );
    assert_eq!(
        structured["retry_contract"]["fixed_arguments"]["baseline_ref"],
        Value::Null
    );
    assert_eq!(
        structured["retry_contract"]["fixed_arguments"]["checkpoint_operation"]["operation"],
        "create_initial"
    );
    assert_eq!(structured["failure"]["current_task_phase"], "shaping");
    assert!(structured["issues"][0]["message"]
        .as_str()
        .is_some_and(|message| message.contains("Core was not reached")));
    assert_eq!(fixture.counts()?, before);

    let retry_adapter = super::support::adapter(&fixture)?;
    let corrected = bind_action_form_arguments(
        &retry_adapter,
        &task_id,
        MethodName::RecordShapingCheckpoint,
        json!({
            "summary": "The exact current authority basis is accepted.",
            "implementation_boundary": "Only the current bounded implementation.",
            "gaps": [{
                "gap_kind": "baseline_missing",
                "summary": "The current Task has no baseline.",
                "affected_refs": [],
                "user_action": null
            }],
            "source_refs": [],
            "evidence_refs": []
        }),
    )?;
    let corrected = retry_adapter.call_tool(
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
        corrected,
    )?;
    assert_eq!(corrected.response_value["base"]["response_kind"], "result");
    Ok(())
}

#[test]
fn invalid_update_scope_variant_is_rejected_before_core_with_exact_current_variants(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-update-scope-invalid-variant")?;
    let adapter = adapter(&fixture)?;
    let (task_id, _, _) = create_implementation_task(&fixture)?;
    let keep_form = action_form_for_variant(
        &adapter,
        &task_id,
        MethodName::UpdateScope,
        WorkflowActionSemanticVariant::KeepCurrentChangeUnit,
    )?;
    let before = fixture.counts()?;
    let error = adapter
        .call_tool(
            AgentToolId::UPDATE_SCOPE.wire_name(),
            json!({
                "action_form_ref": keep_form.form_ref,
                "task_id": task_id,
                "detail": "workflow",
                "goal_summary": null,
                "scope_update": null,
                "scope_boundary": null,
                "non_goals": null,
                "acceptance_criteria": null,
                "autonomy_boundary": null,
                "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
                "change_unit": {
                    "operation": "create_current",
                    "scope_summary": "A second current Change Unit must not be created.",
                    "affected_paths": ["src/current.rs"]
                },
                "related_scope_decision_refs": []
            }),
        )
        .expect_err("create-current is not valid while a current Change Unit exists");
    let error = structured_tool_error(AgentToolId::UPDATE_SCOPE.wire_name(), &error);

    assert_eq!(error["code"], "WORKFLOW_ACTION_VARIANT_NOT_ALLOWED");
    assert_eq!(error["reached_core"], false);
    assert_eq!(error["committed"], false);
    assert_eq!(
        error["workflow_admission"]["called_semantic_variant"],
        WorkflowActionSemanticVariant::CreateCurrentChangeUnit.as_str()
    );
    assert_eq!(
        error["workflow_admission"]["valid_called_method_variants"],
        json!([
            WorkflowActionSemanticVariant::KeepCurrentChangeUnit,
            WorkflowActionSemanticVariant::ReplaceCurrentChangeUnit,
        ])
    );
    assert_eq!(
        error["workflow_admission"]["valid_called_method_form_refs"]
            .as_array()
            .expect("current form refs")
            .len(),
        2
    );
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn projected_replace_form_executes_one_atomic_change_unit_replacement() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-update-scope-replace-form")?;
    let adapter = adapter(&fixture)?;
    let (task_id, current_change_unit_id, _) = create_implementation_task(&fixture)?;
    let replace_form = action_form_for_variant(
        &adapter,
        &task_id,
        MethodName::UpdateScope,
        WorkflowActionSemanticVariant::ReplaceCurrentChangeUnit,
    )?;
    let before = fixture.counts()?;
    let response = adapter.call_tool(
        AgentToolId::UPDATE_SCOPE.wire_name(),
        json!({
            "action_form_ref": replace_form.form_ref,
            "task_id": task_id,
            "detail": "summary",
            "goal_summary": null,
            "scope_update": null,
            "scope_boundary": null,
            "non_goals": null,
            "acceptance_criteria": null,
            "autonomy_boundary": null,
            "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
            "change_unit": {
                "operation": "replace_current",
                "scope_summary": "Replace the current implementation Change Unit.",
                "affected_paths": ["src/current.rs"]
            },
            "related_scope_decision_refs": []
        }),
    )?;
    let replacement_id = response.response_value["change_unit_ref"]["record_id"]
        .as_str()
        .expect("replacement Change Unit ref");

    assert_ne!(replacement_id, current_change_unit_id);
    let after = fixture.counts()?;
    assert_eq!(after.state_version, before.state_version + 1);
    assert_eq!(after.change_units, before.change_units + 1);
    let store = CoreProjectStore::open_read_only(
        fixture.runtime_home_path(),
        &ProjectId::new(fixture.project_id()),
    )?;
    assert_eq!(
        store
            .current_change_unit(&TaskId::new(&task_id))?
            .expect("one current replacement Change Unit")
            .change_unit_id,
        replacement_id
    );
    Ok(())
}

#[test]
fn keep_current_baseline_retarget_surfaces_replace_and_keep_forms_without_effect(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-update-scope-baseline-retarget")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, _, _) = create_implementation_task(&fixture)?;
    let keep_form = action_form_for_variant(
        &setup_adapter,
        &task_id,
        MethodName::UpdateScope,
        WorkflowActionSemanticVariant::KeepCurrentChangeUnit,
    )?;
    let before = fixture.counts()?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::UPDATE_SCOPE.wire_name(),
            json!({
                "action_form_ref": keep_form.form_ref,
                "task_id": task_id,
                "detail": "workflow",
                "goal_summary": null,
                "scope_update": null,
                "scope_boundary": null,
                "non_goals": null,
                "acceptance_criteria": null,
                "autonomy_boundary": null,
                "baseline_ref": "baseline_retargeted",
                "change_unit": {"operation": "keep_current"},
                "related_scope_decision_refs": []
            }),
        ),
    ])?);
    let mut output = Vec::new();
    run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;
    let responses = stdio_responses(&output)?;
    let structured = &responses[1]["result"]["structuredContent"];

    assert_eq!(structured["result_type"], "rejected");
    assert_eq!(
        structured["authority_basis_mismatch"]["field"],
        "baseline_ref"
    );
    assert_eq!(
        structured["authority_basis_mismatch"]["called_method_form"]["selected_semantic_variant"],
        WorkflowActionSemanticVariant::KeepCurrentChangeUnit.as_str()
    );
    assert_eq!(
        structured["authority_basis_mismatch"]["replacement_method_form"]
            ["selected_semantic_variant"],
        WorkflowActionSemanticVariant::ReplaceCurrentChangeUnit.as_str()
    );
    assert_eq!(
        structured["retry_contract"]["selected_semantic_variant"],
        WorkflowActionSemanticVariant::ReplaceCurrentChangeUnit.as_str()
    );
    assert_eq!(structured["failure"]["current_baseline_valid"], false);
    assert_eq!(
        structured["failure"]["exact_retry_action"],
        "baseline retargeting requires replace_current; retry volicord.update_scope with the current replace form"
    );
    assert_eq!(structured["failure"]["reached_core"], true);
    assert_eq!(structured["failure"]["method_committed"], false);
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn implementation_forms_reject_every_tampered_fixed_coordinate_before_core(
) -> Result<(), Box<dyn Error>> {
    fn altered_fixed_value(value: &Value) -> Value {
        match value {
            Value::Null => json!("null"),
            Value::Bool(value) => json!(!value),
            Value::Number(value) => json!(value.as_u64().unwrap_or_default() + 1),
            Value::String(value) => json!(format!("{value}_tampered")),
            Value::Array(values) if values.is_empty() => json!([null]),
            Value::Array(_) => json!([]),
            Value::Object(values) => {
                let mut values = values.clone();
                values.insert("tampered".to_owned(), Value::Bool(true));
                Value::Object(values)
            }
        }
    }

    let methods = [
        MethodName::UpdateScope,
        MethodName::PrepareEvidenceCapture,
        MethodName::PrepareWrite,
        MethodName::StageArtifact,
        MethodName::RecordRun,
        MethodName::RequestUserAction,
        MethodName::ReconcileChanges,
        MethodName::CheckClose,
        MethodName::CloseTask,
    ];
    for (index, method) in methods.into_iter().enumerate() {
        let fixture = CoreFixture::new(&format!("mcp-fixed-argument-{index}"))?;
        let adapter = adapter(&fixture)?;
        let (task_id, _, _) = create_implementation_task(&fixture)?;
        let tool = AgentToolId::from_method(method).expect("state-bound method tool");
        let form = action_form_for_method(&adapter, &task_id, method)?;
        assert_eq!(form.expected_state_version, fixture.counts()?.state_version);
        assert!(!form.fixed_argument_paths.is_empty());

        let authored = match method {
            MethodName::ReconcileChanges => json!({"resolution_requests": []}),
            MethodName::CheckClose => json!({}),
            _ => canonical_tool_examples(tool)
                .first()
                .ok_or_else(|| format!("{} canonical example", tool.wire_name()))?
                .value()
                .clone(),
        };
        let exact = bind_action_form_arguments(&adapter, &task_id, method, authored)?;

        for path in &form.fixed_argument_paths {
            let expected = exact
                .pointer(path)
                .ok_or_else(|| format!("{} missing fixed path {path}", method.as_str()))?
                .clone();
            let mut tampered = exact.clone();
            let received = altered_fixed_value(&expected);
            *tampered
                .pointer_mut(path)
                .ok_or_else(|| format!("{} missing mutable path {path}", method.as_str()))? =
                received.clone();
            let before = fixture.counts()?;
            let error = adapter
                .call_tool(tool.wire_name(), tampered)
                .expect_err("tampered fixed authority must reject before Core");
            let error = structured_tool_error(tool.wire_name(), &error);
            assert_eq!(error["code"], "ACTION_FORM_ARGUMENT_MISMATCH");
            assert_eq!(error["reached_core"], false);
            assert_eq!(error["committed"], false);
            let mismatch = error["action_form_argument_mismatches"]
                .as_array()
                .expect("fixed-argument mismatches")
                .iter()
                .find(|mismatch| mismatch["path"].as_str() == Some(path))
                .ok_or_else(|| format!("{} did not report {path}: {error}", method.as_str()))?;
            assert_eq!(mismatch["expected_value"], expected);
            assert_eq!(mismatch["received_value"], received);
            assert_eq!(mismatch["reached_core"], false);
            assert_eq!(mismatch["state_change_applied"], false);
            assert_eq!(
                mismatch["current_method_form"]["form_ref"],
                form.form_ref.as_str()
            );
            assert_eq!(
                error["retry_contract"]["action_form_ref"],
                form.form_ref.as_str()
            );
            assert_eq!(fixture.counts()?, before);
        }

        let corrected = adapter.call_tool(tool.wire_name(), exact)?;
        assert!(
            corrected.verified_invocation.is_some(),
            "{} corrected retry must enter Core",
            method.as_str()
        );
    }
    Ok(())
}

#[test]
fn removed_checkpoint_mirror_fields_are_branch_local_unknown_arguments(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-checkpoint-removed-mirrors")?;
    let adapter = adapter(&fixture)?;
    let (task_id, _) = create_task(&adapter)?;
    let before = fixture.counts()?;

    for field in ["current_checkpoint_ref", "predecessor_checkpoint_ref"] {
        let mut arguments = bind_action_form_arguments(
            &adapter,
            &task_id,
            MethodName::RecordShapingCheckpoint,
            json!({
                "summary": "Removed checkpoint mirrors are not mutation inputs.",
                "implementation_boundary": null,
                "gaps": [],
                "source_refs": [],
                "evidence_refs": []
            }),
        )?;
        arguments[field] = Value::Null;
        let error = adapter
            .call_tool(
                AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
                arguments,
            )
            .expect_err("removed checkpoint mirror must be unknown");
        let error =
            structured_tool_error(AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(), &error);
        tool_error_issue(&error, &format!("/{field}"), "MCP_ARGUMENT_UNKNOWN");
        assert_eq!(error["reached_core"], false);
        assert_eq!(fixture.counts()?, before);
    }
    Ok(())
}

#[test]
fn nullable_object_validates_its_non_null_schema_and_keeps_nested_issues(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-nullable-object-validation")?;
    let adapter = adapter(&fixture)?;
    let mut arguments = canonical_example_value(
        AgentToolId::RECORD_RUN.wire_name(),
        RECORD_RUN_EVIDENCE_BEARING_EXAMPLE_ID,
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
fn date_time_format_failure_is_one_structured_issue_without_core_effects(
) -> Result<(), Box<dyn Error>> {
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
        .expect_err("invalid timestamp format should fail descriptor validation");
    let response = structured_tool_error(AgentToolId::REQUEST_USER_ACTION.wire_name(), &error);

    assert_eq!(response["issues"].as_array().map(Vec::len), Some(1));
    assert_eq!(response["reported_issue_count"], 1);
    assert_eq!(response["truncated"], false);
    let issue = tool_error_issue(
        &response,
        "/request/expires_at",
        "MCP_ARGUMENT_TYPE_MISMATCH",
    );
    assert_eq!(issue["expected_semantic_type"], "UtcTimestamp | null");
    assert!(issue["message"]
        .as_str()
        .is_some_and(|message| message.contains("date-time")));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[cfg(unix)]
#[test]
fn descriptor_format_failure_precedes_readonly_storage_rejection() -> Result<(), Box<dyn Error>> {
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
        .expect_err("descriptor argument validation should precede storage preconditions");
    let response = structured_tool_error(AgentToolId::REQUEST_USER_ACTION.wire_name(), &error);

    assert_eq!(response["code"], "MCP_INVALID_ARGUMENTS");
    tool_error_issue(
        &response,
        "/request/expires_at",
        "MCP_ARGUMENT_TYPE_MISMATCH",
    );
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
#[cfg(unix)]
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
    assert_eq!(report.available_projects, 1);
    assert!(report.projects[0].available);
    assert_eq!(report.projects[0].state_write, "not_checked");

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
            AgentToolId::LIST_PROJECTS.wire_name(),
            AgentToolId::BEGIN_INTEGRATION_VERIFICATION.wire_name(),
            AgentToolId::GUARD_PROBE.wire_name(),
            AgentToolId::GET_INTEGRATION_VERIFICATION.wire_name(),
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

    let names = tool_names(&adapter.tools()?);
    assert_eq!(names, expected);
    Ok(())
}

#[test]
fn mcp_and_direct_core_status_produce_the_same_domain_outcome() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-core-status-equivalence")?;
    let direct = CoreService::for_read_only(fixture.runtime_home_path()).status(
        fixture.status_request("req_direct_status_equivalence", None),
        test_agent_invocation(&fixture, OperationCategory::Read),
    )?;
    let adapter = adapter(&fixture)?;
    let through_mcp = adapter.call_tool(
        AgentToolId::STATUS.wire_name(),
        json!({ "detail": "full", "task_id": null }),
    )?;

    let mut direct_domain = direct.response_value;
    direct_domain
        .as_object_mut()
        .expect("Core status result must be an object")
        .remove("base");
    let mut adapter_domain = through_mcp.response_value;
    adapter_domain
        .as_object_mut()
        .expect("MCP status result must be an object")
        .remove("base");

    assert_eq!(adapter_domain, direct_domain);
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
        json!({ "detail": "workflow", "task_id": null }),
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
    let before_sessions = read_only_table_count(&fixture, "host_sessions")?;
    let _guard = make_project_state_readonly(&fixture)?;

    let response = adapter.call_tool(
        AgentToolId::STATUS.wire_name(),
        json!({ "detail": "full", "task_id": null }),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "result");
    assert_eq!(read_only_state_version(&fixture)?, before_version);
    assert_eq!(
        read_only_table_count(&fixture, "host_sessions")?,
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
            json!({ "operation_result_ref": operation_result_ref.clone(), "cursor": null }),
        ),
    ])?);
    let mut output = Vec::new();

    run_stdio(read_only_adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let result = &responses[1]["result"];
    let structured = &result["structuredContent"];
    assert_eq!(
        result["isError"],
        false,
        "{}",
        serde_json::to_string_pretty(&responses)?
    );
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
    set_connection_enabled(&fixture.mutation_context()?, fixture.connection_id(), false)?;
    let disabled = stale_adapter
        .call_tool(
            AgentToolId::GET_OPERATION_RESULT.wire_name(),
            json!({ "operation_result_ref": operation_result_ref, "cursor": null }),
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
    let (task_id, _, _) = create_implementation_task(&fixture)?;
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
    let update_scope_arguments = bind_action_form_arguments(
        &setup_adapter,
        &task_id,
        MethodName::UpdateScope,
        json!({
            "detail": "full",
            "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
            "goal_summary": goal_summary,
            "scope_update": null,
            "scope_boundary": scope_boundary,
            "non_goals": non_goals,
            "acceptance_criteria": acceptance_criteria,
            "autonomy_boundary": autonomy_boundary,
            "change_unit": {
                "operation": "keep_current"
            }
        }),
    )?;
    let omitted = call_stdio(
        AgentToolId::UPDATE_SCOPE.wire_name(),
        update_scope_arguments,
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

    let advanced_arguments = bind_action_form_arguments(
        &setup_adapter,
        &task_id,
        MethodName::UpdateScope,
        json!({
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
    let advanced = call_stdio(AgentToolId::UPDATE_SCOPE.wire_name(), advanced_arguments)?;
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
            "operation_result_ref": operation_result_ref_value.clone(),
            "cursor": null
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
        assert!(chunk.len() <= volicord_types::methods::MAX_OPERATION_RESULT_PAGE_BYTES);
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
        status_structured["method_result"]["authority_receipt"]["state_version"],
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

    let error = adapter
        .call_tool(AgentToolId::INTAKE.wire_name(), intake_args(None))
        .expect_err("read-only storage must remain outside the Core method response");

    assert!(matches!(
        error,
        McpAdapterError::OperationalUnavailable {
            retryable: false,
            reached_core: false,
        }
    ));
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
fn mcp_wire_maps_operational_unavailability_to_the_selected_profile() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-readonly-wire-unavailable")?;
    let adapter = adapter(&fixture)?;
    let _guard = make_project_state_readonly(&fixture)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(2, AgentToolId::INTAKE.wire_name(), intake_args(None)),
    ])?);
    let mut output = Vec::new();

    run_stdio(adapter, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    assert_eq!(responses.len(), 2);
    let result = &responses[1]["result"];
    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "MCP_UNAVAILABLE");
    assert_eq!(
        result["structuredContent"]["tool_name"],
        MethodName::Intake.as_str()
    );
    assert_eq!(result["structuredContent"]["operation"], "store_access");
    assert_eq!(result["structuredContent"]["resource"], "project_store");
    assert_eq!(result["structuredContent"]["retryable"], false);
    assert_eq!(result["structuredContent"]["reached_core"], false);
    assert_eq!(result["structuredContent"]["committed"], false);
    let message = result["content"][0]["text"]
        .as_str()
        .ok_or("MCP unavailability must include bounded compatibility text")?;
    assert!(message.len() <= 512);
    assert!(!message.contains(fixture.runtime_home_path().to_string_lossy().as_ref()));
    Ok(())
}

#[test]
fn mcp_mutation_is_typed_no_effect_while_setup_is_exclusive_and_succeeds_after_release(
) -> Result<(), Box<dyn Error>> {
    let mut fixture = CoreFixture::new("mcp-mutation-setup-busy")?;
    let adapter = adapter(&fixture)?;
    let arguments = intake_args(None);
    let before = fixture.counts()?;
    fixture.release_mutation_admission();
    let setup = TestRuntimeHomeSetup::acquire(fixture.runtime_home_path())?;

    let error = adapter
        .call_tool(AgentToolId::INTAKE.wire_name(), arguments.clone())
        .expect_err("MCP mutation must be rejected before Core while setup is exclusive");
    let McpAdapterError::MutationAdmission(condition) = error else {
        panic!("MCP mutation must return the typed setup condition: {error}");
    };
    assert_eq!(condition.code(), "runtime_home.mutation.setup_in_progress");
    assert_eq!(condition.mutation_domain(), "mcp.tool_call");
    assert_eq!(fixture.counts()?, before);
    drop(setup);

    let committed = adapter.call_tool(AgentToolId::INTAKE.wire_name(), arguments)?;
    assert_eq!(committed.response_value["base"]["response_kind"], "result");
    assert_eq!(fixture.counts()?.state_version, before.state_version + 1);
    Ok(())
}

#[test]
fn artifact_staging_is_no_effect_while_setup_is_exclusive() -> Result<(), Box<dyn Error>> {
    let mut fixture = CoreFixture::new("mcp-artifact-staging-setup-busy")?;
    let adapter = adapter(&fixture)?;
    let (task_id, _, _) = create_implementation_task(&fixture)?;
    let action_form_ref =
        action_form_ref_for_method(&adapter, &task_id, MethodName::StageArtifact)?;
    let before_rows = read_only_table_count(&fixture, "artifact_staging")?;
    let tmp_dir = fixture
        .runtime_home_path()
        .join("projects")
        .join(fixture.project_id())
        .join("artifacts/tmp");
    assert!(!tmp_dir.exists());
    fixture.release_mutation_admission();
    let setup = TestRuntimeHomeSetup::acquire(fixture.runtime_home_path())?;
    let arguments = json!({
        "action_form_ref": action_form_ref,
        "task_id": task_id,
        "display_name": "setup-busy.txt",
        "content_type": "text/plain",
        "redaction_state": "redacted",
        "safe_bytes_or_notice": "must not be staged while setup is exclusive",
        "expected_sha256": null,
        "expected_size_bytes": null,
        "relation_hint": null
    });

    let error = adapter
        .call_tool(AgentToolId::STAGE_ARTIFACT.wire_name(), arguments.clone())
        .expect_err("artifact staging must be rejected before creating a file or row");
    assert!(matches!(error, McpAdapterError::MutationAdmission(_)));
    assert_eq!(
        read_only_table_count(&fixture, "artifact_staging")?,
        before_rows
    );
    assert!(!tmp_dir.exists());
    drop(setup);

    let staged = adapter.call_tool(AgentToolId::STAGE_ARTIFACT.wire_name(), arguments)?;
    assert_eq!(
        staged.response_value["base"]["effect_kind"],
        "staging_created"
    );
    assert_eq!(
        read_only_table_count(&fixture, "artifact_staging")?,
        before_rows + 1
    );
    assert!(tmp_dir.is_dir());
    Ok(())
}

#[test]
fn mcp_initialize_observation_is_no_effect_while_setup_is_exclusive() -> Result<(), Box<dyn Error>>
{
    let mut fixture = CoreFixture::new("mcp-initialize-setup-busy")?;
    let adapter = adapter(&fixture)?;
    let registry = open_registry_database_read_only(registry_db_path(fixture.runtime_home_path()))?;
    let before_sessions: i64 = registry.query_row(
        "SELECT COUNT(*) FROM mcp_runtime_sessions WHERE connection_internal_id = ?1",
        [fixture.connection_id()],
        |row| row.get(0),
    )?;
    drop(registry);
    fixture.release_mutation_admission();
    let setup = TestRuntimeHomeSetup::acquire(fixture.runtime_home_path())?;
    let input = Cursor::new(json_lines(&[initialize_request(1, json!({}))])?);
    let mut output = Vec::new();

    let error = run_stdio(adapter, BufReader::new(input), &mut output)
        .expect_err("initialize observation must return the typed setup condition");
    let McpAdapterError::MutationAdmission(condition) = error else {
        panic!("initialize observation must preserve the typed setup condition: {error}");
    };
    assert_eq!(condition.code(), "runtime_home.mutation.setup_in_progress");
    assert!(output.is_empty());
    let registry = open_registry_database_read_only(registry_db_path(fixture.runtime_home_path()))?;
    let after_sessions: i64 = registry.query_row(
        "SELECT COUNT(*) FROM mcp_runtime_sessions WHERE connection_internal_id = ?1",
        [fixture.connection_id()],
        |row| row.get(0),
    )?;
    assert_eq!(after_sessions, before_sessions);
    drop(setup);
    Ok(())
}

struct GatedMcpInput {
    input: Cursor<Vec<u8>>,
    ready: Option<std::sync::mpsc::Sender<()>>,
    release: std::sync::mpsc::Receiver<()>,
}

impl GatedMcpInput {
    fn wait_for_release(&mut self) -> std::io::Result<()> {
        if let Some(ready) = self.ready.take() {
            ready
                .send(())
                .map_err(|_| std::io::Error::other("MCP input ready receiver was dropped"))?;
            self.release
                .recv()
                .map_err(|_| std::io::Error::other("MCP input release sender was dropped"))?;
        }
        Ok(())
    }
}

impl std::io::Read for GatedMcpInput {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.wait_for_release()?;
        self.input.read(buffer)
    }
}

impl std::io::BufRead for GatedMcpInput {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.wait_for_release()?;
        self.input.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.input.consume(amount);
    }
}

#[test]
fn idle_managed_server_releases_admission_and_tools_list_is_no_effect_during_setup(
) -> Result<(), Box<dyn Error>> {
    let mut fixture = CoreFixture::new("mcp-idle-tools-list-setup-busy")?;
    let server_adapter = adapter(&fixture)?;
    fixture.release_mutation_admission();
    let first = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
    ])?);
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (processed_tx, processed_rx) = std::sync::mpsc::channel();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let second = GatedMcpInput {
        input: Cursor::new(json_lines(&[request(2, "tools/list", json!({}))])?),
        ready: Some(ready_tx),
        release: release_rx,
    };
    let finish = GatedMcpInput {
        input: Cursor::new(Vec::new()),
        ready: Some(processed_tx),
        release: finish_rx,
    };

    let (output, tools_list_before, tools_list_after) =
        std::thread::scope(|scope| -> Result<_, Box<dyn Error>> {
            let server = scope.spawn(move || {
                let mut output = Vec::new();
                let input = std::io::Read::chain(std::io::Read::chain(first, second), finish);
                let result = run_stdio(server_adapter, input, &mut output);
                (result, output)
            });
            ready_rx
                .recv()
                .map_err(|_| "managed MCP server exited before becoming idle")?;
            let setup = TestRuntimeHomeSetup::acquire(fixture.runtime_home_path())?;
            let tools_list_before = runtime_tools_list_observation_count(&fixture)?;
            release_tx
                .send(())
                .map_err(|_| "managed MCP server stopped before tools/list release")?;
            processed_rx
                .recv()
                .map_err(|_| "managed MCP server stopped before processing tools/list")?;
            let tools_list_after = runtime_tools_list_observation_count(&fixture)?;
            drop(setup);
            finish_tx
                .send(())
                .map_err(|_| "managed MCP server stopped before normal shutdown")?;
            let (result, output) = server.join().expect("managed MCP server thread panicked");
            result?;
            Ok((output, tools_list_before, tools_list_after))
        })?;

    let responses = output
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(serde_json::from_slice::<Value>)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0]["id"], 1);
    assert_eq!(responses[1]["id"], 2);
    assert_eq!(responses[1]["error"]["code"], -32000);
    assert_eq!(
        responses[1]["error"]["message"],
        "Runtime Home setup in progress"
    );
    assert!(responses[1]["error"]["data"]
        .as_str()
        .is_some_and(|data| data.contains("mcp.lifecycle_message")));
    assert_eq!(tools_list_after, tools_list_before);

    let retry_adapter = adapter(&fixture)?;
    let mut retry_output = Vec::new();
    run_stdio(
        retry_adapter,
        Cursor::new(json_lines(&[
            initialize_request(3, json!({})),
            initialized_notification(),
            request(4, "tools/list", json!({})),
        ])?),
        &mut retry_output,
    )?;
    let retry_responses = retry_output
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(serde_json::from_slice::<Value>)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(retry_responses.len(), 2);
    assert!(retry_responses
        .iter()
        .any(|response| response["id"] == 4 && response["result"]["tools"].is_array()));
    assert_eq!(
        runtime_tools_list_observation_count(&fixture)?,
        tools_list_before + 1
    );
    Ok(())
}

fn runtime_tools_list_observation_count(fixture: &CoreFixture) -> Result<i64, Box<dyn Error>> {
    let registry = open_registry_database_read_only(registry_db_path(fixture.runtime_home_path()))?;
    Ok(registry.query_row(
        "SELECT COUNT(*)
           FROM mcp_runtime_sessions
          WHERE connection_internal_id = ?1
            AND tools_list_observed_at IS NOT NULL",
        [fixture.connection_id()],
        |row| row.get(0),
    )?)
}

#[cfg(unix)]
#[test]
fn readonly_degraded_user_action_tool_rejects_create_but_allows_exact_resume(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-readonly-user-action-resume")?;
    let adapter = adapter(&fixture)?;
    let (task_id, _, state_version) = create_implementation_task(&fixture)?;
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
    let create_error = adapter
        .call_tool(
            AgentToolId::REQUEST_USER_ACTION.wire_name(),
            product_action_args(&fixture, &task_id, before_version),
        )
        .expect_err("read-only create must remain outside the Core method response");
    assert!(matches!(
        create_error,
        McpAdapterError::OperationalUnavailable {
            retryable: false,
            reached_core: false,
        }
    ));

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

    let response = adapter.call_tool("volicord.status", json!({"task_id": null}))?;

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
        tools_call(2, AgentToolId::STATUS.wire_name(), json!({"task_id": null})),
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
                "task_id": null,
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
fn stdio_pending_user_action_returns_cli_inbox_recovery() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-cli-inbox-recovery")?;
    let (task_id, _, state_version) = create_implementation_task(&fixture)?;
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

    run_managed_stdio_with_test_lease(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
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
    let request_id = summary["user_action_request_id"]
        .as_str()
        .expect("pending summary request id");
    let expected_command = volicord_user_action_presentation::cli_resolution_path_command(
        &volicord_types::ids::UserActionRequestId::new(request_id),
    )?;
    assert!(fallback.contains(&format!("`{expected_command}`")));
    assert!(!fallback.contains("request.operation=resume"));
    assert!(values[1]["result"].get("_meta").is_none());
    let diagnostics = read_diagnostic_session(fixture.runtime_home_path(), None)?
        .expect("CLI fallback should create bounded diagnostics");
    assert_eq!(diagnostics.fallback_counts["cli_inbox"], 1);
    Ok(())
}

#[test]
fn awaiting_user_action_presentation_uses_the_canonical_user_channel() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-awaiting-user-action-presentation")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let action_form_ref = current_action_form_ref(&setup_adapter, &task_id)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
            json!({
                "detail": "workflow",
                "action_form_ref": action_form_ref,
                "task_id": task_id,
                "checkpoint_operation": {"operation": "create_initial"},
                "scope_revision": 0,
                "baseline_ref": null,
                "summary": "A current user-owned technical decision is required.",
                "implementation_boundary": "Proceed only after the current User Channel decision.",
                "gaps": [{
                    "gap_kind": "user_technical_decision_required",
                    "summary": "Choose the current technical direction.",
                    "affected_refs": [],
                    "user_action": {
                        "action": {
                            "action_type": "choice",
                            "judgment_kind": "technical_decision",
                            "presentation": "short",
                            "question": "Which current technical direction should be used?",
                            "options": [{
                                "option_id": "first",
                                "label": "First direction",
                                "description": "Use the first bounded direction.",
                                "consequence": "The first direction becomes current.",
                                "is_default": true
                            }, {
                                "option_id": "second",
                                "label": "Second direction",
                                "description": "Use the second bounded direction.",
                                "consequence": "The second direction becomes current.",
                                "is_default": false
                            }],
                            "context": {
                                "summary": "The current shaping boundary needs a user-owned decision.",
                                "related_refs": [],
                                "artifact_refs": [],
                                "visible_risks": [],
                                "constraints": []
                            },
                            "affected_refs": [{
                                "record_kind": "task",
                                "record_id": task_id,
                                "project_id": fixture.project_id(),
                                "task_id": task_id,
                                "produced_at_state_version": state_version
                            }],
                            "sensitive_action_scope": null
                        },
                        "expires_at": null
                    }
                }],
                "source_refs": [],
                "evidence_refs": []
            }),
        ),
    ])?);
    let mut output = Vec::new();
    run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let result = &responses[1]["result"];
    assert_eq!(result["isError"], false);
    let structured = &result["structuredContent"];
    assert_eq!(
        structured["workflow"]["kind"], "awaiting_user_action",
        "unexpected shaping projection: {structured:#}"
    );
    assert_workflow_catalog_matches_allowed_actions(&structured["workflow"]);
    let presentation = &structured["presentation"];
    assert_eq!(presentation["state_change"], "core_committed");
    assert_eq!(presentation["next_actor"], "user");
    assert_eq!(presentation["required_user_action"]["channel_kind"], "cli");
    assert_eq!(
        presentation["required_user_action"]["list_command"],
        format!("volicord inbox --task {task_id} --json")
    );
    assert_eq!(
        presentation["required_user_action"]["chat_reply_is_resolution"],
        false
    );
    assert_eq!(
        presentation["required_user_action"]["request_refs"],
        structured["workflow"]["checkpoint"]["pending_decision_refs"]
    );
    let must_surface = presentation["must_surface"]
        .as_array()
        .expect("pending UserAction presentation must carry mandatory facts");
    for fact_kind in [
        "user_action_request_exists",
        "next_actor_is_user",
        "chat_reply_is_not_resolution",
        "product_repository_mutation_blocked_until_user_channel_resolution",
    ] {
        assert!(must_surface
            .iter()
            .any(|fact| fact["fact_kind"] == fact_kind));
    }
    let before_blocked_mutation = fixture.counts()?;
    let blocked = setup_adapter
        .call_tool(
            AgentToolId::STAGE_ARTIFACT.wire_name(),
            json!({
                "action_form_ref": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                "task_id": task_id,
                "display_name": "must-not-stage.txt",
                "content_type": "text/plain",
                "redaction_state": "none",
                "safe_bytes_or_notice": "No staging is allowed while the user owns the next action."
            }),
        )
        .expect_err("awaiting_user_action must reject Agent-owned Task mutations");
    let blocked = structured_tool_error(AgentToolId::STAGE_ARTIFACT.wire_name(), &blocked);
    assert_eq!(blocked["code"], "WORKFLOW_ACTION_NOT_ALLOWED");
    assert_eq!(blocked["reached_core"], false);
    assert_eq!(blocked["committed"], false);
    assert_eq!(
        blocked["workflow_admission"]["current_workflow_kind"],
        "awaiting_user_action"
    );
    assert_eq!(
        blocked["workflow_admission"]["required_method"],
        "volicord.resolve_user_action"
    );
    assert_eq!(blocked["workflow_admission"]["allowed_methods"], json!([]));
    assert_eq!(
        blocked["workflow_admission"]["required_method_forms"],
        json!([])
    );
    assert_eq!(fixture.counts()?, before_blocked_mutation);
    Ok(())
}

#[test]
fn rejected_shaping_decision_presentation_denies_authority_and_names_recovery(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-rejected-shaping-decision-presentation")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let action_form_ref = current_action_form_ref(&setup_adapter, &task_id)?;
    let shaped = setup_adapter.call_tool(
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
        json!({
            "action_form_ref": action_form_ref,
            "task_id": task_id,
                "checkpoint_operation": {"operation": "create_initial"},
                "scope_revision": 0,
                "baseline_ref": null,
                "summary": "One current scope decision controls the implementation boundary.",
                "implementation_boundary": "Proceed only with accepted current scope authority.",
                "gaps": [{
                    "gap_kind": "user_scope_decision_required",
                    "summary": "Confirm the bounded scope authority.",
                    "affected_refs": [],
                    "user_action": {
                        "action": {
                            "action_type": "choice",
                            "judgment_kind": "scope_decision",
                            "presentation": "short",
                            "question": "Accept the bounded scope authority?",
                            "options": null,
                            "context": {
                                "summary": "The implementation boundary needs an explicit scope decision.",
                                "related_refs": [],
                                "artifact_refs": [],
                                "visible_risks": [],
                                "constraints": []
                            },
                            "affected_refs": [{
                                "record_kind": "task",
                                "record_id": task_id,
                                "project_id": fixture.project_id(),
                                "task_id": task_id,
                                "produced_at_state_version": state_version
                            }],
                            "sensitive_action_scope": null
                        },
                        "expires_at": null
                    }
                }],
                "source_refs": [],
                "evidence_refs": []
        }),
    )?;
    let user_action_request_id = shaped.response_value["created_user_action_request_refs"][0]
        ["record_id"]
        .as_str()
        .ok_or("shaping request id")?;
    let context = fixture.mutation_context()?;
    let core = CoreService::for_mutation(&context);
    let resolved = core.resolve_user_action(
        &context,
        fixture.resolve_user_action_request(ResolveUserActionFixture {
            request_id: "req_mcp_rejected_shaping_decision",
            task_id: &task_id,
            user_action_request_id,
            channel_submission_id: "submission_mcp_rejected_shaping_decision",
            resolution: volicord_types::schema::UserActionResolutionInput::Choice {
                selected_option_id: volicord_types::ids::UserActionOptionId::new("reject"),
                note: None.into(),
            },
        }),
        InvocationContext::local_user(
            ProjectId::new(fixture.project_id()),
            OperationCategory::UserOnly,
            volicord_types::values::UserActionChannelKind::Cli,
        ),
    )?;
    assert_eq!(
        resolved.response_value["state"]["workflow"]["kind"],
        "decision_recovery_required"
    );

    let before_rejected_application = fixture.counts()?;
    let input = Cursor::new(json_lines(&[
        initialize_request(3, json!({})),
        initialized_notification(),
        tools_call(
            4,
            AgentToolId::UPDATE_SCOPE.wire_name(),
            json!({
                "task_id": task_id,
                "baseline_ref": "baseline_rejected_shaping_decision",
                "change_unit": {
                    "operation": "create_current",
                    "scope_summary": "This update must not consume rejected authority.",
                    "affected_paths": ["src/current.rs"]
                }
            }),
        ),
    ])?);
    let mut output = Vec::new();
    run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;
    let responses = stdio_responses(&output)?;
    let result = &responses[1]["result"];
    assert_eq!(result["isError"], true);
    let structured = &result["structuredContent"];
    assert_eq!(structured["code"], "WORKFLOW_ACTION_NOT_ALLOWED");
    assert_eq!(structured["reached_core"], false);
    assert_eq!(structured["committed"], false);
    assert_eq!(
        structured["workflow_admission"]["current_workflow_kind"],
        "decision_recovery_required"
    );
    assert_eq!(
        structured["workflow_admission"]["called_method"],
        "volicord.update_scope"
    );
    assert_eq!(
        structured["workflow_admission"]["required_method"],
        "volicord.record_shaping_checkpoint"
    );
    assert_eq!(
        structured["workflow_admission"]["called_method_form"],
        Value::Null
    );
    assert_eq!(
        structured["workflow_admission"]["required_method_forms"][0]["method"],
        "volicord.record_shaping_checkpoint"
    );
    assert_eq!(fixture.counts()?, before_rejected_application);
    Ok(())
}

#[test]
fn product_and_technical_only_shaping_outputs_do_not_fabricate_scope_gaps(
) -> Result<(), Box<dyn Error>> {
    for (label, gap_kind, judgment_kind) in [
        (
            "product",
            "user_product_decision_required",
            "product_decision",
        ),
        (
            "technical",
            "user_technical_decision_required",
            "technical_decision",
        ),
    ] {
        let fixture = CoreFixture::new(&format!("mcp-{label}-only-shaping"))?;
        let adapter = adapter(&fixture)?;
        let (task_id, _) = create_task(&adapter)?;
        let action_form_ref = current_action_form_ref(&adapter, &task_id)?;
        let shaped = adapter.call_tool(
            AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
            json!({
                "action_form_ref": action_form_ref,
                "task_id": task_id,
                    "checkpoint_operation": {"operation": "create_initial"},
                    "scope_revision": 0,
                    "baseline_ref": null,
                    "summary": format!("One {label}-owned decision is required."),
                    "implementation_boundary": "Proceed only after the exact User Channel decision.",
                    "gaps": [{
                        "gap_kind": gap_kind,
                        "summary": format!("Choose the current {label} direction."),
                        "affected_refs": [],
                        "user_action": {
                            "action": {
                                "action_type": "choice",
                                "judgment_kind": judgment_kind,
                                "presentation": "short",
                                "question": format!("Which current {label} direction should be used?"),
                                "options": [{
                                    "option_id": "accept",
                                    "label": "Accept current direction",
                                    "description": "Accept the bounded current direction.",
                                    "consequence": "Only this exact decision is resolved.",
                                    "is_default": true
                                }, {
                                    "option_id": "revise",
                                    "label": "Revise current direction",
                                    "description": "Request a bounded revision.",
                                    "consequence": "Only this exact decision is resolved with revision.",
                                    "is_default": false
                                }],
                                "context": {
                                    "summary": "The current shaping boundary needs one user-owned decision.",
                                    "related_refs": [],
                                    "artifact_refs": [],
                                    "visible_risks": [],
                                    "constraints": []
                                },
                                "affected_refs": [],
                                "sensitive_action_scope": null
                            },
                            "expires_at": null
                        }
                    }],
                    "source_refs": [],
                    "evidence_refs": []
            }),
        )?;
        let gaps = shaped.response_value["workflow"]["checkpoint"]["gaps"]
            .as_array()
            .expect("checkpoint gaps");
        assert_eq!(gaps.len(), 1, "{label}");
        assert_eq!(gaps[0]["gap_kind"], gap_kind, "{label}");
        assert!(gaps
            .iter()
            .all(|gap| gap["gap_kind"] != "user_scope_decision_required"));
        assert_eq!(
            shaped.response_value["created_user_action_request_refs"]
                .as_array()
                .map(Vec::len),
            Some(1),
            "{label}"
        );
    }
    Ok(())
}

#[test]
fn advisor_close_guidance_names_finalize_advice_and_never_record_run() -> Result<(), Box<dyn Error>>
{
    let fixture = CoreFixture::new("mcp-advisor-close-guidance")?;
    let adapter = adapter(&fixture)?;
    let mut intake = intake_args(None);
    intake["requested_mode"] = json!("advisor");
    intake["initial_scope"]["acceptance_criteria"][0]["evidence_requirement"] =
        json!("not_required");
    let intake = adapter.call_tool(AgentToolId::INTAKE.wire_name(), intake)?;
    let task_id = intake.response_value["task_ref"]["record_id"]
        .as_str()
        .ok_or("advisor intake Task")?;
    let context = fixture.mutation_context()?;
    let mut scope_request = fixture.update_scope_request(UpdateScopeFixture {
        request_id: "req_mcp_advisor_guidance_scope",
        idempotency_key: "idem_mcp_advisor_guidance_scope",
        dry_run: false,
        expected_state_version: Some(1),
        task_id,
        operation: ChangeUnitOperation::CreateCurrent,
        scope_summary: "Read-only advisor guidance boundary.",
    });
    scope_request
        .change_unit
        .fields
        .insert("affected_paths".to_owned(), json!([]));
    scope_request.change_unit.effect_contract =
        Some(volicord_types::schema::ChangeUnitEffectContract {
            allowed_effects: vec![
                volicord_types::values::ChangeUnitEffectKind::ArtifactRegistration,
                volicord_types::values::ChangeUnitEffectKind::UserActionRequest,
                volicord_types::values::ChangeUnitEffectKind::EvidenceUpdate,
            ],
            forbidden_effects: vec![
                volicord_types::values::ChangeUnitEffectKind::ProductFileWrite,
                volicord_types::values::ChangeUnitEffectKind::RunRecording,
                volicord_types::values::ChangeUnitEffectKind::SensitiveAction,
                volicord_types::values::ChangeUnitEffectKind::ExternalNetwork,
                volicord_types::values::ChangeUnitEffectKind::SecretAccess,
            ],
            allowed_paths: Vec::new(),
            expected_outputs: vec!["Advice result".to_owned()],
            invariants: vec!["Observe only".to_owned()],
            evidence_expectations: Vec::new(),
            sensitive_action_expectations: Vec::new(),
        });
    let scope = CoreService::for_mutation(&context).update_scope(
        &context,
        scope_request,
        test_agent_invocation(&fixture, OperationCategory::AgentWorkflow),
    )?;
    assert_eq!(scope.response_value["base"]["response_kind"], "result");
    let change_unit_id = scope.response_value["state"]["active_change_unit_ref"]["record_id"]
        .as_str()
        .ok_or("advisor scope should expose its Change Unit")?;
    let action_form_ref = current_action_form_ref(&adapter, task_id)?;
    let shaped = adapter.call_tool(
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
        json!({
            "action_form_ref": action_form_ref,
            "task_id": task_id,
                "checkpoint_operation": {"operation": "create_initial"},
                "scope_revision": 1,
                "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
                "summary": "The bounded advice is ready to finalize.",
                "implementation_boundary": "Provide advice without repository mutation.",
                "gaps": [],
                "source_refs": [],
                "evidence_refs": []
        }),
    )?;
    assert_eq!(
        shaped.response_value["workflow"]["kind"],
        "ready_to_finalize_advice"
    );
    let checkpoint_id = shaped.response_value["shaping_checkpoint"]["shaping_checkpoint_id"]
        .as_str()
        .ok_or("advisor shaping should expose its checkpoint")?;

    let guidance = serde_json::to_string(&shaped.response_value["workflow"])?;
    assert!(guidance.contains("volicord.finalize_advice"));
    assert!(!guidance.contains("volicord.record_run"));

    let finalize_action_form_ref = current_action_form_ref(&adapter, task_id)?;
    let finalized = adapter.call_tool(
        AgentToolId::FINALIZE_ADVICE.wire_name(),
        json!({
            "action_form_ref": finalize_action_form_ref,
            "task_id": task_id,
            "shaping_checkpoint_id": checkpoint_id,
            "change_unit_id": change_unit_id,
            "scope_revision": 1,
            "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
            "user_action_resolution_ids": [],
            "result_summary": "The bounded advisory result is complete.",
            "result_refs": [],
            "evidence_refs": [],
            "residual_risks": [],
            "recovery_constraints": []
        }),
    )?;
    assert_eq!(finalized.response_value["workflow"]["kind"], "close_review");
    assert_workflow_catalog_matches_allowed_actions(&finalized.response_value["workflow"]);

    let before_disallowed = fixture.counts()?;
    let disallowed = adapter
        .call_tool(
            AgentToolId::STAGE_ARTIFACT.wire_name(),
            json!({
                "action_form_ref": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                "task_id": task_id,
                "display_name": "must-not-stage.txt",
                "content_type": "text/plain",
                "redaction_state": "none",
                "safe_bytes_or_notice": "Close review does not admit artifact staging."
            }),
        )
        .expect_err("close_review must reject a method absent from its action catalog");
    let disallowed = structured_tool_error(AgentToolId::STAGE_ARTIFACT.wire_name(), &disallowed);
    assert_eq!(disallowed["code"], "WORKFLOW_ACTION_NOT_ALLOWED");
    assert_eq!(disallowed["reached_core"], false);
    assert_eq!(disallowed["committed"], false);
    assert_eq!(
        disallowed["workflow_admission"]["current_workflow_kind"],
        "close_review"
    );
    assert_eq!(fixture.counts()?, before_disallowed);

    let before_omission = fixture.counts()?;
    let omitted = adapter
        .call_tool(
            AgentToolId::CHECK_CLOSE.wire_name(),
            json!({"task_id": task_id}),
        )
        .expect_err("current close review must require its exact action form");
    let omitted = structured_error_result(&tool_execution_error_result(
        AgentToolId::CHECK_CLOSE.wire_name(),
        &omitted,
    ));
    assert_eq!(omitted["code"], "MCP_ACTION_FORM_STALE");
    assert_eq!(omitted["reached_core"], false);
    assert_eq!(fixture.counts()?, before_omission);

    let check_close_action_form_ref = current_action_form_ref(&adapter, task_id)?;
    let close = adapter.call_tool(
        AgentToolId::CHECK_CLOSE.wire_name(),
        json!({
            "action_form_ref": check_close_action_form_ref,
            "task_id": task_id
        }),
    )?;
    assert_eq!(close.response_value["base"]["effect_kind"], "read_only");
    Ok(())
}

#[test]
fn rejected_mutation_compact_output_reports_no_effect_and_exact_recovery(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-workflow-rejection-presentation")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, state_version) = create_task(&setup_adapter)?;
    let before = fixture.counts()?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::PREPARE_WRITE.wire_name(),
            json!({
                "task_id": task_id,
                "change_unit_id": null,
                "intended_operation": "Attempt a write before workflow recovery.",
                "intended_paths": ["src/current.rs"],
                "product_file_write_intended": true,
                "sensitive_categories": [],
                "baseline_ref": "baseline_current"
            }),
        ),
    ])?);
    let mut output = Vec::new();
    run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let result = &responses[1]["result"];
    assert_eq!(result["isError"], true);
    let structured = &result["structuredContent"];
    assert_eq!(structured["code"], "WORKFLOW_ACTION_NOT_ALLOWED");
    assert_eq!(structured["reached_core"], false);
    assert_eq!(structured["committed"], false);
    assert_eq!(
        structured["workflow_admission"]["called_method"],
        "volicord.prepare_write"
    );
    assert_eq!(
        structured["workflow_admission"]["current_workflow_kind"],
        "shaping_required"
    );
    assert_eq!(
        structured["workflow_admission"]["required_method"],
        "volicord.record_shaping_checkpoint"
    );
    assert_eq!(
        structured["authoritative_context"]["state_version"],
        state_version
    );
    let text = result["content"][0]["text"]
        .as_str()
        .expect("rejection compatibility text");
    assert!(text.contains("not current"));
    assert!(text.contains("Core was not reached"));
    assert!(text.contains("state did not change"));
    assert_eq!(fixture.counts()?, before);
    Ok(())
}

#[test]
fn phase_transition_presentation_denies_implicit_write_authority() -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-phase-transition-presentation")?;
    let setup_adapter = adapter(&fixture)?;
    let (task_id, _) = create_task(&setup_adapter)?;
    let context = fixture.mutation_context()?;
    let scope = CoreService::for_mutation(&context).update_scope(
        &context,
        fixture.update_scope_request(UpdateScopeFixture {
            request_id: "req_mcp_phase_transition_scope",
            idempotency_key: "idem_mcp_phase_transition_scope",
            dry_run: false,
            expected_state_version: Some(1),
            task_id: &task_id,
            operation: ChangeUnitOperation::CreateCurrent,
            scope_summary: "Current phase-transition boundary.",
        }),
        test_agent_invocation(&fixture, OperationCategory::AgentWorkflow),
    )?;
    let change_unit_id = scope.response_value["change_unit_ref"]["record_id"]
        .as_str()
        .ok_or("scope should expose the current Change Unit")?;
    let checkpoint_action_form_ref = current_action_form_ref(&setup_adapter, &task_id)?;
    let shaped = setup_adapter.call_tool(
        AgentToolId::RECORD_SHAPING_CHECKPOINT.wire_name(),
        json!({
            "action_form_ref": checkpoint_action_form_ref,
            "task_id": task_id,
            "checkpoint_operation": {"operation": "create_initial"},
            "scope_revision": 1,
            "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
            "summary": "The current implementation boundary is ready.",
            "implementation_boundary": "Change only the current scoped path.",
            "gaps": [],
            "source_refs": [],
            "evidence_refs": []
        }),
    )?;
    let checkpoint_id = shaped.response_value["shaping_checkpoint"]["shaping_checkpoint_id"]
        .as_str()
        .ok_or("record_shaping_checkpoint should expose the current checkpoint")?;
    let advance_action_form_ref = current_action_form_ref(&setup_adapter, &task_id)?;
    let input = Cursor::new(json_lines(&[
        initialize_request(1, json!({})),
        initialized_notification(),
        tools_call(
            2,
            AgentToolId::ADVANCE_TASK.wire_name(),
            json!({
                "action_form_ref": advance_action_form_ref,
                "task_id": task_id,
                "shaping_checkpoint_id": checkpoint_id,
                "change_unit_id": change_unit_id,
                "scope_revision": 1,
                "baseline_ref": volicord_test_support::core_fixtures::DEFAULT_BASELINE_REF,
                "user_action_resolution_ids": []
            }),
        ),
    ])?);
    let mut output = Vec::new();
    run_stdio(adapter(&fixture)?, BufReader::new(input), &mut output)?;

    let responses = stdio_responses(&output)?;
    let result = &responses[1]["result"];
    assert_eq!(result["isError"], false);
    let presentation = &result["structuredContent"]["presentation"];
    assert_eq!(presentation["state_change"], "core_committed");
    assert_eq!(presentation["task_phase"]["work_phase"], "implementation");
    let facts = presentation["must_surface"]
        .as_array()
        .expect("phase transition must carry mandatory presentation facts");
    for fact_kind in [
        "entered_implementation",
        "phase_transition_created_no_write_ticket",
        "product_repository_writes_require_prepare_write",
    ] {
        assert!(facts.iter().any(|fact| fact["fact_kind"] == fact_kind));
    }
    let text = result["content"][0]["text"]
        .as_str()
        .expect("phase transition compatibility text");
    assert!(!text.to_ascii_lowercase().contains("write ticket created"));
    assert!(!text.to_ascii_lowercase().contains("task completed"));
    Ok(())
}

#[test]
fn stdio_record_guard_uses_the_cli_inbox_without_projecting_the_private_form(
) -> Result<(), Box<dyn Error>> {
    let fixture = CoreFixture::new("mcp-record-guard-cli-inbox")?;
    install_record_guard(&fixture)?;
    let (task_id, _, state_version) = create_implementation_task(&fixture)?;
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

    run_managed_stdio_with_test_lease(
        project_bound_adapter(&fixture)?,
        BufReader::new(input),
        &mut output,
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
    let request_id = workflow["user_action_request_summary"]["user_action_request_id"]
        .as_str()
        .expect("pending summary request id");
    let expected_command = volicord_user_action_presentation::cli_resolution_path_command(
        &volicord_types::ids::UserActionRequestId::new(request_id),
    )?;
    assert!(fallback_texts
        .iter()
        .any(|text| text.contains("pending UserAction requires the user")));
    assert!(fallback_texts
        .iter()
        .any(|text| text.contains(&format!("`{expected_command}`"))));
    assert!(fallback_texts
        .iter()
        .all(|text| !text.contains("prompt_capture")));
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
    let (task_id, _, state_version) = create_implementation_task(&fixture)?;
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
        let request_id = summary["user_action_request_id"]
            .as_str()
            .expect("pending summary request id");
        let expected_command = volicord_user_action_presentation::cli_resolution_path_command(
            &volicord_types::ids::UserActionRequestId::new(request_id),
        )?;
        assert!(
            tool_result["content"]
                .as_array()
                .is_some_and(|content| content.iter().any(|item| item["text"]
                    .as_str()
                    .is_some_and(|text| text.contains(&format!("`{expected_command}`"))))),
            "{}",
            case.name
        );

        let record = stored_action_record(&fixture, &prepared.task_id, &response)?;
        assert_eq!(
            serde_json::to_value(record.request().action_kind())?,
            json!(case.name),
            "{}: fixture must exercise the intended action kind",
            case.name
        );
        assert!(
            record.resolution().is_none(),
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
        let error = crate::user_action_projection::user_action_tool_output(
            &fixture.mutation_context()?,
            &adapter(&fixture)?,
            response,
        )
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
    let (task_id, _, state_version) = create_implementation_task(&fixture)?;

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

    let context = fixture.mutation_context()?;
    let core = CoreService::for_mutation(&context);
    let resolved = core.resolve_user_action(
        &context,
        fixture.resolve_user_action_request(ResolveUserActionFixture {
            request_id: "req_cli_inbox_resolution",
            task_id: &task_id,
            user_action_request_id: &user_action_request_id,
            channel_submission_id: "submission_cli_inbox_resolution",
            resolution: volicord_types::schema::UserActionResolutionInput::Choice {
                selected_option_id: volicord_types::ids::UserActionOptionId::new("keep"),
                note: Some("This private user note must not enter the MCP projection.".to_owned())
                    .into(),
            },
        }),
        InvocationContext::local_user(
            ProjectId::new(fixture.project_id()),
            OperationCategory::UserOnly,
            volicord_types::values::UserActionChannelKind::Cli,
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
        &context,
        fixture.user_action_request(UserActionFixture {
            request_id: "req_mcp_cross_channel_unrelated_action",
            idempotency_key: "idem_mcp_cross_channel_unrelated_action",
            dry_run: false,
            expected_state_version: Some(resolution_state_version),
            task_id: &task_id,
            change_unit_id: None,
            judgment_kind: volicord_types::values::JudgmentKind::TechnicalDecision,
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
    assert!(!resumed["user_channel_resolution"]
        .to_string()
        .contains("private user note"));
    assert_eq!(
        resume_values[1]["result"]["structuredContent"]["operation_result_ref"],
        origin_operation_result_ref
    );
    assert_eq!(fixture.counts()?, before_resume);
    Ok(())
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
            json!({"detail": "workflow", "task_id": null}),
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
    let before_sessions = read_only_table_count(&fixture, "host_sessions")?;

    let error = adapter
        .call_tool_for_session(
            &fixture.mutation_context()?,
            AgentToolId::STATUS,
            json!({"detail": "workflow", "task_id": null}),
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
        read_only_table_count(&fixture, "host_sessions")?,
        before_sessions
    );
    Ok(())
}
