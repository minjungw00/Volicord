//! MCP tool argument, structured-content, failure, and recovery wire values.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use volicord_types::ids::{
    BaselineRef, ChangeUnitId, ProjectId, RequestHash, RunId, ShapingCheckpointId, TaskId,
    UserActionRequestId, UserActionResolutionId, WriteTicketId,
};
use volicord_types::methods::{
    ChangeUnitUpdate, InitialScope, OperationResultRef, RequestUserActionResponse, ScopeUpdate,
    StatusInclude, UnrecordedChangeRejection, UnrecordedChangeResolutionRequest,
};
use volicord_types::schema::{
    AcceptanceCriterionReplacement, AgentSafeUserActionRequestSummary, ArtifactInput, ArtifactRef,
    AuthorityReceipt, CloseAssessmentInput, ContinuityPageRequest, EvidenceCaptureIntent,
    EvidenceCaptureSpec, EvidenceCoverageUpdate, EvidenceObservationInput, EvidenceTarget,
    EvidenceUpdateProvenance, JsonObject, ObservedChanges, RequiredNullable, ResidualRiskInput,
    ShapingCheckpointOperation, ShapingGapInput, SourceRef, StagedArtifactHandle, StateRecordRef,
    ToolDryRunResponse, ToolRejectedResponse, TransitionAttemptDetails, UserActionDraft,
    WorkflowActionKey, WorkflowProjection, WorkflowRejectionUserAction, WriteDecisionReason,
    WriteTicket,
};
use volicord_types::tool_names::AgentToolId;
use volicord_types::values::{
    AcceptancePolicy, ActorSource, CloseMutationIntent, CloseReason, CloseState, EffectKind,
    ErrorCode, EvidenceAssuranceLevel, EvidenceCoverageUpdateState, EvidenceDisplayState,
    EvidenceRelevanceStatus, EvidenceSourceKind, JudgmentResolutionOutcome, MethodName,
    MutationDetailLevel, PrepareWriteDecision, RedactionState, RequestedControlLevel,
    RequestedMode, ResumePolicy, RunKind, ShapingCheckpointReadiness,
    ShapingDecisionApplicationOwner, StatusDetailLevel, TaskMode, UserActionChannelKind,
    UserActionKind, UserActionOptionAction, UserActionRequiredFor, UserActionStatus, UtcTimestamp,
    WorkPhase, WorkflowActionSemanticVariant, WorkflowStateKind, WriteTicketEffect,
};

/// Compound MCP projection that keeps the agent-workflow transaction distinct
/// from any later immutable user-channel resolution observed by the adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRequestUserActionResponse {
    pub agent_workflow_result: RequestUserActionResponse,
    pub agent_workflow_result_replayed: bool,
    pub current_projection_state_version: u64,
    pub current_projection_observed_at: UtcTimestamp,
    pub current_status: UserActionStatus,
    pub user_channel_resolution_ref: RequiredNullable<StateRecordRef>,
    pub user_channel_resolution: RequiredNullable<McpUserActionResolution>,
    pub derived_refs: Vec<StateRecordRef>,
}

/// MCP-safe projection of a verified user-channel resolution.
///
/// User-authored notes and evidence-observation summaries intentionally remain
/// outside the MCP-visible compound response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpUserActionResolution {
    pub user_action_resolution_id: UserActionResolutionId,
    pub user_action_request_id: UserActionRequestId,
    pub action_kind: UserActionKind,
    pub channel_kind: UserActionChannelKind,
    pub resolved_at: UtcTimestamp,
    pub resolution_summary: McpUserActionResolutionSummary,
}

/// Stable top-level error code for a known MCP tool failure before Core entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpToolErrorCode {
    #[serde(rename = "MCP_INVALID_ARGUMENTS")]
    InvalidArguments,
    #[serde(rename = "MCP_ADAPTER_PRECONDITION_FAILED")]
    AdapterPreconditionFailed,
    #[serde(rename = "MCP_ACTION_FORM_STALE")]
    ActionFormStale,
    #[serde(rename = "ACTION_FORM_ARGUMENT_MISMATCH")]
    ActionFormArgumentMismatch,
    #[serde(rename = "WORKFLOW_ACTION_NOT_ALLOWED")]
    WorkflowActionNotAllowed,
    #[serde(rename = "WORKFLOW_ACTION_VARIANT_NOT_ALLOWED")]
    WorkflowActionVariantNotAllowed,
    #[serde(rename = "INTERNAL_CONTRACT_INCONSISTENT")]
    InternalContractInconsistent,
}

/// MCP wire-owned identity for operational unavailability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpOperationalErrorCode {
    #[serde(rename = "MCP_UNAVAILABLE")]
    Unavailable,
}

/// MCP wire projection of the operation that could not complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpOperationalOperation {
    #[serde(rename = "product_path_observation")]
    ProductPathObservation,
    #[serde(rename = "store_access")]
    StoreAccess,
}

/// MCP wire projection of the unavailable infrastructure resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpOperationalResource {
    #[serde(rename = "product_repository")]
    ProductRepository,
    #[serde(rename = "store")]
    Store,
    #[serde(rename = "registry_store")]
    RegistryStore,
    #[serde(rename = "project_store")]
    ProjectStore,
    #[serde(rename = "runtime_home")]
    RuntimeHome,
    #[serde(rename = "platform_environment")]
    PlatformEnvironment,
}

/// Maximum number of independently discoverable issues returned for one known MCP tool call.
pub const MAX_VALIDATION_ISSUES: usize = 32;

/// Maximum UTF-8 byte length of one returned MCP tool issue JSON Pointer.
pub const MAX_MCP_TOOL_ISSUE_PATH_BYTES: usize = 256;

