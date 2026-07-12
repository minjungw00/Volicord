use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ids::{
    ArtifactId, BaselineRef, ChangeUnitId, RunId, TaskId, UnrecordedChangeId, UserJudgmentId,
    UserJudgmentOptionId, WriteTicketId,
};
use crate::schema::{
    AcceptanceCriterionInput, AcceptanceCriterionReplacement, AcceptedRiskInput, ArtifactInput,
    ArtifactRef, AuthorityReceipt, ChangeUnitEffectContract, CloseAssessmentInput,
    CloseReadinessBlocker, ControlSurfaceSummary, CoverageSummary, CurrentCloseBasis,
    EvidenceCoverageUpdate, EvidenceGateSummary, EvidenceObservation, EvidenceObservationInput,
    EvidenceSummary, EvidenceTarget, EvidenceUpdateProvenance, GuaranteeDisplay,
    GuardHealthSummary, JsonObject, JudgmentInboxItem, JudgmentRationale, NextActionSummary,
    ObservedChanges, ProjectContinuitySummary, RecordUserJudgmentPayload, RequiredNullable,
    RiskAcceptanceCoverage, RunSummary, SensitiveActionScope, SourceRef, StagedArtifactHandle,
    StateRecordRef, StateSummary, SummaryCard, TaskFlowItem, TaskLineageInput, ToolEnvelope,
    ToolResponse, ToolResultBase, UnrecordedChangeFinding, UnrecordedChangeResolutionSummary,
    UserChannelAvailability, UserEvidenceObservation, UserJudgment, UserJudgmentCandidate,
    UserJudgmentContext, UserJudgmentOptionInput, WriteDecisionReason, WriteTicket,
    WriteTicketStateSummary,
};
use crate::values::{
    AcceptancePolicy, ActorSource, ChangeUnitOperation, CloseMutationIntent, CloseReason,
    CloseState, ErrorCode, EvidenceAssuranceLevel, EvidenceCoverageUpdateState,
    EvidenceDisplayState, EvidenceRelevanceStatus, EvidenceSourceKind, JudgmentKind,
    JudgmentPresentation, JudgmentRequiredFor, MethodName, MutationDetailLevel, OperationCategory,
    PrepareWriteDecision, RedactionState, RequestedMode, ResumePolicy, RunKind, StatusCloseState,
    StatusDetailLevel, UnrecordedChangeResolutionBasis, UtcTimestamp, WriteTicketEffect,
};

/// Shared typed mapping from a public request to its operation category.
pub trait MethodOperationCategory {
    /// Returns the public method name for this typed request.
    fn method_name(&self) -> MethodName;

    /// Returns the operation category for this typed request.
    fn operation_category(&self) -> OperationCategory;
}

/// Response branch type for `volicord.intake`.
pub type IntakeResponse = ToolResponse<IntakeResult>;

/// Response branch type for `volicord.update_scope`.
pub type UpdateScopeResponse = ToolResponse<UpdateScopeResult>;

/// Response branch type for `volicord.status`.
pub type StatusResponse = ToolResponse<StatusResult>;

/// Response branch type for `volicord.check_close`.
pub type CheckCloseResponse = ToolResponse<CloseTaskResult>;

/// Response branch type for `volicord.prepare_write`.
pub type PrepareWriteResponse = ToolResponse<PrepareWriteResult>;

/// Response branch type for `volicord.stage_artifact`.
pub type StageArtifactResponse = ToolResponse<StageArtifactResult>;

/// Response branch type for `volicord.record_run`.
pub type RecordRunResponse = ToolResponse<RecordRunResult>;

/// Response branch type for `volicord.request_user_judgment`.
pub type RequestUserJudgmentResponse = ToolResponse<RequestUserJudgmentResult>;

/// Response branch type for `volicord.record_user_judgment`.
pub type RecordUserJudgmentResponse = ToolResponse<RecordUserJudgmentResult>;

/// Response branch type for `volicord.record_user_observation`.
pub type RecordUserObservationResponse = ToolResponse<RecordUserObservationResult>;

/// Response branch type for `volicord.reconcile_changes`.
pub type ReconcileChangesResponse = ToolResponse<ReconcileChangesResult>;

/// Response branch type for `volicord.close_task`.
pub type CloseTaskResponse = ToolResponse<CloseTaskResult>;

