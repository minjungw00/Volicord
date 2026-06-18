use std::{borrow::Cow, ops::Deref};

use schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::ids::{
    ArtifactId, ArtifactInputId, BaselineRef, ChangeUnitId, EventId, IdempotencyKey, ProjectId,
    RecordId, RequestId, RiskId, StagedArtifactHandleId, StorageRef, SurfaceId, SurfaceInstanceId,
    TaskId, UserJudgmentId, UserJudgmentOptionId,
};
use crate::values::{
    ActorKind, ArtifactAvailability, ArtifactInputSourceKind, CloseReadinessBlockerCategory,
    CloseReason, CloseState, EffectKind, ErrorCode, EvidenceCoverageState, EvidenceStatus,
    GuaranteeLevel, JudgmentBasisCompatibilityStatus, JudgmentKind, JudgmentPresentation,
    JudgmentRequiredFor, MethodName, NextActionKind, PlannedBlockerSourceKind, RedactionState,
    ResponseKind, RunKind, StateRecordKind, TaskLifecyclePhase, TaskMode, TaskResult,
    UserJudgmentStatus, ValidatorSeverity, ValidatorStatus, WriteAuthorizationStatus,
    WriteDecisionCategory,
};

/// JSON object used where an owner document defines a field as `object`.
pub type JsonObject = Map<String, Value>;

/// Required public field that may contain JSON `null`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequiredNullable<T>(Option<T>);

impl<T> RequiredNullable<T> {
    /// Creates a required-nullable wrapper from an optional semantic value.
    pub fn new(value: Option<T>) -> Self {
        Self(value)
    }

    /// Creates a present field carrying a non-null value.
    pub fn some(value: T) -> Self {
        Self(Some(value))
    }

    /// Creates a present field carrying JSON `null`.
    pub fn null() -> Self {
        Self(None)
    }

    /// Returns the semantic optional value by reference.
    pub fn as_ref(&self) -> Option<&T> {
        self.0.as_ref()
    }

    /// Returns the semantic optional value by mutable reference.
    pub fn as_mut(&mut self) -> Option<&mut T> {
        self.0.as_mut()
    }

    /// Returns true when the present field carries a non-null value.
    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }

    /// Returns true when the present field carries JSON `null`.
    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }

    /// Consumes the wrapper and returns the semantic optional value.
    pub fn into_option(self) -> Option<T> {
        self.0
    }

    /// Maps a non-null value to another value.
    pub fn map<U, F>(self, f: F) -> Option<U>
    where
        F: FnOnce(T) -> U,
    {
        self.0.map(f)
    }

    /// Returns this value or computes a fallback.
    pub fn or_else<F>(self, f: F) -> Option<T>
    where
        F: FnOnce() -> Option<T>,
    {
        self.0.or_else(f)
    }

    /// Returns the non-null value or computes a fallback.
    pub fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        self.0.unwrap_or_else(f)
    }

    /// Returns the non-null value or panics with the provided message.
    pub fn expect(self, message: &str) -> T {
        self.0.expect(message)
    }
}

impl<T> From<Option<T>> for RequiredNullable<T> {
    fn from(value: Option<T>) -> Self {
        Self::new(value)
    }
}

impl<T> From<T> for RequiredNullable<T> {
    fn from(value: T) -> Self {
        Self::some(value)
    }
}

impl<T> Deref for RequiredNullable<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Serialize for RequiredNullable<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for RequiredNullable<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if value.is_null() {
            Ok(Self(None))
        } else {
            T::deserialize(value)
                .map(Some)
                .map(Self)
                .map_err(serde::de::Error::custom)
        }
    }
}

impl<T> JsonSchema for RequiredNullable<T>
where
    T: JsonSchema,
{
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        format!("RequiredNullable_{}", T::schema_name())
    }

    fn schema_id() -> Cow<'static, str> {
        Cow::Owned(format!("RequiredNullable<{}>", T::schema_id()))
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        Option::<T>::json_schema(generator)
    }
}

/// Common public-method request envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToolEnvelope {
    pub project_id: ProjectId,
    pub task_id: RequiredNullable<TaskId>,
    pub actor_kind: ActorKind,
    pub surface_id: SurfaceId,
    pub request_id: RequestId,
    pub idempotency_key: RequiredNullable<IdempotencyKey>,
    pub expected_state_version: RequiredNullable<u64>,
    pub dry_run: bool,
    pub locale: RequiredNullable<String>,
}

/// Common result metadata carried by each concrete response branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolResultBase {
    pub response_kind: ResponseKind,
    pub effect_kind: EffectKind,
    pub dry_run: bool,
    pub state_version: Option<u64>,
    pub events: Vec<EventRef>,
}

