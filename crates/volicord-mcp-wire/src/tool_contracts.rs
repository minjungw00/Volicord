//! Canonical method/tool contracts assembled from semantic schema descriptors.

use std::collections::BTreeSet;
use std::sync::OnceLock;

#[cfg(test)]
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{Map, Value};
use volicord_types::ids::{
    AcceptanceCriterionId, BaselineRef, ChangeUnitId, IdempotencyKey, ProjectId, RecordId,
    RequestHash, ShapingCheckpointId, TaskId, UserActionOptionId, UserActionRequestId,
    UserActionResolutionId,
};
use volicord_types::integration_verification::{
    BeginIntegrationVerificationArguments, BeginIntegrationVerificationResult,
    GetIntegrationVerificationResult, GuardProbeResult, IntegrationVerificationIdArguments,
};
use volicord_types::methods::{
    AdvanceTaskResponse, ChangeUnitUpdate, CheckCloseResponse, CloseTaskResponse,
    FinalizeAdviceResponse, GetOperationResultResponse, InitialScope, IntakeResponse,
    OperationResultRef, PrepareEvidenceCaptureResponse, PrepareWriteResponse,
    ReconcileChangesResponse, RecordRunResponse, RecordShapingCheckpointResponse, ScopeUpdate,
    StageArtifactResponse, UpdateScopeResponse,
};
use volicord_types::schema::{
    advisor_observe_only_effect_contract, AcceptanceCriterionInput, AcceptanceCriterionReplacement,
    CloseAssessmentInput, EvidenceTarget, ObservedChanges, RepositoryFileSource, RequiredNullable,
    ResidualRiskInput, SensitiveActionScope, ShapingCheckpointOperation, ShapingGapInput,
    ShapingUserActionDraft, SourceLineRange, SourceRef, StaleShapingAuthorityAction,
    StateRecordRef, UserActionChoiceDraft, UserActionContext, UserActionDraft,
    UserActionOptionInput,
};
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::{
    ChangeUnitOperation, CloseMutationIntent, CloseReason, EvidenceAssuranceLevel,
    EvidenceCoverageUpdateState, EvidenceRequirement, EvidenceSourceKind, JudgmentKind,
    JudgmentPresentation, MethodName, MutationDetailLevel, RedactionState, RequestedControlLevel,
    RequestedMode, ResumePolicy, RunKind, ShapingGapKind, StateRecordKind, StatusDetailLevel,
    UserActionRequiredFor,
};

use crate::methods::*;
use crate::semantic_schema::{
    mcp_tagged_union_contract_integrity_errors, CanonicalSchemaExample, ExpectedTaggedVariant,
    McpSemanticSchema, SemanticSchemaDescriptor, SemanticValidationResult,
};

pub const UPDATE_SCOPE_KEEP_CURRENT_EXAMPLE_ID: &str = "keep_current_change_unit";
pub const STATUS_READ_ONLY_EXAMPLE_ID: &str = "read_only_status";
pub const GET_OPERATION_RESULT_FIRST_PAGE_EXAMPLE_ID: &str = "first_operation_result_page";
pub const PREPARE_WRITE_SIMPLE_EXAMPLE_ID: &str = "simple_prepare_write";
pub const PREPARE_EVIDENCE_CAPTURE_VERIFIED_COMMAND_EXAMPLE_ID: &str = "verified_command_capture";
pub const PREPARE_EVIDENCE_CAPTURE_VERIFIED_TOOL_EXAMPLE_ID: &str = "verified_tool_capture";
pub const RECORD_RUN_EVIDENCE_BEARING_EXAMPLE_ID: &str = "evidence_bearing_record_run";
pub const REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID: &str = "final_acceptance_request";
pub const CHECK_CLOSE_MISSING_FINAL_ACCEPTANCE_EXAMPLE_ID: &str =
    "check_close_missing_final_acceptance";

type DecodeInput = fn(&Value) -> Result<Value, String>;

fn example_action_form_ref() -> RequestHash {
    RequestHash::new("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
}

/// Closed result of descriptor validation followed by exact Rust input decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpInputContractValidation {
    Valid,
    Invalid(SemanticValidationResult),
    SchemaContractFailure,
}

/// One production MCP tool's canonical semantic contract entry.
#[derive(Debug, Clone)]
pub struct McpToolContractDescriptor {
    tool: AgentToolId,
    documentation_description: &'static str,
    compact_description: &'static str,
    input: SemanticSchemaDescriptor,
    output: SemanticSchemaDescriptor,
    decode_input: DecodeInput,
    construction_errors: Vec<String>,
}

impl McpToolContractDescriptor {
    pub const fn tool(&self) -> AgentToolId {
        self.tool
    }

    pub const fn documentation_description(&self) -> &'static str {
        self.documentation_description
    }

    pub const fn compact_description(&self) -> &'static str {
        self.compact_description
    }

    pub const fn input_descriptor(&self) -> &SemanticSchemaDescriptor {
        &self.input
    }

    pub const fn output_descriptor(&self) -> &SemanticSchemaDescriptor {
        &self.output
    }

    pub fn input_schema(&self) -> Value {
        self.input.json_schema()
    }

    pub fn runtime_input_schema(&self) -> Value {
        self.input.runtime_json_schema()
    }

    pub fn output_schema(&self) -> Value {
        self.output.json_schema()
    }

    pub fn compact_output_schema(&self) -> Value {
        self.output.compact_root_object_schema()
    }

    pub fn canonical_examples(&self) -> &[CanonicalSchemaExample] {
        self.input.canonical_examples()
    }

    /// Decodes and reserializes a value as this entry's exact Rust input type.
    pub fn decode_input(&self, value: &Value) -> Result<Value, String> {
        (self.decode_input)(value)
    }

    /// Validates one input and, only when valid, requires exact Rust decoding to agree.
    pub fn validate_and_decode_input(&self, value: &Value) -> McpInputContractValidation {
        let validation = self.input.validate(value);
        if !validation.issues.is_empty() {
            return McpInputContractValidation::Invalid(validation);
        }
        match self.decode_input(value) {
            Ok(_) => McpInputContractValidation::Valid,
            Err(_) => McpInputContractValidation::SchemaContractFailure,
        }
    }

    /// Validates schema, example, decode, and deterministic-generation integrity.
    pub fn integrity_errors(&self) -> Vec<String> {
        let mut errors = self.construction_errors.clone();
        errors.extend(
            self.input
                .integrity_errors()
                .into_iter()
                .map(|error| format!("{} input: {error}", self.tool.wire_name()))
                .chain(
                    self.output
                        .integrity_errors()
                        .into_iter()
                        .map(|error| format!("{} output: {error}", self.tool.wire_name())),
                )
                .collect::<Vec<_>>(),
        );
        for example in self.canonical_examples() {
            match self.decode_input(example.value()) {
                Ok(round_trip) if round_trip == *example.value() => {}
                Ok(round_trip) => errors.push(format!(
                    "{} example `{}` decoded but reserialized as {}",
                    self.tool.wire_name(),
                    example.id(),
                    round_trip
                )),
                Err(error) => errors.push(format!(
                    "{} example `{}` failed exact Rust decoding: {error}",
                    self.tool.wire_name(),
                    example.id()
                )),
            }
        }
        let first = self.input_schema();
        let second = self.input_schema();
        if first != second {
            errors.push(format!(
                "{} input schema generation is nondeterministic",
                self.tool.wire_name()
            ));
        }
        let first_schema_digest = self.input.schema_digest();
        let second_schema_digest = self.input.schema_digest();
        let first_descriptor_digest = self.input.descriptor_digest();
        let second_descriptor_digest = self.input.descriptor_digest();
        if first_schema_digest != second_schema_digest
            || first_descriptor_digest != second_descriptor_digest
        {
            errors.push(format!(
                "{} input descriptor digest is nondeterministic",
                self.tool.wire_name()
            ));
        }
        let first = self.output_schema();
        let second = self.output_schema();
        if first != second {
            errors.push(format!(
                "{} output schema generation is nondeterministic",
                self.tool.wire_name()
            ));
        }
        let first_schema_digest = self.output.schema_digest();
        let second_schema_digest = self.output.schema_digest();
        let first_descriptor_digest = self.output.descriptor_digest();
        let second_descriptor_digest = self.output.descriptor_digest();
        if first_schema_digest != second_schema_digest
            || first_descriptor_digest != second_descriptor_digest
        {
            errors.push(format!(
                "{} output descriptor digest is nondeterministic",
                self.tool.wire_name()
            ));
        }
        errors
    }
}

