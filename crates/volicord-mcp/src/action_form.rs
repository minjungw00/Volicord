//! MCP workflow action forms derived from neutral Core-owned transitions.

use serde::Serialize;
use serde_json::{json, Map, Value};
use volicord_mcp_wire::{
    action_form_request_projection, mcp_tool_contract, submitted_action_form_semantic_variant,
    ActionFormRequestProjectionDescriptor, McpActionFormArgumentMismatch,
    McpInputContractValidation, McpWorkflowContractStage, RetryContract, SemanticSchemaDescriptor,
    WorkflowActionForm, WorkflowActionFormCatalog, WorkflowActionInput, MAX_VALIDATION_ISSUES,
};
use volicord_types::canonical::canonical_json_sha256;
use volicord_types::ids::{ProjectId, RequestHash, TaskId};
use volicord_types::schema::{
    JsonObject, RequiredNullable, TransitionAttemptDetails, TransitionDescriptor,
    UserActionChoiceDraft, UserActionDraft, WorkflowActionAuthorityCoordinates, WorkflowActionKey,
    WorkflowCheckpointActionCoordinates, WorkflowProjection,
    WorkflowRecordShapingCheckpointSubmissionContract, WorkflowTransitionSubmissionContract,
    WorkflowUpdateScopeSubmissionContract,
};
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::MethodName;

#[derive(Serialize)]
struct WorkflowActionFormDigestBasis<'a> {
    domain: &'static str,
    project_id: &'a ProjectId,
    task_id: &'a TaskId,
    action_key: WorkflowActionKey,
    expected_state_version: u64,
    fixed_authority_coordinates: &'a WorkflowActionAuthorityCoordinates,
    submission_contract: &'a WorkflowTransitionSubmissionContract,
    fixed_arguments: &'a JsonObject,
    fixed_argument_paths: &'a [String],
    semantic_schema_digest: &'a RequestHash,
    scalar_contract_digest: &'a RequestHash,
    workflow_contract_digest: &'a RequestHash,
    action_form_contract_digest: &'a RequestHash,
}

