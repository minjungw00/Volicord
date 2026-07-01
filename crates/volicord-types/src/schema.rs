use std::{borrow::Cow, fmt, ops::Deref};

use schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::ids::{
    AgentConnectionId, AgentSessionId, ArtifactId, ArtifactInputId, BaselineRef, ChangeUnitId,
    EventId, EvidenceObservationId, GuardEventId, GuardInstallationId, IdempotencyKey,
    ProjectContinuityRecordId, ProjectId, PromptCaptureId, RecordId, RequestId, RiskId, RunId,
    StagedArtifactHandleId, StorageRef, TaskId, UnrecordedChangeId, UserJudgmentId,
    UserJudgmentOptionId,
};
use crate::values::{
    ActorSource, ArtifactAvailability, ArtifactInputSourceKind, ArtifactIntegrityStatus,
    ChangeUnitEffectKind, CloseReadinessBlockerCategory, CloseReason, CloseState, EffectKind,
    EnabledEnforcementMechanism, ErrorCode, EvidenceAssuranceLevel, EvidenceCoverageState,
    EvidenceSourceKind, EvidenceStatus, GuaranteeLevel, GuardConfigurationStatus, GuardDecision,
    GuardEffectiveStatus, GuardInstallationStatus, GuardMode, GuardObservationStatus,
    GuardStrength, HostKind, JudgmentBasisCompatibilityStatus, JudgmentKind, JudgmentPresentation,
    JudgmentRequiredFor, JudgmentResolutionOutcome, MethodName, NextActionKind,
    PlannedBlockerSourceKind, ProjectContinuityKind, ProjectContinuityStatus,
    ProjectEnforcementProfileSource, ProjectEnforcementProfileStatus, PromptCaptureStatus,
    RedactionState, ResponseKind, RunKind, SessionWatchCoverageBasis, SessionWatchStatus,
    StateRecordKind, TaskLifecyclePhase, TaskMode, TaskResult, UnrecordedChangeResolutionBasis,
    UnrecordedChangeStatus, UserJudgmentOptionAction, UserJudgmentStatus, UtcTimestamp,
    ValidatorSeverity, ValidatorStatus, WriteCheckStatus, WriteDecisionCategory,
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

impl<T> Default for RequiredNullable<T> {
    fn default() -> Self {
        Self::null()
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

/// Registry-scoped guard installation and host capability record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardInstallation {
    pub guard_installation_id: GuardInstallationId,
    pub runtime_home_id: String,
    pub connection_id: AgentConnectionId,
    pub project_id: RequiredNullable<ProjectId>,
    pub host_kind: HostKind,
    pub guard_mode: GuardMode,
    pub host_capability: JsonObject,
    pub installation_status: GuardInstallationStatus,
    pub installed_at: RequiredNullable<UtcTimestamp>,
    pub last_checked_at: UtcTimestamp,
    pub metadata: JsonObject,
}

/// Project-scoped Agent Session record for guarded operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentSession {
    pub session_id: AgentSessionId,
    pub project_id: ProjectId,
    pub connection_id: AgentConnectionId,
    pub guard_installation_id: RequiredNullable<GuardInstallationId>,
    pub host_kind: HostKind,
    pub guard_mode: GuardMode,
    pub started_at: UtcTimestamp,
    pub ended_at: RequiredNullable<UtcTimestamp>,
    pub metadata: JsonObject,
}

/// Project-scoped guard event record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardEvent {
    pub guard_event_id: GuardEventId,
    pub project_id: ProjectId,
    pub session_id: RequiredNullable<AgentSessionId>,
    pub connection_id: AgentConnectionId,
    pub guard_installation_id: RequiredNullable<GuardInstallationId>,
    pub event_kind: String,
    pub decision: GuardDecision,
    pub subject: JsonObject,
    pub result: JsonObject,
    pub occurred_at: UtcTimestamp,
    pub metadata: JsonObject,
}