/// Maximum UTF-8 byte length of one returned MCP tool issue message.
pub const MAX_MCP_TOOL_ISSUE_MESSAGE_BYTES: usize = 512;

/// Maximum compact-JSON byte length of one known-tool MCP `CallToolResult` error.
pub const MAX_MCP_TOOL_ERROR_RESULT_BYTES: usize = 64 * 1024;

/// Stable issue code within a known MCP tool error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpToolIssueCode {
    #[serde(rename = "MCP_ARGUMENT_REQUIRED")]
    ArgumentRequired,
    #[serde(rename = "MCP_ARGUMENT_UNKNOWN")]
    ArgumentUnknown,
    #[serde(rename = "MCP_ARGUMENT_TYPE_MISMATCH")]
    ArgumentTypeMismatch,
    #[serde(rename = "MCP_ARGUMENT_ENUM_VALUE")]
    ArgumentEnumValue,
    #[serde(rename = "MCP_ADAPTER_PRECONDITION_FAILED")]
    AdapterPreconditionFailed,
    #[serde(rename = "MCP_ACTION_FORM_MISMATCH")]
    ActionFormMismatch,
    #[serde(rename = "ACTION_FORM_ARGUMENT_MISMATCH")]
    ActionFormArgumentMismatch,
    #[serde(rename = "WORKFLOW_ACTION_NOT_ALLOWED")]
    WorkflowActionNotAllowed,
    #[serde(rename = "WORKFLOW_ACTION_VARIANT_NOT_ALLOWED")]
    WorkflowActionVariantNotAllowed,
    #[serde(rename = "INTERNAL_CONTRACT_INCONSISTENT")]
    InternalContractInconsistent,
}

/// One independently discoverable MCP tool error issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpToolErrorIssue {
    #[schemars(length(max = "MAX_MCP_TOOL_ISSUE_PATH_BYTES"))]
    pub path: String,
    pub code: McpToolIssueCode,
    #[schemars(length(min = 1, max = "MAX_MCP_TOOL_ISSUE_MESSAGE_BYTES"))]
    pub message: String,
    pub expected_semantic_type: RequiredNullable<String>,
    pub allowed_values: Vec<String>,
    pub owner_hint: RequiredNullable<String>,
}

impl McpToolErrorIssue {
    pub fn new(
        path: impl Into<String>,
        code: McpToolIssueCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            code,
            message: message.into(),
            expected_semantic_type: RequiredNullable::null(),
            allowed_values: Vec::new(),
            owner_hint: RequiredNullable::null(),
        }
    }
}

/// One Agent-authored input slot retained by a workflow action form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowActionInput {
    pub path: String,
    pub semantic_type: String,
    pub required: bool,
}

/// MCP executable projection of one Core-owned current workflow action intent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowActionForm {
    pub action_key: volicord_types::schema::WorkflowActionKey,
    pub form_ref: RequestHash,
    pub expected_state_version: u64,
    pub fixed_arguments: JsonObject,
    pub fixed_argument_paths: Vec<String>,
    pub agent_authored_inputs: Vec<WorkflowActionInput>,
    pub canonical_minimal_request: JsonObject,
}

/// Complete MCP form catalog derived from one neutral Core action catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowActionFormCatalog {
    pub required_action_key: RequiredNullable<volicord_types::schema::WorkflowActionKey>,
    pub workflow_contract_digest: RequestHash,
    pub action_form_contract_digest: RequestHash,
    pub semantic_schema_digest: RequestHash,
    pub scalar_contract_digest: RequestHash,
    pub forms: Vec<WorkflowActionForm>,
}

impl WorkflowActionFormCatalog {
    pub fn forms_for_method(
        &self,
        method: MethodName,
    ) -> impl Iterator<Item = &WorkflowActionForm> {
        self.forms
            .iter()
            .filter(move |form| form.action_key.method == method)
    }

    pub fn form(
        &self,
        method: MethodName,
        semantic_variant: WorkflowActionSemanticVariant,
    ) -> Option<&WorkflowActionForm> {
        self.forms.iter().find(|form| {
            form.action_key.method == method && form.action_key.semantic_variant == semantic_variant
        })
    }

    pub fn form_by_ref(&self, form_ref: &RequestHash) -> Option<&WorkflowActionForm> {
        self.forms.iter().find(|form| &form.form_ref == form_ref)
    }

    pub fn required_forms(&self) -> impl Iterator<Item = &WorkflowActionForm> {
        let required_action_key = self.required_action_key.as_ref().copied();
        self.forms
            .iter()
            .filter(move |form| Some(form.action_key) == required_action_key)
    }
}

/// Bounded current authority loaded after independently valid routing coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoritativeArgumentContext {
    pub context_loaded: bool,
    pub project_id: RequiredNullable<ProjectId>,
    pub state_version: RequiredNullable<u64>,
    pub task_mode: RequiredNullable<TaskMode>,
    pub work_phase: RequiredNullable<WorkPhase>,
    pub scope_revision: RequiredNullable<u64>,
    pub baseline_ref: RequiredNullable<BaselineRef>,
    pub current_checkpoint_ref: RequiredNullable<StateRecordRef>,
    pub workflow: RequiredNullable<WorkflowProjection>,
    pub action_form_catalog: RequiredNullable<WorkflowActionFormCatalog>,
}

/// Exact retry values and remaining Agent-authored input slots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetryContract {
    pub recovery_action_key: RequiredNullable<volicord_types::schema::WorkflowActionKey>,
    pub recovery_form: RequiredNullable<WorkflowActionForm>,
    pub attempt_details: TransitionAttemptDetails,
    pub invalid_or_incompatible_submitted_paths: Vec<String>,
    pub retry_possible_in_current_task: bool,
}

