//! Canonical serialized Agent Connection verification report.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::{Map, Number, Value};

use crate::{
    diagnostics::DiagnosticFindingId, schema::JsonObject, tool_names::AgentToolId,
    values::UtcTimestamp,
};

/// Maximum number of required checks in one connection report.
pub const MAX_CONNECTION_CHECKS: usize = 64;
/// Maximum number of top-level steps in one integration activation plan.
pub const MAX_ACTIVATION_STEPS: usize = 32;
/// Maximum number of prerequisite check edges on one connection check.
pub const MAX_CONNECTION_CHECK_DEPENDENCIES: usize = 16;
/// Maximum number of independent root-finding references on one check.
pub const MAX_CONNECTION_CHECK_CAUSES: usize = 32;
/// Maximum UTF-8 byte length of a check ID, action ID, or check code.
pub const MAX_CONNECTION_CODE_BYTES: usize = 128;
/// Maximum UTF-8 byte length of user-visible report text.
pub const MAX_CONNECTION_TEXT_BYTES: usize = 4_096;
/// Maximum serialized JSON byte length of one check-details object.
pub const MAX_CONNECTION_DETAILS_BYTES: usize = 16 * 1_024;
/// Maximum serialized JSON byte length of one complete report.
pub const MAX_CONNECTION_REPORT_BYTES: usize = 64 * 1_024;

/// Canonical aggregate status of an Agent Connection verification report.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    /// Every applicable required check passed and no check is waiting.
    Complete,
    /// No required check failed and at least one remains pending.
    ActionRequired,
    /// At least one required check failed or is blocked by a failed prerequisite.
    Failed,
}

/// Current activation of the project-local host hook source.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum HookActivationState {
    /// The host exposes no authoritative state and no decisive current event exists.
    Unknown,
    /// Current setup created or changed the hook definition.
    ReviewRequiredBySetup,
    /// A current-definition event was observed for the current installation and revision.
    EffectiveByObservation,
    /// Current host evidence identifies a managed policy source.
    ManagedByPolicy,
    /// One invocation explicitly bypassed hook trust without proving persisted activation.
    BypassedForInvocation,
    /// Current host configuration explicitly disables hooks.
    Disabled,
}

/// Decisive host-owned hook evidence, when the host exposes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HostHookActivationEvidence {
    ManagedByPolicy,
    BypassedForInvocation,
    Disabled,
}

/// Typed inputs used to derive project-local hook-source activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HookActivationEvidence {
    pub setup_changed_definition: bool,
    pub host: Option<HostHookActivationEvidence>,
    pub current_definition_event_observed: bool,
}

impl HookActivationState {
    /// Returns the stable serialized spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::ReviewRequiredBySetup => "review_required_by_setup",
            Self::EffectiveByObservation => "effective_by_observation",
            Self::ManagedByPolicy => "managed_by_policy",
            Self::BypassedForInvocation => "bypassed_for_invocation",
            Self::Disabled => "disabled",
        }
    }

    pub fn from_stable_str(value: &str) -> Option<Self> {
        match value {
            "unknown" => Some(Self::Unknown),
            "review_required_by_setup" => Some(Self::ReviewRequiredBySetup),
            "effective_by_observation" => Some(Self::EffectiveByObservation),
            "managed_by_policy" => Some(Self::ManagedByPolicy),
            "bypassed_for_invocation" => Some(Self::BypassedForInvocation),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }

    /// Derives activation without inventing a persisted host trust state.
    pub const fn from_evidence(evidence: HookActivationEvidence) -> Self {
        if matches!(evidence.host, Some(HostHookActivationEvidence::Disabled)) {
            Self::Disabled
        } else if evidence.setup_changed_definition {
            Self::ReviewRequiredBySetup
        } else if matches!(
            evidence.host,
            Some(HostHookActivationEvidence::ManagedByPolicy)
        ) {
            Self::ManagedByPolicy
        } else if matches!(
            evidence.host,
            Some(HostHookActivationEvidence::BypassedForInvocation)
        ) {
            Self::BypassedForInvocation
        } else if evidence.current_definition_event_observed {
            Self::EffectiveByObservation
        } else {
            Self::Unknown
        }
    }
}

/// Current stage of the Agent Connection activation workflow.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationActivationState {
    /// Managed configuration exists, but no later activation stage is yet decisive.
    Configured,
    /// The managed host must reload the current configuration.
    HostReloadRequired,
    /// Hook review is required or hook-source activation remains unknown.
    HookReviewRequiredOrUnknown,
    /// Current managed-host MCP session and capability evidence is incomplete.
    McpObservationRequired,
    /// The first-party correlated Guard verification remains incomplete.
    GuardVerificationRequired,
    /// Every current activation check is complete.
    Complete,
    /// A required activation or diagnostic check failed.
    Failed,
}

impl IntegrationActivationState {
    /// Returns the stable serialized spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Configured => "configured",
            Self::HostReloadRequired => "host_reload_required",
            Self::HookReviewRequiredOrUnknown => "hook_review_required_or_unknown",
            Self::McpObservationRequired => "mcp_observation_required",
            Self::GuardVerificationRequired => "guard_verification_required",
            Self::Complete => "complete",
            Self::Failed => "failed",
        }
    }
}

impl ConnectionStatus {
    /// Returns the stable serialized spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::ActionRequired => "action_required",
            Self::Failed => "failed",
        }
    }
}

/// Canonical status of one required connection check.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionCheckStatus {
    /// The required check succeeded.
    Passed,
    /// A required observation is still outstanding and no failed prerequisite prevents it.
    Pending,
    /// The required check failed.
    Failed,
    /// A failed prerequisite prevented this check from running or being observed.
    Blocked,
    /// The check does not apply to this connection or profile.
    NotApplicable,
}

impl ConnectionCheckStatus {
    /// Returns the stable serialized spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Pending => "pending",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::NotApplicable => "not_applicable",
        }
    }
}

/// Closed current-product vocabulary for one connection check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionCheckKind {
    /// One requested structured diagnostic finding was looked up.
    DiagnosticLookup,
    /// No completed verification report exists yet.
    VerificationNotRun,
    /// Managed host configuration matches its canonical plan.
    ManagedConfig,
    /// The host executable can be discovered and probed.
    HostExecutable,
    /// The Volicord MCP server passes the CLI-owned self-test.
    McpServer,
    /// The managed host started the configured Volicord MCP process.
    ProcessStartup,
    /// A managed-host process loaded the current connection revision.
    HostReload,
    /// Project-local hook-source activation is known or requires a bounded host action.
    HookSourceActivation,
    /// A current managed-host session completed initialization.
    HostSession,
    /// The latest current managed-host attempt is healthy.
    ManagedSessionHealth,
    /// A current managed host exposes every required tool.
    RequiredTools,
    /// One current session completed the full managed capability proof.
    ManagedCapabilityProof,
    /// A current managed host completed the designated safe tool call.
    ToolRoundTrip,
    /// Project trust is satisfied or not separately applicable.
    ProjectTrust,
    /// Current Guard managed-file expectations match.
    GuardFiles,
    /// The current hook definition and every configured phase have ambient coverage.
    AmbientHookCoverage,
    /// Current required Guard phases were observed.
    GuardObservation,
    /// One current managed turn completed the correlated Guard verification workflow.
    CorrelatedGuardVerification,
    /// A setup plan is ready to apply or already matches.
    SetupPlan,
    /// A connection-mode transition was planned or applied.
    ModeTransition,
    /// Connection membership or removal was planned or applied.
    ConnectionRemoval,
    /// One authoritative MCP runtime session was looked up.
    RuntimeSessionLookup,
}

impl ConnectionCheckKind {
    /// Every current check kind in canonical serialized-spelling order.
    pub const ALL: [Self; 22] = [
        Self::ConnectionRemoval,
        Self::DiagnosticLookup,
        Self::GuardFiles,
        Self::AmbientHookCoverage,
        Self::GuardObservation,
        Self::CorrelatedGuardVerification,
        Self::HookSourceActivation,
        Self::HostExecutable,
        Self::HostReload,
        Self::HostSession,
        Self::ManagedCapabilityProof,
        Self::ManagedConfig,
        Self::ManagedSessionHealth,
        Self::McpServer,
        Self::ModeTransition,
        Self::ProcessStartup,
        Self::ProjectTrust,
        Self::RequiredTools,
        Self::RuntimeSessionLookup,
        Self::SetupPlan,
        Self::ToolRoundTrip,
        Self::VerificationNotRun,
    ];

    /// Returns the stable serialized spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DiagnosticLookup => "diagnostic_lookup",
            Self::VerificationNotRun => "verification_not_run",
            Self::ManagedConfig => "managed_config",
            Self::HostExecutable => "host_executable",
            Self::McpServer => "mcp_server",
            Self::ProcessStartup => "process_startup",
            Self::HostReload => "host_reload",
            Self::HookSourceActivation => "hook_source_activation",
            Self::HostSession => "host_session",
            Self::ManagedSessionHealth => "managed_session_health",
            Self::RequiredTools => "required_tools",
            Self::ManagedCapabilityProof => "managed_capability_proof",
            Self::ToolRoundTrip => "tool_round_trip",
            Self::ProjectTrust => "project_trust",
            Self::GuardFiles => "guard_files",
            Self::AmbientHookCoverage => "ambient_hook_coverage",
            Self::GuardObservation => "guard_observation",
            Self::CorrelatedGuardVerification => "correlated_guard_verification",
            Self::SetupPlan => "setup_plan",
            Self::ModeTransition => "mode_transition",
            Self::ConnectionRemoval => "connection_removal",
            Self::RuntimeSessionLookup => "runtime_session_lookup",
        }
    }

    /// Returns the canonical prerequisite edges for this check kind.
    pub const fn dependencies(self) -> &'static [Self] {
        match self {
            Self::McpServer | Self::ProcessStartup | Self::HostReload => &[Self::ManagedConfig],
            Self::HookSourceActivation => &[Self::HostReload],
            Self::HostSession => &[Self::ProcessStartup],
            Self::ManagedSessionHealth => &[Self::HostReload],
            Self::ToolRoundTrip => &[Self::RequiredTools],
            Self::ManagedCapabilityProof => &[Self::ManagedSessionHealth],
            Self::AmbientHookCoverage => &[Self::HookSourceActivation],
            Self::GuardObservation => &[Self::AmbientHookCoverage],
            Self::CorrelatedGuardVerification => &[Self::AmbientHookCoverage],
            Self::VerificationNotRun
            | Self::DiagnosticLookup
            | Self::ManagedConfig
            | Self::HostExecutable
            | Self::RequiredTools
            | Self::ProjectTrust
            | Self::GuardFiles
            | Self::SetupPlan
            | Self::ModeTransition
            | Self::ConnectionRemoval
            | Self::RuntimeSessionLookup => &[],
        }
    }
}

