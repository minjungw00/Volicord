use std::{error::Error, fmt, str::FromStr};

use chrono::{DateTime, Datelike, Duration, SecondsFormat, Utc};
use schemars::{
    gen::SchemaGenerator,
    schema::{InstanceType, Schema, SchemaObject, SingleOrVec},
    JsonSchema,
};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::ids::{AgentConnectionId, TaskId};

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

    /// Confirms this instant has a canonical RFC 3339 UTC representation with
    /// the four-digit year shape used by public and durable timestamps.
    pub fn ensure_canonical_rfc3339_representable(&self) -> Result<(), UtcTimestampRangeError> {
        if !(0..=9999).contains(&self.0.year()) {
            return Err(UtcTimestampRangeError);
        }
        let canonical = self.to_canonical_string();
        let reparsed = Self::parse(&canonical).map_err(|_| UtcTimestampRangeError)?;
        if reparsed == *self && reparsed.to_canonical_string() == canonical {
            Ok(())
        } else {
            Err(UtcTimestampRangeError)
        }
    }

    /// Adds a signed duration without overflowing Chrono or leaving the
    /// canonical four-digit RFC 3339 timestamp range.
    pub fn checked_add(&self, duration: Duration) -> Result<Self, UtcTimestampRangeError> {
        let timestamp = self
            .0
            .checked_add_signed(duration)
            .map(Self::from_datetime)
            .ok_or(UtcTimestampRangeError)?;
        timestamp.ensure_canonical_rfc3339_representable()?;
        Ok(timestamp)
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

/// Error returned when timestamp arithmetic cannot produce the canonical
/// four-digit RFC 3339 representation required by public and durable values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcTimestampRangeError;

impl fmt::Display for UtcTimestampRangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "timestamp must be representable as canonical RFC 3339 UTC with a four-digit year",
        )
    }
}

impl Error for UtcTimestampRangeError {}

/// Supported public Volicord method names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum MethodName {
    #[serde(rename = "volicord.intake")]
    Intake,
    #[serde(rename = "volicord.update_scope")]
    UpdateScope,
    #[serde(rename = "volicord.record_shaping_checkpoint")]
    RecordShapingCheckpoint,
    #[serde(rename = "volicord.finalize_advice")]
    FinalizeAdvice,
    #[serde(rename = "volicord.advance_task")]
    AdvanceTask,
    #[serde(rename = "volicord.status")]
    Status,
    #[serde(rename = "volicord.get_operation_result")]
    GetOperationResult,
    #[serde(rename = "volicord.check_close")]
    CheckClose,
    #[serde(rename = "volicord.prepare_evidence_capture")]
    PrepareEvidenceCapture,
    #[serde(rename = "volicord.prepare_write")]
    PrepareWrite,
    #[serde(rename = "volicord.stage_artifact")]
    StageArtifact,
    #[serde(rename = "volicord.record_run")]
    RecordRun,
    #[serde(rename = "volicord.request_user_action")]
    RequestUserAction,
    #[serde(rename = "volicord.resolve_user_action")]
    ResolveUserAction,
    #[serde(rename = "volicord.reconcile_changes")]
    ReconcileChanges,
    #[serde(rename = "volicord.close_task")]
    CloseTask,
}

impl MethodName {
    /// Complete current public method catalog in declaration order.
    pub const ALL: [Self; 16] = [
        Self::Intake,
        Self::UpdateScope,
        Self::RecordShapingCheckpoint,
        Self::FinalizeAdvice,
        Self::AdvanceTask,
        Self::Status,
        Self::GetOperationResult,
        Self::CheckClose,
        Self::PrepareEvidenceCapture,
        Self::PrepareWrite,
        Self::StageArtifact,
        Self::RecordRun,
        Self::RequestUserAction,
        Self::ResolveUserAction,
        Self::ReconcileChanges,
        Self::CloseTask,
    ];

    /// Returns the public method-name value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Intake => "volicord.intake",
            Self::UpdateScope => "volicord.update_scope",
            Self::RecordShapingCheckpoint => "volicord.record_shaping_checkpoint",
            Self::FinalizeAdvice => "volicord.finalize_advice",
            Self::AdvanceTask => "volicord.advance_task",
            Self::Status => "volicord.status",
            Self::GetOperationResult => "volicord.get_operation_result",
            Self::CheckClose => "volicord.check_close",
            Self::PrepareEvidenceCapture => "volicord.prepare_evidence_capture",
            Self::PrepareWrite => "volicord.prepare_write",
            Self::StageArtifact => "volicord.stage_artifact",
            Self::RecordRun => "volicord.record_run",
            Self::RequestUserAction => "volicord.request_user_action",
            Self::ResolveUserAction => "volicord.resolve_user_action",
            Self::ReconcileChanges => "volicord.reconcile_changes",
            Self::CloseTask => "volicord.close_task",
        }
    }
}

/// Durable actor provenance used after adapter-boundary derivation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActorSource {
    AgentConnection(AgentConnectionId),
    LocalUser,
    System,
}

impl ActorSource {
    /// Creates actor provenance for a bound Agent Connection.
    pub fn agent_connection(connection_id: impl Into<AgentConnectionId>) -> Self {
        Self::AgentConnection(connection_id.into())
    }

    /// Returns the stable string representation.
    pub fn to_canonical_string(&self) -> String {
        match self {
            Self::AgentConnection(connection_id) => {
                format!("agent_connection:{}", connection_id.as_str())
            }
            Self::LocalUser => "local_user".to_owned(),
            Self::System => "system".to_owned(),
        }
    }

    /// Returns the bound Agent Connection id when this source names one.
    pub fn agent_connection_id(&self) -> Option<&AgentConnectionId> {
        match self {
            Self::AgentConnection(connection_id) => Some(connection_id),
            Self::LocalUser | Self::System => None,
        }
    }
}

impl fmt::Display for ActorSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_canonical_string())
    }
}

impl FromStr for ActorSource {
    type Err = ActorSourceParseError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if raw == "local_user" {
            return Ok(Self::LocalUser);
        }
        if raw == "system" {
            return Ok(Self::System);
        }
        let Some(connection_id) = raw.strip_prefix("agent_connection:") else {
            return Err(ActorSourceParseError);
        };
        if connection_id.is_empty() {
            return Err(ActorSourceParseError);
        }
        Ok(Self::AgentConnection(AgentConnectionId::new(connection_id)))
    }
}

impl Serialize for ActorSource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_canonical_string())
    }
}

impl<'de> Deserialize<'de> for ActorSource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::from_str(&raw).map_err(de::Error::custom)
    }
}

impl JsonSchema for ActorSource {
    fn schema_name() -> String {
        "ActorSource".to_owned()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        Schema::Object(SchemaObject {
            instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
            ..Default::default()
        })
    }
}

/// Error returned when an `actor_source` value is not supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorSourceParseError;

impl fmt::Display for ActorSourceParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "actor_source must be local_user, system, or agent_connection:<connection_id>",
        )
    }
}

impl Error for ActorSourceParseError {}

/// Controlled next-action category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NextActionKind {
    UpdateScope,
    FinalizeAdvice,
    PrepareWrite,
    StageArtifact,
    RecordRun,
    RequestUserAction,
    ResolveUserAction,
    ReconcileChanges,
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

/// Internal API operation category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OperationCategory {
    Read,
    AgentWorkflow,
    UserOnly,
    AdminLocal,
    LocalRecovery,
}

impl OperationCategory {
    /// Returns the stable value name for this operation category.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::AgentWorkflow => "agent_workflow",
            Self::UserOnly => "user_only",
            Self::AdminLocal => "admin_local",
            Self::LocalRecovery => "local_recovery",
        }
    }
}

/// Agent Connection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentConnectionMode {
    ReadOnly,
    Workflow,
}

impl AgentConnectionMode {
    /// Returns the stable value name for this Agent Connection mode.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::Workflow => "workflow",
        }
    }

    /// Returns true when this mode can dispatch the supplied category.
    pub fn allows_operation_category(self, category: OperationCategory) -> bool {
        self.operation_categories().contains(&category)
    }

    /// Returns operation categories available to this mode through an Agent Connection.
    pub const fn operation_categories(self) -> &'static [OperationCategory] {
        match self {
            Self::ReadOnly => &READ_ONLY_OPERATION_CATEGORIES,
            Self::Workflow => &WORKFLOW_OPERATION_CATEGORIES,
        }
    }
}

/// Operation categories available to a read-only Agent Connection.
pub const READ_ONLY_OPERATION_CATEGORIES: [OperationCategory; 1] = [OperationCategory::Read];

/// Operation categories available to a workflow Agent Connection.
pub const WORKFLOW_OPERATION_CATEGORIES: [OperationCategory; 2] =
    [OperationCategory::Read, OperationCategory::AgentWorkflow];

/// MCP-visible status detail levels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatusDetailLevel {
    Summary,
    #[default]
    Workflow,
    Full,
}

/// Controlled registration-basis value for local administrative registration.
pub const VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION: &str = "local_admin_registration";

/// Baseline actor assurance level for cooperative Agent Connection provenance.
pub const ACTOR_ASSURANCE_AGENT_CONNECTION_COOPERATIVE: &str = "agent_connection_cooperative";

