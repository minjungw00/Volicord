use schemars::{schema_for, JsonSchema};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

use crate::ids::{
    BaselineRef, ChangeUnitId, IdempotencyKey, ProjectId, RunId, TaskId, UnrecordedChangeId,
    UserActionRequestId, UserActionResolutionId, WriteTicketId,
};
use crate::schema::{
    AcceptanceCriterionInput, AcceptanceCriterionReplacement, AgentSafeUserActionRequestSummary,
    ArtifactInput, ArtifactRef, AuthorityReceipt, ChangeUnitEffectContract, CloseAssessmentInput,
    CloseReadinessBlocker, ContinuityPageRequest, CurrentCloseBasis, EvidenceCaptureIntent,
    EvidenceCaptureSpec, EvidenceCoverageUpdate, EvidenceGateSummary, EvidenceObservation,
    EvidenceObservationInput, EvidenceProducer, EvidenceSummary, EvidenceTarget, GuaranteeDisplay,
    JsonObject, NextActionSummary, ObservedChanges, PreviewableToolResponse, ProjectContinuityPage,
    ProjectContinuitySummary, RequiredNullable, RiskAcceptanceCoverage, RunSummary, SourceRef,
    StagedArtifactHandle, StateRecordRef, StateSummary, SummaryCard, TaskFlowItem,
    TaskLineageInput, ToolEnvelope, ToolResultBase, ToolResultOrRejected, UnrecordedChangeFinding,
    UnrecordedChangeResolutionSummary, UserActionDraft, UserActionRequest, UserActionResolution,
    UserActionResolutionInput, WriteDecisionReason, WriteTicket, WriteTicketStateSummary,
    CHANNEL_SUBMISSION_ID_MAX_BYTES,
};
use crate::values::{
    AcceptancePolicy, ChangeUnitOperation, CloseMutationIntent, CloseReason, CloseState,
    EvidenceDisplayState, MethodName, OperationCategory, PrepareWriteDecision, RedactionState,
    RequestedControlLevel, RequestedMode, ResumePolicy, RunKind, StatusCloseState,
    UnrecordedChangeResolutionBasis, UserActionRequiredFor, UtcTimestamp, WriteTicketEffect,
};

/// Shared typed mapping from a public request to its operation category.
pub trait MethodOperationCategory {
    /// Returns the public method name for this typed request.
    fn method_name(&self) -> MethodName;

    /// Returns the operation category for this typed request.
    fn operation_category(&self) -> OperationCategory;
}

/// Method-specific fields that become a complete public result only when
/// paired with the common result facts selected by the execution pipeline.
pub trait MethodResultFields {
    /// Complete public result type produced from these method fields.
    type Result;

    /// Attaches the final common result facts to these method fields.
    fn with_base(self, base: ToolResultBase) -> Self::Result;
}