/// MCP response branches for `volicord.request_user_judgment`.
///
/// Host elicitation may resolve the newly pending judgment before the original
/// MCP tool call returns, so the tool surface can return either public method
/// response without weakening either public API method's own response type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum McpRequestUserJudgmentResponse {
    Pending(Box<RequestUserJudgmentResponse>),
    Recorded(Box<RecordUserJudgmentResponse>),
}

/// Stable top-level error code for a known MCP tool failure before Core entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpToolErrorCode {
    #[serde(rename = "MCP_INVALID_ARGUMENTS")]
    InvalidArguments,
    #[serde(rename = "MCP_ADAPTER_PRECONDITION_FAILED")]
    AdapterPreconditionFailed,
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
    #[serde(rename = "MCP_ARGUMENT_DECODE_FAILED")]
    ArgumentDecodeFailed,
    #[serde(rename = "MCP_ADAPTER_PRECONDITION_FAILED")]
    AdapterPreconditionFailed,
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
}

/// Structured known-tool failure returned before Core method entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpToolErrorResponse {
    pub code: McpToolErrorCode,
    pub tool_name: String,
    pub retryable: bool,
    pub reached_core: bool,
    pub committed: bool,
    #[schemars(range(min = 1, max = "MAX_VALIDATION_ISSUES"))]
    pub reported_issue_count: usize,
    pub truncated: bool,
    #[schemars(length(min = 1, max = "MAX_VALIDATION_ISSUES"))]
    pub issues: Vec<McpToolErrorIssue>,
}

/// Structured MCP result advertised by each known tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum McpToolStructuredContent<T> {
    Response(Box<T>),
    AdapterError(McpToolErrorResponse),
}

/// Workflow-detail MCP mutation branch over one fresh authority receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpMutationWorkflowResponse {
    pub authority_receipt: AuthorityReceipt,
    pub next_actions: Vec<NextActionSummary>,
}

/// Fail-closed MCP branch used when post-mutation authoritative refresh fails.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpAuthoritativeRefreshFailure {
    pub code: ErrorCode,
    pub tool_name: MethodName,
    pub reached_core: bool,
    pub committed: bool,
    pub completion_claim_withheld: bool,
}

/// Stable MCP adapter code for a compact mutation projection failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum McpMutationProjectionErrorCode {
    McpResponseBudgetExceeded,
}

/// Bounded MCP branch used when a fresh mutation projection exceeds its budget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpMutationResponseBudgetExceeded {
    pub code: McpMutationProjectionErrorCode,
    pub tool_name: MethodName,
    pub requested_detail: MutationDetailLevel,
    pub reached_core: bool,
    pub committed: bool,
    pub authoritative_refresh_succeeded: bool,
    pub response_projection_omitted: bool,
    pub completion_claim_withheld: bool,
}

/// Structured MCP output advertised by mutation tools.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum McpMutationStructuredContent<T> {
    Full(Box<T>),
    Summary(AuthorityReceipt),
    Workflow(McpMutationWorkflowResponse),
    RefreshFailure(McpAuthoritativeRefreshFailure),
    ResponseBudgetExceeded(McpMutationResponseBudgetExceeded),
    AdapterError(McpToolErrorResponse),
}

/// `volicord.intake` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IntakeRequest {
    pub envelope: ToolEnvelope,
    pub plain_language_request: String,
    pub requested_mode: RequestedMode,
    pub resume_policy: ResumePolicy,
    pub acceptance_policy: RequiredNullable<AcceptancePolicy>,
    pub lineage: RequiredNullable<TaskLineageInput>,
    pub initial_scope: InitialScope,
    pub initial_context_refs: Vec<StateRecordRef>,
    pub initial_source_refs: Vec<SourceRef>,
}

impl MethodOperationCategory for IntakeRequest {
    fn method_name(&self) -> MethodName {
        MethodName::Intake
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::AgentWorkflow
    }
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
    pub resume_policy: ResumePolicy,
    pub acceptance_policy: RequiredNullable<AcceptancePolicy>,
    pub lineage: RequiredNullable<TaskLineageInput>,
    pub initial_scope: InitialScope,
    #[serde(default)]
    pub initial_context_refs: Vec<StateRecordRef>,
    #[serde(default)]
    pub initial_source_refs: Vec<SourceRef>,
}

/// Intake initial scope object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitialScope {
    pub boundary: String,
    pub non_goals: Vec<String>,
    pub acceptance_criteria: Vec<AcceptanceCriterionInput>,
}