/// Project-scoped prompt capture record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PromptCapture {
    pub prompt_capture_id: PromptCaptureId,
    pub project_id: ProjectId,
    pub session_id: AgentSessionId,
    pub connection_id: AgentConnectionId,
    pub capture_kind: String,
    pub prompt_sha256: String,
    pub prompt_text: RequiredNullable<String>,
    pub captured_at: UtcTimestamp,
    pub metadata: JsonObject,
}

/// Project-scoped unrecorded Product Repository change record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UnrecordedChange {
    pub unrecorded_change_id: UnrecordedChangeId,
    pub project_id: ProjectId,
    pub session_id: RequiredNullable<AgentSessionId>,
    pub connection_id: AgentConnectionId,
    pub task_id: RequiredNullable<TaskId>,
    pub status: UnrecordedChangeStatus,
    pub summary: String,
    pub observed_paths: Vec<String>,
    pub detection: JsonObject,
    pub resolution: RequiredNullable<JsonObject>,
    pub detected_at: UtcTimestamp,
    pub resolved_at: RequiredNullable<UtcTimestamp>,
    pub resolved_by_actor_source: RequiredNullable<ActorSource>,
    pub metadata: JsonObject,
}

/// Public finding summary for an unresolved unrecorded Product Repository change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UnrecordedChangeFinding {
    pub unrecorded_change_ref: StateRecordRef,
    pub status: UnrecordedChangeStatus,
    pub summary: String,
    pub observed_paths: Vec<String>,
    pub detected_at: UtcTimestamp,
    pub can_resolve_in_chat: bool,
    pub next_action: NextActionSummary,
}

/// Public resolution summary for an unrecorded Product Repository change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UnrecordedChangeResolutionSummary {
    pub unrecorded_change_ref: StateRecordRef,
    pub resolution_basis: UnrecordedChangeResolutionBasis,
    pub resolved_by_actor_source: ActorSource,
    pub capture_basis: String,
    pub user_judgment_ref: RequiredNullable<StateRecordRef>,
    pub resolved_at: UtcTimestamp,
}

/// Compact guard-health projection for close-readiness and status views.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardHealthSummary {
    pub guard_mode: GuardMode,
    pub guard_strength: GuardStrength,
    pub guard_installation_id: RequiredNullable<GuardInstallationId>,
    pub guard_installation_status: GuardInstallationStatus,
    pub guard_configuration_status: GuardConfigurationStatus,
    pub guard_observation_status: GuardObservationStatus,
    pub effective_guard_status: GuardEffectiveStatus,
    pub generated_config_verified: bool,
    pub native_host_output_adapter_verified: bool,
    pub hook_path_safety: String,
    pub hook_commands_cwd_independent: bool,
    pub hook_commands_subdirectory_safe: bool,
    pub pre_tool_blocking_available: bool,
    pub post_tool_correlation_available: bool,
    pub bash_shell_mutation_coverage: bool,
    pub direct_file_write_matcher_coverage: bool,
    pub bypass_detection_active: bool,
    pub guard_hook_observed: bool,
    pub last_guard_observed_at: RequiredNullable<UtcTimestamp>,
    pub last_guard_event_at: RequiredNullable<UtcTimestamp>,
    pub host_kind: RequiredNullable<HostKind>,
    pub observed_hook_phase: RequiredNullable<String>,
    pub observed_host_kind: RequiredNullable<HostKind>,
    pub expected_policy_hash: RequiredNullable<String>,
    pub observed_policy_hash: RequiredNullable<String>,
    pub observed_binary_version: RequiredNullable<String>,
    pub required_hook_phases: Vec<String>,
    pub missing_required_hook_phases: Vec<String>,
    pub prompt_capture_status: PromptCaptureStatus,
    pub prompt_capture_available: bool,
    pub local_web_consent_available: bool,
    pub managed_distribution_verified: bool,
    pub mcp_connection_healthy: bool,
    pub mcp_connection_status: RequiredNullable<String>,
    pub session_watch_status: SessionWatchStatus,
    pub last_session_watch_checked_at: RequiredNullable<UtcTimestamp>,
    pub session_watch_baseline_created_at: RequiredNullable<UtcTimestamp>,
    pub session_watch_coverage_start_at: RequiredNullable<UtcTimestamp>,
    pub session_watch_coverage_basis: RequiredNullable<SessionWatchCoverageBasis>,
    pub session_watch_partial_coverage_warning: RequiredNullable<String>,
    pub session_watch_detail: RequiredNullable<String>,
    pub unresolved_unrecorded_change_count: u64,
    pub missing_or_stale_write_readiness: bool,
}