/// Rejected response branch shared by public methods.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolRejectedResponse {
    pub base: ToolResultBase,
    pub errors: Vec<ToolError>,
}

/// Dry-run preview response branch shared by methods that define one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolDryRunResponse {
    pub base: ToolResultBase,
    pub dry_run_summary: DryRunSummary,
}

/// Method response branch wrapper.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ToolResponse<T> {
    Result(T),
    Rejected(ToolRejectedResponse),
    DryRun(ToolDryRunResponse),
}

/// Public API error item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolError {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
    pub details: Option<JsonObject>,
}

/// Event reference emitted in common result metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EventRef {
    pub event_id: EventId,
    pub event_kind: String,
}

/// Common dry-run summary shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DryRunSummary {
    pub planned_effects: Vec<PlannedEffect>,
    pub would_blockers: Vec<PlannedBlocker>,
    pub would_errors: Vec<ToolError>,
    pub next_actions: Vec<NextActionSummary>,
    pub diagnostics: Vec<String>,
}

/// Descriptive planned effect in a dry-run summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PlannedEffect {
    pub target_kind: String,
    pub action: String,
    pub description: String,
}

/// Descriptive planned blocker in a dry-run summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PlannedBlocker {
    pub source_kind: PlannedBlockerSourceKind,
    pub category: String,
    pub code: String,
    pub message: String,
    pub related_refs: Vec<StateRecordRef>,
}

/// Common public reference for Core-owned state records.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StateRecordRef {
    pub record_kind: StateRecordKind,
    pub record_id: RecordId,
    pub project_id: ProjectId,
    pub task_id: RequiredNullable<TaskId>,
    pub state_version: RequiredNullable<u64>,
}

/// Compact current-position state returned by public methods.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StateSummary {
    pub project_id: ProjectId,
    pub state_version: u64,
    pub task_ref: Option<StateRecordRef>,
    pub mode: Option<TaskMode>,
    pub lifecycle: Option<TaskLifecycleState>,
    pub goal_summary: Option<String>,
    pub scope_summary: Option<String>,
    pub non_goals: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub autonomy_boundary: Option<String>,
    pub active_change_unit_ref: Option<StateRecordRef>,
    pub baseline_ref: Option<BaselineRef>,
    pub shaping_readiness: Option<ShapingReadiness>,
    pub pending_user_judgment_refs: Vec<StateRecordRef>,
    pub blocker_refs: Vec<StateRecordRef>,
    pub write_authority_summary: Option<WriteAuthoritySummary>,
    pub evidence_summary: Option<EvidenceSummary>,
    pub close_state: Option<CloseState>,
    pub close_blockers: Vec<CloseReadinessBlocker>,
    pub guarantee_display: Option<GuaranteeDisplay>,
}

/// Task lifecycle state shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TaskLifecycleState {
    pub lifecycle_phase: TaskLifecyclePhase,
    pub close_reason: CloseReason,
    pub result: TaskResult,
    pub closed_at: Option<String>,
}

/// Shaping-readiness view over current Task and Change Unit state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShapingReadiness {
    pub goal_summary_known: bool,
    pub scope_boundary_known: bool,
    pub non_goals_known: bool,
    pub affected_area_or_paths_known: bool,
    pub acceptance_criteria_known: bool,
    pub autonomy_boundary_known: bool,
    pub first_change_unit_known: bool,
    pub user_owned_blocker_kind: Option<String>,
    pub next_safe_action: Option<NextActionSummary>,
    pub gaps: Vec<ShapingGap>,
}

/// Shaping gap display item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ShapingGap {
    pub gap_kind: String,
    pub message: String,
    pub blocker_ref: Option<StateRecordRef>,
    pub user_judgment_candidate_ref: Option<StateRecordRef>,
}

/// Canonical next-action display shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct NextActionSummary {
    pub action_kind: NextActionKind,
    pub owner_method: Option<MethodName>,
    pub label: String,
    pub blocking_question: Option<String>,
    pub required_refs: Vec<StateRecordRef>,
}

/// Current write-authority display summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WriteAuthoritySummary {
    pub status: WriteAuthorizationStatus,
    pub write_authorization_ref: Option<StateRecordRef>,
    pub basis_state_version: Option<u64>,
    pub intended_paths: Vec<String>,
    pub guarantee_display: Option<GuaranteeDisplay>,
}

/// Write Authorization summary returned by prepare-write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WriteAuthorizationSummary {
    pub write_authorization_ref: StateRecordRef,
    pub status: WriteAuthorizationStatus,
    pub authorized_attempt_scope: AuthorizedAttemptScope,
    pub basis_state_version: u64,
    pub expires_at: Option<String>,
}