/// Returns the one canonical entry for a production MCP tool.
pub fn mcp_tool_contract(tool: AgentToolId) -> Option<&'static McpToolContractDescriptor> {
    mcp_tool_contracts()
        .iter()
        .find(|descriptor| descriptor.tool == tool)
}

/// Returns the complete closed production MCP contract registry.
pub fn mcp_tool_contracts() -> &'static [McpToolContractDescriptor] {
    static CONTRACTS: OnceLock<Vec<McpToolContractDescriptor>> = OnceLock::new();
    CONTRACTS.get_or_init(|| {
        AgentToolId::ALL
            .iter()
            .copied()
            .map(build_tool_contract)
            .collect()
    })
}

/// Checks registry identity and every descriptor-owned integrity rule.
pub fn mcp_tool_contract_integrity_errors() -> Vec<String> {
    let contracts = mcp_tool_contracts();
    let mut errors = Vec::new();
    errors.extend(mcp_tagged_union_contract_integrity_errors());
    let mut tools = BTreeSet::new();
    for contract in contracts {
        if !tools.insert(contract.tool.wire_name()) {
            errors.push(format!(
                "duplicate MCP semantic contract for {}",
                contract.tool.wire_name()
            ));
        }
        errors.extend(contract.integrity_errors());
    }
    for tool in AgentToolId::ALL {
        if !tools.contains(tool.wire_name()) {
            errors.push(format!(
                "missing MCP semantic contract for {}",
                tool.wire_name()
            ));
        }
    }
    errors.extend(crate::action_form::action_form_request_projection_integrity_errors());
    errors
}

fn contract<I, O>(
    tool: AgentToolId,
    documentation_description: &'static str,
    compact_description: &'static str,
    examples: Vec<CanonicalSchemaExample>,
) -> McpToolContractDescriptor
where
    I: McpSemanticSchema + Serialize + DeserializeOwned,
    O: McpSemanticSchema,
{
    McpToolContractDescriptor {
        tool,
        documentation_description,
        compact_description,
        input: SemanticSchemaDescriptor::for_type::<I>(examples),
        output: SemanticSchemaDescriptor::for_object_output::<O>(Vec::new()),
        decode_input: decode_round_trip::<I>,
        construction_errors: Vec::new(),
    }
}

fn checked_contract<I, O>(
    tool: AgentToolId,
    documentation_description: &'static str,
    compact_description: &'static str,
    examples: Result<Vec<CanonicalSchemaExample>, String>,
) -> McpToolContractDescriptor
where
    I: McpSemanticSchema + Serialize + DeserializeOwned,
    O: McpSemanticSchema,
{
    match examples {
        Ok(examples) => contract::<I, O>(
            tool,
            documentation_description,
            compact_description,
            examples,
        ),
        Err(error) => {
            let mut descriptor = contract::<I, O>(
                tool,
                documentation_description,
                compact_description,
                Vec::new(),
            );
            descriptor.construction_errors.push(format!(
                "{} canonical example construction failed: {error}",
                tool.wire_name()
            ));
            descriptor
        }
    }
}

fn example_baseline_ref(value: impl Into<String>) -> Result<BaselineRef, String> {
    BaselineRef::parse(value).map_err(|error| error.to_string())
}

fn decode_round_trip<T: Serialize + DeserializeOwned>(value: &Value) -> Result<Value, String> {
    let decoded = serde_json::from_value::<T>(value.clone()).map_err(|error| error.to_string())?;
    serde_json::to_value(decoded).map_err(|error| error.to_string())
}

fn build_tool_contract(tool: AgentToolId) -> McpToolContractDescriptor {
    match tool {
        AgentToolId::INTAKE => contract::<
            McpIntakeArguments,
            McpMutationStructuredContent<IntakeResponse, McpMutationEffectSummary>,
        >(
            tool,
            "Start, resume, supersede, or reject an ordinary user work loop.",
            "Start or resume work.",
            intake_examples(),
        ),
        AgentToolId::UPDATE_SCOPE => checked_contract::<
            McpUpdateScopeArguments,
            McpMutationStructuredContent<UpdateScopeResponse, McpUpdateScopeCompactResult>,
        >(
            tool,
            "Update the current Task scope and keep, create, or replace its current Change Unit.",
            "Update scope and Change Unit.",
            update_scope_examples(),
        ),
        AgentToolId::RECORD_SHAPING_CHECKPOINT => checked_contract::<
            McpRecordShapingCheckpointArguments,
            McpMutationStructuredContent<
                RecordShapingCheckpointResponse,
                McpRecordShapingCheckpointCompactResult,
            >,
        >(
            tool,
            "Atomically record or replace current shaping authority, preserve compatible applications, and retire or reissue exact stale authority with fresh UserAction identity.",
            "Record shaping authority.",
            record_shaping_checkpoint_examples(),
        ),
        AgentToolId::FINALIZE_ADVICE => checked_contract::<
            McpFinalizeAdviceArguments,
            McpMutationStructuredContent<FinalizeAdviceResponse, McpFinalizeAdviceCompactResult>,
        >(
            tool,
            "Apply exact current advisor decisions and finalize the current advisor result and checkpoint-backed close basis.",
            "Finalize advisor results.",
            finalize_advice_examples(),
        ),
        AgentToolId::ADVANCE_TASK => checked_contract::<
            McpAdvanceTaskArguments,
            McpMutationStructuredContent<AdvanceTaskResponse, McpAdvanceTaskCompactResult>,
        >(
            tool,
            "Advance an exact ready work Task checkpoint and current Change Unit into implementation.",
            "Advance work to implementation.",
            advance_task_examples(),
        ),
        AgentToolId::STATUS => contract::<
            McpStatusArguments,
            McpReadOnlyToolStructuredContent<McpStatusResponse>,
        >(
            tool,
            "Read the current Core status view without creating Core authority state.",
            "Read current authority status.",
            status_examples(),
        ),
        AgentToolId::GET_OPERATION_RESULT => contract::<
            McpGetOperationResultArguments,
            McpReadOnlyToolStructuredContent<GetOperationResultResponse>,
        >(
            tool,
            "Read one bounded page of an immutable historical mutation response; read current status separately.",
            "Read a mutation result page.",
            get_operation_result_examples(),
        ),
        AgentToolId::PREPARE_EVIDENCE_CAPTURE => checked_contract::<
            McpPrepareEvidenceCaptureArguments,
            McpMutationStructuredContent<
                PrepareEvidenceCaptureResponse,
                McpPrepareEvidenceCaptureCompactResult,
            >,
        >(
            tool,
            "Create a short-lived, current-basis intent for a registered evidence source. This does not execute the source or record Evidence.",
            "Register evidence capture intent.",
            prepare_evidence_capture_examples(),
        ),
        AgentToolId::PREPARE_WRITE => checked_contract::<
            McpPrepareWriteArguments,
            McpMutationStructuredContent<PrepareWriteResponse, McpPrepareWriteCompactResult>,
        >(
            tool,
            "Check a proposed Product Repository write against current Core scope. The default result includes the decision and any issued write ticket.",
            "Check Product Repository writes.",
            prepare_write_examples(),
        ),
        AgentToolId::STAGE_ARTIFACT => contract::<
            McpStageArtifactArguments,
            McpMutationStructuredContent<StageArtifactResponse, McpStageArtifactCompactResult>,
        >(
            tool,
            "Prepare an Evidence attachment input; staging alone is not recorded Evidence. The default compact result includes the staged handle and expiry.",
            "Stage an Evidence attachment.",
            stage_artifact_examples(),
        ),
        AgentToolId::RECORD_RUN => checked_contract::<
            McpRecordRunArguments,
            McpMutationStructuredContent<RecordRunResponse, McpRecordRunCompactResult>,
        >(
            tool,
            "Record execution and evidence. Mode/kind: direct/direct or work/implementation.",
            "Record work and evidence.",
            record_run_examples(),
        ),
        AgentToolId::REQUEST_USER_ACTION => contract::<
            McpRequestUserActionArguments,
            McpMutationStructuredContent<
                McpRequestUserActionResponse,
                McpRequestUserActionCompactResult,
            >,
        >(
            tool,
            "Create or resume one focused user action. MCP returns only a bounded pending summary; user-owned delivery and resolution use `volicord inbox`.",
            "Create or resume a user action.",
            request_user_action_examples(),
        ),
        AgentToolId::RECONCILE_CHANGES => contract::<
            McpReconcileChangesArguments,
            McpMutationStructuredContent<
                ReconcileChangesResponse,
                McpReconcileChangesCompactResult,
            >,
        >(
            tool,
            "Reconcile unresolved Product Repository changes without agent-only dismissal. The default result includes per-finding outcomes.",
            "Reconcile repository changes.",
            vec![typed_example(
                "reconcile_current_task",
                "Reconcile the current Task without an agent-supplied resolution request.",
                &McpReconcileChangesArguments {
                    project_selector: None,
                    detail: MutationDetailLevel::Full,
                    action_form_ref: example_action_form_ref(),
                    task_id: TaskId::new("task_reconcile_001"),
                    resolution_requests: Vec::new(),
                },
                Vec::new(),
            )],
        ),
        AgentToolId::CHECK_CLOSE => contract::<
            McpCheckCloseArguments,
            McpReadOnlyToolStructuredContent<CheckCloseResponse>,
        >(
            tool,
            "Read current close readiness without requesting a terminal mutation.",
            "Read close readiness.",
            vec![typed_example(
                CHECK_CLOSE_MISSING_FINAL_ACCEPTANCE_EXAMPLE_ID,
                "Read current close readiness for one Task.",
                &McpCheckCloseArguments {
                    project_selector: None,
                    action_form_ref: example_action_form_ref(),
                    task_id: TaskId::new("task_close_001"),
                },
                Vec::new(),
            )],
        ),
        AgentToolId::CLOSE_TASK => contract::<
            McpCloseTaskArguments,
            McpMutationStructuredContent<CloseTaskResponse, McpMutationEffectSummary>,
        >(
            tool,
            "Request the complete, cancel, or supersede terminal path for one Task.",
            "Request a terminal Task state.",
            close_task_examples(),
        ),
        AgentToolId::LIST_PROJECTS => contract::<
            McpListProjectsArguments,
            McpToolStructuredContent<McpListProjectsResult>,
        >(
            tool,
            "List projects explicitly allowed for this MCP connection.",
            "List allowed projects.",
            Vec::new(),
        ),
        AgentToolId::BEGIN_INTEGRATION_VERIFICATION => contract::<
            BeginIntegrationVerificationArguments,
            McpToolStructuredContent<BeginIntegrationVerificationResult>,
        >(
            tool,
            "Create or resume the one immutable integration-verification attempt for the current semantic coordinate; returns the authoritative tagged workflow state and its exact typed operation.",
            "Begin integration verification.",
            Vec::new(),
        ),
        AgentToolId::GUARD_PROBE => contract::<
            IntegrationVerificationIdArguments,
            McpToolStructuredContent<GuardProbeResult>,
        >(
            tool,
            "Record or replay a first-write-wins MCP probe acknowledgement and return the authoritative tagged workflow state without changing Product Repository workflow state; this exact call is observed by Guard PreToolUse and PostToolUse.",
            "Record a Guard probe.",
            Vec::new(),
        ),
        AgentToolId::GET_INTEGRATION_VERIFICATION => contract::<
            IntegrationVerificationIdArguments,
            McpToolStructuredContent<GetIntegrationVerificationResult>,
        >(
            tool,
            "Observe the authoritative tagged workflow state under the semantic host policy; the bounded read may persist a typed terminal repair reason when expected same-turn Guard correlation is absent or incompatible.",
            "Read integration verification.",
            Vec::new(),
        ),
        _ => unreachable!("AgentToolId is a closed production catalog"),
    }
}

