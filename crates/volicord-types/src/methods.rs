use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ids::{
    BaselineRef, ChangeUnitId, RunId, TaskId, UserJudgmentId, UserJudgmentOptionId,
    WriteAuthorizationId,
};
use crate::schema::{
    AcceptedRiskInput, ArtifactInput, ArtifactRef, CloseAssessmentInput, CloseReadinessBlocker,
    CurrentCloseBasis, EvidenceCoverageItem, EvidenceSummary, GuaranteeDisplay, JsonObject,
    NextActionSummary, ObservedChanges, RecordUserJudgmentPayload, RequiredNullable,
    RiskAcceptanceCoverage, RunSummary, SensitiveActionScope, StagedArtifactHandle, StateRecordRef,
    StateSummary, ToolEnvelope, ToolResponse, ToolResultBase, UserJudgment, UserJudgmentCandidate,
    UserJudgmentContext, UserJudgmentOptionInput, WriteAuthoritySummary, WriteAuthorizationSummary,
    WriteDecisionReason,
};
use crate::values::{
    AccessClass, AuthorizationEffect, ChangeUnitOperation, CloseIntent, CloseReason, CloseState,
    JudgmentKind, JudgmentPresentation, JudgmentRequiredFor, MethodName, PrepareWriteDecision,
    RedactionState, RequestedMode, ResumePolicy, RunKind, StatusCloseState, UtcTimestamp,
};

/// Shared typed mapping from a public request to its request-level access class.
pub trait MethodAccessClass {
    /// Returns the public method name for this typed request.
    fn method_name(&self) -> MethodName;

    /// Returns the access class requested by this typed request.
    fn requested_access_class(&self) -> AccessClass;
}

/// Response branch type for `harness.intake`.
pub type IntakeResponse = ToolResponse<IntakeResult>;

/// Response branch type for `harness.update_scope`.
pub type UpdateScopeResponse = ToolResponse<UpdateScopeResult>;

/// Response branch type for `harness.status`.
pub type StatusResponse = ToolResponse<StatusResult>;

/// Response branch type for `harness.prepare_write`.
pub type PrepareWriteResponse = ToolResponse<PrepareWriteResult>;

/// Response branch type for `harness.stage_artifact`.
pub type StageArtifactResponse = ToolResponse<StageArtifactResult>;

/// Response branch type for `harness.record_run`.
pub type RecordRunResponse = ToolResponse<RecordRunResult>;

/// Response branch type for `harness.request_user_judgment`.
pub type RequestUserJudgmentResponse = ToolResponse<RequestUserJudgmentResult>;

/// Response branch type for `harness.record_user_judgment`.
pub type RecordUserJudgmentResponse = ToolResponse<RecordUserJudgmentResult>;

/// Response branch type for `harness.close_task`.
pub type CloseTaskResponse = ToolResponse<CloseTaskResult>;

/// `harness.intake` request params.
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

impl MethodAccessClass for IntakeRequest {
    fn method_name(&self) -> MethodName {
        MethodName::Intake
    }

    fn requested_access_class(&self) -> AccessClass {
        AccessClass::CoreMutation
    }
}

/// Intake initial scope object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitialScope {
    pub boundary: String,
    pub non_goals: Vec<String>,
    pub acceptance_criteria: Vec<String>,
}

/// `harness.intake` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct IntakeResult {
    pub base: ToolResultBase,
    pub task_ref: StateRecordRef,
    pub change_unit_ref: Option<StateRecordRef>,
    pub state: StateSummary,
    pub next_actions: Vec<NextActionSummary>,
}

/// `harness.update_scope` request params.
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

impl MethodAccessClass for UpdateScopeRequest {
    fn method_name(&self) -> MethodName {
        MethodName::UpdateScope
    }

    fn requested_access_class(&self) -> AccessClass {
        AccessClass::CoreMutation
    }
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
    #[serde(flatten)]
    pub fields: JsonObject,
}

