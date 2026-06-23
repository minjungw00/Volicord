use std::{error::Error, fmt, str::FromStr};

use chrono::{DateTime, SecondsFormat, Utc};
use schemars::{
    gen::SchemaGenerator,
    schema::{InstanceType, Schema, SchemaObject, SingleOrVec},
    JsonSchema,
};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// Parsed RFC 3339 timestamp normalized to a UTC instant.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcTimestamp(DateTime<Utc>);

impl UtcTimestamp {
    /// Parses an RFC 3339 timestamp with an explicit offset and normalizes it to UTC.
    pub fn parse(raw: &str) -> Result<Self, UtcTimestampParseError> {
        DateTime::parse_from_rfc3339(raw)
            .map(|timestamp| Self(timestamp.with_timezone(&Utc)))
            .map_err(|_| UtcTimestampParseError)
    }

    /// Wraps an already-UTC timestamp.
    pub fn from_datetime(timestamp: DateTime<Utc>) -> Self {
        Self(timestamp)
    }

    /// Returns the UTC instant.
    pub fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }

    /// Consumes the wrapper and returns the UTC instant.
    pub fn into_datetime(self) -> DateTime<Utc> {
        self.0
    }

    /// Returns the deterministic RFC 3339 UTC wire representation.
    pub fn to_canonical_string(&self) -> String {
        self.0.to_rfc3339_opts(SecondsFormat::AutoSi, true)
    }
}

impl fmt::Display for UtcTimestamp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_canonical_string())
    }
}

impl From<DateTime<Utc>> for UtcTimestamp {
    fn from(timestamp: DateTime<Utc>) -> Self {
        Self::from_datetime(timestamp)
    }
}

impl FromStr for UtcTimestamp {
    type Err = UtcTimestampParseError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Self::parse(raw)
    }
}

impl Serialize for UtcTimestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_canonical_string())
    }
}

impl<'de> Deserialize<'de> for UtcTimestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).map_err(de::Error::custom)
    }
}

impl JsonSchema for UtcTimestamp {
    fn schema_name() -> String {
        "UtcTimestamp".to_owned()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        Schema::Object(SchemaObject {
            instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
            format: Some("date-time".to_owned()),
            ..Default::default()
        })
    }
}

/// Error returned when a public or persisted timestamp is not RFC 3339.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcTimestampParseError;

impl fmt::Display for UtcTimestampParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("timestamp must be a valid RFC 3339 string with an explicit offset")
    }
}

impl Error for UtcTimestampParseError {}

/// Supported public Harness method names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum MethodName {
    #[serde(rename = "harness.intake")]
    Intake,
    #[serde(rename = "harness.update_scope")]
    UpdateScope,
    #[serde(rename = "harness.status")]
    Status,
    #[serde(rename = "harness.prepare_write")]
    PrepareWrite,
    #[serde(rename = "harness.stage_artifact")]
    StageArtifact,
    #[serde(rename = "harness.record_run")]
    RecordRun,
    #[serde(rename = "harness.request_user_judgment")]
    RequestUserJudgment,
    #[serde(rename = "harness.record_user_judgment")]
    RecordUserJudgment,
    #[serde(rename = "harness.close_task")]
    CloseTask,
}

impl MethodName {
    /// Returns the public method-name value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Intake => "harness.intake",
            Self::UpdateScope => "harness.update_scope",
            Self::Status => "harness.status",
            Self::PrepareWrite => "harness.prepare_write",
            Self::StageArtifact => "harness.stage_artifact",
            Self::RecordRun => "harness.record_run",
            Self::RequestUserJudgment => "harness.request_user_judgment",
            Self::RecordUserJudgment => "harness.record_user_judgment",
            Self::CloseTask => "harness.close_task",
        }
    }
}

/// Controlled API actor kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    Agent,
    User,
}

/// Controlled registered surface role for actor-provenance derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceInteractionRole {
    Agent,
    UserInteraction,
}

impl SurfaceInteractionRole {
    /// Returns the stable storage value name for this interaction role.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::UserInteraction => "user_interaction",
        }
    }
}

/// Controlled next-action category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NextActionKind {
    UpdateScope,
    PrepareWrite,
    StageArtifact,
    RecordRun,
    RequestUserJudgment,
    RecordUserJudgment,
    CloseTask,
}

