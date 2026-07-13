use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    fmt,
    ops::Deref,
};

use schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::ids::{
    AcceptanceCriterionId, AgentConnectionId, AgentSessionId, ArtifactId, ArtifactInputId,
    BaselineRef, ChangeUnitId, EventId, EvidenceCaptureIntentId, EvidenceCaptureReceiptId,
    EvidenceClaimId, EvidenceObservationId, EvidenceProducerId, GuardEventId, GuardInstallationId,
    IdempotencyKey, ProjectContinuityRecordId, ProjectId, PromptCaptureId, RecordId, RequestId,
    RiskId, RunId, StagedArtifactHandleId, StorageRef, TaskId, UnrecordedChangeId,
    UserActionOptionId, UserActionRequestId, UserActionResolutionId, WriteTicketId,
};
use crate::values::{
    AcceptancePolicy, ActorSource, ArtifactAvailability, ArtifactInputSourceKind,
    ArtifactIntegrityStatus, AuthorityNextActor, CarryForwardDispositionStatus, CarryForwardKind,
    ChangeUnitEffectKind, CloseReadinessBlockerCategory, CloseReason, CloseState,
    ConnectionObservationSourceKind, CoverageHostHookState, CoverageSessionWatcherState,
    EffectKind, EnabledEnforcementMechanism, ErrorCode, EvidenceAssuranceLevel,
    EvidenceCoverageState, EvidenceCoverageUpdateState, EvidenceDisplayState, EvidenceGateState,
    EvidenceProducerKind, EvidenceRelevanceStatus, EvidenceRequirement, EvidenceSourceKind,
    EvidenceStatus, GuaranteeClass, GuaranteeLevel, GuardConfigurationStatus, GuardDecision,
    GuardEffectiveStatus, GuardInstallationStatus, GuardObservationStatus, HostKind,
    IntegrationProfile, JudgmentKind, JudgmentPresentation, JudgmentResolutionOutcome, MethodName,
    NextActionKind, NextActionPresentationRole, NonGuarantee, OperationCategory,
    PlannedBlockerSourceKind, ProjectContinuityKind, ProjectContinuityStatus,
    ProjectEnforcementProfileSource, ProjectEnforcementProfileStatus, PromptCaptureStatus,
    RedactionState, ResponseKind, RunKind, SessionWatchCoverageBasis, SessionWatchStatus,
    StateRecordKind, StatusCloseState, TaskLifecyclePhase, TaskLineageRelation, TaskMode,
    TaskResult, UnrecordedChangeResolutionBasis, UnrecordedChangeStatus, UserActionBasisStatus,
    UserActionChannelKind, UserActionKind, UserActionOptionAction, UserActionRequiredFor,
    UserActionStatus, UtcTimestamp, ValidatorSeverity, ValidatorStatus, WorkPhase, WorkspaceVcs,
    WriteDecisionCategory, WriteTicketState, WriteTicketStatus,
};

/// JSON object used where an owner document defines a field as `object`.
pub type JsonObject = Map<String, Value>;

/// Stable snapshot digest algorithm used by session-watch capture outcomes.
pub const WATCH_SNAPSHOT_ALGORITHM: &str = "volicord_session_watch_snapshot_v1_sha256";

/// Owner-defined lifetime of one immutable evidence-capture intent.
pub const EVIDENCE_CAPTURE_INTENT_TTL_MINUTES: i64 = 15;

/// Owner-defined lifetime of one evidence-observation user-action request.
pub const USER_ACTION_EVIDENCE_OBSERVATION_TTL_MINUTES: i64 = 15;

/// Owner-defined maximum lifetime of one request-bound User Channel token.
pub const USER_ACTION_CHANNEL_TOKEN_MAX_TTL_SECONDS: u64 = 10 * 60;

/// Controlled limitation recorded for Volicord-owned command capture.
pub const EVIDENCE_CAPTURE_COMMAND_LIMITATION: &str = "environment_not_bound";

/// Controlled limitation recorded for registered cooperative guard capture.
pub const EVIDENCE_CAPTURE_GUARD_LIMITATION: &str = "registered_hook_cooperative_not_attested";

/// Controlled limitation recorded for registered session-watcher capture.
pub const EVIDENCE_CAPTURE_WATCHER_LIMITATION: &str = "registered_session_watcher_not_attested";

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
    pub disclosure: GuaranteeDisclosure,
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

/// Shared public disclosure for what a result means and does not prove.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuaranteeDisclosure {
    pub guarantee_class: GuaranteeClass,
    pub guarantees: Vec<String>,
    pub non_guarantees: Vec<NonGuarantee>,
}

impl GuaranteeDisclosure {
    /// Disclosure for public Core method responses and persisted authority-state views.
    pub fn authority_record() -> Self {
        Self {
            guarantee_class: GuaranteeClass::AuthorityRecord,
            guarantees: vec![
                "Reports Core authority state, response branch metadata, and method-owned result fields for the selected project.".to_owned(),
                "Reports close-readiness and write-compatibility results only within the documented method contract.".to_owned(),
            ],
            non_guarantees: broad_non_guarantees(),
        }
    }

    /// Disclosure for cooperative host-hook decisions returned to an external host.
    pub fn cooperative_host_decision() -> Self {
        Self {
            guarantee_class: GuaranteeClass::CooperativeHostDecision,
            guarantees: vec![
                "Reports the decision Volicord returned to a cooperative host hook for one observed event.".to_owned(),
                "May record the host event and Volicord decision when the host-hook command reaches the storage path.".to_owned(),
            ],
            non_guarantees: broad_non_guarantees(),
        }
    }

    /// Disclosure for local diagnostic, verification, observation, or transport-status results.
    pub fn detective_observation() -> Self {
        Self {
            guarantee_class: GuaranteeClass::DetectiveObservation,
            guarantees: vec![
                "Reports local diagnostic observations from accessible Runtime Home, host configuration, transport, or hook state.".to_owned(),
                "Records what Volicord could inspect at the time of the check.".to_owned(),
            ],
            non_guarantees: broad_non_guarantees(),
        }
    }

    /// Disclosure for immutable user-owned action resolutions.
    pub fn user_action_resolution() -> Self {
        Self {
            guarantee_class: GuaranteeClass::UserActionResolution,
            guarantees: vec![
                "Records a user-owned action resolution received through a supported User Channel path."
                    .to_owned(),
                "Preserves the closed resolution body and compatibility basis used by the owning method."
                    .to_owned(),
            ],
            non_guarantees: broad_non_guarantees(),
        }
    }
}

fn broad_non_guarantees() -> Vec<NonGuarantee> {
    vec![
        NonGuarantee::NotOsSandbox,
        NonGuarantee::NotNetworkIsolation,
        NonGuarantee::NotMalwareDefense,
        NonGuarantee::NotTamperProofAuditLog,
        NonGuarantee::NotCorrectnessProof,
        NonGuarantee::NotTestSufficiencyProof,
        NonGuarantee::NotHumanReviewReplacement,
        NonGuarantee::NotFullWritePrevention,
        NonGuarantee::NotFullFilesystemMonitoring,
        NonGuarantee::NotActorAttributionProof,
        NonGuarantee::NotIntentProof,
    ]
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
    pub produced_at_state_version: RequiredNullable<u64>,
}

/// One-based inclusive line range within a repository-file source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceLineRange {
    pub start_line: u64,
    pub end_line: u64,
}

/// Non-authoritative Product Repository file source metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RepositoryFileSource {
    pub repository_path: String,
    pub baseline_commit_sha: String,
    pub content_sha256: String,
    pub line_range: RequiredNullable<SourceLineRange>,
}

/// Non-authoritative Git commit source metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitCommitSource {
    pub commit_sha: String,
}

/// Non-authoritative Git diff source metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitDiffSource {
    pub base_commit_sha: String,
    pub head_commit_sha: String,
    pub diff_artifact_ref: RequiredNullable<ArtifactRef>,
}