fn typed_example<T: Serialize>(
    id: &'static str,
    description: &'static str,
    value: &T,
    expected_variants: Vec<ExpectedTaggedVariant>,
) -> CanonicalSchemaExample {
    CanonicalSchemaExample::from_typed(id, description, value, expected_variants)
        .unwrap_or_else(|error| panic!("typed canonical example `{id}` must serialize: {error}"))
}

fn intake_examples() -> Vec<CanonicalSchemaExample> {
    [
        (
            "create_new",
            "Create a new Task when no active Task should be resumed.",
            "Create an onboarding checklist.",
            RequestedMode::Work,
            ResumePolicy::CreateNew,
            "Onboarding checklist setup.",
            Vec::new(),
            EvidenceRequirement::Required,
        ),
        (
            "resume_active",
            "Resume the active Task.",
            "Continue the active onboarding checklist work.",
            RequestedMode::Auto,
            ResumePolicy::ResumeActive,
            "Continue the current onboarding checklist scope.",
            Vec::new(),
            EvidenceRequirement::NotRequired,
        ),
        (
            "supersede_active",
            "Supersede the active Task with revised work.",
            "Replace the active onboarding work with the revised checklist.",
            RequestedMode::Work,
            ResumePolicy::SupersedeActive,
            "Revised onboarding checklist setup.",
            vec!["Changing account creation.".to_owned()],
            EvidenceRequirement::Required,
        ),
        (
            "reject_if_active",
            "Reject intake when a Task is already active.",
            "Start an onboarding checklist only when no Task is active.",
            RequestedMode::Advisor,
            ResumePolicy::RejectIfActive,
            "Onboarding checklist guidance.",
            Vec::new(),
            EvidenceRequirement::NotRequired,
        ),
    ]
    .into_iter()
    .map(
        |(id, description, request, mode, resume_policy, boundary, non_goals, requirement)| {
            typed_example(
                id,
                description,
                &McpIntakeArguments {
                    project_selector: None,
                    detail: MutationDetailLevel::Summary,
                    plain_language_request: request.to_owned(),
                    requested_mode: mode,
                    requested_control_level: RequestedControlLevel::Auto,
                    resume_policy,
                    acceptance_policy: RequiredNullable::null(),
                    lineage: RequiredNullable::null(),
                    initial_scope: InitialScope {
                        boundary: boundary.to_owned(),
                        non_goals,
                        acceptance_criteria: if id == "resume_active" {
                            Vec::new()
                        } else {
                            vec![AcceptanceCriterionInput {
                                statement: "The checklist outcome is available.".to_owned(),
                                evidence_requirement: requirement,
                            }]
                        },
                    },
                    initial_context_refs: Vec::new(),
                    initial_source_refs: Vec::new(),
                },
                Vec::new(),
            )
        },
    )
    .collect()
}

fn advance_task_examples() -> Result<Vec<CanonicalSchemaExample>, String> {
    Ok(vec![typed_example(
        "enter_implementation",
        "Advance one ready work Task into implementation.",
        &McpAdvanceTaskArguments {
            project_selector: None,
            detail: MutationDetailLevel::Summary,
            action_form_ref: example_action_form_ref(),
            task_id: TaskId::new("task_shape_001"),
            shaping_checkpoint_id: ShapingCheckpointId::new("shaping_checkpoint_001"),
            change_unit_id: ChangeUnitId::new("change_unit_001"),
            scope_revision: 4,
            baseline_ref: example_baseline_ref("baseline_shape_001")?,
            user_action_resolution_ids: Vec::new(),
        },
        Vec::new(),
    )])
}