/// Common API response branch metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResponseKind {
    Result,
    Rejected,
    DryRun,
}

/// Common API effect metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    ReadOnly,
    CoreCommitted,
    StagingCreated,
    NoEffect,
}

/// Request-level API compatibility access class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessClass {
    ReadStatus,
    CoreMutation,
    WriteAuthorization,
    RunRecording,
    ArtifactRegistration,
    ArtifactRead,
}

impl AccessClass {
    /// Returns the stable public value name for this access class.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadStatus => "read_status",
            Self::CoreMutation => "core_mutation",
            Self::WriteAuthorization => "write_authorization",
            Self::RunRecording => "run_recording",
            Self::ArtifactRegistration => "artifact_registration",
            Self::ArtifactRead => "artifact_read",
        }
    }
}

/// Explicit grants expanded by the local baseline-workflow registration profile.
pub const BASELINE_WORKFLOW_ACCESS_CLASSES: [AccessClass; 5] = [
    AccessClass::ReadStatus,
    AccessClass::CoreMutation,
    AccessClass::WriteAuthorization,
    AccessClass::ArtifactRegistration,
    AccessClass::RunRecording,
];

/// Controlled registration-basis value for local administrative registration.
pub const VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION: &str = "local_admin_registration";

/// Controlled adapter-binding basis value for MCP stdio sessions.
pub const VERIFICATION_BASIS_MCP_STDIO_SURFACE_BINDING: &str = "mcp_stdio_surface_binding";

/// Controlled adapter-binding basis value for direct CLI invocation.
pub const VERIFICATION_BASIS_CLI_DIRECT_SURFACE_BINDING: &str = "cli_direct_surface_binding";

/// Controlled binding basis value for repository tests and fixtures.
pub const VERIFICATION_BASIS_TEST_FIXTURE_BINDING: &str = "test_fixture_binding";

/// Baseline actor assurance level for cooperative registered-surface provenance.
pub const ACTOR_ASSURANCE_REGISTERED_SURFACE_COOPERATIVE: &str = "registered_surface_cooperative";

/// State reference discriminator values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StateRecordKind {
    ProjectState,
    Task,
    ChangeUnit,
    WriteAuthorization,
    UserJudgment,
    Run,
    EvidenceSummary,
    Artifact,
    Blocker,
    TaskEvent,
    LocalSurfaceRegistration,
}

/// Concrete output Task modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskMode {
    Advisor,
    Direct,
    Work,
}

/// Intake input mode, including the input-only `auto` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequestedMode {
    Advisor,
    Direct,
    Work,
    Auto,
}

/// Task lifecycle phase values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskLifecyclePhase {
    Shaping,
    Ready,
    Executing,
    WaitingUser,
    Blocked,
    Completed,
    Cancelled,
    Superseded,
}

/// Close-state values returned by close-task result paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CloseState {
    Ready,
    Blocked,
    Closed,
    Cancelled,
    Superseded,
}

/// Status close-state values, including `none` for no current close state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatusCloseState {
    Ready,
    Blocked,
    Closed,
    Cancelled,
    Superseded,
    None,
}

/// Task close-reason values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CloseReason {
    None,
    CompletedSelfChecked,
    CompletedWithRiskAccepted,
    Cancelled,
    Superseded,
}

/// Task result values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskResult {
    None,
    AdviceOnly,
    Completed,
    Cancelled,
    Superseded,
}

/// Intake resume-policy values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResumePolicy {
    ResumeActive,
    CreateNew,
    SupersedeActive,
    RejectIfActive,
}

/// Update-scope Change Unit operation values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChangeUnitOperation {
    KeepCurrent,
    CreateCurrent,
    ReplaceCurrent,
}

/// Close-task intent values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CloseIntent {
    Check,
    Complete,
    Cancel,
    Supersede,
}

/// Prepare-write decision values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrepareWriteDecision {
    Allowed,
    Blocked,
    ApprovalRequired,
    DecisionRequired,
}

/// Prepare-write authorization-effect values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationEffect {
    None,
    WouldCreate,
    Created,
}

/// Write Authorization status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteAuthorizationStatus {
    Active,
    Consumed,
    Expired,
    Stale,
    Revoked,
}

/// Run kind values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
    ShapingUpdate,
    Implementation,
    Direct,
}