/// Host family accepted by the current Agent Connection contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostKind {
    Codex,
}

impl HostKind {
    /// Returns the stable host-kind string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Codex => "codex",
        }
    }

    /// Returns the stable host-kind string as an owned value.
    pub fn to_canonical_string(&self) -> String {
        self.as_str().to_owned()
    }
}

impl fmt::Display for HostKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for HostKind {
    type Err = HostKindParseError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "codex" => Ok(Self::Codex),
            _ => Err(HostKindParseError),
        }
    }
}

impl Serialize for HostKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for HostKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::from_str(&raw).map_err(de::Error::custom)
    }
}

impl JsonSchema for HostKind {
    fn schema_name() -> String {
        "HostKind".to_owned()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        Schema::Object(SchemaObject {
            instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
            ..Default::default()
        })
    }
}

/// Error returned when a `host_kind` value is not usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostKindParseError;

impl fmt::Display for HostKindParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("host_kind must be `codex`")
    }
}

impl Error for HostKindParseError {}

/// Public integration profile selected for host integration.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationProfile {
    Record,
}

impl IntegrationProfile {
    /// Returns the stable value name for this integration profile.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Record => "record",
        }
    }
}

/// Cooperative host-hook decision recorded for a host-observed operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardDecision {
    Allow,
    Deny,
    Warn,
    InjectContext,
}

impl GuardDecision {
    /// Returns the stable value name for this host-hook decision.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::Warn => "warn",
            Self::InjectContext => "inject_context",
        }
    }
}

/// Guard hook phase owned by the current installation manifest and observation contract.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GuardHookPhase {
    PreTool,
    PostTool,
    PromptCapture,
}

impl GuardHookPhase {
    /// All hook phases required by the current Guard contract.
    pub const REQUIRED: [Self; 3] = [Self::PreTool, Self::PostTool, Self::PromptCapture];

    /// Returns the stable stored and policy key for this phase.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PreTool => "pre_tool",
            Self::PostTool => "post_tool",
            Self::PromptCapture => "prompt_capture",
        }
    }

    /// Returns the exact internal command name for this phase.
    pub const fn command_name(self) -> &'static str {
        match self {
            Self::PreTool => "pre-tool",
            Self::PostTool => "post-tool",
            Self::PromptCapture => "prompt-capture",
        }
    }
}

impl FromStr for GuardHookPhase {
    type Err = GuardHookPhaseParseError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "pre_tool" => Ok(Self::PreTool),
            "post_tool" => Ok(Self::PostTool),
            "prompt_capture" => Ok(Self::PromptCapture),
            _ => Err(GuardHookPhaseParseError),
        }
    }
}

/// Error returned when a Guard hook phase is not part of the current contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardHookPhaseParseError;

impl fmt::Display for GuardHookPhaseParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Guard hook phase must be pre_tool, post_tool, or prompt_capture")
    }
}

impl Error for GuardHookPhaseParseError {}

/// Compatibility result recorded for one actual Guard hook event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardHookContractStatus {
    Compatible,
    Malformed,
    Incompatible,
}

impl GuardHookContractStatus {
    /// Returns the stable stored value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::Malformed => "malformed",
            Self::Incompatible => "incompatible",
        }
    }
}

/// Derived availability of the Codex prompt-observation Guard hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PromptCaptureStatus {
    Unavailable,
    UnsupportedByHost,
    NotConfigured,
    ReloadRequired,
    Configured,
    Observed,
    Active,
    Degraded,
}

impl PromptCaptureStatus {
    /// Returns the stable value name for this prompt-capture status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::UnsupportedByHost => "unsupported_by_host",
            Self::NotConfigured => "not_configured",
            Self::ReloadRequired => "reload_required",
            Self::Configured => "configured",
            Self::Observed => "observed",
            Self::Active => "active",
            Self::Degraded => "degraded",
        }
    }

    /// Returns true when prompt observations can be recorded without degradation.
    pub const fn is_operational(self) -> bool {
        matches!(self, Self::Configured | Self::Observed | Self::Active)
    }
}

/// Resolution status for an unrecorded Product Repository change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnrecordedChangeStatus {
    Unresolved,
    Resolved,
}

impl UnrecordedChangeStatus {
    /// Returns the stable value name for this unrecorded-change status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unresolved => "unresolved",
            Self::Resolved => "resolved",
        }
    }
}

/// Confidence of one mutation-effect observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObservationConfidence {
    Confirmed,
    Structured,
    Heuristic,
    Unknown,
}

/// Effect kind reported by one mutation observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObservedEffectKind {
    ReadOnly,
    ProductFileWrite,
    NonProductWrite,
    ExternalEffect,
    Unknown,
}

/// Resolution basis for an unrecorded Product Repository change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnrecordedChangeResolutionBasis {
    Reverted,
    CoveredByWriteTicket,
    RecordedAsExpectedWrite,
    AcceptedByUser,
}

impl UnrecordedChangeResolutionBasis {
    /// Returns the stable value name for this unrecorded-change resolution basis.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reverted => "reverted",
            Self::CoveredByWriteTicket => "covered_by_write_ticket",
            Self::RecordedAsExpectedWrite => "recorded_as_expected_write",
            Self::AcceptedByUser => "accepted_by_user",
        }
    }
}

/// State reference discriminator values.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum StateRecordKind {
    ProjectState,
    Task,
    ChangeUnit,
    ShapingCheckpoint,
    ShapingGap,
    ShapingDecisionApplication,
    ShapingAuthorityReauthorization,
    WriteTicket,
    UserActionRequest,
    UserActionResolution,
    Run,
    EvidenceSummary,
    EvidenceObservation,
    EvidenceCaptureIntent,
    EvidenceProducer,
    Artifact,
    Blocker,
    TaskEvent,
    AgentConnection,
    UnrecordedChange,
    ProjectContinuityRecord,
}

/// Project-level continuity record family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectContinuityKind {
    Decision,
    Obligation,
    KnownLimit,
    AcceptedRisk,
    Constraint,
}

/// Lifecycle status for a project-level continuity record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectContinuityStatus {
    Active,
    Superseded,
    Closed,
}

/// Concrete output Task modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskMode {
    Advisor,
    Direct,
    Work,
}

/// Intake control request, including Core-selected automatic resolution.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequestedControlLevel {
    #[default]
    Auto,
    Observe,
    Light,
    Tracked,
    Sensitive,
}

impl RequestedControlLevel {
    /// Returns the stable value name for this requested control.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Observe => "observe",
            Self::Light => "light",
            Self::Tracked => "tracked",
            Self::Sensitive => "sensitive",
        }
    }
}

/// Effective upward-only Task control level.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TaskControlLevel {
    Observe,
    Light,
    Tracked,
    Sensitive,
}

impl TaskControlLevel {
    /// Returns the stable value name for this effective control.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Light => "light",
            Self::Tracked => "tracked",
            Self::Sensitive => "sensitive",
        }
    }
}

/// Current delivery phase inside one Task's longer-lived outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkPhase {
    Shaping,
    Implementation,
}

/// Durable readiness of one shaping checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShapingCheckpointReadiness {
    Blocked,
    Ready,
    Superseded,
}

impl ShapingCheckpointReadiness {
    /// Returns the stable stored and public spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::Ready => "ready",
            Self::Superseded => "superseded",
        }
    }
}

/// Closed shaping-gap kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShapingGapKind {
    GoalMissing,
    ScopeBoundaryMissing,
    NonGoalsMissing,
    AcceptanceCriteriaMissing,
    AutonomyBoundaryMissing,
    ImplementationBoundaryMissing,
    BaselineMissing,
    UserProductDecisionRequired,
    UserTechnicalDecisionRequired,
    UserScopeDecisionRequired,
    SensitiveApprovalRequired,
}

/// Closed authority method that applies one accepted shaping decision.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub enum ShapingDecisionApplicationOwner {
    #[serde(rename = "volicord.update_scope")]
    UpdateScope,
    #[serde(rename = "volicord.advance_task")]
    AdvanceTask,
    #[serde(rename = "volicord.finalize_advice")]
    FinalizeAdvice,
}

/// Closed current-authority status of one shaping decision application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShapingDecisionApplicationAuthorityStatus {
    Current,
    Stale,
    Superseded,
}

/// Closed terminal outcome of consuming one stale shaping application.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ShapingAuthorityReauthorizationOutcome {
    Retired,
    Reissued,
}

impl ShapingAuthorityReauthorizationOutcome {
    /// Returns the stable stored and public spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Retired => "retired",
            Self::Reissued => "reissued",
        }
    }
}

impl ShapingDecisionApplicationAuthorityStatus {
    /// Returns the stable stored and public spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Stale => "stale",
            Self::Superseded => "superseded",
        }
    }
}

impl ShapingDecisionApplicationOwner {
    /// Returns the public method that owns the semantic application effect.
    pub const fn method(self) -> MethodName {
        match self {
            Self::UpdateScope => MethodName::UpdateScope,
            Self::AdvanceTask => MethodName::AdvanceTask,
            Self::FinalizeAdvice => MethodName::FinalizeAdvice,
        }
    }
}