macro_rules! declare_method_result {
    (
        $(#[$result_meta:meta])*
        pub struct $result:ident from $fields:ident {
            $(
                $(#[$field_meta:meta])*
                pub $field:ident: $field_type:ty
            ),* $(,)?
        }
    ) => {
        $(#[$result_meta])*
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $fields {
            $(
                $(#[$field_meta])*
                pub $field: $field_type,
            )*
        }

        $(#[$result_meta])*
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $result {
            pub base: ToolResultBase,
            $(
                $(#[$field_meta])*
                pub $field: $field_type,
            )*
        }

        impl MethodResultFields for $fields {
            type Result = $result;

            fn with_base(self, base: ToolResultBase) -> Self::Result {
                let Self {
                    $($field,)*
                } = self;
                $result {
                    base,
                    $($field,)*
                }
            }
        }
    };
}

fn deserialize_present_nullable<'de, D, T>(
    deserializer: D,
) -> Result<Option<RequiredNullable<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
        .map(RequiredNullable::from)
        .map(Some)
}

/// One branch allowed by a public method's response contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MethodResponseBranch {
    Result,
    Rejected,
    DryRun,
}

/// Typed relationship between one public request and its exact response family.
pub trait MethodResponseContract {
    type Response: DeserializeOwned + JsonSchema;

    const METHOD: MethodName;
    const RESPONSE_BRANCHES: &'static [MethodResponseBranch];
}

/// Canonical machine-readable declaration for one public method contract.
#[derive(Clone, Copy)]
pub struct PublicMethodContract {
    method: MethodName,
    request_contract_id: &'static str,
    response_contract_id: &'static str,
    response_branches: &'static [MethodResponseBranch],
    committed_result_replay: bool,
    request_schema: fn() -> Value,
    response_schema: fn() -> Value,
    result_schema: fn() -> Value,
    accepts_response: fn(&Value) -> bool,
    accepts_result_json: fn(&str, &Value) -> bool,
}

impl PublicMethodContract {
    pub const fn method(self) -> MethodName {
        self.method
    }

    pub const fn request_contract_id(self) -> &'static str {
        self.request_contract_id
    }

    pub const fn response_contract_id(self) -> &'static str {
        self.response_contract_id
    }

    pub const fn response_branches(self) -> &'static [MethodResponseBranch] {
        self.response_branches
    }

    pub fn supports_response_branch(self, branch: MethodResponseBranch) -> bool {
        self.response_branches.contains(&branch)
    }

    pub const fn has_committed_result_replay(self) -> bool {
        self.committed_result_replay
    }

    pub fn request_schema(self) -> Value {
        (self.request_schema)()
    }

    pub fn response_schema(self) -> Value {
        (self.response_schema)()
    }

    pub fn result_schema(self) -> Value {
        (self.result_schema)()
    }

    pub fn accepts_response(self, value: &Value) -> bool {
        (self.accepts_response)(value)
    }

    pub fn accepts_result_json(self, json: &str, value: &Value) -> bool {
        (self.accepts_result_json)(json, value)
    }
}

/// Maximum number of source UTF-8 bytes returned in one operation-result page.
pub const MAX_OPERATION_RESULT_PAGE_BYTES: usize = 16_384;

/// Immutable lookup coordinates for one exact committed mutation response.
///
/// This value is sidecar metadata over the stored response bytes. It must not
/// be inserted into the exact response JSON whose digest and size it carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationResultRef {
    pub project_id: ProjectId,
    pub source_method: MethodName,
    pub source_idempotency_key: IdempotencyKey,
    pub committed_state_version: u64,
    pub response_sha256: String,
    pub response_size_bytes: u64,
}

/// `volicord.intake` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IntakeRequest {
    pub envelope: ToolEnvelope,
    pub plain_language_request: String,
    pub requested_mode: RequestedMode,
    #[serde(default)]
    pub requested_control_level: RequestedControlLevel,
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

/// Intake initial scope object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitialScope {
    pub boundary: String,
    pub non_goals: Vec<String>,
    pub acceptance_criteria: Vec<AcceptanceCriterionInput>,
}

declare_method_result! {
    /// `volicord.intake` method result branch and its method-specific fields.
    pub struct IntakeResult from IntakeResultFields {
        pub task_ref: StateRecordRef,
        pub change_unit_ref: Option<StateRecordRef>,
        pub state: StateSummary,
        pub next_actions: Vec<NextActionSummary>,
    }
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

declare_method_result! {
    /// `volicord.update_scope` method result branch and its method-specific fields.
    pub struct UpdateScopeResult from UpdateScopeResultFields {
        pub task_ref: StateRecordRef,
        pub change_unit_ref: Option<StateRecordRef>,
        pub linked_scope_decision_refs: Vec<StateRecordRef>,
        pub stale_write_ticket_refs: Vec<StateRecordRef>,
        pub blocker_refs: Vec<StateRecordRef>,
        pub state: StateSummary,
        pub next_actions: Vec<NextActionSummary>,
    }
}

/// `volicord.status` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatusRequest {
    pub envelope: ToolEnvelope,
    pub include: StatusInclude,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuity_page: Option<RequiredNullable<ContinuityPageRequest>>,
}