/// Non-authoritative command-invocation source metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandSource {
    pub invocation_id: String,
    pub command_summary: String,
    pub exit_code: i32,
    pub output_artifact_ref: RequiredNullable<ArtifactRef>,
}

/// Non-authoritative external HTTP source metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExternalUriSource {
    pub uri: String,
    pub retrieved_at: UtcTimestamp,
    pub content_sha256: String,
}

/// Non-authoritative host or user-context correlation metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserContextSource {
    pub context_id: String,
}

/// Caller-supplied context or provenance that never grants Core authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "source_kind",
    content = "source",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SourceRef {
    RepositoryFile(RepositoryFileSource),
    GitCommit(GitCommitSource),
    GitDiff(GitDiffSource),
    Command(CommandSource),
    ExternalUri(ExternalUriSource),
    UserContext(UserContextSource),
}

/// Registry-scoped host-hook installation and host capability record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardInstallation {
    pub guard_installation_id: GuardInstallationId,
    pub runtime_home_id: String,
    pub connection_id: AgentConnectionId,
    pub project_id: RequiredNullable<ProjectId>,
    pub host_kind: HostKind,
    pub integration_profile: IntegrationProfile,
    pub host_capability: JsonObject,
    pub installation_status: GuardInstallationStatus,
    pub installed_at: RequiredNullable<UtcTimestamp>,
    pub last_checked_at: UtcTimestamp,
    pub metadata: JsonObject,
}

/// Project-scoped Agent Session record for host-observed operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentSession {
    pub session_id: AgentSessionId,
    pub project_id: ProjectId,
    pub connection_id: AgentConnectionId,
    pub guard_installation_id: RequiredNullable<GuardInstallationId>,
    pub host_kind: HostKind,
    pub integration_profile: IntegrationProfile,
    pub started_at: UtcTimestamp,
    pub ended_at: RequiredNullable<UtcTimestamp>,
    pub metadata: JsonObject,
}

/// Project-scoped host-hook event record.
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
    pub user_action_resolution_ref: RequiredNullable<StateRecordRef>,
    pub resolved_at: UtcTimestamp,
}

/// Public summary of Volicord control-surface capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ControlSurfaceSummary {
    pub selected_profile: IntegrationProfile,
    pub host_hooks_active: bool,
    pub session_watcher_active: bool,
    pub cooperative_pre_tool_warning_available: bool,
    pub cooperative_pre_tool_denial_available: bool,
    pub unrecorded_changes_detectable: bool,
    pub actor_identity_provable: bool,
    pub os_enforced: bool,
}

/// Compact integration-health projection for close-readiness and status views.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardHealthSummary {
    pub selected_profile: IntegrationProfile,
    pub control_surface: ControlSurfaceSummary,
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
    pub cooperative_pre_tool_warning_available: bool,
    pub cooperative_pre_tool_denial_available: bool,
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
    pub mcp_connection_healthy: bool,
    pub mcp_connection_status: RequiredNullable<String>,
    pub session_watch_status: SessionWatchStatus,
    pub last_session_watch_checked_at: RequiredNullable<UtcTimestamp>,
    pub session_watch_baseline_created_at: RequiredNullable<UtcTimestamp>,
    pub session_watch_coverage_start_at: RequiredNullable<UtcTimestamp>,
    pub session_watch_coverage_basis: RequiredNullable<SessionWatchCoverageBasis>,
    pub session_watch_partial_coverage_warning: RequiredNullable<String>,
    pub session_watch_detail: RequiredNullable<String>,
    pub session_watch_scan_summary: RequiredNullable<SessionWatchScanSummary>,
    pub unresolved_unrecorded_change_count: u64,
    pub missing_or_stale_write_ticket: bool,
    pub write_ticket_path_scope_violation: bool,
}

/// Compact scan-limit and skipped-path summary for a session watcher snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionWatchScanSummary {
    pub files_scanned: u64,
    pub files_skipped: u64,
    pub unreadable_paths_count: u64,
    pub degraded_reasons: Vec<String>,
    pub degraded_reason_counts: BTreeMap<String, u64>,
    pub skipped_paths_sample: Vec<String>,
    pub skipped_paths_truncated: bool,
    pub default_excluded_paths: Vec<String>,
    pub max_file_size_bytes: u64,
    pub max_file_count: u64,
    pub follows_symlinks: bool,
    pub not_full_filesystem_monitoring: bool,
}

/// Concise coverage projection for status and close-readiness views.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CoverageSummary {
    pub active_profile: IntegrationProfile,
    pub host_hook_state: CoverageHostHookState,
    pub session_watcher_state: CoverageSessionWatcherState,
    pub coverage_started_at: RequiredNullable<UtcTimestamp>,
    pub last_snapshot_at: RequiredNullable<UtcTimestamp>,
    pub watcher_scan_summary: RequiredNullable<SessionWatchScanSummary>,
    pub unresolved_unrecorded_change_count: u64,
    pub non_guarantees: Vec<NonGuarantee>,
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
    pub work_phase: Option<WorkPhase>,
    pub acceptance_policy: Option<AcceptancePolicy>,
    pub acceptance_policy_reason: Option<String>,
    pub lineage: Option<TaskLineageSummary>,
    pub lifecycle: Option<TaskLifecycleState>,
    pub scope_revision: u64,
    pub goal_summary: Option<String>,
    pub scope_summary: Option<String>,
    pub non_goals: Vec<String>,
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    pub autonomy_boundary: Option<String>,
    pub active_change_unit_ref: Option<StateRecordRef>,
    pub effect_contract: Option<ChangeUnitEffectContract>,
    pub baseline_ref: Option<BaselineRef>,
    pub workspace_context: Option<WorkspaceContext>,
    pub shaping_readiness: Option<ShapingReadiness>,
    pub pending_user_action_refs: Vec<StateRecordRef>,
    pub blocker_refs: Vec<StateRecordRef>,
    pub write_ticket_summary: Option<WriteTicketStateSummary>,
    pub evidence_summary: Option<EvidenceSummary>,
    pub evidence_gate: Option<EvidenceGateSummary>,
    pub close_state: Option<CloseState>,
    pub close_blockers: Vec<CloseReadinessBlocker>,
    pub guard_health: Option<GuardHealthSummary>,
    pub guarantee_display: Option<GuaranteeDisplay>,
}

/// Explicit predecessor selection supplied only when intake creates a Task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskLineageInput {
    pub predecessor_task_id: TaskId,
    pub relation: TaskLineageRelation,
    pub creation_reason: String,
    pub carry_forward: Vec<CarryForwardKind>,
}

/// Recorded disposition for one selected predecessor material category.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CarryForwardDisposition {
    pub kind: CarryForwardKind,
    pub status: CarryForwardDispositionStatus,
    pub source_refs: Vec<StateRecordRef>,
}

/// Current Task's canonical predecessor edge and carry-forward audit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskLineageSummary {
    pub predecessor_task_ref: StateRecordRef,
    pub relation: TaskLineageRelation,
    pub creation_reason: String,
    pub carry_forward: Vec<CarryForwardDisposition>,
}

/// One Task in the connected predecessor flow returned by full status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskFlowItem {
    pub task_ref: StateRecordRef,
    pub predecessor_task_ref: Option<StateRecordRef>,
    pub relation: Option<TaskLineageRelation>,
    pub mode: TaskMode,
    pub work_phase: WorkPhase,
    pub lifecycle_phase: TaskLifecyclePhase,
}

/// Verified Git coordinate captured with a Change Unit baseline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceContext {
    pub vcs: WorkspaceVcs,
    pub git_common_dir: String,
    pub worktree_id: String,
    pub branch_ref: Option<String>,
    pub head_sha: Option<String>,
    pub workspace_fingerprint: String,
}