/// Canonical semantic policy for one user-owned shaping-gap kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapingDecisionPolicy {
    pub user_action_kind: UserActionKind,
    pub required_for: &'static [UserActionRequiredFor],
    pub application_owner: ShapingDecisionApplicationOwner,
    pub changes_scope_revision: bool,
    pub retain_resolution_for_downstream: bool,
}

impl ShapingGapKind {
    /// Returns whether this gap requires exact User Channel authority.
    pub const fn is_user_owned(self) -> bool {
        matches!(
            self,
            Self::UserProductDecisionRequired
                | Self::UserTechnicalDecisionRequired
                | Self::UserScopeDecisionRequired
                | Self::SensitiveApprovalRequired
        )
    }

    /// Returns the compatible UserAction kind for a user-owned gap.
    pub const fn user_action_kind(self) -> Option<UserActionKind> {
        match self.decision_policy() {
            Some(policy) => Some(policy.user_action_kind),
            None => None,
        }
    }

    /// Returns the compatible judgment kind for a user-owned gap.
    pub const fn judgment_kind(self) -> Option<JudgmentKind> {
        match self {
            Self::UserProductDecisionRequired => Some(JudgmentKind::ProductDecision),
            Self::UserTechnicalDecisionRequired => Some(JudgmentKind::TechnicalDecision),
            Self::UserScopeDecisionRequired => Some(JudgmentKind::ScopeDecision),
            Self::SensitiveApprovalRequired => Some(JudgmentKind::SensitiveApproval),
            Self::GoalMissing
            | Self::ScopeBoundaryMissing
            | Self::NonGoalsMissing
            | Self::AcceptanceCriteriaMissing
            | Self::AutonomyBoundaryMissing
            | Self::ImplementationBoundaryMissing
            | Self::BaselineMissing => None,
        }
    }

    /// Returns the one canonical policy for a user-owned shaping decision.
    pub const fn decision_policy(self) -> Option<ShapingDecisionPolicy> {
        self.decision_policy_for_mode(TaskMode::Work)
    }

    /// Returns the canonical policy for a user-owned shaping decision in the
    /// Task mode whose method will apply it.
    pub const fn decision_policy_for_mode(self, mode: TaskMode) -> Option<ShapingDecisionPolicy> {
        match self {
            Self::UserProductDecisionRequired => Some(ShapingDecisionPolicy {
                user_action_kind: UserActionKind::ProductDecision,
                required_for: advisor_or_work_required_for(mode),
                application_owner: advisor_or_work_application_owner(mode),
                changes_scope_revision: false,
                retain_resolution_for_downstream: false,
            }),
            Self::UserTechnicalDecisionRequired => Some(ShapingDecisionPolicy {
                user_action_kind: UserActionKind::TechnicalDecision,
                required_for: advisor_or_work_required_for(mode),
                application_owner: advisor_or_work_application_owner(mode),
                changes_scope_revision: false,
                retain_resolution_for_downstream: false,
            }),
            Self::UserScopeDecisionRequired => Some(ShapingDecisionPolicy {
                user_action_kind: UserActionKind::ScopeDecision,
                required_for: &[UserActionRequiredFor::ScopeUpdate],
                application_owner: ShapingDecisionApplicationOwner::UpdateScope,
                changes_scope_revision: true,
                retain_resolution_for_downstream: false,
            }),
            Self::SensitiveApprovalRequired => Some(ShapingDecisionPolicy {
                user_action_kind: UserActionKind::SensitiveApproval,
                required_for: if matches!(mode, TaskMode::Advisor) {
                    &[UserActionRequiredFor::FinalizeAdvice]
                } else {
                    &[
                        UserActionRequiredFor::AdvanceTask,
                        UserActionRequiredFor::PrepareWrite,
                        UserActionRequiredFor::RecordRun,
                        UserActionRequiredFor::CloseComplete,
                    ]
                },
                application_owner: advisor_or_work_application_owner(mode),
                changes_scope_revision: false,
                retain_resolution_for_downstream: true,
            }),
            Self::GoalMissing
            | Self::ScopeBoundaryMissing
            | Self::NonGoalsMissing
            | Self::AcceptanceCriteriaMissing
            | Self::AutonomyBoundaryMissing
            | Self::ImplementationBoundaryMissing
            | Self::BaselineMissing => None,
        }
    }

    /// Returns the stable stored and public spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GoalMissing => "goal_missing",
            Self::ScopeBoundaryMissing => "scope_boundary_missing",
            Self::NonGoalsMissing => "non_goals_missing",
            Self::AcceptanceCriteriaMissing => "acceptance_criteria_missing",
            Self::AutonomyBoundaryMissing => "autonomy_boundary_missing",
            Self::ImplementationBoundaryMissing => "implementation_boundary_missing",
            Self::BaselineMissing => "baseline_missing",
            Self::UserProductDecisionRequired => "user_product_decision_required",
            Self::UserTechnicalDecisionRequired => "user_technical_decision_required",
            Self::UserScopeDecisionRequired => "user_scope_decision_required",
            Self::SensitiveApprovalRequired => "sensitive_approval_required",
        }
    }
}

const fn advisor_or_work_required_for(mode: TaskMode) -> &'static [UserActionRequiredFor] {
    if matches!(mode, TaskMode::Advisor) {
        &[UserActionRequiredFor::FinalizeAdvice]
    } else {
        &[UserActionRequiredFor::AdvanceTask]
    }
}

const fn advisor_or_work_application_owner(mode: TaskMode) -> ShapingDecisionApplicationOwner {
    if matches!(mode, TaskMode::Advisor) {
        ShapingDecisionApplicationOwner::FinalizeAdvice
    } else {
        ShapingDecisionApplicationOwner::AdvanceTask
    }
}

/// Durable disposition of one shaping gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShapingGapStatus {
    Current,
    Accepted,
    Rejected,
    Deferred,
    Applied,
}

impl ShapingGapStatus {
    /// Returns the stable stored and public spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Deferred => "deferred",
            Self::Applied => "applied",
        }
    }
}

/// Effective authority state of one shaping-linked UserAction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShapingDecisionAuthorityState {
    AwaitingUser,
    AcceptedUnapplied,
    Applied,
    Rejected,
    Deferred,
    Expired,
    Stale,
    Superseded,
    Inconsistent,
}

/// Typed reason that requires an agent to revise the current shaping plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShapingDecisionRecoveryReason {
    Rejected,
    Deferred,
    Expired,
}

impl ShapingDecisionAuthorityState {
    /// Returns the recovery reason for a current non-authorizing terminal state.
    pub const fn recovery_reason(self) -> Option<ShapingDecisionRecoveryReason> {
        match self {
            Self::Rejected => Some(ShapingDecisionRecoveryReason::Rejected),
            Self::Deferred => Some(ShapingDecisionRecoveryReason::Deferred),
            Self::Expired => Some(ShapingDecisionRecoveryReason::Expired),
            Self::AwaitingUser
            | Self::AcceptedUnapplied
            | Self::Applied
            | Self::Stale
            | Self::Superseded
            | Self::Inconsistent => None,
        }
    }
}

/// Exact facts consumed by the canonical shaping-decision authority evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapingDecisionAuthorityFacts {
    pub effective_user_action_status: UserActionStatus,
    pub resolution_present: bool,
    pub machine_action: Option<UserActionOptionAction>,
    pub resolution_outcome: Option<JudgmentResolutionOutcome>,
    pub request_basis_status: UserActionBasisStatus,
    pub basis_compatibility_status: UserActionBasisStatus,
    pub checkpoint_identity_matches: bool,
    pub gap_identity_matches: bool,
    pub resolution_identity_matches: bool,
    pub policy_matches: bool,
    pub verified_user_channel: bool,
    pub task_mode_matches: bool,
    pub scope_revision_matches: bool,
    pub baseline_matches: bool,
    pub change_unit_matches: bool,
    pub gap_status: ShapingGapStatus,
    pub application_present: bool,
    pub application_authority_status: Option<ShapingDecisionApplicationAuthorityStatus>,
    pub application_identity_matches: bool,
    pub application_lineage_current: bool,
}