/// `harness.update_scope` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateScopeResult {
    pub base: ToolResultBase,
    pub task_ref: StateRecordRef,
    pub change_unit_ref: Option<StateRecordRef>,
    pub linked_scope_decision_refs: Vec<StateRecordRef>,
    pub stale_write_authorization_refs: Vec<StateRecordRef>,
    pub blocker_refs: Vec<StateRecordRef>,
    pub state: StateSummary,
    pub next_actions: Vec<NextActionSummary>,
}

/// `harness.status` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatusRequest {
    pub envelope: ToolEnvelope,
    pub include: StatusInclude,
}

impl MethodAccessClass for StatusRequest {
    fn method_name(&self) -> MethodName {
        MethodName::Status
    }

    fn requested_access_class(&self) -> AccessClass {
        AccessClass::ReadStatus
    }
}

/// Status include flags shown by the method owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatusInclude {
    pub task: bool,
    pub pending_user_judgments: bool,
    pub write_authority: bool,
    pub evidence: bool,
    pub close: bool,
    pub guarantees: bool,
}

/// `harness.status` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StatusResult {
    pub base: ToolResultBase,
    pub active_task: Option<StateSummary>,
    pub status_summary: String,
    pub next_actions: Vec<NextActionSummary>,
    pub pending_user_judgments: Vec<StateRecordRef>,
    pub blocker_refs: Vec<StateRecordRef>,
    pub write_authority_summary: Option<WriteAuthoritySummary>,
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
}

/// `harness.prepare_write` request params.
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

impl MethodAccessClass for PrepareWriteRequest {
    fn method_name(&self) -> MethodName {
        MethodName::PrepareWrite
    }

    fn requested_access_class(&self) -> AccessClass {
        AccessClass::WriteAuthorization
    }
}

/// `harness.prepare_write` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PrepareWriteResult {
    pub base: ToolResultBase,
    pub decision: PrepareWriteDecision,
    pub state: Option<StateSummary>,
    pub write_authorization_ref: Option<StateRecordRef>,
    pub write_authorization: Option<WriteAuthorizationSummary>,
    pub authorization_effect: AuthorizationEffect,
    pub active_user_judgment_refs: Vec<StateRecordRef>,
    pub write_decision_reasons: Vec<WriteDecisionReason>,
    pub user_judgment_candidate: Option<UserJudgmentCandidate>,
    pub guarantee_display: Option<GuaranteeDisplay>,
}

/// `harness.stage_artifact` request params.
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

impl MethodAccessClass for StageArtifactRequest {
    fn method_name(&self) -> MethodName {
        MethodName::StageArtifact
    }

    fn requested_access_class(&self) -> AccessClass {
        AccessClass::ArtifactRegistration
    }
}

/// `harness.stage_artifact` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StageArtifactResult {
    pub base: ToolResultBase,
    pub staged_artifact_handle: StagedArtifactHandle,
    pub expires_at: UtcTimestamp,
}

/// `harness.record_run` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordRunRequest {
    pub envelope: ToolEnvelope,
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub kind: RunKind,
    pub run_id: RequiredNullable<RunId>,
    pub baseline_ref: BaselineRef,
    pub write_authorization_id: RequiredNullable<WriteAuthorizationId>,
    pub summary: String,
    pub observed_changes: ObservedChanges,
    pub artifact_inputs: Vec<ArtifactInput>,
    pub evidence_updates: Vec<EvidenceCoverageItem>,
    pub close_assessment: RequiredNullable<CloseAssessmentInput>,
}

impl MethodAccessClass for RecordRunRequest {
    fn method_name(&self) -> MethodName {
        MethodName::RecordRun
    }

    fn requested_access_class(&self) -> AccessClass {
        AccessClass::RunRecording
    }
}