impl MethodOperationCategory for StatusRequest {
    fn method_name(&self) -> MethodName {
        MethodName::Status
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::Read
    }
}

/// Status include flags shown by the method owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatusInclude {
    pub task: bool,
    pub pending_user_actions: bool,
    pub write_ticket: bool,
    pub evidence: bool,
    pub close: bool,
    pub guarantees: bool,
    pub continuity: bool,
}

/// Status-owned projection of `StateSummary` with include-controlled members
/// represented as optional object members instead of dynamically edited JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatusStateSummary {
    pub project_id: ProjectId,
    pub state_version: u64,
    pub task_ref: Option<StateRecordRef>,
    pub mode: Option<crate::values::TaskMode>,
    pub requested_control_level: Option<RequestedControlLevel>,
    pub effective_control_level: Option<crate::values::TaskControlLevel>,
    pub control_level_reason: Option<String>,
    pub project_policy: Option<crate::schema::ProjectWorkflowPolicySummary>,
    pub work_phase: Option<crate::values::WorkPhase>,
    pub acceptance_policy: Option<AcceptancePolicy>,
    pub acceptance_policy_reason: Option<String>,
    pub lineage: Option<crate::schema::TaskLineageSummary>,
    pub lifecycle: Option<crate::schema::TaskLifecycleState>,
    pub scope_revision: u64,
    pub goal_summary: Option<String>,
    pub scope_summary: Option<String>,
    pub non_goals: Vec<String>,
    pub acceptance_criteria: Vec<crate::schema::AcceptanceCriterion>,
    pub autonomy_boundary: Option<String>,
    pub active_change_unit_ref: Option<StateRecordRef>,
    pub effect_contract: Option<ChangeUnitEffectContract>,
    pub baseline_ref: Option<BaselineRef>,
    pub workspace_context: Option<crate::schema::WorkspaceContext>,
    pub shaping_readiness: Option<crate::schema::ShapingReadiness>,
    pub pending_user_action_summaries: Vec<AgentSafeUserActionRequestSummary>,
    pub blocker_refs: Vec<StateRecordRef>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub write_ticket_summary: Option<RequiredNullable<WriteTicketStateSummary>>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub evidence_summary: Option<RequiredNullable<EvidenceSummary>>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub evidence_gate: Option<RequiredNullable<EvidenceGateSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_state: Option<crate::values::CloseState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_blockers: Option<Vec<CloseReadinessBlocker>>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub guarantee_display: Option<RequiredNullable<GuaranteeDisplay>>,
}

impl StatusStateSummary {
    /// Selects the include-controlled members while retaining the typed state
    /// summary for every field that is present.
    pub fn from_state_summary(summary: StateSummary, include: &StatusInclude) -> Self {
        let StateSummary {
            project_id,
            state_version,
            task_ref,
            mode,
            requested_control_level,
            effective_control_level,
            control_level_reason,
            project_policy,
            work_phase,
            acceptance_policy,
            acceptance_policy_reason,
            lineage,
            lifecycle,
            scope_revision,
            goal_summary,
            scope_summary,
            non_goals,
            acceptance_criteria,
            autonomy_boundary,
            active_change_unit_ref,
            effect_contract,
            baseline_ref,
            workspace_context,
            shaping_readiness,
            pending_user_action_summaries,
            blocker_refs,
            write_ticket_summary,
            evidence_summary,
            evidence_gate,
            close_state,
            close_blockers,
            guarantee_display,
        } = summary;
        Self {
            project_id,
            state_version,
            task_ref,
            mode,
            requested_control_level,
            effective_control_level,
            control_level_reason,
            project_policy,
            work_phase,
            acceptance_policy,
            acceptance_policy_reason,
            lineage,
            lifecycle,
            scope_revision,
            goal_summary,
            scope_summary,
            non_goals,
            acceptance_criteria,
            autonomy_boundary,
            active_change_unit_ref,
            effect_contract,
            baseline_ref,
            workspace_context,
            shaping_readiness,
            pending_user_action_summaries,
            blocker_refs,
            write_ticket_summary: include
                .write_ticket
                .then(|| RequiredNullable::from(write_ticket_summary)),
            evidence_summary: include
                .evidence
                .then(|| RequiredNullable::from(evidence_summary)),
            evidence_gate: (include.evidence || include.close)
                .then(|| RequiredNullable::from(evidence_gate)),
            close_state: include.close.then_some(close_state).flatten(),
            close_blockers: include.close.then_some(close_blockers),
            guarantee_display: include
                .guarantees
                .then(|| RequiredNullable::from(guarantee_display)),
        }
    }
}