impl PartialOrd for ConnectionCheckKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ConnectionCheckKind {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

/// Strict, bounded JSON object containing observed check facts.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ConnectionCheckDetails(JsonObject);

impl ConnectionCheckDetails {
    /// Validates a details object without normalizing it.
    pub fn try_new(value: JsonObject) -> Result<Self, ConnectionVerificationError> {
        let details = Self(value);
        validate_details_size(&details)?;
        Ok(details)
    }

    /// Returns the underlying JSON object.
    pub fn as_object(&self) -> &JsonObject {
        &self.0
    }

    /// Consumes the wrapper and returns the JSON object.
    pub fn into_object(self) -> JsonObject {
        self.0
    }
}

impl JsonSchema for ConnectionCheckDetails {
    fn schema_name() -> String {
        "ConnectionCheckDetails".to_owned()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        JsonObject::json_schema(generator)
    }
}

impl<'de> Deserialize<'de> for ConnectionCheckDetails {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = StrictJsonValue::deserialize(deserializer)?.0;
        let Value::Object(object) = value else {
            return Err(de::Error::custom(
                "connection check details must be a JSON object",
            ));
        };
        Self::try_new(object).map_err(de::Error::custom)
    }
}

/// One required connection check and its bounded observed facts.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct ConnectionCheck {
    id: ConnectionCheckKind,
    status: ConnectionCheckStatus,
    depends_on: Vec<ConnectionCheckKind>,
    cause_finding_ids: Vec<DiagnosticFindingId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<ConnectionCheckDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<UtcTimestamp>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionCheckWire {
    id: ConnectionCheckKind,
    status: ConnectionCheckStatus,
    depends_on: Vec<ConnectionCheckKind>,
    cause_finding_ids: Vec<DiagnosticFindingId>,
    code: Option<String>,
    summary: String,
    details: Option<ConnectionCheckDetails>,
    observed_at: Option<UtcTimestamp>,
}

impl<'de> Deserialize<'de> for ConnectionCheck {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ConnectionCheckWire::deserialize(deserializer)?;
        if wire.depends_on.len() > MAX_CONNECTION_CHECK_DEPENDENCIES {
            return Err(de::Error::custom(
                "connection check has too many dependency edges",
            ));
        }
        if wire.depends_on.as_slice() != wire.id.dependencies() {
            return Err(de::Error::custom(format!(
                "connection check {} does not use its canonical dependency edges",
                wire.id.as_str()
            )));
        }
        Self::try_new(
            wire.id,
            wire.status,
            wire.cause_finding_ids,
            wire.code,
            wire.summary,
            wire.details,
            wire.observed_at,
        )
        .map_err(de::Error::custom)
    }
}

impl ConnectionCheck {
    /// Validates and constructs one required check.
    pub fn try_new(
        id: ConnectionCheckKind,
        status: ConnectionCheckStatus,
        mut cause_finding_ids: Vec<DiagnosticFindingId>,
        code: Option<String>,
        summary: impl Into<String>,
        details: Option<ConnectionCheckDetails>,
        observed_at: Option<UtcTimestamp>,
    ) -> Result<Self, ConnectionVerificationError> {
        let depends_on = id.dependencies().to_vec();
        if cause_finding_ids.len() > MAX_CONNECTION_CHECK_CAUSES {
            return Err(invalid("connection check has too many root-cause findings"));
        }
        cause_finding_ids.sort();
        if cause_finding_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(invalid(
                "connection check contains duplicate root-cause finding ids",
            ));
        }
        if status == ConnectionCheckStatus::Blocked && cause_finding_ids.is_empty() {
            return Err(invalid(
                "blocked connection check requires at least one root-cause finding id",
            ));
        }
        if !matches!(
            status,
            ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked
        ) && !cause_finding_ids.is_empty()
        {
            return Err(invalid(
                "only failed or blocked connection checks may reference root-cause findings",
            ));
        }
        if let Some(code) = code.as_deref() {
            validate_code("check code", code)?;
        }
        let summary = summary.into();
        validate_text("check summary", &summary)?;
        if let Some(details) = details.as_ref() {
            validate_details_size(details)?;
        }
        if let Some(observed_at) = observed_at.as_ref() {
            observed_at
                .ensure_canonical_rfc3339_representable()
                .map_err(|_| {
                    invalid("check observed_at is outside the canonical timestamp range")
                })?;
        }
        Ok(Self {
            id,
            status,
            depends_on,
            cause_finding_ids,
            code,
            summary,
            details,
            observed_at,
        })
    }

    /// Returns the stable check ID.
    pub const fn id(&self) -> ConnectionCheckKind {
        self.id
    }

    /// Returns the required check status.
    pub const fn status(&self) -> ConnectionCheckStatus {
        self.status
    }

    /// Returns prerequisite check edges in canonical check-ID order.
    pub fn depends_on(&self) -> &[ConnectionCheckKind] {
        &self.depends_on
    }

    /// Returns independent typed root findings in canonical finding-ID order.
    pub fn cause_finding_ids(&self) -> &[DiagnosticFindingId] {
        &self.cause_finding_ids
    }

    /// Returns the optional machine-readable code.
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Returns the user-visible summary.
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// Returns the optional observed facts.
    pub fn details(&self) -> Option<&ConnectionCheckDetails> {
        self.details.as_ref()
    }

    /// Returns the optional observation time.
    pub fn observed_at(&self) -> Option<&UtcTimestamp> {
        self.observed_at.as_ref()
    }

    /// Replaces direct failure references with their computed independent roots.
    pub fn with_cause_finding_ids(
        mut self,
        mut cause_finding_ids: Vec<DiagnosticFindingId>,
    ) -> Result<Self, ConnectionVerificationError> {
        if !matches!(
            self.status,
            ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked
        ) {
            return Err(invalid(
                "only failed or blocked connection checks may reference root-cause findings",
            ));
        }
        if cause_finding_ids.len() > MAX_CONNECTION_CHECK_CAUSES {
            return Err(invalid("connection check has too many root-cause findings"));
        }
        cause_finding_ids.sort();
        cause_finding_ids.dedup();
        if self.status == ConnectionCheckStatus::Blocked && cause_finding_ids.is_empty() {
            return Err(invalid(
                "blocked connection check requires at least one root-cause finding id",
            ));
        }
        self.cause_finding_ids = cause_finding_ids;
        Ok(self)
    }

    /// Converts a check that could not run into a typed blocked state.
    pub fn blocked_by(
        mut self,
        mut cause_finding_ids: Vec<DiagnosticFindingId>,
    ) -> Result<Self, ConnectionVerificationError> {
        if cause_finding_ids.is_empty() {
            return Err(invalid(
                "blocked connection check requires at least one root-cause finding id",
            ));
        }
        if cause_finding_ids.len() > MAX_CONNECTION_CHECK_CAUSES {
            return Err(invalid("connection check has too many root-cause findings"));
        }
        cause_finding_ids.sort();
        cause_finding_ids.dedup();
        self.status = ConnectionCheckStatus::Blocked;
        self.cause_finding_ids = cause_finding_ids;
        self.code = Some("blocked_by_failed_prerequisite".to_owned());
        self.summary = "Check is blocked by a failed prerequisite".to_owned();
        self.observed_at = None;
        Ok(self)
    }
}

/// Stable semantic identifier for one top-level integration activation step.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ActivationStepId {
    ReloadCodex,
    ReviewProjectHooks,
    RequestIntegrationVerification,
    ReadConnectionStatus,
    RunOptionalActiveDiagnostics,
    RepairHookContract,
    RepairManagedConfiguration,
}

impl ActivationStepId {
    pub const ALL: [Self; 7] = [
        Self::ReloadCodex,
        Self::ReviewProjectHooks,
        Self::RequestIntegrationVerification,
        Self::ReadConnectionStatus,
        Self::RunOptionalActiveDiagnostics,
        Self::RepairHookContract,
        Self::RepairManagedConfiguration,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReloadCodex => "reload_codex",
            Self::ReviewProjectHooks => "review_project_hooks",
            Self::RequestIntegrationVerification => "request_integration_verification",
            Self::ReadConnectionStatus => "read_connection_status",
            Self::RunOptionalActiveDiagnostics => "run_optional_active_diagnostics",
            Self::RepairHookContract => "repair_hook_contract",
            Self::RepairManagedConfiguration => "repair_managed_configuration",
        }
    }

    const fn workflow_order(self) -> u8 {
        match self {
            Self::RepairManagedConfiguration => 0,
            Self::ReloadCodex => 10,
            Self::ReviewProjectHooks => 20,
            Self::RepairHookContract => 25,
            Self::RequestIntegrationVerification => 30,
            Self::ReadConnectionStatus => 40,
            Self::RunOptionalActiveDiagnostics => 50,
        }
    }
}

