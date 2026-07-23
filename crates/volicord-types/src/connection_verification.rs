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

use crate::{DiagnosticFindingId, JsonObject, UtcTimestamp};

/// Maximum number of required checks in one connection report.
pub const MAX_CONNECTION_CHECKS: usize = 64;
/// Maximum number of user actions in one connection report.
pub const MAX_CONNECTION_ACTIONS: usize = 32;
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
    /// A current managed-host session completed initialization.
    HostSession,
    /// A current managed host exposes every required tool.
    RequiredTools,
    /// A current managed host completed the designated safe tool call.
    ToolRoundTrip,
    /// Project trust is satisfied or not separately applicable.
    ProjectTrust,
    /// Current Guard managed-file expectations match.
    GuardFiles,
    /// A current Guard hook executed under the managed installation.
    GuardHookExecution,
    /// Current required Guard phases were observed.
    GuardObservation,
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
    pub const ALL: [Self; 17] = [
        Self::ConnectionRemoval,
        Self::DiagnosticLookup,
        Self::GuardFiles,
        Self::GuardHookExecution,
        Self::GuardObservation,
        Self::HostExecutable,
        Self::HostSession,
        Self::ManagedConfig,
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
            Self::HostSession => "host_session",
            Self::RequiredTools => "required_tools",
            Self::ToolRoundTrip => "tool_round_trip",
            Self::ProjectTrust => "project_trust",
            Self::GuardFiles => "guard_files",
            Self::GuardHookExecution => "guard_hook_execution",
            Self::GuardObservation => "guard_observation",
            Self::SetupPlan => "setup_plan",
            Self::ModeTransition => "mode_transition",
            Self::ConnectionRemoval => "connection_removal",
            Self::RuntimeSessionLookup => "runtime_session_lookup",
        }
    }

    /// Returns the canonical prerequisite edges for this check kind.
    pub const fn dependencies(self) -> &'static [Self] {
        match self {
            Self::McpServer | Self::ProcessStartup => &[Self::ManagedConfig],
            Self::HostSession => &[Self::ProcessStartup],
            Self::ToolRoundTrip => &[Self::RequiredTools],
            Self::GuardHookExecution => &[Self::GuardFiles],
            Self::GuardObservation => &[Self::GuardHookExecution],
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

/// Closed current-product vocabulary for one connection action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionActionKind {
    /// Run active connection verification.
    RunVerification,
    /// Apply planned setup changes.
    ApplySetup,
    /// Satisfy the host's project-trust requirement.
    HostTrustRequired,
    /// Repair the Volicord-managed host configuration.
    RepairManagedConfig,
    /// Install or repair the Codex executable.
    InstallOrRepairCodex,
    /// Repair the Volicord MCP server or its storage preflight.
    RepairMcpServer,
    /// Reload the managed host against current configuration.
    ReloadHost,
    /// Produce current managed-host and Guard observations.
    ObserveCodex,
    /// Inspect and repair an observed Codex protocol failure.
    InspectCodexProtocol,
    /// Repair the Volicord Guard integration.
    RepairGuard,
    /// Apply a planned connection membership or removal change.
    ApplyRemoval,
}

impl ConnectionActionKind {
    /// Every current action kind in canonical serialized-spelling order.
    pub const ALL: [Self; 11] = [
        Self::ApplyRemoval,
        Self::ApplySetup,
        Self::HostTrustRequired,
        Self::InspectCodexProtocol,
        Self::InstallOrRepairCodex,
        Self::ObserveCodex,
        Self::ReloadHost,
        Self::RepairGuard,
        Self::RepairManagedConfig,
        Self::RepairMcpServer,
        Self::RunVerification,
    ];

    /// Returns the stable serialized spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RunVerification => "run_verification",
            Self::ApplySetup => "apply_setup",
            Self::HostTrustRequired => "host_trust_required",
            Self::RepairManagedConfig => "repair_managed_config",
            Self::InstallOrRepairCodex => "install_or_repair_codex",
            Self::RepairMcpServer => "repair_mcp_server",
            Self::ReloadHost => "reload_host",
            Self::ObserveCodex => "observe_codex",
            Self::InspectCodexProtocol => "inspect_codex_protocol",
            Self::RepairGuard => "repair_guard",
            Self::ApplyRemoval => "apply_removal",
        }
    }
}