declare_method_result! {
    /// `volicord.status` method result branch and its method-specific fields.
    pub struct StatusResult from StatusResultFields {
        pub summary_card: SummaryCard,
        pub active_task: Option<StatusStateSummary>,
        pub status_summary: String,
        pub next_actions: Vec<NextActionSummary>,
        pub pending_user_action_summaries: Vec<AgentSafeUserActionRequestSummary>,
        pub blocker_refs: Vec<StateRecordRef>,
        pub write_ticket_summary: Option<WriteTicketStateSummary>,
        #[serde(
            default,
            deserialize_with = "deserialize_present_nullable",
            skip_serializing_if = "Option::is_none"
        )]
        pub evidence_summary: Option<RequiredNullable<EvidenceSummary>>,
        #[serde(
            default,
            deserialize_with = "deserialize_present_nullable",
            skip_serializing_if = "Option::is_none"
        )]
        pub evidence_gate: Option<RequiredNullable<EvidenceGateSummary>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub close_state: Option<StatusCloseState>,
        #[serde(
            default,
            deserialize_with = "deserialize_present_nullable",
            skip_serializing_if = "Option::is_none"
        )]
        pub current_close_basis: Option<RequiredNullable<CurrentCloseBasis>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub risk_acceptance_coverage: Option<Vec<RiskAcceptanceCoverage>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub close_blockers: Option<Vec<CloseReadinessBlocker>>,
        #[serde(
            default,
            deserialize_with = "deserialize_present_nullable",
            skip_serializing_if = "Option::is_none"
        )]
        pub guarantee_display: Option<RequiredNullable<GuaranteeDisplay>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub continuity_summary: Option<ProjectContinuityPage>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub task_flow: Option<Vec<TaskFlowItem>>,
        pub authority_receipt: Option<AuthorityReceipt>,
    }
}

/// `volicord.get_operation_result` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetOperationResultRequest {
    pub envelope: ToolEnvelope,
    pub operation_result_ref: OperationResultRef,
    pub cursor: RequiredNullable<String>,
}

impl MethodOperationCategory for GetOperationResultRequest {
    fn method_name(&self) -> MethodName {
        MethodName::GetOperationResult
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::Read
    }
}

declare_method_result! {
    /// One bounded page of an immutable historical mutation response.
    pub struct GetOperationResultResult from GetOperationResultResultFields {
        pub operation_result_ref: OperationResultRef,
        pub start_offset_bytes: u64,
        pub end_offset_bytes: u64,
        pub chunk_utf8: String,
        pub next_cursor: RequiredNullable<String>,
        pub complete: bool,
        pub historical: bool,
        pub current_authority_refresh_required: bool,
    }
}

/// `volicord.prepare_evidence_capture` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrepareEvidenceCaptureRequest {
    pub envelope: ToolEnvelope,
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub baseline_ref: BaselineRef,
    pub target: EvidenceTarget,
    pub capture: EvidenceCaptureSpec,
}

impl MethodOperationCategory for PrepareEvidenceCaptureRequest {
    fn method_name(&self) -> MethodName {
        MethodName::PrepareEvidenceCapture
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::AgentWorkflow
    }
}