fn update_scope_examples() -> Result<Vec<CanonicalSchemaExample>, String> {
    let keep = McpUpdateScopeArguments {
        project_selector: None,
        detail: MutationDetailLevel::Summary,
        action_form_ref: example_action_form_ref(),
        task_id: TaskId::new("task_filter_001"),
        goal_summary: RequiredNullable::null(),
        scope_update: RequiredNullable::null(),
        scope_boundary: RequiredNullable::null(),
        non_goals: RequiredNullable::null(),
        acceptance_criteria: RequiredNullable::null(),
        autonomy_boundary: RequiredNullable::null(),
        baseline_ref: RequiredNullable::null(),
        change_unit: ChangeUnitUpdate {
            operation: ChangeUnitOperation::KeepCurrent,
            effect_contract: None,
            fields: Map::new(),
        },
        related_scope_decision_refs: Vec::new(),
    };
    let changed = |task: &str,
                   operation,
                   boundary: &str,
                   path: &str|
     -> Result<McpUpdateScopeArguments, String> {
        let mut fields = Map::new();
        fields.insert(
            "scope_summary".to_owned(),
            Value::String("Saved-filter validation.".to_owned()),
        );
        fields.insert(
            "affected_paths".to_owned(),
            Value::Array(vec![Value::String(path.to_owned())]),
        );
        Ok(McpUpdateScopeArguments {
            action_form_ref: example_action_form_ref(),
            project_selector: None,
            detail: MutationDetailLevel::Summary,
            task_id: TaskId::new(task),
            goal_summary: RequiredNullable::some("Limit saved search filters.".to_owned()),
            scope_update: RequiredNullable::some(ScopeUpdate {
                include: vec![boundary.to_owned()],
                exclude: Vec::new(),
            }),
            scope_boundary: RequiredNullable::some(boundary.to_owned()),
            non_goals: RequiredNullable::null(),
            acceptance_criteria: RequiredNullable::some(vec![AcceptanceCriterionReplacement {
                acceptance_criterion_id: RequiredNullable::null(),
                statement: "Saved filters reject out-of-scope edits.".to_owned(),
                evidence_requirement: EvidenceRequirement::Required,
            }]),
            autonomy_boundary: RequiredNullable::null(),
            baseline_ref: RequiredNullable::some(example_baseline_ref(format!("baseline_{task}"))?),
            change_unit: ChangeUnitUpdate {
                operation,
                effect_contract: None,
                fields,
            },
            related_scope_decision_refs: Vec::new(),
        })
    };
    let advisor_changed = |task: &str, operation| -> Result<McpUpdateScopeArguments, String> {
        let mut fields = Map::new();
        fields.insert(
            "scope_summary".to_owned(),
            Value::String("Bounded observe-only advice.".to_owned()),
        );
        fields.insert("affected_paths".to_owned(), Value::Array(Vec::new()));
        Ok(McpUpdateScopeArguments {
            action_form_ref: example_action_form_ref(),
            project_selector: None,
            detail: MutationDetailLevel::Summary,
            task_id: TaskId::new(task),
            goal_summary: RequiredNullable::null(),
            scope_update: RequiredNullable::null(),
            scope_boundary: RequiredNullable::null(),
            non_goals: RequiredNullable::null(),
            acceptance_criteria: RequiredNullable::null(),
            autonomy_boundary: RequiredNullable::null(),
            baseline_ref: RequiredNullable::some(example_baseline_ref(format!("baseline_{task}"))?),
            change_unit: ChangeUnitUpdate {
                operation,
                effect_contract: Some(advisor_observe_only_effect_contract()),
                fields,
            },
            related_scope_decision_refs: Vec::new(),
        })
    };
    Ok(vec![
        typed_example(
            UPDATE_SCOPE_KEEP_CURRENT_EXAMPLE_ID,
            "Keep the current Change Unit and leave omitted scope fields unchanged.",
            &keep,
            Vec::new(),
        ),
        typed_example(
            "create_current_change_unit",
            "Create a current Change Unit for the updated scope.",
            &changed(
                "task_filter_002",
                ChangeUnitOperation::CreateCurrent,
                "Saved-filter owner and label edits.",
                "src/search/saved-filters.ts",
            )?,
            Vec::new(),
        ),
        typed_example(
            "replace_current_change_unit",
            "Replace the current Change Unit for revised scope.",
            &changed(
                "task_filter_003",
                ChangeUnitOperation::ReplaceCurrent,
                "Saved-filter owner, label, and visibility edits.",
                "src/search/saved-filters.ts",
            )?,
            Vec::new(),
        ),
        typed_example(
            "advisor_create_current_change_unit",
            "Create the canonical observe-only Advisor Change Unit.",
            &advisor_changed(
                "task_advisor_filter_001",
                ChangeUnitOperation::CreateCurrent,
            )?,
            Vec::new(),
        ),
        typed_example(
            "advisor_replace_current_change_unit",
            "Replace the current Advisor Change Unit with the canonical observe-only boundary.",
            &advisor_changed(
                "task_advisor_filter_002",
                ChangeUnitOperation::ReplaceCurrent,
            )?,
            Vec::new(),
        ),
    ])
}

fn record_shaping_checkpoint_examples() -> Result<Vec<CanonicalSchemaExample>, String> {
    let base = |id: &str, baseline_ref, checkpoint_operation, gaps, source_refs| {
        McpRecordShapingCheckpointArguments {
            project_selector: None,
            detail: MutationDetailLevel::Summary,
            action_form_ref: example_action_form_ref(),
            task_id: TaskId::new(id),
            checkpoint_operation,
            scope_revision: 4,
            baseline_ref,
            summary: "The implementation boundary and open decisions are recorded.".to_owned(),
            implementation_boundary: RequiredNullable::some(
                "Implement only the current saved-filter scope.".to_owned(),
            ),
            gaps,
            source_refs,
            evidence_refs: Vec::new(),
        }
    };
    let initial_variant = vec![ExpectedTaggedVariant {
        instance_path: "/checkpoint_operation",
        discriminator_path: "/operation",
        discriminator_value: "create_initial",
        semantic_type: "ShapingCheckpointOperation::create_initial",
    }];
    let replace_variant = vec![ExpectedTaggedVariant {
        instance_path: "/checkpoint_operation",
        discriminator_path: "/operation",
        discriminator_value: "replace_current",
        semantic_type: "ShapingCheckpointOperation::replace_current",
    }];
    let gap = |kind, summary: &str, action| ShapingGapInput {
        gap_kind: kind,
        summary: summary.to_owned(),
        affected_refs: Vec::new(),
        user_action: action,
    };
    let user_gap = |kind, judgment_kind, summary: &str| {
        gap(
            kind,
            summary,
            RequiredNullable::some(ShapingUserActionDraft {
                action: choice_draft(judgment_kind, summary),
                expires_at: RequiredNullable::null(),
            }),
        )
    };
    let mut examples = vec![
        typed_example(
            "create_initial_null_baseline",
            "Create the initial checkpoint while the required baseline field is JSON null.",
            &base(
                "task_shape_null_001",
                RequiredNullable::null(),
                ShapingCheckpointOperation::CreateInitial,
                Vec::new(),
                Vec::new(),
            ),
            initial_variant.clone(),
        ),
        typed_example(
            "create_initial_with_baseline",
            "Create the initial checkpoint with a current baseline.",
            &base(
                "task_shape_001",
                RequiredNullable::some(example_baseline_ref("baseline_shape_001")?),
                ShapingCheckpointOperation::CreateInitial,
                Vec::new(),
                Vec::new(),
            ),
            initial_variant.clone(),
        ),
        typed_example(
            "replace_current",
            "Replace the exact current checkpoint without stale authority.",
            &base(
                "task_shape_replace_001",
                RequiredNullable::some(example_baseline_ref("baseline_shape_replace_001")?),
                ShapingCheckpointOperation::ReplaceCurrent {
                    expected_current_checkpoint_id: ShapingCheckpointId::new(
                        "shaping_checkpoint_current_001",
                    ),
                    retired_non_authorizing_request_refs: Vec::new(),
                    carry_forward_application_refs: Vec::new(),
                    stale_authority_actions: Vec::new(),
                },
                Vec::new(),
                Vec::new(),
            ),
            replace_variant.clone(),
        ),
        typed_example(
            "structural_gap",
            "Record one structural implementation-boundary gap.",
            &base(
                "task_shape_structural_001",
                RequiredNullable::some(example_baseline_ref("baseline_shape_structural_001")?),
                ShapingCheckpointOperation::CreateInitial,
                vec![gap(
                    ShapingGapKind::ImplementationBoundaryMissing,
                    "The implementation boundary needs a precise path set.",
                    RequiredNullable::null(),
                )],
                Vec::new(),
            ),
            initial_variant.clone(),
        ),
    ];
    for (id, kind, judgment, summary) in [
        (
            "product_decision_gap",
            ShapingGapKind::UserProductDecisionRequired,
            JudgmentKind::ProductDecision,
            "Choose the saved-filter product behavior.",
        ),
        (
            "technical_decision_gap",
            ShapingGapKind::UserTechnicalDecisionRequired,
            JudgmentKind::TechnicalDecision,
            "Choose the saved-filter persistence approach.",
        ),
        (
            "scope_decision_gap",
            ShapingGapKind::UserScopeDecisionRequired,
            JudgmentKind::ScopeDecision,
            "Confirm whether visibility settings are in scope.",
        ),
        (
            "sensitive_approval_gap",
            ShapingGapKind::SensitiveApprovalRequired,
            JudgmentKind::SensitiveApproval,
            "Approve the bounded sensitive preference operation.",
        ),
    ] {
        let expected_variants = vec![
            initial_variant[0].clone(),
            ExpectedTaggedVariant {
                instance_path: "/gaps/0/user_action/action",
                discriminator_path: "/action_type",
                discriminator_value: "choice",
                semantic_type: "UserActionDraft::choice",
            },
        ];
        examples.push(typed_example(
            id,
            "Record one typed user-owned shaping decision gap.",
            &base(
                &format!("task_{id}"),
                RequiredNullable::some(example_baseline_ref(format!("baseline_{id}"))?),
                ShapingCheckpointOperation::CreateInitial,
                vec![user_gap(kind, judgment, summary)],
                Vec::new(),
            ),
            expected_variants,
        ));
    }
    examples.push(typed_example(
        "repository_file_source_ref",
        "Record repository-file provenance with an exact nested source discriminator.",
        &base(
            "task_shape_source_001",
            RequiredNullable::some(example_baseline_ref("baseline_shape_source_001")?),
            ShapingCheckpointOperation::CreateInitial,
            Vec::new(),
            vec![SourceRef::RepositoryFile(RepositoryFileSource {
                repository_path: "src/search/saved-filters.ts".to_owned(),
                baseline_commit_sha: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
                content_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_owned(),
                line_range: RequiredNullable::some(SourceLineRange {
                    start_line: 10,
                    end_line: 24,
                }),
            })],
        ),
        vec![
            initial_variant[0].clone(),
            ExpectedTaggedVariant {
                instance_path: "/source_refs/0",
                discriminator_path: "/source_kind",
                discriminator_value: "repository_file",
                semantic_type: "SourceRef::repository_file",
            },
        ],
    ));
    examples.push(typed_example(
        "exact_stale_authority_recovery",
        "Replace the current checkpoint with exact retire and fresh-identity reauthorization actions.",
        &base(
            "task_shape_recovery_001",
            RequiredNullable::some(example_baseline_ref("baseline_shape_recovery_001")?),
            ShapingCheckpointOperation::ReplaceCurrent {
                expected_current_checkpoint_id: ShapingCheckpointId::new(
                    "shaping_checkpoint_stale_001",
                ),
                retired_non_authorizing_request_refs: vec![state_ref(
                    StateRecordKind::UserActionRequest,
                    "user_action_request_terminal_001",
                    "task_shape_recovery_001",
                )],
                carry_forward_application_refs: Vec::new(),
                stale_authority_actions: vec![
                    StaleShapingAuthorityAction::Retire {
                        stale_application_ref: state_ref(
                            StateRecordKind::ShapingDecisionApplication,
                            "shaping_application_retire_001",
                            "task_shape_recovery_001",
                        ),
                    },
                    StaleShapingAuthorityAction::Reauthorize {
                        stale_application_ref: state_ref(
                            StateRecordKind::ShapingDecisionApplication,
                            "shaping_application_reauthorize_001",
                            "task_shape_recovery_001",
                        ),
                        successor_gap: user_gap(
                            ShapingGapKind::UserTechnicalDecisionRequired,
                            JudgmentKind::TechnicalDecision,
                            "Reconfirm the persistence approach on the current baseline.",
                        ),
                    },
                ],
            },
            Vec::new(),
            Vec::new(),
        ),
        vec![
            replace_variant[0].clone(),
            ExpectedTaggedVariant {
                instance_path: "/checkpoint_operation/stale_authority_actions/0",
                discriminator_path: "/action",
                discriminator_value: "retire",
                semantic_type: "StaleShapingAuthorityAction::retire",
            },
            ExpectedTaggedVariant {
                instance_path: "/checkpoint_operation/stale_authority_actions/1",
                discriminator_path: "/action",
                discriminator_value: "reauthorize",
                semantic_type: "StaleShapingAuthorityAction::reauthorize",
            },
            ExpectedTaggedVariant {
                instance_path: "/checkpoint_operation/stale_authority_actions/1/successor_gap/user_action/action",
                discriminator_path: "/action_type",
                discriminator_value: "choice",
                semantic_type: "UserActionDraft::choice",
            },
        ],
    ));
    Ok(examples)
}