/// One-attempt boundary captured by a Write Authorization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AuthorizedAttemptScope {
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub intended_operation: String,
    pub intended_paths: Vec<String>,
    pub product_file_write_intended: bool,
    pub sensitive_categories: Vec<String>,
    pub baseline_ref: Option<BaselineRef>,
}

/// Method-scoped prepare-write decision reason.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WriteDecisionReason {
    pub category: WriteDecisionCategory,
    pub code: String,
    pub message: String,
    pub related_refs: Vec<StateRecordRef>,
}

/// Evidence coverage summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceSummary {
    pub status: EvidenceStatus,
    pub completion_policy: CompletionPolicy,
    pub coverage_items: Vec<EvidenceCoverageItem>,
    pub artifact_refs: Vec<ArtifactRef>,
    pub updated_by_run_ref: Option<StateRecordRef>,
}

/// Evidence completion policy display shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CompletionPolicy {
    pub evidence_required: bool,
    pub required_claims: Vec<String>,
}

/// Evidence claim coverage item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCoverageItem {
    pub claim: String,
    pub required_for_close: bool,
    pub coverage_state: EvidenceCoverageState,
    pub supporting_refs: Vec<StateRecordRef>,
    pub supporting_artifact_refs: Vec<ArtifactRef>,
    pub gap_refs: Vec<StateRecordRef>,
}

/// Recorded run summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RunSummary {
    pub run_ref: StateRecordRef,
    pub kind: RunKind,
    pub summary: String,
    pub observed_changes: ObservedChanges,
    pub artifact_refs: Vec<ArtifactRef>,
}

/// Observed changes for a recorded run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservedChanges {
    pub changed_paths: Vec<String>,
    pub product_file_write_observed: bool,
    pub sensitive_categories: Vec<String>,
    pub baseline_ref: RequiredNullable<BaselineRef>,
}

/// Public close assessment input supplied by `harness.record_run`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CloseAssessmentInput {
    pub result_summary: String,
    pub result_refs: Vec<StateRecordRef>,
    pub residual_risks: Vec<ResidualRiskInput>,
    pub sensitive_categories: Vec<String>,
    pub recovery_constraints: Vec<String>,
}

/// Public residual-risk input supplied inside `CloseAssessmentInput`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResidualRiskInput {
    pub summary: String,
    pub consequence: String,
    pub acceptance_required: bool,
    pub source_refs: Vec<StateRecordRef>,
}

/// Current result and residual-risk state used for close-readiness responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CurrentCloseBasis {
    pub close_basis_revision: u64,
    pub scope_revision: u64,
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub baseline_ref: RequiredNullable<BaselineRef>,
    pub result_summary: String,
    pub result_refs: Vec<StateRecordRef>,
    pub evidence_summary_ref: RequiredNullable<StateRecordRef>,
    pub residual_risks: Vec<ResidualRisk>,
    pub sensitive_categories: Vec<String>,
    pub recovery_constraints: Vec<String>,
    pub source_run_ref: StateRecordRef,
    pub updated_at: String,
}

/// Named visible residual risk in a current close basis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResidualRisk {
    pub risk_id: RiskId,
    pub summary: String,
    pub consequence: String,
    pub acceptance_required: bool,
    pub source_refs: Vec<StateRecordRef>,
}

/// Residual-risk acceptance coverage for a current close basis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RiskAcceptanceCoverage {
    pub risk_id: RiskId,
    pub accepted: bool,
    pub accepted_by_judgment_refs: Vec<StateRecordRef>,
    pub missing_reason: RequiredNullable<String>,
}

/// Close-readiness blocker data shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CloseReadinessBlocker {
    pub category: CloseReadinessBlockerCategory,
    pub code: String,
    pub message: String,
    pub related_refs: Vec<StateRecordRef>,
    pub next_actions: Vec<NextActionSummary>,
}

/// Validator result display shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ValidatorResult {
    pub validator_id: String,
    pub status: ValidatorStatus,
    pub severity: Option<ValidatorSeverity>,
    pub message: String,
    pub related_refs: Vec<StateRecordRef>,
}

/// Security or capability guarantee display shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GuaranteeDisplay {
    pub level: GuaranteeLevel,
    pub basis: String,
    pub capability_refs: Vec<StateRecordRef>,
}

/// Public artifact reference and metadata shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub artifact_id: ArtifactId,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub display_name: String,
    pub content_type: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub redaction_state: RedactionState,
    pub availability: ArtifactAvailability,
    pub created_by_run_ref: RequiredNullable<StateRecordRef>,
    pub created_by_surface_id: RequiredNullable<SurfaceId>,
    pub created_by_surface_instance_id: RequiredNullable<SurfaceInstanceId>,
    pub storage_ref: RequiredNullable<StorageRef>,
}