fn descriptor_owned_inputs(
    descriptor: &SemanticSchemaDescriptor,
    projection: &ActionFormRequestProjectionDescriptor,
) -> Option<Vec<WorkflowActionInput>> {
    fn typed_input(
        descriptor: &SemanticSchemaDescriptor,
        path: &str,
        required: bool,
    ) -> Option<WorkflowActionInput> {
        let contracts = descriptor.field_contracts_at_pointer_pattern(path);
        if contracts.is_empty() {
            return None;
        }
        let semantic_types = contracts
            .iter()
            .map(|contract| contract.semantic_type.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(" | ");
        Some(WorkflowActionInput {
            path: path.to_owned(),
            semantic_type: semantic_types,
            required,
        })
    }

    let mut inputs = Vec::new();
    for authored in projection.required_agent_inputs {
        inputs.push(typed_input(descriptor, authored.path_pattern, true)?);
    }
    for authored in projection.optional_agent_inputs {
        inputs.push(typed_input(descriptor, authored.path_pattern, false)?);
    }
    inputs.sort_by(|left, right| left.path.cmp(&right.path));
    Some(inputs)
}

fn checkpoint_operation(operation: &WorkflowCheckpointActionCoordinates) -> Value {
    match operation {
        WorkflowCheckpointActionCoordinates::CreateInitial => {
            json!({ "operation": "create_initial" })
        }
        WorkflowCheckpointActionCoordinates::ReplaceCurrent {
            current_checkpoint_ref,
            predecessor_checkpoint_ref: _,
            retired_non_authorizing_request_refs,
            carry_forward_application_refs,
            stale_application_refs,
            ..
        } => {
            let stale_authority_actions = stale_application_refs
                .iter()
                .map(|stale_application_ref| {
                    json!({
                        "stale_application_ref": stale_application_ref,
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "operation": "replace_current",
                "expected_current_checkpoint_id": current_checkpoint_ref.record_id.as_str(),
                "retired_non_authorizing_request_refs": retired_non_authorizing_request_refs,
                "carry_forward_application_refs": carry_forward_application_refs,
                "stale_authority_actions": stale_authority_actions,
            })
        }
    }
}

fn project_fixed_arguments(
    coordinates: &WorkflowActionAuthorityCoordinates,
    submission_contract: &WorkflowTransitionSubmissionContract,
) -> Result<(TaskId, JsonObject), String> {
    let mut fixed = Map::new();
    let task_id = match coordinates {
        WorkflowActionAuthorityCoordinates::RecordShapingCheckpoint {
            task_id,
            checkpoint_operation: operation,
            scope_revision,
            baseline_ref,
        } => {
            fixed.insert("task_id".to_owned(), json!(task_id));
            fixed.insert(
                "checkpoint_operation".to_owned(),
                checkpoint_operation(operation),
            );
            fixed.insert("scope_revision".to_owned(), json!(scope_revision));
            fixed.insert("baseline_ref".to_owned(), json!(baseline_ref));
            task_id.clone()
        }
        WorkflowActionAuthorityCoordinates::UpdateScope {
            task_id,
            related_scope_decision_refs,
            selected_change_unit_operation,
            ..
        } => {
            fixed.insert("task_id".to_owned(), json!(task_id));
            fixed.insert(
                "related_scope_decision_refs".to_owned(),
                Value::Array(
                    related_scope_decision_refs
                        .iter()
                        .map(|reference| json!(reference))
                        .collect(),
                ),
            );
            let change_unit = match submission_contract {
                WorkflowTransitionSubmissionContract::UpdateScope {
                    contract:
                        WorkflowUpdateScopeSubmissionContract::AdvisorCreateCurrentChangeUnit {
                            fixed_values,
                            ..
                        }
                        | WorkflowUpdateScopeSubmissionContract::AdvisorReplaceCurrentChangeUnit {
                            fixed_values,
                            ..
                        },
                } => json!({
                    "operation": selected_change_unit_operation,
                    "affected_paths": fixed_values.affected_paths,
                    "effect_contract": fixed_values.effect_contract,
                }),
                WorkflowTransitionSubmissionContract::UpdateScope { .. } => {
                    json!({ "operation": selected_change_unit_operation })
                }
                _ => {
                    return Err(
                        "update-scope coordinates have another submission contract".to_owned()
                    )
                }
            };
            fixed.insert("change_unit".to_owned(), change_unit);
            task_id.clone()
        }
        WorkflowActionAuthorityCoordinates::FinalizeAdvice {
            task_id,
            shaping_checkpoint_id,
            change_unit_id,
            scope_revision,
            baseline_ref,
            user_action_resolution_ids,
        } => {
            fixed.extend([
                ("task_id".to_owned(), json!(task_id)),
                (
                    "shaping_checkpoint_id".to_owned(),
                    json!(shaping_checkpoint_id),
                ),
                ("change_unit_id".to_owned(), json!(change_unit_id)),
                ("scope_revision".to_owned(), json!(scope_revision)),
                ("baseline_ref".to_owned(), json!(baseline_ref)),
                (
                    "user_action_resolution_ids".to_owned(),
                    json!(user_action_resolution_ids),
                ),
            ]);
            task_id.clone()
        }
        WorkflowActionAuthorityCoordinates::AdvanceTask {
            task_id,
            shaping_checkpoint_id,
            change_unit_id,
            scope_revision,
            baseline_ref,
            user_action_resolution_ids,
        } => {
            fixed.extend([
                ("task_id".to_owned(), json!(task_id)),
                (
                    "shaping_checkpoint_id".to_owned(),
                    json!(shaping_checkpoint_id),
                ),
                ("change_unit_id".to_owned(), json!(change_unit_id)),
                ("scope_revision".to_owned(), json!(scope_revision)),
                ("baseline_ref".to_owned(), json!(baseline_ref)),
                (
                    "user_action_resolution_ids".to_owned(),
                    json!(user_action_resolution_ids),
                ),
            ]);
            task_id.clone()
        }
        WorkflowActionAuthorityCoordinates::PrepareEvidenceCapture {
            task_id,
            change_unit_id,
            baseline_ref,
        } => {
            fixed.extend([
                ("task_id".to_owned(), json!(task_id)),
                ("change_unit_id".to_owned(), json!(change_unit_id)),
                ("baseline_ref".to_owned(), json!(baseline_ref)),
            ]);
            task_id.clone()
        }
        WorkflowActionAuthorityCoordinates::PrepareWrite {
            task_id,
            change_unit_id,
            baseline_ref,
        } => {
            fixed.extend([
                ("task_id".to_owned(), json!(task_id)),
                ("change_unit_id".to_owned(), json!(change_unit_id)),
                ("baseline_ref".to_owned(), json!(baseline_ref)),
            ]);
            task_id.clone()
        }
        WorkflowActionAuthorityCoordinates::StageArtifact { task_id } => {
            fixed.insert("task_id".to_owned(), json!(task_id));
            task_id.clone()
        }
        WorkflowActionAuthorityCoordinates::RecordRun {
            task_id,
            change_unit_id,
            baseline_ref,
            ..
        } => {
            fixed.extend([
                ("task_id".to_owned(), json!(task_id)),
                ("change_unit_id".to_owned(), json!(change_unit_id)),
                ("baseline_ref".to_owned(), json!(baseline_ref)),
            ]);
            task_id.clone()
        }
        WorkflowActionAuthorityCoordinates::RequestUserAction {
            task_id,
            change_unit_id,
        } => {
            fixed.insert(
                "request".to_owned(),
                json!({
                    "operation": "create",
                    "task_id": task_id,
                    "change_unit_id": change_unit_id,
                }),
            );
            task_id.clone()
        }
        WorkflowActionAuthorityCoordinates::ResolveUserAction {
            task_id,
            user_action_request_refs,
        } => {
            fixed.insert("task_id".to_owned(), json!(task_id));
            fixed.insert(
                "user_action_request_refs".to_owned(),
                json!(user_action_request_refs),
            );
            task_id.clone()
        }
        WorkflowActionAuthorityCoordinates::ReconcileChanges { task_id } => {
            fixed.insert("task_id".to_owned(), json!(task_id));
            task_id.clone()
        }
        WorkflowActionAuthorityCoordinates::CheckClose { task_id } => {
            fixed.insert("task_id".to_owned(), json!(task_id));
            task_id.clone()
        }
        WorkflowActionAuthorityCoordinates::CloseTask { task_id } => {
            fixed.insert("task_id".to_owned(), json!(task_id));
            task_id.clone()
        }
    };
    Ok((task_id, fixed))
}

fn insert_pointer(request: &mut Value, pointer: &str, value: Value) -> Result<(), String> {
    let (parent_pointer, leaf) = pointer
        .rsplit_once('/')
        .ok_or_else(|| format!("invalid action-form pointer {pointer}"))?;
    let leaf = leaf.replace("~1", "/").replace("~0", "~");
    let parent = request
        .pointer_mut(parent_pointer)
        .ok_or_else(|| format!("action-form pointer parent {parent_pointer} is absent"))?;
    match parent {
        Value::Object(object) => {
            object.insert(leaf, value);
            Ok(())
        }
        Value::Array(array) => {
            let index = leaf
                .parse::<usize>()
                .map_err(|_| format!("action-form array pointer {pointer} is invalid"))?;
            let slot = array
                .get_mut(index)
                .ok_or_else(|| format!("action-form array pointer {pointer} is absent"))?;
            *slot = value;
            Ok(())
        }
        _ => Err(format!(
            "action-form pointer parent {parent_pointer} is not a container"
        )),
    }
}

fn submission_witnesses(contract: &WorkflowTransitionSubmissionContract) -> (Value, Value) {
    let empty = || Value::Object(Map::new());
    match contract {
        WorkflowTransitionSubmissionContract::RecordShapingCheckpoint { contract } => {
            let (required, optional) = match contract {
                WorkflowRecordShapingCheckpointSubmissionContract::CreateInitial {
                    required_agent_input_witness,
                    optional_agent_input_witness,
                }
                | WorkflowRecordShapingCheckpointSubmissionContract::ReplaceCurrent {
                    required_agent_input_witness,
                    optional_agent_input_witness,
                } => (required_agent_input_witness, optional_agent_input_witness),
            };
            (
                json!({
                    "summary": required.summary,
                    "implementation_boundary": required.implementation_boundary,
                    "gaps": required.gaps,
                    "checkpoint_operation": {
                        "stale_authority_actions": required
                            .stale_authority_actions
                            .iter()
                            .map(|_| json!({"action": "retire"}))
                            .collect::<Vec<_>>(),
                    },
                }),
                json!({
                    "source_refs": optional.source_refs,
                    "evidence_refs": optional.evidence_refs,
                }),
            )
        }
        WorkflowTransitionSubmissionContract::UpdateScope { contract } => {
            let common = match contract {
                WorkflowUpdateScopeSubmissionContract::KeepCurrentChangeUnit {
                    required_agent_input_witness,
                    ..
                }
                | WorkflowUpdateScopeSubmissionContract::GeneralCreateCurrentChangeUnit {
                    required_agent_input_witness,
                    ..
                }
                | WorkflowUpdateScopeSubmissionContract::GeneralReplaceCurrentChangeUnit {
                    required_agent_input_witness,
                    ..
                }
                | WorkflowUpdateScopeSubmissionContract::AdvisorCreateCurrentChangeUnit {
                    required_agent_input_witness,
                    ..
                }
                | WorkflowUpdateScopeSubmissionContract::AdvisorReplaceCurrentChangeUnit {
                    required_agent_input_witness,
                    ..
                } => required_agent_input_witness,
            };
            let mut required = json!({
                "goal_summary": common.goal_summary,
                "scope_update": common.scope_update,
                "scope_boundary": common.scope_boundary,
                "non_goals": common.non_goals,
                "acceptance_criteria": common.acceptance_criteria,
                "autonomy_boundary": common.autonomy_boundary,
                "baseline_ref": common.baseline_ref,
                "change_unit": {},
            });
            let mut optional = json!({"change_unit": {}});
            match contract {
                WorkflowUpdateScopeSubmissionContract::KeepCurrentChangeUnit { .. } => {}
                WorkflowUpdateScopeSubmissionContract::GeneralCreateCurrentChangeUnit {
                    required_change_unit_witness,
                    optional_agent_input_witness,
                    ..
                }
                | WorkflowUpdateScopeSubmissionContract::GeneralReplaceCurrentChangeUnit {
                    required_change_unit_witness,
                    optional_agent_input_witness,
                    ..
                } => {
                    required["change_unit"] = json!({
                        "scope_summary": required_change_unit_witness.scope_summary,
                        "affected_paths": required_change_unit_witness.affected_paths,
                    });
                    optional["change_unit"] = json!({
                        "affected_areas": optional_agent_input_witness.affected_areas,
                        "constraints": optional_agent_input_witness.constraints,
                        "effect_contract": optional_agent_input_witness.effect_contract,
                    });
                }
                WorkflowUpdateScopeSubmissionContract::AdvisorCreateCurrentChangeUnit {
                    required_change_unit_witness,
                    optional_agent_input_witness,
                    ..
                }
                | WorkflowUpdateScopeSubmissionContract::AdvisorReplaceCurrentChangeUnit {
                    required_change_unit_witness,
                    optional_agent_input_witness,
                    ..
                } => {
                    required["change_unit"] = json!({
                        "scope_summary": required_change_unit_witness.scope_summary,
                    });
                    optional["change_unit"] = json!({
                        "affected_areas": optional_agent_input_witness.affected_areas,
                        "constraints": optional_agent_input_witness.constraints,
                    });
                }
            }
            (required, optional)
        }
        WorkflowTransitionSubmissionContract::FinalizeAdvice {
            required_agent_input_witness,
            optional_agent_input_witness,
        } => (
            json!({"result_summary": required_agent_input_witness.result_summary}),
            json!({
                "result_refs": optional_agent_input_witness.result_refs,
                "evidence_refs": optional_agent_input_witness.evidence_refs,
                "residual_risks": optional_agent_input_witness.residual_risks,
                "recovery_constraints": optional_agent_input_witness.recovery_constraints,
            }),
        ),
        WorkflowTransitionSubmissionContract::AdvanceTask { .. }
        | WorkflowTransitionSubmissionContract::ResolveUserAction { .. }
        | WorkflowTransitionSubmissionContract::CheckClose { .. } => (empty(), empty()),
        WorkflowTransitionSubmissionContract::PrepareEvidenceCapture {
            required_agent_input_witness,
            ..
        } => (
            json!({
                "target": required_agent_input_witness.target,
                "capture": required_agent_input_witness.capture,
            }),
            empty(),
        ),
        WorkflowTransitionSubmissionContract::PrepareWrite {
            required_agent_input_witness,
            optional_agent_input_witness,
        } => (
            json!({
                "intended_operation": required_agent_input_witness.intended_operation,
                "intended_paths": required_agent_input_witness.intended_paths,
                "product_file_write_intended": required_agent_input_witness.product_file_write_intended,
            }),
            json!({"sensitive_categories": optional_agent_input_witness.sensitive_categories}),
        ),
        WorkflowTransitionSubmissionContract::StageArtifact {
            required_agent_input_witness,
            ..
        } => (
            json!({
                "display_name": required_agent_input_witness.display_name,
                "content_type": required_agent_input_witness.content_type,
                "redaction_state": required_agent_input_witness.redaction_state,
                "safe_bytes_or_notice": required_agent_input_witness.safe_bytes_or_notice,
                "expected_sha256": required_agent_input_witness.expected_sha256,
                "expected_size_bytes": required_agent_input_witness.expected_size_bytes,
                "relation_hint": required_agent_input_witness.relation_hint,
            }),
            empty(),
        ),
        WorkflowTransitionSubmissionContract::RecordRun {
            required_agent_input_witness,
            optional_agent_input_witness,
        } => (
            json!({
                "kind": required_agent_input_witness.kind,
                "run_id": required_agent_input_witness.run_id,
                "write_ticket_id": required_agent_input_witness.write_ticket_id,
                "performed_operation": required_agent_input_witness.performed_operation,
                "summary": required_agent_input_witness.summary,
                "observed_changes": required_agent_input_witness.observed_changes,
                "close_assessment": required_agent_input_witness.close_assessment,
            }),
            json!({
                "artifact_inputs": optional_agent_input_witness.artifact_inputs,
                "evidence_updates": optional_agent_input_witness.evidence_updates,
                "evidence_observations": optional_agent_input_witness.evidence_observations,
            }),
        ),
        WorkflowTransitionSubmissionContract::RequestUserAction {
            required_agent_input_witness,
            ..
        } => (
            json!({
                "request": {
                    "action": UserActionDraft::Choice(Box::new(UserActionChoiceDraft {
                        judgment_kind: required_agent_input_witness.choice.judgment_kind,
                        presentation: required_agent_input_witness.choice.presentation,
                        question: required_agent_input_witness.choice.prompt.clone(),
                        options: required_agent_input_witness.choice.options.clone(),
                        context: required_agent_input_witness.choice.context.clone(),
                        affected_refs: required_agent_input_witness.choice.affected_refs.clone(),
                        sensitive_action_scope: RequiredNullable::null(),
                    })),
                    "required_for": required_agent_input_witness.required_for,
                    "expires_at": required_agent_input_witness.expires_at,
                },
            }),
            empty(),
        ),
        WorkflowTransitionSubmissionContract::ReconcileChanges {
            optional_agent_input_witness,
            ..
        } => (
            empty(),
            json!({"resolution_requests": optional_agent_input_witness.resolution_requests}),
        ),
        WorkflowTransitionSubmissionContract::CloseTask {
            required_agent_input_witness,
            ..
        } => (
            json!({
                "intent": required_agent_input_witness.intent,
                "close_reason": required_agent_input_witness.close_reason,
                "superseding_task_id": required_agent_input_witness.superseding_task_id,
                "user_note": required_agent_input_witness.user_note,
            }),
            empty(),
        ),
    }
}

fn copy_submission_witness_input(
    request: &mut Value,
    witness: &Value,
    authored: &WorkflowActionInput,
    required: bool,
) -> Result<(), String> {
    let pattern = authored.path.as_str();
    if let Some((prefix, suffix)) = pattern.split_once("/*") {
        let current_len = request
            .pointer(prefix)
            .and_then(Value::as_array)
            .map(Vec::len)
            .ok_or_else(|| format!("action-form authored wildcard prefix {prefix} is absent"))?;
        let Some(witness_values) = witness.pointer(prefix).and_then(Value::as_array) else {
            return if required {
                Err(format!(
                    "required submission witness prefix {prefix} is absent"
                ))
            } else {
                Ok(())
            };
        };
        for index in 0..current_len {
            let source_pointer = suffix.strip_prefix('/').unwrap_or(suffix);
            let value = witness_values
                .get(index)
                .and_then(|source| source.pointer(&format!("/{source_pointer}")))
                .cloned()
                .ok_or_else(|| format!("submission witness omits authored input {pattern}"))?;
            insert_pointer(request, &format!("{prefix}/{index}{suffix}"), value)?;
        }
        return Ok(());
    }
    let Some(value) = witness.pointer(pattern).cloned() else {
        return if required {
            Err(format!(
                "required submission witness omits authored input {pattern}"
            ))
        } else {
            Ok(())
        };
    };
    insert_pointer(request, pattern, value)
}

fn fixed_leaf_paths(value: &Value, prefix: &str, paths: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                fixed_leaf_paths(value, &format!("{prefix}/{key}"), paths);
            }
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                fixed_leaf_paths(value, &format!("{prefix}/{index}"), paths);
            }
        }
        _ => paths.push(prefix.to_owned()),
    }
}