/// Core-generated compact receipt over current Task authority state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthorityReceipt {
    pub project_id: ProjectId,
    pub state_version: u64,
    pub task_ref: StateRecordRef,
    pub change_unit_ref: Option<StateRecordRef>,
    pub scope_revision: u64,
    pub latest_run_ref: Option<StateRecordRef>,
    pub product_file_write_observed: bool,
    pub evidence_gate: Option<EvidenceGateSummary>,
    pub close_state: StatusCloseState,
    pub close_blockers: Vec<CloseReadinessBlocker>,
    pub next_actor: AuthorityNextActor,
    pub next_action: Option<NextActionSummary>,
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
    pub user_action_request_candidate_ref: Option<StateRecordRef>,
}

/// Canonical next-action display shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct NextActionSummary {
    pub presentation_role: NextActionPresentationRole,
    pub action_kind: NextActionKind,
    pub owner_method: Option<MethodName>,
    pub allowed_operation_categories: Vec<OperationCategory>,
    pub label: String,
    pub blocking_question: Option<String>,
    pub expected_state_version: RequiredNullable<u64>,
    pub required_refs: Vec<StateRecordRef>,
}

/// Stable compact status card shared by status-like outputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SummaryCard {
    pub task: String,
    pub recording: String,
    pub profile: String,
    pub write_ticket: String,
    pub evidence: String,
    pub user_action: String,
    pub changes: String,
    pub close_status: String,
    pub transport: String,
    pub next: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_action: Option<NextActionSummary>,
    pub guarantee: String,
}

/// Current write ticket display summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WriteTicketStateSummary {
    pub status: WriteTicketStatus,
    pub write_ticket_ref: Option<StateRecordRef>,
    pub basis_state_version: Option<u64>,
    pub intended_paths: Vec<String>,
    pub consumed_by_run_ref: Option<StateRecordRef>,
    pub observation_refs: Vec<StateRecordRef>,
    pub guarantee_display: Option<GuaranteeDisplay>,
}

/// Allowed and denied Product Repository path patterns captured by a write ticket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WriteTicketPathPatterns {
    pub allowed: Vec<String>,
    pub denied: Vec<String>,
}

/// One-attempt boundary captured by a write ticket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WriteTicketScope {
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub intended_operation: String,
    pub product_file_write_intended: bool,
    pub sensitive_categories: Vec<String>,
    pub baseline_ref: Option<BaselineRef>,
}

/// Write ticket authority record returned by prepare-write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WriteTicket {
    pub write_ticket_id: WriteTicketId,
    pub write_ticket_ref: StateRecordRef,
    pub state: WriteTicketState,
    pub scope: WriteTicketScope,
    pub path_patterns: WriteTicketPathPatterns,
    pub observed_paths: Vec<String>,
    pub basis_state_version: u64,
    pub expires_at: Option<UtcTimestamp>,
    pub control_surface: Option<ControlSurfaceSummary>,
    pub guarantee_display: Option<GuaranteeDisplay>,
}

/// One-attempt boundary captured by a write ticket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WriteTicketAttemptScope {
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

/// Intake-side acceptance criterion without caller-selected identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceCriterionInput {
    pub statement: String,
    pub evidence_requirement: EvidenceRequirement,
}

/// Update-scope replacement entry for one acceptance criterion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceCriterionReplacement {
    pub acceptance_criterion_id: RequiredNullable<AcceptanceCriterionId>,
    pub statement: String,
    pub evidence_requirement: EvidenceRequirement,
}

/// Current canonical acceptance criterion projected for a Task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceCriterion {
    pub acceptance_criterion_id: AcceptanceCriterionId,
    pub statement: String,
    pub evidence_requirement: EvidenceRequirement,
}

/// Stable evidence target selected by coverage, observations, and artifacts.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(tag = "target_kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvidenceTarget {
    AcceptanceCriterion {
        acceptance_criterion_id: AcceptanceCriterionId,
    },
    SupplementalClaim {
        evidence_claim_id: EvidenceClaimId,
        statement: String,
    },
}

/// Exact source selection and expected outcome for one evidence-capture intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "capture_kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvidenceCaptureSpec {
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
    RegisteredConnectionObservation {
        source_kind: ConnectionObservationSourceKind,
        observation_input_sha256: String,
        expected_complete: RequiredNullable<bool>,
    },
}

/// Immutable current-basis request for a registered source to capture evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCaptureIntent {
    pub capture_intent_id: EvidenceCaptureIntentId,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub scope_revision: u64,
    pub baseline_ref: BaselineRef,
    pub target: EvidenceTarget,
    pub capture: EvidenceCaptureSpec,
    pub input_sha256: String,
    pub expected_outcome: JsonObject,
    pub requested_by_actor_source: ActorSource,
    pub workspace_context: JsonObject,
    pub created_at: UtcTimestamp,
    pub expires_at: UtcTimestamp,
}

/// Immutable durable source-fact receipt created by one registered evidence source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCaptureReceipt {
    pub capture_receipt_id: EvidenceCaptureReceiptId,
    pub capture_intent_id: EvidenceCaptureIntentId,
    pub capture_intent_ref: StateRecordRef,
    pub producer_kind: EvidenceProducerKind,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub scope_revision: u64,
    pub baseline_ref: BaselineRef,
    pub target: EvidenceTarget,
    pub input_sha256: String,
    pub result_sha256: String,
    pub expected_outcome: JsonObject,
    pub observed_outcome: JsonObject,
    pub source_refs: Vec<StateRecordRef>,
    pub connection_id: AgentConnectionId,
    pub session_id: RequiredNullable<AgentSessionId>,
    pub guard_installation_id: RequiredNullable<GuardInstallationId>,
    pub guard_event_ids: Vec<GuardEventId>,
    pub watch_observation_refs: Vec<String>,
    pub staged_receipt_handle: StagedArtifactHandle,
    pub complete: bool,
    pub limitations: Vec<String>,
    pub redaction_state: RedactionState,
    pub observed_by_actor_source: ActorSource,
    pub observed_at: UtcTimestamp,
    pub recorded_at: UtcTimestamp,
}

/// Persisted registered-source coordinates inside one safe capture receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedEvidenceCaptureReceiptSource {
    pub connection_id: AgentConnectionId,
    pub session_id: RequiredNullable<AgentSessionId>,
    pub guard_installation_id: RequiredNullable<GuardInstallationId>,
    pub guard_event_ids: Vec<GuardEventId>,
    pub watch_observation_refs: Vec<String>,
    pub host_invocation_id: RequiredNullable<String>,
}