/// Bounded current workflow-contract facts used by rejection and session diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpWorkflowContractDiagnostics {
    pub normalized_workflow_snapshot: WorkflowProjection,
    pub current_transition_catalog: volicord_types::schema::WorkflowTransitionCatalog,
    pub current_action_forms: RequiredNullable<WorkflowActionFormCatalog>,
    pub attempted_action_key: RequiredNullable<volicord_types::schema::WorkflowActionKey>,
    pub typed_rejection_reason: RequiredNullable<volicord_types::values::TransitionRejectionReason>,
    pub recovery_action_key: RequiredNullable<volicord_types::schema::WorkflowActionKey>,
    pub failed_action_key: RequiredNullable<volicord_types::schema::WorkflowActionKey>,
    pub failed_stage: RequiredNullable<McpWorkflowContractStage>,
    pub workflow_contract_digest: RequestHash,
    pub action_form_contract_digest: RequestHash,
    pub semantic_schema_digest: RequestHash,
    pub scalar_contract_digest: RequestHash,
}

/// Bounded stage at which current action-form catalog validation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpWorkflowContractStage {
    TransitionContract,
    WitnessProjection,
    SemanticValidation,
    ExactDecode,
    FixedBinding,
    AdapterProjection,
    CorePlanning,
    ExpectedResultValidation,
    CatalogTotality,
}

/// Compact truth about a failed workflow-action attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpArgumentFailurePresentation {
    pub method_committed: bool,
    pub reached_core: bool,
    pub current_task_phase: RequiredNullable<WorkPhase>,
    pub current_state_version: RequiredNullable<u64>,
    pub checkpoint_recorded: bool,
    pub user_action_created: bool,
    pub product_repository_changed: bool,
    pub core_state_unchanged: bool,
    pub current_baseline_canonical: RequiredNullable<bool>,
    pub submitted_baseline_canonical: RequiredNullable<bool>,
    pub submitted_baseline_matches_current: RequiredNullable<bool>,
    pub submitted_baseline_compatible_with_transition: RequiredNullable<bool>,
    pub exact_retry_action: RequiredNullable<String>,
    pub repair_required: bool,
}

/// Exact pre-Core workflow admission facts for a task-state-bound call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpWorkflowAdmissionRejection {
    pub called_method: MethodName,
    pub called_semantic_variant: RequiredNullable<WorkflowActionSemanticVariant>,
    pub current_workflow_kind: WorkflowStateKind,
    pub required_method: RequiredNullable<MethodName>,
    pub allowed_methods: Vec<MethodName>,
    pub valid_called_method_variants: Vec<WorkflowActionSemanticVariant>,
    pub valid_called_method_form_refs: Vec<RequestHash>,
    pub called_method_form: RequiredNullable<WorkflowActionForm>,
    pub required_method_forms: Vec<WorkflowActionForm>,
    pub state_change_applied: bool,
    pub reached_core: bool,
}

/// One exact fixed-authority argument mismatch detected before Core entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpActionFormArgumentMismatch {
    pub method: MethodName,
    pub form_ref: RequestHash,
    pub path: String,
    #[schemars(with = "McpJsonValueSchema")]
    pub expected_value: Value,
    #[schemars(with = "McpJsonValueSchema")]
    pub received_value: Value,
    pub received_value_present: bool,
    pub state_change_applied: bool,
    pub reached_core: bool,
    pub current_method_form: WorkflowActionForm,
}

/// Schema-only recursive projection for raw JSON values in mismatch details.
#[derive(JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "JsonValue")]
#[allow(dead_code)]
enum McpJsonValueSchema {
    Null(()),
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<McpJsonValueSchema>),
    Object(BTreeMap<String, McpJsonValueSchema>),
}

/// Structured known-tool failure returned before Core method entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpToolErrorResponse {
    pub code: McpToolErrorCode,
    pub tool_name: String,
    pub selected_variant: RequiredNullable<String>,
    pub canonical_example: RequiredNullable<JsonObject>,
    pub retryable: bool,
    pub reached_core: bool,
    pub committed: bool,
    pub failed_action_key: RequiredNullable<volicord_types::schema::WorkflowActionKey>,
    pub failed_stage: RequiredNullable<McpWorkflowContractStage>,
    #[schemars(range(min = 1, max = "MAX_VALIDATION_ISSUES"))]
    pub reported_issue_count: usize,
    pub truncated: bool,
    #[schemars(length(min = 1, max = "MAX_VALIDATION_ISSUES"))]
    pub issues: Vec<McpToolErrorIssue>,
    pub authoritative_context: RequiredNullable<AuthoritativeArgumentContext>,
    pub retry_contract: RequiredNullable<RetryContract>,
    pub failure: RequiredNullable<McpArgumentFailurePresentation>,
    pub workflow_admission: RequiredNullable<McpWorkflowAdmissionRejection>,
    pub action_form_argument_mismatches: Vec<McpActionFormArgumentMismatch>,
    pub transition_rejection: RequiredNullable<volicord_types::schema::TransitionRejection>,
    pub contract_diagnostics: RequiredNullable<McpWorkflowContractDiagnostics>,
}

/// Structured MCP failure when an operational dependency cannot produce a tool result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpOperationalFailure {
    pub code: McpOperationalErrorCode,
    pub tool_name: MethodName,
    pub operation: McpOperationalOperation,
    pub resource: McpOperationalResource,
    pub retryable: bool,
    pub reached_core: bool,
    pub committed: bool,
}

/// Structured MCP result advertised by each known tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result_type", rename_all = "snake_case")]
pub enum McpToolStructuredContent<T> {
    Response(Box<T>),
    AdapterError(Box<McpToolErrorResponse>),
}

