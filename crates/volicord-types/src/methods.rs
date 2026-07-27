use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ids::{
    BaselineRef, ChangeUnitId, IdempotencyKey, ProjectId, RunId, TaskId, UnrecordedChangeId,
    UserActionRequestId, UserActionResolutionId, WriteTicketId,
};
use crate::schema::{
    AcceptanceCriterionInput, AcceptanceCriterionReplacement, AgentSafeUserActionRequestSummary,
    ArtifactInput, ArtifactRef, AuthorityReceipt, ChangeUnitEffectContract, CloseAssessmentInput,
    CloseReadinessBlocker, ContinuityPageRequest, CurrentCloseBasis, EventRef,
    EvidenceCaptureIntent, EvidenceCaptureSpec, EvidenceCoverageUpdate, EvidenceGateSummary,
    EvidenceObservation, EvidenceObservationInput, EvidenceProducer, EvidenceSummary,
    EvidenceTarget, EvidenceUpdateProvenance, GuaranteeDisplay, JsonObject, NextActionSummary,
    ObservedChanges, ProjectContinuityPage, ProjectContinuitySummary, RequiredNullable,
    RiskAcceptanceCoverage, RunSummary, SourceRef, StagedArtifactHandle, StateRecordRef,
    StateSummary, SummaryCard, TaskFlowItem, TaskLineageInput, ToolDryRunResponse, ToolEnvelope,
    ToolRejectedResponse, ToolResponse, ToolResultBase, UnrecordedChangeFinding,
    UnrecordedChangeResolutionSummary, UserActionDraft, UserActionRequest, UserActionResolution,
    UserActionResolutionInput, WriteDecisionReason, WriteTicket, WriteTicketStateSummary,
    CHANNEL_SUBMISSION_ID_MAX_BYTES,
};
use crate::tool_names::AgentToolId;
use crate::values::{
    AcceptancePolicy, ActorSource, ChangeUnitOperation, CloseMutationIntent, CloseReason,
    CloseState, EffectKind, EvidenceAssuranceLevel, EvidenceCoverageUpdateState,
    EvidenceDisplayState, EvidenceRelevanceStatus, EvidenceSourceKind, JudgmentResolutionOutcome,
    MethodName, MutationDetailLevel, OperationCategory, PrepareWriteDecision, RedactionState,
    RequestedControlLevel, RequestedMode, ResumePolicy, RunKind, StatusCloseState,
    StatusDetailLevel, UnrecordedChangeResolutionBasis, UserActionChannelKind, UserActionKind,
    UserActionOptionAction, UserActionRequiredFor, UserActionStatus, UtcTimestamp,
    WriteTicketEffect,
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

/// Response branch type for `volicord.intake`.
pub type IntakeResponse = ToolResponse<IntakeResult>;

/// Response branch type for `volicord.update_scope`.
pub type UpdateScopeResponse = ToolResponse<UpdateScopeResult>;

/// Response branch type for `volicord.status`.
pub type StatusResponse = ToolResponse<StatusResult>;

/// Response branch type for `volicord.get_operation_result`.
pub type GetOperationResultResponse = ToolResponse<GetOperationResultResult>;

/// Response branch type for `volicord.check_close`.
pub type CheckCloseResponse = ToolResponse<CloseTaskResult>;

/// Response branch type for `volicord.prepare_write`.
pub type PrepareWriteResponse = ToolResponse<PrepareWriteResult>;

/// Response branch type for `volicord.prepare_evidence_capture`.
pub type PrepareEvidenceCaptureResponse = ToolResponse<PrepareEvidenceCaptureResult>;

/// Response branch type for `volicord.stage_artifact`.
pub type StageArtifactResponse = ToolResponse<StageArtifactResult>;

/// Response branch type for `volicord.record_run`.
pub type RecordRunResponse = ToolResponse<RecordRunResult>;

/// Response branch type for `volicord.request_user_action`.
pub type RequestUserActionResponse = ToolResponse<RequestUserActionResult>;

/// Response branch type for `volicord.resolve_user_action`.
pub type ResolveUserActionResponse = ToolResponse<ResolveUserActionResult>;

/// Response branch type for `volicord.reconcile_changes`.
pub type ReconcileChangesResponse = ToolResponse<ReconcileChangesResult>;

/// Response branch type for `volicord.close_task`.
pub type CloseTaskResponse = ToolResponse<CloseTaskResult>;

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
    pub user_channel_resolution: RequiredNullable<AgentSafeUserActionResolution>,
    pub derived_refs: Vec<StateRecordRef>,
}