/// Evaluates whether a shaping-linked UserAction is waiting, authorizing,
/// applied, non-authorizing, obsolete, or contradictory.
pub fn evaluate_shaping_decision_authority(
    facts: ShapingDecisionAuthorityFacts,
) -> ShapingDecisionAuthorityState {
    let exact_accepted_resolution = facts.resolution_present
        && facts.resolution_identity_matches
        && facts.verified_user_channel
        && matches!(facts.machine_action, Some(UserActionOptionAction::Accept))
        && matches!(
            facts.resolution_outcome,
            Some(JudgmentResolutionOutcome::Accepted)
        );
    if facts.effective_user_action_status == UserActionStatus::Superseded
        || facts.application_authority_status
            == Some(ShapingDecisionApplicationAuthorityStatus::Superseded)
    {
        return ShapingDecisionAuthorityState::Superseded;
    }
    if facts.effective_user_action_status == UserActionStatus::Stale
        || facts.application_authority_status
            == Some(ShapingDecisionApplicationAuthorityStatus::Stale)
    {
        if facts.application_authority_status
            == Some(ShapingDecisionApplicationAuthorityStatus::Stale)
            && (!facts.application_present
                || !facts.application_identity_matches
                || !facts.policy_matches
                || !facts.task_mode_matches
                || !exact_accepted_resolution)
        {
            return ShapingDecisionAuthorityState::Inconsistent;
        }
        return ShapingDecisionAuthorityState::Stale;
    }

    let identity_matches = facts.checkpoint_identity_matches
        && facts.gap_identity_matches
        && facts.policy_matches
        && facts.task_mode_matches;
    if !identity_matches {
        return ShapingDecisionAuthorityState::Inconsistent;
    }

    let current_compatible_basis = facts.request_basis_status == UserActionBasisStatus::Current
        && facts.basis_compatibility_status == UserActionBasisStatus::Current
        && facts.scope_revision_matches
        && facts.baseline_matches
        && facts.change_unit_matches;
    if !current_compatible_basis {
        return ShapingDecisionAuthorityState::Inconsistent;
    }

    match facts.effective_user_action_status {
        UserActionStatus::Pending
            if !facts.resolution_present && facts.gap_status == ShapingGapStatus::Current =>
        {
            ShapingDecisionAuthorityState::AwaitingUser
        }
        UserActionStatus::Expired
            if !facts.resolution_present && facts.gap_status == ShapingGapStatus::Current =>
        {
            ShapingDecisionAuthorityState::Expired
        }
        UserActionStatus::Resolved if exact_accepted_resolution => {
            if facts.application_present {
                if !facts.application_identity_matches {
                    ShapingDecisionAuthorityState::Inconsistent
                } else {
                    match facts.application_authority_status {
                        Some(ShapingDecisionApplicationAuthorityStatus::Current)
                            if facts.application_lineage_current
                                && facts.gap_status == ShapingGapStatus::Applied =>
                        {
                            ShapingDecisionAuthorityState::Applied
                        }
                        Some(ShapingDecisionApplicationAuthorityStatus::Stale) => {
                            ShapingDecisionAuthorityState::Stale
                        }
                        Some(ShapingDecisionApplicationAuthorityStatus::Superseded) => {
                            ShapingDecisionAuthorityState::Superseded
                        }
                        Some(ShapingDecisionApplicationAuthorityStatus::Current) | None => {
                            ShapingDecisionAuthorityState::Inconsistent
                        }
                    }
                }
            } else if facts.gap_status == ShapingGapStatus::Accepted {
                ShapingDecisionAuthorityState::AcceptedUnapplied
            } else {
                ShapingDecisionAuthorityState::Inconsistent
            }
        }
        UserActionStatus::Resolved
            if facts.resolution_present
                && facts.resolution_identity_matches
                && facts.verified_user_channel
                && matches!(facts.machine_action, Some(UserActionOptionAction::Reject))
                && matches!(
                    facts.resolution_outcome,
                    Some(JudgmentResolutionOutcome::Rejected)
                ) =>
        {
            if facts.gap_status == ShapingGapStatus::Rejected {
                ShapingDecisionAuthorityState::Rejected
            } else {
                ShapingDecisionAuthorityState::Inconsistent
            }
        }
        UserActionStatus::Resolved
            if facts.resolution_present
                && facts.resolution_identity_matches
                && facts.verified_user_channel
                && matches!(facts.machine_action, Some(UserActionOptionAction::Defer))
                && matches!(
                    facts.resolution_outcome,
                    Some(JudgmentResolutionOutcome::Deferred)
                ) =>
        {
            if facts.gap_status == ShapingGapStatus::Deferred {
                ShapingDecisionAuthorityState::Deferred
            } else {
                ShapingDecisionAuthorityState::Inconsistent
            }
        }
        UserActionStatus::Pending
        | UserActionStatus::Resolved
        | UserActionStatus::Stale
        | UserActionStatus::Superseded
        | UserActionStatus::Expired => ShapingDecisionAuthorityState::Inconsistent,
    }
}

/// Closed workflow-progression states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStateKind {
    NoActiveTask,
    ShapingRequired,
    AwaitingUserAction,
    DecisionRecoveryRequired,
    ReadyToApplyDecisions,
    ReadyForChangeUnit,
    ReadyToFinalizeAdvice,
    ReadyForImplementation,
    Implementation,
    CloseReview,
    Terminal,
}

/// Typed reason that blocks the current workflow transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowBlockingReason {
    NoCurrentCheckpoint,
    ShapingGapsCurrent,
    UserActionPending,
    AcceptedDecisionsNotApplied,
    DecisionRecoveryRequired,
    ApplicationAuthorityStale,
    ChangeUnitRequired,
    AdvisorFinalizationRequired,
    ExplicitAdvanceRequired,
    RecoveryConstraint,
    InconsistentAuthorityState,
}

/// Task-owned final-acceptance policy selected at intake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AcceptancePolicy {
    Required,
    NotRequired,
    PolicyDependent,
}

/// Canonical relation from a newly created Task to one predecessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskLineageRelation {
    Continues,
    DerivedFrom,
    SplitFrom,
    Replaces,
    ImplementsAdviceFrom,
}

/// Selectable predecessor material considered during Task creation.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CarryForwardKind {
    Scope,
    NonGoals,
    UserDecisions,
    SourceRefs,
    ContextRefs,
    KnownLimitations,
    UnresolvedObligations,
    ResidualRisks,
    Baseline,
}

/// Core disposition for one explicitly selected carry-forward category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CarryForwardDispositionStatus {
    Applied,
    ReferenceOnly,
}

/// Optional VCS binding kind exposed by current-position state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceVcs {
    Git,
}

/// Responsible party projected by a compact authority receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityNextActor {
    Agent,
    User,
    None,
}

impl AuthorityNextActor {
    /// Returns the stable value name for this next-actor value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::User => "user",
            Self::None => "none",
        }
    }

    /// Parses one stable next-actor value.
    pub fn from_stable_str(value: &str) -> Option<Self> {
        match value {
            "agent" => Some(Self::Agent),
            "user" => Some(Self::User),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

/// MCP mutation response detail requested by the caller.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MutationDetailLevel {
    #[default]
    Summary,
    Workflow,
    Full,
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

impl TaskLifecyclePhase {
    /// Returns the stable value name for this lifecycle phase.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shaping => "shaping",
            Self::Ready => "ready",
            Self::Executing => "executing",
            Self::WaitingUser => "waiting_user",
            Self::Blocked => "blocked",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Superseded => "superseded",
        }
    }
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

impl CloseState {
    /// Returns the stable value name for this close state.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Blocked => "blocked",
            Self::Closed => "closed",
            Self::Cancelled => "cancelled",
            Self::Superseded => "superseded",
        }
    }
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

impl StatusCloseState {
    /// Returns the stable value name for this status close state.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Blocked => "blocked",
            Self::Closed => "closed",
            Self::Cancelled => "cancelled",
            Self::Superseded => "superseded",
            Self::None => "none",
        }
    }
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

/// Canonical persisted Task close summary.
///
/// Every Task records an explicit close reason, including `none` while the
/// Task is open. The remaining members are present only when their owning
/// close workflow has produced them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedCloseSummary {
    pub close_reason: CloseReason,
    #[serde(default)]
    pub closed_at: Option<UtcTimestamp>,
    #[serde(default)]
    pub intent: Option<CloseIntent>,
    #[serde(default)]
    pub user_note: Option<String>,
    #[serde(default)]
    pub superseding_task_id: Option<TaskId>,
    #[serde(default)]
    pub required_sensitive_categories: Vec<String>,
    #[serde(default)]
    pub sensitive_categories: Vec<String>,
    #[serde(default)]
    pub baseline_stale: bool,
    #[serde(default)]
    pub baseline_status: Option<String>,
    #[serde(default)]
    pub recovery_required: bool,
    #[serde(default)]
    pub visible_risks: Vec<Value>,
    #[serde(default)]
    pub residual_risk_visible: bool,
    #[serde(default)]
    pub residual_risks: Vec<Value>,
    #[serde(default)]
    pub residual_risk_present: bool,
}

impl Default for PersistedCloseSummary {
    fn default() -> Self {
        Self {
            close_reason: CloseReason::None,
            closed_at: None,
            intent: None,
            user_note: None,
            superseding_task_id: None,
            required_sensitive_categories: Vec::new(),
            sensitive_categories: Vec::new(),
            baseline_stale: false,
            baseline_status: None,
            recovery_required: false,
            visible_risks: Vec::new(),
            residual_risk_visible: false,
            residual_risks: Vec::new(),
            residual_risk_present: false,
        }
    }
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

/// Closed semantic identity for one variant of a task-state-bound workflow action.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowActionSemanticVariant {
    CreateInitial,
    ReplaceCurrent,
    KeepCurrentChangeUnit,
    CreateCurrentChangeUnit,
    ReplaceCurrentChangeUnit,
    FinalizeAdvice,
    AdvanceTask,
    PrepareEvidenceCapture,
    PrepareWrite,
    StageArtifact,
    RecordRun,
    RequestUserAction,
    ResolveUserAction,
    ReconcileChanges,
    CheckClose,
    CloseTask,
}

