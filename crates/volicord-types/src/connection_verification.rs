//! Canonical serialized Agent Connection verification report.

use std::{cmp::Ordering, collections::BTreeSet, error::Error, fmt};

use schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::{Map, Number, Value};

use crate::{JsonObject, UtcTimestamp};

/// Maximum number of required checks in one connection report.
pub const MAX_CONNECTION_CHECKS: usize = 64;
/// Maximum number of user actions in one connection report.
pub const MAX_CONNECTION_ACTIONS: usize = 32;
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
    /// Every required check passed.
    Complete,
    /// No required check failed and at least one remains pending.
    ActionRequired,
    /// At least one required check failed.
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
    /// The required check has not yet succeeded or failed.
    Pending,
    /// The required check failed.
    Failed,
}

impl ConnectionCheckStatus {
    /// Returns the stable serialized spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Pending => "pending",
            Self::Failed => "failed",
        }
    }
}

/// Closed current-product vocabulary for one connection check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionCheckKind {
    /// No completed verification report exists yet.
    VerificationNotRun,
    /// Managed host configuration matches its canonical plan.
    ManagedConfig,
    /// The host executable can be discovered and probed.
    HostExecutable,
    /// The Volicord MCP server passes the CLI-owned self-test.
    McpServer,
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
    /// Current required Guard phases were observed.
    GuardObservation,
    /// A setup plan is ready to apply or already matches.
    SetupPlan,
    /// A connection-mode transition was planned or applied.
    ModeTransition,
    /// Connection membership or removal was planned or applied.
    ConnectionRemoval,
}

impl ConnectionCheckKind {
    /// Every current check kind in canonical serialized-spelling order.
    pub const ALL: [Self; 13] = [
        Self::ConnectionRemoval,
        Self::GuardFiles,
        Self::GuardObservation,
        Self::HostExecutable,
        Self::HostSession,
        Self::ManagedConfig,
        Self::McpServer,
        Self::ModeTransition,
        Self::ProjectTrust,
        Self::RequiredTools,
        Self::SetupPlan,
        Self::ToolRoundTrip,
        Self::VerificationNotRun,
    ];

    /// Returns the stable serialized spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VerificationNotRun => "verification_not_run",
            Self::ManagedConfig => "managed_config",
            Self::HostExecutable => "host_executable",
            Self::McpServer => "mcp_server",
            Self::HostSession => "host_session",
            Self::RequiredTools => "required_tools",
            Self::ToolRoundTrip => "tool_round_trip",
            Self::ProjectTrust => "project_trust",
            Self::GuardFiles => "guard_files",
            Self::GuardObservation => "guard_observation",
            Self::SetupPlan => "setup_plan",
            Self::ModeTransition => "mode_transition",
            Self::ConnectionRemoval => "connection_removal",
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
        Self::try_new(
            wire.id,
            wire.status,
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
        code: Option<String>,
        summary: impl Into<String>,
        details: Option<ConnectionCheckDetails>,
        observed_at: Option<UtcTimestamp>,
    ) -> Result<Self, ConnectionVerificationError> {
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
    /// Use a Volicord tool through the managed host.
    UseVolicordTool,
    /// Produce current managed-host and Guard observations.
    ObserveCodex,
    /// Inspect and repair an observed Codex protocol failure.
    InspectCodexProtocol,
    /// Reload Guard after current managed-file setup.
    ReloadGuard,
    /// Repair the Volicord Guard integration.
    RepairGuard,
    /// Apply a planned connection membership or removal change.
    ApplyRemoval,
}

impl ConnectionActionKind {
    /// Every current action kind in canonical serialized-spelling order.
    pub const ALL: [Self; 13] = [
        Self::ApplyRemoval,
        Self::ApplySetup,
        Self::HostTrustRequired,
        Self::InspectCodexProtocol,
        Self::InstallOrRepairCodex,
        Self::ObserveCodex,
        Self::ReloadGuard,
        Self::ReloadHost,
        Self::RepairGuard,
        Self::RepairManagedConfig,
        Self::RepairMcpServer,
        Self::RunVerification,
        Self::UseVolicordTool,
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
            Self::UseVolicordTool => "use_volicord_tool",
            Self::ObserveCodex => "observe_codex",
            Self::InspectCodexProtocol => "inspect_codex_protocol",
            Self::ReloadGuard => "reload_guard",
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
pub struct ConnectionAction {
    id: ConnectionActionKind,
    instruction: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionActionWire {
    id: ConnectionActionKind,
    instruction: String,
    command: Option<String>,
}

impl<'de> Deserialize<'de> for ConnectionAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ConnectionActionWire::deserialize(deserializer)?;
        Self::try_new(wire.id, wire.instruction, wire.command).map_err(de::Error::custom)
    }
}

impl ConnectionAction {
    /// Validates and constructs one connection action.
    pub fn try_new(
        id: ConnectionActionKind,
        instruction: impl Into<String>,
        command: Option<String>,
    ) -> Result<Self, ConnectionVerificationError> {
        let instruction = instruction.into();
        validate_text("action instruction", &instruction)?;
        if let Some(command) = command.as_deref() {
            validate_text("action command", command)?;
        }
        Ok(Self {
            id,
            instruction,
            command,
        })
    }

    /// Returns the stable action ID.
    pub const fn id(&self) -> ConnectionActionKind {
        self.id
    }

    /// Returns the user-visible instruction.
    pub fn instruction(&self) -> &str {
        &self.instruction
    }