/// Actor that initiates or executes an activation step.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ActivationActor {
    User,
    Host,
    Volicord,
    Agent,
}

impl ActivationActor {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Host => "host",
            Self::Volicord => "volicord",
            Self::Agent => "agent",
        }
    }
}

/// Channel through which one activation step executes.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ActivationExecutionChannel {
    Cli,
    CodexUi,
    CodexChat,
    McpTool,
}

impl ActivationExecutionChannel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::CodexUi => "codex_ui",
            Self::CodexChat => "codex_chat",
            Self::McpTool => "mcp_tool",
        }
    }
}

/// Condition that permits one nested agent tool step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentSequenceCondition {
    Always,
    WorkflowAwaitingProbe,
    WorkflowAwaitingObservation,
}

/// One internal agent tool call nested under a user-level activation step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentSequenceStep {
    tool: AgentToolId,
    condition: AgentSequenceCondition,
}

impl AgentSequenceStep {
    pub const fn tool(&self) -> AgentToolId {
        self.tool
    }

    pub const fn condition(&self) -> AgentSequenceCondition {
        self.condition
    }
}

/// One bounded top-level step in the integration activation plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ActivationStep {
    id: ActivationStepId,
    initiator: ActivationActor,
    executor: ActivationActor,
    execution_channel: ActivationExecutionChannel,
    prerequisites: Vec<ActivationStepId>,
    completes_checks: Vec<ConnectionCheckKind>,
    root_finding_ids: Vec<DiagnosticFindingId>,
    instruction: String,
    diagnostic_only: bool,
    agent_sequence: Vec<AgentSequenceStep>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivationStepWire {
    id: ActivationStepId,
    initiator: ActivationActor,
    executor: ActivationActor,
    execution_channel: ActivationExecutionChannel,
    prerequisites: Vec<ActivationStepId>,
    completes_checks: Vec<ConnectionCheckKind>,
    root_finding_ids: Vec<DiagnosticFindingId>,
    instruction: String,
    diagnostic_only: bool,
    agent_sequence: Vec<AgentSequenceStep>,
}

impl<'de> Deserialize<'de> for ActivationStep {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ActivationStepWire::deserialize(deserializer)?;
        let prerequisites = wire.prerequisites.clone();
        let completes_checks = wire.completes_checks.clone();
        let root_finding_ids = wire.root_finding_ids.clone();
        let step = Self::try_new_with_context(
            wire.id,
            wire.initiator,
            wire.executor,
            wire.execution_channel,
            wire.prerequisites,
            wire.completes_checks,
            wire.root_finding_ids,
            wire.instruction,
            wire.diagnostic_only,
            wire.agent_sequence,
        )
        .map_err(de::Error::custom)?;
        if prerequisites != step.prerequisites
            || completes_checks != step.completes_checks
            || root_finding_ids != step.root_finding_ids
        {
            return Err(de::Error::custom(
                "activation step references are not unique and canonically ordered",
            ));
        }
        Ok(step)
    }
}

impl ActivationStep {
    pub fn try_new(
        id: ActivationStepId,
        prerequisites: Vec<ActivationStepId>,
        instruction: impl Into<String>,
    ) -> Result<Self, ConnectionVerificationError> {
        let context = canonical_activation_step_context(id);
        Self::try_new_with_context(
            id,
            context.initiator,
            context.executor,
            context.channel,
            prerequisites,
            context.completes_checks.to_vec(),
            Vec::new(),
            instruction,
            context.diagnostic_only,
            context.agent_sequence.to_vec(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn try_new_with_context(
        id: ActivationStepId,
        initiator: ActivationActor,
        executor: ActivationActor,
        execution_channel: ActivationExecutionChannel,
        mut prerequisites: Vec<ActivationStepId>,
        mut completes_checks: Vec<ConnectionCheckKind>,
        mut root_finding_ids: Vec<DiagnosticFindingId>,
        instruction: impl Into<String>,
        diagnostic_only: bool,
        agent_sequence: Vec<AgentSequenceStep>,
    ) -> Result<Self, ConnectionVerificationError> {
        let instruction = instruction.into();
        validate_text("activation step instruction", &instruction)?;
        prerequisites.sort();
        prerequisites.dedup();
        completes_checks.sort();
        completes_checks.dedup();
        root_finding_ids.sort();
        root_finding_ids.dedup();
        if prerequisites.len() > MAX_CONNECTION_CHECK_DEPENDENCIES
            || completes_checks.len() > MAX_CONNECTION_CHECK_DEPENDENCIES
        {
            return Err(invalid("activation step has too many references"));
        }
        if root_finding_ids.len() > MAX_CONNECTION_CHECK_CAUSES {
            return Err(invalid("activation step has too many root findings"));
        }
        Ok(Self {
            id,
            initiator,
            executor,
            execution_channel,
            prerequisites,
            completes_checks,
            root_finding_ids,
            instruction,
            diagnostic_only,
            agent_sequence,
        })
    }

    pub fn with_root_finding_ids(
        mut self,
        mut root_finding_ids: Vec<DiagnosticFindingId>,
    ) -> Result<Self, ConnectionVerificationError> {
        root_finding_ids.sort();
        root_finding_ids.dedup();
        if root_finding_ids.len() > MAX_CONNECTION_CHECK_CAUSES {
            return Err(invalid("activation step has too many root findings"));
        }
        self.root_finding_ids = root_finding_ids;
        Ok(self)
    }

    pub const fn id(&self) -> ActivationStepId {
        self.id
    }

    pub const fn initiator(&self) -> ActivationActor {
        self.initiator
    }

    pub const fn executor(&self) -> ActivationActor {
        self.executor
    }

    pub const fn execution_channel(&self) -> ActivationExecutionChannel {
        self.execution_channel
    }

    pub fn prerequisites(&self) -> &[ActivationStepId] {
        &self.prerequisites
    }

    pub fn completes_checks(&self) -> &[ConnectionCheckKind] {
        &self.completes_checks
    }

    pub fn root_finding_ids(&self) -> &[DiagnosticFindingId] {
        &self.root_finding_ids
    }

    pub fn instruction(&self) -> &str {
        &self.instruction
    }

    pub const fn diagnostic_only(&self) -> bool {
        self.diagnostic_only
    }

    pub fn agent_sequence(&self) -> &[AgentSequenceStep] {
        &self.agent_sequence
    }
}

struct CanonicalActivationStepContext {
    initiator: ActivationActor,
    executor: ActivationActor,
    channel: ActivationExecutionChannel,
    completes_checks: &'static [ConnectionCheckKind],
    diagnostic_only: bool,
    agent_sequence: &'static [AgentSequenceStep],
}

const INTEGRATION_VERIFICATION_AGENT_SEQUENCE: [AgentSequenceStep; 4] = [
    AgentSequenceStep {
        tool: AgentToolId::LIST_PROJECTS,
        condition: AgentSequenceCondition::Always,
    },
    AgentSequenceStep {
        tool: AgentToolId::BEGIN_INTEGRATION_VERIFICATION,
        condition: AgentSequenceCondition::Always,
    },
    AgentSequenceStep {
        tool: AgentToolId::GUARD_PROBE,
        condition: AgentSequenceCondition::WorkflowAwaitingProbe,
    },
    AgentSequenceStep {
        tool: AgentToolId::GET_INTEGRATION_VERIFICATION,
        condition: AgentSequenceCondition::WorkflowAwaitingObservation,
    },
];

fn canonical_activation_step_context(id: ActivationStepId) -> CanonicalActivationStepContext {
    match id {
        ActivationStepId::ReloadCodex => CanonicalActivationStepContext {
            initiator: ActivationActor::User,
            executor: ActivationActor::Host,
            channel: ActivationExecutionChannel::CodexUi,
            completes_checks: &[ConnectionCheckKind::HostReload],
            diagnostic_only: false,
            agent_sequence: &[],
        },
        ActivationStepId::ReviewProjectHooks => CanonicalActivationStepContext {
            initiator: ActivationActor::User,
            executor: ActivationActor::User,
            channel: ActivationExecutionChannel::CodexUi,
            completes_checks: &[ConnectionCheckKind::HookSourceActivation],
            diagnostic_only: false,
            agent_sequence: &[],
        },
        ActivationStepId::RequestIntegrationVerification => CanonicalActivationStepContext {
            initiator: ActivationActor::User,
            executor: ActivationActor::Agent,
            channel: ActivationExecutionChannel::CodexChat,
            completes_checks: &[
                ConnectionCheckKind::AmbientHookCoverage,
                ConnectionCheckKind::CorrelatedGuardVerification,
                ConnectionCheckKind::ManagedCapabilityProof,
                ConnectionCheckKind::ManagedSessionHealth,
            ],
            diagnostic_only: false,
            agent_sequence: &INTEGRATION_VERIFICATION_AGENT_SEQUENCE,
        },
        ActivationStepId::ReadConnectionStatus => CanonicalActivationStepContext {
            initiator: ActivationActor::User,
            executor: ActivationActor::Volicord,
            channel: ActivationExecutionChannel::Cli,
            completes_checks: &[],
            diagnostic_only: false,
            agent_sequence: &[],
        },
        ActivationStepId::RunOptionalActiveDiagnostics => CanonicalActivationStepContext {
            initiator: ActivationActor::User,
            executor: ActivationActor::Volicord,
            channel: ActivationExecutionChannel::Cli,
            completes_checks: &[
                ConnectionCheckKind::HostExecutable,
                ConnectionCheckKind::ManagedConfig,
                ConnectionCheckKind::McpServer,
                ConnectionCheckKind::ProcessStartup,
                ConnectionCheckKind::RequiredTools,
                ConnectionCheckKind::ToolRoundTrip,
            ],
            diagnostic_only: true,
            agent_sequence: &[],
        },
        ActivationStepId::RepairHookContract => CanonicalActivationStepContext {
            initiator: ActivationActor::User,
            executor: ActivationActor::User,
            channel: ActivationExecutionChannel::CodexUi,
            completes_checks: &[
                ConnectionCheckKind::AmbientHookCoverage,
                ConnectionCheckKind::HookSourceActivation,
            ],
            diagnostic_only: false,
            agent_sequence: &[],
        },
        ActivationStepId::RepairManagedConfiguration => CanonicalActivationStepContext {
            initiator: ActivationActor::User,
            executor: ActivationActor::Volicord,
            channel: ActivationExecutionChannel::Cli,
            completes_checks: &[ConnectionCheckKind::ManagedConfig],
            diagnostic_only: false,
            agent_sequence: &[],
        },
    }
}

/// One authoritative hierarchical plan for current integration activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct IntegrationActivationPlan {
    state: IntegrationActivationState,
    required_steps: Vec<ActivationStep>,
    optional_diagnostics: Vec<ActivationStep>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IntegrationActivationPlanWire {
    state: IntegrationActivationState,
    required_steps: Vec<ActivationStep>,
    optional_diagnostics: Vec<ActivationStep>,
}

impl<'de> Deserialize<'de> for IntegrationActivationPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = IntegrationActivationPlanWire::deserialize(deserializer)?;
        let required_steps = wire.required_steps.clone();
        let optional_diagnostics = wire.optional_diagnostics.clone();
        let plan = Self::try_new(wire.state, wire.required_steps, wire.optional_diagnostics)
            .map_err(de::Error::custom)?;
        if required_steps != plan.required_steps
            || optional_diagnostics != plan.optional_diagnostics
        {
            return Err(de::Error::custom(
                "activation plan steps are not in canonical topological order",
            ));
        }
        Ok(plan)
    }
}

impl IntegrationActivationPlan {
    pub fn try_new(
        state: IntegrationActivationState,
        required_steps: Vec<ActivationStep>,
        optional_diagnostics: Vec<ActivationStep>,
    ) -> Result<Self, ConnectionVerificationError> {
        if required_steps.len() + optional_diagnostics.len() > MAX_ACTIVATION_STEPS {
            return Err(invalid("activation plan has too many steps"));
        }
        let mut seen = BTreeSet::new();
        for step in required_steps.iter().chain(&optional_diagnostics) {
            if !seen.insert(step.id) {
                return Err(invalid("activation plan contains a duplicate step id"));
            }
            if step.execution_channel == ActivationExecutionChannel::McpTool {
                return Err(invalid(
                    "nested agent tool step cannot be exposed as a top-level activation step",
                ));
            }
            validate_activation_step_context(step)?;
        }
        if required_steps.iter().any(ActivationStep::diagnostic_only) {
            return Err(invalid(
                "diagnostic-only step cannot appear in the required activation plan",
            ));
        }
        if optional_diagnostics
            .iter()
            .any(|step| !step.diagnostic_only())
        {
            return Err(invalid(
                "required activation step cannot appear in optional diagnostics",
            ));
        }
        Ok(Self {
            state,
            required_steps: topological_activation_steps(required_steps)?,
            optional_diagnostics: topological_activation_steps(optional_diagnostics)?,
        })
    }