impl WorkflowActionSemanticVariant {
    /// Returns the canonical wire value owned by this closed variant set.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CreateInitial => "create_initial",
            Self::ReplaceCurrent => "replace_current",
            Self::KeepCurrentChangeUnit => "keep_current_change_unit",
            Self::CreateCurrentChangeUnit => "create_current_change_unit",
            Self::ReplaceCurrentChangeUnit => "replace_current_change_unit",
            Self::FinalizeAdvice => "finalize_advice",
            Self::AdvanceTask => "advance_task",
            Self::PrepareEvidenceCapture => "prepare_evidence_capture",
            Self::PrepareWrite => "prepare_write",
            Self::StageArtifact => "stage_artifact",
            Self::RecordRun => "record_run",
            Self::RequestUserAction => "request_user_action",
            Self::ResolveUserAction => "resolve_user_action",
            Self::ReconcileChanges => "reconcile_changes",
            Self::CheckClose => "check_close",
            Self::CloseTask => "close_task",
        }
    }

    /// Returns the public method that owns this semantic variant.
    pub const fn method(self) -> MethodName {
        match self {
            Self::CreateInitial | Self::ReplaceCurrent => MethodName::RecordShapingCheckpoint,
            Self::KeepCurrentChangeUnit
            | Self::CreateCurrentChangeUnit
            | Self::ReplaceCurrentChangeUnit => MethodName::UpdateScope,
            Self::FinalizeAdvice => MethodName::FinalizeAdvice,
            Self::AdvanceTask => MethodName::AdvanceTask,
            Self::PrepareEvidenceCapture => MethodName::PrepareEvidenceCapture,
            Self::PrepareWrite => MethodName::PrepareWrite,
            Self::StageArtifact => MethodName::StageArtifact,
            Self::RecordRun => MethodName::RecordRun,
            Self::RequestUserAction => MethodName::RequestUserAction,
            Self::ResolveUserAction => MethodName::ResolveUserAction,
            Self::ReconcileChanges => MethodName::ReconcileChanges,
            Self::CheckClose => MethodName::CheckClose,
            Self::CloseTask => MethodName::CloseTask,
        }
    }

    /// Selects the exact update-scope variant for a Change Unit operation.
    pub const fn for_change_unit_operation(operation: ChangeUnitOperation) -> Self {
        match operation {
            ChangeUnitOperation::KeepCurrent => Self::KeepCurrentChangeUnit,
            ChangeUnitOperation::CreateCurrent => Self::CreateCurrentChangeUnit,
            ChangeUnitOperation::ReplaceCurrent => Self::ReplaceCurrentChangeUnit,
        }
    }

    /// Returns the Change Unit operation selected by an update-scope variant.
    pub const fn change_unit_operation(self) -> Option<ChangeUnitOperation> {
        match self {
            Self::KeepCurrentChangeUnit => Some(ChangeUnitOperation::KeepCurrent),
            Self::CreateCurrentChangeUnit => Some(ChangeUnitOperation::CreateCurrent),
            Self::ReplaceCurrentChangeUnit => Some(ChangeUnitOperation::ReplaceCurrent),
            _ => None,
        }
    }

    /// Returns the sole semantic variant for a method without state-selected variants.
    pub const fn for_single_variant_method(method: MethodName) -> Option<Self> {
        match method {
            MethodName::FinalizeAdvice => Some(Self::FinalizeAdvice),
            MethodName::AdvanceTask => Some(Self::AdvanceTask),
            MethodName::PrepareEvidenceCapture => Some(Self::PrepareEvidenceCapture),
            MethodName::PrepareWrite => Some(Self::PrepareWrite),
            MethodName::StageArtifact => Some(Self::StageArtifact),
            MethodName::RecordRun => Some(Self::RecordRun),
            MethodName::RequestUserAction => Some(Self::RequestUserAction),
            MethodName::ResolveUserAction => Some(Self::ResolveUserAction),
            MethodName::ReconcileChanges => Some(Self::ReconcileChanges),
            MethodName::CheckClose => Some(Self::CheckClose),
            MethodName::CloseTask => Some(Self::CloseTask),
            _ => None,
        }
    }
}

/// Actor that can execute one transition selected by the workflow machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTransitionActor {
    Agent,
    User,
    System,
}

/// Observable effect family for one workflow transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTransitionEffectClass {
    CoreStateMutation,
    UserChannelMutation,
    WriteAuthorization,
    ArtifactStaging,
    EvidenceCapture,
    ExecutionRecording,
    ReadOnlyAssessment,
    TerminalMutation,
}

/// Expected state family after a transition is evaluated and committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowExpectedResultState {
    ReevaluateCurrentAuthority,
    AwaitingUserAction,
    Implementation,
    CloseReview,
    Terminal,
}

/// Whether submitted content may invalidate current workflow authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowAuthorityInvalidationPolicy {
    Permitted,
    Forbidden,
}

/// Closed reason that an exact workflow transition was not admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransitionRejectionReason {
    ActionNotCurrent,
    VariantNotCurrent,
    AuthorityBasisMismatch,
    ImplementationAuthorityWouldBeInvalidated,
    UserAuthorityMissing,
    CheckpointStale,
    ChangeUnitStale,
    WorkspaceBasisStale,
    ClosePreconditionMissing,
}

/// Change Unit effect contract values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChangeUnitEffectKind {
    ProductFileWrite,
    ArtifactRegistration,
    RunRecording,
    UserActionRequest,
    EvidenceUpdate,
    SensitiveAction,
    ExternalNetwork,
    SecretAccess,
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

/// MCP-visible close-task intents that can mutate Task state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CloseMutationIntent {
    Complete,
    Cancel,
    Supersede,
}

impl From<CloseMutationIntent> for CloseIntent {
    fn from(value: CloseMutationIntent) -> Self {
        match value {
            CloseMutationIntent::Complete => Self::Complete,
            CloseMutationIntent::Cancel => Self::Cancel,
            CloseMutationIntent::Supersede => Self::Supersede,
        }
    }
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

/// Prepare-write write ticket effect values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteTicketEffect {
    None,
    WouldIssue,
    Issued,
    Reused,
}

/// Write ticket state values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteTicketState {
    Open,
    Observed,
    Reconciled,
    Closed,
    Invalidated,
    Revoked,
}

/// Write ticket status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteTicketStatus {
    Active,
    Consumed,
    Invalidated,
    Revoked,
}

impl WriteTicketStatus {
    /// Returns the stable value name for this write-ticket status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Consumed => "consumed",
            Self::Invalidated => "invalidated",
            Self::Revoked => "revoked",
        }
    }
}

/// Stable reason recorded when a write ticket becomes invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteTicketInvalidationReason {
    ScopeRevisionChanged,
    ChangeUnitChanged,
    BaselineChanged,
    WorkspaceChanged,
    ApprovalBasisChanged,
    IdleTimeout,
    TaskClosed,
    ExplicitRevoke,
}

impl WriteTicketInvalidationReason {
    /// Returns the stable value name for this invalidation reason.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ScopeRevisionChanged => "scope_revision_changed",
            Self::ChangeUnitChanged => "change_unit_changed",
            Self::BaselineChanged => "baseline_changed",
            Self::WorkspaceChanged => "workspace_changed",
            Self::ApprovalBasisChanged => "approval_basis_changed",
            Self::IdleTimeout => "idle_timeout",
            Self::TaskClosed => "task_closed",
            Self::ExplicitRevoke => "explicit_revoke",
        }
    }
}

/// Run kind values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
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
    Workspace,
    UserAction,
    SensitiveApproval,
    WriteCompatibility,
    Baseline,
    EffectContract,
    ConnectionCapability,
}

/// Close-readiness blocker category values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CloseReadinessBlockerCategory {
    Task,
    OpenRun,
    Scope,
    UserAction,
    PendingUserAction,
    SensitiveApproval,
    WriteCompatibility,
    Baseline,
    ConnectionCapability,
    Evidence,
    EvidenceClaim,
    EvidenceProvenance,
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

/// User-facing evidence presentation state values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceDisplayState {
    Prepared,
    Attached,
    AcceptedForClose,
}

/// Derived evidence gate states shared by structured and human projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceGateState {
    NotRequired,
    OptionalNone,
    RequiredMissing,
    Partial,
    Sufficient,
    Stale,
    Blocked,
}

impl EvidenceGateState {
    /// Returns the stable value name for this evidence-gate state.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotRequired => "not_required",
            Self::OptionalNone => "optional_none",
            Self::RequiredMissing => "required_missing",
            Self::Partial => "partial",
            Self::Sufficient => "sufficient",
            Self::Stale => "stale",
            Self::Blocked => "blocked",
        }
    }
}

/// Evidence coverage item state values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceCoverageState {
    Unsupported,
    Partial,
    Supported,
    Contradicted,
    NotApplicable,
    Stale,
}

/// Request-side evidence coverage update states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceCoverageUpdateState {
    Unsupported,
    Partial,
    Supported,
    Contradicted,
    NotApplicable,
}