fn finalize_advice_examples() -> Result<Vec<CanonicalSchemaExample>, String> {
    let base = |id: &str| -> Result<McpFinalizeAdviceArguments, String> {
        Ok(McpFinalizeAdviceArguments {
            project_selector: None,
            detail: MutationDetailLevel::Summary,
            action_form_ref: example_action_form_ref(),
            task_id: TaskId::new(format!("task_{id}")),
            shaping_checkpoint_id: ShapingCheckpointId::new(format!("shaping_checkpoint_{id}")),
            change_unit_id: ChangeUnitId::new(format!("change_unit_{id}")),
            scope_revision: 2,
            baseline_ref: example_baseline_ref(format!("baseline_{id}"))?,
            user_action_resolution_ids: Vec::new(),
            result_summary: "The current advisory result is finalized.".to_owned(),
            result_refs: Vec::new(),
            evidence_refs: Vec::new(),
            residual_risks: Vec::new(),
            recovery_constraints: Vec::new(),
        })
    };
    let without_decisions = base("advice_001")?;
    let mut with_decisions = base("advice_decisions_001")?;
    with_decisions.user_action_resolution_ids = vec![
        UserActionResolutionId::new("user_action_resolution_product_001"),
        UserActionResolutionId::new("user_action_resolution_technical_001"),
    ];
    let mut with_evidence = base("advice_evidence_001")?;
    with_evidence.result_refs = vec![state_ref(
        StateRecordKind::ProjectContinuityRecord,
        "advice_result_ref_001",
        "task_advice_evidence_001",
    )];
    with_evidence.evidence_refs = vec![state_ref(
        StateRecordKind::EvidenceSummary,
        "evidence_summary_advice_001",
        "task_advice_evidence_001",
    )];
    with_evidence.residual_risks = vec![ResidualRiskInput {
        summary: "The recommendation depends on current provider behavior.".to_owned(),
        consequence: "A provider change may require revisiting the recommendation.".to_owned(),
        acceptance_required: false,
        source_refs: with_evidence.evidence_refs.clone(),
    }];
    with_evidence.recovery_constraints =
        vec!["Revalidate provider behavior before implementation.".to_owned()];
    Ok(vec![
        typed_example(
            "advisor_without_user_decisions",
            "Finalize advisor work that required no user decision.",
            &without_decisions,
            Vec::new(),
        ),
        typed_example(
            "advisor_with_accepted_resolution_refs",
            "Finalize advisor work with the exact accepted resolution identifiers.",
            &with_decisions,
            Vec::new(),
        ),
        typed_example(
            "advisor_with_evidence_and_residual_risks",
            "Finalize advisor work with evidence refs, result refs, and residual risks.",
            &with_evidence,
            Vec::new(),
        ),
    ])
}

fn status_examples() -> Vec<CanonicalSchemaExample> {
    [
        (
            "summary_status",
            "Read the compact status summary.",
            StatusDetailLevel::Summary,
        ),
        (
            STATUS_READ_ONLY_EXAMPLE_ID,
            "Read the normal workflow status view.",
            StatusDetailLevel::Workflow,
        ),
        (
            "full_status",
            "Read the full status view including continuity detail.",
            StatusDetailLevel::Full,
        ),
    ]
    .into_iter()
    .map(|(id, description, detail)| {
        typed_example(
            id,
            description,
            &McpStatusArguments {
                project_selector: None,
                task_id: RequiredNullable::null(),
                detail,
                continuity_page: None,
            },
            Vec::new(),
        )
    })
    .collect()
}

fn get_operation_result_examples() -> Vec<CanonicalSchemaExample> {
    vec![typed_example(
        GET_OPERATION_RESULT_FIRST_PAGE_EXAMPLE_ID,
        "Read the first bounded page of one immutable historical mutation response.",
        &McpGetOperationResultArguments {
            project_selector: None,
            operation_result_ref: OperationResultRef {
                project_id: ProjectId::new("proj_history_001"),
                source_method: MethodName::RecordRun,
                source_idempotency_key: IdempotencyKey::new("idem_run_history_001"),
                committed_state_version: 42,
                response_sha256:
                    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                        .to_owned(),
                response_size_bytes: 32_768,
            },
            cursor: RequiredNullable::null(),
        },
        Vec::new(),
    )]
}

