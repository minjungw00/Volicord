use std::{error::Error, fmt, str::FromStr};

use chrono::{DateTime, SecondsFormat, Utc};
use schemars::{
    gen::SchemaGenerator,
    schema::{InstanceType, Schema, SchemaObject, SingleOrVec},
    JsonSchema,
};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::ids::AgentConnectionId;

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

/// Supported public Volicord method names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum MethodName {
    #[serde(rename = "volicord.intake")]
    Intake,
    #[serde(rename = "volicord.update_scope")]
    UpdateScope,
    #[serde(rename = "volicord.status")]
    Status,
    #[serde(rename = "volicord.prepare_write")]
    PrepareWrite,
    #[serde(rename = "volicord.stage_artifact")]
    StageArtifact,
    #[serde(rename = "volicord.record_run")]
    RecordRun,
    #[serde(rename = "volicord.request_user_judgment")]
    RequestUserJudgment,
    #[serde(rename = "volicord.record_user_judgment")]
    RecordUserJudgment,
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
            Self::PrepareWrite => "volicord.prepare_write",
            Self::StageArtifact => "volicord.stage_artifact",
            Self::RecordRun => "volicord.record_run",
            Self::RequestUserJudgment => "volicord.request_user_judgment",
            Self::RecordUserJudgment => "volicord.record_user_judgment",
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
    RequestUserJudgment,
    RecordUserJudgment,
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

/// Controlled adapter-binding basis value for MCP stdio sessions.
pub const VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING: &str = "mcp_stdio_connection_binding";

/// Controlled adapter-binding basis value for MCP Streamable HTTP sessions.
pub const VERIFICATION_BASIS_MCP_STREAMABLE_HTTP_CONNECTION_BINDING: &str =
    "mcp_streamable_http_connection_binding";

/// Controlled adapter-binding basis value for direct CLI invocation.
pub const VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL: &str = "cli_direct_user_channel";

/// Controlled User Channel basis value for host prompt-submit hook capture.
pub const VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK: &str = "user_prompt_submit_hook";

/// Controlled User Channel basis value for MCP elicitation capture.
pub const VERIFICATION_BASIS_MCP_ELICITATION_USER_CHANNEL: &str = "mcp_elicitation_user_channel";

/// Controlled User Channel basis value for local loopback web consent capture.
pub const VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB: &str = "local_user_local_web";

/// Controlled binding basis value for repository tests and fixtures.
pub const VERIFICATION_BASIS_TEST_FIXTURE_BINDING: &str = "test_fixture_binding";

/// Builds the copy-paste chat verification code for one pending judgment and connection.
pub fn chat_judgment_verification_code(
    project_id: &str,
    task_id: &str,
    judgment_id: &str,
    requested_at: &str,
    connection_id: &str,
) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

    let mut hasher = Sha256::new();
    for part in [
        "chat_judgment_verification_code",
        project_id,
        task_id,
        judgment_id,
        requested_at,
        connection_id,
    ] {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    let mut bits = 0_u64;
    for byte in digest.iter().take(5) {
        bits = (bits << 8) | u64::from(*byte);
    }

    let mut code = String::with_capacity(7);
    code.push('#');
    for shift in (2..=7).rev() {
        let index = ((bits >> (shift * 5)) & 0b11111) as usize;
        code.push(char::from(ALPHABET[index]));
    }
    code
}

/// Baseline actor assurance level for cooperative Agent Connection provenance.
pub const ACTOR_ASSURANCE_AGENT_CONNECTION_COOPERATIVE: &str = "agent_connection_cooperative";

/// Host family associated with an Agent Connection or guard record.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostKind {
    Codex,
    ClaudeCode,
    Generic,
    Custom(String),
}

impl HostKind {
    /// Returns the stable host-kind string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude_code",
            Self::Generic => "generic",
            Self::Custom(value) => value.as_str(),
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
        if raw.trim().is_empty() || raw.contains('\0') {
            return Err(HostKindParseError);
        }
        Ok(match raw {
            "codex" => Self::Codex,
            "claude_code" => Self::ClaudeCode,
            "generic" => Self::Generic,
            value => Self::Custom(value.to_owned()),
        })
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
        formatter.write_str("host_kind must be a non-empty string without NUL bytes")
    }
}

impl Error for HostKindParseError {}

/// Guard integration mode recorded for a connection or session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardMode {
    McpOnly,
    Guarded,
    Managed,
}

impl GuardMode {
    /// Returns the stable value name for this guard mode.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::McpOnly => "mcp_only",
            Self::Guarded => "guarded",
            Self::Managed => "managed",
        }
    }
}

/// Guard decision recorded for a guarded operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardDecision {
    Allow,
    Deny,
    Warn,
    InjectContext,
}

impl GuardDecision {
    /// Returns the stable value name for this guard decision.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::Warn => "warn",
            Self::InjectContext => "inject_context",
        }
    }
}

/// Local guard-installation lifecycle status.
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
    /// Returns the stable value name for this guard-installation status.
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

/// Derived local guard configuration health.
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
    /// Returns the stable value name for this guard-configuration status.
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

/// Derived local guard runtime-observation health.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardObservationStatus {
    NotObserved,
    Observed,
    StaleObservation,
}

impl GuardObservationStatus {
    /// Returns the stable value name for this guard-observation status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotObserved => "not_observed",
            Self::Observed => "observed",
            Self::StaleObservation => "stale_observation",
        }
    }
}

/// Derived effective guard health used by close-readiness checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuardEffectiveStatus {
    Inactive,
    ActionRequired,
    Active,
    Degraded,
    Broken,
}

impl GuardEffectiveStatus {
    /// Returns the stable value name for this effective guard status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::ActionRequired => "action_required",
            Self::Active => "active",
            Self::Degraded => "degraded",
            Self::Broken => "broken",
        }
    }
}

/// Session-level Product Repository watch availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionWatchStatus {
    Disabled,
    Active,
    Degraded,
    Unavailable,
}

impl SessionWatchStatus {
    /// Returns the stable value name for this session-watch status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Active => "active",
            Self::Degraded => "degraded",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Derived prompt-capture availability for guarded User Channel chat commands.
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

    /// Returns true when chat judgment commands may be presented or recorded.
    pub const fn allows_chat_judgment_commands(self) -> bool {
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

/// Resolution basis for an unrecorded Product Repository change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnrecordedChangeResolutionBasis {
    Reverted,
    CoveredByWriteReadiness,
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
            Self::CoveredByWriteReadiness => "covered_by_write_readiness",
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
    WriteCheck,
    UserJudgment,
    Run,
    EvidenceSummary,
    EvidenceObservation,
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

/// Change Unit effect contract values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChangeUnitEffectKind {
    ProductFileWrite,
    ArtifactRegistration,
    RunRecording,
    UserJudgmentRequest,
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

/// Prepare-write Write Check effect values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteCheckEffect {
    None,
    WouldCreate,
    Created,
}

/// Write Check status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteCheckStatus {
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
    UserJudgment,
    PendingUserJudgment,
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

/// Evidence observation source-kind values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSourceKind {
    AgentReport,
    ConnectionObservation,
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
    RegisteredConnectionObserved,
    ExternalToolResult,
    UserObserved,
    Unverified,
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
    InvocationContextMismatch,
    NoActiveTask,
    NoActiveChangeUnit,
    BaselineStale,
    ScopeRequired,
    ScopeViolation,
    WriteCheckRequired,
    WriteCheckInvalid,
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