impl From<EvidenceCoverageUpdateState> for EvidenceCoverageState {
    fn from(value: EvidenceCoverageUpdateState) -> Self {
        match value {
            EvidenceCoverageUpdateState::Unsupported => Self::Unsupported,
            EvidenceCoverageUpdateState::Partial => Self::Partial,
            EvidenceCoverageUpdateState::Supported => Self::Supported,
            EvidenceCoverageUpdateState::Contradicted => Self::Contradicted,
            EvidenceCoverageUpdateState::NotApplicable => Self::NotApplicable,
        }
    }
}

/// Evidence requirement attached to one current acceptance criterion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRequirement {
    Required,
    Optional,
    NotRequired,
}

/// Evidence observation source-kind values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSourceKind {
    AgentReport,
    ExternalTool,
    UserObservation,
    ReusedEvidence,
    UnverifiedClaim,
}

/// Evidence observation assurance-level values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceAssuranceLevel {
    CooperativeReport,
    ExternalToolResult,
    UserObserved,
    Unverified,
}

/// Core-derived producer anchor classifications for evidence observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceProducerKind {
    UnverifiedCaller,
    UserChannelObservation,
    VerifiedToolInvocation,
    VerifiedCommandExecution,
    ReusedEvidence,
}

/// Core-derived claim-relevance assessment states for evidence observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRelevanceStatus {
    Unassessed,
    Supported,
    Contradicted,
}

impl EvidenceRelevanceStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unassessed => "unassessed",
            Self::Supported => "supported",
            Self::Contradicted => "contradicted",
        }
    }
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
}

/// Public disclosure classes for result interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuaranteeClass {
    AuthorityRecord,
    CooperativeHostDecision,
    UserActionResolution,
}

/// Stable public non-guarantee values for result interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum NonGuarantee {
    NotOsSandbox,
    NotNetworkIsolation,
    NotMalwareDefense,
    NotTamperProofAuditLog,
    NotCorrectnessProof,
    NotTestSufficiencyProof,
    NotHumanReviewReplacement,
    NotFullWritePrevention,
    NotFullFilesystemMonitoring,
    NotActorAttributionProof,
    NotIntentProof,
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

/// Canonical user-action family values derived from the closed action draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserActionKind {
    ProductDecision,
    TechnicalDecision,
    ScopeDecision,
    SensitiveApproval,
    FinalAcceptance,
    ResidualRiskAcceptance,
    Cancellation,
    EvidenceObservation,
}

impl From<JudgmentKind> for UserActionKind {
    fn from(value: JudgmentKind) -> Self {
        match value {
            JudgmentKind::ProductDecision => Self::ProductDecision,
            JudgmentKind::TechnicalDecision => Self::TechnicalDecision,
            JudgmentKind::ScopeDecision => Self::ScopeDecision,
            JudgmentKind::SensitiveApproval => Self::SensitiveApproval,
            JudgmentKind::FinalAcceptance => Self::FinalAcceptance,
            JudgmentKind::ResidualRiskAcceptance => Self::ResidualRiskAcceptance,
            JudgmentKind::Cancellation => Self::Cancellation,
        }
    }
}

impl UserActionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProductDecision => "product_decision",
            Self::TechnicalDecision => "technical_decision",
            Self::ScopeDecision => "scope_decision",
            Self::SensitiveApproval => "sensitive_approval",
            Self::FinalAcceptance => "final_acceptance",
            Self::ResidualRiskAcceptance => "residual_risk_acceptance",
            Self::Cancellation => "cancellation",
            Self::EvidenceObservation => "evidence_observation",
        }
    }

    /// Returns the choice judgment kind, or `None` for evidence observation.
    pub const fn judgment_kind(self) -> Option<JudgmentKind> {
        match self {
            Self::ProductDecision => Some(JudgmentKind::ProductDecision),
            Self::TechnicalDecision => Some(JudgmentKind::TechnicalDecision),
            Self::ScopeDecision => Some(JudgmentKind::ScopeDecision),
            Self::SensitiveApproval => Some(JudgmentKind::SensitiveApproval),
            Self::FinalAcceptance => Some(JudgmentKind::FinalAcceptance),
            Self::ResidualRiskAcceptance => Some(JudgmentKind::ResidualRiskAcceptance),
            Self::Cancellation => Some(JudgmentKind::Cancellation),
            Self::EvidenceObservation => None,
        }
    }

    /// Returns whether this action kind may declare the required operation target.
    pub const fn is_compatible_with_required_for(
        self,
        required_for: UserActionRequiredFor,
    ) -> bool {
        match required_for {
            UserActionRequiredFor::ScopeUpdate => matches!(
                self,
                Self::ProductDecision | Self::TechnicalDecision | Self::ScopeDecision
            ),
            UserActionRequiredFor::PrepareWrite => matches!(
                self,
                Self::ProductDecision
                    | Self::TechnicalDecision
                    | Self::ScopeDecision
                    | Self::SensitiveApproval
            ),
            UserActionRequiredFor::AdvanceTask => matches!(
                self,
                Self::ProductDecision
                    | Self::TechnicalDecision
                    | Self::ScopeDecision
                    | Self::SensitiveApproval
            ),
            UserActionRequiredFor::FinalizeAdvice => matches!(
                self,
                Self::ProductDecision | Self::TechnicalDecision | Self::SensitiveApproval
            ),
            UserActionRequiredFor::RecordRun => matches!(
                self,
                Self::ProductDecision
                    | Self::TechnicalDecision
                    | Self::ScopeDecision
                    | Self::SensitiveApproval
                    | Self::EvidenceObservation
            ),
            UserActionRequiredFor::CloseComplete => matches!(
                self,
                Self::ProductDecision
                    | Self::TechnicalDecision
                    | Self::ScopeDecision
                    | Self::SensitiveApproval
                    | Self::FinalAcceptance
                    | Self::ResidualRiskAcceptance
                    | Self::EvidenceObservation
            ),
            UserActionRequiredFor::CloseCancel => matches!(self, Self::Cancellation),
            UserActionRequiredFor::CloseSupersede => matches!(
                self,
                Self::ProductDecision
                    | Self::TechnicalDecision
                    | Self::ScopeDecision
                    | Self::SensitiveApproval
            ),
            UserActionRequiredFor::Informational => true,
        }
    }
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
pub enum UserActionRequiredFor {
    ScopeUpdate,
    AdvanceTask,
    FinalizeAdvice,
    PrepareWrite,
    RecordRun,
    CloseComplete,
    CloseCancel,
    CloseSupersede,
    Informational,
}

/// Effective user-action lifecycle values derived by the canonical evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserActionStatus {
    Pending,
    Resolved,
    Stale,
    Superseded,
    Expired,
}

impl UserActionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Resolved => "resolved",
            Self::Stale => "stale",
            Self::Superseded => "superseded",
            Self::Expired => "expired",
        }
    }
}

/// Choice action resolution outcome values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JudgmentResolutionOutcome {
    Accepted,
    Rejected,
    Deferred,
}

/// Core-owned machine action for current user-action options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserActionOptionAction {
    Accept,
    Reject,
    Defer,
}

impl UserActionOptionAction {
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
pub enum UserActionBasisStatus {
    Current,
    Stale,
    Superseded,
}

/// Verified User Channel kinds that can resolve one pending user action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserActionChannelKind {
    Cli,
}

/// Verified bases accepted for one User Channel resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserActionVerificationBasis {
    CliDirectUserChannel,
}

impl UserActionVerificationBasis {
    /// Returns the stable serialized value owned by this basis.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CliDirectUserChannel => "cli_direct_user_channel",
        }
    }

    /// Strictly decodes one persisted basis value.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "cli_direct_user_channel" => Some(Self::CliDirectUserChannel),
            _ => None,
        }
    }
}

impl UserActionChannelKind {
    /// Returns the single verified invocation basis owned by this User Channel.
    pub const fn verification_basis(self) -> UserActionVerificationBasis {
        match self {
            Self::Cli => UserActionVerificationBasis::CliDirectUserChannel,
        }
    }

    /// Resolves one controlled verification basis to its owning User Channel.
    pub const fn from_verification_basis(verification_basis: UserActionVerificationBasis) -> Self {
        match verification_basis {
            UserActionVerificationBasis::CliDirectUserChannel => Self::Cli,
        }
    }
}

/// Product-wide primary failure classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailureCategory {
    Rejected,
    NotAllowed,
    Unavailable,
    Degraded,
    Corrupt,
}

impl FailureCategory {
    /// Every current product-wide failure category.
    pub const ALL: &'static [Self] = &[
        Self::Rejected,
        Self::NotAllowed,
        Self::Unavailable,
        Self::Degraded,
        Self::Corrupt,
    ];

    /// Returns the stable machine-readable category identifier.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rejected => "rejected",
            Self::NotAllowed => "not_allowed",
            Self::Unavailable => "unavailable",
            Self::Degraded => "degraded",
            Self::Corrupt => "corrupt",
        }
    }
}

/// One canonical public error-code/category relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PublicErrorCodeContract {
    code: ErrorCode,
    wire_name: &'static str,
    category: FailureCategory,
}