impl PartialOrd for ConnectionActionKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ConnectionActionKind {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

/// One bounded user instruction produced by connection verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ConnectionAction {
    id: ConnectionActionKind,
    instruction: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionActionWire {
    id: ConnectionActionKind,
    instruction: String,
}

impl<'de> Deserialize<'de> for ConnectionAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ConnectionActionWire::deserialize(deserializer)?;
        Self::try_new(wire.id, wire.instruction).map_err(de::Error::custom)
    }
}

impl ConnectionAction {
    /// Validates and constructs one connection action.
    pub fn try_new(
        id: ConnectionActionKind,
        instruction: impl Into<String>,
    ) -> Result<Self, ConnectionVerificationError> {
        let instruction = instruction.into();
        validate_text("action instruction", &instruction)?;
        Ok(Self { id, instruction })
    }

    /// Returns the stable action ID.
    pub const fn id(&self) -> ConnectionActionKind {
        self.id
    }

    /// Returns the user-visible instruction.
    pub fn instruction(&self) -> &str {
        &self.instruction
    }
}

/// Canonical serialized result of connection verification.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct ConnectionVerificationReport {
    status: ConnectionStatus,
    checked_at: UtcTimestamp,
    checks: Vec<ConnectionCheck>,
    root_cause_ids: Vec<DiagnosticFindingId>,
    actions: Vec<ConnectionAction>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionVerificationReportWire {
    status: ConnectionStatus,
    checked_at: UtcTimestamp,
    checks: Vec<ConnectionCheck>,
    root_cause_ids: Vec<DiagnosticFindingId>,
    actions: Vec<ConnectionAction>,
}

impl<'de> Deserialize<'de> for ConnectionVerificationReport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ConnectionVerificationReportWire::deserialize(deserializer)?;
        Self::from_canonical_parts(
            wire.status,
            wire.checked_at,
            wire.checks,
            wire.root_cause_ids,
            wire.actions,
        )
        .map_err(de::Error::custom)
    }
}

impl ConnectionVerificationReport {
    /// Constructs a report, sorting checks and actions by their stable IDs.
    pub fn try_new(
        checked_at: UtcTimestamp,
        mut checks: Vec<ConnectionCheck>,
        mut actions: Vec<ConnectionAction>,
    ) -> Result<Self, ConnectionVerificationError> {
        checks.sort_by_key(|check| check.id);
        actions.sort_by_key(|action| action.id);
        let status = aggregate_status(&checks);
        let root_cause_ids = aggregate_root_causes(&checks);
        Self::from_canonical_parts(status, checked_at, checks, root_cause_ids, actions)
    }

    /// Synthesizes the canonical projection for a connection not yet verified.
    pub fn verification_not_run(
        checked_at: UtcTimestamp,
    ) -> Result<Self, ConnectionVerificationError> {
        Self::try_new(
            checked_at,
            vec![ConnectionCheck::try_new(
                ConnectionCheckKind::VerificationNotRun,
                ConnectionCheckStatus::Pending,
                Vec::new(),
                Some("verification_not_run".to_owned()),
                "Connection verification has not been run",
                None,
                None,
            )?],
            vec![ConnectionAction::try_new(
                ConnectionActionKind::RunVerification,
                "Run connection verification to observe current host behavior",
            )?],
        )
    }