/// Project-level continuity record that preserves durable context after Task close.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectContinuityRecord {
    pub continuity_record_id: ProjectContinuityRecordId,
    pub project_id: ProjectId,
    pub source_task_id: TaskId,
    pub source_change_unit_id: RequiredNullable<ChangeUnitId>,
    pub kind: ProjectContinuityKind,
    pub title: String,
    pub summary: String,
    pub rationale: RequiredNullable<String>,
    pub applies_to_paths: Vec<String>,
    pub applies_to_refs: Vec<StateRecordRef>,
    pub source_refs: Vec<StateRecordRef>,
    pub artifact_refs: Vec<ArtifactRef>,
    pub status: ProjectContinuityStatus,
    pub supersedes_refs: Vec<StateRecordRef>,
    pub review_triggers: Vec<String>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

/// Compact project-level continuity view for status responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectContinuitySummary {
    pub continuity_record_ref: StateRecordRef,
    pub kind: ProjectContinuityKind,
    pub status: ProjectContinuityStatus,
    pub title: String,
    pub summary: String,
    pub source_task_ref: StateRecordRef,
    pub source_change_unit_ref: RequiredNullable<StateRecordRef>,
    pub review_triggers: Vec<String>,
}

/// Baseline cooperative project enforcement profile identifier.
pub const BASELINE_COOPERATIVE_ENFORCEMENT_PROFILE_ID: &str = "baseline_cooperative";

/// Canonical baseline cooperative enforcement profile JSON stored for projects.
pub const BASELINE_PROJECT_ENFORCEMENT_PROFILE_JSON: &str = r#"{"profile_id":"baseline_cooperative","guarantee_level":"cooperative","enabled_mechanisms":[],"source":"baseline_scope","status":"active"}"#;

/// Persisted project-owned enforcement profile used to project guarantee display.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectEnforcementProfile {
    pub profile_id: String,
    pub guarantee_level: GuaranteeLevel,
    pub enabled_mechanisms: Vec<EnabledEnforcementMechanism>,
    pub source: ProjectEnforcementProfileSource,
    pub status: ProjectEnforcementProfileStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<StateRecordRef>,
}

/// Returns the baseline cooperative project enforcement profile.
pub fn baseline_project_enforcement_profile() -> ProjectEnforcementProfile {
    ProjectEnforcementProfile {
        profile_id: BASELINE_COOPERATIVE_ENFORCEMENT_PROFILE_ID.to_owned(),
        guarantee_level: GuaranteeLevel::Cooperative,
        enabled_mechanisms: Vec::new(),
        source: ProjectEnforcementProfileSource::BaselineScope,
        status: ProjectEnforcementProfileStatus::Active,
        notes: Vec::new(),
        refs: Vec::new(),
    }
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
    pub effect_contract: Option<ChangeUnitEffectContract>,
    pub baseline_ref: Option<BaselineRef>,
    pub shaping_readiness: Option<ShapingReadiness>,
    pub pending_user_judgment_refs: Vec<StateRecordRef>,
    pub blocker_refs: Vec<StateRecordRef>,
    pub write_check_summary: Option<WriteCheckStateSummary>,
    pub evidence_summary: Option<EvidenceSummary>,
    pub close_state: Option<CloseState>,
    pub close_blockers: Vec<CloseReadinessBlocker>,
    pub guard_health: Option<GuardHealthSummary>,
    pub guarantee_display: Option<GuaranteeDisplay>,
}

