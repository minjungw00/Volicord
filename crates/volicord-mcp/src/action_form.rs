//! MCP workflow action forms derived from neutral Core-owned transitions.

use serde::Serialize;
use serde_json::{json, Map, Value};
use volicord_mcp_wire::{
    action_form_request_projection, mcp_tool_contract, submitted_action_form_semantic_variant,
    McpActionFormArgumentMismatch, McpInputContractValidation, RetryContract,
    SemanticSchemaDescriptor, WorkflowActionForm, WorkflowActionFormCatalog, WorkflowActionInput,
    MAX_VALIDATION_ISSUES,
};
use volicord_types::canonical::canonical_json_sha256;
use volicord_types::ids::{ProjectId, RequestHash, TaskId};
use volicord_types::schema::{
    JsonObject, RequiredNullable, TransitionDescriptor, WorkflowActionAuthorityCoordinates,
    WorkflowActionKey, WorkflowCheckpointActionCoordinates, WorkflowProjection,
};
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::{ChangeUnitOperation, MethodName};

#[derive(Serialize)]
struct WorkflowActionFormDigestBasis<'a> {
    domain: &'static str,
    project_id: &'a ProjectId,
    task_id: &'a TaskId,
    action_key: WorkflowActionKey,
    expected_state_version: u64,
    fixed_authority_coordinates: &'a WorkflowActionAuthorityCoordinates,
    fixed_arguments: &'a JsonObject,
    fixed_argument_paths: &'a [String],
    semantic_schema_digest: &'a RequestHash,
    scalar_contract_digest: &'a RequestHash,
    workflow_contract_digest: &'a RequestHash,
    action_form_contract_digest: &'a RequestHash,
}

fn input(path: &str, semantic_type: &str) -> WorkflowActionInput {
    WorkflowActionInput {
        path: path.to_owned(),
        semantic_type: semantic_type.to_owned(),
        required: false,
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
        let required = contracts.iter().all(|field| field.required);
        Some((
            WorkflowActionInput {
                path: authored.path,
                semantic_type: semantic_types,
                required,
            },
            required,
        ))
    }

    let mut descriptor_required = Vec::new();
    for authored in required {
        let mut authored = typed_input(descriptor, authored)?.0;
        authored.required = true;
        descriptor_required.push(authored);
    }
    let mut descriptor_optional = Vec::new();
    for authored in optional {
        let (mut authored, is_required) = typed_input(descriptor, authored)?;
        if is_required && !authored.path.ends_with("/successor_gap") {
            descriptor_required.push(authored);
        } else {
            authored.required = false;
            descriptor_optional.push(authored);
        }
    }
    Some((descriptor_required, descriptor_optional))
}