/// Agent-safe projection of a verified user-channel resolution.
///
/// User-authored notes and evidence-observation summaries intentionally remain
/// outside the MCP-visible compound response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentSafeUserActionResolution {
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
    #[serde(rename = "store_access")]
    StoreAccess,
}

/// MCP wire projection of the unavailable infrastructure resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum McpOperationalResource {
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
#[serde(untagged)]
pub enum McpToolStructuredContent<T> {
    Response(Box<T>),
    AdapterError(McpToolErrorResponse),
}

/// Structured MCP result advertised by a Core-owned read-only tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum McpCoreToolStructuredContent<T> {
    Response(Box<T>),
    OperationalFailure(McpOperationalFailure),
    AdapterError(McpToolErrorResponse),
}

/// Compact method-effect facts preserved by mutation projections and recoveries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpMutationEffectSummary {
    pub effect_kind: EffectKind,
    pub state_version: Option<u64>,
    pub events: Vec<EventRef>,
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
    pub source_run_ref: StateRecordRef,
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
        selected_option_id: crate::ids::UserActionOptionId,
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
    pub unresolved_changes: Vec<UnrecordedChangeFinding>,
    pub resolved_changes: Vec<UnrecordedChangeResolutionSummary>,
    pub pending_user_action_summaries: Vec<AgentSafeUserActionRequestSummary>,
    pub rejected_resolution_requests: Vec<UnrecordedChangeRejection>,
}

/// Summary-detail MCP mutation branch over one fresh authority receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpMutationSummaryResponse<T> {
    pub operation_result_ref: RequiredNullable<OperationResultRef>,
    pub authority_receipt: AuthorityReceipt,
    pub method_result: T,
}

/// Workflow-detail MCP mutation branch over one fresh authority receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpMutationWorkflowResponse<T> {
    pub operation_result_ref: RequiredNullable<OperationResultRef>,
    pub authority_receipt: AuthorityReceipt,
    pub method_result: T,
    pub next_actions: Vec<NextActionSummary>,
}

/// Full-detail MCP mutation branch pairing fresh authority with the exact method result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpMutationFullResponse<T> {
    pub operation_result_ref: RequiredNullable<OperationResultRef>,
    pub authority_receipt: AuthorityReceipt,
    pub method_result: T,
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
    /// Correlates the applied effect; it is not an exact-result lookup credential.
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
    /// Correlates the applied effect; it is not an exact-result lookup credential.
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
    /// Correlates the applied effect; it is not an exact-result lookup credential.
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
#[serde(untagged)]
pub enum McpMutationStructuredContent<T, C> {
    Rejected(ToolRejectedResponse),
    DryRun(ToolDryRunResponse),
    Full(McpMutationFullResponse<Box<T>>),
    Summary(McpMutationSummaryResponse<C>),
    Workflow(McpMutationWorkflowResponse<C>),
    OperationalFailure(McpOperationalFailure),
    RefreshFailure(McpAuthoritativeRefreshFailure<C>),
    ResponseBudgetExceeded(McpMutationResponseBudgetExceeded<C>),
    PostEffectFailure(McpMutationPostEffectFailure),
    AdapterError(McpToolErrorResponse),
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuity_page: Option<RequiredNullable<ContinuityPageRequest>>,
}