declare_method_result! {
    /// `volicord.prepare_evidence_capture` method result branch and its method-specific fields.
    pub struct PrepareEvidenceCaptureResult from PrepareEvidenceCaptureResultFields {
        pub capture_intent_ref: StateRecordRef,
        pub capture_intent: EvidenceCaptureIntent,
        pub expires_at: UtcTimestamp,
    }
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

declare_method_result! {
    /// `volicord.prepare_write` method result branch and its method-specific fields.
    pub struct PrepareWriteResult from PrepareWriteResultFields {
        pub decision: PrepareWriteDecision,
        pub state: Option<StateSummary>,
        pub write_ticket_id: Option<WriteTicketId>,
        pub write_ticket_ref: Option<StateRecordRef>,
        pub write_ticket: Option<WriteTicket>,
        pub write_ticket_effect: WriteTicketEffect,
        pub allowed_path_patterns: Vec<String>,
        pub denied_path_patterns: Vec<String>,
        pub active_user_action_refs: Vec<StateRecordRef>,
        pub write_decision_reasons: Vec<WriteDecisionReason>,
        pub user_action_draft: Option<UserActionDraft>,
        pub guarantee_display: Option<GuaranteeDisplay>,
    }
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
    #[serde(default, skip_serializing_if = "RequiredNullable::is_none")]
    pub performed_operation: RequiredNullable<String>,
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

declare_method_result! {
    /// `volicord.record_run` method result branch and its method-specific fields.
    pub struct RecordRunResult from RecordRunResultFields {
        pub run_summary: RunSummary,
        pub registered_artifacts: Vec<ArtifactRef>,
        pub evidence_summary: Option<EvidenceSummary>,
        pub evidence_observations: Vec<EvidenceObservation>,
        pub evidence_producers: Vec<EvidenceProducer>,
        pub current_close_basis: Option<CurrentCloseBasis>,
        pub blocker_refs: Vec<StateRecordRef>,
        pub state: StateSummary,
    }
}

/// `volicord.request_user_action` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestUserActionRequest {
    pub envelope: ToolEnvelope,
    pub task_id: TaskId,
    pub change_unit_id: RequiredNullable<ChangeUnitId>,
    pub action: UserActionDraft,
    pub required_for: Vec<UserActionRequiredFor>,
    pub expires_at: RequiredNullable<UtcTimestamp>,
}

impl MethodOperationCategory for RequestUserActionRequest {
    fn method_name(&self) -> MethodName {
        MethodName::RequestUserAction
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::AgentWorkflow
    }
}

declare_method_result! {
    /// `volicord.request_user_action` method result branch and its method-specific fields.
    pub struct RequestUserActionResult from RequestUserActionResultFields {
        pub user_action_request_summary: AgentSafeUserActionRequestSummary,
        pub blocker_refs: Vec<StateRecordRef>,
        pub state: StateSummary,
    }
}

/// `volicord.resolve_user_action` request params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolveUserActionRequest {
    pub envelope: ToolEnvelope,
    pub user_action_request_id: UserActionRequestId,
    #[schemars(
        length(min = 1, max = "CHANNEL_SUBMISSION_ID_MAX_BYTES"),
        regex(pattern = "^[!-~]+$")
    )]
    pub channel_submission_id: String,
    pub resolution: UserActionResolutionInput,
}

impl MethodOperationCategory for ResolveUserActionRequest {
    fn method_name(&self) -> MethodName {
        MethodName::ResolveUserAction
    }

    fn operation_category(&self) -> OperationCategory {
        OperationCategory::UserOnly
    }
}

declare_method_result! {
    /// `volicord.resolve_user_action` method result branch and its method-specific fields.
    pub struct ResolveUserActionResult from ResolveUserActionResultFields {
        pub user_action_request_ref: StateRecordRef,
        pub user_action_resolution_ref: StateRecordRef,
        pub user_action_request: UserActionRequest,
        pub user_action_resolution: UserActionResolution,
        pub derived_refs: Vec<StateRecordRef>,
        pub state: StateSummary,
        pub next_actions: Vec<NextActionSummary>,
    }
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

/// Optional caller-supplied reconciliation request for a specific finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UnrecordedChangeResolutionRequest {
    pub unrecorded_change_id: UnrecordedChangeId,
    pub basis: UnrecordedChangeResolutionBasis,
    #[serde(default)]
    pub user_action_resolution_id: RequiredNullable<UserActionResolutionId>,
}