/// Structured MCP result advertised by a read-only Core-owned tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result_type", rename_all = "snake_case")]
pub enum McpReadOnlyToolStructuredContent<T> {
    Response(Box<T>),
    OperationalFailure(McpOperationalFailure),
    AdapterError(Box<McpToolErrorResponse>),
}

/// Compact method-effect facts preserved by mutation projections and recoveries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpMutationEffectSummary {
    pub effect_kind: EffectKind,
    pub state_version: Option<u64>,
    pub events: Vec<volicord_types::schema::EventRef>,
}

/// Compact `volicord.record_shaping_checkpoint` authority and routing outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRecordShapingCheckpointCompactResult {
    pub effect: McpMutationEffectSummary,
    pub shaping_checkpoint_id: ShapingCheckpointId,
    pub readiness: ShapingCheckpointReadiness,
    pub unresolved_application_owners: Vec<ShapingDecisionApplicationOwner>,
    pub decision_recovery_requirements:
        Vec<volicord_types::schema::ShapingDecisionRecoveryRequirement>,
    pub created_user_action_request_refs: Vec<StateRecordRef>,
    pub shaping_authority_reauthorization_refs: Vec<StateRecordRef>,
    pub workflow_kind: WorkflowStateKind,
    pub close_state: Option<CloseState>,
    pub close_blocker_count: usize,
}

/// Compact `volicord.finalize_advice` authority and close-basis outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpFinalizeAdviceCompactResult {
    pub effect: McpMutationEffectSummary,
    pub shaping_checkpoint_id: ShapingCheckpointId,
    pub readiness: ShapingCheckpointReadiness,
    pub unresolved_application_owners: Vec<ShapingDecisionApplicationOwner>,
    pub decision_recovery_requirements:
        Vec<volicord_types::schema::ShapingDecisionRecoveryRequirement>,
    pub applied_shaping_decision_application_refs: Vec<StateRecordRef>,
    pub workflow_kind: WorkflowStateKind,
    pub close_state: Option<CloseState>,
    pub close_blocker_count: usize,
}

/// Compact `volicord.update_scope` exact decision-application outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpUpdateScopeCompactResult {
    pub effect: McpMutationEffectSummary,
    pub applied_shaping_gap_refs: Vec<StateRecordRef>,
    pub applied_scope_decision_refs: Vec<StateRecordRef>,
    pub applied_shaping_decision_application_refs: Vec<StateRecordRef>,
}

/// Compact `volicord.advance_task` exact decision-application outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpAdvanceTaskCompactResult {
    pub effect: McpMutationEffectSummary,
    pub applied_shaping_gap_refs: Vec<StateRecordRef>,
    pub applied_user_action_resolution_refs: Vec<StateRecordRef>,
    pub applied_shaping_decision_application_refs: Vec<StateRecordRef>,
}

/// Compact `volicord.prepare_write` outcome needed by the next write step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpPrepareWriteCompactResult {
    pub effect: McpMutationEffectSummary,
    pub decision: PrepareWriteDecision,
    pub write_ticket_id: Option<WriteTicketId>,
    pub write_ticket_ref: Option<StateRecordRef>,
    pub write_ticket: Option<WriteTicket>,
    pub write_ticket_effect: WriteTicketEffect,
    pub allowed_path_patterns: Vec<String>,
    pub denied_path_patterns: Vec<String>,
    pub write_decision_reasons: Vec<WriteDecisionReason>,
    pub user_action_draft: Option<UserActionDraft>,
}

/// Compact `volicord.prepare_evidence_capture` outcome needed by the source and Run steps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpPrepareEvidenceCaptureCompactResult {
    pub effect: McpMutationEffectSummary,
    pub capture_intent_ref: StateRecordRef,
    pub capture_intent: EvidenceCaptureIntent,
    pub expires_at: UtcTimestamp,
}

/// Compact `volicord.stage_artifact` outcome needed to consume the staged input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpStageArtifactCompactResult {
    pub effect: McpMutationEffectSummary,
    pub evidence_state: EvidenceDisplayState,
    pub staged_artifact_handle: StagedArtifactHandle,
    pub expires_at: UtcTimestamp,
}

/// Task-owned close-basis coordinates created by one compact `volicord.record_run` result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRecordRunCloseBasisAnchor {
    pub close_basis_revision: u64,
    pub scope_revision: u64,
    pub source_run_ref: RequiredNullable<StateRecordRef>,
    pub evidence_summary_ref: RequiredNullable<StateRecordRef>,
}

/// Compact `volicord.record_run` outcome needed by evidence and close follow-up work.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRecordRunCompactResult {
    pub effect: McpMutationEffectSummary,
    pub run_ref: StateRecordRef,
    pub registered_artifact_refs: Vec<ArtifactRef>,
    pub evidence_observation_refs: Vec<StateRecordRef>,
    pub evidence_producer_refs: Vec<StateRecordRef>,
    pub close_basis_anchor: RequiredNullable<McpRecordRunCloseBasisAnchor>,
}

/// Compact host-native user-action outcome safe for ordinary agent consumption.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRequestUserActionCompactResult {
    pub effect: McpMutationEffectSummary,
    pub agent_workflow_result_replayed: bool,
    pub user_action_request_summary: AgentSafeUserActionRequestSummary,
    pub current_projection_state_version: u64,
    pub current_projection_observed_at: UtcTimestamp,
    pub user_action_resolution_ref: RequiredNullable<StateRecordRef>,
    pub status: UserActionStatus,
    pub resolution_summary: RequiredNullable<McpUserActionResolutionSummary>,
    pub derived_refs: Vec<StateRecordRef>,
}

