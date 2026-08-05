//! MCP workflow action forms derived from neutral Core-owned intent.

use serde::Serialize;
use serde_json::{json, Map, Value};
use volicord_mcp_wire::{
    action_form_request_projection, mcp_tool_contract, McpActionFormArgumentMismatch,
    RetryContract, SemanticSchemaDescriptor, WorkflowActionForm, WorkflowActionFormCatalog,
    WorkflowActionInput, MAX_VALIDATION_ISSUES,
};
use volicord_types::canonical::canonical_json_sha256;
use volicord_types::ids::{ProjectId, RequestHash, TaskId};
use volicord_types::schema::{
    JsonObject, RequiredNullable, WorkflowActionAuthorityCoordinates, WorkflowActionIntent,
    WorkflowCheckpointActionCoordinates, WorkflowProjection,
};
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::MethodName;

#[derive(Serialize)]
struct WorkflowActionFormDigestBasis<'a> {
    domain: &'static str,
    project_id: &'a ProjectId,
    task_id: &'a TaskId,
    method: MethodName,
    selected_semantic_variant: &'a str,
    expected_state_version: u64,
    fixed_authority_coordinates: &'a WorkflowActionAuthorityCoordinates,
    fixed_arguments: &'a JsonObject,
    fixed_argument_paths: &'a [String],
    semantic_schema_digest: &'a RequestHash,
}

fn input(path: &str, semantic_type: &str) -> WorkflowActionInput {
    WorkflowActionInput {
        path: path.to_owned(),
        semantic_type: semantic_type.to_owned(),
    }
}

fn descriptor_owned_inputs(
    descriptor: &SemanticSchemaDescriptor,
    required: Vec<WorkflowActionInput>,
    optional: Vec<WorkflowActionInput>,
) -> Option<(Vec<WorkflowActionInput>, Vec<WorkflowActionInput>)> {
    fn typed_input(
        descriptor: &SemanticSchemaDescriptor,
        authored: WorkflowActionInput,
    ) -> Option<(WorkflowActionInput, bool)> {
        let contracts = descriptor.field_contracts_at_pointer_pattern(&authored.path);
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
        Some((
            input(&authored.path, &semantic_types),
            contracts.iter().all(|field| field.required),
        ))
    }

    let mut descriptor_required = Vec::new();
    for authored in required {
        descriptor_required.push(typed_input(descriptor, authored)?.0);
    }
    let mut descriptor_optional = Vec::new();
    for authored in optional {
        let (authored, is_required) = typed_input(descriptor, authored)?;
        if is_required {
            descriptor_required.push(authored);
        } else {
            descriptor_optional.push(authored);
        }
    }
    Some((descriptor_required, descriptor_optional))
}

fn selected_variant(coordinates: &WorkflowActionAuthorityCoordinates) -> &'static str {
    match coordinates {
        WorkflowActionAuthorityCoordinates::RecordShapingCheckpoint {
            checkpoint_operation: WorkflowCheckpointActionCoordinates::CreateInitial,
            ..
        } => "create_initial",
        WorkflowActionAuthorityCoordinates::RecordShapingCheckpoint {
            checkpoint_operation: WorkflowCheckpointActionCoordinates::ReplaceCurrent { .. },
            ..
        } => "replace_current",
        WorkflowActionAuthorityCoordinates::UpdateScope { .. } => "update_scope",
        WorkflowActionAuthorityCoordinates::FinalizeAdvice { .. } => "finalize_advice",
        WorkflowActionAuthorityCoordinates::AdvanceTask { .. } => "advance_task",
        WorkflowActionAuthorityCoordinates::PrepareEvidenceCapture { .. } => {
            "prepare_evidence_capture"
        }
        WorkflowActionAuthorityCoordinates::PrepareWrite { .. } => "prepare_write",
        WorkflowActionAuthorityCoordinates::StageArtifact { .. } => "stage_artifact",
        WorkflowActionAuthorityCoordinates::RecordRun { .. } => "record_run",
        WorkflowActionAuthorityCoordinates::RequestUserAction { .. } => "request_user_action",
        WorkflowActionAuthorityCoordinates::ReconcileChanges { .. } => "reconcile_changes",
        WorkflowActionAuthorityCoordinates::CheckClose { .. } => "check_close",
        WorkflowActionAuthorityCoordinates::CloseTask { .. } => "close_task",
    }
}