    /// Returns the optional executable command text.
    pub fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }
}

/// Canonical serialized result of connection verification.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct ConnectionVerificationReport {
    status: ConnectionStatus,
    checked_at: UtcTimestamp,
    checks: Vec<ConnectionCheck>,
    actions: Vec<ConnectionAction>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionVerificationReportWire {
    status: ConnectionStatus,
    checked_at: UtcTimestamp,
    checks: Vec<ConnectionCheck>,
    actions: Vec<ConnectionAction>,
}

impl<'de> Deserialize<'de> for ConnectionVerificationReport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ConnectionVerificationReportWire::deserialize(deserializer)?;
        Self::from_canonical_parts(wire.status, wire.checked_at, wire.checks, wire.actions)
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
        Self::from_canonical_parts(status, checked_at, checks, actions)
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
                Some("verification_not_run".to_owned()),
                "Connection verification has not been run",
                None,
                None,
            )?],
            vec![ConnectionAction::try_new(
                ConnectionActionKind::RunVerification,
                "Run connection verification to observe current host behavior",
                Some("volicord connection verify".to_owned()),
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

    /// Returns user actions in canonical ID order.
    pub fn actions(&self) -> &[ConnectionAction] {
        &self.actions
    }

    fn from_canonical_parts(
        status: ConnectionStatus,
        checked_at: UtcTimestamp,
        checks: Vec<ConnectionCheck>,
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
        require_canonical_action_order(&actions)?;
        let derived = aggregate_status(&checks);
        if status != derived {
            return Err(invalid(
                "connection report status does not match its checks",
            ));
        }
        let report = Self {
            status,
            checked_at,
            checks,
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
    if checks
        .iter()
        .any(|check| check.status == ConnectionCheckStatus::Failed)
    {
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
            Some(format!("{}_result", id.as_str())),
            format!("{} summary", id.as_str()),
            None,
            None,
        )
        .expect("test check")
    }

    fn action(id: ConnectionActionKind) -> ConnectionAction {
        ConnectionAction::try_new(id, format!("{} instruction", id.as_str()), None)
            .expect("test action")
    }

    #[test]
    fn every_current_check_kind_round_trips_exact_json() {
        let expected = [
            "connection_removal",
            "guard_files",
            "guard_observation",
            "host_executable",
            "host_session",
            "managed_config",
            "mcp_server",
            "mode_transition",
            "project_trust",
            "required_tools",
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
            "reload_guard",
            "reload_host",
            "repair_guard",
            "repair_managed_config",
            "repair_mcp_server",
            "run_verification",
            "use_volicord_tool",
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
    fn unknown_and_removed_report_kinds_fail() {
        for value in ["unknown_check", "mcp_handshake", "host"] {
            assert!(serde_json::from_value::<ConnectionCheckKind>(json!(value)).is_err());
        }
        for value in ["unknown_action", "reload_required", "legacy_reload"] {
            assert!(serde_json::from_value::<ConnectionActionKind>(json!(value)).is_err());
        }

        let report = ConnectionVerificationReport::verification_not_run(timestamp()).unwrap();
        let mut unknown_check = serde_json::to_value(&report).unwrap();
        unknown_check["checks"][0]["id"] = json!("unknown_check");
        assert!(
            serde_json::from_value::<ConnectionVerificationReport>(unknown_check).is_err(),
            "persisted reports must reject unknown check kinds"
        );

        let mut removed_action = serde_json::to_value(&report).unwrap();
        removed_action["actions"][0]["id"] = json!("reload_required");
        assert!(
            serde_json::from_value::<ConnectionVerificationReport>(removed_action).is_err(),
            "persisted reports must reject removed action kinds"
        );
    }

    #[test]
    fn report_serialization_and_strict_deserialization_are_stable() {
        let report = ConnectionVerificationReport::try_new(
            timestamp(),
            vec![check(
                ConnectionCheckKind::McpServer,
                ConnectionCheckStatus::Passed,
            )],
            vec![ConnectionAction::try_new(
                ConnectionActionKind::ReloadHost,
                "Reload the host",
                None,
            )
            .unwrap()],
        )
        .unwrap();
        let expected = json!({
            "status": "complete",
            "checked_at": "2026-07-18T00:00:00Z",
            "checks": [{
                "id": "mcp_server",
                "status": "passed",
                "code": "mcp_server_result",
                "summary": "mcp_server summary",
            }],
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
        ];
        for left in statuses {
            for right in statuses {
                for third in statuses {
                    let report = ConnectionVerificationReport::try_new(
                        timestamp(),
                        vec![
                            check(ConnectionCheckKind::ManagedConfig, left),
                            check(ConnectionCheckKind::McpServer, right),
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
                action(ConnectionActionKind::UseVolicordTool),
                action(ConnectionActionKind::ApplyRemoval),
            ],
        )
        .unwrap();
        assert_eq!(
            report.checks()[0].id(),
            ConnectionCheckKind::ConnectionRemoval
        );
        assert_eq!(report.actions()[0].id(), ConnectionActionKind::ApplyRemoval);

        let mut value = serde_json::to_value(&report).unwrap();
        value["checks"].as_array_mut().unwrap().swap(0, 1);
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
    fn obsolete_status_values_are_rejected() {
        for obsolete in [
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
            assert!(serde_json::from_value::<ConnectionStatus>(json!(obsolete)).is_err());
            assert!(serde_json::from_value::<ConnectionCheckStatus>(json!(obsolete)).is_err());
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