/// `volicord.intake` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct IntakeResult {
    pub base: ToolResultBase,
    pub task_ref: StateRecordRef,
    pub change_unit_ref: Option<StateRecordRef>,
    pub state: StateSummary,
    pub next_actions: Vec<NextActionSummary>,
}

/// `volicord.update_scope` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateScopeRequest {
    pub envelope: ToolEnvelope,
    pub task_id: TaskId,
    pub goal_summary: RequiredNullable<String>,
    pub scope_update: RequiredNullable<ScopeUpdate>,
    pub scope_boundary: RequiredNullable<String>,
    pub non_goals: RequiredNullable<Vec<String>>,
    pub acceptance_criteria: RequiredNullable<Vec<AcceptanceCriterionReplacement>>,
    pub autonomy_boundary: RequiredNullable<String>,
    pub baseline_ref: RequiredNullable<BaselineRef>,
    pub change_unit: ChangeUnitUpdate,
    pub related_scope_decision_refs: Vec<StateRecordRef>,
}

impl MethodOperationCategory for UpdateScopeRequest {
    fn method_name(&self) -> MethodName {
        MethodName::UpdateScope
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::AgentWorkflow
    }
}

/// MCP-visible `volicord.update_scope` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpUpdateScopeArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub task_id: TaskId,
    #[serde(default)]
    pub goal_summary: RequiredNullable<String>,
    #[serde(default)]
    pub scope_update: RequiredNullable<ScopeUpdate>,
    #[serde(default)]
    pub scope_boundary: RequiredNullable<String>,
    #[serde(default)]
    pub non_goals: RequiredNullable<Vec<String>>,
    #[serde(default)]
    pub acceptance_criteria: RequiredNullable<Vec<AcceptanceCriterionReplacement>>,
    #[serde(default)]
    pub autonomy_boundary: RequiredNullable<String>,
    #[serde(default)]
    pub baseline_ref: RequiredNullable<BaselineRef>,
    pub change_unit: ChangeUnitUpdate,
    #[serde(default)]
    pub related_scope_decision_refs: Vec<StateRecordRef>,
}

/// Include/exclude scope-update object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScopeUpdate {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

/// Change Unit update object. Additional method-owned fields remain object data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChangeUnitUpdate {
    pub operation: ChangeUnitOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect_contract: Option<ChangeUnitEffectContract>,
    #[serde(flatten)]
    pub fields: JsonObject,
}

/// `volicord.update_scope` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateScopeResult {
    pub base: ToolResultBase,
    pub task_ref: StateRecordRef,
    pub change_unit_ref: Option<StateRecordRef>,
    pub linked_scope_decision_refs: Vec<StateRecordRef>,
    pub stale_write_ticket_refs: Vec<StateRecordRef>,
    pub blocker_refs: Vec<StateRecordRef>,
    pub state: StateSummary,
    pub next_actions: Vec<NextActionSummary>,
}

/// `volicord.status` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatusRequest {
    pub envelope: ToolEnvelope,
    pub include: StatusInclude,
}

impl MethodOperationCategory for StatusRequest {
    fn method_name(&self) -> MethodName {
        MethodName::Status
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::Read
    }
}

/// MCP-visible `volicord.status` arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpStatusArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub task_id: RequiredNullable<TaskId>,
    #[serde(default)]
    pub detail: StatusDetailLevel,
}

impl StatusDetailLevel {
    /// Expands the MCP-visible detail level into the Core status include matrix.
    pub const fn include(self) -> StatusInclude {
        match self {
            Self::Summary => StatusInclude {
                task: true,
                pending_user_judgments: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: false,
                continuity: false,
            },
            Self::Workflow => StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_ticket: true,
                evidence: true,
                close: true,
                guarantees: true,
                continuity: false,
            },
            Self::Full => StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_ticket: true,
                evidence: true,
                close: true,
                guarantees: true,
                continuity: true,
            },
        }
    }
}

/// Status include flags shown by the method owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatusInclude {
    pub task: bool,
    pub pending_user_judgments: bool,
    pub write_ticket: bool,
    pub evidence: bool,
    pub close: bool,
    pub guarantees: bool,
    pub continuity: bool,
}