/// Projects one current neutral transition through the canonical MCP descriptor.
fn workflow_action_form(
    project_id: &ProjectId,
    workflow: &WorkflowProjection,
    transition: &TransitionDescriptor,
) -> Result<Option<ProjectedWorkflowActionForm>, String> {
    if transition.actor != volicord_types::values::WorkflowTransitionActor::Agent {
        return Ok(None);
    }
    let method = transition.action_key.method;
    let tool = AgentToolId::from_method(method)
        .ok_or_else(|| format!("{} has no canonical MCP tool", method.as_str()))?;
    let descriptor = mcp_tool_contract(tool)
        .ok_or_else(|| format!("{} has no semantic request descriptor", method.as_str()))?;
    let semantic_schema_digest = volicord_types::managed_guidance::mcp_semantic_schema_digest();
    let workflow_contract_digest =
        volicord_types::managed_guidance::workflow_contract_semantic_digest();
    let action_form_contract_digest =
        volicord_types::managed_guidance::action_form_contract_semantic_digest();
    let scalar_contract_digest =
        volicord_types::canonical_scalar::baseline_ref_scalar_contract_digest();
    let selected_semantic_variant = transition.action_key.semantic_variant;
    let projection =
        action_form_request_projection(&transition.submission_contract).ok_or_else(|| {
            format!(
                "{} {} has no action-form request projection",
                method.as_str(),
                selected_semantic_variant.as_str()
            )
        })?;
    let (task_id, fixed_arguments) = project_fixed_arguments(
        &transition.fixed_authority_coordinates,
        &transition.submission_contract,
    )?;
    let agent_authored_inputs = descriptor_owned_inputs(descriptor.input_descriptor(), projection)
        .ok_or_else(|| {
            format!(
                "{} authored input descriptor is incomplete",
                method.as_str()
            )
        })?;
    let fixed_argument_paths = projection
        .concrete_fixed_argument_paths(&fixed_arguments)
        .map_err(|detail| format!("action-form fixed arguments are inconsistent: {detail}"))?;
    let fixed = Value::Object(fixed_arguments.clone());
    if fixed_argument_paths.iter().any(|path| {
        fixed.pointer(path).is_none_or(|value| {
            !descriptor
                .input_descriptor()
                .accepts_value_at_pointer(path, value)
        })
    }) {
        return Err(format!(
            "{} fixed value does not match its semantic request type",
            method.as_str()
        ));
    }
    let mut projected_leaf_paths = Vec::new();
    fixed_leaf_paths(&fixed, "", &mut projected_leaf_paths);
    projected_leaf_paths.sort();
    if projected_leaf_paths.iter().any(|path| {
        !fixed_argument_paths
            .iter()
            .any(|fixed_path| path == fixed_path || path.starts_with(&format!("{fixed_path}/")))
    }) {
        return Err(format!(
            "{} fixed arguments contain a value not consumed by its request projection",
            method.as_str()
        ));
    }
    let current = workflow
        .transition_catalog()
        .transition(&transition.action_key)
        .ok_or_else(|| "action-form transition is absent from the current catalog".to_owned())?;
    if current != transition {
        return Err("action-form transition differs from the current descriptor".to_owned());
    }
    let form_ref = canonical_json_sha256(&WorkflowActionFormDigestBasis {
        domain: "volicord.mcp-workflow-action-form",
        project_id,
        task_id: &task_id,
        action_key: transition.action_key,
        expected_state_version: transition.expected_state_version,
        fixed_authority_coordinates: &transition.fixed_authority_coordinates,
        submission_contract: &transition.submission_contract,
        fixed_arguments: &fixed_arguments,
        fixed_argument_paths: &fixed_argument_paths,
        semantic_schema_digest: &semantic_schema_digest,
        scalar_contract_digest: &scalar_contract_digest,
        workflow_contract_digest: &workflow_contract_digest,
        action_form_contract_digest: &action_form_contract_digest,
    })
    .map_err(|error| error.to_string())?;
    let (required_witness, optional_witness) =
        submission_witnesses(&transition.submission_contract);
    let mut canonical_minimal_request = fixed;
    for authored in agent_authored_inputs.iter().filter(|input| input.required) {
        copy_submission_witness_input(
            &mut canonical_minimal_request,
            &required_witness,
            authored,
            true,
        )?;
    }
    canonical_minimal_request
        .as_object_mut()
        .ok_or_else(|| "action-form minimal request is not an object".to_owned())?
        .insert("action_form_ref".to_owned(), json!(form_ref));
    let form = WorkflowActionForm {
        action_key: transition.action_key,
        form_ref,
        expected_state_version: transition.expected_state_version,
        fixed_arguments,
        fixed_argument_paths,
        agent_authored_inputs,
        canonical_minimal_request: canonical_minimal_request
            .as_object()
            .cloned()
            .ok_or_else(|| "action-form minimal request is not an object".to_owned())?,
    };
    match descriptor.validate_and_decode_input(&canonical_minimal_request) {
        McpInputContractValidation::Valid => {}
        McpInputContractValidation::Invalid(validation) => {
            return Err(format!(
                "{} canonical minimal request failed semantic validation at {}",
                method.as_str(),
                validation
                    .issues
                    .first()
                    .map(|issue| issue.path.as_str())
                    .unwrap_or("")
            ));
        }
        McpInputContractValidation::SchemaContractFailure => {
            return Err(format!(
                "{} canonical minimal request failed exact decoding",
                method.as_str()
            ));
        }
    }
    let mut complete_validation_witness = canonical_minimal_request.clone();
    for authored in form
        .agent_authored_inputs
        .iter()
        .filter(|input| !input.required)
    {
        copy_submission_witness_input(
            &mut complete_validation_witness,
            &optional_witness,
            authored,
            false,
        )?;
    }
    match descriptor.validate_and_decode_input(&complete_validation_witness) {
        McpInputContractValidation::Valid => {}
        McpInputContractValidation::Invalid(_) => {
            return Err(format!(
                "{} complete submission witness failed semantic validation",
                method.as_str()
            ));
        }
        McpInputContractValidation::SchemaContractFailure => {
            return Err(format!(
                "{} complete submission witness failed exact decoding",
                method.as_str()
            ));
        }
    }
    if submitted_action_form_semantic_variant(method, &canonical_minimal_request)
        != Some(selected_semantic_variant)
        || submitted_action_form_semantic_variant(method, &complete_validation_witness)
            != Some(selected_semantic_variant)
    {
        return Err(format!(
            "{} canonical minimal request reaches another semantic variant",
            method.as_str()
        ));
    }
    let binding = bind_fixed_arguments(&form, &canonical_minimal_request)?;
    if !binding.mismatches.is_empty() {
        return Err(format!(
            "{} canonical minimal request does not bind every fixed argument",
            method.as_str()
        ));
    }
    let complete_binding = bind_fixed_arguments(&form, &complete_validation_witness)?;
    if !complete_binding.mismatches.is_empty() {
        return Err(format!(
            "{} complete submission witness does not bind every fixed argument",
            method.as_str()
        ));
    }
    Ok(Some(ProjectedWorkflowActionForm {
        form,
        complete_validation_witness,
    }))
}