fn prepare_evidence_capture_examples() -> Result<Vec<CanonicalSchemaExample>, String> {
    let common = |capture| -> Result<McpPrepareEvidenceCaptureArguments, String> {
        Ok(McpPrepareEvidenceCaptureArguments {
            project_selector: None,
            detail: MutationDetailLevel::Summary,
            action_form_ref: example_action_form_ref(),
            task_id: TaskId::new("task_capture_001"),
            change_unit_id: ChangeUnitId::new("cu_capture_001"),
            baseline_ref: example_baseline_ref("baseline_capture_001")?,
            target: EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id: AcceptanceCriterionId::new("criterion_capture_001"),
            },
            capture,
        })
    };
    Ok(vec![
        typed_example(
            PREPARE_EVIDENCE_CAPTURE_VERIFIED_COMMAND_EXAMPLE_ID,
            "Create an intent for a registered command evidence source.",
            &common(McpEvidenceCaptureSpec::VerifiedCommandExecution {
                command_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_owned(),
                command_label: "Focused validation".to_owned(),
                expected_exit_code: RequiredNullable::null(),
            })?,
            vec![
                ExpectedTaggedVariant {
                    instance_path: "/target",
                    discriminator_path: "/target_kind",
                    discriminator_value: "acceptance_criterion",
                    semantic_type: "EvidenceTarget::acceptance_criterion",
                },
                ExpectedTaggedVariant {
                    instance_path: "/capture",
                    discriminator_path: "/capture_kind",
                    discriminator_value: "verified_command_execution",
                    semantic_type: "McpEvidenceCaptureSpec::verified_command_execution",
                },
            ],
        ),
        typed_example(
            PREPARE_EVIDENCE_CAPTURE_VERIFIED_TOOL_EXAMPLE_ID,
            "Create an intent for an exact registered tool invocation.",
            &common(McpEvidenceCaptureSpec::VerifiedToolInvocation {
                tool_name: "example.validate".to_owned(),
                tool_input_sha256:
                    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
                expected_success: RequiredNullable::null(),
            })?,
            vec![
                ExpectedTaggedVariant {
                    instance_path: "/target",
                    discriminator_path: "/target_kind",
                    discriminator_value: "acceptance_criterion",
                    semantic_type: "EvidenceTarget::acceptance_criterion",
                },
                ExpectedTaggedVariant {
                    instance_path: "/capture",
                    discriminator_path: "/capture_kind",
                    discriminator_value: "verified_tool_invocation",
                    semantic_type: "McpEvidenceCaptureSpec::verified_tool_invocation",
                },
            ],
        ),
    ])
}

fn prepare_write_examples() -> Result<Vec<CanonicalSchemaExample>, String> {
    Ok(vec![typed_example(
        PREPARE_WRITE_SIMPLE_EXAMPLE_ID,
        "Check one Product Repository write intent.",
        &McpPrepareWriteArguments {
            project_selector: None,
            detail: MutationDetailLevel::Full,
            action_form_ref: example_action_form_ref(),
            task_id: TaskId::new("task_pref_001"),
            change_unit_id: ChangeUnitId::new("cu_pref_001"),
            intended_operation: "Update the profile preference save flow.".to_owned(),
            intended_paths: vec!["src/preferences/profile-save.ts".to_owned()],
            product_file_write_intended: true,
            sensitive_categories: Vec::new(),
            baseline_ref: example_baseline_ref("baseline_pref_001")?,
        },
        Vec::new(),
    )])
}

fn stage_artifact_examples() -> Vec<CanonicalSchemaExample> {
    vec![typed_example(
        "stage_safe_text",
        "Stage a text attachment input.",
        &McpStageArtifactArguments {
            project_selector: None,
            detail: MutationDetailLevel::Full,
            action_form_ref: example_action_form_ref(),
            task_id: TaskId::new("task_trace_001"),
            display_name: "diagnostic_trace.log".to_owned(),
            content_type: "text/plain".to_owned(),
            redaction_state: RedactionState::None,
            safe_bytes_or_notice: "Local trace sample captured for debugging.".to_owned(),
            expected_sha256: RequiredNullable::null(),
            expected_size_bytes: RequiredNullable::null(),
            relation_hint: RequiredNullable::null(),
        },
        Vec::new(),
    )]
}

fn record_run_examples() -> Result<Vec<CanonicalSchemaExample>, String> {
    let baseline_ref = example_baseline_ref("baseline_run_002")?;
    Ok(vec![typed_example(
        RECORD_RUN_EVIDENCE_BEARING_EXAMPLE_ID,
        "Record target-scoped evidence and a close assessment.",
        &McpRecordRunArguments {
            project_selector: None,
            detail: MutationDetailLevel::Summary,
            action_form_ref: example_action_form_ref(),
            task_id: TaskId::new("task_run_002"),
            change_unit_id: ChangeUnitId::new("cu_run_002"),
            kind: RunKind::Implementation,
            run_id: RequiredNullable::null(),
            baseline_ref: baseline_ref.clone(),
            write_ticket_id: RequiredNullable::null(),
            performed_operation: RequiredNullable::null(),
            summary: "Saved-filter validation reviewed.".to_owned(),
            observed_changes: ObservedChanges {
                changed_paths: Vec::new(),
                product_file_write_observed: false,
                sensitive_categories: Vec::new(),
                baseline_ref: RequiredNullable::some(baseline_ref),
            },
            artifact_inputs: Vec::new(),
            evidence_updates: vec![McpEvidenceCoverageUpdate {
                target: EvidenceTarget::AcceptanceCriterion {
                    acceptance_criterion_id: AcceptanceCriterionId::new(
                        "criterion_saved_filter_001",
                    ),
                },
                coverage_state: EvidenceCoverageUpdateState::Supported,
                provenance: None,
                supporting_run_refs: Vec::new(),
                observation_refs: Vec::new(),
                supporting_artifact_refs: Vec::new(),
                gap_refs: Vec::new(),
            }],
            evidence_observations: vec![McpEvidenceObservationInput {
                target: EvidenceTarget::AcceptanceCriterion {
                    acceptance_criterion_id: AcceptanceCriterionId::new(
                        "criterion_saved_filter_001",
                    ),
                },
                source_kind: EvidenceSourceKind::AgentReport,
                assurance_level: EvidenceAssuranceLevel::CooperativeReport,
                observed_by_actor_source: RequiredNullable::null(),
                tool_name: RequiredNullable::null(),
                tool_invocation_id: RequiredNullable::null(),
                tool_metadata: Map::new(),
                input_refs: Vec::new(),
                source_refs: Vec::new(),
                output_artifact_refs: Vec::new(),
                limitations: Vec::new(),
                observed_at: volicord_types::values::UtcTimestamp::parse("2026-07-12T00:00:00Z")
                    .expect("canonical timestamp"),
            }],
            close_assessment: RequiredNullable::some(CloseAssessmentInput {
                result_summary: "Saved-filter validation reviewed.".to_owned(),
                result_refs: Vec::new(),
                residual_risks: Vec::new(),
                sensitive_categories: Vec::new(),
                recovery_constraints: Vec::new(),
            }),
        },
        vec![
            ExpectedTaggedVariant {
                instance_path: "/evidence_updates/0/target",
                discriminator_path: "/target_kind",
                discriminator_value: "acceptance_criterion",
                semantic_type: "EvidenceTarget::acceptance_criterion",
            },
            ExpectedTaggedVariant {
                instance_path: "/evidence_observations/0/target",
                discriminator_path: "/target_kind",
                discriminator_value: "acceptance_criterion",
                semantic_type: "EvidenceTarget::acceptance_criterion",
            },
        ],
    )])
}