impl PublicErrorCodeContract {
    /// Returns the typed public error code.
    pub const fn code(self) -> ErrorCode {
        self.code
    }

    /// Returns the exact public wire identifier.
    pub const fn wire_name(self) -> &'static str {
        self.wire_name
    }

    /// Returns the one required failure category.
    pub const fn category(self) -> FailureCategory {
        self.category
    }
}

macro_rules! public_error_codes {
    (
        $(
            $variant:ident => {
                wire: $wire_name:literal,
                category: $category:ident,
            },
        )+
    ) => {
        /// Public API error code values.
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
        )]
        pub enum ErrorCode {
            $(
                #[serde(rename = $wire_name)]
                $variant,
            )+
        }

        impl ErrorCode {
            /// Every current public error code in declaration order.
            pub const ALL: &'static [Self] = &[
                $(Self::$variant,)+
            ];

            /// Returns the exact public wire identifier.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire_name,)+
                }
            }

            /// Returns the failure category carried by a public `ToolError` with this code.
            pub const fn failure_category(self) -> FailureCategory {
                match self {
                    $(Self::$variant => FailureCategory::$category,)+
                }
            }
        }

        /// Canonical public error-code/category catalog.
        pub const PUBLIC_ERROR_CODE_CONTRACTS: &[PublicErrorCodeContract] = &[
            $(
                PublicErrorCodeContract {
                    code: ErrorCode::$variant,
                    wire_name: $wire_name,
                    category: FailureCategory::$category,
                },
            )+
        ];
    };
}