pub(crate) struct ProjectedWorkflowActionForm {
    pub form: WorkflowActionForm,
    pub complete_validation_witness: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActionFormCatalogError {
    pub action_key: Option<WorkflowActionKey>,
    pub stage: McpWorkflowContractStage,
    pub detail: String,
}

impl std::fmt::Display for ActionFormCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.stage, self.detail)
    }
}

impl std::error::Error for ActionFormCatalogError {}

impl ActionFormCatalogError {
    pub(crate) fn reached_core(&self) -> bool {
        matches!(
            self.stage,
            McpWorkflowContractStage::CorePlanning
                | McpWorkflowContractStage::ExpectedResultValidation
        )
    }
}

pub(crate) struct FixedArgumentBindingResult {
    pub mismatches: Vec<McpActionFormArgumentMismatch>,
    pub truncated: bool,
}

/// Validates every current fixed authority value through one descriptor-driven binder.
pub(crate) fn bind_fixed_arguments(
    form: &WorkflowActionForm,
    submitted: &Value,
) -> Result<FixedArgumentBindingResult, String> {
    let method = form.action_key.method;
    let expected_paths = &form.fixed_argument_paths;

    let fixed = Value::Object(form.fixed_arguments.clone());
    let tool = AgentToolId::from_method(method)
        .ok_or_else(|| format!("{} has no canonical MCP tool", method.as_str()))?;
    let request = mcp_tool_contract(tool)
        .ok_or_else(|| format!("{} has no semantic request descriptor", method.as_str()))?;
    let mut mismatches = Vec::new();
    for path in expected_paths {
        let expected_value = fixed.pointer(path).ok_or_else(|| {
            format!(
                "{} current form omits fixed value {}",
                method.as_str(),
                path
            )
        })?;
        if !request
            .input_descriptor()
            .accepts_value_at_pointer(path, expected_value)
        {
            return Err(format!(
                "{} fixed value at {} does not match its request semantic type",
                method.as_str(),
                path
            ));
        }
        let received = submitted.pointer(path);
        if received == Some(expected_value) {
            continue;
        }
        mismatches.push(McpActionFormArgumentMismatch {
            method,
            form_ref: form.form_ref.clone(),
            path: path.clone(),
            expected_value: expected_value.clone(),
            received_value: received.cloned().unwrap_or(Value::Null),
            received_value_present: received.is_some(),
            state_change_applied: false,
            reached_core: false,
            current_method_form: form.clone(),
        });
    }
    mismatches.sort_by(|left, right| left.path.cmp(&right.path));
    let truncated = mismatches.len() > MAX_VALIDATION_ISSUES;
    mismatches.truncate(MAX_VALIDATION_ISSUES);
    Ok(FixedArgumentBindingResult {
        mismatches,
        truncated,
    })
}

pub(crate) fn workflow_action_form_catalog<F>(
    project_id: &ProjectId,
    workflow: &WorkflowProjection,
    mut validate_plan: F,
) -> Result<WorkflowActionFormCatalog, ActionFormCatalogError>
where
    F: FnMut(&WorkflowActionForm, Value) -> Result<(), (McpWorkflowContractStage, String)>,
{
    let mut forms = Vec::new();
    for transition in &workflow.transition_catalog().transitions {
        if transition.actor != volicord_types::values::WorkflowTransitionActor::Agent {
            continue;
        }
        let projected = workflow_action_form(project_id, workflow, transition)
            .map_err(|detail| ActionFormCatalogError {
                action_key: Some(transition.action_key),
                stage: projection_failure_stage(&detail),
                detail,
            })?
            .ok_or_else(|| ActionFormCatalogError {
                action_key: Some(transition.action_key),
                stage: McpWorkflowContractStage::CatalogTotality,
                detail: format!(
                    "Agent transition {} {} did not produce an MCP action form",
                    transition.action_key.method.as_str(),
                    transition.action_key.semantic_variant.as_str()
                ),
            })?;
        validate_plan(&projected.form, projected.complete_validation_witness).map_err(
            |(stage, detail)| ActionFormCatalogError {
                action_key: Some(transition.action_key),
                stage,
                detail,
            },
        )?;
        forms.push(projected.form);
    }
    validate_action_form_totality(workflow, &forms)?;
    Ok(WorkflowActionFormCatalog {
        required_action_key: RequiredNullable::new(
            workflow
                .transition_catalog()
                .required_transition()
                .filter(|transition| {
                    transition.actor == volicord_types::values::WorkflowTransitionActor::Agent
                })
                .map(|transition| transition.action_key),
        ),
        workflow_contract_digest:
            volicord_types::managed_guidance::workflow_contract_semantic_digest(),
        action_form_contract_digest:
            volicord_types::managed_guidance::action_form_contract_semantic_digest(),
        semantic_schema_digest: volicord_types::managed_guidance::mcp_semantic_schema_digest(),
        scalar_contract_digest:
            volicord_types::canonical_scalar::baseline_ref_scalar_contract_digest(),
        forms,
    })
}

fn projection_failure_stage(detail: &str) -> McpWorkflowContractStage {
    if detail.contains("semantic validation") {
        McpWorkflowContractStage::SemanticValidation
    } else if detail.contains("exact decoding") {
        McpWorkflowContractStage::ExactDecode
    } else if detail.contains("bind") || detail.contains("fixed argument") {
        McpWorkflowContractStage::FixedBinding
    } else if detail.contains("transition") || detail.contains("descriptor") {
        McpWorkflowContractStage::TransitionContract
    } else {
        McpWorkflowContractStage::WitnessProjection
    }
}

fn validate_action_form_totality(
    workflow: &WorkflowProjection,
    forms: &[WorkflowActionForm],
) -> Result<(), ActionFormCatalogError> {
    for transition in workflow
        .transition_catalog()
        .transitions
        .iter()
        .filter(|transition| {
            transition.actor == volicord_types::values::WorkflowTransitionActor::Agent
        })
    {
        if !forms
            .iter()
            .any(|form| form.action_key == transition.action_key)
        {
            return Err(ActionFormCatalogError {
                action_key: Some(transition.action_key),
                stage: McpWorkflowContractStage::CatalogTotality,
                detail: "a current Agent transition has no exact validated MCP action form"
                    .to_owned(),
            });
        }
    }
    if forms.len()
        != workflow
            .transition_catalog()
            .transitions
            .iter()
            .filter(|transition| {
                transition.actor == volicord_types::values::WorkflowTransitionActor::Agent
            })
            .count()
    {
        return Err(ActionFormCatalogError {
            action_key: None,
            stage: McpWorkflowContractStage::CatalogTotality,
            detail: "the MCP action-form catalog is not a one-to-one Agent-transition projection"
                .to_owned(),
        });
    }
    Ok(())
}

