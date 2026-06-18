use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    NextAction,
    Write,
    Run,
    Close,
    Acceptance,
    Risk,
}

/// User judgment status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserJudgmentStatus {
    Pending,
    Resolved,
    Rejected,
    Deferred,
    Blocked,
    Stale,
    Superseded,
    Incompatible,
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