/// Closed compact resolution summary preserving choice and observation semantics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "resolution_type",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum McpUserActionResolutionSummary {
    Choice {
        selected_option_id: volicord_types::ids::UserActionOptionId,
        selected_option_label: String,
        machine_action: UserActionOptionAction,
        resolution_outcome: JudgmentResolutionOutcome,
    },
    EvidenceObservation {
        target: EvidenceTarget,
        artifact_refs: Vec<ArtifactRef>,
        relevance_status: EvidenceRelevanceStatus,
    },
}

/// Compact per-finding `volicord.reconcile_changes` outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpReconcileChangesCompactResult {
    pub effect: McpMutationEffectSummary,
    pub unresolved_changes: Vec<volicord_types::schema::UnrecordedChangeFinding>,
    pub resolved_changes: Vec<volicord_types::schema::UnrecordedChangeResolutionSummary>,
    pub pending_user_action_summaries: Vec<AgentSafeUserActionRequestSummary>,
    pub rejected_resolution_requests: Vec<UnrecordedChangeRejection>,
}

/// State-change class used by the canonical agent-facing presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpAgentStateChange {
    CoreCommitted,
    StagingCreated,
    DryRun,
    Rejected,
    ReadOnlyResume,
    NoEffect,
}

/// Exact Task coordinates shown in every workflow presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpTaskPhasePresentation {
    pub mode: TaskMode,
    pub work_phase: WorkPhase,
}

/// One structured blocker and its exact recovery owner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpWorkflowBlockerSummary {
    pub code: RequiredNullable<ErrorCode>,
    pub owner_method: MethodName,
    pub required_refs: Vec<StateRecordRef>,
    pub user_actions: Vec<WorkflowRejectionUserAction>,
}

/// MCP wire projection of the canonical CLI User Channel instruction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpUserChannelInstructions {
    pub channel_kind: UserActionChannelKind,
    pub list_command: String,
    pub request_refs: Vec<StateRecordRef>,
    pub chat_reply_is_resolution: volicord_types::schema::FalseValue,
}

/// A fact the caller must preserve when presenting one workflow result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "fact_kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpMustSurfaceFact {
    MethodRejected {
        method: MethodName,
        core_state_unchanged: volicord_types::schema::TrueValue,
    },
    CurrentTaskPhase {
        mode: TaskMode,
        work_phase: WorkPhase,
    },
    RecordRunKindRejected {
        received_run_kind: RunKind,
        allowed_run_kinds: Vec<RunKind>,
    },
    RecoveryAction {
        action_key: WorkflowActionKey,
    },
    ShapingDecisionOutcome {
        request_ref: StateRecordRef,
        resolution_ref: RequiredNullable<StateRecordRef>,
        disposition: volicord_types::values::ShapingDecisionAuthorityState,
        authority_granted: bool,
    },
    NonAuthorizingShapingDecision {
        request_ref: StateRecordRef,
        resolution_ref: RequiredNullable<StateRecordRef>,
        disposition: volicord_types::values::ShapingDecisionAuthorityState,
        recovery_action_key: RequiredNullable<WorkflowActionKey>,
        authority_granted: volicord_types::schema::FalseValue,
        terminal_request_cannot_be_retried: volicord_types::schema::TrueValue,
        successor_request_required_if_still_needed: volicord_types::schema::TrueValue,
        chat_text_cannot_replace_successor: volicord_types::schema::TrueValue,
        product_repository_mutation_available: volicord_types::schema::FalseValue,
    },
    UserActionRequestExists {
        request_refs: Vec<StateRecordRef>,
    },
    NextActorIsUser,
    ChatReplyIsNotResolution,
    ProductRepositoryMutationBlockedUntilUserChannelResolution,
    ImplementationBlockedUntilUserActionAuthoritySatisfied {
        request_refs: Vec<StateRecordRef>,
    },
    EnteredImplementation,
    PhaseTransitionCreatedNoWriteTicket,
    ProductRepositoryWritesRequirePrepareWrite {
        owner_method: MethodName,
    },
}

/// Canonical agent-facing presentation carried by mutation projections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpWorkflowPresentation {
    pub headline: String,
    pub state_change: McpAgentStateChange,
    pub task_phase: McpTaskPhasePresentation,
    pub next_actor: volicord_types::values::AuthorityNextActor,
    pub blocker_summary: Vec<McpWorkflowBlockerSummary>,
    pub required_user_action: RequiredNullable<McpUserChannelInstructions>,
    pub must_surface: Vec<McpMustSurfaceFact>,
    pub action_form_catalog: WorkflowActionFormCatalog,
}

/// Rejected mutation plus current authoritative workflow and presentation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpWorkflowRejectedResponse {
    pub method_result: ToolRejectedResponse,
    pub authority_receipt: AuthorityReceipt,
    pub workflow: WorkflowProjection,
    pub presentation: McpWorkflowPresentation,
    pub transition_rejection: RequiredNullable<volicord_types::schema::TransitionRejection>,
    pub retry_contract: RequiredNullable<RetryContract>,
    pub failure: McpArgumentFailurePresentation,
    pub contract_diagnostics: McpWorkflowContractDiagnostics,
}

/// MCP status projection with the executable current action form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpStatusResponse {
    pub method_result: volicord_types::methods::StatusResponse,
    pub action_form_catalog: RequiredNullable<WorkflowActionFormCatalog>,
}

/// Dry-run mutation preview plus current authoritative workflow and presentation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpWorkflowDryRunResponse {
    pub method_result: ToolDryRunResponse,
    pub authority_receipt: AuthorityReceipt,
    pub workflow: WorkflowProjection,
    pub presentation: McpWorkflowPresentation,
}