/// `volicord.status` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StatusResult {
    pub base: ToolResultBase,
    pub summary_card: SummaryCard,
    pub active_task: Option<StateSummary>,
    pub status_summary: String,
    pub next_actions: Vec<NextActionSummary>,
    pub pending_user_judgments: Vec<StateRecordRef>,
    pub pending_judgment_inbox_items: Vec<JudgmentInboxItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_channel_availability: Option<UserChannelAvailability>,
    pub blocker_refs: Vec<StateRecordRef>,
    pub write_ticket_summary: Option<WriteTicketStateSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_summary: Option<RequiredNullable<EvidenceSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_gate: Option<RequiredNullable<EvidenceGateSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_state: Option<StatusCloseState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_close_basis: Option<RequiredNullable<CurrentCloseBasis>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_acceptance_coverage: Option<Vec<RiskAcceptanceCoverage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_blockers: Option<Vec<CloseReadinessBlocker>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard_health: Option<GuardHealthSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage_summary: Option<CoverageSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guarantee_display: Option<RequiredNullable<GuaranteeDisplay>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuity_summary: Option<Vec<ProjectContinuitySummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_flow: Option<Vec<TaskFlowItem>>,
    pub authority_receipt: Option<AuthorityReceipt>,
}

/// `volicord.prepare_write` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrepareWriteRequest {
    pub envelope: ToolEnvelope,
    pub task_id: RequiredNullable<TaskId>,
    pub change_unit_id: RequiredNullable<ChangeUnitId>,
    pub intended_operation: String,
    pub intended_paths: Vec<String>,
    pub product_file_write_intended: bool,
    pub sensitive_categories: Vec<String>,
    pub baseline_ref: BaselineRef,
}

impl MethodOperationCategory for PrepareWriteRequest {
    fn method_name(&self) -> MethodName {
        MethodName::PrepareWrite
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::AgentWorkflow
    }
}

/// MCP-visible `volicord.prepare_write` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpPrepareWriteArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    #[serde(default)]
    pub task_id: RequiredNullable<TaskId>,
    #[serde(default)]
    pub change_unit_id: RequiredNullable<ChangeUnitId>,
    pub intended_operation: String,
    pub intended_paths: Vec<String>,
    pub product_file_write_intended: bool,
    #[serde(default)]
    pub sensitive_categories: Vec<String>,
    pub baseline_ref: BaselineRef,
}

/// `volicord.prepare_write` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PrepareWriteResult {
    pub base: ToolResultBase,
    pub decision: PrepareWriteDecision,
    pub state: Option<StateSummary>,
    pub write_ticket_id: Option<WriteTicketId>,
    pub write_ticket_ref: Option<StateRecordRef>,
    pub write_ticket: Option<WriteTicket>,
    pub write_ticket_effect: WriteTicketEffect,
    pub allowed_path_patterns: Vec<String>,
    pub denied_path_patterns: Vec<String>,
    pub control_surface: Option<ControlSurfaceSummary>,
    pub active_user_judgment_refs: Vec<StateRecordRef>,
    pub write_decision_reasons: Vec<WriteDecisionReason>,
    pub user_judgment_candidate: Option<UserJudgmentCandidate>,
    pub guarantee_display: Option<GuaranteeDisplay>,
}

/// `volicord.stage_artifact` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StageArtifactRequest {
    pub envelope: ToolEnvelope,
    pub task_id: TaskId,
    pub display_name: String,
    pub content_type: String,
    pub redaction_state: RedactionState,
    pub safe_bytes_or_notice: String,
    pub expected_sha256: RequiredNullable<String>,
    pub expected_size_bytes: RequiredNullable<u64>,
    pub relation_hint: RequiredNullable<String>,
}

impl MethodOperationCategory for StageArtifactRequest {
    fn method_name(&self) -> MethodName {
        MethodName::StageArtifact
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::AgentWorkflow
    }
}

/// MCP-visible `volicord.stage_artifact` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpStageArtifactArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub task_id: TaskId,
    pub display_name: String,
    pub content_type: String,
    pub redaction_state: RedactionState,
    pub safe_bytes_or_notice: String,
    #[serde(default)]
    pub expected_sha256: RequiredNullable<String>,
    #[serde(default)]
    pub expected_size_bytes: RequiredNullable<u64>,
    #[serde(default)]
    pub relation_hint: RequiredNullable<String>,
}

