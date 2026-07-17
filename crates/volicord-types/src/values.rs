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
    /// Returns the public method-name value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Intake => "volicord.intake",
            Self::UpdateScope => "volicord.update_scope",
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
    PrepareWrite,
    StageArtifact,
    RecordRun,
    RequestUserAction,
    ResolveUserAction,
    ReconcileChanges,
    CloseTask,
}

/// Controlled presentation role for an owner-composed next action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NextActionPresentationRole {
    Primary,
    Additional,
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

/// Controlled adapter-binding basis value for MCP stdio sessions.
pub const VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING: &str = "mcp_stdio_connection_binding";

/// Controlled adapter-binding basis value for direct CLI invocation.
pub const VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL: &str = "cli_direct_user_channel";

/// Baseline actor assurance level for cooperative Agent Connection provenance.
pub const ACTOR_ASSURANCE_AGENT_CONNECTION_COOPERATIVE: &str = "agent_connection_cooperative";

/// Host family supported by the release Agent Connection contract.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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

/// Local host-hook installation lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardInstallationStatus {
    Absent,
    Configured,
    ReloadRequired,
    Active,
    Degraded,
    Stale,
    Broken,
}

impl GuardInstallationStatus {
    /// Returns the stable value name for this host-hook installation status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Configured => "configured",
            Self::ReloadRequired => "reload_required",
            Self::Active => "active",
            Self::Degraded => "degraded",
            Self::Stale => "stale",
            Self::Broken => "broken",
        }
    }
}

/// Derived local host-hook configuration health.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardConfigurationStatus {
    Absent,
    Configured,
    ReloadRequired,
    Degraded,
    Stale,
    Broken,
}

impl GuardConfigurationStatus {
    /// Returns the stable value name for this host-hook configuration status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Configured => "configured",
            Self::ReloadRequired => "reload_required",
            Self::Degraded => "degraded",
            Self::Stale => "stale",
            Self::Broken => "broken",
        }
    }
}

/// Derived local host-hook runtime-observation health.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardObservationStatus {
    NotObserved,
    Observed,
    StaleObservation,
}

impl GuardObservationStatus {
    /// Returns the stable value name for this host-hook observation status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotObserved => "not_observed",
            Self::Observed => "observed",
            Self::StaleObservation => "stale_observation",
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

/// Confidence assigned to an observed unrecorded Product Repository change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnrecordedChangeConfidence {
    Confirmed,
    Suspected,
}

impl UnrecordedChangeConfidence {
    /// Returns the stable value name for this confidence.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Suspected => "suspected",
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
    NotProductChange,
    SupersededByNewObservation,
    InvalidObservation,
}

impl UnrecordedChangeResolutionBasis {
    /// Returns the stable value name for this unrecorded-change resolution basis.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reverted => "reverted",
            Self::CoveredByWriteTicket => "covered_by_write_ticket",
            Self::RecordedAsExpectedWrite => "recorded_as_expected_write",
            Self::AcceptedByUser => "accepted_by_user",
            Self::NotProductChange => "not_product_change",
            Self::SupersededByNewObservation => "superseded_by_new_observation",
            Self::InvalidObservation => "invalid_observation",
        }
    }
}

/// State reference discriminator values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StateRecordKind {
    ProjectState,
    Task,
    ChangeUnit,
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

/// Canonical persisted Task close summary.
///
/// Every Task records an explicit close reason, including `none` while the
/// Task is open. The remaining members are present only when their owning
/// close workflow has produced them.
#[derive(Debug, Clone, PartialEq, Deserialize)]
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

impl UserActionChannelKind {
    /// Returns the single verified invocation basis owned by this User Channel.
    pub const fn verification_basis(self) -> &'static str {
        match self {
            Self::Cli => VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL,
        }
    }

    /// Resolves one controlled verification basis to its owning User Channel.
    pub fn from_verification_basis(verification_basis: &str) -> Option<Self> {
        match verification_basis {
            VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL => Some(Self::Cli),
            _ => None,
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
    UnsupportedContract,
}

impl FailureCategory {
    /// Returns the stable machine-readable category identifier.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rejected => "rejected",
            Self::NotAllowed => "not_allowed",
            Self::Unavailable => "unavailable",
            Self::Degraded => "degraded",
            Self::Corrupt => "corrupt",
            Self::UnsupportedContract => "unsupported_contract",
        }
    }
}