public_error_codes! {
    ValidationFailed => {
        wire: "VALIDATION_FAILED",
        category: Rejected,
    },
    RunKindIncompatible => {
        wire: "RUN_KIND_INCOMPATIBLE",
        category: Rejected,
    },
    TaskPhaseTransitionRequired => {
        wire: "TASK_PHASE_TRANSITION_REQUIRED",
        category: Rejected,
    },
    ShapingCheckpointRequired => {
        wire: "SHAPING_CHECKPOINT_REQUIRED",
        category: Rejected,
    },
    ShapingCheckpointStale => {
        wire: "SHAPING_CHECKPOINT_STALE",
        category: Rejected,
    },
    UserDecisionUnresolved => {
        wire: "USER_DECISION_UNRESOLVED",
        category: Rejected,
    },
    ChangeUnitRequired => {
        wire: "CHANGE_UNIT_REQUIRED",
        category: Rejected,
    },
    ChangeUnitStale => {
        wire: "CHANGE_UNIT_STALE",
        category: Rejected,
    },
    WorkspaceBasisStale => {
        wire: "WORKSPACE_BASIS_STALE",
        category: Rejected,
    },
    WorkflowActionNotAllowed => {
        wire: "WORKFLOW_ACTION_NOT_ALLOWED",
        category: NotAllowed,
    },
    PersistedDataCorrupt => {
        wire: "PERSISTED_DATA_CORRUPT",
        category: Corrupt,
    },
    StateVersionConflict => {
        wire: "STATE_VERSION_CONFLICT",
        category: Rejected,
    },
    InvocationContextMismatch => {
        wire: "INVOCATION_CONTEXT_MISMATCH",
        category: Rejected,
    },
    NoActiveTask => {
        wire: "NO_ACTIVE_TASK",
        category: Rejected,
    },
    NoActiveChangeUnit => {
        wire: "NO_ACTIVE_CHANGE_UNIT",
        category: Rejected,
    },
    BaselineStale => {
        wire: "BASELINE_STALE",
        category: Rejected,
    },
    ScopeRequired => {
        wire: "SCOPE_REQUIRED",
        category: Rejected,
    },
    ScopeViolation => {
        wire: "SCOPE_VIOLATION",
        category: NotAllowed,
    },
    WriteTicketRequired => {
        wire: "WRITE_TICKET_REQUIRED",
        category: Rejected,
    },
    WriteTicketInvalid => {
        wire: "WRITE_TICKET_INVALID",
        category: Rejected,
    },
    ApprovalDenied => {
        wire: "APPROVAL_DENIED",
        category: NotAllowed,
    },
    ApprovalExpired => {
        wire: "APPROVAL_EXPIRED",
        category: Rejected,
    },
    ApprovalRequired => {
        wire: "APPROVAL_REQUIRED",
        category: Rejected,
    },
    DecisionUnresolved => {
        wire: "DECISION_UNRESOLVED",
        category: Rejected,
    },
    AutonomyBoundaryExceeded => {
        wire: "AUTONOMY_BOUNDARY_EXCEEDED",
        category: NotAllowed,
    },
    DecisionRequired => {
        wire: "DECISION_REQUIRED",
        category: Rejected,
    },
    CapabilityInsufficient => {
        wire: "CAPABILITY_INSUFFICIENT",
        category: NotAllowed,
    },
    EvidenceInsufficient => {
        wire: "EVIDENCE_INSUFFICIENT",
        category: NotAllowed,
    },
    ResidualRiskNotVisible => {
        wire: "RESIDUAL_RISK_NOT_VISIBLE",
        category: NotAllowed,
    },
    AcceptanceRequired => {
        wire: "ACCEPTANCE_REQUIRED",
        category: NotAllowed,
    },
    ProjectionStale => {
        wire: "PROJECTION_STALE",
        category: Unavailable,
    },
    ArtifactMissing => {
        wire: "ARTIFACT_MISSING",
        category: Unavailable,
    },
    ValidatorFailed => {
        wire: "VALIDATOR_FAILED",
        category: Unavailable,
    },
    OperationResultUnavailable => {
        wire: "OPERATION_RESULT_UNAVAILABLE",
        category: Unavailable,
    },
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use chrono::{DateTime, Duration, Utc};
    use serde_json::json;

    use super::{
        evaluate_shaping_decision_authority, AuthorityNextActor, ErrorCode, FailureCategory,
        JudgmentResolutionOutcome, ObservationConfidence, ObservedEffectKind,
        RequestedControlLevel, ShapingDecisionApplicationOwner, ShapingDecisionAuthorityFacts,
        ShapingDecisionAuthorityState, ShapingGapKind, ShapingGapStatus, TaskControlLevel,
        UserActionBasisStatus, UserActionKind, UserActionOptionAction, UserActionRequiredFor,
        UserActionStatus, UtcTimestamp, WriteTicketEffect, WriteTicketInvalidationReason,
        WriteTicketState, WriteTicketStatus,
    };

    fn current_shaping_authority_facts() -> ShapingDecisionAuthorityFacts {
        ShapingDecisionAuthorityFacts {
            effective_user_action_status: UserActionStatus::Pending,
            resolution_present: false,
            machine_action: None,
            resolution_outcome: None,
            request_basis_status: UserActionBasisStatus::Current,
            basis_compatibility_status: UserActionBasisStatus::Current,
            checkpoint_identity_matches: true,
            gap_identity_matches: true,
            resolution_identity_matches: true,
            policy_matches: true,
            verified_user_channel: true,
            task_mode_matches: true,
            scope_revision_matches: true,
            baseline_matches: true,
            change_unit_matches: true,
            gap_status: ShapingGapStatus::Current,
            application_present: false,
            application_authority_status: None,
            application_identity_matches: false,
            application_lineage_current: false,
        }
    }

    #[test]
    fn failure_categories_have_exact_machine_readable_names() {
        for (category, expected) in [
            (FailureCategory::Rejected, "rejected"),
            (FailureCategory::NotAllowed, "not_allowed"),
            (FailureCategory::Unavailable, "unavailable"),
            (FailureCategory::Degraded, "degraded"),
            (FailureCategory::Corrupt, "corrupt"),
        ] {
            assert_eq!(category.as_str(), expected);
            assert_eq!(
                serde_json::to_value(category).expect("category serializes"),
                expected
            );
        }
    }

    #[test]
    fn public_error_contract_declares_each_code_and_wire_name_once() {
        assert_eq!(
            ErrorCode::ALL.len(),
            super::PUBLIC_ERROR_CODE_CONTRACTS.len()
        );
        let mut codes = HashSet::new();
        let mut wire_names = HashSet::new();
        for contract in super::PUBLIC_ERROR_CODE_CONTRACTS {
            assert!(codes.insert(contract.code()));
            assert!(wire_names.insert(contract.wire_name()));
            assert_eq!(contract.code().as_str(), contract.wire_name());
            assert_eq!(
                contract.code().failure_category(),
                contract.category(),
                "{}",
                contract.wire_name()
            );
            assert_eq!(
                serde_json::to_value(contract.code()).expect("error code serializes"),
                contract.wire_name()
            );
            assert_eq!(
                serde_json::from_value::<ErrorCode>(serde_json::json!(contract.wire_name()))
                    .expect("error code deserializes"),
                contract.code()
            );
        }
        assert_eq!(codes.len(), ErrorCode::ALL.len());
    }

    #[test]
    fn canonical_workflow_values_have_exact_json_names_and_ordering() {
        let cases = [
            (
                serde_json::to_value(RequestedControlLevel::Auto).unwrap(),
                json!("auto"),
            ),
            (
                serde_json::to_value(TaskControlLevel::Light).unwrap(),
                json!("light"),
            ),
            (
                serde_json::to_value(WriteTicketEffect::Reused).unwrap(),
                json!("reused"),
            ),
            (
                serde_json::to_value(WriteTicketState::Invalidated).unwrap(),
                json!("invalidated"),
            ),
            (
                serde_json::to_value(WriteTicketStatus::Invalidated).unwrap(),
                json!("invalidated"),
            ),
            (
                serde_json::to_value(WriteTicketInvalidationReason::WorkspaceChanged).unwrap(),
                json!("workspace_changed"),
            ),
            (
                serde_json::to_value(ObservationConfidence::Structured).unwrap(),
                json!("structured"),
            ),
            (
                serde_json::to_value(ObservedEffectKind::ProductFileWrite).unwrap(),
                json!("product_file_write"),
            ),
        ];
        for (actual, expected) in cases {
            assert_eq!(actual, expected);
        }
        assert!(TaskControlLevel::Observe < TaskControlLevel::Sensitive);
        assert_eq!(
            AuthorityNextActor::from_stable_str("user"),
            Some(AuthorityNextActor::User)
        );
    }

    #[test]
    fn shaping_decision_policy_is_one_closed_application_owner_matrix() {
        let cases = [
            (
                ShapingGapKind::UserProductDecisionRequired,
                UserActionKind::ProductDecision,
                &[UserActionRequiredFor::AdvanceTask][..],
                ShapingDecisionApplicationOwner::AdvanceTask,
                false,
                false,
            ),
            (
                ShapingGapKind::UserTechnicalDecisionRequired,
                UserActionKind::TechnicalDecision,
                &[UserActionRequiredFor::AdvanceTask][..],
                ShapingDecisionApplicationOwner::AdvanceTask,
                false,
                false,
            ),
            (
                ShapingGapKind::UserScopeDecisionRequired,
                UserActionKind::ScopeDecision,
                &[UserActionRequiredFor::ScopeUpdate][..],
                ShapingDecisionApplicationOwner::UpdateScope,
                true,
                false,
            ),
            (
                ShapingGapKind::SensitiveApprovalRequired,
                UserActionKind::SensitiveApproval,
                &[
                    UserActionRequiredFor::AdvanceTask,
                    UserActionRequiredFor::PrepareWrite,
                    UserActionRequiredFor::RecordRun,
                    UserActionRequiredFor::CloseComplete,
                ][..],
                ShapingDecisionApplicationOwner::AdvanceTask,
                false,
                true,
            ),
        ];

        for (gap_kind, action_kind, required_for, owner, changes_scope, retain) in cases {
            let policy = gap_kind.decision_policy().expect("user-owned policy");
            assert_eq!(policy.user_action_kind, action_kind);
            assert_eq!(policy.required_for, required_for);
            assert_eq!(policy.application_owner, owner);
            assert_eq!(policy.application_owner.method(), owner.method());
            assert_eq!(policy.changes_scope_revision, changes_scope);
            assert_eq!(policy.retain_resolution_for_downstream, retain);
            assert_eq!(gap_kind.user_action_kind(), Some(action_kind));
        }

        for gap_kind in [
            ShapingGapKind::GoalMissing,
            ShapingGapKind::ScopeBoundaryMissing,
            ShapingGapKind::NonGoalsMissing,
            ShapingGapKind::AcceptanceCriteriaMissing,
            ShapingGapKind::AutonomyBoundaryMissing,
            ShapingGapKind::ImplementationBoundaryMissing,
            ShapingGapKind::BaselineMissing,
        ] {
            assert!(gap_kind.decision_policy().is_none());
        }
    }

    #[test]
    fn shaping_decision_authority_evaluator_is_closed_and_outcome_specific() {
        let pending = current_shaping_authority_facts();
        assert_eq!(
            evaluate_shaping_decision_authority(pending),
            ShapingDecisionAuthorityState::AwaitingUser
        );

        let mut expired = pending;
        expired.effective_user_action_status = UserActionStatus::Expired;
        assert_eq!(
            evaluate_shaping_decision_authority(expired),
            ShapingDecisionAuthorityState::Expired
        );

        let mut accepted = pending;
        accepted.effective_user_action_status = UserActionStatus::Resolved;
        accepted.resolution_present = true;
        accepted.machine_action = Some(UserActionOptionAction::Accept);
        accepted.resolution_outcome = Some(JudgmentResolutionOutcome::Accepted);
        accepted.gap_status = ShapingGapStatus::Accepted;
        assert_eq!(
            evaluate_shaping_decision_authority(accepted),
            ShapingDecisionAuthorityState::AcceptedUnapplied
        );

        let mut applied = accepted;
        applied.gap_status = ShapingGapStatus::Applied;
        applied.application_present = true;
        applied.application_authority_status =
            Some(super::ShapingDecisionApplicationAuthorityStatus::Current);
        applied.application_identity_matches = true;
        applied.application_lineage_current = true;
        assert_eq!(
            evaluate_shaping_decision_authority(applied),
            ShapingDecisionAuthorityState::Applied
        );

        let mut rejected = accepted;
        rejected.machine_action = Some(UserActionOptionAction::Reject);
        rejected.resolution_outcome = Some(JudgmentResolutionOutcome::Rejected);
        rejected.gap_status = ShapingGapStatus::Rejected;
        assert_eq!(
            evaluate_shaping_decision_authority(rejected),
            ShapingDecisionAuthorityState::Rejected
        );

        let mut deferred = accepted;
        deferred.machine_action = Some(UserActionOptionAction::Defer);
        deferred.resolution_outcome = Some(JudgmentResolutionOutcome::Deferred);
        deferred.gap_status = ShapingGapStatus::Deferred;
        assert_eq!(
            evaluate_shaping_decision_authority(deferred),
            ShapingDecisionAuthorityState::Deferred
        );

        let mut stale = accepted;
        stale.effective_user_action_status = UserActionStatus::Stale;
        stale.request_basis_status = UserActionBasisStatus::Stale;
        assert_eq!(
            evaluate_shaping_decision_authority(stale),
            ShapingDecisionAuthorityState::Stale
        );

        let mut superseded = accepted;
        superseded.effective_user_action_status = UserActionStatus::Superseded;
        superseded.request_basis_status = UserActionBasisStatus::Superseded;
        superseded.checkpoint_identity_matches = false;
        superseded.gap_identity_matches = false;
        assert_eq!(
            evaluate_shaping_decision_authority(superseded),
            ShapingDecisionAuthorityState::Superseded
        );

        let mut stale_application = applied;
        stale_application.application_authority_status =
            Some(super::ShapingDecisionApplicationAuthorityStatus::Stale);
        stale_application.checkpoint_identity_matches = false;
        stale_application.application_lineage_current = false;
        assert_eq!(
            evaluate_shaping_decision_authority(stale_application),
            ShapingDecisionAuthorityState::Stale
        );

        let mut malformed_stale_application = stale_application;
        malformed_stale_application.application_identity_matches = false;
        assert_eq!(
            evaluate_shaping_decision_authority(malformed_stale_application),
            ShapingDecisionAuthorityState::Inconsistent
        );

        for contradictory in [
            ShapingDecisionAuthorityFacts {
                gap_status: ShapingGapStatus::Rejected,
                ..accepted
            },
            ShapingDecisionAuthorityFacts {
                verified_user_channel: false,
                ..accepted
            },
            ShapingDecisionAuthorityFacts {
                scope_revision_matches: false,
                ..accepted
            },
            ShapingDecisionAuthorityFacts {
                resolution_identity_matches: false,
                ..accepted
            },
        ] {
            assert_eq!(
                evaluate_shaping_decision_authority(contradictory),
                ShapingDecisionAuthorityState::Inconsistent
            );
        }

        for incompatible_application in [
            ShapingDecisionAuthorityFacts {
                scope_revision_matches: false,
                ..applied
            },
            ShapingDecisionAuthorityFacts {
                baseline_matches: false,
                ..applied
            },
            ShapingDecisionAuthorityFacts {
                change_unit_matches: false,
                ..applied
            },
        ] {
            assert_eq!(
                evaluate_shaping_decision_authority(incompatible_application),
                ShapingDecisionAuthorityState::Inconsistent
            );
        }
    }

    #[test]
    fn utc_timestamp_checked_add_enforces_canonical_four_digit_range() {
        let ordinary = UtcTimestamp::parse("2026-07-13T00:00:00Z").expect("ordinary timestamp");
        assert_eq!(
            ordinary
                .checked_add(Duration::minutes(15))
                .expect("ordinary TTL should fit")
                .to_string(),
            "2026-07-13T00:15:00Z"
        );

        let near_upper =
            UtcTimestamp::parse("9999-12-31T23:50:00Z").expect("four-digit upper timestamp");
        assert!(near_upper.checked_add(Duration::minutes(15)).is_err());
        assert!(near_upper.ensure_canonical_rfc3339_representable().is_ok());

        let chrono_max = UtcTimestamp::from_datetime(DateTime::<Utc>::MAX_UTC);
        assert!(chrono_max.ensure_canonical_rfc3339_representable().is_err());
        assert!(chrono_max.checked_add(Duration::zero()).is_err());
    }
}