/// `volicord.stage_artifact` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StageArtifactResult {
    pub base: ToolResultBase,
    pub evidence_state: EvidenceDisplayState,
    pub staged_artifact_handle: StagedArtifactHandle,
    pub expires_at: UtcTimestamp,
}

/// `volicord.record_run` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordRunRequest {
    pub envelope: ToolEnvelope,
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub kind: RunKind,
    pub run_id: RequiredNullable<RunId>,
    pub baseline_ref: BaselineRef,
    pub write_ticket_id: RequiredNullable<WriteTicketId>,
    pub summary: String,
    pub observed_changes: ObservedChanges,
    pub artifact_inputs: Vec<ArtifactInput>,
    pub evidence_updates: Vec<EvidenceCoverageUpdate>,
    pub evidence_observations: Vec<EvidenceObservationInput>,
    pub close_assessment: RequiredNullable<CloseAssessmentInput>,
}

impl MethodOperationCategory for RecordRunRequest {
    fn method_name(&self) -> MethodName {
        MethodName::RecordRun
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::AgentWorkflow
    }
}

/// MCP-visible `volicord.record_run` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRecordRunArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub kind: RunKind,
    #[serde(default)]
    pub run_id: RequiredNullable<RunId>,
    pub baseline_ref: BaselineRef,
    #[serde(default)]
    pub write_ticket_id: RequiredNullable<WriteTicketId>,
    pub summary: String,
    pub observed_changes: ObservedChanges,
    #[serde(default)]
    pub artifact_inputs: Vec<ArtifactInput>,
    #[serde(default)]
    pub evidence_updates: Vec<McpEvidenceCoverageUpdate>,
    #[serde(default)]
    pub evidence_observations: Vec<McpEvidenceObservationInput>,
    #[serde(default)]
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

impl From<McpEvidenceCoverageUpdate> for EvidenceCoverageUpdate {
    fn from(value: McpEvidenceCoverageUpdate) -> Self {
        Self {
            target: value.target,
            coverage_state: value.coverage_state,
            provenance: value.provenance,
            supporting_run_refs: value.supporting_run_refs,
            observation_refs: value.observation_refs,
            supporting_artifact_refs: value.supporting_artifact_refs,
            gap_refs: value.gap_refs,
        }
    }
}

/// MCP-visible evidence observation input with omission-equivalent null and collection defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpEvidenceObservationInput {
    pub target: EvidenceTarget,
    pub source_kind: EvidenceSourceKind,
    pub assurance_level: EvidenceAssuranceLevel,
    #[serde(default)]
    pub observed_by_actor_source: RequiredNullable<ActorSource>,
    #[serde(default)]
    pub tool_name: RequiredNullable<String>,
    #[serde(default)]
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

impl From<McpEvidenceObservationInput> for EvidenceObservationInput {
    fn from(value: McpEvidenceObservationInput) -> Self {
        Self {
            target: value.target,
            source_kind: value.source_kind,
            assurance_level: value.assurance_level,
            observed_by_actor_source: value.observed_by_actor_source,
            tool_name: value.tool_name,
            tool_invocation_id: value.tool_invocation_id,
            tool_metadata: value.tool_metadata,
            input_refs: value.input_refs,
            source_refs: value.source_refs,
            output_artifact_refs: value.output_artifact_refs,
            limitations: value.limitations,
            observed_at: value.observed_at,
        }
    }
}

/// `volicord.record_run` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RecordRunResult {
    pub base: ToolResultBase,
    pub run_summary: RunSummary,
    pub registered_artifacts: Vec<ArtifactRef>,
    pub evidence_summary: Option<EvidenceSummary>,
    pub evidence_observations: Vec<EvidenceObservation>,
    pub current_close_basis: Option<CurrentCloseBasis>,
    pub blocker_refs: Vec<StateRecordRef>,
    pub state: StateSummary,
}

/// `volicord.request_user_judgment` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestUserJudgmentRequest {
    pub envelope: ToolEnvelope,
    pub task_id: TaskId,
    pub change_unit_id: RequiredNullable<ChangeUnitId>,
    #[serde(default)]
    pub sensitive_action_scope: RequiredNullable<SensitiveActionScope>,
    pub judgment_kind: JudgmentKind,
    pub presentation: JudgmentPresentation,
    pub question: String,
    #[serde(default)]
    pub options: RequiredNullable<Vec<UserJudgmentOptionInput>>,
    pub context: UserJudgmentContext,
    pub affected_refs: Vec<StateRecordRef>,
    pub required_for: Vec<JudgmentRequiredFor>,
    pub expires_at: RequiredNullable<UtcTimestamp>,
}