/// Optional Change Unit effect contract recorded as Core state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangeUnitEffectContract {
    #[serde(default)]
    pub allowed_effects: Vec<ChangeUnitEffectKind>,
    #[serde(default)]
    pub forbidden_effects: Vec<ChangeUnitEffectKind>,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub expected_outputs: Vec<String>,
    #[serde(default)]
    pub invariants: Vec<String>,
    #[serde(default)]
    pub evidence_expectations: Vec<String>,
    #[serde(default)]
    pub sensitive_action_expectations: Vec<String>,
}

/// Task lifecycle state shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TaskLifecycleState {
    pub lifecycle_phase: TaskLifecyclePhase,
    pub close_reason: CloseReason,
    pub result: TaskResult,
    pub closed_at: Option<UtcTimestamp>,
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

/// Current Write Check display summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WriteCheckStateSummary {
    pub status: WriteCheckStatus,
    pub write_check_ref: Option<StateRecordRef>,
    pub basis_state_version: Option<u64>,
    pub intended_paths: Vec<String>,
    pub consumed_by_run_ref: Option<StateRecordRef>,
    pub observation_refs: Vec<StateRecordRef>,
    pub guarantee_display: Option<GuaranteeDisplay>,
}

/// Write Check summary returned by prepare-write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WriteCheckSummary {
    pub write_check_ref: StateRecordRef,
    pub status: WriteCheckStatus,
    pub attempt_scope: WriteCheckAttemptScope,
    pub basis_state_version: u64,
    pub expires_at: Option<UtcTimestamp>,
}

/// One-attempt boundary captured by a Write Check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WriteCheckAttemptScope {
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
    pub observation_refs: Vec<StateRecordRef>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<EvidenceUpdateProvenance>,
    pub supporting_refs: Vec<StateRecordRef>,
    pub observation_refs: Vec<StateRecordRef>,
    pub supporting_artifact_refs: Vec<ArtifactRef>,
    pub gap_refs: Vec<StateRecordRef>,
}

/// Request-side provenance used by `volicord.record_run` to create an evidence observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceUpdateProvenance {
    pub source_kind: EvidenceSourceKind,
    pub assurance_level: EvidenceAssuranceLevel,
    pub observed_at: RequiredNullable<UtcTimestamp>,
    pub tool_name: RequiredNullable<String>,
    pub tool_invocation_id: RequiredNullable<String>,
    pub tool_metadata: JsonObject,
    pub limitations: Vec<String>,
}

/// Durable evidence observation record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceObservation {
    pub observation_id: EvidenceObservationId,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub change_unit_id: RequiredNullable<ChangeUnitId>,
    pub run_ref: RequiredNullable<StateRecordRef>,
    pub claim: String,
    pub source_kind: EvidenceSourceKind,
    pub assurance_level: EvidenceAssuranceLevel,
    pub observed_by_actor_source: RequiredNullable<ActorSource>,
    pub tool_name: RequiredNullable<String>,
    pub tool_invocation_id: RequiredNullable<String>,
    pub tool_metadata: JsonObject,
    pub input_refs: Vec<StateRecordRef>,
    pub output_artifact_refs: Vec<ArtifactRef>,
    pub limitations: Vec<String>,
    pub observed_at: UtcTimestamp,
    pub recorded_at: UtcTimestamp,
}

/// Request-side evidence observation input supplied by `volicord.record_run`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceObservationInput {
    pub claim: String,
    pub source_kind: EvidenceSourceKind,
    pub assurance_level: EvidenceAssuranceLevel,
    pub observed_by_actor_source: RequiredNullable<ActorSource>,
    pub tool_name: RequiredNullable<String>,
    pub tool_invocation_id: RequiredNullable<String>,
    pub tool_metadata: JsonObject,
    pub input_refs: Vec<StateRecordRef>,
    pub output_artifact_refs: Vec<ArtifactRef>,
    pub limitations: Vec<String>,
    pub observed_at: UtcTimestamp,
}