/// Public API error code values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ValidationFailed,
    UnsupportedContract,
    PersistedDataCorrupt,
    StateVersionConflict,
    McpUnavailable,
    InvocationContextMismatch,
    NoActiveTask,
    NoActiveChangeUnit,
    BaselineStale,
    ScopeRequired,
    ScopeViolation,
    WriteTicketRequired,
    WriteTicketInvalid,
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
    OperationResultUnavailable,
}

impl ErrorCode {
    /// Returns the failure category carried by a public `ToolError` with this code.
    pub const fn failure_category(self) -> FailureCategory {
        match self {
            Self::UnsupportedContract => FailureCategory::UnsupportedContract,
            Self::PersistedDataCorrupt => FailureCategory::Corrupt,
            Self::McpUnavailable
            | Self::OperationResultUnavailable
            | Self::ProjectionStale
            | Self::ArtifactMissing
            | Self::ValidatorFailed => FailureCategory::Unavailable,
            Self::ScopeViolation
            | Self::ApprovalDenied
            | Self::AutonomyBoundaryExceeded
            | Self::CapabilityInsufficient
            | Self::EvidenceInsufficient
            | Self::ResidualRiskNotVisible
            | Self::AcceptanceRequired => FailureCategory::NotAllowed,
            Self::ValidationFailed
            | Self::StateVersionConflict
            | Self::InvocationContextMismatch
            | Self::NoActiveTask
            | Self::NoActiveChangeUnit
            | Self::BaselineStale
            | Self::ScopeRequired
            | Self::WriteTicketRequired
            | Self::WriteTicketInvalid
            | Self::ApprovalExpired
            | Self::ApprovalRequired
            | Self::DecisionUnresolved
            | Self::DecisionRequired => FailureCategory::Rejected,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Duration, Utc};
    use serde_json::json;

    use super::{
        AuthorityNextActor, ErrorCode, FailureCategory, ObservationConfidence, ObservedEffectKind,
        RequestedControlLevel, TaskControlLevel, UnrecordedChangeConfidence, UtcTimestamp,
        WriteTicketEffect, WriteTicketInvalidationReason, WriteTicketState, WriteTicketStatus,
    };

    #[test]
    fn failure_categories_have_exact_machine_readable_names() {
        for (category, expected) in [
            (FailureCategory::Rejected, "rejected"),
            (FailureCategory::NotAllowed, "not_allowed"),
            (FailureCategory::Unavailable, "unavailable"),
            (FailureCategory::Degraded, "degraded"),
            (FailureCategory::Corrupt, "corrupt"),
            (FailureCategory::UnsupportedContract, "unsupported_contract"),
        ] {
            assert_eq!(category.as_str(), expected);
            assert_eq!(
                serde_json::to_value(category).expect("category serializes"),
                expected
            );
        }
    }

    #[test]
    fn public_error_codes_select_one_explicit_failure_category() {
        assert_eq!(
            ErrorCode::UnsupportedContract.failure_category(),
            FailureCategory::UnsupportedContract
        );
        assert_eq!(
            ErrorCode::PersistedDataCorrupt.failure_category(),
            FailureCategory::Corrupt
        );
        assert_eq!(
            ErrorCode::McpUnavailable.failure_category(),
            FailureCategory::Unavailable
        );
        assert_eq!(
            ErrorCode::ScopeViolation.failure_category(),
            FailureCategory::NotAllowed
        );
        assert_eq!(
            ErrorCode::NoActiveChangeUnit.failure_category(),
            FailureCategory::Rejected
        );
    }

    #[test]
    fn tool_error_decode_rejects_a_mismatched_category() {
        let mismatch = serde_json::json!({
            "category": "unavailable",
            "code": "NO_ACTIVE_CHANGE_UNIT",
            "message": "fixture",
            "retryable": false,
            "details": null,
        });

        assert!(serde_json::from_value::<crate::ToolError>(mismatch).is_err());
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
                serde_json::to_value(UnrecordedChangeConfidence::Suspected).unwrap(),
                json!("suspected"),
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