declare_method_result! {
    /// `volicord.reconcile_changes` method result branch and its method-specific fields.
    pub struct ReconcileChangesResult from ReconcileChangesResultFields {
        pub summary_card: SummaryCard,
        pub task_ref: StateRecordRef,
        pub unresolved_changes: Vec<UnrecordedChangeFinding>,
        pub resolved_changes: Vec<UnrecordedChangeResolutionSummary>,
        pub pending_user_action_summaries: Vec<AgentSafeUserActionRequestSummary>,
        pub rejected_resolution_requests: Vec<UnrecordedChangeRejection>,
        pub state: StateSummary,
        pub close_blockers: Vec<CloseReadinessBlocker>,
        pub next_actions: Vec<NextActionSummary>,
    }
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

declare_method_result! {
    /// `volicord.close_task` method result branch and its method-specific fields.
    pub struct CloseTaskResult from CloseTaskResultFields {
        pub summary_card: SummaryCard,
        pub close_state: CloseState,
        pub current_close_basis: Option<CurrentCloseBasis>,
        pub risk_acceptance_coverage: Vec<RiskAcceptanceCoverage>,
        pub continuity_summary: Vec<ProjectContinuitySummary>,
        pub state: StateSummary,
        pub blockers: Vec<CloseReadinessBlocker>,
        pub pending_user_action_summaries: Vec<AgentSafeUserActionRequestSummary>,
        pub evidence_summary: Option<EvidenceSummary>,
        pub evidence_gate: EvidenceGateSummary,
        pub artifact_refs: Vec<ArtifactRef>,
        pub authority_receipt: AuthorityReceipt,
    }
}

macro_rules! response_family {
    (result_or_rejected, $result:ty) => {
        ToolResultOrRejected<$result>
    };
    (previewable, $result:ty) => {
        PreviewableToolResponse<$result>
    };
}

macro_rules! response_branches {
    (result_or_rejected) => {
        &[MethodResponseBranch::Result, MethodResponseBranch::Rejected]
    };
    (previewable) => {
        &[
            MethodResponseBranch::Result,
            MethodResponseBranch::Rejected,
            MethodResponseBranch::DryRun,
        ]
    };
}

macro_rules! declare_public_method_contracts {
    (
        $(
            $variant:ident {
                request: $request:ty,
                result: $result:ty,
                response: $response:ident,
                family: $family:ident,
                request_contract: $request_contract:literal,
                response_contract: $response_contract:literal,
                committed_result_replay: $committed_result_replay:literal
            }
        ),+ $(,)?
    ) => {
        $(
            #[doc = concat!("Exact public response family for `", $response_contract, "`.")]
            pub type $response = response_family!($family, $result);

            impl MethodResponseContract for $request {
                type Response = $response;

                const METHOD: MethodName = MethodName::$variant;
                const RESPONSE_BRANCHES: &'static [MethodResponseBranch] =
                    response_branches!($family);
            }
        )+

        /// Every public method's request, result, response family, and contract identities.
        pub const PUBLIC_METHOD_CONTRACTS: &[PublicMethodContract] = &[
            $(
                PublicMethodContract {
                    method: MethodName::$variant,
                    request_contract_id: $request_contract,
                    response_contract_id: $response_contract,
                    response_branches: response_branches!($family),
                    committed_result_replay: $committed_result_replay,
                    request_schema: request_schema::<$request>,
                    response_schema: response_schema::<$response>,
                    result_schema: response_schema::<$result>,
                    accepts_response: accepts_response::<$response>,
                    accepts_result_json: accepts_result_json::<$result>,
                },
            )+
        ];
    };
}

declare_public_method_contracts! {
    Intake {
        request: IntakeRequest,
        result: IntakeResult,
        response: IntakeResponse,
        family: previewable,
        request_contract: "api.method.intake.request",
        response_contract: "api.method.intake.response",
        committed_result_replay: true
    },
    UpdateScope {
        request: UpdateScopeRequest,
        result: UpdateScopeResult,
        response: UpdateScopeResponse,
        family: previewable,
        request_contract: "api.method.update_scope.request",
        response_contract: "api.method.update_scope.response",
        committed_result_replay: true
    },
    Status {
        request: StatusRequest,
        result: StatusResult,
        response: StatusResponse,
        family: result_or_rejected,
        request_contract: "api.method.status.request",
        response_contract: "api.method.status.response",
        committed_result_replay: false
    },
    GetOperationResult {
        request: GetOperationResultRequest,
        result: GetOperationResultResult,
        response: GetOperationResultResponse,
        family: result_or_rejected,
        request_contract: "api.method.get_operation_result.request",
        response_contract: "api.method.get_operation_result.response",
        committed_result_replay: false
    },
    CheckClose {
        request: CheckCloseRequest,
        result: CloseTaskResult,
        response: CheckCloseResponse,
        family: result_or_rejected,
        request_contract: "api.method.check_close.request",
        response_contract: "api.method.check_close.response",
        committed_result_replay: false
    },
    PrepareEvidenceCapture {
        request: PrepareEvidenceCaptureRequest,
        result: PrepareEvidenceCaptureResult,
        response: PrepareEvidenceCaptureResponse,
        family: previewable,
        request_contract: "api.method.prepare_evidence_capture.request",
        response_contract: "api.method.prepare_evidence_capture.response",
        committed_result_replay: true
    },
    PrepareWrite {
        request: PrepareWriteRequest,
        result: PrepareWriteResult,
        response: PrepareWriteResponse,
        family: previewable,
        request_contract: "api.method.prepare_write.request",
        response_contract: "api.method.prepare_write.response",
        committed_result_replay: true
    },
    StageArtifact {
        request: StageArtifactRequest,
        result: StageArtifactResult,
        response: StageArtifactResponse,
        family: previewable,
        request_contract: "api.method.stage_artifact.request",
        response_contract: "api.method.stage_artifact.response",
        committed_result_replay: false
    },
    RecordRun {
        request: RecordRunRequest,
        result: RecordRunResult,
        response: RecordRunResponse,
        family: previewable,
        request_contract: "api.method.record_run.request",
        response_contract: "api.method.record_run.response",
        committed_result_replay: true
    },
    RequestUserAction {
        request: RequestUserActionRequest,
        result: RequestUserActionResult,
        response: RequestUserActionResponse,
        family: previewable,
        request_contract: "api.method.request_user_action.request",
        response_contract: "api.method.request_user_action.response",
        committed_result_replay: true
    },
    ResolveUserAction {
        request: ResolveUserActionRequest,
        result: ResolveUserActionResult,
        response: ResolveUserActionResponse,
        family: previewable,
        request_contract: "api.method.resolve_user_action.request",
        response_contract: "api.method.resolve_user_action.response",
        committed_result_replay: true
    },
    ReconcileChanges {
        request: ReconcileChangesRequest,
        result: ReconcileChangesResult,
        response: ReconcileChangesResponse,
        family: previewable,
        request_contract: "api.method.reconcile_changes.request",
        response_contract: "api.method.reconcile_changes.response",
        committed_result_replay: true
    },
    CloseTask {
        request: CloseTaskRequest,
        result: CloseTaskResult,
        response: CloseTaskResponse,
        family: previewable,
        request_contract: "api.method.close_task.request",
        response_contract: "api.method.close_task.response",
        committed_result_replay: true
    },
}

/// Returns the canonical declaration for one public method.
pub fn public_method_contract(method: MethodName) -> &'static PublicMethodContract {
    PUBLIC_METHOD_CONTRACTS
        .iter()
        .find(|contract| contract.method == method)
        .expect("every MethodName must have one public method contract")
}