fn request_user_action_examples() -> Vec<CanonicalSchemaExample> {
    vec![
        typed_example(
            REQUEST_USER_ACTION_FINAL_ACCEPTANCE_EXAMPLE_ID,
            "Create final acceptance through the common user-action model.",
            &McpRequestUserActionArguments {
                project_selector: None,
                detail: MutationDetailLevel::Summary,
                action_form_ref: Some(example_action_form_ref()),
                request: McpRequestUserActionOperation::Create {
                    task_id: TaskId::new("task_close_001"),
                    change_unit_id: RequiredNullable::null(),
                    action: choice_draft(
                        JudgmentKind::FinalAcceptance,
                        "Do you accept this result as complete?",
                    ),
                    required_for: vec![UserActionRequiredFor::CloseComplete],
                    expires_at: RequiredNullable::null(),
                },
            },
            vec![
                ExpectedTaggedVariant {
                    instance_path: "/request",
                    discriminator_path: "/operation",
                    discriminator_value: "create",
                    semantic_type: "McpRequestUserActionOperation::create",
                },
                ExpectedTaggedVariant {
                    instance_path: "/request/action",
                    discriminator_path: "/action_type",
                    discriminator_value: "choice",
                    semantic_type: "UserActionDraft::choice",
                },
            ],
        ),
        typed_example(
            "resume_user_action",
            "Resume the original exact Agent Connection result after a later CLI inbox resolution.",
            &McpRequestUserActionArguments {
                project_selector: None,
                detail: MutationDetailLevel::Summary,
                action_form_ref: None,
                request: McpRequestUserActionOperation::Resume {
                    user_action_request_id: UserActionRequestId::new("uact_existing_001"),
                },
            },
            vec![ExpectedTaggedVariant {
                instance_path: "/request",
                discriminator_path: "/operation",
                discriminator_value: "resume",
                semantic_type: "McpRequestUserActionOperation::resume",
            }],
        ),
    ]
}

fn close_task_examples() -> Vec<CanonicalSchemaExample> {
    [
        (
            "close_complete",
            "Request the completion close path.",
            "task_close_001",
            CloseMutationIntent::Complete,
            CloseReason::CompletedSelfChecked,
            None,
        ),
        (
            "close_cancel",
            "Request the cancellation close path.",
            "task_cancel_001",
            CloseMutationIntent::Cancel,
            CloseReason::Cancelled,
            None,
        ),
        (
            "close_supersede",
            "Request the supersession close path.",
            "task_supersede_001",
            CloseMutationIntent::Supersede,
            CloseReason::Superseded,
            Some(TaskId::new("task_replacement_001")),
        ),
    ]
    .into_iter()
    .map(
        |(id, description, task_id, intent, close_reason, superseding_task_id)| {
            typed_example(
                id,
                description,
                &McpCloseTaskArguments {
                    project_selector: None,
                    detail: MutationDetailLevel::Summary,
                    action_form_ref: example_action_form_ref(),
                    task_id: TaskId::new(task_id),
                    intent,
                    close_reason: RequiredNullable::some(close_reason),
                    superseding_task_id: RequiredNullable::new(superseding_task_id),
                    user_note: RequiredNullable::null(),
                },
                Vec::new(),
            )
        },
    )
    .collect()
}

fn choice_draft(judgment_kind: JudgmentKind, question: &str) -> UserActionDraft {
    let caller_options = matches!(
        judgment_kind,
        JudgmentKind::ProductDecision | JudgmentKind::TechnicalDecision
    )
    .then(|| {
        vec![
            UserActionOptionInput {
                option_id: UserActionOptionId::new("option_accept_001"),
                label: "Use the proposed choice".to_owned(),
                description: "Apply the bounded proposed choice.".to_owned(),
                consequence: "The current shaping plan uses this choice.".to_owned(),
                is_default: true,
            },
            UserActionOptionInput {
                option_id: UserActionOptionId::new("option_reject_001"),
                label: "Choose another approach".to_owned(),
                description: "Return the choice to shaping.".to_owned(),
                consequence: "The current shaping plan must be revised.".to_owned(),
                is_default: false,
            },
        ]
    });
    let sensitive_action_scope = if judgment_kind == JudgmentKind::SensitiveApproval {
        RequiredNullable::some(SensitiveActionScope {
            action_kind: "bounded_preference_update".to_owned(),
            description: "Update only the approved saved-filter preference.".to_owned(),
            intended_paths: vec!["src/search/saved-filters.ts".to_owned()],
            sensitive_categories: vec!["user_preference".to_owned()],
            command_or_tool_summary: RequiredNullable::some(
                "Apply the approved preference update.".to_owned(),
            ),
            network_or_host_summary: RequiredNullable::null(),
            secret_or_credential_summary: RequiredNullable::null(),
            capability_claim: "repository_write".to_owned(),
            expires_at: RequiredNullable::null(),
        })
    } else {
        RequiredNullable::null()
    };
    UserActionDraft::Choice(Box::new(UserActionChoiceDraft {
        judgment_kind,
        presentation: JudgmentPresentation::Short,
        question: question.to_owned(),
        options: RequiredNullable::new(caller_options),
        context: UserActionContext {
            summary: "Review the current shaping basis and decide this question.".to_owned(),
            related_refs: Vec::new(),
            artifact_refs: Vec::new(),
            visible_risks: Vec::new(),
            constraints: vec!["Only the current shaping question is in scope.".to_owned()],
        },
        affected_refs: Vec::new(),
        sensitive_action_scope,
    }))
}