    pub fn empty(state: IntegrationActivationState) -> Self {
        Self {
            state,
            required_steps: Vec::new(),
            optional_diagnostics: Vec::new(),
        }
    }

    pub const fn state(&self) -> IntegrationActivationState {
        self.state
    }

    pub fn required_steps(&self) -> &[ActivationStep] {
        &self.required_steps
    }

    pub fn optional_diagnostics(&self) -> &[ActivationStep] {
        &self.optional_diagnostics
    }
}

fn validate_activation_step_context(
    step: &ActivationStep,
) -> Result<(), ConnectionVerificationError> {
    let canonical = canonical_activation_step_context(step.id);
    if step.initiator != canonical.initiator
        || step.executor != canonical.executor
        || step.execution_channel != canonical.channel
        || step.completes_checks != canonical.completes_checks
        || step.diagnostic_only != canonical.diagnostic_only
        || step.agent_sequence != canonical.agent_sequence
    {
        return Err(invalid(
            "activation step initiator, executor, channel, completed checks, diagnostic class, or agent sequence does not match its stable ID",
        ));
    }
    Ok(())
}

fn topological_activation_steps(
    steps: Vec<ActivationStep>,
) -> Result<Vec<ActivationStep>, ConnectionVerificationError> {
    let indexes = steps
        .iter()
        .enumerate()
        .map(|(index, step)| (step.id, index))
        .collect::<BTreeMap<_, _>>();
    for step in &steps {
        for prerequisite in &step.prerequisites {
            if !indexes.contains_key(prerequisite) {
                return Err(invalid(format!(
                    "activation step {} has unknown prerequisite {}",
                    step.id.as_str(),
                    prerequisite.as_str()
                )));
            }
        }
    }
    let mut emitted = BTreeSet::new();
    let mut ordered = Vec::with_capacity(steps.len());
    while ordered.len() < steps.len() {
        let next = steps
            .iter()
            .filter(|step| !emitted.contains(&step.id))
            .filter(|step| {
                step.prerequisites
                    .iter()
                    .all(|prerequisite| emitted.contains(prerequisite))
            })
            .min_by_key(|step| step.id.workflow_order());
        let Some(next) = next else {
            return Err(invalid(
                "activation plan prerequisite graph contains a cycle",
            ));
        };
        emitted.insert(next.id);
        ordered.push(next.clone());
    }
    Ok(ordered)
}

/// Canonical serialized result of connection verification.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct ConnectionVerificationReport {
    status: ConnectionStatus,
    activation_state: IntegrationActivationState,
    hook_activation_state: HookActivationState,
    checked_at: UtcTimestamp,
    checks: Vec<ConnectionCheck>,
    root_cause_ids: Vec<DiagnosticFindingId>,
    activation_plan: IntegrationActivationPlan,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionVerificationReportWire {
    status: ConnectionStatus,
    activation_state: IntegrationActivationState,
    hook_activation_state: HookActivationState,
    checked_at: UtcTimestamp,
    checks: Vec<ConnectionCheck>,
    root_cause_ids: Vec<DiagnosticFindingId>,
    activation_plan: IntegrationActivationPlan,
}

impl<'de> Deserialize<'de> for ConnectionVerificationReport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ConnectionVerificationReportWire::deserialize(deserializer)?;
        Self::from_canonical_parts(
            wire.status,
            wire.activation_state,
            wire.hook_activation_state,
            wire.checked_at,
            wire.checks,
            wire.root_cause_ids,
            wire.activation_plan,
        )
        .map_err(de::Error::custom)
    }
}

impl ConnectionVerificationReport {
    /// Constructs a report with canonical checks and one typed activation plan.
    pub fn try_new(
        checked_at: UtcTimestamp,
        mut checks: Vec<ConnectionCheck>,
        activation_plan: IntegrationActivationPlan,
    ) -> Result<Self, ConnectionVerificationError> {
        checks.sort_by_key(|check| check.id);
        let status = aggregate_status(&checks);
        let hook_activation_state = derive_hook_activation_state(&checks);
        let activation_state = derive_integration_activation_state(&checks, hook_activation_state);
        let root_cause_ids = aggregate_root_causes(&checks);
        Self::from_canonical_parts(
            status,
            activation_state,
            hook_activation_state,
            checked_at,
            checks,
            root_cause_ids,
            activation_plan,
        )
    }

    /// Constructs a report with operation-local hook activation evidence.
    pub fn try_new_with_hook_activation(
        checked_at: UtcTimestamp,
        mut checks: Vec<ConnectionCheck>,
        hook_activation_state: HookActivationState,
        activation_plan: IntegrationActivationPlan,
    ) -> Result<Self, ConnectionVerificationError> {
        checks.sort_by_key(|check| check.id);
        let status = aggregate_status(&checks);
        let activation_state = derive_integration_activation_state(&checks, hook_activation_state);
        let root_cause_ids = aggregate_root_causes(&checks);
        Self::from_canonical_parts(
            status,
            activation_state,
            hook_activation_state,
            checked_at,
            checks,
            root_cause_ids,
            activation_plan,
        )
    }

    /// Synthesizes the canonical projection for a connection not yet verified.
    pub fn verification_not_run(
        checked_at: UtcTimestamp,
    ) -> Result<Self, ConnectionVerificationError> {
        let checks = vec![ConnectionCheck::try_new(
            ConnectionCheckKind::VerificationNotRun,
            ConnectionCheckStatus::Pending,
            Vec::new(),
            Some("verification_not_run".to_owned()),
            "Connection verification has not been run",
            None,
            None,
        )?];
        let state = derive_integration_activation_state(&checks, HookActivationState::Unknown);
        let plan = IntegrationActivationPlan::try_new(
            state,
            vec![ActivationStep::try_new(
                ActivationStepId::RequestIntegrationVerification,
                Vec::new(),
                "In a new managed Codex conversation, request `Run the Volicord integration verification.`",
            )?],
            vec![ActivationStep::try_new(
                ActivationStepId::RunOptionalActiveDiagnostics,
                Vec::new(),
                "Run `volicord connection verify` only when optional active diagnostics are needed",
            )?],
        )?;
        Self::try_new(checked_at, checks, plan)
    }

    /// Returns the derived aggregate status.
    pub const fn status(&self) -> ConnectionStatus {
        self.status
    }

    pub const fn activation_state(&self) -> IntegrationActivationState {
        self.activation_state
    }

    pub const fn hook_activation_state(&self) -> HookActivationState {
        self.hook_activation_state
    }

