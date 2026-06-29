use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ids::{
    BaselineRef, ChangeUnitId, RunId, TaskId, UserJudgmentId, UserJudgmentOptionId, WriteCheckId,
};
use crate::schema::{
    AcceptedRiskInput, ArtifactInput, ArtifactRef, ChangeUnitEffectContract, CloseAssessmentInput,
    CloseReadinessBlocker, CurrentCloseBasis, EvidenceCoverageItem, EvidenceObservation,
    EvidenceObservationInput, EvidenceSummary, GuaranteeDisplay, JsonObject, JudgmentRationale,
    NextActionSummary, ObservedChanges, ProjectContinuitySummary, RecordUserJudgmentPayload,
    RequiredNullable, RiskAcceptanceCoverage, RunSummary, SensitiveActionScope,
    StagedArtifactHandle, StateRecordRef, StateSummary, ToolEnvelope, ToolResponse, ToolResultBase,
    UserJudgment, UserJudgmentCandidate, UserJudgmentContext, UserJudgmentOptionInput,
    WriteCheckStateSummary, WriteCheckSummary, WriteDecisionReason,
};
use crate::values::{
    ChangeUnitOperation, CloseIntent, CloseMutationIntent, CloseReason, CloseState, JudgmentKind,
    JudgmentPresentation, JudgmentRequiredFor, MethodName, OperationCategory, PrepareWriteDecision,
    RedactionState, RequestedMode, ResumePolicy, RunKind, StatusCloseState, StatusDetailLevel,
    UtcTimestamp, WriteCheckEffect,
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

/// Response branch type for `volicord.close_task`.
pub type CloseTaskResponse = ToolResponse<CloseTaskResult>;

/// `volicord.intake` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IntakeRequest {
    pub envelope: ToolEnvelope,
    pub plain_language_request: String,
    pub requested_mode: RequestedMode,
    pub resume_policy: ResumePolicy,
    pub initial_scope: InitialScope,
    pub initial_context_refs: Vec<StateRecordRef>,
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
    pub plain_language_request: String,
    pub requested_mode: RequestedMode,
    pub resume_policy: ResumePolicy,
    pub initial_scope: InitialScope,
    pub initial_context_refs: Vec<StateRecordRef>,
}

/// Intake initial scope object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitialScope {
    pub boundary: String,
    pub non_goals: Vec<String>,
    pub acceptance_criteria: Vec<String>,
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
    pub acceptance_criteria: RequiredNullable<Vec<String>>,
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
    pub task_id: TaskId,
    pub goal_summary: RequiredNullable<String>,
    pub scope_update: RequiredNullable<ScopeUpdate>,
    pub scope_boundary: RequiredNullable<String>,
    pub non_goals: RequiredNullable<Vec<String>>,
    pub acceptance_criteria: RequiredNullable<Vec<String>>,
    pub autonomy_boundary: RequiredNullable<String>,
    pub baseline_ref: RequiredNullable<BaselineRef>,
    pub change_unit: ChangeUnitUpdate,
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
    pub stale_write_check_refs: Vec<StateRecordRef>,
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
                write_check: false,
                evidence: false,
                close: false,
                guarantees: false,
                continuity: false,
            },
            Self::Workflow => StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_check: true,
                evidence: true,
                close: true,
                guarantees: true,
                continuity: false,
            },
            Self::Full => StatusInclude {
                task: true,
                pending_user_judgments: true,
                write_check: true,
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
    pub write_check: bool,
    pub evidence: bool,
    pub close: bool,
    pub guarantees: bool,
    pub continuity: bool,
}

/// `volicord.status` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StatusResult {
    pub base: ToolResultBase,
    pub active_task: Option<StateSummary>,
    pub status_summary: String,
    pub next_actions: Vec<NextActionSummary>,
    pub pending_user_judgments: Vec<StateRecordRef>,
    pub blocker_refs: Vec<StateRecordRef>,
    pub write_check_summary: Option<WriteCheckStateSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_summary: Option<RequiredNullable<EvidenceSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_state: Option<StatusCloseState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_close_basis: Option<RequiredNullable<CurrentCloseBasis>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_acceptance_coverage: Option<Vec<RiskAcceptanceCoverage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_blockers: Option<Vec<CloseReadinessBlocker>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guarantee_display: Option<RequiredNullable<GuaranteeDisplay>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuity_summary: Option<Vec<ProjectContinuitySummary>>,
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
    pub task_id: RequiredNullable<TaskId>,
    pub change_unit_id: RequiredNullable<ChangeUnitId>,
    pub intended_operation: String,
    pub intended_paths: Vec<String>,
    pub product_file_write_intended: bool,
    pub sensitive_categories: Vec<String>,
    pub baseline_ref: BaselineRef,
}