fn checkpoint_operation(
    operation: &WorkflowCheckpointActionCoordinates,
) -> (Value, Vec<WorkflowActionInput>, Vec<WorkflowActionInput>) {
    match operation {
        WorkflowCheckpointActionCoordinates::CreateInitial => (
            json!({ "operation": "create_initial" }),
            Vec::new(),
            Vec::new(),
        ),
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
            let required = stale_application_refs
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    input(
                        &format!("/checkpoint_operation/stale_authority_actions/{index}/action"),
                        "retire | reauthorize",
                    )
                })
                .collect::<Vec<_>>();
            let optional = if stale_application_refs.is_empty() {
                Vec::new()
            } else {
                vec![input(
                    "/checkpoint_operation/stale_authority_actions/*/successor_gap",
                    "ShapingGapInput when action=reauthorize",
                )]
            };
            (
                json!({
                    "operation": "replace_current",
                    "expected_current_checkpoint_id": current_checkpoint_ref.record_id.as_str(),
                    "retired_non_authorizing_request_refs": retired_non_authorizing_request_refs,
                    "carry_forward_application_refs": carry_forward_application_refs,
                    "stale_authority_actions": stale_authority_actions,
                }),
                required,
                optional,
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
    Vec<WorkflowActionInput>,
    Vec<WorkflowActionInput>,
) {
    let mut fixed = Map::new();
    match coordinates {
        WorkflowActionAuthorityCoordinates::RecordShapingCheckpoint {
            task_id,
            checkpoint_operation: operation,
            scope_revision,
            baseline_ref,
        } => {
            let (operation_value, mut conditional_inputs, conditional_optional_inputs) =
                checkpoint_operation(operation);
            fixed.insert("task_id".to_owned(), json!(task_id));
            fixed.insert("checkpoint_operation".to_owned(), operation_value);
            fixed.insert("scope_revision".to_owned(), json!(scope_revision));
            fixed.insert("baseline_ref".to_owned(), json!(baseline_ref));
            let mut required = vec![
                input("/summary", "string"),
                input("/implementation_boundary", "string | null"),
                input("/gaps", "array<ShapingGapInput>"),
            ];
            required.append(&mut conditional_inputs);
            (
                task_id.clone(),
                fixed,
                required,
                vec![
                    input("/source_refs", "array<SourceRef>"),
                    input("/evidence_refs", "array<StateRecordRef>"),
                ]
                .into_iter()
                .chain(conditional_optional_inputs)
                .collect(),
            )
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
            let mut required = vec![input("/baseline_ref", "string | null")];
            fixed.insert(
                "change_unit".to_owned(),
                json!({ "operation": selected_change_unit_operation }),
            );
            if matches!(
                selected_change_unit_operation,
                ChangeUnitOperation::CreateCurrent | ChangeUnitOperation::ReplaceCurrent
            ) {
                required.extend([
                    input("/change_unit/scope_summary", "string"),
                    input("/change_unit/affected_paths", "array<string>"),
                ]);
            }
            (
                task_id.clone(),
                fixed,
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
            (task_id.clone(), fixed, Vec::new(), Vec::new())
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
                vec![
                    input("/request/action", "UserActionDraft"),
                    input("/request/required_for", "array<UserActionRequiredFor>"),
                ],
                vec![input("/request/expires_at", "UtcTimestamp | null")],
            )
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
            (task_id.clone(), fixed, Vec::new(), Vec::new())
        }
        WorkflowActionAuthorityCoordinates::ReconcileChanges { task_id } => {
            fixed.insert("task_id".to_owned(), json!(task_id));
            (
                task_id.clone(),
                fixed,
                Vec::new(),
                vec![input(
                    "/resolution_requests",
                    "array<UnrecordedChangeResolutionRequest>",
                )],
            )
        }
        WorkflowActionAuthorityCoordinates::CheckClose { task_id } => {
            fixed.insert("task_id".to_owned(), json!(task_id));
            (task_id.clone(), fixed, Vec::new(), Vec::new())
        }
        WorkflowActionAuthorityCoordinates::CloseTask { task_id } => {
            fixed.insert("task_id".to_owned(), json!(task_id));
            (
                task_id.clone(),
                fixed,
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

fn copy_required_authored_input(
    request: &mut Value,
    examples: &[&Value],
    authored: &WorkflowActionInput,
) -> Result<(), String> {
    let pattern = authored.path.as_str();
    if let Some((prefix, suffix)) = pattern.split_once("/*") {
        let current_len = request
            .pointer(prefix)
            .and_then(Value::as_array)
            .map(Vec::len)
            .ok_or_else(|| format!("action-form authored wildcard prefix {prefix} is absent"))?;
        let example_values = examples
            .iter()
            .filter_map(|example| example.pointer(prefix).and_then(Value::as_array))
            .find(|values| !values.is_empty())
            .ok_or_else(|| format!("canonical example wildcard prefix {prefix} is absent"))?;
        for index in 0..current_len {
            let source_pointer = suffix.strip_prefix('/').unwrap_or(suffix);
            let value = example_values
                .first()
                .and_then(|source| source.pointer(&format!("/{source_pointer}")))
                .cloned()
                .ok_or_else(|| format!("canonical example omits authored input {pattern}"))?;
            insert_pointer(request, &format!("{prefix}/{index}{suffix}"), value)?;
        }
        return Ok(());
    }
    let value = examples
        .iter()
        .find_map(|example| example.pointer(pattern).cloned())
        .ok_or_else(|| format!("canonical example omits authored input {pattern}"))?;
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
pub(crate) fn workflow_action_form(
    project_id: &ProjectId,
    workflow: &WorkflowProjection,
    transition: &TransitionDescriptor,
) -> Result<Option<WorkflowActionForm>, String> {
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
    let (task_id, fixed_arguments, required_inputs, optional_inputs) =
        project_fixed_arguments(project_id, &transition.fixed_authority_coordinates);
    let (required_inputs, optional_inputs) = descriptor_owned_inputs(
        descriptor.input_descriptor(),
        required_inputs,
        optional_inputs,
    )
    .ok_or_else(|| {
        format!(
            "{} authored input descriptor is incomplete",
            method.as_str()
        )
    })?;
    let mut agent_authored_inputs = required_inputs;
    agent_authored_inputs.extend(optional_inputs);
    agent_authored_inputs.sort_by(|left, right| left.path.cmp(&right.path));
    let selected_semantic_variant = transition.action_key.semantic_variant;
    let projection =
        action_form_request_projection(method, selected_semantic_variant).ok_or_else(|| {
            format!(
                "{} {} has no action-form request projection",
                method.as_str(),
                selected_semantic_variant.as_str()
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
    volicord_core::model_check_current_transition(workflow, transition).map_err(str::to_owned)?;
    let form_ref = canonical_json_sha256(&WorkflowActionFormDigestBasis {
        domain: "volicord.mcp-workflow-action-form",
        project_id,
        task_id: &task_id,
        action_key: transition.action_key,
        expected_state_version: transition.expected_state_version,
        fixed_authority_coordinates: &transition.fixed_authority_coordinates,
        fixed_arguments: &fixed_arguments,
        fixed_argument_paths: &fixed_argument_paths,
        semantic_schema_digest: &semantic_schema_digest,
        scalar_contract_digest: &scalar_contract_digest,
        workflow_contract_digest: &workflow_contract_digest,
        action_form_contract_digest: &action_form_contract_digest,
    })
    .map_err(|error| error.to_string())?;
    let matching_examples = descriptor
        .canonical_examples()
        .iter()
        .filter(|example| {
            submitted_action_form_semantic_variant(method, example.value())
                == Some(selected_semantic_variant)
        })
        .map(|example| example.value())
        .collect::<Vec<_>>();
    if matching_examples.is_empty() {
        return Err(format!(
            "{} {} has no canonical semantic example",
            method.as_str(),
            selected_semantic_variant.as_str()
        ));
    }
    let mut canonical_minimal_request = fixed;
    for authored in agent_authored_inputs.iter().filter(|input| input.required) {
        copy_required_authored_input(&mut canonical_minimal_request, &matching_examples, authored)?;
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
    if submitted_action_form_semantic_variant(method, &canonical_minimal_request)
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
    Ok(Some(form))
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
    let semantic_variant = form.action_key.semantic_variant;
    let projection = action_form_request_projection(method, semantic_variant).ok_or_else(|| {
        format!(
            "{} {} has no request projection descriptor",
            method.as_str(),
            semantic_variant.as_str()
        )
    })?;
    let expected_paths = projection.concrete_fixed_argument_paths(&form.fixed_arguments)?;

    let fixed = Value::Object(form.fixed_arguments.clone());
    let tool = AgentToolId::from_method(method)
        .ok_or_else(|| format!("{} has no canonical MCP tool", method.as_str()))?;
    let request = mcp_tool_contract(tool)
        .ok_or_else(|| format!("{} has no semantic request descriptor", method.as_str()))?;
    let mut mismatches = Vec::new();
    for path in &expected_paths {
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

pub(crate) fn workflow_action_form_catalog(
    project_id: &ProjectId,
    workflow: &WorkflowProjection,
) -> Result<WorkflowActionFormCatalog, String> {
    let mut forms = Vec::new();
    for transition in &workflow.transition_catalog().transitions {
        if transition.actor != volicord_types::values::WorkflowTransitionActor::Agent {
            continue;
        }
        let form = workflow_action_form(project_id, workflow, transition)?.ok_or_else(|| {
            format!(
                "Agent transition {} {} did not produce an MCP action form",
                transition.action_key.method.as_str(),
                transition.action_key.semantic_variant.as_str()
            )
        })?;
        forms.push(form);
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
        return Err(
            "the MCP action-form catalog is not a total Agent-transition projection".to_owned(),
        );
    }
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

pub(crate) fn retry_contract(
    attempted_action_key: WorkflowActionKey,
    recovery_action_key: WorkflowActionKey,
    workflow: &WorkflowProjection,
    catalog: &WorkflowActionFormCatalog,
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
        invalid_or_incompatible_submitted_paths,
        retry_possible_in_current_task,
    })
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
    use volicord_types::values::{
        AuthorityNextActor, RunKind, StateRecordKind, WorkflowActionSemanticVariant,
    };

    fn workflow_action_form(
        project_id: &ProjectId,
        transition: &TransitionDescriptor,
    ) -> Option<WorkflowActionForm> {
        let workflow = workflow_for_transitions(vec![transition.clone()]);
        super::workflow_action_form(project_id, &workflow, transition)
            .expect("valid action-form projection")
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
        action_form_request_projection(form.action_key.method, form.action_key.semantic_variant)
            .expect("test form projection")
            .concrete_fixed_argument_paths(&form.fixed_arguments)
            .expect("test fixed paths")
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
            WorkflowAgentInputRequirement as Input, WorkflowExpectedResultState as ResultState,
            WorkflowTransitionEffectClass as Effect,
        };
        let agent_input_requirements = match method {
            MethodName::RecordShapingCheckpoint => vec![Input::ShapingCheckpoint],
            MethodName::UpdateScope => vec![Input::ScopeAndChangeUnit],
            MethodName::FinalizeAdvice => vec![Input::AdviceResult],
            MethodName::AdvanceTask | MethodName::CheckClose => Vec::new(),
            MethodName::PrepareEvidenceCapture => vec![Input::EvidenceCapture],
            MethodName::PrepareWrite => vec![Input::ProposedWrite],
            MethodName::StageArtifact => vec![Input::Artifact],
            MethodName::RecordRun => vec![Input::RunObservation],
            MethodName::RequestUserAction => vec![Input::UserActionDraft],
            MethodName::ReconcileChanges => vec![Input::ChangeReconciliation],
            MethodName::CloseTask => vec![Input::CloseIntent],
            _ => panic!("test helper requires an Agent transition method"),
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
        TransitionDescriptor {
            action_key: volicord_types::schema::WorkflowActionKey::new(method, semantic_variant)
                .expect("test transition key"),
            actor: volicord_types::values::WorkflowTransitionActor::Agent,
            role,
            expected_state_version,
            fixed_authority_coordinates,
            agent_input_requirements,
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
        let optional = authored_inputs(&form, false);
        assert_eq!(
            optional.get("/change_unit/effect_contract"),
            Some(&"ChangeUnitEffectContract | null")
        );
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
    }

    #[test]
    fn current_change_unit_projects_distinct_keep_and_replace_forms() {
        let project_id = ProjectId::new("prj_current_change_unit_forms");
        let coordinates = |operation| WorkflowActionAuthorityCoordinates::UpdateScope {
            task_id: TaskId::new("task_current_change_unit_forms"),
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
        let catalog = workflow_action_form_catalog(&project_id, &workflow).expect("catalog");

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
        let mut catalog = workflow_action_form_catalog(&project_id, &workflow).expect("catalog");
        let external_close = retry_contract(
            attempted.action_key,
            close.action_key,
            &workflow,
            &catalog,
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
            Vec::new(),
        )
        .expect_err("missing exact recovery form must not fall back");
        assert!(error.contains("missing its current MCP action form"));
    }
}