/// Transient staged-artifact handle shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StagedArtifactHandle {
    pub handle_id: StagedArtifactHandleId,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub created_by_surface_id: SurfaceId,
    pub created_by_surface_instance_id: SurfaceInstanceId,
    pub content_type: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub redaction_state: RedactionState,
    pub expires_at: String,
    pub consumed: bool,
}

/// Request-side artifact link input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactInput {
    pub artifact_input_id: ArtifactInputId,
    pub source_kind: ArtifactInputSourceKind,
    pub staged_artifact_handle: RequiredNullable<StagedArtifactHandle>,
    pub existing_artifact_ref: RequiredNullable<ArtifactRef>,
    pub relation_hint: RequiredNullable<String>,
    pub claim: RequiredNullable<String>,
    pub expected_sha256: RequiredNullable<String>,
    pub expected_size_bytes: RequiredNullable<u64>,
    pub redaction_state: RequiredNullable<RedactionState>,
}

/// Durable user-owned judgment shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UserJudgment {
    pub judgment_id: UserJudgmentId,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub change_unit_id: Option<ChangeUnitId>,
    pub judgment_kind: JudgmentKind,
    pub status: UserJudgmentStatus,
    pub presentation: JudgmentPresentation,
    pub question: String,
    pub options: Vec<UserJudgmentOption>,
    pub context: UserJudgmentContext,
    pub affected_refs: Vec<StateRecordRef>,
    pub basis: Option<JudgmentBasis>,
    pub required_for: JudgmentRequiredFor,
    pub resolution: Option<UserJudgmentResolution>,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

/// Core-derived state snapshot used to decide whether a judgment is compatible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JudgmentBasis {
    pub task_id: TaskId,
    pub change_unit_id: RequiredNullable<ChangeUnitId>,
    pub scope_revision: u64,
    pub close_basis_revision: RequiredNullable<u64>,
    pub baseline_ref: RequiredNullable<BaselineRef>,
    pub result_refs: Vec<StateRecordRef>,
    pub residual_risk_ids: Vec<RiskId>,
    pub sensitive_action_scope: RequiredNullable<SensitiveActionScope>,
    pub created_at_state_version: u64,
    pub compatibility_status: JudgmentBasisCompatibilityStatus,
}

/// Proposed focused judgment shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UserJudgmentCandidate {
    pub judgment_kind: JudgmentKind,
    pub presentation: JudgmentPresentation,
    pub question: String,
    pub options: Vec<UserJudgmentOption>,
    pub context: UserJudgmentContext,
    pub affected_refs: Vec<StateRecordRef>,
    pub required_for: JudgmentRequiredFor,
    pub expires_at: Option<String>,
}

/// User judgment option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserJudgmentOption {
    pub option_id: UserJudgmentOptionId,
    pub label: String,
    pub description: String,
    pub consequence: String,
    pub is_default: bool,
}

/// User judgment context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserJudgmentContext {
    pub summary: String,
    pub related_refs: Vec<StateRecordRef>,
    pub artifact_refs: Vec<ArtifactRef>,
    pub visible_risks: Vec<AcceptedRiskInput>,
    pub constraints: Vec<String>,
}

/// Recorded judgment resolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UserJudgmentResolution {
    pub selected_option_id: UserJudgmentOptionId,
    pub answer: RecordUserJudgmentPayload,
    pub note: Option<String>,
    pub accepted_risks: Vec<AcceptedRiskInput>,
    pub resolved_by_actor_kind: ActorKind,
}

/// Decision-specific judgment payload branches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordUserJudgmentPayload {
    pub product_decision: RequiredNullable<JsonObject>,
    pub technical_decision: RequiredNullable<JsonObject>,
    pub scope_decision: RequiredNullable<JsonObject>,
    pub sensitive_action_scope: RequiredNullable<SensitiveActionScope>,
    pub final_acceptance: RequiredNullable<JsonObject>,
    pub residual_risk_acceptance: RequiredNullable<JsonObject>,
    pub cancellation: RequiredNullable<JsonObject>,
}

/// Sensitive-action approval context shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SensitiveActionScope {
    pub action_kind: String,
    pub description: String,
    pub intended_paths: Vec<String>,
    pub sensitive_categories: Vec<String>,
    pub command_or_tool_summary: RequiredNullable<String>,
    pub network_or_host_summary: RequiredNullable<String>,
    pub secret_or_credential_summary: RequiredNullable<String>,
    pub capability_claim: String,
    pub expires_at: RequiredNullable<String>,
}

/// Visible residual-risk input shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptedRiskInput {
    pub risk_id: RiskId,
    pub summary: String,
    pub consequence: String,
    pub related_refs: Vec<StateRecordRef>,
    pub accepted_for_close: bool,
}