impl MethodOperationCategory for RequestUserJudgmentRequest {
    fn method_name(&self) -> MethodName {
        MethodName::RequestUserJudgment
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::AgentWorkflow
    }
}

/// MCP-visible `volicord.request_user_judgment` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRequestUserJudgmentArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub task_id: TaskId,
    #[serde(default)]
    pub change_unit_id: RequiredNullable<ChangeUnitId>,
    #[serde(default)]
    pub sensitive_action_scope: RequiredNullable<SensitiveActionScope>,
    pub judgment_kind: JudgmentKind,
    pub presentation: JudgmentPresentation,
    pub question: String,
    #[serde(default)]
    pub options: RequiredNullable<Vec<UserJudgmentOptionInput>>,
    pub context: UserJudgmentContext,
    #[serde(default)]
    pub affected_refs: Vec<StateRecordRef>,
    pub required_for: Vec<JudgmentRequiredFor>,
    #[serde(default)]
    pub expires_at: RequiredNullable<UtcTimestamp>,
}

/// `volicord.request_user_judgment` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RequestUserJudgmentResult {
    pub base: ToolResultBase,
    pub user_judgment_ref: StateRecordRef,
    pub user_judgment: UserJudgment,
    pub inbox_item: JudgmentInboxItem,
    pub blocker_refs: Vec<StateRecordRef>,
    pub state: StateSummary,
}

/// `volicord.record_user_judgment` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordUserJudgmentRequest {
    pub envelope: ToolEnvelope,
    pub user_judgment_id: UserJudgmentId,
    pub judgment_kind: JudgmentKind,
    pub selected_option_id: UserJudgmentOptionId,
    pub answer: RecordUserJudgmentPayload,
    pub rationale: JudgmentRationale,
    pub note: RequiredNullable<String>,
    pub accepted_risks: Vec<AcceptedRiskInput>,
}

impl MethodOperationCategory for RecordUserJudgmentRequest {
    fn method_name(&self) -> MethodName {
        MethodName::RecordUserJudgment
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::UserOnly
    }
}

/// `volicord.record_user_judgment` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RecordUserJudgmentResult {
    pub base: ToolResultBase,
    pub user_judgment_ref: StateRecordRef,
    pub user_judgment: UserJudgment,
    pub updated_refs: Vec<StateRecordRef>,
    pub state: StateSummary,
    pub next_actions: Vec<NextActionSummary>,
}

/// `volicord.record_user_observation` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordUserObservationRequest {
    pub envelope: ToolEnvelope,
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub target: EvidenceTarget,
    pub relevance_status: EvidenceRelevanceStatus,
    pub artifact_ids: Vec<ArtifactId>,
    pub summary: String,
    pub observed_at: UtcTimestamp,
}

impl MethodOperationCategory for RecordUserObservationRequest {
    fn method_name(&self) -> MethodName {
        MethodName::RecordUserObservation
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::UserOnly
    }
}

/// `volicord.record_user_observation` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RecordUserObservationResult {
    pub base: ToolResultBase,
    pub user_observation_ref: StateRecordRef,
    pub user_observation: UserEvidenceObservation,
}

/// `volicord.reconcile_changes` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReconcileChangesRequest {
    pub envelope: ToolEnvelope,
    pub task_id: TaskId,
    #[serde(default)]
    pub resolution_requests: Vec<UnrecordedChangeResolutionRequest>,
}

impl MethodOperationCategory for ReconcileChangesRequest {
    fn method_name(&self) -> MethodName {
        MethodName::ReconcileChanges
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::AgentWorkflow
    }
}

/// MCP-visible `volicord.reconcile_changes` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpReconcileChangesArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub task_id: TaskId,
    #[serde(default)]
    pub resolution_requests: Vec<UnrecordedChangeResolutionRequest>,
}

/// Optional caller-supplied reconciliation request for a specific finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UnrecordedChangeResolutionRequest {
    pub unrecorded_change_id: UnrecordedChangeId,
    pub basis: UnrecordedChangeResolutionBasis,
    #[serde(default)]
    pub user_judgment_id: RequiredNullable<UserJudgmentId>,
}