/// Canonical bounded safe receipt body shared by source adapters and Core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedEvidenceCaptureReceiptBody {
    pub schema_version: String,
    pub capture_kind: EvidenceProducerKind,
    pub capture_intent_id: EvidenceCaptureIntentId,
    pub input_sha256: String,
    pub result_sha256: String,
    pub expected_outcome: JsonObject,
    pub observed_outcome: JsonObject,
    pub source: PersistedEvidenceCaptureReceiptSource,
    pub complete: bool,
    pub limitations: Vec<String>,
    pub redaction_state: RedactionState,
    pub observed_by_actor_source: ActorSource,
    pub observed_at: UtcTimestamp,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandCaptureObservedOutcome {
    #[allow(dead_code)]
    exit_code: i32,
    stdout_sha256: String,
    #[allow(dead_code)]
    stdout_size_bytes: u64,
    stderr_sha256: String,
    #[allow(dead_code)]
    stderr_size_bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolCaptureObservedOutcome {
    #[allow(dead_code)]
    success: bool,
    #[allow(dead_code)]
    exit_code: RequiredNullable<i32>,
    tool_result_sha256: String,
    #[allow(dead_code)]
    tool_result_size_bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GuardConnectionCaptureObservedOutcome {
    complete: bool,
    guard_event_kind: String,
    guard_decision: String,
    observation_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WatcherConnectionCaptureObservedOutcome {
    complete: bool,
    snapshot_algorithm: String,
    snapshot_digest: String,
    observation_sha256: String,
}

/// Strictly validates the class-specific expected-outcome object derived for an
/// evidence-capture intent.
pub fn validate_evidence_capture_expected_outcome(
    capture: &EvidenceCaptureSpec,
    outcome: &JsonObject,
) -> Result<(), String> {
    if *outcome == evidence_capture_expected_outcome(capture) {
        Ok(())
    } else {
        Err("expected outcome does not match the canonical capture expectation".to_owned())
    }
}

/// Derives the one canonical expected-outcome object for a normalized capture
/// specification.
pub fn evidence_capture_expected_outcome(capture: &EvidenceCaptureSpec) -> JsonObject {
    let mut outcome = JsonObject::new();
    match capture {
        EvidenceCaptureSpec::VerifiedCommandExecution {
            expected_exit_code, ..
        } => {
            outcome.insert(
                "expected_exit_code".to_owned(),
                Value::from(expected_exit_code.as_ref().copied().unwrap_or(0)),
            );
        }
        EvidenceCaptureSpec::VerifiedToolInvocation {
            expected_success, ..
        } => {
            outcome.insert(
                "expected_success".to_owned(),
                Value::from(expected_success.as_ref().copied().unwrap_or(true)),
            );
        }
        EvidenceCaptureSpec::RegisteredConnectionObservation {
            expected_complete, ..
        } => {
            outcome.insert(
                "expected_complete".to_owned(),
                Value::from(expected_complete.as_ref().copied().unwrap_or(true)),
            );
        }
    }
    outcome
}

/// Strictly validates the complete safe observed-outcome object for its exact
/// evidence-capture source class.
pub fn validate_evidence_capture_observed_outcome(
    capture: &EvidenceCaptureSpec,
    outcome: &JsonObject,
) -> Result<(), String> {
    match capture {
        EvidenceCaptureSpec::VerifiedCommandExecution { .. } => {
            let decoded = decode_capture_outcome::<CommandCaptureObservedOutcome>(outcome)?;
            validate_capture_sha256("stdout_sha256", &decoded.stdout_sha256)?;
            validate_capture_sha256("stderr_sha256", &decoded.stderr_sha256)?;
        }
        EvidenceCaptureSpec::VerifiedToolInvocation { .. } => {
            let decoded = decode_capture_outcome::<ToolCaptureObservedOutcome>(outcome)?;
            validate_capture_sha256("tool_result_sha256", &decoded.tool_result_sha256)?;
        }
        EvidenceCaptureSpec::RegisteredConnectionObservation {
            source_kind: ConnectionObservationSourceKind::GuardEvent,
            ..
        } => {
            let decoded = decode_capture_outcome::<GuardConnectionCaptureObservedOutcome>(outcome)?;
            if !decoded.complete {
                return Err("registered guard observation must be complete".to_owned());
            }
            validate_capture_nonempty("guard_event_kind", &decoded.guard_event_kind)?;
            validate_capture_nonempty("guard_decision", &decoded.guard_decision)?;
            validate_capture_sha256("observation_sha256", &decoded.observation_sha256)?;
        }
        EvidenceCaptureSpec::RegisteredConnectionObservation {
            source_kind: ConnectionObservationSourceKind::SessionWatcher,
            ..
        } => {
            let decoded =
                decode_capture_outcome::<WatcherConnectionCaptureObservedOutcome>(outcome)?;
            if !decoded.complete {
                return Err("registered session-watcher observation must be complete".to_owned());
            }
            if decoded.snapshot_algorithm != WATCH_SNAPSHOT_ALGORITHM {
                return Err(
                    "snapshot_algorithm must identify the canonical session-watch snapshot"
                        .to_owned(),
                );
            }
            validate_capture_sha256("snapshot_digest", &decoded.snapshot_digest)?;
            validate_capture_sha256("observation_sha256", &decoded.observation_sha256)?;
        }
    }
    Ok(())
}

/// Validates a complete observed outcome and evaluates it against the canonical
/// stored expectation for its capture class.
pub fn evidence_capture_observed_outcome_matches_expected(
    capture: &EvidenceCaptureSpec,
    expected: &JsonObject,
    observed: &JsonObject,
) -> Result<bool, String> {
    validate_evidence_capture_expected_outcome(capture, expected)?;
    validate_evidence_capture_observed_outcome(capture, observed)?;
    Ok(match capture {
        EvidenceCaptureSpec::VerifiedCommandExecution { .. } => {
            observed.get("exit_code").and_then(Value::as_i64)
                == expected.get("expected_exit_code").and_then(Value::as_i64)
        }
        EvidenceCaptureSpec::VerifiedToolInvocation { .. } => {
            observed.get("success").and_then(Value::as_bool)
                == expected.get("expected_success").and_then(Value::as_bool)
        }
        EvidenceCaptureSpec::RegisteredConnectionObservation { .. } => {
            observed.get("complete").and_then(Value::as_bool)
                == expected.get("expected_complete").and_then(Value::as_bool)
        }
    })
}

/// Validates the bounded controlled limitation disclosure for a persisted
/// evidence-capture receipt class.
pub fn validate_evidence_capture_limitations(
    capture: &EvidenceCaptureSpec,
    limitations: &[String],
) -> Result<(), String> {
    let expected = match capture {
        EvidenceCaptureSpec::VerifiedCommandExecution { .. } => EVIDENCE_CAPTURE_COMMAND_LIMITATION,
        EvidenceCaptureSpec::VerifiedToolInvocation { .. }
        | EvidenceCaptureSpec::RegisteredConnectionObservation {
            source_kind: ConnectionObservationSourceKind::GuardEvent,
            ..
        } => EVIDENCE_CAPTURE_GUARD_LIMITATION,
        EvidenceCaptureSpec::RegisteredConnectionObservation {
            source_kind: ConnectionObservationSourceKind::SessionWatcher,
            ..
        } => EVIDENCE_CAPTURE_WATCHER_LIMITATION,
    };
    if limitations.len() == 1 && limitations[0] == expected {
        Ok(())
    } else {
        Err(format!(
            "capture-class limitations must contain exactly {expected}"
        ))
    }
}

fn decode_capture_outcome<T: DeserializeOwned>(outcome: &JsonObject) -> Result<T, String> {
    serde_json::from_value(Value::Object(outcome.clone()))
        .map_err(|error| format!("invalid evidence-capture outcome: {error}"))
}

fn validate_capture_sha256(field: &str, value: &str) -> Result<(), String> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(format!(
            "{field} must be lowercase 64-character SHA-256 hex"
        ))
    }
}

fn validate_capture_nonempty(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field} must not be empty"))
    } else {
        Ok(())
    }
}

/// Immutable Core-finalized producer bound one-to-one to a Run observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceProducer {
    pub evidence_producer_id: EvidenceProducerId,
    pub capture_receipt_id: EvidenceCaptureReceiptId,
    pub capture_intent_id: EvidenceCaptureIntentId,
    pub capture_intent_ref: StateRecordRef,
    pub producer_kind: EvidenceProducerKind,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub scope_revision: u64,
    pub baseline_ref: BaselineRef,
    pub target: EvidenceTarget,
    pub input_sha256: String,
    pub result_sha256: String,
    pub expected_outcome: JsonObject,
    pub observed_outcome: JsonObject,
    pub source_refs: Vec<StateRecordRef>,
    pub connection_id: AgentConnectionId,
    pub session_id: RequiredNullable<AgentSessionId>,
    pub guard_installation_id: RequiredNullable<GuardInstallationId>,
    pub guard_event_ids: Vec<GuardEventId>,
    pub watch_observation_refs: Vec<String>,
    pub receipt_artifact_refs: Vec<ArtifactRef>,
    pub complete: bool,
    pub limitations: Vec<String>,
    pub redaction_state: RedactionState,
    pub observed_by_actor_source: ActorSource,
    pub observed_at: UtcTimestamp,
    pub finalized_at: UtcTimestamp,
    pub run_ref: StateRecordRef,
    pub observation_ref: StateRecordRef,
}