fn checkpoint_operation(
    operation: &WorkflowCheckpointActionCoordinates,
) -> (Value, Vec<WorkflowActionInput>) {
    match operation {
        WorkflowCheckpointActionCoordinates::CreateInitial => {
            (json!({ "operation": "create_initial" }), Vec::new())
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
            let mut required = stale_application_refs
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    input(
                        &format!("/checkpoint_operation/stale_authority_actions/{index}/action"),
                        "retire | reauthorize",
                    )
                })
                .collect::<Vec<_>>();
            if !stale_application_refs.is_empty() {
                required.push(input(
                    "/checkpoint_operation/stale_authority_actions/*/successor_gap",
                    "ShapingGapInput when action=reauthorize",
                ));
            }
            (
                json!({
                    "operation": "replace_current",
                    "expected_current_checkpoint_id": current_checkpoint_ref.record_id.as_str(),
                    "retired_non_authorizing_request_refs": retired_non_authorizing_request_refs,
                    "carry_forward_application_refs": carry_forward_application_refs,
                    "stale_authority_actions": stale_authority_actions,
                }),
                required,
            )
        }
    }
}

fn project_fixed_arguments(
    _project_id: &ProjectId,
    coordinates: &WorkflowActionAuthorityCoordinates,
) -> (
    TaskId,
    JsonObject,
    JsonObject,
    Vec<WorkflowActionInput>,
    Vec<WorkflowActionInput>,
) {
    let mut fixed = Map::new();
    let mut suggested = Map::new();
    match coordinates {
        WorkflowActionAuthorityCoordinates::RecordShapingCheckpoint {
            task_id,
            checkpoint_operation: operation,
            scope_revision,
            baseline_ref,
        } => {
            let (operation_value, mut conditional_inputs) = checkpoint_operation(operation);
            fixed.insert("task_id".to_owned(), json!(task_id));
            fixed.insert("checkpoint_operation".to_owned(), operation_value);
            fixed.insert("scope_revision".to_owned(), json!(scope_revision));
            fixed.insert("baseline_ref".to_owned(), json!(baseline_ref));
            suggested.insert("source_refs".to_owned(), json!([]));
            suggested.insert("evidence_refs".to_owned(), json!([]));
            let mut required = vec![
                input("/summary", "string"),
                input("/implementation_boundary", "string | null"),
                input("/gaps", "array<ShapingGapInput>"),
            ];
            required.append(&mut conditional_inputs);
            (
                task_id.clone(),
                fixed,
                suggested,
                required,
                vec![
                    input("/source_refs", "array<SourceRef>"),
                    input("/evidence_refs", "array<StateRecordRef>"),
                ],
            )
        }
        WorkflowActionAuthorityCoordinates::UpdateScope {
            task_id,
            current_change_unit_id,
            related_scope_decision_refs,
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
            let mut required = vec![input("/baseline_ref", "string | null")];
            if current_change_unit_id.as_ref().is_some() {
                fixed.insert(
                    "change_unit".to_owned(),
                    json!({ "operation": "keep_current" }),
                );
            } else {
                fixed.insert(
                    "change_unit".to_owned(),
                    json!({ "operation": "create_current" }),
                );
                required.extend([
                    input("/change_unit/scope_summary", "string"),
                    input("/change_unit/affected_paths", "array<string>"),
                ]);
            }
            (
                task_id.clone(),
                fixed,
                suggested,
                required,
                vec![
                    input("/change_unit/effect_contract", "ChangeUnitEffectContract"),
                    input("/goal_summary", "string | null"),
                    input("/scope_update", "ScopeUpdate | null"),
                    input("/scope_boundary", "string | null"),
                    input("/non_goals", "array<string> | null"),
                    input(
                        "/acceptance_criteria",
                        "array<AcceptanceCriterionReplacement> | null",
                    ),
                    input("/autonomy_boundary", "string | null"),
                ],
            )
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
            (
                task_id.clone(),
                fixed,
                suggested,
                vec![input("/result_summary", "string")],
                vec![
                    input("/result_refs", "array<StateRecordRef>"),
                    input("/evidence_refs", "array<StateRecordRef>"),
                    input("/residual_risks", "array<ResidualRiskInput>"),
                    input("/recovery_constraints", "array<string>"),
                ],
            )
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
            (task_id.clone(), fixed, suggested, Vec::new(), Vec::new())
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
            (
                task_id.clone(),
                fixed,
                suggested,
                vec![
                    input("/target", "EvidenceTarget"),
                    input("/capture", "McpEvidenceCaptureSpec"),
                ],
                Vec::new(),
            )
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
            (
                task_id.clone(),
                fixed,
                suggested,
                vec![
                    input("/intended_operation", "string"),
                    input("/intended_paths", "array<string>"),
                    input("/product_file_write_intended", "boolean"),
                ],
                vec![input("/sensitive_categories", "array<string>")],
            )
        }
        WorkflowActionAuthorityCoordinates::StageArtifact { task_id } => {
            fixed.insert("task_id".to_owned(), json!(task_id));
            (
                task_id.clone(),
                fixed,
                suggested,
                vec![
                    input("/display_name", "string"),
                    input("/content_type", "string"),
                    input("/redaction_state", "RedactionState"),
                    input("/safe_bytes_or_notice", "string"),
                ],
                vec![
                    input("/expected_sha256", "string | null"),
                    input("/expected_size_bytes", "integer | null"),
                    input("/relation_hint", "string | null"),
                ],
            )
        }
        WorkflowActionAuthorityCoordinates::RecordRun {
            task_id,
            change_unit_id,
            baseline_ref,
            run_kind,
        } => {
            fixed.extend([
                ("task_id".to_owned(), json!(task_id)),
                ("change_unit_id".to_owned(), json!(change_unit_id)),
                ("baseline_ref".to_owned(), json!(baseline_ref)),
                ("kind".to_owned(), json!(run_kind)),
            ]);
            (
                task_id.clone(),
                fixed,
                suggested,
                vec![
                    input("/summary", "string"),
                    input("/observed_changes", "ObservedChanges"),
                ],
                vec![
                    input("/run_id", "RunId | null"),
                    input("/write_ticket_id", "WriteTicketId | null"),
                    input("/performed_operation", "string | null"),
                    input("/artifact_inputs", "array<ArtifactInput>"),
                    input("/evidence_updates", "array<McpEvidenceCoverageUpdate>"),
                    input(
                        "/evidence_observations",
                        "array<McpEvidenceObservationInput>",
                    ),
                    input("/close_assessment", "CloseAssessmentInput | null"),
                ],
            )
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
            (
                task_id.clone(),
                fixed,
                suggested,
                vec![
                    input("/request/action", "UserActionDraft"),
                    input("/request/required_for", "array<UserActionRequiredFor>"),
                ],
                vec![input("/request/expires_at", "UtcTimestamp | null")],
            )
        }
        WorkflowActionAuthorityCoordinates::ReconcileChanges { task_id } => {
            fixed.insert("task_id".to_owned(), json!(task_id));
            (
                task_id.clone(),
                fixed,
                suggested,
                Vec::new(),
                vec![input(
                    "/resolution_requests",
                    "array<UnrecordedChangeResolutionRequest>",
                )],
            )
        }
        WorkflowActionAuthorityCoordinates::CheckClose { task_id } => {
            fixed.insert("task_id".to_owned(), json!(task_id));
            (task_id.clone(), fixed, suggested, Vec::new(), Vec::new())
        }
        WorkflowActionAuthorityCoordinates::CloseTask { task_id } => {
            fixed.insert("task_id".to_owned(), json!(task_id));
            (
                task_id.clone(),
                fixed,
                suggested,
                vec![input("/intent", "CloseMutationIntent")],
                vec![
                    input("/close_reason", "CloseReason | null"),
                    input("/superseding_task_id", "TaskId | null"),
                    input("/user_note", "string | null"),
                ],
            )
        }
    }
}