/// `volicord.reconcile_changes` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ReconcileChangesResult {
    pub base: ToolResultBase,
    pub summary_card: SummaryCard,
    pub task_ref: StateRecordRef,
    pub unresolved_changes: Vec<UnrecordedChangeFinding>,
    pub resolved_changes: Vec<UnrecordedChangeResolutionSummary>,
    pub pending_user_judgment_refs: Vec<StateRecordRef>,
    pub rejected_resolution_requests: Vec<UnrecordedChangeRejection>,
    pub state: StateSummary,
    pub close_blockers: Vec<CloseReadinessBlocker>,
    pub guard_health: Option<GuardHealthSummary>,
    pub next_actions: Vec<NextActionSummary>,
}

/// Rejected requested reconciliation item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UnrecordedChangeRejection {
    pub unrecorded_change_id: UnrecordedChangeId,
    pub basis: UnrecordedChangeResolutionBasis,
    pub code: String,
    pub message: String,
}

/// `volicord.check_close` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckCloseRequest {
    pub envelope: ToolEnvelope,
    pub task_id: TaskId,
}

impl MethodOperationCategory for CheckCloseRequest {
    fn method_name(&self) -> MethodName {
        MethodName::CheckClose
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::Read
    }
}

/// `volicord.close_task` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CloseTaskRequest {
    pub envelope: ToolEnvelope,
    pub task_id: TaskId,
    pub intent: CloseMutationIntent,
    pub close_reason: RequiredNullable<CloseReason>,
    pub superseding_task_id: RequiredNullable<TaskId>,
    pub user_note: RequiredNullable<String>,
}

impl MethodOperationCategory for CloseTaskRequest {
    fn method_name(&self) -> MethodName {
        MethodName::CloseTask
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::AgentWorkflow
    }
}

/// MCP-visible read-only `volicord.check_close` arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpCheckCloseArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
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
    pub task_id: TaskId,
    pub intent: CloseMutationIntent,
    #[serde(default)]
    pub close_reason: RequiredNullable<CloseReason>,
    #[serde(default)]
    pub superseding_task_id: RequiredNullable<TaskId>,
    #[serde(default)]
    pub user_note: RequiredNullable<String>,
}

/// `volicord.close_task` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CloseTaskResult {
    pub base: ToolResultBase,
    pub summary_card: SummaryCard,
    pub close_state: CloseState,
    pub current_close_basis: Option<CurrentCloseBasis>,
    pub risk_acceptance_coverage: Vec<RiskAcceptanceCoverage>,
    pub continuity_summary: Vec<ProjectContinuitySummary>,
    pub state: StateSummary,
    pub blockers: Vec<CloseReadinessBlocker>,
    pub pending_judgment_inbox_items: Vec<JudgmentInboxItem>,
    pub guard_health: Option<GuardHealthSummary>,
    pub coverage_summary: Option<CoverageSummary>,
    pub evidence_summary: Option<EvidenceSummary>,
    pub evidence_gate: EvidenceGateSummary,
    pub artifact_refs: Vec<ArtifactRef>,
}

/// Returns the generated JSON Schema for one public method request shape.
pub fn public_request_schema(method_name: &str) -> Option<Value> {
    match method_name {
        "volicord.intake" => Some(request_schema::<IntakeRequest>()),
        "volicord.update_scope" => Some(request_schema::<UpdateScopeRequest>()),
        "volicord.status" => Some(request_schema::<StatusRequest>()),
        "volicord.check_close" => Some(request_schema::<CheckCloseRequest>()),
        "volicord.prepare_write" => Some(request_schema::<PrepareWriteRequest>()),
        "volicord.stage_artifact" => Some(request_schema::<StageArtifactRequest>()),
        "volicord.record_run" => Some(request_schema::<RecordRunRequest>()),
        "volicord.request_user_judgment" => Some(request_schema::<RequestUserJudgmentRequest>()),
        "volicord.record_user_judgment" => Some(request_schema::<RecordUserJudgmentRequest>()),
        "volicord.record_user_observation" => {
            Some(request_schema::<RecordUserObservationRequest>())
        }
        "volicord.reconcile_changes" => Some(request_schema::<ReconcileChangesRequest>()),
        "volicord.close_task" => Some(request_schema::<CloseTaskRequest>()),
        _ => None,
    }
}