impl StatusDetailLevel {
    /// Expands the MCP-visible detail level into the Core status include matrix.
    pub const fn include(self) -> StatusInclude {
        match self {
            Self::Summary => StatusInclude {
                task: true,
                pending_user_actions: false,
                write_ticket: false,
                evidence: false,
                close: false,
                guarantees: false,
                continuity: false,
            },
            Self::Workflow => StatusInclude {
                task: true,
                pending_user_actions: true,
                write_ticket: true,
                evidence: true,
                close: true,
                guarantees: true,
                continuity: false,
            },
            Self::Full => StatusInclude {
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

/// MCP-visible `volicord.get_operation_result` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpGetOperationResultArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    pub operation_result_ref: OperationResultRef,
    #[serde(default)]
    pub cursor: RequiredNullable<String>,
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

/// MCP-visible evidence-capture source selection with omission-equivalent expected outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "capture_kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpEvidenceCaptureSpec {
    VerifiedCommandExecution {
        command_sha256: String,
        command_label: String,
        #[serde(default)]
        expected_exit_code: RequiredNullable<i32>,
    },
    VerifiedToolInvocation {
        tool_name: String,
        tool_input_sha256: String,
        #[serde(default)]
        expected_success: RequiredNullable<bool>,
    },
}

impl From<McpEvidenceCaptureSpec> for EvidenceCaptureSpec {
    fn from(value: McpEvidenceCaptureSpec) -> Self {
        match value {
            McpEvidenceCaptureSpec::VerifiedCommandExecution {
                command_sha256,
                command_label,
                expected_exit_code,
            } => Self::VerifiedCommandExecution {
                command_sha256,
                command_label,
                expected_exit_code,
            },
            McpEvidenceCaptureSpec::VerifiedToolInvocation {
                tool_name,
                tool_input_sha256,
                expected_success,
            } => Self::VerifiedToolInvocation {
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
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub baseline_ref: BaselineRef,
    pub target: EvidenceTarget,
    pub capture: McpEvidenceCaptureSpec,
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
    #[serde(default)]
    pub performed_operation: RequiredNullable<String>,
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

/// MCP-visible `volicord.request_user_action` arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRequestUserActionArguments {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_selector: Option<String>,
    #[serde(default)]
    pub detail: MutationDetailLevel,
    pub request: McpRequestUserActionOperation,
}

/// Create-or-resume operation selected for the MCP user-action tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum McpRequestUserActionOperation {
    Create {
        task_id: TaskId,
        #[serde(default)]
        change_unit_id: RequiredNullable<ChangeUnitId>,
        action: UserActionDraft,
        required_for: Vec<UserActionRequiredFor>,
        #[serde(default)]
        expires_at: RequiredNullable<UtcTimestamp>,
    },
    Resume {
        user_action_request_id: UserActionRequestId,
    },
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

/// Returns the generated JSON Schema for one public method request shape.
pub fn public_request_schema(method_name: &str) -> Option<Value> {
    match method_name {
        "volicord.intake" => Some(request_schema::<IntakeRequest>()),
        "volicord.update_scope" => Some(request_schema::<UpdateScopeRequest>()),
        "volicord.status" => Some(request_schema::<StatusRequest>()),
        "volicord.get_operation_result" => Some(request_schema::<GetOperationResultRequest>()),
        "volicord.check_close" => Some(request_schema::<CheckCloseRequest>()),
        "volicord.prepare_evidence_capture" => {
            Some(request_schema::<PrepareEvidenceCaptureRequest>())
        }
        "volicord.prepare_write" => Some(request_schema::<PrepareWriteRequest>()),
        "volicord.stage_artifact" => Some(request_schema::<StageArtifactRequest>()),
        "volicord.record_run" => Some(request_schema::<RecordRunRequest>()),
        "volicord.request_user_action" => Some(request_schema::<RequestUserActionRequest>()),
        "volicord.resolve_user_action" => Some(request_schema::<ResolveUserActionRequest>()),
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
        "volicord.get_operation_result" => Some(response_schema::<GetOperationResultResponse>()),
        "volicord.check_close" => Some(response_schema::<CheckCloseResponse>()),
        "volicord.prepare_evidence_capture" => {
            Some(response_schema::<PrepareEvidenceCaptureResponse>())
        }
        "volicord.prepare_write" => Some(response_schema::<PrepareWriteResponse>()),
        "volicord.stage_artifact" => Some(response_schema::<StageArtifactResponse>()),
        "volicord.record_run" => Some(response_schema::<RecordRunResponse>()),
        "volicord.request_user_action" => Some(response_schema::<RequestUserActionResponse>()),
        "volicord.resolve_user_action" => Some(response_schema::<ResolveUserActionResponse>()),
        "volicord.reconcile_changes" => Some(response_schema::<ReconcileChangesResponse>()),
        "volicord.close_task" => Some(response_schema::<CloseTaskResponse>()),
        _ => None,
    }
}

/// Returns the generated JSON Schema for one MCP-visible tool argument shape.
pub fn mcp_request_schema(tool: AgentToolId) -> Option<Value> {
    match tool.method()? {
        MethodName::Intake => Some(request_schema::<McpIntakeArguments>()),
        MethodName::UpdateScope => Some(request_schema::<McpUpdateScopeArguments>()),
        MethodName::Status => Some(request_schema::<McpStatusArguments>()),
        MethodName::GetOperationResult => Some(request_schema::<McpGetOperationResultArguments>()),
        MethodName::PrepareEvidenceCapture => {
            Some(request_schema::<McpPrepareEvidenceCaptureArguments>())
        }
        MethodName::PrepareWrite => Some(request_schema::<McpPrepareWriteArguments>()),
        MethodName::StageArtifact => Some(request_schema::<McpStageArtifactArguments>()),
        MethodName::RecordRun => Some(request_schema::<McpRecordRunArguments>()),
        MethodName::RequestUserAction => Some(request_schema::<McpRequestUserActionArguments>()),
        MethodName::ReconcileChanges => Some(request_schema::<McpReconcileChangesArguments>()),
        MethodName::CheckClose => Some(request_schema::<McpCheckCloseArguments>()),
        MethodName::CloseTask => Some(request_schema::<McpCloseTaskArguments>()),
        MethodName::ResolveUserAction => None,
    }
}

/// Returns the generated JSON Schema for one MCP-visible public method result.
pub fn mcp_response_schema(tool: AgentToolId) -> Option<Value> {
    match tool.method()? {
        MethodName::RequestUserAction => Some(response_schema::<
            McpMutationStructuredContent<
                McpRequestUserActionResponse,
                McpRequestUserActionCompactResult,
            >,
        >()),
        MethodName::Intake => Some(response_schema::<
            McpMutationStructuredContent<IntakeResponse, McpMutationEffectSummary>,
        >()),
        MethodName::UpdateScope => Some(response_schema::<
            McpMutationStructuredContent<UpdateScopeResponse, McpMutationEffectSummary>,
        >()),
        MethodName::Status => Some(response_schema::<
            McpCoreToolStructuredContent<StatusResponse>,
        >()),
        MethodName::GetOperationResult => Some(response_schema::<
            McpCoreToolStructuredContent<GetOperationResultResponse>,
        >()),
        MethodName::PrepareEvidenceCapture => Some(response_schema::<
            McpMutationStructuredContent<
                PrepareEvidenceCaptureResponse,
                McpPrepareEvidenceCaptureCompactResult,
            >,
        >()),
        MethodName::PrepareWrite => Some(response_schema::<
            McpMutationStructuredContent<PrepareWriteResponse, McpPrepareWriteCompactResult>,
        >()),
        MethodName::StageArtifact => Some(response_schema::<
            McpMutationStructuredContent<StageArtifactResponse, McpStageArtifactCompactResult>,
        >()),
        MethodName::RecordRun => Some(response_schema::<
            McpMutationStructuredContent<RecordRunResponse, McpRecordRunCompactResult>,
        >()),
        MethodName::ReconcileChanges => Some(response_schema::<
            McpMutationStructuredContent<
                ReconcileChangesResponse,
                McpReconcileChangesCompactResult,
            >,
        >()),
        MethodName::CheckClose => Some(response_schema::<
            McpCoreToolStructuredContent<CheckCloseResponse>,
        >()),
        MethodName::CloseTask => Some(response_schema::<
            McpMutationStructuredContent<CloseTaskResponse, McpMutationEffectSummary>,
        >()),
        MethodName::ResolveUserAction => None,
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