/// Evidence coverage summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceSummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_state: Option<EvidenceDisplayState>,
    pub status: EvidenceStatus,
    pub coverage_items: Vec<EvidenceCoverageItem>,
    pub artifact_refs: Vec<ArtifactRef>,
    pub observation_refs: Vec<StateRecordRef>,
    pub updated_by_run_ref: Option<StateRecordRef>,
}

/// Canonical derived evidence gate projection copied across status-like views.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceGateSummary {
    pub state: EvidenceGateState,
}

/// Evidence claim coverage item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCoverageItem {
    pub target: EvidenceTarget,
    pub coverage_state: EvidenceCoverageState,
    pub supporting_run_refs: Vec<StateRecordRef>,
    pub observation_refs: Vec<StateRecordRef>,
    pub supporting_artifact_refs: Vec<ArtifactRef>,
    pub gap_refs: Vec<StateRecordRef>,
}

/// Request-side update for one stable evidence target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCoverageUpdate {
    pub target: EvidenceTarget,
    pub coverage_state: EvidenceCoverageUpdateState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<EvidenceUpdateProvenance>,
    pub supporting_run_refs: Vec<StateRecordRef>,
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
    pub source_refs: Vec<SourceRef>,
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
    pub target: EvidenceTarget,
    pub source_kind: EvidenceSourceKind,
    pub assurance_level: EvidenceAssuranceLevel,
    pub producer_anchor: EvidenceProducerAnchor,
    pub relevance_assessment: EvidenceRelevanceAssessment,
    pub observed_by_actor_source: RequiredNullable<ActorSource>,
    pub tool_name: RequiredNullable<String>,
    pub tool_invocation_id: RequiredNullable<String>,
    pub tool_metadata: JsonObject,
    pub input_refs: Vec<StateRecordRef>,
    pub source_refs: Vec<SourceRef>,
    pub output_artifact_refs: Vec<ArtifactRef>,
    pub limitations: Vec<String>,
    pub observed_at: UtcTimestamp,
    pub recorded_at: UtcTimestamp,
}

/// Core-derived producer record and exact-output binding for one observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceProducerAnchor {
    pub producer_kind: EvidenceProducerKind,
    pub producer_ref: RequiredNullable<StateRecordRef>,
    pub output_artifact_refs: Vec<ArtifactRef>,
    pub verification_basis: RequiredNullable<String>,
}

/// Core-derived target relevance assessment for one observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRelevanceAssessment {
    pub status: EvidenceRelevanceStatus,
    pub assessment_ref: RequiredNullable<StateRecordRef>,
    pub assessed_by_actor_source: RequiredNullable<ActorSource>,
}

/// User-action-resolution projection over exact artifact bytes and one evidence target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionEvidenceObservation {
    pub target: EvidenceTarget,
    pub relevance_status: EvidenceRelevanceStatus,
    pub output_artifact_refs: Vec<ArtifactRef>,
    pub summary: String,
}

/// Persisted authority metadata for one evidence observation row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedEvidenceObservationAuthority {
    pub recorded_by_run_id: RunId,
    pub invocation_verification_basis: String,
    pub producer_anchor: EvidenceProducerAnchor,
    pub relevance_assessment: EvidenceRelevanceAssessment,
}

/// Request-side evidence observation input supplied by `volicord.record_run`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceObservationInput {
    pub target: EvidenceTarget,
    pub source_kind: EvidenceSourceKind,
    pub assurance_level: EvidenceAssuranceLevel,
    pub observed_by_actor_source: RequiredNullable<ActorSource>,
    pub tool_name: RequiredNullable<String>,
    pub tool_invocation_id: RequiredNullable<String>,
    pub tool_metadata: JsonObject,
    pub input_refs: Vec<StateRecordRef>,
    pub source_refs: Vec<SourceRef>,
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
    pub source_write_ticket_ref: StateRecordRef,
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
    pub accepted_by_user_action_resolution_refs: Vec<StateRecordRef>,
    pub missing_reason: RequiredNullable<String>,
}

/// Close-readiness blocker data shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CloseReadinessBlocker {
    pub category: CloseReadinessBlockerCategory,
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_surface: Option<ControlSurfaceSummary>,
    #[serde(default)]
    pub can_resolve_in_chat: bool,
    #[serde(default)]
    pub outside_chat_action_required: bool,
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
    pub evidence_target: RequiredNullable<EvidenceTarget>,
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
    pub evidence_target: RequiredNullable<EvidenceTarget>,
    pub expected_sha256: RequiredNullable<String>,
    pub expected_size_bytes: RequiredNullable<u64>,
    pub redaction_state: RequiredNullable<RedactionState>,
}

/// Maximum Unicode scalar count for a user-authored action-resolution note.
pub const USER_ACTION_NOTE_MAX_CHARS: usize = 1_000;

/// Maximum Unicode scalar count for a user-authored evidence-observation summary.
pub const USER_ACTION_OBSERVATION_SUMMARY_MAX_CHARS: usize = 4_000;

/// Maximum number of target candidates in one observation action.
pub const USER_ACTION_TARGET_CANDIDATE_LIMIT: usize = 32;

/// Maximum number of artifact candidates in one observation action.
pub const USER_ACTION_ARTIFACT_CANDIDATE_LIMIT: usize = 32;

/// Maximum serialized byte size of one closed user-action form or resolution input.
pub const USER_ACTION_FORM_MAX_BYTES: usize = 32 * 1_024;
/// Maximum UTF-8 byte length of one adapter-owned user-channel submission id.
///
/// Canonical adapters use visible ASCII, so this is also the public schema's
/// maximum character length.
pub const CHANNEL_SUBMISSION_ID_MAX_BYTES: usize = 256;

/// Agent-authored draft for one pending user action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionChoiceDraft {
    pub judgment_kind: JudgmentKind,
    pub presentation: JudgmentPresentation,
    pub question: String,
    #[serde(default)]
    pub options: RequiredNullable<Vec<UserActionOptionInput>>,
    pub context: UserActionContext,
    pub affected_refs: Vec<StateRecordRef>,
    #[schemars(required)]
    pub sensitive_action_scope: RequiredNullable<SensitiveActionScope>,
}

/// Agent-authored draft for a bounded evidence observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionEvidenceObservationDraft {
    pub question: String,
    pub context_summary: String,
    pub target_candidates: Vec<EvidenceTarget>,
    pub artifact_candidate_ids: Vec<ArtifactId>,
}

/// Agent-authored draft for one pending user action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action_type", rename_all = "snake_case", deny_unknown_fields)]
pub enum UserActionDraft {
    Choice(Box<UserActionChoiceDraft>),
    EvidenceObservation(UserActionEvidenceObservationDraft),
}

impl UserActionDraft {
    /// Returns the canonical action kind derived from this closed draft.
    pub const fn action_kind(&self) -> UserActionKind {
        match self {
            Self::Choice(choice) => match choice.judgment_kind {
                JudgmentKind::ProductDecision => UserActionKind::ProductDecision,
                JudgmentKind::TechnicalDecision => UserActionKind::TechnicalDecision,
                JudgmentKind::ScopeDecision => UserActionKind::ScopeDecision,
                JudgmentKind::SensitiveApproval => UserActionKind::SensitiveApproval,
                JudgmentKind::FinalAcceptance => UserActionKind::FinalAcceptance,
                JudgmentKind::ResidualRiskAcceptance => UserActionKind::ResidualRiskAcceptance,
                JudgmentKind::Cancellation => UserActionKind::Cancellation,
            },
            Self::EvidenceObservation(_) => UserActionKind::EvidenceObservation,
        }
    }

    /// Returns the user-facing question from either draft shape.
    pub fn question(&self) -> &str {
        match self {
            Self::Choice(choice) => &choice.question,
            Self::EvidenceObservation(observation) => &observation.question,
        }
    }