    /// Returns the report construction or projection time.
    pub fn checked_at(&self) -> &UtcTimestamp {
        &self.checked_at
    }

    /// Returns required checks in canonical ID order.
    pub fn checks(&self) -> &[ConnectionCheck] {
        &self.checks
    }

    /// Returns independent root findings in canonical finding-ID order.
    pub fn root_cause_ids(&self) -> &[DiagnosticFindingId] {
        &self.root_cause_ids
    }

    /// Returns the one authoritative current activation plan.
    pub fn activation_plan(&self) -> &IntegrationActivationPlan {
        &self.activation_plan
    }

    fn from_canonical_parts(
        status: ConnectionStatus,
        activation_state: IntegrationActivationState,
        hook_activation_state: HookActivationState,
        checked_at: UtcTimestamp,
        checks: Vec<ConnectionCheck>,
        root_cause_ids: Vec<DiagnosticFindingId>,
        activation_plan: IntegrationActivationPlan,
    ) -> Result<Self, ConnectionVerificationError> {
        checked_at
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| invalid("checked_at is outside the canonical timestamp range"))?;
        if checks.len() > MAX_CONNECTION_CHECKS {
            return Err(invalid("connection report has too many checks"));
        }
        require_canonical_check_order(&checks)?;
        validate_check_dependency_graph(&checks)?;
        let derived = aggregate_status(&checks);
        if status != derived {
            return Err(invalid(
                "connection report status does not match its checks",
            ));
        }
        if activation_state != derive_integration_activation_state(&checks, hook_activation_state) {
            return Err(invalid(
                "connection report activation_state does not match its typed checks",
            ));
        }
        if activation_plan.state() != activation_state {
            return Err(invalid(
                "connection report activation plan state does not match its typed checks",
            ));
        }
        let derived_roots = aggregate_root_causes(&checks);
        if root_cause_ids != derived_roots {
            return Err(invalid(
                "connection report root_cause_ids do not match its check graph",
            ));
        }
        let report = Self {
            status,
            activation_state,
            hook_activation_state,
            checked_at,
            checks,
            root_cause_ids,
            activation_plan,
        };
        let size = serde_json::to_vec(&report)
            .map_err(|_| invalid("connection report could not be serialized"))?
            .len();
        if size > MAX_CONNECTION_REPORT_BYTES {
            return Err(invalid(
                "connection report exceeds its serialized size bound",
            ));
        }
        Ok(report)
    }
}

fn derive_hook_activation_state(checks: &[ConnectionCheck]) -> HookActivationState {
    checks
        .iter()
        .find(|check| check.id == ConnectionCheckKind::HookSourceActivation)
        .and_then(|check| check.details.as_ref())
        .and_then(|details| details.as_object().get("activation_state"))
        .and_then(Value::as_str)
        .and_then(HookActivationState::from_stable_str)
        .unwrap_or(HookActivationState::Unknown)
}

pub fn derive_integration_activation_state(
    checks: &[ConnectionCheck],
    hook_activation_state: HookActivationState,
) -> IntegrationActivationState {
    if checks.iter().any(|check| {
        matches!(
            check.status,
            ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked
        )
    }) {
        return IntegrationActivationState::Failed;
    }
    let passed = |kind| {
        checks
            .iter()
            .find(|check| check.id == kind)
            .is_some_and(|check| {
                matches!(
                    check.status,
                    ConnectionCheckStatus::Passed | ConnectionCheckStatus::NotApplicable
                )
            })
    };
    if !passed(ConnectionCheckKind::ManagedConfig) {
        return IntegrationActivationState::Configured;
    }
    if !passed(ConnectionCheckKind::HostReload) {
        return IntegrationActivationState::HostReloadRequired;
    }
    if !matches!(
        hook_activation_state,
        HookActivationState::EffectiveByObservation | HookActivationState::ManagedByPolicy
    ) {
        return IntegrationActivationState::HookReviewRequiredOrUnknown;
    }
    if !passed(ConnectionCheckKind::ManagedSessionHealth)
        || !passed(ConnectionCheckKind::ManagedCapabilityProof)
    {
        return IntegrationActivationState::McpObservationRequired;
    }
    if !passed(ConnectionCheckKind::CorrelatedGuardVerification) {
        return IntegrationActivationState::GuardVerificationRequired;
    }
    IntegrationActivationState::Complete
}

fn aggregate_status(checks: &[ConnectionCheck]) -> ConnectionStatus {
    let recoverable_failure = |check: &ConnectionCheck| {
        check.status == ConnectionCheckStatus::Failed
            && check.id == ConnectionCheckKind::CorrelatedGuardVerification
            && check
                .details
                .as_ref()
                .and_then(|details| details.as_object().get("recoverability"))
                .and_then(Value::as_str)
                == Some("recoverable")
    };
    if checks.iter().any(|check| {
        check.status == ConnectionCheckStatus::Blocked
            || (check.status == ConnectionCheckStatus::Failed && !recoverable_failure(check))
    }) {
        ConnectionStatus::Failed
    } else if checks
        .iter()
        .any(|check| check.status == ConnectionCheckStatus::Pending || recoverable_failure(check))
    {
        ConnectionStatus::ActionRequired
    } else {
        ConnectionStatus::Complete
    }
}

fn aggregate_root_causes(checks: &[ConnectionCheck]) -> Vec<DiagnosticFindingId> {
    checks
        .iter()
        .flat_map(|check| check.cause_finding_ids.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn validate_check_dependency_graph(
    checks: &[ConnectionCheck],
) -> Result<(), ConnectionVerificationError> {
    let indexes = checks
        .iter()
        .enumerate()
        .map(|(index, check)| (check.id, index))
        .collect::<BTreeMap<_, _>>();
    let mut marks = vec![0_u8; checks.len()];
    for index in 0..checks.len() {
        visit_check_dependency(index, checks, &indexes, &mut marks)?;
    }
    for check in checks {
        let failed_dependency_causes = check
            .depends_on
            .iter()
            .filter_map(|dependency| indexes.get(dependency))
            .filter_map(|index| {
                matches!(
                    checks[*index].status,
                    ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked
                )
                .then_some(checks[*index].cause_finding_ids.iter().cloned())
            })
            .flatten()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let failed_dependency = check.depends_on.iter().any(|dependency| {
            indexes.get(dependency).is_some_and(|index| {
                matches!(
                    checks[*index].status,
                    ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked
                )
            })
        });
        if failed_dependency
            && !matches!(
                check.status,
                ConnectionCheckStatus::Blocked | ConnectionCheckStatus::NotApplicable
            )
        {
            return Err(invalid(format!(
                "connection check {} must be blocked by its failed prerequisite",
                check.id.as_str()
            )));
        }
        if check.status == ConnectionCheckStatus::Blocked && !failed_dependency {
            return Err(invalid(format!(
                "connection check {} is blocked without a failed prerequisite check",
                check.id.as_str()
            )));
        }
        if check.status == ConnectionCheckStatus::Blocked
            && check.cause_finding_ids != failed_dependency_causes
        {
            return Err(invalid(format!(
                "connection check {} root causes do not match its failed prerequisites",
                check.id.as_str()
            )));
        }
    }
    Ok(())
}

fn visit_check_dependency(
    index: usize,
    checks: &[ConnectionCheck],
    indexes: &BTreeMap<ConnectionCheckKind, usize>,
    marks: &mut [u8],
) -> Result<(), ConnectionVerificationError> {
    match marks[index] {
        1 => {
            return Err(invalid(format!(
                "connection check dependency graph contains a cycle at {}",
                checks[index].id.as_str()
            )))
        }
        2 => return Ok(()),
        _ => {}
    }
    marks[index] = 1;
    for dependency in &checks[index].depends_on {
        if let Some(dependency_index) = indexes.get(dependency).copied() {
            visit_check_dependency(dependency_index, checks, indexes, marks)?;
        }
    }
    marks[index] = 2;
    Ok(())
}

fn require_canonical_check_order(
    checks: &[ConnectionCheck],
) -> Result<(), ConnectionVerificationError> {
    let mut seen = BTreeSet::new();
    let mut previous: Option<ConnectionCheckKind> = None;
    for check in checks {
        if !seen.insert(check.id) {
            return Err(invalid("connection report contains a duplicate check id"));
        }
        if previous.is_some_and(|previous| previous >= check.id) {
            return Err(invalid("connection checks are not in canonical id order"));
        }
        previous = Some(check.id);
    }
    Ok(())
}

fn validate_code(field: &str, value: &str) -> Result<(), ConnectionVerificationError> {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return Err(invalid(format!("{field} must not be empty")));
    };
    if value.len() > MAX_CONNECTION_CODE_BYTES
        || !first.is_ascii_lowercase()
        || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(invalid(format!(
            "{field} must be 1 through {MAX_CONNECTION_CODE_BYTES} ASCII bytes matching [a-z][a-z0-9_]*"
        )));
    }
    Ok(())
}

fn validate_text(field: &str, value: &str) -> Result<(), ConnectionVerificationError> {
    if value.is_empty() || value.len() > MAX_CONNECTION_TEXT_BYTES || value.as_bytes().contains(&0)
    {
        return Err(invalid(format!(
            "{field} must be 1 through {MAX_CONNECTION_TEXT_BYTES} UTF-8 bytes and contain no NUL"
        )));
    }
    Ok(())
}

fn validate_details_size(
    details: &ConnectionCheckDetails,
) -> Result<(), ConnectionVerificationError> {
    let size = serde_json::to_vec(details)
        .map_err(|_| invalid("connection check details could not be serialized"))?
        .len();
    if size > MAX_CONNECTION_DETAILS_BYTES {
        Err(invalid(
            "connection check details exceed their serialized size bound",
        ))
    } else {
        Ok(())
    }
}

/// Validation failure for a canonical connection report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionVerificationError {
    detail: String,
}