/// Persisted audit metadata for an evidence summary row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedEvidenceMetadata {
    pub updated_by_run_id: RunId,
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

/// Public close assessment input supplied by `volicord.record_run`.
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
    pub sensitive_action_requirements: Vec<SensitiveActionRequirement>,
    pub recovery_constraints: Vec<String>,
    pub source_run_ref: StateRecordRef,
    pub updated_at: UtcTimestamp,
}

/// Core-derived sensitive action requirement in a current close basis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SensitiveActionRequirement {
    pub action_kind: String,
    pub normalized_paths: Vec<String>,
    pub sensitive_categories: Vec<String>,
    pub baseline_ref: RequiredNullable<BaselineRef>,
    pub change_unit_id: ChangeUnitId,
    pub source_run_ref: StateRecordRef,
    pub source_write_check_ref: StateRecordRef,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard_strength: Option<GuardStrength>,
    #[serde(default)]
    pub can_resolve_in_chat: bool,
    #[serde(default)]
    pub terminal_action_required: bool,
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
    pub content_type: RequiredNullable<String>,
    pub sha256: RequiredNullable<String>,
    pub size_bytes: RequiredNullable<u64>,
    pub integrity_status: ArtifactIntegrityStatus,
    pub redaction_state: RedactionState,
    pub availability: ArtifactAvailability,
    pub created_by_run_ref: RequiredNullable<StateRecordRef>,
    pub created_by_actor_source: RequiredNullable<ActorSource>,
    pub storage_ref: RequiredNullable<StorageRef>,
}

/// Persisted producer identity facts for a durable artifact row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedArtifactProducer {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    pub created_by_actor_source: ActorSource,
    pub artifact_input_id: ArtifactInputId,
    #[serde(default)]
    pub relation_hint: RequiredNullable<String>,
    #[serde(default)]
    pub claim: RequiredNullable<String>,
}

/// Persisted provenance facts for a durable artifact row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedArtifactProvenance {
    pub source_kind: ArtifactInputSourceKind,
    pub producer_run_id: RunId,
    pub source_staging_handle_id: StagedArtifactHandleId,
}

/// Persisted JSON metadata used to complete artifact provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedArtifactProvenanceMetadata {
    pub source_kind: ArtifactInputSourceKind,
}

/// Transient staged-artifact handle shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StagedArtifactHandle {
    pub handle_id: StagedArtifactHandleId,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub created_by_actor_source: ActorSource,
    pub content_type: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub redaction_state: RedactionState,
    pub expires_at: UtcTimestamp,
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
    pub basis: JudgmentBasis,
    pub required_for: Vec<JudgmentRequiredFor>,
    pub resolution: Option<UserJudgmentResolution>,
    pub expires_at: Option<UtcTimestamp>,
    pub created_at: UtcTimestamp,
    pub resolved_at: Option<UtcTimestamp>,
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

/// Stored shape for `user_judgments.basis_json`.
pub type PersistedJudgmentBasis = JudgmentBasis;

/// Stored shape for `user_judgments.request_json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedUserJudgmentRequest {
    pub presentation: JudgmentPresentation,
    pub question: String,
    pub required_for: Vec<JudgmentRequiredFor>,
    pub expires_at: RequiredNullable<UtcTimestamp>,
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
    pub required_for: Vec<JudgmentRequiredFor>,
    pub expires_at: Option<UtcTimestamp>,
}

/// Caller-authored request input for a non-authority judgment option.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserJudgmentOptionInput {
    pub option_id: UserJudgmentOptionId,
    pub label: String,
    pub description: String,
    pub consequence: String,
    pub is_default: bool,
}

/// Stored representation for `user_judgments.options_json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedUserJudgmentOptions {
    pub schema_version: u32,
    pub options: Vec<PersistedUserJudgmentOption>,
}