    /// Validates shared bounded-form invariants before Core planning or Store persistence.
    pub fn validate_bounds(&self) -> Result<(), UserActionShapeError> {
        match self {
            Self::Choice(choice) => {
                if choice
                    .options
                    .as_ref()
                    .is_some_and(|options| options.len() > USER_ACTION_TARGET_CANDIDATE_LIMIT)
                {
                    return Err(UserActionShapeError::new(
                        "action.options",
                        "choice option count exceeds the user-action candidate limit",
                    ));
                }
            }
            Self::EvidenceObservation(observation) => {
                validate_candidate_count(
                    "action.target_candidates",
                    observation.target_candidates.len(),
                    USER_ACTION_TARGET_CANDIDATE_LIMIT,
                )?;
                validate_candidate_count(
                    "action.artifact_candidate_ids",
                    observation.artifact_candidate_ids.len(),
                    USER_ACTION_ARTIFACT_CANDIDATE_LIMIT,
                )?;
            }
        }
        validate_user_action_serialized_size("action", self)
    }
}

/// Canonical Core-owned body stored for one user-action request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionChoiceRequestBody {
    pub judgment_kind: JudgmentKind,
    pub presentation: JudgmentPresentation,
    pub question: String,
    pub options: Vec<UserActionOption>,
    pub context: UserActionContext,
    pub affected_refs: Vec<StateRecordRef>,
    #[schemars(required)]
    pub sensitive_action_scope: RequiredNullable<SensitiveActionScope>,
}

/// Core-owned bounded evidence-observation request body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionEvidenceObservationRequestBody {
    pub question: String,
    pub context_summary: String,
    pub target_candidates: Vec<EvidenceTarget>,
    pub artifact_candidates: Vec<ArtifactRef>,
}

/// Canonical Core-owned body stored for one user-action request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action_type", rename_all = "snake_case", deny_unknown_fields)]
pub enum UserActionRequestBody {
    Choice(Box<UserActionChoiceRequestBody>),
    EvidenceObservation(UserActionEvidenceObservationRequestBody),
}

impl UserActionRequestBody {
    /// Returns the canonical action kind derived from this stored body.
    pub const fn action_kind(&self) -> UserActionKind {
        match self {
            Self::Choice(choice) => match choice.judgment_kind {
                JudgmentKind::ProductDecision => UserActionKind::ProductDecision,
                JudgmentKind::TechnicalDecision => UserActionKind::TechnicalDecision,
                JudgmentKind::ScopeDecision => UserActionKind::ScopeDecision,
                JudgmentKind::SensitiveApproval => UserActionKind::SensitiveApproval,
                JudgmentKind::FinalAcceptance => UserActionKind::FinalAcceptance,
                JudgmentKind::ResidualRiskAcceptance => UserActionKind::ResidualRiskAcceptance,
                JudgmentKind::Cancellation => UserActionKind::Cancellation,
            },
            Self::EvidenceObservation(_) => UserActionKind::EvidenceObservation,
        }
    }

    /// Returns the user-facing question for this closed request body.
    pub fn question(&self) -> &str {
        match self {
            Self::Choice(choice) => &choice.question,
            Self::EvidenceObservation(observation) => &observation.question,
        }
    }

    /// Returns the compact context summary for inbox projection.
    pub fn context_summary(&self) -> &str {
        match self {
            Self::Choice(choice) => &choice.context.summary,
            Self::EvidenceObservation(observation) => &observation.context_summary,
        }
    }

    /// Returns choice-specific affected refs, empty for observation requests.
    pub fn affected_refs(&self) -> &[StateRecordRef] {
        match self {
            Self::Choice(choice) => &choice.affected_refs,
            Self::EvidenceObservation(_) => &[],
        }
    }

    /// Projects this immutable request body into the canonical User Channel form.
    ///
    /// The projection validates the stored body first and copies only the exact
    /// user-presentable candidates and canonical input limits. Current channel
    /// availability and capture-path selection are separate runtime facts.
    pub fn capture_form(&self) -> Result<UserActionInboxForm, UserActionShapeError> {
        self.validate_bounds()?;
        let form = match self {
            Self::Choice(choice) => UserActionInboxForm::Choice {
                choices: choice
                    .options
                    .iter()
                    .map(|option| UserActionInboxChoice {
                        choice_id: option.option_id.clone(),
                        label: option.label.clone(),
                        description: option.description.clone(),
                        consequence: option.consequence.clone(),
                        is_default: option.is_default,
                    })
                    .collect(),
                note_allowed: true,
                note_max_chars: USER_ACTION_NOTE_MAX_CHARS as u64,
            },
            Self::EvidenceObservation(observation) => UserActionInboxForm::EvidenceObservation {
                target_candidates: observation.target_candidates.clone(),
                artifact_candidates: observation.artifact_candidates.clone(),
                relevance_options: vec![
                    EvidenceRelevanceStatus::Supported,
                    EvidenceRelevanceStatus::Contradicted,
                ],
                summary_max_chars: USER_ACTION_OBSERVATION_SUMMARY_MAX_CHARS as u64,
            },
        };
        form.validate_canonical_size()?;
        Ok(form)
    }

    /// Validates shared persisted-form candidate and byte bounds.
    pub fn validate_bounds(&self) -> Result<(), UserActionShapeError> {
        match self {
            Self::Choice(choice) => {
                validate_nonblank_user_action_text("body.question", &choice.question)?;
                validate_nonblank_user_action_text(
                    "body.context.summary",
                    &choice.context.summary,
                )?;
                validate_candidate_count(
                    "body.options",
                    choice.options.len(),
                    USER_ACTION_TARGET_CANDIDATE_LIMIT,
                )?;
                if choice
                    .options
                    .iter()
                    .map(|option| &option.option_id)
                    .collect::<BTreeSet<_>>()
                    .len()
                    != choice.options.len()
                {
                    return Err(UserActionShapeError::new(
                        "body.options",
                        "choice option identifiers must be unique",
                    ));
                }
                if choice
                    .options
                    .iter()
                    .filter(|option| option.is_default)
                    .count()
                    > 1
                {
                    return Err(UserActionShapeError::new(
                        "body.options",
                        "choice options may contain at most one default",
                    ));
                }
                if choice.options.iter().any(|option| {
                    option.label.trim().is_empty()
                        || option.description.trim().is_empty()
                        || option.consequence.trim().is_empty()
                        || option.machine_action.resolution_outcome() != option.resolution_outcome
                }) {
                    return Err(UserActionShapeError::new(
                        "body.options",
                        "choice options must be nonblank and have matching action/outcome authority",
                    ));
                }
            }
            Self::EvidenceObservation(observation) => {
                validate_nonblank_user_action_text("body.question", &observation.question)?;
                validate_nonblank_user_action_text(
                    "body.context_summary",
                    &observation.context_summary,
                )?;
                validate_candidate_count(
                    "body.target_candidates",
                    observation.target_candidates.len(),
                    USER_ACTION_TARGET_CANDIDATE_LIMIT,
                )?;
                validate_candidate_count(
                    "body.artifact_candidates",
                    observation.artifact_candidates.len(),
                    USER_ACTION_ARTIFACT_CANDIDATE_LIMIT,
                )?;
                if observation
                    .target_candidates
                    .iter()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != observation.target_candidates.len()
                {
                    return Err(UserActionShapeError::new(
                        "body.target_candidates",
                        "observation targets must be unique",
                    ));
                }
                if observation
                    .artifact_candidates
                    .iter()
                    .map(|artifact| &artifact.artifact_id)
                    .collect::<BTreeSet<_>>()
                    .len()
                    != observation.artifact_candidates.len()
                {
                    return Err(UserActionShapeError::new(
                        "body.artifact_candidates",
                        "observation artifact identifiers must be unique",
                    ));
                }
            }
        }
        validate_user_action_serialized_size("body", self)
    }
}

/// Shared Core-derived request-basis coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionBasisCoordinates {
    pub task_id: TaskId,
    pub change_unit_id: RequiredNullable<ChangeUnitId>,
    pub scope_revision: u64,
    pub baseline_ref: RequiredNullable<BaselineRef>,
    pub created_at_state_version: u64,
    pub compatibility_status: UserActionBasisStatus,
}