    /// Returns the derived aggregate status.
    pub const fn status(&self) -> ConnectionStatus {
        self.status
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

    /// Returns user actions in canonical ID order.
    pub fn actions(&self) -> &[ConnectionAction] {
        &self.actions
    }

    fn from_canonical_parts(
        status: ConnectionStatus,
        checked_at: UtcTimestamp,
        checks: Vec<ConnectionCheck>,
        root_cause_ids: Vec<DiagnosticFindingId>,
        actions: Vec<ConnectionAction>,
    ) -> Result<Self, ConnectionVerificationError> {
        checked_at
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| invalid("checked_at is outside the canonical timestamp range"))?;
        if checks.len() > MAX_CONNECTION_CHECKS {
            return Err(invalid("connection report has too many checks"));
        }
        if actions.len() > MAX_CONNECTION_ACTIONS {
            return Err(invalid("connection report has too many actions"));
        }
        require_canonical_check_order(&checks)?;
        validate_check_dependency_graph(&checks)?;
        require_canonical_action_order(&actions)?;
        let derived = aggregate_status(&checks);
        if status != derived {
            return Err(invalid(
                "connection report status does not match its checks",
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
            checked_at,
            checks,
            root_cause_ids,
            actions,
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

fn aggregate_status(checks: &[ConnectionCheck]) -> ConnectionStatus {
    if checks.iter().any(|check| {
        matches!(
            check.status,
            ConnectionCheckStatus::Failed | ConnectionCheckStatus::Blocked
        )
    }) {
        ConnectionStatus::Failed
    } else if checks
        .iter()
        .any(|check| check.status == ConnectionCheckStatus::Pending)
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

fn require_canonical_action_order(
    actions: &[ConnectionAction],
) -> Result<(), ConnectionVerificationError> {
    let mut seen = BTreeSet::new();
    let mut previous: Option<ConnectionActionKind> = None;
    for action in actions {
        if !seen.insert(action.id) {
            return Err(invalid("connection report contains a duplicate action id"));
        }
        if previous.is_some_and(|previous| previous >= action.id) {
            return Err(invalid("connection actions are not in canonical id order"));
        }
        previous = Some(action.id);
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

    fn action(id: ConnectionActionKind) -> ConnectionAction {
        ConnectionAction::try_new(id, format!("{} instruction", id.as_str())).expect("test action")
    }

    #[test]
    fn every_current_check_kind_round_trips_exact_json() {
        let expected = [
            "connection_removal",
            "diagnostic_lookup",
            "guard_files",
            "guard_hook_execution",
            "guard_observation",
            "host_executable",
            "host_session",
            "managed_config",
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
    fn every_current_action_kind_round_trips_exact_json() {
        let expected = [
            "apply_removal",
            "apply_setup",
            "host_trust_required",
            "inspect_codex_protocol",
            "install_or_repair_codex",
            "observe_codex",
            "reload_host",
            "repair_guard",
            "repair_managed_config",
            "repair_mcp_server",
            "run_verification",
        ];
        for (kind, expected) in ConnectionActionKind::ALL.into_iter().zip(expected) {
            assert_eq!(serde_json::to_value(kind).unwrap(), json!(expected));
            assert_eq!(
                serde_json::from_value::<ConnectionActionKind>(json!(expected)).unwrap(),
                kind
            );
        }
    }

    #[test]
    fn action_instruction_validation_is_strict() {
        let action = ConnectionAction::try_new(
            ConnectionActionKind::InspectCodexProtocol,
            "Inspect the Codex protocol failure",
        )
        .expect("bounded action");
        assert_eq!(
            serde_json::to_value(&action).unwrap(),
            json!({
                "id": "inspect_codex_protocol",
                "instruction": "Inspect the Codex protocol failure",
            })
        );

        for instruction in ["", "invalid\0instruction"] {
            assert!(
                ConnectionAction::try_new(ConnectionActionKind::ObserveCodex, instruction,)
                    .is_err()
            );
        }
        assert!(ConnectionAction::try_new(
            ConnectionActionKind::ObserveCodex,
            "x".repeat(MAX_CONNECTION_TEXT_BYTES + 1),
        )
        .is_err());
    }

    #[test]
    fn action_schema_and_strict_decoding_use_exactly_id_and_instruction() {
        let schema = serde_json::to_value(schemars::schema_for!(ConnectionAction)).unwrap();
        assert_eq!(
            schema["properties"]
                .as_object()
                .expect("ConnectionAction schema properties")
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["id", "instruction"])
        );
        assert_eq!(schema["required"], json!(["id", "instruction"]));
        assert_eq!(schema["additionalProperties"], json!(false));

        for rejected in [
            json!({
                "id": "repair_mcp_server",
                "instruction": "Repair the MCP server",
                "command": "volicord connection verify",
            }),
            json!({
                "id": "repair_mcp_server",
                "instruction": "Repair the MCP server",
                "command": null,
            }),
            json!({
                "id": "repair_mcp_server",
                "instruction": "Repair the MCP server",
                "adapter_action": "verify",
            }),
        ] {
            assert!(serde_json::from_value::<ConnectionAction>(rejected).is_err());
        }
    }

    #[test]
    fn unknown_and_noncanonical_report_kinds_fail() {
        for value in ["unknown_check", "mcp_handshake", "host"] {
            assert!(serde_json::from_value::<ConnectionCheckKind>(json!(value)).is_err());
        }
        for value in [
            "unknown_action",
            "reload_required",
            "unsupported_reload_action",
            "reload_guard",
            "use_volicord_tool",
        ] {
            assert!(serde_json::from_value::<ConnectionActionKind>(json!(value)).is_err());
        }

        let report = ConnectionVerificationReport::verification_not_run(timestamp()).unwrap();
        let mut unknown_check = serde_json::to_value(&report).unwrap();
        unknown_check["checks"][0]["id"] = json!("unknown_check");
        assert!(
            serde_json::from_value::<ConnectionVerificationReport>(unknown_check).is_err(),
            "persisted reports must reject unknown check kinds"
        );

        for value in ["reload_guard", "use_volicord_tool"] {
            let mut removed_action = serde_json::to_value(&report).unwrap();
            removed_action["actions"][0]["id"] = json!(value);
            assert!(
                serde_json::from_value::<ConnectionVerificationReport>(removed_action).is_err(),
                "persisted reports must reject removed action kind {value}"
            );
        }
    }

    #[test]
    fn report_serialization_and_strict_deserialization_are_stable() {
        let report = ConnectionVerificationReport::try_new(
            timestamp(),
            vec![check(
                ConnectionCheckKind::McpServer,
                ConnectionCheckStatus::Passed,
            )],
            vec![
                ConnectionAction::try_new(ConnectionActionKind::ReloadHost, "Reload the host")
                    .unwrap(),
            ],
        )
        .unwrap();
        let expected = json!({
            "status": "complete",
            "checked_at": "2026-07-18T00:00:00Z",
            "checks": [{
                "id": "mcp_server",
                "status": "passed",
                "depends_on": ["managed_config"],
                "cause_finding_ids": [],
                "code": "mcp_server_result",
                "summary": "mcp_server summary",
            }],
            "root_cause_ids": [],
            "actions": [{
                "id": "reload_host",
                "instruction": "Reload the host",
            }]
        });
        assert_eq!(serde_json::to_value(&report).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<ConnectionVerificationReport>(expected).unwrap(),
            report
        );

        let mut command_bearing_report = serde_json::to_value(&report).unwrap();
        command_bearing_report["actions"][0]["command"] = json!("volicord connection verify");
        assert!(
            serde_json::from_value::<ConnectionVerificationReport>(command_bearing_report).is_err(),
            "a complete report must reject an unknown action member"
        );

        for damaged in [
            json!({
                "status": "complete",
                "checked_at": "2026-07-18T00:00:00Z",
                "checks": [],
                "actions": [],
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
                "actions": []
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
                    let report = ConnectionVerificationReport::try_new(
                        timestamp(),
                        vec![
                            check(ConnectionCheckKind::ManagedConfig, left),
                            check(ConnectionCheckKind::HostExecutable, right),
                            check(ConnectionCheckKind::ProjectTrust, third),
                        ],
                        Vec::new(),
                    )
                    .unwrap();
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
            ConnectionVerificationReport::try_new(timestamp(), Vec::new(), Vec::new())
                .unwrap()
                .status(),
            ConnectionStatus::Complete
        );
    }

    #[test]
    fn duplicate_check_ids_are_rejected() {
        let error = ConnectionVerificationReport::try_new(
            timestamp(),
            vec![
                check(
                    ConnectionCheckKind::ManagedConfig,
                    ConnectionCheckStatus::Passed,
                ),
                check(
                    ConnectionCheckKind::ManagedConfig,
                    ConnectionCheckStatus::Pending,
                ),
            ],
            Vec::new(),
        )
        .expect_err("duplicate checks must fail");
        assert!(error.detail().contains("duplicate check"));
    }

    #[test]
    fn duplicate_action_ids_are_rejected() {
        let error = ConnectionVerificationReport::try_new(
            timestamp(),
            Vec::new(),
            vec![
                action(ConnectionActionKind::ReloadHost),
                action(ConnectionActionKind::ReloadHost),
            ],
        )
        .expect_err("duplicate actions must fail");
        assert!(error.detail().contains("duplicate action"));
    }

    #[test]
    fn report_collection_and_byte_bounds_remain_enforced() {
        let error = ConnectionVerificationReport::try_new(
            timestamp(),
            vec![
                check(
                    ConnectionCheckKind::ManagedConfig,
                    ConnectionCheckStatus::Passed,
                );
                MAX_CONNECTION_CHECKS + 1
            ],
            Vec::new(),
        )
        .expect_err("the check collection bound must fail before duplicate validation");
        assert!(error.detail().contains("too many checks"));

        let error = ConnectionVerificationReport::try_new(
            timestamp(),
            Vec::new(),
            vec![action(ConnectionActionKind::ReloadHost); MAX_CONNECTION_ACTIONS + 1],
        )
        .expect_err("the action collection bound must fail before duplicate validation");
        assert!(error.detail().contains("too many actions"));

        let checks = ConnectionCheckKind::ALL
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
        let actions = ConnectionActionKind::ALL
            .into_iter()
            .map(|id| {
                ConnectionAction::try_new(id, "x".repeat(MAX_CONNECTION_TEXT_BYTES))
                    .expect("individually bounded action")
            })
            .collect();
        let error = ConnectionVerificationReport::try_new(timestamp(), checks, actions)
            .expect_err("the complete serialized report bound must still apply");
        assert!(error.detail().contains("serialized size bound"));
    }

    #[test]
    fn ordering_is_canonical_and_noncanonical_wire_order_is_rejected() {
        let report = ConnectionVerificationReport::try_new(
            timestamp(),
            vec![
                check(
                    ConnectionCheckKind::VerificationNotRun,
                    ConnectionCheckStatus::Passed,
                ),
                check(
                    ConnectionCheckKind::ConnectionRemoval,
                    ConnectionCheckStatus::Passed,
                ),
            ],
            vec![
                action(ConnectionActionKind::RunVerification),
                action(ConnectionActionKind::ApplyRemoval),
            ],
        )
        .unwrap();
        assert_eq!(
            report.checks()[0].id(),
            ConnectionCheckKind::ConnectionRemoval
        );
        assert_eq!(
            report
                .actions()
                .iter()
                .map(ConnectionAction::id)
                .collect::<Vec<_>>(),
            vec![
                ConnectionActionKind::ApplyRemoval,
                ConnectionActionKind::RunVerification,
            ]
        );

        let mut value = serde_json::to_value(&report).unwrap();
        value["checks"].as_array_mut().unwrap().swap(0, 1);
        assert!(serde_json::from_value::<ConnectionVerificationReport>(value).is_err());

        let mut value = serde_json::to_value(&report).unwrap();
        value["actions"].as_array_mut().unwrap().swap(0, 1);
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
            report.actions()[0].id(),
            ConnectionActionKind::RunVerification
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
        let report = ConnectionVerificationReport::try_new(
            timestamp(),
            vec![process, managed_config],
            Vec::new(),
        )
        .unwrap();
        assert_eq!(report.status(), ConnectionStatus::Failed);
        assert_eq!(report.root_cause_ids(), std::slice::from_ref(&root));

        let unrelated = DiagnosticFindingId::parse("finding.unrelated").unwrap();
        let mismatched = check(
            ConnectionCheckKind::ProcessStartup,
            ConnectionCheckStatus::Pending,
        )
        .blocked_by(vec![unrelated])
        .unwrap();
        assert!(ConnectionVerificationReport::try_new(
            timestamp(),
            vec![
                check(
                    ConnectionCheckKind::ManagedConfig,
                    ConnectionCheckStatus::Failed,
                )
                .with_cause_finding_ids(vec![root])
                .unwrap(),
                mismatched,
            ],
            Vec::new(),
        )
        .unwrap_err()
        .detail()
        .contains("root causes do not match"));
    }

    #[test]
    fn not_applicable_checks_do_not_require_actions_or_degrade_the_report() {
        let report = ConnectionVerificationReport::try_new(
            timestamp(),
            vec![check(
                ConnectionCheckKind::ProjectTrust,
                ConnectionCheckStatus::NotApplicable,
            )],
            Vec::new(),
        )
        .unwrap();
        assert_eq!(report.status(), ConnectionStatus::Complete);
        assert!(report.root_cause_ids().is_empty());
        assert!(report.actions().is_empty());
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
            "checked_at":"2026-07-18T00:00:00Z",
                "checks":[{
                "id":"host_session",
                "status":"passed",
                "code":null,
                "summary":"host",
                "details":{"nested":{"same":1,"same":2}},
                "observed_at":null
            }],
            "actions":[]
        }"#;
        assert!(serde_json::from_str::<ConnectionVerificationReport>(json).is_err());
    }
}