impl ConnectionVerificationError {
    /// Returns bounded implementation-facing failure detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for ConnectionVerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for ConnectionVerificationError {}

fn invalid(detail: impl Into<String>) -> ConnectionVerificationError {
    ConnectionVerificationError {
        detail: detail.into(),
    }
}

struct StrictJsonValue(Value);

impl<'de> Deserialize<'de> for StrictJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_any(StrictJsonValueVisitor)
            .map(Self)
    }
}

struct StrictJsonValueVisitor;

impl<'de> Visitor<'de> for StrictJsonValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        StrictJsonValue::deserialize(deserializer).map(|value| value.0)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<StrictJsonValue>()? {
            values.push(value.0);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        while let Some((key, value)) = object.next_entry::<String, StrictJsonValue>()? {
            if values.insert(key.clone(), value.0).is_some() {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object key {key}"
                )));
            }
        }
        Ok(Value::Object(values))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn timestamp() -> UtcTimestamp {
        UtcTimestamp::parse("2026-07-18T00:00:00Z").expect("test timestamp")
    }

    fn check(id: ConnectionCheckKind, status: ConnectionCheckStatus) -> ConnectionCheck {
        ConnectionCheck::try_new(
            id,
            status,
            Vec::new(),
            Some(format!("{}_result", id.as_str())),
            format!("{} summary", id.as_str()),
            None,
            None,
        )
        .expect("test check")
    }

    fn step(id: ActivationStepId, prerequisites: Vec<ActivationStepId>) -> ActivationStep {
        ActivationStep::try_new(id, prerequisites, format!("{} instruction", id.as_str()))
            .expect("test activation step")
    }

    fn empty_plan(
        checks: &[ConnectionCheck],
        hook_activation_state: HookActivationState,
    ) -> IntegrationActivationPlan {
        IntegrationActivationPlan::empty(derive_integration_activation_state(
            checks,
            hook_activation_state,
        ))
    }

    #[test]
    fn every_current_check_kind_round_trips_exact_json() {
        assert_eq!(ConnectionCheckKind::ALL.len(), 22);
        assert_eq!(
            ConnectionCheckKind::CorrelatedGuardVerification.dependencies(),
            &[ConnectionCheckKind::AmbientHookCoverage]
        );
        let expected = [
            "connection_removal",
            "diagnostic_lookup",
            "guard_files",
            "ambient_hook_coverage",
            "guard_observation",
            "correlated_guard_verification",
            "hook_source_activation",
            "host_executable",
            "host_reload",
            "host_session",
            "managed_capability_proof",
            "managed_config",
            "managed_session_health",
            "mcp_server",
            "mode_transition",
            "process_startup",
            "project_trust",
            "required_tools",
            "runtime_session_lookup",
            "setup_plan",
            "tool_round_trip",
            "verification_not_run",
        ];
        for (kind, expected) in ConnectionCheckKind::ALL.into_iter().zip(expected) {
            assert_eq!(serde_json::to_value(kind).unwrap(), json!(expected));
            assert_eq!(
                serde_json::from_value::<ConnectionCheckKind>(json!(expected)).unwrap(),
                kind
            );
        }
    }

    #[test]
    fn every_current_activation_step_id_round_trips_exact_json() {
        let expected = [
            "reload_codex",
            "review_project_hooks",
            "request_integration_verification",
            "read_connection_status",
            "run_optional_active_diagnostics",
            "repair_hook_contract",
            "repair_managed_configuration",
        ];
        for (id, expected) in ActivationStepId::ALL.into_iter().zip(expected) {
            assert_eq!(serde_json::to_value(id).unwrap(), json!(expected));
            assert_eq!(
                serde_json::from_value::<ActivationStepId>(json!(expected)).unwrap(),
                id
            );
        }
    }

    #[test]
    fn request_step_has_distinct_actors_codex_chat_and_nested_agent_sequence() {
        let step = ActivationStep::try_new(
            ActivationStepId::RequestIntegrationVerification,
            Vec::new(),
            "Request integration verification",
        )
        .expect("request step");
        assert_eq!(step.initiator(), ActivationActor::User);
        assert_eq!(step.executor(), ActivationActor::Agent);
        assert_eq!(
            step.execution_channel(),
            ActivationExecutionChannel::CodexChat
        );
        assert_eq!(
            step.agent_sequence()
                .iter()
                .map(AgentSequenceStep::tool)
                .collect::<Vec<_>>(),
            vec![
                AgentToolId::LIST_PROJECTS,
                AgentToolId::BEGIN_INTEGRATION_VERIFICATION,
                AgentToolId::GUARD_PROBE,
                AgentToolId::GET_INTEGRATION_VERIFICATION,
            ]
        );
        assert!(ActivationStepId::ALL
            .iter()
            .all(|id| !matches!(id.as_str(), "run_guard_probe" | "run_mcp_verification")));
    }

    #[test]
    fn activation_step_instruction_validation_is_strict() {
        for instruction in ["", "invalid\0instruction"] {
            assert!(ActivationStep::try_new(
                ActivationStepId::ReviewProjectHooks,
                Vec::new(),
                instruction,
            )
            .is_err());
        }
        assert!(ActivationStep::try_new(
            ActivationStepId::ReviewProjectHooks,
            Vec::new(),
            "x".repeat(MAX_CONNECTION_TEXT_BYTES + 1),
        )
        .is_err());
    }

    #[test]
    fn activation_step_schema_and_strict_decoding_are_closed() {
        let schema = serde_json::to_value(schemars::schema_for!(ActivationStep)).unwrap();
        assert_eq!(
            schema["properties"]
                .as_object()
                .expect("ActivationStep schema properties")
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "agent_sequence",
                "completes_checks",
                "diagnostic_only",
                "execution_channel",
                "executor",
                "id",
                "initiator",
                "instruction",
                "prerequisites",
                "root_finding_ids",
            ])
        );
        assert_eq!(schema["required"].as_array().map(Vec::len), Some(10));
        assert_eq!(schema["additionalProperties"], json!(false));

        let mut unknown = serde_json::to_value(step(
            ActivationStepId::RepairManagedConfiguration,
            Vec::new(),
        ))
        .unwrap();
        unknown["command"] = json!("volicord connection verify");
        assert!(serde_json::from_value::<ActivationStep>(unknown).is_err());
    }

    #[test]
    fn hook_activation_evidence_keeps_unknown_disabled_and_bypass_distinct() {
        let evidence = |host, observed| HookActivationEvidence {
            setup_changed_definition: false,
            host,
            current_definition_event_observed: observed,
        };
        assert_eq!(
            HookActivationState::from_evidence(evidence(None, false)),
            HookActivationState::Unknown
        );
        assert_eq!(
            HookActivationState::from_evidence(evidence(None, true)),
            HookActivationState::EffectiveByObservation
        );
        assert_eq!(
            HookActivationState::from_evidence(evidence(
                Some(HostHookActivationEvidence::Disabled),
                true,
            )),
            HookActivationState::Disabled
        );
        assert_eq!(
            HookActivationState::from_evidence(evidence(
                Some(HostHookActivationEvidence::BypassedForInvocation),
                true,
            )),
            HookActivationState::BypassedForInvocation
        );
        assert!(
            serde_json::from_value::<HookActivationState>(json!("trusted")).is_err(),
            "the host does not expose authoritative persisted hook trust"
        );
    }

    fn activation_check(
        id: ConnectionCheckKind,
        status: ConnectionCheckStatus,
        hook: Option<HookActivationState>,
    ) -> ConnectionCheck {
        let details = hook.map(|state| {
            ConnectionCheckDetails::try_new(serde_json::Map::from_iter([(
                "activation_state".to_owned(),
                json!(state.as_str()),
            )]))
            .unwrap()
        });
        ConnectionCheck::try_new(
            id,
            status,
            Vec::new(),
            Some(format!("{}_state", id.as_str())),
            format!("{} state", id.as_str()),
            details,
            None,
        )
        .unwrap()
    }

    fn activation_report(
        host_reload: ConnectionCheckStatus,
        hook: HookActivationState,
        session: ConnectionCheckStatus,
        capability: ConnectionCheckStatus,
        guard_execution: ConnectionCheckStatus,
        guard_verification: ConnectionCheckStatus,
    ) -> ConnectionVerificationReport {
        let checks = vec![
            activation_check(
                ConnectionCheckKind::ManagedConfig,
                ConnectionCheckStatus::Passed,
                None,
            ),
            activation_check(ConnectionCheckKind::HostReload, host_reload, None),
            activation_check(
                ConnectionCheckKind::HookSourceActivation,
                match hook {
                    HookActivationState::EffectiveByObservation
                    | HookActivationState::ManagedByPolicy => ConnectionCheckStatus::Passed,
                    HookActivationState::Disabled => ConnectionCheckStatus::Failed,
                    HookActivationState::Unknown
                    | HookActivationState::ReviewRequiredBySetup
                    | HookActivationState::BypassedForInvocation => ConnectionCheckStatus::Pending,
                },
                Some(hook),
            ),
            activation_check(ConnectionCheckKind::ManagedSessionHealth, session, None),
            activation_check(
                ConnectionCheckKind::ManagedCapabilityProof,
                capability,
                None,
            ),
            activation_check(
                ConnectionCheckKind::AmbientHookCoverage,
                guard_execution,
                None,
            ),
            activation_check(
                ConnectionCheckKind::CorrelatedGuardVerification,
                guard_verification,
                None,
            ),
        ];
        let plan = empty_plan(&checks, hook);
        ConnectionVerificationReport::try_new(timestamp(), checks, plan).unwrap()
    }

    #[test]
    fn activation_state_transitions_are_derived_from_typed_checks() {
        let pending = ConnectionCheckStatus::Pending;
        let passed = ConnectionCheckStatus::Passed;
        assert_eq!(
            activation_report(
                pending,
                HookActivationState::Unknown,
                pending,
                pending,
                pending,
                pending,
            )
            .activation_state(),
            IntegrationActivationState::HostReloadRequired
        );
        assert_eq!(
            activation_report(
                passed,
                HookActivationState::Unknown,
                pending,
                pending,
                pending,
                pending,
            )
            .activation_state(),
            IntegrationActivationState::HookReviewRequiredOrUnknown
        );
        assert_eq!(
            activation_report(
                passed,
                HookActivationState::EffectiveByObservation,
                pending,
                pending,
                pending,
                pending,
            )
            .activation_state(),
            IntegrationActivationState::McpObservationRequired
        );
        assert_eq!(
            activation_report(
                passed,
                HookActivationState::EffectiveByObservation,
                passed,
                passed,
                passed,
                pending,
            )
            .activation_state(),
            IntegrationActivationState::GuardVerificationRequired
        );
        assert_eq!(
            activation_report(
                passed,
                HookActivationState::EffectiveByObservation,
                passed,
                passed,
                passed,
                passed,
            )
            .activation_state(),
            IntegrationActivationState::Complete
        );
    }

    #[test]
    fn project_trust_not_applicable_does_not_activate_project_hooks() {
        let mut report = activation_report(
            ConnectionCheckStatus::Passed,
            HookActivationState::Unknown,
            ConnectionCheckStatus::Pending,
            ConnectionCheckStatus::Pending,
            ConnectionCheckStatus::Pending,
            ConnectionCheckStatus::Pending,
        );
        let mut checks = report.checks().to_vec();
        checks.push(activation_check(
            ConnectionCheckKind::ProjectTrust,
            ConnectionCheckStatus::NotApplicable,
            None,
        ));
        let plan = empty_plan(&checks, HookActivationState::Unknown);
        report = ConnectionVerificationReport::try_new(timestamp(), checks, plan).unwrap();
        assert_eq!(
            report.activation_state(),
            IntegrationActivationState::HookReviewRequiredOrUnknown
        );
        assert_eq!(report.hook_activation_state(), HookActivationState::Unknown);
    }

    #[test]
    fn unknown_report_kinds_fail_strict_typed_decoding() {
        assert!(
            serde_json::from_value::<ConnectionCheckKind>(json!("not_a_current_check")).is_err()
        );
        assert!(serde_json::from_value::<ActivationStepId>(json!("not_a_current_step")).is_err());

        let report = ConnectionVerificationReport::verification_not_run(timestamp()).unwrap();
        let mut unknown_check = serde_json::to_value(&report).unwrap();
        unknown_check["checks"][0]["id"] = json!("not_a_current_check");
        assert!(
            serde_json::from_value::<ConnectionVerificationReport>(unknown_check).is_err(),
            "persisted reports must reject unknown check kinds"
        );

        let mut unknown_step = serde_json::to_value(&report).unwrap();
        unknown_step["activation_plan"]["required_steps"][0]["id"] = json!("not_a_current_step");
        assert!(
            serde_json::from_value::<ConnectionVerificationReport>(unknown_step).is_err(),
            "persisted reports must reject unknown activation step kinds"
        );
    }

    #[test]
    fn report_serialization_and_strict_deserialization_are_stable() {
        let checks = vec![check(
            ConnectionCheckKind::McpServer,
            ConnectionCheckStatus::Passed,
        )];
        let plan = IntegrationActivationPlan::try_new(
            IntegrationActivationState::Configured,
            vec![step(ActivationStepId::ReloadCodex, Vec::new())],
            Vec::new(),
        )
        .unwrap();
        let report = ConnectionVerificationReport::try_new(timestamp(), checks, plan).unwrap();
        let expected = serde_json::to_value(&report).unwrap();
        assert_eq!(
            serde_json::from_value::<ConnectionVerificationReport>(expected.clone()).unwrap(),
            report
        );

        let mut command_bearing_report = serde_json::to_value(&report).unwrap();
        command_bearing_report["activation_plan"]["required_steps"][0]["command"] =
            json!("volicord connection verify");
        assert!(
            serde_json::from_value::<ConnectionVerificationReport>(command_bearing_report).is_err(),
            "a complete report must reject an unknown activation-step member"
        );

        for damaged in [
            json!({
                "status": "complete",
                "checked_at": "2026-07-18T00:00:00Z",
                "checks": [],
                "activation_plan": {
                    "state": "configured",
                    "required_steps": [],
                    "optional_diagnostics": []
                },
                "extra": true
            }),
            json!({
                "status": "complete",
                "checked_at": "2026-07-18T00:00:00Z",
                "checks": [{
                    "id": "host",
                    "status": "passed",
                    "code": null,
                    "summary": "host",
                    "details": null,
                    "observed_at": null,
                    "extra": true
                }],
                "activation_plan": {
                    "state": "configured",
                    "required_steps": [],
                    "optional_diagnostics": []
                }
            }),
        ] {
            assert!(serde_json::from_value::<ConnectionVerificationReport>(damaged).is_err());
        }
    }

    #[test]
    fn every_check_status_combination_aggregates_deterministically() {
        let statuses = [
            ConnectionCheckStatus::Passed,
            ConnectionCheckStatus::Pending,
            ConnectionCheckStatus::Failed,
            ConnectionCheckStatus::NotApplicable,
        ];
        for left in statuses {
            for right in statuses {
                for third in statuses {
                    let checks = vec![
                        check(ConnectionCheckKind::ManagedConfig, left),
                        check(ConnectionCheckKind::HostExecutable, right),
                        check(ConnectionCheckKind::ProjectTrust, third),
                    ];
                    let plan = empty_plan(&checks, HookActivationState::Unknown);
                    let report =
                        ConnectionVerificationReport::try_new(timestamp(), checks, plan).unwrap();
                    let expected = if [left, right, third].contains(&ConnectionCheckStatus::Failed)
                    {
                        ConnectionStatus::Failed
                    } else if [left, right, third].contains(&ConnectionCheckStatus::Pending) {
                        ConnectionStatus::ActionRequired
                    } else {
                        ConnectionStatus::Complete
                    };
                    assert_eq!(
                        report.status(),
                        expected,
                        "{left:?} + {right:?} + {third:?}"
                    );
                }
            }
        }
        assert_eq!(
            ConnectionVerificationReport::try_new(
                timestamp(),
                Vec::new(),
                IntegrationActivationPlan::empty(IntegrationActivationState::Configured),
            )
            .unwrap()
            .status(),
            ConnectionStatus::Complete
        );
    }

    #[test]
    fn recoverable_correlated_failure_is_action_required_without_losing_failed_state() {
        let correlated = ConnectionCheck::try_new(
            ConnectionCheckKind::CorrelatedGuardVerification,
            ConnectionCheckStatus::Failed,
            Vec::new(),
            Some("correlated_guard_verification_failed".to_owned()),
            "The latest correlated Guard verification requires repair",
            Some(
                ConnectionCheckDetails::try_new(
                    json!({
                        "recoverability": "recoverable",
                        "latest_attempt": {
                            "attempt_state": "repair_required",
                            "repair_reason": "hook_event_not_observed"
                        }
                    })
                    .as_object()
                    .expect("details object")
                    .clone(),
                )
                .expect("typed details"),
            ),
            None,
        )
        .expect("recoverable correlated failure");
        let checks = vec![
            check(
                ConnectionCheckKind::AmbientHookCoverage,
                ConnectionCheckStatus::Passed,
            ),
            correlated,
        ];
        let plan = empty_plan(&checks, HookActivationState::Unknown);
        let report = ConnectionVerificationReport::try_new(timestamp(), checks, plan)
            .expect("action-required report");

        assert_eq!(report.status(), ConnectionStatus::ActionRequired);
        let correlated = report
            .checks()
            .iter()
            .find(|check| check.id() == ConnectionCheckKind::CorrelatedGuardVerification)
            .expect("correlated check");
        assert_eq!(correlated.status(), ConnectionCheckStatus::Failed);
        assert_eq!(
            correlated.details().expect("typed evidence").as_object()["latest_attempt"]
                ["attempt_state"],
            "repair_required"
        );
    }

    #[test]
    fn duplicate_check_ids_are_rejected() {
        let checks = vec![
            check(
                ConnectionCheckKind::ManagedConfig,
                ConnectionCheckStatus::Passed,
            ),
            check(
                ConnectionCheckKind::ManagedConfig,
                ConnectionCheckStatus::Pending,
            ),
        ];
        let plan = empty_plan(&checks, HookActivationState::Unknown);
        let error = ConnectionVerificationReport::try_new(timestamp(), checks, plan)
            .expect_err("duplicate checks must fail");
        assert!(error.detail().contains("duplicate check"));
    }

    #[test]
    fn duplicate_step_ids_are_rejected() {
        let error = IntegrationActivationPlan::try_new(
            IntegrationActivationState::Configured,
            vec![
                step(ActivationStepId::ReloadCodex, Vec::new()),
                step(ActivationStepId::ReloadCodex, Vec::new()),
            ],
            Vec::new(),
        )
        .expect_err("duplicate steps must fail");
        assert!(error.detail().contains("duplicate step"));
    }

    #[test]
    fn activation_plan_rejects_unknown_cycles_nested_tools_and_required_diagnostics() {
        let unknown = IntegrationActivationPlan::try_new(
            IntegrationActivationState::Configured,
            vec![step(
                ActivationStepId::ReadConnectionStatus,
                vec![ActivationStepId::RequestIntegrationVerification],
            )],
            Vec::new(),
        )
        .expect_err("unknown prerequisite must fail");
        assert!(unknown.detail().contains("unknown prerequisite"));

        let cycle = IntegrationActivationPlan::try_new(
            IntegrationActivationState::Configured,
            vec![
                step(
                    ActivationStepId::RequestIntegrationVerification,
                    vec![ActivationStepId::ReadConnectionStatus],
                ),
                step(
                    ActivationStepId::ReadConnectionStatus,
                    vec![ActivationStepId::RequestIntegrationVerification],
                ),
            ],
            Vec::new(),
        )
        .expect_err("cycle must fail");
        assert!(cycle.detail().contains("cycle"));

        let context =
            canonical_activation_step_context(ActivationStepId::RequestIntegrationVerification);
        let nested = ActivationStep::try_new_with_context(
            ActivationStepId::RequestIntegrationVerification,
            context.initiator,
            context.executor,
            ActivationExecutionChannel::McpTool,
            Vec::new(),
            context.completes_checks.to_vec(),
            Vec::new(),
            "Expose an internal agent tool",
            context.diagnostic_only,
            context.agent_sequence.to_vec(),
        )
        .unwrap();
        let nested = IntegrationActivationPlan::try_new(
            IntegrationActivationState::Configured,
            vec![nested],
            Vec::new(),
        )
        .expect_err("nested tool at top level must fail");
        assert!(nested.detail().contains("nested agent tool"));

        let diagnostic = IntegrationActivationPlan::try_new(
            IntegrationActivationState::Configured,
            vec![step(
                ActivationStepId::RunOptionalActiveDiagnostics,
                Vec::new(),
            )],
            Vec::new(),
        )
        .expect_err("diagnostic-only required step must fail");
        assert!(diagnostic.detail().contains("diagnostic-only"));
    }

    #[test]
    fn report_collection_and_byte_bounds_remain_enforced() {
        let checks = vec![
            check(
                ConnectionCheckKind::ManagedConfig,
                ConnectionCheckStatus::Passed,
            );
            MAX_CONNECTION_CHECKS + 1
        ];
        let plan = empty_plan(&checks, HookActivationState::Unknown);
        let error = ConnectionVerificationReport::try_new(timestamp(), checks, plan)
            .expect_err("the check collection bound must fail before duplicate validation");
        assert!(error.detail().contains("too many checks"));

        let error = IntegrationActivationPlan::try_new(
            IntegrationActivationState::Configured,
            vec![step(ActivationStepId::ReloadCodex, Vec::new()); MAX_ACTIVATION_STEPS + 1],
            Vec::new(),
        )
        .expect_err("the step collection bound must fail before duplicate validation");
        assert!(error.detail().contains("too many steps"));

        let checks: Vec<ConnectionCheck> = ConnectionCheckKind::ALL
            .into_iter()
            .map(|id| {
                ConnectionCheck::try_new(
                    id,
                    ConnectionCheckStatus::Passed,
                    Vec::new(),
                    None,
                    "x".repeat(MAX_CONNECTION_TEXT_BYTES),
                    None,
                    None,
                )
                .expect("individually bounded check")
            })
            .collect();
        let steps = ActivationStepId::ALL
            .into_iter()
            .filter(|id| *id != ActivationStepId::RunOptionalActiveDiagnostics)
            .map(|id| {
                ActivationStep::try_new(id, Vec::new(), "x".repeat(MAX_CONNECTION_TEXT_BYTES))
                    .expect("individually bounded step")
            })
            .collect();
        let optional = vec![ActivationStep::try_new(
            ActivationStepId::RunOptionalActiveDiagnostics,
            Vec::new(),
            "x".repeat(MAX_CONNECTION_TEXT_BYTES),
        )
        .expect("individually bounded diagnostic step")];
        let state = derive_integration_activation_state(&checks, HookActivationState::Unknown);
        let plan = IntegrationActivationPlan::try_new(state, steps, optional)
            .expect("individually bounded plan");
        let error = ConnectionVerificationReport::try_new(timestamp(), checks, plan)
            .expect_err("the complete serialized report bound must still apply");
        assert!(error.detail().contains("serialized size bound"));
    }

    #[test]
    fn checks_and_activation_steps_use_canonical_graph_order() {
        let checks = vec![
            check(
                ConnectionCheckKind::VerificationNotRun,
                ConnectionCheckStatus::Passed,
            ),
            check(
                ConnectionCheckKind::ConnectionRemoval,
                ConnectionCheckStatus::Passed,
            ),
        ];
        let state = derive_integration_activation_state(&checks, HookActivationState::Unknown);
        let plan = IntegrationActivationPlan::try_new(
            state,
            vec![
                step(
                    ActivationStepId::ReadConnectionStatus,
                    vec![ActivationStepId::RequestIntegrationVerification],
                ),
                step(ActivationStepId::RepairManagedConfiguration, Vec::new()),
                step(ActivationStepId::RequestIntegrationVerification, Vec::new()),
            ],
            Vec::new(),
        )
        .unwrap();
        let report = ConnectionVerificationReport::try_new(timestamp(), checks, plan).unwrap();
        assert_eq!(
            report.checks()[0].id(),
            ConnectionCheckKind::ConnectionRemoval
        );
        assert_eq!(
            report
                .activation_plan()
                .required_steps()
                .iter()
                .map(ActivationStep::id)
                .collect::<Vec<_>>(),
            vec![
                ActivationStepId::RepairManagedConfiguration,
                ActivationStepId::RequestIntegrationVerification,
                ActivationStepId::ReadConnectionStatus,
            ]
        );

        let mut value = serde_json::to_value(&report).unwrap();
        value["checks"].as_array_mut().unwrap().swap(0, 1);
        assert!(serde_json::from_value::<ConnectionVerificationReport>(value).is_err());

        let mut value = serde_json::to_value(&report).unwrap();
        value["activation_plan"]["required_steps"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
        assert!(serde_json::from_value::<ConnectionVerificationReport>(value).is_err());
    }

    #[test]
    fn missing_report_projection_is_action_required() {
        let report = ConnectionVerificationReport::verification_not_run(timestamp()).unwrap();
        assert_eq!(report.status(), ConnectionStatus::ActionRequired);
        assert_eq!(
            report.checks()[0].id(),
            ConnectionCheckKind::VerificationNotRun
        );
        assert_eq!(report.checks()[0].status(), ConnectionCheckStatus::Pending);
        assert_eq!(
            report.activation_plan().required_steps()[0].id(),
            ActivationStepId::RequestIntegrationVerification
        );
    }

    #[test]
    fn blocked_checks_inherit_exact_root_findings_from_failed_prerequisites() {
        let root = DiagnosticFindingId::parse("finding.managed_config").unwrap();
        let managed_config = check(
            ConnectionCheckKind::ManagedConfig,
            ConnectionCheckStatus::Failed,
        )
        .with_cause_finding_ids(vec![root.clone()])
        .unwrap();
        let process = check(
            ConnectionCheckKind::ProcessStartup,
            ConnectionCheckStatus::Pending,
        )
        .blocked_by(vec![root.clone()])
        .unwrap();
        let checks = vec![process, managed_config];
        let plan = empty_plan(&checks, HookActivationState::Unknown);
        let report = ConnectionVerificationReport::try_new(timestamp(), checks, plan).unwrap();
        assert_eq!(report.status(), ConnectionStatus::Failed);
        assert_eq!(report.root_cause_ids(), std::slice::from_ref(&root));

        let unrelated = DiagnosticFindingId::parse("finding.unrelated").unwrap();
        let mismatched = check(
            ConnectionCheckKind::ProcessStartup,
            ConnectionCheckStatus::Pending,
        )
        .blocked_by(vec![unrelated])
        .unwrap();
        let checks = vec![
            check(
                ConnectionCheckKind::ManagedConfig,
                ConnectionCheckStatus::Failed,
            )
            .with_cause_finding_ids(vec![root])
            .unwrap(),
            mismatched,
        ];
        let plan = empty_plan(&checks, HookActivationState::Unknown);
        assert!(
            ConnectionVerificationReport::try_new(timestamp(), checks, plan)
                .unwrap_err()
                .detail()
                .contains("root causes do not match")
        );
    }

    #[test]
    fn not_applicable_checks_do_not_require_steps_or_degrade_the_report() {
        let checks = vec![check(
            ConnectionCheckKind::ProjectTrust,
            ConnectionCheckStatus::NotApplicable,
        )];
        let plan = empty_plan(&checks, HookActivationState::Unknown);
        let report = ConnectionVerificationReport::try_new(timestamp(), checks, plan).unwrap();
        assert_eq!(report.status(), ConnectionStatus::Complete);
        assert!(report.root_cause_ids().is_empty());
        assert!(report.activation_plan().required_steps().is_empty());
    }

    #[test]
    fn noncanonical_status_values_are_rejected() {
        for noncanonical in [
            "not_verified",
            "missing",
            "changed",
            "rejected",
            "unavailable",
            "unknown",
            "skipped",
            "stale",
            "broken",
            "degraded",
            "dry_run",
        ] {
            assert!(serde_json::from_value::<ConnectionStatus>(json!(noncanonical)).is_err());
            assert!(serde_json::from_value::<ConnectionCheckStatus>(json!(noncanonical)).is_err());
        }
    }

    #[test]
    fn details_reject_duplicate_keys_at_any_depth() {
        let json = r#"{
            "status":"complete",
            "activation_state":"configured",
            "hook_activation_state":"unknown",
            "checked_at":"2026-07-18T00:00:00Z",
            "checks":[{
                "id":"host_session",
                "status":"passed",
                "depends_on":[],
                "cause_finding_ids":[],
                "summary":"host",
                "details":{"nested":{"same":1,"same":2}}
            }],
            "root_cause_ids":[],
            "activation_plan":{
                "state":"configured",
                "required_steps":[],
                "optional_diagnostics":[]
            }
        }"#;
        assert!(serde_json::from_str::<ConnectionVerificationReport>(json).is_err());
    }
}