/// Closed Core-derived compatibility basis for one user-action request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionChoiceBasis {
    pub coordinates: UserActionBasisCoordinates,
    pub close_basis_revision: RequiredNullable<u64>,
    pub result_refs: Vec<StateRecordRef>,
    pub residual_risk_ids: Vec<RiskId>,
    pub sensitive_action_scope: RequiredNullable<SensitiveActionScope>,
}

/// Core-derived compatibility basis for an evidence-observation request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionEvidenceObservationBasis {
    pub coordinates: UserActionBasisCoordinates,
    pub target_candidates: Vec<EvidenceTarget>,
    pub artifact_candidates: Vec<ArtifactRef>,
}

/// Closed Core-derived compatibility basis for one user-action request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action_type", rename_all = "snake_case", deny_unknown_fields)]
pub enum UserActionBasis {
    Choice(Box<UserActionChoiceBasis>),
    EvidenceObservation(UserActionEvidenceObservationBasis),
}

impl UserActionBasis {
    /// Returns the stored compatibility status for the request basis.
    pub const fn compatibility_status(&self) -> UserActionBasisStatus {
        match self {
            Self::Choice(choice) => choice.coordinates.compatibility_status,
            Self::EvidenceObservation(observation) => observation.coordinates.compatibility_status,
        }
    }

    /// Returns the shared compatibility coordinates for either action shape.
    pub const fn coordinates(&self) -> &UserActionBasisCoordinates {
        match self {
            Self::Choice(choice) => &choice.coordinates,
            Self::EvidenceObservation(observation) => &observation.coordinates,
        }
    }

    /// Returns the close-basis revision carried by a choice action.
    pub fn close_basis_revision(&self) -> Option<u64> {
        match self {
            Self::Choice(choice) => choice.close_basis_revision.as_ref().copied(),
            Self::EvidenceObservation(_) => None,
        }
    }

    /// Returns choice result refs, empty for observation actions.
    pub fn result_refs(&self) -> &[StateRecordRef] {
        match self {
            Self::Choice(choice) => &choice.result_refs,
            Self::EvidenceObservation(_) => &[],
        }
    }

    /// Returns choice residual-risk ids, empty for observation actions.
    pub fn residual_risk_ids(&self) -> &[RiskId] {
        match self {
            Self::Choice(choice) => &choice.residual_risk_ids,
            Self::EvidenceObservation(_) => &[],
        }
    }

    /// Returns the choice sensitive-action scope when one is bound.
    pub fn sensitive_action_scope(&self) -> Option<&SensitiveActionScope> {
        match self {
            Self::Choice(choice) => choice.sensitive_action_scope.as_ref(),
            Self::EvidenceObservation(_) => None,
        }
    }
}

/// User-owned input for resolving one pending action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "resolution_type",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum UserActionResolutionInput {
    Choice {
        selected_option_id: UserActionOptionId,
        #[serde(default)]
        note: RequiredNullable<String>,
    },
    EvidenceObservation {
        target: EvidenceTarget,
        artifact_ids: Vec<ArtifactId>,
        relevance_status: EvidenceRelevanceStatus,
        summary: String,
    },
}

impl UserActionResolutionInput {
    /// Validates shared user-authored text, candidate, and byte bounds.
    pub fn validate_bounds(&self) -> Result<(), UserActionShapeError> {
        match self {
            Self::Choice { note, .. } => {
                if note
                    .as_ref()
                    .is_some_and(|note| note.chars().count() > USER_ACTION_NOTE_MAX_CHARS)
                {
                    return Err(UserActionShapeError::new(
                        "resolution.note",
                        "note exceeds the user-action character limit",
                    ));
                }
            }
            Self::EvidenceObservation {
                artifact_ids,
                relevance_status,
                summary,
                ..
            } => {
                validate_candidate_count(
                    "resolution.artifact_ids",
                    artifact_ids.len(),
                    USER_ACTION_ARTIFACT_CANDIDATE_LIMIT,
                )?;
                if artifact_ids.iter().collect::<BTreeSet<_>>().len() != artifact_ids.len() {
                    return Err(UserActionShapeError::new(
                        "resolution.artifact_ids",
                        "observation artifact identifiers must be unique",
                    ));
                }
                if !matches!(
                    relevance_status,
                    EvidenceRelevanceStatus::Supported | EvidenceRelevanceStatus::Contradicted
                ) {
                    return Err(UserActionShapeError::new(
                        "resolution.relevance_status",
                        "observation relevance must be supported or contradicted",
                    ));
                }
                if summary.trim().is_empty()
                    || summary.chars().count() > USER_ACTION_OBSERVATION_SUMMARY_MAX_CHARS
                {
                    return Err(UserActionShapeError::new(
                        "resolution.summary",
                        "summary exceeds the user-action character limit",
                    ));
                }
            }
        }
        validate_user_action_serialized_size("resolution", self)
    }
}

/// Canonical Core-derived resolution body for one user-owned action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "resolution_type",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum UserActionResolutionBody {
    Choice {
        selected_option_id: UserActionOptionId,
        machine_action: UserActionOptionAction,
        resolution_outcome: JudgmentResolutionOutcome,
        note: RequiredNullable<String>,
        accepted_risk_ids: Vec<RiskId>,
    },
    EvidenceObservation {
        observation: UserActionEvidenceObservation,
    },
}

impl UserActionResolutionBody {
    /// Validates shared persisted resolution bounds and action/outcome agreement.
    pub fn validate(&self) -> Result<(), UserActionShapeError> {
        match self {
            Self::Choice {
                machine_action,
                resolution_outcome,
                note,
                ..
            } => {
                if machine_action.resolution_outcome() != *resolution_outcome {
                    return Err(UserActionShapeError::new(
                        "resolution.resolution_outcome",
                        "choice resolution outcome must match its machine action",
                    ));
                }
                if note
                    .as_ref()
                    .is_some_and(|note| note.chars().count() > USER_ACTION_NOTE_MAX_CHARS)
                {
                    return Err(UserActionShapeError::new(
                        "resolution.note",
                        "note exceeds the user-action character limit",
                    ));
                }
            }
            Self::EvidenceObservation { observation } => {
                validate_candidate_count(
                    "resolution.observation.output_artifact_refs",
                    observation.output_artifact_refs.len(),
                    USER_ACTION_ARTIFACT_CANDIDATE_LIMIT,
                )?;
                if observation
                    .output_artifact_refs
                    .iter()
                    .map(|artifact| &artifact.artifact_id)
                    .collect::<BTreeSet<_>>()
                    .len()
                    != observation.output_artifact_refs.len()
                {
                    return Err(UserActionShapeError::new(
                        "resolution.observation.output_artifact_refs",
                        "observation artifact identifiers must be unique",
                    ));
                }
                if !matches!(
                    observation.relevance_status,
                    EvidenceRelevanceStatus::Supported | EvidenceRelevanceStatus::Contradicted
                ) {
                    return Err(UserActionShapeError::new(
                        "resolution.observation.relevance_status",
                        "observation relevance must be supported or contradicted",
                    ));
                }
                if observation.summary.trim().is_empty()
                    || observation.summary.chars().count()
                        > USER_ACTION_OBSERVATION_SUMMARY_MAX_CHARS
                {
                    return Err(UserActionShapeError::new(
                        "resolution.observation.summary",
                        "summary exceeds the user-action character limit",
                    ));
                }
            }
        }
        validate_user_action_serialized_size("resolution", self)
    }
}

/// Durable user-action request plus its current effective status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionRequest {
    pub user_action_request_id: UserActionRequestId,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub change_unit_id: RequiredNullable<ChangeUnitId>,
    pub action_kind: UserActionKind,
    pub status: UserActionStatus,
    pub body: UserActionRequestBody,
    pub basis: UserActionBasis,
    pub required_for: Vec<UserActionRequiredFor>,
    pub user_action_resolution_ref: RequiredNullable<StateRecordRef>,
    pub expires_at: RequiredNullable<UtcTimestamp>,
    pub created_at: UtcTimestamp,
}