/// Dry-run planned blocker source values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlannedBlockerSourceKind {
    WriteDecision,
    CloseReadiness,
}

/// Write-decision reason category values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteDecisionCategory {
    Scope,
    UserJudgment,
    SensitiveApproval,
    WriteCompatibility,
    Baseline,
    SurfaceCapability,
}

/// Close-readiness blocker category values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CloseReadinessBlockerCategory {
    Task,
    OpenRun,
    Scope,
    UserJudgment,
    SensitiveApproval,
    WriteCompatibility,
    Baseline,
    SurfaceCapability,
    Evidence,
    ArtifactAvailability,
    FinalAcceptance,
    ResidualRiskVisibility,
    ResidualRiskAcceptance,
    Recovery,
}

/// Evidence summary status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Unknown,
    Insufficient,
    Sufficient,
    Blocked,
}

/// Evidence coverage item state values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceCoverageState {
    Unsupported,
    Partial,
    Supported,
    NotApplicable,
    Stale,
    Blocked,
}

/// Validator status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorStatus {
    Passed,
    Warning,
    Failed,
    Blocked,
}

/// Validator severity values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorSeverity {
    Info,
    Warning,
    Error,
    Blocking,
}

/// Guarantee-display level values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuaranteeLevel {
    Cooperative,
    Detective,
}

/// Controlled source value for a project enforcement profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectEnforcementProfileSource {
    BaselineScope,
}

/// Controlled active-state value for a project enforcement profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectEnforcementProfileStatus {
    Active,
}

/// Controlled enabled enforcement mechanisms supported by the baseline build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnabledEnforcementMechanism {}

/// Artifact input source values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactInputSourceKind {
    StagedArtifact,
    ExistingArtifact,
}

/// Artifact redaction-state values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RedactionState {
    None,
    Redacted,
    SecretOmitted,
    Blocked,
}

/// Artifact availability display values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactAvailability {
    Available,
    Unavailable,
    Missing,
    IntegrityFailed,
    Blocked,
    Unusable,
}

/// Artifact integrity fact classification values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactIntegrityStatus {
    Verified,
    Corrupt,
}

/// Judgment-kind values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JudgmentKind {
    ProductDecision,
    TechnicalDecision,
    ScopeDecision,
    SensitiveApproval,
    FinalAcceptance,
    ResidualRiskAcceptance,
    Cancellation,
}

/// Judgment presentation values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JudgmentPresentation {
    Short,
}

/// Judgment required-for values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JudgmentRequiredFor {
    ScopeUpdate,
    PrepareWrite,
    RecordRun,
    CloseComplete,
    CloseCancel,
    CloseSupersede,
    Informational,
}

/// User judgment status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserJudgmentStatus {
    Pending,
    Resolved,
    Stale,
    Superseded,
    Expired,
}

/// User judgment resolution outcome values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JudgmentResolutionOutcome {
    Accepted,
    Rejected,
    Deferred,
}

/// Core-owned machine action for current user judgment options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserJudgmentOptionAction {
    Accept,
    Reject,
    Defer,
}

impl UserJudgmentOptionAction {
    /// Returns the resolution outcome owned by this option action.
    pub const fn resolution_outcome(self) -> JudgmentResolutionOutcome {
        match self {
            Self::Accept => JudgmentResolutionOutcome::Accepted,
            Self::Reject => JudgmentResolutionOutcome::Rejected,
            Self::Defer => JudgmentResolutionOutcome::Deferred,
        }
    }
}

/// Judgment-basis compatibility status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JudgmentBasisCompatibilityStatus {
    Current,
    Stale,
    Superseded,
}

/// Public API error code values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ValidationFailed,
    StateVersionConflict,
    McpUnavailable,
    LocalAccessMismatch,
    NoActiveTask,
    NoActiveChangeUnit,
    BaselineStale,
    ScopeRequired,
    ScopeViolation,
    WriteAuthorizationRequired,
    WriteAuthorizationInvalid,
    ApprovalDenied,
    ApprovalExpired,
    ApprovalRequired,
    DecisionUnresolved,
    AutonomyBoundaryExceeded,
    DecisionRequired,
    CapabilityInsufficient,
    EvidenceInsufficient,
    ResidualRiskNotVisible,
    AcceptanceRequired,
    ProjectionStale,
    ArtifactMissing,
    ValidatorFailed,
}