/// Projects one current neutral intent through the canonical MCP descriptor.
pub(crate) fn workflow_action_form(
    project_id: &ProjectId,
    intent: &WorkflowActionIntent,
) -> Option<WorkflowActionForm> {
    let tool = AgentToolId::from_method(intent.method)?;
    let descriptor = mcp_tool_contract(tool)?;
    let semantic_schema_digest = canonical_json_sha256(&descriptor.input_schema()).ok()?;
    let (task_id, fixed_arguments, suggested_arguments, required_inputs, optional_inputs) =
        project_fixed_arguments(project_id, &intent.fixed_authority_coordinates);
    let (required_inputs, optional_inputs) = descriptor_owned_inputs(
        descriptor.input_descriptor(),
        required_inputs,
        optional_inputs,
    )?;
    let selected_semantic_variant = selected_variant(&intent.fixed_authority_coordinates);
    let projection = action_form_request_projection(intent.method, selected_semantic_variant)?;
    let fixed_argument_paths = projection
        .concrete_fixed_argument_paths(&fixed_arguments)
        .ok()?;
    let fixed = Value::Object(fixed_arguments.clone());
    if fixed_argument_paths.iter().any(|path| {
        fixed.pointer(path).is_none_or(|value| {
            !descriptor
                .input_descriptor()
                .accepts_value_at_pointer(path, value)
        })
    }) {
        return None;
    }
    let form_ref = canonical_json_sha256(&WorkflowActionFormDigestBasis {
        domain: "volicord.mcp-workflow-action-form",
        project_id,
        task_id: &task_id,
        method: intent.method,
        selected_semantic_variant,
        expected_state_version: intent.expected_state_version,
        fixed_authority_coordinates: &intent.fixed_authority_coordinates,
        fixed_arguments: &fixed_arguments,
        fixed_argument_paths: &fixed_argument_paths,
        semantic_schema_digest: &semantic_schema_digest,
    })
    .ok()?;
    Some(WorkflowActionForm {
        form_ref,
        method: intent.method,
        selected_semantic_variant: selected_semantic_variant.to_owned(),
        role: intent.role,
        expected_state_version: intent.expected_state_version,
        fixed_arguments,
        fixed_argument_paths,
        suggested_arguments,
        required_inputs,
        optional_inputs,
    })
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
    let projection =
        action_form_request_projection(form.method, form.selected_semantic_variant.as_str())
            .ok_or_else(|| {
                format!(
                    "{} {} has no request projection descriptor",
                    form.method.as_str(),
                    form.selected_semantic_variant
                )
            })?;
    let expected_paths = projection.concrete_fixed_argument_paths(&form.fixed_arguments)?;
    if expected_paths != form.fixed_argument_paths {
        return Err(format!(
            "{} current form fixed path set disagrees with its request projection",
            form.method.as_str()
        ));
    }

    let fixed = Value::Object(form.fixed_arguments.clone());
    let tool = AgentToolId::from_method(form.method)
        .ok_or_else(|| format!("{} has no canonical MCP tool", form.method.as_str()))?;
    let request = mcp_tool_contract(tool).ok_or_else(|| {
        format!(
            "{} has no semantic request descriptor",
            form.method.as_str()
        )
    })?;
    let mut mismatches = Vec::new();
    for path in &form.fixed_argument_paths {
        let expected_value = fixed.pointer(path).ok_or_else(|| {
            format!(
                "{} current form omits fixed value {}",
                form.method.as_str(),
                path
            )
        })?;
        if !request
            .input_descriptor()
            .accepts_value_at_pointer(path, expected_value)
        {
            return Err(format!(
                "{} fixed value at {} does not match its request semantic type",
                form.method.as_str(),
                path
            ));
        }
        let received = submitted.pointer(path);
        if received == Some(expected_value) {
            continue;
        }
        mismatches.push(McpActionFormArgumentMismatch {
            method: form.method,
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

pub(crate) fn workflow_action_form_catalog(
    project_id: &ProjectId,
    workflow: &WorkflowProjection,
) -> WorkflowActionFormCatalog {
    WorkflowActionFormCatalog {
        required_method: workflow.action_catalog().required_method.clone(),
        forms: workflow
            .action_catalog()
            .actions
            .iter()
            .map(|intent| {
                workflow_action_form(project_id, intent).unwrap_or_else(|| {
                    panic!(
                        "workflow action {} has no canonical MCP form",
                        intent.method.as_str()
                    )
                })
            })
            .collect(),
    }
}

pub(crate) fn retry_contract(
    form: &WorkflowActionForm,
    catalog: &WorkflowActionFormCatalog,
    invalid_paths: Vec<String>,
) -> RetryContract {
    RetryContract {
        method: form.method,
        action_form_ref: RequiredNullable::some(form.form_ref.clone()),
        fixed_arguments: form.fixed_arguments.clone(),
        fixed_argument_paths: form.fixed_argument_paths.clone(),
        invalid_paths,
        required_inputs: form.required_inputs.clone(),
        action_form_catalog: catalog.clone(),
        corrected_retry_allowed: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use volicord_types::canonical::is_canonical_sha256_digest;
    use volicord_types::ids::{
        BaselineRef, ChangeUnitId, RecordId, ShapingCheckpointId, UserActionResolutionId,
    };
    use volicord_types::schema::{StateRecordRef, WorkflowActionRole};
    use volicord_types::values::{RunKind, StateRecordKind};

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

    #[test]
    fn first_checkpoint_form_fixes_null_baseline_and_exact_input_slots() {
        let project_id = ProjectId::new("prj_action_form");
        let task_id = TaskId::new("task_action_form");
        let intent = WorkflowActionIntent {
            method: MethodName::RecordShapingCheckpoint,
            role: WorkflowActionRole::Required,
            expected_state_version: 2,
            fixed_authority_coordinates:
                WorkflowActionAuthorityCoordinates::RecordShapingCheckpoint {
                    task_id: task_id.clone(),
                    checkpoint_operation: WorkflowCheckpointActionCoordinates::CreateInitial,
                    scope_revision: 0,
                    baseline_ref: RequiredNullable::null(),
                },
            required_refs: vec![reference(
                StateRecordKind::Task,
                task_id.as_str(),
                &project_id,
                &task_id,
                2,
            )],
        };

        let form = workflow_action_form(&project_id, &intent).expect("current form");
        assert!(is_canonical_sha256_digest(form.form_ref.as_str()));
        assert_eq!(form.method, MethodName::RecordShapingCheckpoint);
        assert_eq!(form.expected_state_version, 2);
        assert_eq!(
            Value::Object(form.fixed_arguments),
            json!({
                "task_id": "task_action_form",
                "checkpoint_operation": { "operation": "create_initial" },
                "scope_revision": 0,
                "baseline_ref": null,
            })
        );
        assert_eq!(
            Value::Object(form.suggested_arguments),
            json!({"source_refs": [], "evidence_refs": []})
        );
        assert_eq!(
            form.required_inputs,
            vec![
                input("/summary", "string"),
                input("/implementation_boundary", "string | null"),
                input("/gaps", "array<ShapingGapInput>"),
            ]
        );
    }

    #[test]
    fn action_form_inputs_use_descriptor_owned_types_and_required_nullable_presence() {
        let project_id = ProjectId::new("prj_descriptor_inputs");
        let intent = WorkflowActionIntent {
            method: MethodName::UpdateScope,
            role: WorkflowActionRole::Allowed,
            expected_state_version: 3,
            fixed_authority_coordinates: WorkflowActionAuthorityCoordinates::UpdateScope {
                task_id: TaskId::new("task_descriptor_inputs"),
                scope_revision: 1,
                baseline_ref: RequiredNullable::some(BaselineRef::new("baseline_current")),
                current_change_unit_id: RequiredNullable::some(ChangeUnitId::new("cu_current")),
                related_scope_decision_refs: Vec::new(),
            },
            required_refs: Vec::new(),
        };

        let form = workflow_action_form(&project_id, &intent).expect("current form");
        let required = form
            .required_inputs
            .iter()
            .map(|input| (input.path.as_str(), input.semantic_type.as_str()))
            .collect::<BTreeMap<_, _>>();
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
        assert_eq!(
            form.optional_inputs,
            vec![input(
                "/change_unit/effect_contract",
                "ChangeUnitEffectContract | null"
            )]
        );
    }

    #[test]
    fn create_change_unit_form_resolves_type_owned_flattened_fields() {
        let project_id = ProjectId::new("prj_create_change_unit_form");
        let intent = WorkflowActionIntent {
            method: MethodName::UpdateScope,
            role: WorkflowActionRole::Allowed,
            expected_state_version: 1,
            fixed_authority_coordinates: WorkflowActionAuthorityCoordinates::UpdateScope {
                task_id: TaskId::new("task_create_change_unit_form"),
                scope_revision: 0,
                baseline_ref: RequiredNullable::null(),
                current_change_unit_id: RequiredNullable::null(),
                related_scope_decision_refs: Vec::new(),
            },
            required_refs: Vec::new(),
        };

        let form = workflow_action_form(&project_id, &intent).expect("create-current form");
        let required = form
            .required_inputs
            .iter()
            .map(|input| (input.path.as_str(), input.semantic_type.as_str()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(required.get("/change_unit/scope_summary"), Some(&"string"));
        assert_eq!(
            required.get("/change_unit/affected_paths"),
            Some(&"array<string>")
        );
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
        let intent = WorkflowActionIntent {
            method: MethodName::RecordShapingCheckpoint,
            role: WorkflowActionRole::Required,
            expected_state_version: 9,
            fixed_authority_coordinates:
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
                    baseline_ref: RequiredNullable::some(BaselineRef::new("baseline_current")),
                },
            required_refs: Vec::new(),
        };

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
        let intent = WorkflowActionIntent {
            method: MethodName::FinalizeAdvice,
            role: WorkflowActionRole::Required,
            expected_state_version: 12,
            fixed_authority_coordinates: WorkflowActionAuthorityCoordinates::FinalizeAdvice {
                task_id,
                shaping_checkpoint_id: ShapingCheckpointId::new("checkpoint_advisor"),
                change_unit_id: ChangeUnitId::new("change_unit_advisor"),
                scope_revision: 4,
                baseline_ref: RequiredNullable::some(BaselineRef::new("baseline_advisor")),
                user_action_resolution_ids: vec![UserActionResolutionId::new("resolution_advisor")],
            },
            required_refs: Vec::new(),
        };
        let form = workflow_action_form(&project_id, &intent).expect("advisor form");
        assert_eq!(
            form.fixed_arguments["user_action_resolution_ids"],
            json!(["resolution_advisor"])
        );
        assert_eq!(
            form.required_inputs,
            vec![input("/result_summary", "string")]
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
        let baseline = BaselineRef::new("baseline_binding_table");
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
                scope_revision: 4,
                baseline_ref: RequiredNullable::some(baseline.clone()),
                current_change_unit_id: RequiredNullable::some(change_unit.clone()),
                related_scope_decision_refs: vec![resolution_ref],
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
            let form = workflow_action_form(
                &project_id,
                &WorkflowActionIntent {
                    method,
                    role: WorkflowActionRole::Allowed,
                    expected_state_version: 21,
                    fixed_authority_coordinates,
                    required_refs: Vec::new(),
                },
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

            for path in &form.fixed_argument_paths {
                let expected = exact.pointer(path).expect("fixed value").clone();
                let mut mutated = exact.clone();
                *mutated.pointer_mut(path).expect("fixed value") = altered(&expected);
                let mismatch = bind_fixed_arguments(&form, &mutated)
                    .expect("binding contract")
                    .mismatches;
                assert_eq!(mismatch.len(), 1, "{} {path}", method.as_str());
                assert_eq!(mismatch[0].path, *path);
                assert_eq!(mismatch[0].expected_value, expected);
                assert!(mismatch[0].received_value_present);
                assert!(!mismatch[0].reached_core);
                assert!(!mismatch[0].state_change_applied);

                let mut omitted = exact.clone();
                remove_pointer(&mut omitted, path);
                let mismatch = bind_fixed_arguments(&form, &omitted)
                    .expect("binding contract")
                    .mismatches;
                assert_eq!(mismatch.len(), 1, "{} omitted {path}", method.as_str());
                assert_eq!(mismatch[0].path, *path);
                assert!(!mismatch[0].received_value_present);
            }
        }
    }
}