/// Immutable user-owned resolution of one action request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionResolution {
    pub user_action_resolution_id: UserActionResolutionId,
    pub user_action_request_id: UserActionRequestId,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub action_kind: UserActionKind,
    pub body: UserActionResolutionBody,
    pub resolved_by_actor_source: ActorSource,
    pub resolved_verification_basis: String,
    pub resolved_assurance_level: String,
    pub channel_kind: UserActionChannelKind,
    #[schemars(
        length(min = 1, max = "CHANNEL_SUBMISSION_ID_MAX_BYTES"),
        regex(pattern = "^[!-~]+$")
    )]
    pub channel_submission_id: String,
    pub resolved_at: UtcTimestamp,
}

/// Stored request JSON shape for `user_action_requests.request_json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedUserActionRequest {
    pub body: UserActionRequestBody,
    pub required_for: Vec<UserActionRequiredFor>,
    pub expires_at: RequiredNullable<UtcTimestamp>,
}

/// Stored resolution JSON shape for `user_action_resolutions.resolution_json`.
pub type PersistedUserActionResolution = UserActionResolutionBody;

/// User-facing inbox item for one actionable request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionInboxItem {
    pub user_action_request_id: UserActionRequestId,
    pub request_ref: StateRecordRef,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub change_unit_id: RequiredNullable<ChangeUnitId>,
    pub action_kind: UserActionKind,
    pub question: String,
    pub context_summary: String,
    pub form: UserActionInboxForm,
    pub required: bool,
    pub requirement_status: String,
    pub required_for: Vec<UserActionRequiredFor>,
    pub status: UserActionStatus,
    pub answer_path_availability: UserChannelAvailability,
    pub preferred_capture_path: RequiredNullable<UserActionCapturePath>,
    pub fallbacks: Vec<UserActionCapturePath>,
    pub expires_at: RequiredNullable<UtcTimestamp>,
}

/// Closed user-facing form derived from the stored request body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "form_type", rename_all = "snake_case", deny_unknown_fields)]
pub enum UserActionInboxForm {
    Choice {
        choices: Vec<UserActionInboxChoice>,
        note_allowed: bool,
        note_max_chars: u64,
    },
    EvidenceObservation {
        target_candidates: Vec<EvidenceTarget>,
        artifact_candidates: Vec<ArtifactRef>,
        relevance_options: Vec<EvidenceRelevanceStatus>,
        summary_max_chars: u64,
    },
}

impl UserActionInboxForm {
    /// Validates the canonical serialized form size shared by every User Channel.
    pub fn validate_canonical_size(&self) -> Result<(), UserActionShapeError> {
        validate_user_action_serialized_size("form", self)
    }
}

/// Current availability summary for supported User Channel answer paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserChannelAvailability {
    pub paths: Vec<UserChannelPathAvailability>,
    pub recommended_path_kind: RequiredNullable<String>,
    pub recommended_path_label: RequiredNullable<String>,
    pub recommendation: RequiredNullable<String>,
}

/// Availability entry for one User Channel answer path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserChannelPathAvailability {
    pub kind: String,
    pub label: String,
    pub available: bool,
    pub status: String,
    pub capture_basis: RequiredNullable<String>,
    pub detail: RequiredNullable<String>,
}

/// User-facing answer choice for a choice-action inbox item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionInboxChoice {
    pub choice_id: UserActionOptionId,
    pub label: String,
    pub description: String,
    pub consequence: String,
    pub is_default: bool,
}

/// User-facing capture path for resolving one user action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionCapturePath {
    pub kind: String,
    pub label: String,
    pub available: bool,
    pub command: RequiredNullable<String>,
    pub url: RequiredNullable<String>,
    pub capture_basis: RequiredNullable<String>,
    pub expires_at: RequiredNullable<UtcTimestamp>,
    pub detail: RequiredNullable<String>,
}

/// Caller-authored request input for a non-authority choice option.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionOptionInput {
    pub option_id: UserActionOptionId,
    pub label: String,
    pub description: String,
    pub consequence: String,
    pub is_default: bool,
}

/// Current Core-owned choice option.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionOption {
    pub option_id: UserActionOptionId,
    pub label: String,
    pub description: String,
    pub consequence: String,
    pub machine_action: UserActionOptionAction,
    pub resolution_outcome: JudgmentResolutionOutcome,
    pub is_default: bool,
}

/// User-action choice context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserActionContext {
    pub summary: String,
    pub related_refs: Vec<StateRecordRef>,
    pub artifact_refs: Vec<ArtifactRef>,
    pub visible_risks: Vec<AcceptedRiskInput>,
    pub constraints: Vec<String>,
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

/// Derives the effective request status from immutable resolution and current time facts.
///
/// Returns `None` when the supplied current time precedes the immutable request creation time.
pub fn effective_user_action_status(
    basis_status: UserActionBasisStatus,
    created_at: &UtcTimestamp,
    expires_at: Option<&UtcTimestamp>,
    has_resolution: bool,
    now: &UtcTimestamp,
) -> Option<UserActionStatus> {
    if now < created_at {
        return None;
    }
    Some(match basis_status {
        UserActionBasisStatus::Stale => UserActionStatus::Stale,
        UserActionBasisStatus::Superseded => UserActionStatus::Superseded,
        UserActionBasisStatus::Current if has_resolution => UserActionStatus::Resolved,
        UserActionBasisStatus::Current
            if expires_at.is_some_and(|expires_at| now >= expires_at) =>
        {
            UserActionStatus::Expired
        }
        UserActionBasisStatus::Current => UserActionStatus::Pending,
    })
}

/// Shared typed user-action shape-validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionShapeError {
    field: &'static str,
    message: &'static str,
}

impl UserActionShapeError {
    /// Creates one bounded user-action shape failure.
    pub const fn new(field: &'static str, message: &'static str) -> Self {
        Self { field, message }
    }

    /// Returns the stable logical field path.
    pub const fn field(&self) -> &'static str {
        self.field
    }

    /// Returns the stable validation message.
    pub const fn message(&self) -> &'static str {
        self.message
    }
}

impl fmt::Display for UserActionShapeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for UserActionShapeError {}

/// Validates the closed adapter-owned shape of a channel submission id.
pub fn validate_channel_submission_id(value: &str) -> Result<(), UserActionShapeError> {
    if value.is_empty()
        || value.len() > CHANNEL_SUBMISSION_ID_MAX_BYTES
        || !value.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
    {
        return Err(UserActionShapeError::new(
            "channel_submission_id",
            "channel submission id must be 1..=256 bytes of visible ASCII",
        ));
    }
    Ok(())
}

fn validate_candidate_count(
    field: &'static str,
    count: usize,
    limit: usize,
) -> Result<(), UserActionShapeError> {
    if count == 0 {
        return Err(UserActionShapeError::new(
            field,
            "candidate selection must not be empty",
        ));
    }
    if count > limit {
        return Err(UserActionShapeError::new(
            field,
            "candidate selection exceeds the user-action limit",
        ));
    }
    Ok(())
}

fn validate_nonblank_user_action_text(
    field: &'static str,
    value: &str,
) -> Result<(), UserActionShapeError> {
    if value.trim().is_empty() {
        Err(UserActionShapeError::new(
            field,
            "user-action text must not be blank",
        ))
    } else {
        Ok(())
    }
}

fn validate_user_action_serialized_size<T: Serialize>(
    field: &'static str,
    value: &T,
) -> Result<(), UserActionShapeError> {
    let size = crate::canonical_json_size_bytes(value)
        .map_err(|_| UserActionShapeError::new(field, "user-action JSON cannot be serialized"))?;
    if size > USER_ACTION_FORM_MAX_BYTES {
        return Err(UserActionShapeError::new(
            field,
            "user-action JSON exceeds the canonical byte limit",
        ));
    }
    Ok(())
}