/// Summary-detail MCP mutation branch over one fresh authority receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpMutationSummaryResponse<T> {
    pub operation_result_ref: RequiredNullable<OperationResultRef>,
    pub authority_receipt: AuthorityReceipt,
    pub method_result: T,
    pub presentation: McpWorkflowPresentation,
}

/// Workflow-detail MCP mutation branch over one fresh authority receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpMutationWorkflowResponse<T> {
    pub operation_result_ref: RequiredNullable<OperationResultRef>,
    pub authority_receipt: AuthorityReceipt,
    pub method_result: T,
    pub workflow: WorkflowProjection,
    pub presentation: McpWorkflowPresentation,
}

/// Full-detail MCP mutation branch over one fresh authority receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpMutationFullResponse<T> {
    pub operation_result_ref: RequiredNullable<OperationResultRef>,
    pub authority_receipt: AuthorityReceipt,
    pub method_result: T,
    pub presentation: McpWorkflowPresentation,
}

/// Bounded non-retryable MCP recovery branch used when authoritative refresh fails.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpAuthoritativeRefreshFailure<T> {
    pub code: McpOperationalErrorCode,
    pub tool_name: MethodName,
    pub retryable: bool,
    pub reached_core: bool,
    pub committed: bool,
    pub effect_kind: RequiredNullable<EffectKind>,
    pub effect_applied: bool,
    pub effect_anchor: RequiredNullable<String>,
    pub operation_result_ref: RequiredNullable<OperationResultRef>,
    pub method_result: RequiredNullable<T>,
    pub status_read_required: bool,
    pub completion_claim_withheld: bool,
}

/// Stable MCP adapter code for a compact mutation projection failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum McpMutationProjectionErrorCode {
    McpResponseBudgetExceeded,
}

/// Stable MCP adapter code for a failure after a mutation effect was observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum McpPostEffectFailureCode {
    McpResponseProjectionFailed,
    McpPostEffectAdapterFailed,
}

/// Bounded success-class recovery when post-effect adapter work cannot safely
/// project a normal mutation response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpMutationPostEffectFailure {
    pub code: McpPostEffectFailureCode,
    pub tool_name: MethodName,
    pub requested_detail: MutationDetailLevel,
    pub retryable: bool,
    pub reached_core: bool,
    pub committed: bool,
    pub effect_kind: RequiredNullable<EffectKind>,
    pub effect_applied: bool,
    pub effect_anchor: RequiredNullable<String>,
    pub operation_result_ref: RequiredNullable<OperationResultRef>,
    pub authority_receipt: RequiredNullable<AuthorityReceipt>,
    pub method_result: RequiredNullable<JsonObject>,
    pub authoritative_refresh_succeeded: bool,
    pub response_projection_omitted: bool,
    pub status_read_required: bool,
    pub completion_claim_withheld: bool,
}

/// Bounded non-retryable MCP recovery branch used when a projection exceeds its budget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpMutationResponseBudgetExceeded<T> {
    pub code: McpMutationProjectionErrorCode,
    pub tool_name: MethodName,
    pub requested_detail: MutationDetailLevel,
    pub retryable: bool,
    pub reached_core: bool,
    pub committed: bool,
    pub effect_kind: RequiredNullable<EffectKind>,
    pub effect_applied: bool,
    pub effect_anchor: RequiredNullable<String>,
    pub operation_result_ref: RequiredNullable<OperationResultRef>,
    pub authority_receipt: RequiredNullable<AuthorityReceipt>,
    pub method_result: RequiredNullable<T>,
    pub authoritative_refresh_succeeded: bool,
    pub response_projection_omitted: bool,
    pub status_read_required: bool,
    pub completion_claim_withheld: bool,
}

/// Structured MCP output advertised by mutation tools.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result_type", rename_all = "snake_case")]
pub enum McpMutationStructuredContent<T, C> {
    Rejected(Box<McpWorkflowRejectedResponse>),
    DryRun(Box<McpWorkflowDryRunResponse>),
    Full(McpMutationFullResponse<Box<T>>),
    Summary(McpMutationSummaryResponse<C>),
    Workflow(Box<McpMutationWorkflowResponse<C>>),
    OperationalFailure(McpOperationalFailure),
    RefreshFailure(McpAuthoritativeRefreshFailure<C>),
    ResponseBudgetExceeded(McpMutationResponseBudgetExceeded<C>),
    PostEffectFailure(McpMutationPostEffectFailure),
    AdapterError(Box<McpToolErrorResponse>),
}

/// MCP-visible `volicord.intake` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpIntakeArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub plain_language_request: String,
    pub requested_mode: RequestedMode,
    #[serde(default)]
    pub requested_control_level: RequestedControlLevel,
    pub resume_policy: ResumePolicy,
    pub acceptance_policy: RequiredNullable<AcceptancePolicy>,
    pub lineage: RequiredNullable<volicord_types::schema::TaskLineageInput>,
    pub initial_scope: InitialScope,
    /// Typed authority context only. Each item is a complete `StateRecordRef`;
    /// human prose belongs in `plain_language_request`.
    #[serde(default)]
    pub initial_context_refs: Vec<StateRecordRef>,
    #[serde(default)]
    pub initial_source_refs: Vec<SourceRef>,
}

/// MCP-visible `volicord.update_scope` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpUpdateScopeArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub action_form_ref: RequestHash,
    pub task_id: TaskId,
    pub goal_summary: RequiredNullable<String>,
    pub scope_update: RequiredNullable<ScopeUpdate>,
    pub scope_boundary: RequiredNullable<String>,
    pub non_goals: RequiredNullable<Vec<String>>,
    pub acceptance_criteria: RequiredNullable<Vec<AcceptanceCriterionReplacement>>,
    pub autonomy_boundary: RequiredNullable<String>,
    pub baseline_ref: RequiredNullable<BaselineRef>,
    pub change_unit: ChangeUnitUpdate,
    #[serde(default)]
    pub related_scope_decision_refs: Vec<StateRecordRef>,
}