/// Returns the canonical declaration for one public method name.
pub fn public_method_contract_by_name(method_name: &str) -> Option<&'static PublicMethodContract> {
    PUBLIC_METHOD_CONTRACTS
        .iter()
        .find(|contract| contract.method.as_str() == method_name)
}

/// Returns the generated JSON Schema for one public method request shape.
pub fn public_request_schema(method_name: &str) -> Option<Value> {
    public_method_contract_by_name(method_name).map(|contract| contract.request_schema())
}

/// Returns the generated JSON Schema for one public method response shape.
///
/// Public responses are object values even when their generated schema uses a
/// branch combinator for the method's exact response family.
pub fn public_response_schema(method_name: &str) -> Option<Value> {
    public_method_contract_by_name(method_name).map(|contract| contract.response_schema())
}

/// Returns the generated JSON Schema for one successful public method result body.
pub fn public_result_schema(method_name: &str) -> Option<Value> {
    public_method_contract_by_name(method_name).map(|contract| contract.result_schema())
}

fn accepts_response<T: DeserializeOwned>(value: &Value) -> bool {
    serde_json::from_value::<T>(value.clone()).is_ok()
}

fn accepts_result_json<T>(json: &str, value: &Value) -> bool
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json)
        .and_then(serde_json::to_value)
        .is_ok_and(|round_trip| round_trip == *value)
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::schema::{DryRunSummary, GuaranteeDisclosure, ToolDryRunResponse};
    use crate::values::{EffectKind, ResponseKind};

    fn dry_run_response_value() -> Value {
        serde_json::to_value(ToolDryRunResponse {
            base: ToolResultBase {
                response_kind: ResponseKind::DryRun,
                effect_kind: EffectKind::NoEffect,
                dry_run: true,
                state_version: Some(1),
                disclosure: GuaranteeDisclosure::authority_record(),
                events: Vec::new(),
            },
            dry_run_summary: DryRunSummary {
                planned_effects: Vec::new(),
                would_blockers: Vec::new(),
                would_errors: Vec::new(),
                next_actions: Vec::new(),
                diagnostics: Vec::new(),
            },
        })
        .expect("dry-run response should serialize")
    }

    #[test]
    fn canonical_registry_covers_every_method_exactly_once() {
        assert_eq!(PUBLIC_METHOD_CONTRACTS.len(), MethodName::ALL.len());
        assert_eq!(
            PUBLIC_METHOD_CONTRACTS
                .iter()
                .map(|contract| contract.method())
                .collect::<Vec<_>>(),
            MethodName::ALL
        );

        let unique_methods = PUBLIC_METHOD_CONTRACTS
            .iter()
            .map(|contract| contract.method().as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(unique_methods.len(), MethodName::ALL.len());
    }

    #[test]
    fn exact_response_decoders_and_schemas_follow_declared_branches() {
        let dry_run = dry_run_response_value();

        for contract in PUBLIC_METHOD_CONTRACTS {
            let supports_preview = contract.supports_response_branch(MethodResponseBranch::DryRun);
            assert_eq!(
                contract.accepts_response(&dry_run),
                supports_preview,
                "{} decoder disagrees with its response declaration",
                contract.method().as_str()
            );

            let schema = contract.response_schema();
            let branches = schema
                .get("anyOf")
                .and_then(Value::as_array)
                .expect("untagged exact response family should generate anyOf");
            assert_eq!(
                branches.len(),
                contract.response_branches().len(),
                "{} schema branch count disagrees with its response declaration",
                contract.method().as_str()
            );

            let rendered = serde_json::to_string(&schema).expect("schema should serialize");
            assert_eq!(
                rendered.contains("ToolDryRunResponse"),
                supports_preview,
                "{} schema exposes an impossible dry-run branch",
                contract.method().as_str()
            );
        }
    }
}