impl PersistedUserJudgmentOptions {
    /// Creates the current persisted option representation.
    pub fn current(options: Vec<UserJudgmentOption>) -> Self {
        Self {
            schema_version: 1,
            options: options
                .into_iter()
                .map(PersistedUserJudgmentOption::from_current)
                .collect(),
        }
    }

    /// Consumes the persisted wrapper and returns its current public option set.
    pub fn into_current_options(
        self,
    ) -> Result<Vec<UserJudgmentOption>, PersistedUserJudgmentCompatibilityError> {
        self.options
            .into_iter()
            .map(PersistedUserJudgmentOption::into_current)
            .collect()
    }
}

impl<'de> Deserialize<'de> for PersistedUserJudgmentOptions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            schema_version: u32,
            options: Vec<PersistedUserJudgmentOption>,
        }

        let value = Value::deserialize(deserializer)?;
        let wire = Wire::deserialize(value).map_err(serde::de::Error::custom)?;
        if wire.schema_version != 1 {
            return Err(serde::de::Error::custom(
                "persisted user judgment options schema_version must be 1",
            ));
        }
        Ok(Self {
            schema_version: wire.schema_version,
            options: wire.options,
        })
    }
}

/// Versioned internal representation for `user_judgments.options_json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedUserJudgmentOption {
    pub option_id: UserJudgmentOptionId,
    pub label: String,
    pub description: String,
    pub consequence: String,
    pub machine_action: UserJudgmentOptionAction,
    pub resolution_outcome: JudgmentResolutionOutcome,
    pub is_default: bool,
}

impl PersistedUserJudgmentOption {
    /// Creates a persisted option from a current public option.
    pub fn from_current(option: UserJudgmentOption) -> Self {
        Self {
            option_id: option.option_id,
            label: option.label,
            description: option.description,
            consequence: option.consequence,
            machine_action: option.machine_action,
            resolution_outcome: option.resolution_outcome,
            is_default: option.is_default,
        }
    }

    /// Converts a persisted option into the current public option shape.
    pub fn into_current(
        self,
    ) -> Result<UserJudgmentOption, PersistedUserJudgmentCompatibilityError> {
        if self.resolution_outcome != self.machine_action.resolution_outcome() {
            return Err(PersistedUserJudgmentCompatibilityError::MismatchedResolutionOutcome);
        }
        Ok(UserJudgmentOption {
            option_id: self.option_id,
            label: self.label,
            description: self.description,
            consequence: self.consequence,
            machine_action: self.machine_action,
            resolution_outcome: self.resolution_outcome,
            is_default: self.is_default,
        })
    }
}

/// Reason a persisted judgment shape cannot be emitted as current public data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistedUserJudgmentCompatibilityError {
    MismatchedResolutionOutcome,
}

impl fmt::Display for PersistedUserJudgmentCompatibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MismatchedResolutionOutcome => {
                "persisted judgment option outcome does not match machine_action"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PersistedUserJudgmentCompatibilityError {}

/// Current Core-owned user judgment option.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserJudgmentOption {
    pub option_id: UserJudgmentOptionId,
    pub label: String,
    pub description: String,
    pub consequence: String,
    pub machine_action: UserJudgmentOptionAction,
    pub resolution_outcome: JudgmentResolutionOutcome,
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
#[serde(deny_unknown_fields)]
pub struct UserJudgmentResolution {
    pub selected_option_id: UserJudgmentOptionId,
    pub machine_action: UserJudgmentOptionAction,
    pub resolution_outcome: JudgmentResolutionOutcome,
    pub answer: RecordUserJudgmentPayload,
    pub rationale: JudgmentRationale,
    pub note: RequiredNullable<String>,
    pub accepted_risks: Vec<AcceptedRiskInput>,
    pub resolved_by_actor_source: ActorSource,
}

/// Stored shape for `user_judgments.resolution_json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PersistedUserJudgmentResolution {
    pub selected_option_id: UserJudgmentOptionId,
    pub machine_action: UserJudgmentOptionAction,
    pub resolution_outcome: JudgmentResolutionOutcome,
    pub answer: RecordUserJudgmentPayload,
    pub note: Option<String>,
    pub accepted_risks: Vec<AcceptedRiskInput>,
    pub resolved_by_actor_source: ActorSource,
}