/// `harness.record_run` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RecordRunResult {
    pub base: ToolResultBase,
    pub run_summary: RunSummary,
    pub registered_artifacts: Vec<ArtifactRef>,
    pub evidence_summary: Option<EvidenceSummary>,
    pub current_close_basis: Option<CurrentCloseBasis>,
    pub blocker_refs: Vec<StateRecordRef>,
    pub state: StateSummary,
}

/// `harness.request_user_judgment` request params.
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

impl MethodAccessClass for RequestUserJudgmentRequest {
    fn method_name(&self) -> MethodName {
        MethodName::RequestUserJudgment
    }

    fn requested_access_class(&self) -> AccessClass {
        AccessClass::CoreMutation
    }
}

/// `harness.request_user_judgment` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RequestUserJudgmentResult {
    pub base: ToolResultBase,
    pub user_judgment_ref: StateRecordRef,
    pub user_judgment: UserJudgment,
    pub blocker_refs: Vec<StateRecordRef>,
    pub state: StateSummary,
}

/// `harness.record_user_judgment` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordUserJudgmentRequest {
    pub envelope: ToolEnvelope,
    pub user_judgment_id: UserJudgmentId,
    pub judgment_kind: JudgmentKind,
    pub selected_option_id: UserJudgmentOptionId,
    pub answer: RecordUserJudgmentPayload,
    pub note: RequiredNullable<String>,
    pub accepted_risks: Vec<AcceptedRiskInput>,
}

impl MethodAccessClass for RecordUserJudgmentRequest {
    fn method_name(&self) -> MethodName {
        MethodName::RecordUserJudgment
    }

    fn requested_access_class(&self) -> AccessClass {
        AccessClass::CoreMutation
    }
}

/// `harness.record_user_judgment` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RecordUserJudgmentResult {
    pub base: ToolResultBase,
    pub user_judgment_ref: StateRecordRef,
    pub user_judgment: UserJudgment,
    pub updated_refs: Vec<StateRecordRef>,
    pub state: StateSummary,
    pub next_actions: Vec<NextActionSummary>,
}

/// `harness.close_task` request params.
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

impl MethodAccessClass for CloseTaskRequest {
    fn method_name(&self) -> MethodName {
        MethodName::CloseTask
    }

    fn requested_access_class(&self) -> AccessClass {
        match self.intent {
            CloseIntent::Check => AccessClass::ReadStatus,
            CloseIntent::Complete | CloseIntent::Cancel | CloseIntent::Supersede => {
                AccessClass::CoreMutation
            }
        }
    }
}

/// `harness.close_task` method result branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CloseTaskResult {
    pub base: ToolResultBase,
    pub close_state: CloseState,
    pub current_close_basis: Option<CurrentCloseBasis>,
    pub risk_acceptance_coverage: Vec<RiskAcceptanceCoverage>,
    pub state: StateSummary,
    pub blockers: Vec<CloseReadinessBlocker>,
    pub evidence_summary: Option<EvidenceSummary>,
    pub artifact_refs: Vec<ArtifactRef>,
}

/// Returns the generated JSON Schema for one public method request shape.
pub fn public_request_schema(method_name: &str) -> Option<Value> {
    match method_name {
        "harness.intake" => Some(request_schema::<IntakeRequest>()),
        "harness.update_scope" => Some(request_schema::<UpdateScopeRequest>()),
        "harness.status" => Some(request_schema::<StatusRequest>()),
        "harness.prepare_write" => Some(request_schema::<PrepareWriteRequest>()),
        "harness.stage_artifact" => Some(request_schema::<StageArtifactRequest>()),
        "harness.record_run" => Some(request_schema::<RecordRunRequest>()),
        "harness.request_user_judgment" => Some(request_schema::<RequestUserJudgmentRequest>()),
        "harness.record_user_judgment" => Some(request_schema::<RecordUserJudgmentRequest>()),
        "harness.close_task" => Some(request_schema::<CloseTaskRequest>()),
        _ => None,
    }
}

fn request_schema<T: JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).expect("request schema should serialize")
}