/// Returns the generated JSON Schema for one public method response shape.
///
/// Public responses are object values even when their generated schema uses
/// branch combinators for result, rejected, and dry-run variants.
pub fn public_response_schema(method_name: &str) -> Option<Value> {
    match method_name {
        "volicord.intake" => Some(response_schema::<IntakeResponse>()),
        "volicord.update_scope" => Some(response_schema::<UpdateScopeResponse>()),
        "volicord.status" => Some(response_schema::<StatusResponse>()),
        "volicord.check_close" => Some(response_schema::<CheckCloseResponse>()),
        "volicord.prepare_write" => Some(response_schema::<PrepareWriteResponse>()),
        "volicord.stage_artifact" => Some(response_schema::<StageArtifactResponse>()),
        "volicord.record_run" => Some(response_schema::<RecordRunResponse>()),
        "volicord.request_user_judgment" => Some(response_schema::<RequestUserJudgmentResponse>()),
        "volicord.record_user_judgment" => Some(response_schema::<RecordUserJudgmentResponse>()),
        "volicord.record_user_observation" => {
            Some(response_schema::<RecordUserObservationResponse>())
        }
        "volicord.reconcile_changes" => Some(response_schema::<ReconcileChangesResponse>()),
        "volicord.close_task" => Some(response_schema::<CloseTaskResponse>()),
        _ => None,
    }
}

/// Returns the generated JSON Schema for one MCP-visible tool argument shape.
pub fn mcp_request_schema(tool_name: &str) -> Option<Value> {
    match tool_name {
        "volicord.intake" => Some(request_schema::<McpIntakeArguments>()),
        "volicord.update_scope" => Some(request_schema::<McpUpdateScopeArguments>()),
        "volicord.status" => Some(request_schema::<McpStatusArguments>()),
        "volicord.prepare_write" => Some(request_schema::<McpPrepareWriteArguments>()),
        "volicord.stage_artifact" => Some(request_schema::<McpStageArtifactArguments>()),
        "volicord.record_run" => Some(request_schema::<McpRecordRunArguments>()),
        "volicord.request_user_judgment" => {
            Some(request_schema::<McpRequestUserJudgmentArguments>())
        }
        "volicord.reconcile_changes" => Some(request_schema::<McpReconcileChangesArguments>()),
        "volicord.check_close" => Some(request_schema::<McpCheckCloseArguments>()),
        "volicord.close_task" => Some(request_schema::<McpCloseTaskArguments>()),
        _ => None,
    }
}

/// Returns the generated JSON Schema for one MCP-visible public method result.
pub fn mcp_response_schema(tool_name: &str) -> Option<Value> {
    match tool_name {
        "volicord.request_user_judgment" => Some(response_schema::<
            McpMutationStructuredContent<McpRequestUserJudgmentResponse>,
        >()),
        "volicord.intake" => Some(response_schema::<
            McpMutationStructuredContent<IntakeResponse>,
        >()),
        "volicord.update_scope" => Some(response_schema::<
            McpMutationStructuredContent<UpdateScopeResponse>,
        >()),
        "volicord.status" => Some(response_schema::<McpToolStructuredContent<StatusResponse>>()),
        "volicord.prepare_write" => Some(response_schema::<
            McpMutationStructuredContent<PrepareWriteResponse>,
        >()),
        "volicord.stage_artifact" => Some(response_schema::<
            McpMutationStructuredContent<StageArtifactResponse>,
        >()),
        "volicord.record_run" => Some(response_schema::<
            McpMutationStructuredContent<RecordRunResponse>,
        >()),
        "volicord.reconcile_changes" => Some(response_schema::<
            McpMutationStructuredContent<ReconcileChangesResponse>,
        >()),
        "volicord.check_close" => Some(response_schema::<
            McpToolStructuredContent<CheckCloseResponse>,
        >()),
        "volicord.close_task" => Some(response_schema::<
            McpMutationStructuredContent<CloseTaskResponse>,
        >()),
        _ => None,
    }
}

fn request_schema<T: JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).expect("request schema should serialize")
}

fn response_schema<T: JsonSchema>() -> Value {
    let mut schema =
        serde_json::to_value(schema_for!(T)).expect("response schema should serialize");
    schema
        .as_object_mut()
        .expect("generated response schema should be an object")
        .insert("type".to_owned(), Value::String("object".to_owned()));
    schema
}