pub(crate) fn retry_contract(
    attempted_action_key: WorkflowActionKey,
    recovery_action_key: WorkflowActionKey,
    workflow: &WorkflowProjection,
    catalog: &WorkflowActionFormCatalog,
    attempt_details: TransitionAttemptDetails,
    invalid_or_incompatible_submitted_paths: Vec<String>,
) -> Result<RetryContract, String> {
    let transition = workflow
        .transition_catalog()
        .transition(&recovery_action_key)
        .ok_or_else(|| {
            "Core recovery action is absent from the current transition catalog".to_owned()
        })?;
    let recovery_is_external_close = recovery_action_key.method == MethodName::CloseTask
        && recovery_action_key != attempted_action_key;
    let retry_possible_in_current_task = transition.actor
        == volicord_types::values::WorkflowTransitionActor::Agent
        && !recovery_is_external_close;
    let recovery_form = retry_possible_in_current_task
        .then(|| {
            catalog
                .form(
                    recovery_action_key.method,
                    recovery_action_key.semantic_variant,
                )
                .cloned()
        })
        .flatten();
    if retry_possible_in_current_task && recovery_form.is_none() {
        return Err("Core recovery action is missing its current MCP action form".to_owned());
    }
    Ok(RetryContract {
        recovery_action_key: RequiredNullable::some(recovery_action_key),
        recovery_form: RequiredNullable::new(recovery_form),
        attempt_details,
        invalid_or_incompatible_submitted_paths,
        retry_possible_in_current_task,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use volicord_types::canonical::is_canonical_sha256_digest;
    use volicord_types::ids::{
        BaselineRef, ChangeUnitId, RecordId, ShapingCheckpointId, UserActionResolutionId,
    };
    use volicord_types::schema::{StateRecordRef, WorkflowActionRole};
    use volicord_types::values::{
        AuthorityNextActor, ChangeUnitOperation, RunKind, StateRecordKind, TaskMode,
        WorkflowActionSemanticVariant,
    };

    fn workflow_action_form(
        project_id: &ProjectId,
        transition: &TransitionDescriptor,
    ) -> Option<WorkflowActionForm> {
        let workflow = workflow_for_transitions(vec![transition.clone()]);
        super::workflow_action_form(project_id, &workflow, transition)
            .expect("valid action-form projection")
            .map(|projected| projected.form)
    }

    fn workflow_for_transitions(mut transitions: Vec<TransitionDescriptor>) -> WorkflowProjection {
        transitions.sort_by_key(|transition| {
            (
                transition.action_key.method.as_str(),
                transition.action_key.semantic_variant.as_str(),
            )
        });
        let first = transitions.first().expect("test transition");
        WorkflowProjection::ShapingRequired {
            next_actor: AuthorityNextActor::Agent,
            required_refs: first.required_refs.clone(),
            expected_state_version: first.expected_state_version,
            blocking_reason: RequiredNullable::null(),
            checkpoint: RequiredNullable::null(),
            transition_catalog: volicord_types::schema::WorkflowTransitionCatalog::new(transitions)
                .expect("test transition catalog"),
            close_readiness: volicord_types::schema::WorkflowCloseReadiness {
                assessment_required: false,
                current_close_basis_present: false,
            },
        }
    }

    fn authored_inputs(form: &WorkflowActionForm, required: bool) -> BTreeMap<&str, &str> {
        form.agent_authored_inputs
            .iter()
            .filter(|input| input.required == required)
            .map(|input| (input.path.as_str(), input.semantic_type.as_str()))
            .collect()
    }

    fn fixed_argument_paths(form: &WorkflowActionForm) -> Vec<String> {
        form.fixed_argument_paths.clone()
    }

    fn reference(
        kind: StateRecordKind,
        record_id: &str,
        project_id: &ProjectId,
        task_id: &TaskId,
        state_version: u64,
    ) -> StateRecordRef {
        StateRecordRef {
            record_kind: kind,
            record_id: RecordId::new(record_id),
            project_id: project_id.clone(),
            task_id: RequiredNullable::some(task_id.clone()),
            produced_at_state_version: RequiredNullable::some(state_version),
        }
    }

    fn agent_transition(
        method: MethodName,
        semantic_variant: WorkflowActionSemanticVariant,
        role: WorkflowActionRole,
        expected_state_version: u64,
        fixed_authority_coordinates: WorkflowActionAuthorityCoordinates,
        required_refs: Vec<StateRecordRef>,
    ) -> TransitionDescriptor {
        use volicord_types::values::{
            TaskMode, WorkflowExpectedResultState as ResultState,
            WorkflowTransitionEffectClass as Effect,
        };
        let effect_class = match method {
            MethodName::PrepareEvidenceCapture => Effect::EvidenceCapture,
            MethodName::PrepareWrite => Effect::WriteAuthorization,
            MethodName::StageArtifact => Effect::ArtifactStaging,
            MethodName::RecordRun => Effect::ExecutionRecording,
            MethodName::CheckClose => Effect::ReadOnlyAssessment,
            MethodName::CloseTask => Effect::TerminalMutation,
            _ => Effect::CoreStateMutation,
        };
        let expected_result_state = match method {
            MethodName::AdvanceTask
            | MethodName::PrepareEvidenceCapture
            | MethodName::PrepareWrite
            | MethodName::RecordRun
            | MethodName::ReconcileChanges => ResultState::Implementation,
            MethodName::FinalizeAdvice | MethodName::CheckClose => ResultState::CloseReview,
            MethodName::RequestUserAction => ResultState::AwaitingUserAction,
            MethodName::CloseTask => ResultState::Terminal,
            _ => ResultState::ReevaluateCurrentAuthority,
        };
        let submission_contract = WorkflowTransitionSubmissionContract::for_current_transition(
            TaskMode::Work,
            &fixed_authority_coordinates,
        );
        TransitionDescriptor {
            action_key: volicord_types::schema::WorkflowActionKey::new(method, semantic_variant)
                .expect("test transition key"),
            actor: volicord_types::values::WorkflowTransitionActor::Agent,
            role,
            expected_state_version,
            fixed_authority_coordinates,
            submission_contract,
            effect_class,
            expected_result_state,
            authority_invalidation:
                volicord_types::values::WorkflowAuthorityInvalidationPolicy::Permitted,
            required_refs,
        }
    }

    #[test]
    fn first_checkpoint_form_fixes_null_baseline_and_exact_input_slots() {
        let project_id = ProjectId::new("prj_action_form");
        let task_id = TaskId::new("task_action_form");
        let intent = agent_transition(
            MethodName::RecordShapingCheckpoint,
            WorkflowActionSemanticVariant::CreateInitial,
            WorkflowActionRole::Required,
            2,
            WorkflowActionAuthorityCoordinates::RecordShapingCheckpoint {
                task_id: task_id.clone(),
                checkpoint_operation: WorkflowCheckpointActionCoordinates::CreateInitial,
                scope_revision: 0,
                baseline_ref: RequiredNullable::null(),
            },
            vec![reference(
                StateRecordKind::Task,
                task_id.as_str(),
                &project_id,
                &task_id,
                2,
            )],
        );

        let form = workflow_action_form(&project_id, &intent).expect("current form");
        assert!(is_canonical_sha256_digest(form.form_ref.as_str()));
        assert_eq!(form.action_key.method, MethodName::RecordShapingCheckpoint);
        assert_eq!(form.expected_state_version, 2);
        assert_eq!(
            Value::Object(form.fixed_arguments.clone()),
            json!({
                "task_id": "task_action_form",
                "checkpoint_operation": { "operation": "create_initial" },
                "scope_revision": 0,
                "baseline_ref": null,
            })
        );
        let required = authored_inputs(&form, true);
        assert_eq!(required.get("/summary"), Some(&"string"));
        assert_eq!(
            required.get("/implementation_boundary"),
            Some(&"string | null")
        );
        assert_eq!(required.get("/gaps"), Some(&"array<ShapingGapInput>"));
    }

    #[test]
    fn action_form_inputs_use_descriptor_owned_types_and_required_nullable_presence() {
        let project_id = ProjectId::new("prj_descriptor_inputs");
        let intent = agent_transition(
            MethodName::UpdateScope,
            WorkflowActionSemanticVariant::KeepCurrentChangeUnit,
            WorkflowActionRole::Allowed,
            3,
            WorkflowActionAuthorityCoordinates::UpdateScope {
                task_id: TaskId::new("task_descriptor_inputs"),
                task_mode: TaskMode::Work,
                scope_revision: 1,
                baseline_ref: RequiredNullable::some(
                    BaselineRef::parse("baseline_current").expect("canonical test BaselineRef"),
                ),
                current_change_unit_id: RequiredNullable::some(ChangeUnitId::new("cu_current")),
                related_scope_decision_refs: Vec::new(),
                selected_change_unit_operation: ChangeUnitOperation::KeepCurrent,
            },
            Vec::new(),
        );

        let form = workflow_action_form(&project_id, &intent).expect("current form");
        let required = authored_inputs(&form, true);
        assert_eq!(required.get("/baseline_ref"), Some(&"BaselineRef | null"));
        for path in [
            "/goal_summary",
            "/scope_update",
            "/scope_boundary",
            "/non_goals",
            "/acceptance_criteria",
            "/autonomy_boundary",
        ] {
            assert!(
                required.contains_key(path),
                "{path} must be required-nullable"
            );
        }
        assert!(!form
            .agent_authored_inputs
            .iter()
            .any(|input| input.path.starts_with("/change_unit/")
                && input.path != "/change_unit/operation"));
    }

    #[test]
    fn create_change_unit_form_resolves_type_owned_flattened_fields() {
        let project_id = ProjectId::new("prj_create_change_unit_form");
        let intent = agent_transition(
            MethodName::UpdateScope,
            WorkflowActionSemanticVariant::CreateCurrentChangeUnit,
            WorkflowActionRole::Allowed,
            1,
            WorkflowActionAuthorityCoordinates::UpdateScope {
                task_id: TaskId::new("task_create_change_unit_form"),
                task_mode: TaskMode::Work,
                scope_revision: 0,
                baseline_ref: RequiredNullable::null(),
                current_change_unit_id: RequiredNullable::null(),
                related_scope_decision_refs: Vec::new(),
                selected_change_unit_operation: ChangeUnitOperation::CreateCurrent,
            },
            Vec::new(),
        );

        let form = workflow_action_form(&project_id, &intent).expect("create-current form");
        let required = authored_inputs(&form, true);
        assert_eq!(required.get("/change_unit/scope_summary"), Some(&"string"));
        assert_eq!(
            required.get("/change_unit/affected_paths"),
            Some(&"array<string>")
        );
        assert_eq!(
            form.canonical_minimal_request["change_unit"]["scope_summary"],
            "Bounded current work."
        );
    }

    #[test]
    fn advisor_create_and_replace_forms_fix_the_canonical_observe_only_boundary() {
        let project_id = ProjectId::new("prj_advisor_scope_forms");
        for operation in [
            ChangeUnitOperation::CreateCurrent,
            ChangeUnitOperation::ReplaceCurrent,
        ] {
            let coordinates = WorkflowActionAuthorityCoordinates::UpdateScope {
                task_id: TaskId::new("task_advisor_scope_forms"),
                task_mode: TaskMode::Advisor,
                scope_revision: 2,
                baseline_ref: RequiredNullable::some(
                    BaselineRef::parse("baseline_advisor_scope_forms").expect("baseline"),
                ),
                current_change_unit_id: match operation {
                    ChangeUnitOperation::CreateCurrent => RequiredNullable::null(),
                    ChangeUnitOperation::ReplaceCurrent => {
                        RequiredNullable::some(ChangeUnitId::new("cu_advisor_scope_forms"))
                    }
                    ChangeUnitOperation::KeepCurrent => unreachable!(),
                },
                related_scope_decision_refs: Vec::new(),
                selected_change_unit_operation: operation,
            };
            let mut transition = agent_transition(
                MethodName::UpdateScope,
                WorkflowActionSemanticVariant::for_change_unit_operation(operation),
                WorkflowActionRole::Allowed,
                4,
                coordinates,
                Vec::new(),
            );
            transition.submission_contract =
                WorkflowTransitionSubmissionContract::for_current_transition(
                    TaskMode::Advisor,
                    &transition.fixed_authority_coordinates,
                );

            let form = workflow_action_form(&project_id, &transition).expect("advisor form");
            assert_eq!(
                form.fixed_arguments["change_unit"]["affected_paths"],
                json!([])
            );
            assert_eq!(
                form.fixed_arguments["change_unit"]["effect_contract"],
                json!(volicord_types::schema::advisor_observe_only_effect_contract())
            );
            assert!(form
                .fixed_argument_paths
                .contains(&"/change_unit/affected_paths".to_owned()));
            assert!(form
                .fixed_argument_paths
                .contains(&"/change_unit/effect_contract".to_owned()));
            assert!(!form.agent_authored_inputs.iter().any(|input| matches!(
                input.path.as_str(),
                "/change_unit/affected_paths" | "/change_unit/effect_contract"
            )));
            assert_eq!(
                form.canonical_minimal_request["change_unit"]["affected_paths"],
                json!([])
            );
            assert_eq!(
                form.canonical_minimal_request["change_unit"]["effect_contract"],
                form.fixed_arguments["change_unit"]["effect_contract"]
            );
            let mut custom = Value::Object(form.canonical_minimal_request.clone());
            custom["change_unit"]["effect_contract"]["expected_outputs"] =
                json!(["Caller-authored output"]);
            let mismatches = bind_fixed_arguments(&form, &custom)
                .expect("Advisor fixed binding")
                .mismatches;
            assert_eq!(mismatches.len(), 1);
            assert_eq!(mismatches[0].path, "/change_unit/effect_contract");
            assert!(!mismatches[0].reached_core);
            assert!(!mismatches[0].state_change_applied);
        }
    }

    #[test]
    fn direct_and_work_create_and_replace_forms_retain_general_authored_semantics() {
        let project_id = ProjectId::new("prj_general_scope_forms");
        for task_mode in [TaskMode::Direct, TaskMode::Work] {
            for operation in [
                ChangeUnitOperation::CreateCurrent,
                ChangeUnitOperation::ReplaceCurrent,
            ] {
                let coordinates = WorkflowActionAuthorityCoordinates::UpdateScope {
                    task_id: TaskId::new("task_general_scope_forms"),
                    task_mode,
                    scope_revision: 3,
                    baseline_ref: RequiredNullable::some(
                        BaselineRef::parse("baseline_general_scope_forms").expect("baseline"),
                    ),
                    current_change_unit_id: match operation {
                        ChangeUnitOperation::CreateCurrent => RequiredNullable::null(),
                        ChangeUnitOperation::ReplaceCurrent => {
                            RequiredNullable::some(ChangeUnitId::new("cu_general_scope_forms"))
                        }
                        ChangeUnitOperation::KeepCurrent => unreachable!(),
                    },
                    related_scope_decision_refs: Vec::new(),
                    selected_change_unit_operation: operation,
                };
                let mut transition = agent_transition(
                    MethodName::UpdateScope,
                    WorkflowActionSemanticVariant::for_change_unit_operation(operation),
                    WorkflowActionRole::Allowed,
                    5,
                    coordinates,
                    Vec::new(),
                );
                transition.submission_contract =
                    WorkflowTransitionSubmissionContract::for_current_transition(
                        task_mode,
                        &transition.fixed_authority_coordinates,
                    );
                let form = workflow_action_form(&project_id, &transition)
                    .expect("general product-capable Agent form");
                let authored = form
                    .agent_authored_inputs
                    .iter()
                    .map(|input| (input.path.as_str(), input.required))
                    .collect::<BTreeSet<_>>();
                assert!(authored.contains(&("/change_unit/scope_summary", true)));
                assert!(authored.contains(&("/change_unit/affected_paths", true)));
                assert!(authored.contains(&("/change_unit/effect_contract", false)));
                assert!(authored.contains(&("/baseline_ref", true)));
                assert!(form.fixed_arguments["change_unit"]
                    .get("affected_paths")
                    .is_none());
                assert!(form.fixed_arguments["change_unit"]
                    .get("effect_contract")
                    .is_none());
                assert_eq!(
                    form.canonical_minimal_request["baseline_ref"],
                    "baseline_general_scope_forms"
                );
            }
        }
    }

    #[test]
    fn form_digest_and_witness_change_with_the_submission_contract() {
        let project_id = ProjectId::new("prj_submission_digest");
        let mut transition = agent_transition(
            MethodName::UpdateScope,
            WorkflowActionSemanticVariant::CreateCurrentChangeUnit,
            WorkflowActionRole::Allowed,
            3,
            WorkflowActionAuthorityCoordinates::UpdateScope {
                task_id: TaskId::new("task_submission_digest"),
                task_mode: TaskMode::Work,
                scope_revision: 1,
                baseline_ref: RequiredNullable::null(),
                current_change_unit_id: RequiredNullable::null(),
                related_scope_decision_refs: Vec::new(),
                selected_change_unit_operation: ChangeUnitOperation::CreateCurrent,
            },
            Vec::new(),
        );
        let first = workflow_action_form(&project_id, &transition).expect("first form");
        let WorkflowTransitionSubmissionContract::UpdateScope {
            contract:
                WorkflowUpdateScopeSubmissionContract::GeneralCreateCurrentChangeUnit {
                    required_change_unit_witness,
                    ..
                },
        } = &mut transition.submission_contract
        else {
            panic!("general create contract")
        };
        required_change_unit_witness.scope_summary = "Alternate bounded witness.".to_owned();
        let second = workflow_action_form(&project_id, &transition).expect("second form");
        assert_eq!(first.fixed_arguments, second.fixed_arguments);
        assert_ne!(first.form_ref, second.form_ref);
        assert_ne!(
            first.canonical_minimal_request,
            second.canonical_minimal_request
        );
    }

    #[test]
    fn current_change_unit_projects_distinct_keep_and_replace_forms() {
        let project_id = ProjectId::new("prj_current_change_unit_forms");
        let coordinates = |operation| WorkflowActionAuthorityCoordinates::UpdateScope {
            task_id: TaskId::new("task_current_change_unit_forms"),
            task_mode: TaskMode::Work,
            scope_revision: 2,
            baseline_ref: RequiredNullable::some(
                BaselineRef::parse("baseline_current").expect("canonical test BaselineRef"),
            ),
            current_change_unit_id: RequiredNullable::some(ChangeUnitId::new("cu_current")),
            related_scope_decision_refs: Vec::new(),
            selected_change_unit_operation: operation,
        };
        let form = |operation| {
            workflow_action_form(
                &project_id,
                &agent_transition(
                    MethodName::UpdateScope,
                    WorkflowActionSemanticVariant::for_change_unit_operation(operation),
                    WorkflowActionRole::Allowed,
                    4,
                    coordinates(operation),
                    Vec::new(),
                ),
            )
            .expect("current update-scope form")
        };

        let keep = form(ChangeUnitOperation::KeepCurrent);
        let replace = form(ChangeUnitOperation::ReplaceCurrent);
        assert_ne!(keep.form_ref, replace.form_ref);
        assert_eq!(
            keep.fixed_arguments["change_unit"]["operation"],
            "keep_current"
        );
        assert_eq!(
            replace.fixed_arguments["change_unit"]["operation"],
            "replace_current"
        );
        assert!(replace
            .agent_authored_inputs
            .iter()
            .any(|input| input.required && input.path == "/change_unit/scope_summary"));
        assert!(replace
            .agent_authored_inputs
            .iter()
            .any(|input| input.required && input.path == "/change_unit/affected_paths"));
    }

    #[test]
    fn replacement_form_preserves_exact_checkpoint_lineage_and_authority_refs() {
        let project_id = ProjectId::new("prj_replace");
        let task_id = TaskId::new("task_replace");
        let current = reference(
            StateRecordKind::ShapingCheckpoint,
            "checkpoint_current",
            &project_id,
            &task_id,
            9,
        );
        let predecessor = reference(
            StateRecordKind::ShapingCheckpoint,
            "checkpoint_predecessor",
            &project_id,
            &task_id,
            6,
        );
        let retired = reference(
            StateRecordKind::UserActionRequest,
            "request_retired",
            &project_id,
            &task_id,
            7,
        );
        let carried = reference(
            StateRecordKind::ShapingDecisionApplication,
            "application_carried",
            &project_id,
            &task_id,
            8,
        );
        let stale = reference(
            StateRecordKind::ShapingDecisionApplication,
            "application_stale",
            &project_id,
            &task_id,
            8,
        );
        let intent = agent_transition(
            MethodName::RecordShapingCheckpoint,
            WorkflowActionSemanticVariant::ReplaceCurrent,
            WorkflowActionRole::Required,
            9,
            WorkflowActionAuthorityCoordinates::RecordShapingCheckpoint {
                task_id,
                checkpoint_operation: WorkflowCheckpointActionCoordinates::ReplaceCurrent {
                    current_checkpoint_ref: current.clone(),
                    predecessor_checkpoint_ref: RequiredNullable::some(predecessor.clone()),
                    retired_non_authorizing_request_refs: vec![retired.clone()],
                    carry_forward_application_refs: vec![carried.clone()],
                    stale_application_refs: vec![stale.clone()],
                },
                scope_revision: 3,
                baseline_ref: RequiredNullable::some(
                    BaselineRef::parse("baseline_current").expect("canonical test BaselineRef"),
                ),
            },
            Vec::new(),
        );

        let form = workflow_action_form(&project_id, &intent).expect("replacement form");
        let fixed = Value::Object(form.fixed_arguments);
        assert!(fixed.get("current_checkpoint_ref").is_none());
        assert!(fixed.get("predecessor_checkpoint_ref").is_none());
        assert_eq!(
            fixed["checkpoint_operation"]["expected_current_checkpoint_id"],
            json!(current.record_id.as_str())
        );
        assert_eq!(
            fixed["checkpoint_operation"]["retired_non_authorizing_request_refs"],
            json!([retired])
        );
        assert_eq!(
            fixed["checkpoint_operation"]["carry_forward_application_refs"],
            json!([carried])
        );
        assert_eq!(
            fixed["checkpoint_operation"]["stale_authority_actions"][0]["stale_application_ref"],
            json!(stale)
        );
    }

    #[test]
    fn advisor_finalization_form_fixes_current_authority_and_changes_with_state() {
        let project_id = ProjectId::new("prj_advisor");
        let task_id = TaskId::new("task_advisor");
        let intent = agent_transition(
            MethodName::FinalizeAdvice,
            WorkflowActionSemanticVariant::FinalizeAdvice,
            WorkflowActionRole::Required,
            12,
            WorkflowActionAuthorityCoordinates::FinalizeAdvice {
                task_id,
                shaping_checkpoint_id: ShapingCheckpointId::new("checkpoint_advisor"),
                change_unit_id: ChangeUnitId::new("change_unit_advisor"),
                scope_revision: 4,
                baseline_ref: RequiredNullable::some(
                    BaselineRef::parse("baseline_advisor").expect("canonical test BaselineRef"),
                ),
                user_action_resolution_ids: vec![UserActionResolutionId::new("resolution_advisor")],
            },
            Vec::new(),
        );
        let form = workflow_action_form(&project_id, &intent).expect("advisor form");
        assert_eq!(
            form.fixed_arguments["user_action_resolution_ids"],
            json!(["resolution_advisor"])
        );
        assert_eq!(
            authored_inputs(&form, true).get("/result_summary"),
            Some(&"string")
        );

        let mut next = intent;
        next.expected_state_version += 1;
        let next_form = workflow_action_form(&project_id, &next).expect("next form");
        assert_ne!(form.form_ref, next_form.form_ref);
    }

    fn altered(value: &Value) -> Value {
        match value {
            Value::Null => json!("null"),
            Value::Bool(value) => json!(!value),
            Value::Number(value) => json!(value.as_u64().unwrap_or_default() + 1),
            Value::String(value) => json!(format!("{value}_altered")),
            Value::Array(values) if values.is_empty() => json!([null]),
            Value::Array(_) => json!([]),
            Value::Object(values) => {
                let mut values = values.clone();
                values.insert("altered".to_owned(), Value::Bool(true));
                Value::Object(values)
            }
        }
    }

    fn remove_pointer(value: &mut Value, pointer: &str) {
        let (parent, leaf) = pointer
            .rsplit_once('/')
            .expect("fixed pointer must have a parent");
        let leaf = leaf.replace("~1", "/").replace("~0", "~");
        let parent = value
            .pointer_mut(parent)
            .expect("fixed pointer parent must exist");
        match parent {
            Value::Object(object) => {
                object.remove(&leaf);
            }
            Value::Array(array) => {
                array.remove(leaf.parse::<usize>().expect("array pointer index"));
            }
            _ => panic!("fixed pointer parent must be a container"),
        }
    }

    #[test]
    fn every_state_bound_form_binds_and_rejects_each_altered_or_omitted_fixed_value() {
        let project_id = ProjectId::new("prj_binding_table");
        let task_id = TaskId::new("task_binding_table");
        let checkpoint_ref = reference(
            StateRecordKind::ShapingCheckpoint,
            "checkpoint_binding_table",
            &project_id,
            &task_id,
            20,
        );
        let resolution_ref = reference(
            StateRecordKind::UserActionResolution,
            "resolution_binding_table",
            &project_id,
            &task_id,
            20,
        );
        let application_ref = reference(
            StateRecordKind::ShapingDecisionApplication,
            "application_binding_table",
            &project_id,
            &task_id,
            20,
        );
        let baseline =
            BaselineRef::parse("baseline_binding_table").expect("canonical test BaselineRef");
        let change_unit = ChangeUnitId::new("change_unit_binding_table");
        let coordinates = vec![
            WorkflowActionAuthorityCoordinates::RecordShapingCheckpoint {
                task_id: task_id.clone(),
                checkpoint_operation: WorkflowCheckpointActionCoordinates::CreateInitial,
                scope_revision: 4,
                baseline_ref: RequiredNullable::null(),
            },
            WorkflowActionAuthorityCoordinates::RecordShapingCheckpoint {
                task_id: task_id.clone(),
                checkpoint_operation: WorkflowCheckpointActionCoordinates::ReplaceCurrent {
                    current_checkpoint_ref: checkpoint_ref.clone(),
                    predecessor_checkpoint_ref: RequiredNullable::null(),
                    retired_non_authorizing_request_refs: vec![resolution_ref.clone()],
                    carry_forward_application_refs: vec![application_ref.clone()],
                    stale_application_refs: vec![application_ref.clone()],
                },
                scope_revision: 4,
                baseline_ref: RequiredNullable::some(baseline.clone()),
            },
            WorkflowActionAuthorityCoordinates::UpdateScope {
                task_id: task_id.clone(),
                task_mode: TaskMode::Work,
                scope_revision: 4,
                baseline_ref: RequiredNullable::some(baseline.clone()),
                current_change_unit_id: RequiredNullable::some(change_unit.clone()),
                related_scope_decision_refs: vec![resolution_ref],
                selected_change_unit_operation: ChangeUnitOperation::KeepCurrent,
            },
            WorkflowActionAuthorityCoordinates::FinalizeAdvice {
                task_id: task_id.clone(),
                shaping_checkpoint_id: ShapingCheckpointId::new("checkpoint_binding_table"),
                change_unit_id: change_unit.clone(),
                scope_revision: 4,
                baseline_ref: RequiredNullable::some(baseline.clone()),
                user_action_resolution_ids: vec![UserActionResolutionId::new(
                    "resolution_binding_table",
                )],
            },
            WorkflowActionAuthorityCoordinates::AdvanceTask {
                task_id: task_id.clone(),
                shaping_checkpoint_id: ShapingCheckpointId::new("checkpoint_binding_table"),
                change_unit_id: change_unit.clone(),
                scope_revision: 4,
                baseline_ref: RequiredNullable::some(baseline.clone()),
                user_action_resolution_ids: Vec::new(),
            },
            WorkflowActionAuthorityCoordinates::PrepareEvidenceCapture {
                task_id: task_id.clone(),
                change_unit_id: change_unit.clone(),
                baseline_ref: baseline.clone(),
            },
            WorkflowActionAuthorityCoordinates::PrepareWrite {
                task_id: task_id.clone(),
                change_unit_id: change_unit.clone(),
                baseline_ref: baseline.clone(),
            },
            WorkflowActionAuthorityCoordinates::StageArtifact {
                task_id: task_id.clone(),
            },
            WorkflowActionAuthorityCoordinates::RecordRun {
                task_id: task_id.clone(),
                change_unit_id: change_unit.clone(),
                baseline_ref: baseline,
                run_kind: RunKind::Implementation,
            },
            WorkflowActionAuthorityCoordinates::RequestUserAction {
                task_id: task_id.clone(),
                change_unit_id: RequiredNullable::some(change_unit),
            },
            WorkflowActionAuthorityCoordinates::ReconcileChanges {
                task_id: task_id.clone(),
            },
            WorkflowActionAuthorityCoordinates::CheckClose {
                task_id: task_id.clone(),
            },
            WorkflowActionAuthorityCoordinates::CloseTask { task_id },
        ];

        for fixed_authority_coordinates in coordinates {
            let method = fixed_authority_coordinates.method();
            let semantic_variant = fixed_authority_coordinates.semantic_variant();
            let form = workflow_action_form(
                &project_id,
                &agent_transition(
                    method,
                    semantic_variant,
                    WorkflowActionRole::Allowed,
                    21,
                    fixed_authority_coordinates,
                    Vec::new(),
                ),
            )
            .expect("current form");
            let exact = Value::Object(form.fixed_arguments.clone());
            assert!(
                bind_fixed_arguments(&form, &exact)
                    .expect("binding contract")
                    .mismatches
                    .is_empty(),
                "{} exact fixed arguments",
                method.as_str()
            );

            for path in fixed_argument_paths(&form) {
                let expected = exact.pointer(&path).expect("fixed value").clone();
                let mut mutated = exact.clone();
                *mutated.pointer_mut(&path).expect("fixed value") = altered(&expected);
                let mismatch = bind_fixed_arguments(&form, &mutated)
                    .expect("binding contract")
                    .mismatches;
                assert_eq!(mismatch.len(), 1, "{} {path}", method.as_str());
                assert_eq!(mismatch[0].path, path);
                assert_eq!(mismatch[0].expected_value, expected);
                assert!(mismatch[0].received_value_present);
                assert!(!mismatch[0].reached_core);
                assert!(!mismatch[0].state_change_applied);

                let mut omitted = exact.clone();
                remove_pointer(&mut omitted, &path);
                let mismatch = bind_fixed_arguments(&form, &omitted)
                    .expect("binding contract")
                    .mismatches;
                assert_eq!(mismatch.len(), 1, "{} omitted {path}", method.as_str());
                assert_eq!(mismatch[0].path, path);
                assert!(!mismatch[0].received_value_present);
            }
        }
    }

    #[test]
    fn catalog_is_a_total_exact_projection_and_minimal_requests_are_executable() {
        let project_id = ProjectId::new("prj_total_projection");
        let task_id = TaskId::new("task_total_projection");
        let coordinates = |operation| WorkflowActionAuthorityCoordinates::UpdateScope {
            task_id: task_id.clone(),
            task_mode: TaskMode::Work,
            scope_revision: 2,
            baseline_ref: RequiredNullable::some(
                BaselineRef::parse("baseline_total_projection").expect("baseline"),
            ),
            current_change_unit_id: RequiredNullable::some(ChangeUnitId::new(
                "cu_total_projection",
            )),
            related_scope_decision_refs: Vec::new(),
            selected_change_unit_operation: operation,
        };
        let transitions = [
            ChangeUnitOperation::KeepCurrent,
            ChangeUnitOperation::ReplaceCurrent,
        ]
        .into_iter()
        .map(|operation| {
            agent_transition(
                MethodName::UpdateScope,
                WorkflowActionSemanticVariant::for_change_unit_operation(operation),
                WorkflowActionRole::Allowed,
                7,
                coordinates(operation),
                Vec::new(),
            )
        })
        .collect::<Vec<_>>();
        let workflow = workflow_for_transitions(transitions.clone());
        let catalog =
            workflow_action_form_catalog(&project_id, &workflow, |_, _| Ok(())).expect("catalog");

        assert_eq!(catalog.forms.len(), transitions.len());
        for transition in transitions {
            let form = catalog
                .form(
                    transition.action_key.method,
                    transition.action_key.semantic_variant,
                )
                .expect("one exact form per Agent transition");
            assert_eq!(form.action_key, transition.action_key);
            assert_eq!(
                submitted_action_form_semantic_variant(
                    form.action_key.method,
                    &Value::Object(form.canonical_minimal_request.clone()),
                ),
                Some(form.action_key.semantic_variant)
            );
            assert!(bind_fixed_arguments(
                form,
                &Value::Object(form.canonical_minimal_request.clone())
            )
            .expect("fixed binding")
            .mismatches
            .is_empty());
        }
        assert!(catalog
            .form(
                MethodName::UpdateScope,
                WorkflowActionSemanticVariant::CreateCurrentChangeUnit,
            )
            .is_none());
    }

    #[test]
    fn malformed_submission_contract_and_missing_required_form_fail_closed() {
        let project_id = ProjectId::new("prj_invalid_catalog");
        let task_id = TaskId::new("task_invalid_catalog");
        let mut transition = agent_transition(
            MethodName::UpdateScope,
            WorkflowActionSemanticVariant::CreateCurrentChangeUnit,
            WorkflowActionRole::Required,
            3,
            WorkflowActionAuthorityCoordinates::UpdateScope {
                task_id,
                task_mode: TaskMode::Work,
                scope_revision: 0,
                baseline_ref: RequiredNullable::null(),
                current_change_unit_id: RequiredNullable::null(),
                related_scope_decision_refs: Vec::new(),
                selected_change_unit_operation: ChangeUnitOperation::CreateCurrent,
            },
            Vec::new(),
        );
        let valid_workflow = workflow_for_transitions(vec![transition.clone()]);
        let missing = validate_action_form_totality(&valid_workflow, &[])
            .expect_err("required form omission must fail closed");
        assert_eq!(missing.action_key, Some(transition.action_key));
        assert_eq!(missing.stage, McpWorkflowContractStage::CatalogTotality);

        transition.submission_contract = WorkflowTransitionSubmissionContract::CheckClose {
            required_agent_input_witness: volicord_types::schema::WorkflowNoSubmissionValues {},
            optional_agent_input_witness: volicord_types::schema::WorkflowNoSubmissionValues {},
        };
        assert!(volicord_types::schema::WorkflowTransitionCatalog::new(vec![transition]).is_err());

        let _ = project_id;
    }

    #[test]
    fn retry_lookup_never_falls_back_and_external_close_has_no_form() {
        let project_id = ProjectId::new("prj_retry_projection");
        let task_id = TaskId::new("task_retry_projection");
        let attempted = agent_transition(
            MethodName::UpdateScope,
            WorkflowActionSemanticVariant::KeepCurrentChangeUnit,
            WorkflowActionRole::Allowed,
            9,
            WorkflowActionAuthorityCoordinates::UpdateScope {
                task_id: task_id.clone(),
                task_mode: TaskMode::Work,
                scope_revision: 3,
                baseline_ref: RequiredNullable::some(
                    BaselineRef::parse("baseline_retry_projection").expect("baseline"),
                ),
                current_change_unit_id: RequiredNullable::some(ChangeUnitId::new(
                    "cu_retry_projection",
                )),
                related_scope_decision_refs: Vec::new(),
                selected_change_unit_operation: ChangeUnitOperation::KeepCurrent,
            },
            Vec::new(),
        );
        let close = agent_transition(
            MethodName::CloseTask,
            WorkflowActionSemanticVariant::CloseTask,
            WorkflowActionRole::Allowed,
            9,
            WorkflowActionAuthorityCoordinates::CloseTask { task_id },
            Vec::new(),
        );
        let workflow = workflow_for_transitions(vec![attempted.clone(), close.clone()]);
        let mut catalog =
            workflow_action_form_catalog(&project_id, &workflow, |_, _| Ok(())).expect("catalog");
        let external_close = retry_contract(
            attempted.action_key,
            close.action_key,
            &workflow,
            &catalog,
            TransitionAttemptDetails::None,
            vec!["/baseline_ref".to_owned()],
        )
        .expect("typed external close recovery");
        assert_eq!(
            external_close.recovery_action_key.as_ref(),
            Some(&close.action_key)
        );
        assert!(external_close.recovery_form.as_ref().is_none());
        assert!(!external_close.retry_possible_in_current_task);

        catalog
            .forms
            .retain(|form| form.action_key != attempted.action_key);
        let error = retry_contract(
            attempted.action_key,
            attempted.action_key,
            &workflow,
            &catalog,
            TransitionAttemptDetails::None,
            Vec::new(),
        )
        .expect_err("missing exact recovery form must not fall back");
        assert!(error.contains("missing its current MCP action form"));
    }
}