/// MCP-visible `volicord.record_shaping_checkpoint` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRecordShapingCheckpointArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub action_form_ref: RequestHash,
    pub task_id: TaskId,
    pub checkpoint_operation: ShapingCheckpointOperation,
    pub scope_revision: u64,
    pub baseline_ref: RequiredNullable<BaselineRef>,
    pub summary: String,
    pub implementation_boundary: RequiredNullable<String>,
    #[serde(default)]
    pub gaps: Vec<ShapingGapInput>,
    #[serde(default)]
    pub source_refs: Vec<SourceRef>,
    #[serde(default)]
    pub evidence_refs: Vec<StateRecordRef>,
}

/// MCP-visible `volicord.finalize_advice` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpFinalizeAdviceArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub action_form_ref: RequestHash,
    pub task_id: TaskId,
    pub shaping_checkpoint_id: ShapingCheckpointId,
    pub change_unit_id: ChangeUnitId,
    pub scope_revision: u64,
    pub baseline_ref: BaselineRef,
    #[serde(default)]
    pub user_action_resolution_ids: Vec<UserActionResolutionId>,
    pub result_summary: String,
    #[serde(default)]
    pub result_refs: Vec<StateRecordRef>,
    #[serde(default)]
    pub evidence_refs: Vec<StateRecordRef>,
    #[serde(default)]
    pub residual_risks: Vec<ResidualRiskInput>,
    #[serde(default)]
    pub recovery_constraints: Vec<String>,
}

/// MCP-visible `volicord.advance_task` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpAdvanceTaskArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub action_form_ref: RequestHash,
    pub task_id: TaskId,
    pub shaping_checkpoint_id: ShapingCheckpointId,
    pub change_unit_id: ChangeUnitId,
    pub scope_revision: u64,
    pub baseline_ref: BaselineRef,
    #[serde(default)]
    pub user_action_resolution_ids: Vec<UserActionResolutionId>,
}

/// MCP-visible `volicord.status` arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpStatusArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    pub task_id: RequiredNullable<TaskId>,
    #[serde(default)]
    pub detail: StatusDetailLevel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuity_page: Option<RequiredNullable<ContinuityPageRequest>>,
}

/// Expands the MCP-visible detail level into the Core status include matrix.
pub const fn status_include(detail: StatusDetailLevel) -> StatusInclude {
    match detail {
        StatusDetailLevel::Summary => StatusInclude {
            task: true,
            pending_user_actions: false,
            write_ticket: false,
            evidence: false,
            close: false,
            guarantees: false,
            continuity: false,
        },
        StatusDetailLevel::Workflow => StatusInclude {
            task: true,
            pending_user_actions: true,
            write_ticket: true,
            evidence: true,
            close: true,
            guarantees: true,
            continuity: false,
        },
        StatusDetailLevel::Full => StatusInclude {
            task: true,
            pending_user_actions: true,
            write_ticket: true,
            evidence: true,
            close: true,
            guarantees: true,
            continuity: true,
        },
    }
}

/// MCP-visible `volicord.get_operation_result` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpGetOperationResultArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    pub operation_result_ref: OperationResultRef,
    pub cursor: RequiredNullable<String>,
}

/// MCP-visible evidence-capture source selection with required-nullable expected outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "capture_kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpEvidenceCaptureSpec {
    VerifiedCommandExecution {
        command_sha256: String,
        command_label: String,
        expected_exit_code: RequiredNullable<i32>,
    },
    VerifiedToolInvocation {
        tool_name: String,
        tool_input_sha256: String,
        expected_success: RequiredNullable<bool>,
    },
}

impl McpEvidenceCaptureSpec {
    /// Maps the MCP wire value into the adapter-neutral Core schema.
    pub fn into_core(self) -> EvidenceCaptureSpec {
        match self {
            Self::VerifiedCommandExecution {
                command_sha256,
                command_label,
                expected_exit_code,
            } => EvidenceCaptureSpec::VerifiedCommandExecution {
                command_sha256,
                command_label,
                expected_exit_code,
            },
            Self::VerifiedToolInvocation {
                tool_name,
                tool_input_sha256,
                expected_success,
            } => EvidenceCaptureSpec::VerifiedToolInvocation {
                tool_name,
                tool_input_sha256,
                expected_success,
            },
        }
    }
}

/// MCP-visible `volicord.prepare_evidence_capture` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpPrepareEvidenceCaptureArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub action_form_ref: RequestHash,
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub baseline_ref: BaselineRef,
    pub target: EvidenceTarget,
    pub capture: McpEvidenceCaptureSpec,
}

/// MCP-visible `volicord.prepare_write` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpPrepareWriteArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub action_form_ref: RequestHash,
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub intended_operation: String,
    pub intended_paths: Vec<String>,
    pub product_file_write_intended: bool,
    #[serde(default)]
    pub sensitive_categories: Vec<String>,
    pub baseline_ref: BaselineRef,
}

/// MCP-visible `volicord.stage_artifact` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpStageArtifactArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub action_form_ref: RequestHash,
    pub task_id: TaskId,
    pub display_name: String,
    pub content_type: String,
    pub redaction_state: RedactionState,
    pub safe_bytes_or_notice: String,
    pub expected_sha256: RequiredNullable<String>,
    pub expected_size_bytes: RequiredNullable<u64>,
    pub relation_hint: RequiredNullable<String>,
}