impl PersistedUserJudgmentResolution {
    /// Creates the current persisted representation from a current public resolution.
    pub fn current(resolution: UserJudgmentResolution) -> Self {
        Self {
            selected_option_id: resolution.selected_option_id,
            machine_action: resolution.machine_action,
            resolution_outcome: resolution.resolution_outcome,
            answer: resolution.answer,
            note: resolution.note.into_option(),
            accepted_risks: resolution.accepted_risks,
            resolved_by_actor_source: resolution.resolved_by_actor_source,
        }
    }

    /// Converts a persisted resolution into the current public resolution shape.
    pub fn into_current_with_outcome(
        self,
        resolution_outcome: JudgmentResolutionOutcome,
        rationale: JudgmentRationale,
    ) -> Result<UserJudgmentResolution, PersistedUserJudgmentCompatibilityError> {
        if self.resolution_outcome != resolution_outcome {
            return Err(PersistedUserJudgmentCompatibilityError::MismatchedResolutionOutcome);
        }
        if self.machine_action.resolution_outcome() != resolution_outcome {
            return Err(PersistedUserJudgmentCompatibilityError::MismatchedResolutionOutcome);
        }
        Ok(UserJudgmentResolution {
            selected_option_id: self.selected_option_id,
            machine_action: self.machine_action,
            resolution_outcome,
            answer: self.answer,
            rationale,
            note: self.note.into(),
            accepted_risks: self.accepted_risks,
            resolved_by_actor_source: self.resolved_by_actor_source,
        })
    }
}

impl<'de> Deserialize<'de> for PersistedUserJudgmentResolution {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            selected_option_id: UserJudgmentOptionId,
            machine_action: UserJudgmentOptionAction,
            resolution_outcome: JudgmentResolutionOutcome,
            answer: RecordUserJudgmentPayload,
            note: Option<String>,
            accepted_risks: Vec<AcceptedRiskInput>,
            resolved_by_actor_source: ActorSource,
        }

        let wire = Wire::deserialize(deserializer)?;
        if populated_judgment_answer_branches(&wire.answer) != 1 {
            return Err(serde::de::Error::custom(
                "persisted judgment resolution must populate exactly one answer branch",
            ));
        }
        Ok(Self {
            selected_option_id: wire.selected_option_id,
            machine_action: wire.machine_action,
            resolution_outcome: wire.resolution_outcome,
            answer: wire.answer,
            note: wire.note,
            accepted_risks: wire.accepted_risks,
            resolved_by_actor_source: wire.resolved_by_actor_source,
        })
    }
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

/// Descriptive, non-authority explanation for a recorded judgment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JudgmentRationale {
    pub summary: String,
    pub selected_reason: RequiredNullable<String>,
    pub considered_alternatives: Vec<String>,
    pub rejected_alternatives: Vec<String>,
    pub assumptions: Vec<String>,
    pub tradeoffs: Vec<String>,
    pub uncertainties: Vec<String>,
    pub review_triggers: Vec<String>,
    pub related_refs: Vec<StateRecordRef>,
    pub artifact_refs: Vec<ArtifactRef>,
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
    pub expires_at: RequiredNullable<UtcTimestamp>,
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

fn populated_judgment_answer_branches(answer: &RecordUserJudgmentPayload) -> usize {
    usize::from(answer.product_decision.is_some())
        + usize::from(answer.technical_decision.is_some())
        + usize::from(answer.scope_decision.is_some())
        + usize::from(answer.sensitive_action_scope.is_some())
        + usize::from(answer.final_acceptance.is_some())
        + usize::from(answer.residual_risk_acceptance.is_some())
        + usize::from(answer.cancellation.is_some())
}