fn state_ref(kind: StateRecordKind, id: &str, task_id: &str) -> StateRecordRef {
    StateRecordRef::new(
        kind,
        RecordId::new(id),
        ProjectId::new("project_schema_examples"),
        Some(TaskId::new(task_id)),
        Some(7),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::semantic_schema::{SemanticSchemaNode, SemanticValidationIssueCode};
    use serde::{Deserialize, Deserializer};
    use serde_json::json;

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    struct DecodeNarrowerThanSchema {
        #[serde(deserialize_with = "reject_schema_valid_literal")]
        value: String,
    }

    fn reject_schema_valid_literal<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value == "schema-valid" {
            return Err(serde::de::Error::custom(
                "test decoder rejects a schema-valid literal",
            ));
        }
        Ok(value)
    }

    fn shaping_example<'a>(
        contract: &'a McpToolContractDescriptor,
        id: &str,
    ) -> &'a CanonicalSchemaExample {
        contract
            .canonical_examples()
            .iter()
            .find(|example| example.id() == id)
            .unwrap_or_else(|| panic!("missing shaping example `{id}`"))
    }

    fn assert_tagged_definition(
        contract: &McpToolContractDescriptor,
        name: &str,
        discriminator_path: &str,
        expected_values: &[&str],
    ) {
        let SemanticSchemaNode::TaggedUnion(union) = contract
            .input_descriptor()
            .definitions()
            .get(name)
            .unwrap_or_else(|| panic!("missing `{name}` definition"))
        else {
            panic!("`{name}` must be an explicit tagged union");
        };
        assert_eq!(union.discriminator_path, discriminator_path);
        assert_eq!(
            union
                .variants
                .iter()
                .map(|variant| variant.discriminator_value.as_str())
                .collect::<Vec<_>>(),
            expected_values
        );
    }

    #[test]
    fn every_production_tool_has_one_integral_descriptor() {
        let contracts = mcp_tool_contracts();
        assert_eq!(contracts.len(), AgentToolId::ALL.len());
        let errors = mcp_tool_contract_integrity_errors();
        assert!(errors.is_empty(), "{}", errors.join("\n"));
    }

    #[test]
    fn descriptor_decode_disagreement_is_a_schema_contract_failure() {
        let descriptor = contract::<DecodeNarrowerThanSchema, DecodeNarrowerThanSchema>(
            AgentToolId::STATUS,
            "test",
            "test",
            Vec::new(),
        );
        let value = serde_json::json!({"value": "schema-valid"});

        assert!(descriptor
            .input_descriptor()
            .validate(&value)
            .issues
            .is_empty());
        assert_eq!(
            descriptor.validate_and_decode_input(&value),
            McpInputContractValidation::SchemaContractFailure
        );
    }

    #[test]
    fn every_canonical_tagged_union_rejects_invalid_discriminator_without_branch_guessing() {
        for contract in mcp_tool_contracts() {
            for example in contract.canonical_examples() {
                for expected in example.expected_variants() {
                    let mut value = example.value().clone();
                    let discriminator_pointer =
                        format!("{}{}", expected.instance_path, expected.discriminator_path);
                    *value
                        .pointer_mut(&discriminator_pointer)
                        .unwrap_or_else(|| {
                            panic!(
                                "{} example {} must contain {}",
                                contract.tool().wire_name(),
                                example.id(),
                                discriminator_pointer
                            )
                        }) = Value::String("__invalid_discriminator__".to_owned());

                    let validation = contract.input_descriptor().validate(&value);
                    let local_issues = validation
                        .issues
                        .iter()
                        .filter(|issue| issue.path.starts_with(expected.instance_path))
                        .collect::<Vec<_>>();
                    assert_eq!(
                        local_issues.len(),
                        1,
                        "{} example {} guessed fields for {}: {:#?}",
                        contract.tool().wire_name(),
                        example.id(),
                        discriminator_pointer,
                        validation.issues
                    );
                    assert_eq!(local_issues[0].path, discriminator_pointer);
                    assert_eq!(local_issues[0].code, SemanticValidationIssueCode::EnumValue);
                    assert!(local_issues[0]
                        .allowed_values
                        .iter()
                        .any(|value| value == expected.discriminator_value));
                    assert!(validation
                        .canonical_example
                        .as_ref()
                        .is_some_and(|summary| summary.contains_key("variants")));
                }
            }
        }
    }

    #[test]
    fn every_production_result_union_rejects_invalid_discriminator_before_branch_fields() {
        for contract in mcp_tool_contracts() {
            let validation = contract
                .output_descriptor()
                .validate(&serde_json::json!({"result_type": "__invalid_discriminator__"}));
            assert_eq!(
                validation.issues.len(),
                1,
                "{} output guessed a result branch: {:#?}",
                contract.tool().wire_name(),
                validation.issues
            );
            assert_eq!(validation.issues[0].path, "/result_type");
            assert_eq!(
                validation.issues[0].code,
                SemanticValidationIssueCode::EnumValue
            );
            assert!(validation
                .canonical_example
                .as_ref()
                .is_some_and(|summary| summary.contains_key("variants")));
        }
    }

    #[test]
    fn shaping_required_nullable_baseline_is_exact() {
        use jsonschema::{Draft, JSONSchema};

        let contract = mcp_tool_contract(AgentToolId::RECORD_SHAPING_CHECKPOINT)
            .expect("record-shaping semantic contract");
        let example = contract
            .canonical_examples()
            .iter()
            .find(|example| example.id() == "create_initial_null_baseline")
            .expect("null-baseline canonical example");
        assert!(example.value()["baseline_ref"].is_null());
        let validation = contract.input_descriptor().validate(example.value());
        assert!(validation.issues.is_empty(), "{:#?}", validation.issues);
        let schema = contract.input_schema();
        assert_eq!(
            schema.pointer("/definitions/BaselineRef/not/const"),
            Some(&Value::String("null".to_owned()))
        );
        assert_eq!(
            schema.pointer("/definitions/BaselineRef/maxLength"),
            Some(&Value::from(BaselineRef::spec().maximum_length))
        );
        assert_eq!(
            schema.pointer("/definitions/BaselineRef/pattern"),
            Some(&Value::String(BaselineRef::spec().json_schema_pattern()))
        );
        let compiled_schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema)
            .expect("generated MCP input schema must compile");

        for valid_baseline_ref in BaselineRef::spec().examples {
            let mut valid = example.value().clone();
            valid["baseline_ref"] = Value::String((*valid_baseline_ref).to_owned());
            assert!(contract
                .input_descriptor()
                .validate(&valid)
                .issues
                .is_empty());
            assert!(compiled_schema.is_valid(&valid));
            assert!(decode_round_trip::<McpRecordShapingCheckpointArguments>(&valid).is_ok());
        }

        for invalid_baseline_ref in BaselineRef::spec().generated_invalid_corpus() {
            let mut invalid = example.value().clone();
            invalid["baseline_ref"] = Value::String(invalid_baseline_ref.clone());
            assert!(
                !contract
                    .input_descriptor()
                    .validate(&invalid)
                    .issues
                    .is_empty(),
                "record-shaping input accepted invalid BaselineRef {invalid_baseline_ref:?}"
            );
            assert!(!compiled_schema.is_valid(&invalid));
            assert!(decode_round_trip::<McpRecordShapingCheckpointArguments>(&invalid).is_err());
        }

        let mut omitted = example.value().clone();
        omitted
            .as_object_mut()
            .expect("example object")
            .remove("baseline_ref");
        assert!(!contract
            .input_descriptor()
            .validate(&omitted)
            .issues
            .is_empty());
    }

    #[test]
    fn shaping_nested_union_families_are_explicit_and_reject_mixed_fields() {
        let contract = mcp_tool_contract(AgentToolId::RECORD_SHAPING_CHECKPOINT)
            .expect("record-shaping semantic contract");
        assert_tagged_definition(
            contract,
            "SourceRef",
            "/source_kind",
            &[
                "repository_file",
                "git_commit",
                "git_diff",
                "command",
                "external_uri",
                "user_context",
            ],
        );
        assert_tagged_definition(
            contract,
            "UserActionDraft",
            "/action_type",
            &["choice", "evidence_observation"],
        );
        assert_tagged_definition(
            contract,
            "StaleShapingAuthorityAction",
            "/action",
            &["retire", "reauthorize"],
        );
        for (definition, variant) in [
            ("SourceRef", "repository_file"),
            ("UserActionDraft", "choice"),
            ("StaleShapingAuthorityAction", "retire"),
            ("StaleShapingAuthorityAction", "reauthorize"),
        ] {
            let SemanticSchemaNode::TaggedUnion(union) = contract
                .input_descriptor()
                .definitions()
                .get(definition)
                .expect("nested tagged-union definition")
            else {
                panic!("`{definition}` must be tagged");
            };
            assert!(
                contract
                    .canonical_examples()
                    .iter()
                    .flat_map(CanonicalSchemaExample::expected_variants)
                    .any(
                        |expected| expected.discriminator_path == union.discriminator_path
                            && expected.discriminator_value == variant
                    ),
                "`{definition}` variant `{variant}` must have a typed canonical example"
            );
        }

        let mut mixed_source = shaping_example(contract, "repository_file_source_ref")
            .value()
            .clone();
        mixed_source
            .pointer_mut("/source_refs/0")
            .and_then(Value::as_object_mut)
            .expect("repository source object")
            .insert("command".to_owned(), Value::String("cargo test".to_owned()));
        assert!(!contract
            .input_descriptor()
            .validate(&mixed_source)
            .issues
            .is_empty());

        let mut mixed_action = shaping_example(contract, "product_decision_gap")
            .value()
            .clone();
        mixed_action
            .pointer_mut("/gaps/0/user_action/action")
            .and_then(Value::as_object_mut)
            .expect("choice action object")
            .insert(
                "context_summary".to_owned(),
                Value::String("mixed evidence-observation field".to_owned()),
            );
        assert!(!contract
            .input_descriptor()
            .validate(&mixed_action)
            .issues
            .is_empty());

        let mut mixed_stale = shaping_example(contract, "exact_stale_authority_recovery")
            .value()
            .clone();
        let successor_gap = mixed_stale
            .pointer("/checkpoint_operation/stale_authority_actions/1/successor_gap")
            .expect("reauthorize successor gap")
            .clone();
        mixed_stale
            .pointer_mut("/checkpoint_operation/stale_authority_actions/0")
            .and_then(Value::as_object_mut)
            .expect("retire action object")
            .insert("successor_gap".to_owned(), successor_gap);
        assert!(!contract
            .input_descriptor()
            .validate(&mixed_stale)
            .issues
            .is_empty());
    }

    #[test]
    fn invalid_shaping_discriminator_takes_precedence_over_unselected_root_inputs() {
        let contract = mcp_tool_contract(AgentToolId::RECORD_SHAPING_CHECKPOINT)
            .expect("record-shaping semantic contract");
        let validation = contract.input_descriptor().validate(&json!({
            "checkpoint_operation": {"operation": "create"},
            "baseline_ref": null
        }));

        assert_eq!(validation.issues.len(), 1, "{:#?}", validation.issues);
        assert_eq!(validation.issues[0].path, "/checkpoint_operation/operation");
        assert_eq!(
            validation.issues[0].code,
            SemanticValidationIssueCode::EnumValue
        );
    }

    #[test]
    fn sensitive_shaping_example_has_non_null_bounded_scope() {
        let contract = mcp_tool_contract(AgentToolId::RECORD_SHAPING_CHECKPOINT)
            .expect("record-shaping semantic contract");
        let example = shaping_example(contract, "sensitive_approval_gap");
        let scope = example
            .value()
            .pointer("/gaps/0/user_action/action/sensitive_action_scope")
            .expect("required sensitive action scope");
        assert!(scope.is_object());
        assert!(contract
            .input_descriptor()
            .validate(example.value())
            .issues
            .is_empty());
    }
}