/// MCP-visible `volicord.record_run` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRecordRunArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub action_form_ref: RequestHash,
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub kind: RunKind,
    pub run_id: RequiredNullable<RunId>,
    pub baseline_ref: BaselineRef,
    pub write_ticket_id: RequiredNullable<WriteTicketId>,
    pub performed_operation: RequiredNullable<String>,
    pub summary: String,
    pub observed_changes: ObservedChanges,
    #[serde(default)]
    pub artifact_inputs: Vec<ArtifactInput>,
    #[serde(default)]
    pub evidence_updates: Vec<McpEvidenceCoverageUpdate>,
    #[serde(default)]
    pub evidence_observations: Vec<McpEvidenceObservationInput>,
    pub close_assessment: RequiredNullable<CloseAssessmentInput>,
}

/// MCP-visible evidence coverage input with omission-equivalent collection defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpEvidenceCoverageUpdate {
    pub target: EvidenceTarget,
    pub coverage_state: EvidenceCoverageUpdateState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<EvidenceUpdateProvenance>,
    #[serde(default)]
    pub supporting_run_refs: Vec<StateRecordRef>,
    #[serde(default)]
    pub observation_refs: Vec<StateRecordRef>,
    #[serde(default)]
    pub supporting_artifact_refs: Vec<ArtifactRef>,
    #[serde(default)]
    pub gap_refs: Vec<StateRecordRef>,
}

impl McpEvidenceCoverageUpdate {
    /// Maps the MCP wire value into the adapter-neutral Core schema.
    pub fn into_core(self) -> EvidenceCoverageUpdate {
        EvidenceCoverageUpdate {
            target: self.target,
            coverage_state: self.coverage_state,
            provenance: self.provenance,
            supporting_run_refs: self.supporting_run_refs,
            observation_refs: self.observation_refs,
            supporting_artifact_refs: self.supporting_artifact_refs,
            gap_refs: self.gap_refs,
        }
    }
}

/// MCP-visible evidence observation input with required-nullable source coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpEvidenceObservationInput {
    pub target: EvidenceTarget,
    pub source_kind: EvidenceSourceKind,
    pub assurance_level: EvidenceAssuranceLevel,
    pub observed_by_actor_source: RequiredNullable<ActorSource>,
    pub tool_name: RequiredNullable<String>,
    pub tool_invocation_id: RequiredNullable<String>,
    #[serde(default)]
    pub tool_metadata: JsonObject,
    #[serde(default)]
    pub input_refs: Vec<StateRecordRef>,
    #[serde(default)]
    pub source_refs: Vec<SourceRef>,
    #[serde(default)]
    pub output_artifact_refs: Vec<ArtifactRef>,
    #[serde(default)]
    pub limitations: Vec<String>,
    pub observed_at: UtcTimestamp,
}

impl McpEvidenceObservationInput {
    /// Maps the MCP wire value into the adapter-neutral Core schema.
    pub fn into_core(self) -> EvidenceObservationInput {
        EvidenceObservationInput {
            target: self.target,
            source_kind: self.source_kind,
            assurance_level: self.assurance_level,
            observed_by_actor_source: self.observed_by_actor_source,
            tool_name: self.tool_name,
            tool_invocation_id: self.tool_invocation_id,
            tool_metadata: self.tool_metadata,
            input_refs: self.input_refs,
            source_refs: self.source_refs,
            output_artifact_refs: self.output_artifact_refs,
            limitations: self.limitations,
            observed_at: self.observed_at,
        }
    }
}

/// MCP-visible `volicord.request_user_action` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRequestUserActionArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_form_ref: Option<RequestHash>,
    pub request: McpRequestUserActionOperation,
}

/// Create-or-resume operation selected for the MCP user-action tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpRequestUserActionOperation {
    Create {
        task_id: TaskId,
        change_unit_id: RequiredNullable<ChangeUnitId>,
        action: UserActionDraft,
        required_for: Vec<UserActionRequiredFor>,
        expires_at: RequiredNullable<UtcTimestamp>,
    },
    Resume {
        user_action_request_id: UserActionRequestId,
    },
}

/// MCP-visible `volicord.reconcile_changes` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpReconcileChangesArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub action_form_ref: RequestHash,
    pub task_id: TaskId,
    #[serde(default)]
    pub resolution_requests: Vec<UnrecordedChangeResolutionRequest>,
}

/// MCP-visible read-only `volicord.check_close` arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpCheckCloseArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    pub action_form_ref: RequestHash,
    pub task_id: TaskId,
}

/// MCP-visible workflow `volicord.close_task` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpCloseTaskArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub action_form_ref: RequestHash,
    pub task_id: TaskId,
    pub intent: CloseMutationIntent,
    pub close_reason: RequiredNullable<CloseReason>,
    pub superseding_task_id: RequiredNullable<TaskId>,
    pub user_note: RequiredNullable<String>,
}

/// MCP-visible empty argument object for `volicord.list_projects`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpListProjectsArguments {}

/// MCP-visible `volicord.list_projects` result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpListProjectsResult {
    pub connection_id: String,
    pub mode: volicord_types::values::AgentConnectionMode,
    pub projects: Vec<McpListProjectItem>,
}

/// One connection-allowed project returned by `volicord.list_projects`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpListProjectItem {
    pub project_selector: String,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub repo_root: String,
}

/// Returns the generated JSON Schema for one MCP-visible tool argument shape.
pub fn mcp_request_schema(tool: AgentToolId) -> Option<Value> {
    crate::tool_contracts::mcp_tool_contract(tool).map(|contract| contract.input_schema())
}

/// Returns the generated JSON Schema for one MCP-visible public method result.
pub fn mcp_response_schema(tool: AgentToolId) -> Option<Value> {
    crate::tool_contracts::mcp_tool_contract(tool).map(|contract| contract.output_schema())
}