/// `volicord.prepare_write` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PrepareWriteResult {
    pub base: ToolResultBase,
    pub decision: PrepareWriteDecision,
    pub state: Option<StateSummary>,
    pub write_check_ref: Option<StateRecordRef>,
    pub write_check: Option<WriteCheckSummary>,
    pub write_check_effect: WriteCheckEffect,
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
    pub task_id: TaskId,
    pub display_name: String,
    pub content_type: String,
    pub redaction_state: RedactionState,
    pub safe_bytes_or_notice: String,
    pub expected_sha256: RequiredNullable<String>,
    pub expected_size_bytes: RequiredNullable<u64>,
    pub relation_hint: RequiredNullable<String>,
}

/// `volicord.stage_artifact` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StageArtifactResult {
    pub base: ToolResultBase,
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
    pub write_check_id: RequiredNullable<WriteCheckId>,
    pub summary: String,
    pub observed_changes: ObservedChanges,
    pub artifact_inputs: Vec<ArtifactInput>,
    pub evidence_updates: Vec<EvidenceCoverageItem>,
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
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub kind: RunKind,
    pub run_id: RequiredNullable<RunId>,
    pub baseline_ref: BaselineRef,
    pub write_check_id: RequiredNullable<WriteCheckId>,
    pub summary: String,
    pub observed_changes: ObservedChanges,
    pub artifact_inputs: Vec<ArtifactInput>,
    pub evidence_updates: Vec<EvidenceCoverageItem>,
    pub evidence_observations: Vec<EvidenceObservationInput>,
    pub close_assessment: RequiredNullable<CloseAssessmentInput>,
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

/// `volicord.request_user_judgment` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RequestUserJudgmentResult {
    pub base: ToolResultBase,
    pub user_judgment_ref: StateRecordRef,
    pub user_judgment: UserJudgment,
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

/// `volicord.close_task` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CloseTaskRequest {
    pub envelope: ToolEnvelope,
    pub task_id: TaskId,
    pub intent: CloseIntent,
    pub close_reason: RequiredNullable<CloseReason>,
    pub superseding_task_id: RequiredNullable<TaskId>,
    pub user_note: RequiredNullable<String>,
}

impl MethodOperationCategory for CloseTaskRequest {
    fn method_name(&self) -> MethodName {
        MethodName::CloseTask
    }

    fn operation_category(&self) -> OperationCategory {
        match self.intent {
            CloseIntent::Check => OperationCategory::Read,
            CloseIntent::Complete | CloseIntent::Cancel | CloseIntent::Supersede => {
                OperationCategory::AgentWorkflow
            }
        }
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
    pub close_state: CloseState,
    pub current_close_basis: Option<CurrentCloseBasis>,
    pub risk_acceptance_coverage: Vec<RiskAcceptanceCoverage>,
    pub continuity_summary: Vec<ProjectContinuitySummary>,
    pub state: StateSummary,
    pub blockers: Vec<CloseReadinessBlocker>,
    pub evidence_summary: Option<EvidenceSummary>,
    pub artifact_refs: Vec<ArtifactRef>,
}

/// Returns the generated JSON Schema for one public method request shape.
pub fn public_request_schema(method_name: &str) -> Option<Value> {
    match method_name {
        "volicord.intake" => Some(request_schema::<IntakeRequest>()),
        "volicord.update_scope" => Some(request_schema::<UpdateScopeRequest>()),
        "volicord.status" => Some(request_schema::<StatusRequest>()),
        "volicord.prepare_write" => Some(request_schema::<PrepareWriteRequest>()),
        "volicord.stage_artifact" => Some(request_schema::<StageArtifactRequest>()),
        "volicord.record_run" => Some(request_schema::<RecordRunRequest>()),
        "volicord.request_user_judgment" => Some(request_schema::<RequestUserJudgmentRequest>()),
        "volicord.record_user_judgment" => Some(request_schema::<RecordUserJudgmentRequest>()),
        "volicord.close_task" => Some(request_schema::<CloseTaskRequest>()),
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
        "volicord.check_close" => Some(request_schema::<McpCheckCloseArguments>()),
        "volicord.close_task" => Some(request_schema::<McpCloseTaskArguments>()),
        _ => None,
    }
}

fn request_schema<T: JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).expect("request schema should serialize")
}
