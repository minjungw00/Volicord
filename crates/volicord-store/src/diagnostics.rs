//! Bounded local operability diagnostics stored outside authority databases.
//!
//! This store intentionally has no references to `registry.sqlite` or project
//! `state.sqlite`. Callers treat writes as best-effort observations; failure of
//! this store must never change a Core or User Channel result.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use volicord_types::{
    validate_managed_host_session_id, IntegrationProfile, MethodName, ObservationConfidence,
    MANAGED_HOST_SESSION_ID_PREFIX,
};

use crate::{sqlite::enable_foreign_keys, StoreError, StoreResult};

/// Runtime Home filename for the non-authoritative diagnostics store.
pub const DIAGNOSTICS_DB_FILE: &str = "diagnostics.sqlite";
/// Current local diagnostics schema version.
pub const DIAGNOSTICS_SCHEMA_VERSION: u32 = 2;
/// Maximum age retained for diagnostic sessions.
pub const DIAGNOSTICS_RETENTION_DAYS: u32 = 7;
/// Maximum diagnostic sessions retained in one Runtime Home.
pub const DIAGNOSTICS_MAX_SESSIONS: u32 = 64;
/// Maximum diagnostic events retained for one session.
pub const DIAGNOSTICS_MAX_EVENTS_PER_SESSION: u32 = 1_024;

const DATABASE_KIND: &str = "local_diagnostics";
const BUSY_TIMEOUT_MILLIS: u64 = 250;

pub(crate) const DIAGNOSTICS_SCHEMA_V1_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS diagnostic_sessions (
    session_id TEXT PRIMARY KEY NOT NULL,
    connection_id TEXT,
    project_id TEXT,
    transport TEXT NOT NULL CHECK (transport IN ('mcp_stdio', 'guard_hook', 'local_http', 'unknown')),
    host_kind TEXT CHECK (host_kind IS NULL OR host_kind IN ('codex', 'claude_code', 'generic', 'unknown')),
    package_version TEXT NOT NULL,
    build_id TEXT NOT NULL,
    started_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS diagnostic_events (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES diagnostic_sessions(session_id) ON DELETE CASCADE,
    event_kind TEXT NOT NULL CHECK (event_kind IN ('mcp_tool_call', 'guard_hook', 'session')),
    tool_name TEXT,
    latency_micros INTEGER NOT NULL CHECK (latency_micros >= 0),
    request_bytes INTEGER NOT NULL CHECK (request_bytes >= 0),
    response_bytes INTEGER NOT NULL CHECK (response_bytes >= 0),
    validation_failure INTEGER NOT NULL CHECK (validation_failure IN (0, 1)),
    retry_after_validation_failure INTEGER NOT NULL CHECK (retry_after_validation_failure IN (0, 1)),
    core_reached INTEGER NOT NULL CHECK (core_reached IN (0, 1)),
    core_committed INTEGER NOT NULL CHECK (core_committed IN (0, 1)),
    replayed INTEGER NOT NULL CHECK (replayed IN (0, 1)),
    user_channel_kind TEXT CHECK (
        user_channel_kind IS NULL OR user_channel_kind IN (
            'mcp_elicitation', 'prompt_capture', 'local_web_consent', 'cli_inbox'
        )
    ),
    fallback_kind TEXT CHECK (
        fallback_kind IS NULL OR fallback_kind IN (
            'prompt_capture', 'local_web_consent', 'cli_inbox'
        )
    ),
    product_file_write_count INTEGER NOT NULL CHECK (product_file_write_count >= 0),
    authoritative_refresh_failure INTEGER NOT NULL CHECK (authoritative_refresh_failure IN (0, 1)),
    outcome TEXT NOT NULL CHECK (
        outcome IN ('success', 'rejected', 'validation_failure', 'tool_error', 'transport_error', 'unavailable')
    ),
    occurred_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_diagnostic_sessions_updated
    ON diagnostic_sessions(updated_at DESC, session_id DESC);
CREATE INDEX IF NOT EXISTS idx_diagnostic_events_session
    ON diagnostic_events(session_id, event_id);
CREATE INDEX IF NOT EXISTS idx_diagnostic_events_tool
    ON diagnostic_events(session_id, tool_name, event_id);
"#;

pub(crate) const DIAGNOSTICS_SCHEMA_V2_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS workflow_metric_events (
    workflow_metric_event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES diagnostic_sessions(session_id) ON DELETE CASCADE,
    project_id TEXT,
    metric_kind TEXT NOT NULL CHECK (
        metric_kind IN (
            'task_duration_micros',
            'first_product_write_duration_micros',
            'mcp_method_call',
            'status_reread',
            'authority_refresh',
            'write_ticket_issued',
            'write_ticket_reused',
            'write_ticket_reissued',
            'user_roundtrip',
            'stop_call',
            'stop_repeat',
            'tools_list_serialized_bytes',
            'pre_tool_decision',
            'observation_assessment',
            'confirmed_out_of_scope_write',
            'suspected_resolved_no_change',
            'confirmed_unrecorded_false_positive',
            'confirmed_structured_write_deny',
            'sensitive_approval_missing_block',
            'completion_claim_suppressed'
        )
    ),
    value INTEGER NOT NULL CHECK (value >= 0),
    method_name TEXT CHECK (
        method_name IS NULL OR method_name IN (
            'volicord.intake',
            'volicord.update_scope',
            'volicord.status',
            'volicord.get_operation_result',
            'volicord.check_close',
            'volicord.prepare_evidence_capture',
            'volicord.prepare_write',
            'volicord.stage_artifact',
            'volicord.record_run',
            'volicord.request_user_action',
            'volicord.resolve_user_action',
            'volicord.reconcile_changes',
            'volicord.close_task'
        )
    ),
    integration_profile TEXT CHECK (
        integration_profile IS NULL OR integration_profile IN ('record', 'detective')
    ),
    decision TEXT CHECK (
        decision IS NULL OR decision IN ('allow', 'warn', 'deny')
    ),
    observation_confidence TEXT CHECK (
        observation_confidence IS NULL OR observation_confidence IN (
            'confirmed', 'structured', 'heuristic', 'unknown'
        )
    ),
    outcome TEXT CHECK (
        outcome IS NULL OR outcome IN (
            'success', 'rejected', 'validation_failure', 'tool_error',
            'transport_error', 'unavailable', 'read_only',
            'product_file_write', 'non_product_write', 'external_effect',
            'unknown'
        )
    ),
    occurred_at TEXT NOT NULL,
    CHECK (
        (metric_kind = 'mcp_method_call' AND method_name IS NOT NULL)
        OR (metric_kind <> 'mcp_method_call' AND method_name IS NULL)
    ),
    CHECK (
        (
            metric_kind = 'pre_tool_decision'
            AND decision IS NOT NULL
            AND observation_confidence IS NOT NULL
            AND outcome IS NULL
        )
        OR (
            metric_kind = 'observation_assessment'
            AND decision IS NULL
            AND observation_confidence IS NOT NULL
            AND outcome IN (
                'read_only', 'product_file_write', 'non_product_write',
                'external_effect', 'unknown'
            )
        )
        OR (
            metric_kind NOT IN ('pre_tool_decision', 'observation_assessment')
            AND decision IS NULL
            AND observation_confidence IS NULL
            AND (
                outcome IS NULL OR outcome IN (
                    'success', 'rejected', 'validation_failure', 'tool_error',
                    'transport_error', 'unavailable'
                )
            )
        )
    )
);

CREATE INDEX IF NOT EXISTS idx_workflow_metric_events_session
    ON workflow_metric_events(session_id, workflow_metric_event_id);
CREATE INDEX IF NOT EXISTS idx_workflow_metric_events_aggregate
    ON workflow_metric_events(
        project_id, metric_kind, method_name, integration_profile,
        decision, observation_confidence, outcome
    );
"#;

/// Returns the diagnostics database path for a Runtime Home.
pub fn diagnostics_db_path(runtime_home: impl AsRef<Path>) -> PathBuf {
    runtime_home.as_ref().join(DIAGNOSTICS_DB_FILE)
}

/// Controlled transport category for one diagnostic session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTransport {
    McpStdio,
    GuardHook,
    LocalHttp,
    Unknown,
}

impl DiagnosticTransport {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::McpStdio => "mcp_stdio",
            Self::GuardHook => "guard_hook",
            Self::LocalHttp => "local_http",
            Self::Unknown => "unknown",
        }
    }
}

/// Controlled host category retained without host configuration content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticHostKind {
    Codex,
    ClaudeCode,
    Generic,
    Unknown,
}

impl DiagnosticHostKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude_code",
            Self::Generic => "generic",
            Self::Unknown => "unknown",
        }
    }

    /// Maps a stored Agent Connection host kind to the bounded diagnostic set.
    pub fn from_connection_host_kind(value: &str) -> Self {
        match value {
            "codex" => Self::Codex,
            "claude_code" => Self::ClaudeCode,
            "generic" => Self::Generic,
            _ => Self::Unknown,
        }
    }
}

/// Controlled diagnostic event category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticEventKind {
    McpToolCall,
    GuardHook,
    Session,
}

impl DiagnosticEventKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::McpToolCall => "mcp_tool_call",
            Self::GuardHook => "guard_hook",
            Self::Session => "session",
        }
    }
}

/// Controlled result category. Error bodies are deliberately not accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticOutcome {
    Success,
    Rejected,
    ValidationFailure,
    ToolError,
    TransportError,
    Unavailable,
}

impl DiagnosticOutcome {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Rejected => "rejected",
            Self::ValidationFailure => "validation_failure",
            Self::ToolError => "tool_error",
            Self::TransportError => "transport_error",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Controlled verified User Channel category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticUserChannelKind {
    McpElicitation,
    PromptCapture,
    LocalWebConsent,
    CliInbox,
}

impl DiagnosticUserChannelKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::McpElicitation => "mcp_elicitation",
            Self::PromptCapture => "prompt_capture",
            Self::LocalWebConsent => "local_web_consent",
            Self::CliInbox => "cli_inbox",
        }
    }
}

/// Controlled pending-user-action fallback category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticFallbackKind {
    PromptCapture,
    LocalWebConsent,
    CliInbox,
}

impl DiagnosticFallbackKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::PromptCapture => "prompt_capture",
            Self::LocalWebConsent => "local_web_consent",
            Self::CliInbox => "cli_inbox",
        }
    }
}

/// Closed kind set for privacy-bounded workflow measurements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowMetricKind {
    TaskDurationMicros,
    FirstProductWriteDurationMicros,
    McpMethodCall,
    StatusReread,
    AuthorityRefresh,
    WriteTicketIssued,
    WriteTicketReused,
    WriteTicketReissued,
    UserRoundtrip,
    StopCall,
    StopRepeat,
    ToolsListSerializedBytes,
    PreToolDecision,
    ObservationAssessment,
    ConfirmedOutOfScopeWrite,
    SuspectedResolvedNoChange,
    ConfirmedUnrecordedFalsePositive,
    ConfirmedStructuredWriteDeny,
    SensitiveApprovalMissingBlock,
    CompletionClaimSuppressed,
}

impl WorkflowMetricKind {
    /// All supported workflow metric kinds in stable contract order.
    pub const ALL: [Self; 20] = [
        Self::TaskDurationMicros,
        Self::FirstProductWriteDurationMicros,
        Self::McpMethodCall,
        Self::StatusReread,
        Self::AuthorityRefresh,
        Self::WriteTicketIssued,
        Self::WriteTicketReused,
        Self::WriteTicketReissued,
        Self::UserRoundtrip,
        Self::StopCall,
        Self::StopRepeat,
        Self::ToolsListSerializedBytes,
        Self::PreToolDecision,
        Self::ObservationAssessment,
        Self::ConfirmedOutOfScopeWrite,
        Self::SuspectedResolvedNoChange,
        Self::ConfirmedUnrecordedFalsePositive,
        Self::ConfirmedStructuredWriteDeny,
        Self::SensitiveApprovalMissingBlock,
        Self::CompletionClaimSuppressed,
    ];

    /// Returns the stable storage spelling for this metric kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TaskDurationMicros => "task_duration_micros",
            Self::FirstProductWriteDurationMicros => "first_product_write_duration_micros",
            Self::McpMethodCall => "mcp_method_call",
            Self::StatusReread => "status_reread",
            Self::AuthorityRefresh => "authority_refresh",
            Self::WriteTicketIssued => "write_ticket_issued",
            Self::WriteTicketReused => "write_ticket_reused",
            Self::WriteTicketReissued => "write_ticket_reissued",
            Self::UserRoundtrip => "user_roundtrip",
            Self::StopCall => "stop_call",
            Self::StopRepeat => "stop_repeat",
            Self::ToolsListSerializedBytes => "tools_list_serialized_bytes",
            Self::PreToolDecision => "pre_tool_decision",
            Self::ObservationAssessment => "observation_assessment",
            Self::ConfirmedOutOfScopeWrite => "confirmed_out_of_scope_write",
            Self::SuspectedResolvedNoChange => "suspected_resolved_no_change",
            Self::ConfirmedUnrecordedFalsePositive => "confirmed_unrecorded_false_positive",
            Self::ConfirmedStructuredWriteDeny => "confirmed_structured_write_deny",
            Self::SensitiveApprovalMissingBlock => "sensitive_approval_missing_block",
            Self::CompletionClaimSuppressed => "completion_claim_suppressed",
        }
    }
}

/// Closed allow/warn/deny dimension for a PreTool decision metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowMetricDecision {
    Allow,
    Warn,
    Deny,
}

impl WorkflowMetricDecision {
    /// Returns the stable storage spelling for this decision.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Deny => "deny",
        }
    }
}

/// Closed categorical result or observation-effect dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowMetricOutcome {
    Success,
    Rejected,
    ValidationFailure,
    ToolError,
    TransportError,
    Unavailable,
    ReadOnly,
    ProductFileWrite,
    NonProductWrite,
    ExternalEffect,
    Unknown,
}

impl WorkflowMetricOutcome {
    /// Returns the stable storage spelling for this bounded outcome.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Rejected => "rejected",
            Self::ValidationFailure => "validation_failure",
            Self::ToolError => "tool_error",
            Self::TransportError => "transport_error",
            Self::Unavailable => "unavailable",
            Self::ReadOnly => "read_only",
            Self::ProductFileWrite => "product_file_write",
            Self::NonProductWrite => "non_product_write",
            Self::ExternalEffect => "external_effect",
            Self::Unknown => "unknown",
        }
    }

    const fn is_observation_effect(self) -> bool {
        matches!(
            self,
            Self::ReadOnly
                | Self::ProductFileWrite
                | Self::NonProductWrite
                | Self::ExternalEffect
                | Self::Unknown
        )
    }
}

/// Strict content-free input for one workflow metric observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowMetricEvent {
    pub session_id: String,
    pub metric_kind: WorkflowMetricKind,
    pub value: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method_name: Option<MethodName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration_profile: Option<IntegrationProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<WorkflowMetricDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation_confidence: Option<ObservationConfidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<WorkflowMetricOutcome>,
}

/// Aggregate-only workflow metric row for one bounded dimension group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkflowMetricAggregateRow {
    pub metric_kind: String,
    pub method_name: Option<String>,
    pub host_kind: Option<String>,
    pub integration_profile: Option<String>,
    pub decision: Option<String>,
    pub effect: Option<String>,
    pub observation_confidence: Option<String>,
    pub outcome: Option<String>,
    pub sample_count: u64,
    pub value_total: u64,
    pub value_min: u64,
    pub value_max: u64,
}

/// Metadata used to start or refresh one bounded diagnostic session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticSessionStart<'a> {
    pub session_id: &'a str,
    pub connection_id: Option<&'a str>,
    pub project_id: Option<&'a str>,
    pub transport: DiagnosticTransport,
    pub host_kind: Option<DiagnosticHostKind>,
    pub package_version: &'a str,
    pub build_id: &'a str,
}

/// Content-free observation recorded for one tool or hook boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticEvent<'a> {
    pub session_id: &'a str,
    pub event_kind: DiagnosticEventKind,
    pub tool_name: Option<&'a str>,
    pub latency_micros: u64,
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub validation_failure: bool,
    pub core_reached: bool,
    pub core_committed: bool,
    pub replayed: bool,
    pub user_channel_kind: Option<DiagnosticUserChannelKind>,
    pub fallback_kind: Option<DiagnosticFallbackKind>,
    pub product_file_write_count: u64,
    pub authoritative_refresh_failure: bool,
    pub outcome: DiagnosticOutcome,
}

/// Bounded per-tool aggregate returned by the diagnostics reader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticToolAggregate {
    pub tool_name: String,
    pub call_count: u64,
    pub latency_micros_total: u64,
    pub latency_micros_max: u64,
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub validation_failures: u64,
    pub retries_after_validation_failure: u64,
    pub core_reached_count: u64,
    pub core_committed_count: u64,
    pub replayed_count: u64,
}

/// Aggregate counters for one diagnostic session.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DiagnosticTotals {
    pub event_count: u64,
    pub tool_call_count: u64,
    pub validation_failures: u64,
    pub retries_after_validation_failure: u64,
    pub core_reached_count: u64,
    pub core_committed_count: u64,
    pub replayed_count: u64,
    pub product_file_write_count: u64,
    pub authoritative_refresh_failures: u64,
}

/// Read-only aggregate for one local diagnostic session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticSessionAggregate {
    pub session_id: String,
    pub connection_id: Option<String>,
    pub project_id: Option<String>,
    pub transport: String,
    pub host_kind: Option<String>,
    pub package_version: String,
    pub build_id: String,
    pub started_at: String,
    pub updated_at: String,
    pub tools: Vec<DiagnosticToolAggregate>,
    pub totals: DiagnosticTotals,
    pub user_channel_counts: BTreeMap<String, u64>,
    pub fallback_counts: BTreeMap<String, u64>,
}

/// Creates or refreshes a local diagnostic session and enforces retention.
pub fn start_diagnostic_session(
    runtime_home: impl AsRef<Path>,
    input: DiagnosticSessionStart<'_>,
) -> StoreResult<()> {
    validate_diagnostic_session_start_shape(&input)?;

    let mut conn = open_diagnostics_database(runtime_home)?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    validate_managed_diagnostic_session_binding(&tx, &input)?;
    tx.execute(
        "INSERT INTO diagnostic_sessions (
             session_id, connection_id, project_id, transport, host_kind,
             package_version, build_id, started_at, updated_at
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7,
             strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
             strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         )
         ON CONFLICT(session_id) DO UPDATE SET
             connection_id = COALESCE(excluded.connection_id, diagnostic_sessions.connection_id),
             project_id = COALESCE(excluded.project_id, diagnostic_sessions.project_id),
             transport = CASE
                 WHEN diagnostic_sessions.transport = 'unknown'
                      OR excluded.transport IN ('mcp_stdio', 'local_http')
                 THEN excluded.transport
                 ELSE diagnostic_sessions.transport
             END,
             host_kind = COALESCE(excluded.host_kind, diagnostic_sessions.host_kind),
             package_version = excluded.package_version,
             build_id = excluded.build_id,
             updated_at = excluded.updated_at",
        params![
            input.session_id,
            input.connection_id,
            input.project_id,
            input.transport.as_str(),
            input.host_kind.map(DiagnosticHostKind::as_str),
            input.package_version,
            input.build_id,
        ],
    )?;
    prune_diagnostics(&tx)?;
    tx.commit()?;
    Ok(())
}

/// Validates a diagnostic session start without creating or updating diagnostics storage.
pub fn validate_diagnostic_session_start(
    runtime_home: impl AsRef<Path>,
    input: DiagnosticSessionStart<'_>,
) -> StoreResult<()> {
    validate_diagnostic_session_start_shape(&input)?;
    let path = diagnostics_db_path(runtime_home);
    if !path.exists() {
        return Ok(());
    }
    let conn = open_diagnostics_database_read_only(&path)?;
    validate_managed_diagnostic_session_binding(&conn, &input)
}

fn validate_diagnostic_session_start_shape(input: &DiagnosticSessionStart<'_>) -> StoreResult<()> {
    validate_identifier("session_id", input.session_id)?;
    validate_optional_identifier("connection_id", input.connection_id)?;
    validate_optional_identifier("project_id", input.project_id)?;
    validate_build_value("package_version", input.package_version, 128)?;
    validate_build_value("build_id", input.build_id, 2_048)?;
    if input.session_id.starts_with(MANAGED_HOST_SESSION_ID_PREFIX) {
        validate_managed_host_session_id(input.session_id).map_err(|error| {
            StoreError::InvalidInput {
                detail: error.to_string(),
            }
        })?;
    }
    if input.session_id.starts_with(MANAGED_HOST_SESSION_ID_PREFIX)
        && (input.connection_id.is_none()
            || !matches!(
                input.host_kind,
                Some(DiagnosticHostKind::Codex | DiagnosticHostKind::ClaudeCode)
            )
            || !matches!(
                input.transport,
                DiagnosticTransport::McpStdio | DiagnosticTransport::GuardHook
            ))
    {
        return Err(StoreError::InvalidInput {
            detail: "mhs_ diagnostic sessions require a managed built-in host, connection, and managed transport"
                .to_owned(),
        });
    }
    Ok(())
}

fn validate_managed_diagnostic_session_binding(
    conn: &Connection,
    input: &DiagnosticSessionStart<'_>,
) -> StoreResult<()> {
    if !input.session_id.starts_with(MANAGED_HOST_SESSION_ID_PREFIX) {
        return Ok(());
    }
    let existing = conn
        .query_row(
            "SELECT connection_id, host_kind
               FROM diagnostic_sessions
              WHERE session_id = ?1",
            [input.session_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        )
        .optional()?;
    let Some((existing_connection_id, existing_host_kind)) = existing else {
        return Ok(());
    };
    let expected_connection_id = input.connection_id.map(str::to_owned);
    let expected_host_kind = input
        .host_kind
        .map(DiagnosticHostKind::as_str)
        .map(str::to_owned);
    if existing_connection_id != expected_connection_id || existing_host_kind != expected_host_kind
    {
        return Err(StoreError::Conflict {
            entity: "diagnostic_session",
            id: input.session_id.to_owned(),
            detail: "managed-host session diagnostics are already bound to a different connection or host"
                .to_owned(),
        });
    }
    Ok(())
}

/// Records one content-free event and enforces per-session retention.
pub fn record_diagnostic_event(
    runtime_home: impl AsRef<Path>,
    input: DiagnosticEvent<'_>,
) -> StoreResult<()> {
    validate_identifier("session_id", input.session_id)?;
    validate_optional_tool_name(input.tool_name)?;
    let latency_micros = sqlite_integer(input.latency_micros, "latency_micros")?;
    let request_bytes = sqlite_integer(input.request_bytes, "request_bytes")?;
    let response_bytes = sqlite_integer(input.response_bytes, "response_bytes")?;
    let product_file_write_count =
        sqlite_integer(input.product_file_write_count, "product_file_write_count")?;

    let mut conn = open_diagnostics_database(runtime_home)?;
    let tx = conn.transaction()?;
    let retry_after_validation_failure = input.tool_name.is_some()
        && tx
            .query_row(
                "SELECT validation_failure
                   FROM diagnostic_events
                  WHERE session_id = ?1 AND tool_name = ?2
                  ORDER BY event_id DESC
                  LIMIT 1",
                params![input.session_id, input.tool_name],
                |row| row.get::<_, bool>(0),
            )
            .optional()?
            .unwrap_or(false)
        && !input.validation_failure;
    tx.execute(
        "INSERT INTO diagnostic_events (
             session_id, event_kind, tool_name, latency_micros,
             request_bytes, response_bytes, validation_failure,
             retry_after_validation_failure, core_reached, core_committed,
             replayed, user_channel_kind, fallback_kind,
             product_file_write_count, authoritative_refresh_failure,
             outcome, occurred_at
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
             ?13, ?14, ?15, ?16,
             strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         )",
        params![
            input.session_id,
            input.event_kind.as_str(),
            input.tool_name,
            latency_micros,
            request_bytes,
            response_bytes,
            input.validation_failure,
            retry_after_validation_failure,
            input.core_reached,
            input.core_committed,
            input.replayed,
            input
                .user_channel_kind
                .map(DiagnosticUserChannelKind::as_str),
            input.fallback_kind.map(DiagnosticFallbackKind::as_str),
            product_file_write_count,
            input.authoritative_refresh_failure,
            input.outcome.as_str(),
        ],
    )?;
    tx.execute(
        "UPDATE diagnostic_sessions
            SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE session_id = ?1",
        [input.session_id],
    )?;
    trim_session_events(&tx, input.session_id)?;
    prune_diagnostics(&tx)?;
    tx.commit()?;
    Ok(())
}

/// Records one privacy-bounded workflow metric and enforces shared event retention.
pub fn record_workflow_metric_event(
    runtime_home: impl AsRef<Path>,
    input: &WorkflowMetricEvent,
) -> StoreResult<()> {
    validate_workflow_metric_event(input)?;
    let value = sqlite_integer(input.value, "workflow metric value")?;

    let mut conn = open_diagnostics_database(runtime_home)?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let project_id = tx
        .query_row(
            "SELECT project_id
               FROM diagnostic_sessions
              WHERE session_id = ?1",
            [&input.session_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "diagnostic_session",
            id: input.session_id.clone(),
        })?;
    tx.execute(
        "INSERT INTO workflow_metric_events (
             session_id, project_id, metric_kind, value, method_name,
             integration_profile, decision, observation_confidence, outcome,
             occurred_at
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
             strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         )",
        params![
            input.session_id,
            project_id,
            input.metric_kind.as_str(),
            value,
            input.method_name.map(MethodName::as_str),
            input.integration_profile.map(IntegrationProfile::as_str),
            input.decision.map(WorkflowMetricDecision::as_str),
            input
                .observation_confidence
                .map(observation_confidence_as_str),
            input.outcome.map(WorkflowMetricOutcome::as_str),
        ],
    )?;
    tx.execute(
        "UPDATE diagnostic_sessions
            SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE session_id = ?1",
        [&input.session_id],
    )?;
    trim_session_events(&tx, &input.session_id)?;
    prune_diagnostics(&tx)?;
    tx.commit()?;
    Ok(())
}

/// Reads project-scoped workflow metrics as aggregate rows without creating storage.
pub fn read_workflow_metric_aggregates(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<Vec<WorkflowMetricAggregateRow>> {
    validate_identifier("project_id", project_id)?;
    let path = diagnostics_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_diagnostics_database_read_only(&path)?;
    let mut statement = conn.prepare(
        "SELECT metrics.metric_kind,
                metrics.method_name,
                sessions.host_kind,
                metrics.integration_profile,
                metrics.decision,
                CASE
                    WHEN metrics.metric_kind = 'observation_assessment'
                    THEN metrics.outcome
                    ELSE NULL
                END AS effect,
                metrics.observation_confidence,
                CASE
                    WHEN metrics.metric_kind = 'observation_assessment'
                    THEN NULL
                    ELSE metrics.outcome
                END AS outcome,
                COUNT(*),
                COALESCE(SUM(metrics.value), 0),
                COALESCE(MIN(metrics.value), 0),
                COALESCE(MAX(metrics.value), 0)
           FROM workflow_metric_events AS metrics
           JOIN diagnostic_sessions AS sessions
             ON sessions.session_id = metrics.session_id
          WHERE metrics.project_id = ?1
          GROUP BY metrics.metric_kind,
                   metrics.method_name,
                   sessions.host_kind,
                   metrics.integration_profile,
                   metrics.decision,
                   metrics.observation_confidence,
                   metrics.outcome
          ORDER BY metrics.metric_kind,
                   COALESCE(metrics.method_name, ''),
                   COALESCE(sessions.host_kind, ''),
                   COALESCE(metrics.integration_profile, ''),
                   COALESCE(metrics.decision, ''),
                   COALESCE(metrics.observation_confidence, ''),
                   COALESCE(metrics.outcome, '')",
    )?;
    let rows = statement.query_map([project_id], |row| {
        Ok(WorkflowMetricAggregateRow {
            metric_kind: row.get(0)?,
            method_name: row.get(1)?,
            host_kind: row.get(2)?,
            integration_profile: row.get(3)?,
            decision: row.get(4)?,
            effect: row.get(5)?,
            observation_confidence: row.get(6)?,
            outcome: row.get(7)?,
            sample_count: row.get(8)?,
            value_total: row.get(9)?,
            value_min: row.get(10)?,
            value_max: row.get(11)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Reads the latest session, or an explicitly selected session, without creating storage.
pub fn read_diagnostic_session(
    runtime_home: impl AsRef<Path>,
    session_id: Option<&str>,
) -> StoreResult<Option<DiagnosticSessionAggregate>> {
    if let Some(session_id) = session_id {
        validate_identifier("session_id", session_id)?;
    }
    let path = diagnostics_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_diagnostics_database_read_only(&path)?;
    let selected = if let Some(session_id) = session_id {
        conn.query_row(
            "SELECT session_id, connection_id, project_id, transport, host_kind,
                    package_version, build_id, started_at, updated_at
               FROM diagnostic_sessions
              WHERE session_id = ?1",
            [session_id],
            read_session_row,
        )
        .optional()?
    } else {
        conn.query_row(
            "SELECT session_id, connection_id, project_id, transport, host_kind,
                    package_version, build_id, started_at, updated_at
               FROM diagnostic_sessions
              ORDER BY julianday(updated_at) DESC, session_id DESC
              LIMIT 1",
            [],
            read_session_row,
        )
        .optional()?
    };
    let Some(mut aggregate) = selected else {
        return Ok(None);
    };
    aggregate.tools = read_tool_aggregates(&conn, &aggregate.session_id)?;
    aggregate.totals = read_totals(&conn, &aggregate.session_id)?;
    aggregate.user_channel_counts =
        read_category_counts(&conn, &aggregate.session_id, "user_channel_kind")?;
    aggregate.fallback_counts =
        read_category_counts(&conn, &aggregate.session_id, "fallback_kind")?;
    Ok(Some(aggregate))
}

fn open_diagnostics_database(runtime_home: impl AsRef<Path>) -> StoreResult<Connection> {
    let path = diagnostics_db_path(runtime_home);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(&path)?;
    harden_diagnostics_permissions(&path)?;
    conn.busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MILLIS))?;
    enable_foreign_keys(&conn)?;
    let version = conn.query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))?;
    match version {
        0 => {
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            tx.execute_batch(DIAGNOSTICS_SCHEMA_V1_SQL)?;
            tx.execute_batch(DIAGNOSTICS_SCHEMA_V2_SQL)?;
            tx.pragma_update(None, "user_version", DIAGNOSTICS_SCHEMA_VERSION)?;
            tx.commit()?;
        }
        1 => {
            validate_diagnostics_schema_v1(&conn)?;
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            tx.execute_batch(DIAGNOSTICS_SCHEMA_V2_SQL)?;
            tx.pragma_update(None, "user_version", DIAGNOSTICS_SCHEMA_VERSION)?;
            tx.commit()?;
        }
        DIAGNOSTICS_SCHEMA_VERSION => {}
        _ => {
            return Err(StoreError::UnsupportedStorageProfile {
                database_kind: DATABASE_KIND,
                actual_storage_profile: version.to_string(),
                expected_storage_profile: "2",
            });
        }
    }
    validate_diagnostics_schema(&conn)?;
    Ok(conn)
}

pub(crate) fn ensure_current_diagnostics_schema(runtime_home: impl AsRef<Path>) -> StoreResult<()> {
    open_diagnostics_database(runtime_home).map(drop)
}

fn open_diagnostics_database_read_only(path: &Path) -> StoreResult<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MILLIS))?;
    conn.pragma_update(None, "query_only", "ON")?;
    let version = conn.query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))?;
    if version != DIAGNOSTICS_SCHEMA_VERSION {
        return Err(StoreError::UnsupportedStorageProfile {
            database_kind: DATABASE_KIND,
            actual_storage_profile: version.to_string(),
            expected_storage_profile: "2",
        });
    }
    validate_diagnostics_schema(&conn)?;
    Ok(conn)
}

fn validate_diagnostics_schema(conn: &Connection) -> StoreResult<()> {
    validate_diagnostics_schema_v1(conn)?;
    let columns = table_columns(conn, "workflow_metric_events")?;
    let expected_columns = [
        "workflow_metric_event_id",
        "session_id",
        "project_id",
        "metric_kind",
        "value",
        "method_name",
        "integration_profile",
        "decision",
        "observation_confidence",
        "outcome",
        "occurred_at",
    ];
    if columns != expected_columns {
        return Err(StoreError::SchemaInvariant {
            database_kind: DATABASE_KIND,
            detail: format!(
                "workflow_metric_events columns do not match the privacy-bounded v2 schema: {columns:?}"
            ),
        });
    }
    let table_sql = conn
        .query_row(
            "SELECT sql
               FROM sqlite_schema
              WHERE type = 'table' AND name = 'workflow_metric_events'",
            [],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten()
        .ok_or_else(|| StoreError::SchemaInvariant {
            database_kind: DATABASE_KIND,
            detail: "required table workflow_metric_events is missing".to_owned(),
        })?;
    for metric_kind in WorkflowMetricKind::ALL {
        let expected = format!("'{}'", metric_kind.as_str());
        if !table_sql.contains(&expected) {
            return Err(StoreError::SchemaInvariant {
                database_kind: DATABASE_KIND,
                detail: format!(
                    "workflow_metric_events is missing the closed metric kind {}",
                    metric_kind.as_str()
                ),
            });
        }
    }
    Ok(())
}

fn validate_diagnostics_schema_v1(conn: &Connection) -> StoreResult<()> {
    for table in ["diagnostic_sessions", "diagnostic_events"] {
        let exists = conn
            .query_row(
                "SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?1",
                [table],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !exists {
            return Err(StoreError::SchemaInvariant {
                database_kind: DATABASE_KIND,
                detail: format!("required table {table} is missing"),
            });
        }
    }
    Ok(())
}

fn table_columns(conn: &Connection, table: &'static str) -> StoreResult<Vec<String>> {
    // `table` is selected only by this module and is never runtime input.
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn prune_diagnostics(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM diagnostic_sessions
          WHERE julianday(updated_at) < julianday('now', ?1)",
        [format!("-{} days", DIAGNOSTICS_RETENTION_DAYS)],
    )?;
    conn.execute(
        "DELETE FROM diagnostic_sessions
          WHERE session_id NOT IN (
              SELECT session_id
                FROM diagnostic_sessions
               ORDER BY julianday(updated_at) DESC, session_id DESC
               LIMIT ?1
          )",
        [DIAGNOSTICS_MAX_SESSIONS],
    )?;
    Ok(())
}

fn trim_session_events(conn: &Connection, session_id: &str) -> rusqlite::Result<()> {
    const RETAINED_EVENTS: &str = "SELECT item_id, source_order
           FROM (
               SELECT event_id AS item_id,
                      0 AS source_order,
                      julianday(occurred_at) AS occurred_order,
                      occurred_at
                 FROM diagnostic_events
                WHERE session_id = ?1
               UNION ALL
               SELECT workflow_metric_event_id AS item_id,
                      1 AS source_order,
                      julianday(occurred_at) AS occurred_order,
                      occurred_at
                 FROM workflow_metric_events
                WHERE session_id = ?1
                ORDER BY occurred_order DESC,
                         occurred_at DESC,
                         source_order DESC,
                         item_id DESC
                LIMIT ?2
           )";
    conn.execute(
        &format!(
            "DELETE FROM diagnostic_events
              WHERE session_id = ?1
                AND event_id NOT IN (
                    SELECT item_id FROM ({RETAINED_EVENTS}) WHERE source_order = 0
                )"
        ),
        params![session_id, DIAGNOSTICS_MAX_EVENTS_PER_SESSION],
    )?;
    conn.execute(
        &format!(
            "DELETE FROM workflow_metric_events
              WHERE session_id = ?1
                AND workflow_metric_event_id NOT IN (
                    SELECT item_id FROM ({RETAINED_EVENTS}) WHERE source_order = 1
                )"
        ),
        params![session_id, DIAGNOSTICS_MAX_EVENTS_PER_SESSION],
    )?;
    Ok(())
}

fn read_session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DiagnosticSessionAggregate> {
    Ok(DiagnosticSessionAggregate {
        session_id: row.get(0)?,
        connection_id: row.get(1)?,
        project_id: row.get(2)?,
        transport: row.get(3)?,
        host_kind: row.get(4)?,
        package_version: row.get(5)?,
        build_id: row.get(6)?,
        started_at: row.get(7)?,
        updated_at: row.get(8)?,
        tools: Vec::new(),
        totals: DiagnosticTotals::default(),
        user_channel_counts: BTreeMap::new(),
        fallback_counts: BTreeMap::new(),
    })
}

fn read_tool_aggregates(
    conn: &Connection,
    session_id: &str,
) -> StoreResult<Vec<DiagnosticToolAggregate>> {
    let mut statement = conn.prepare(
        "SELECT tool_name,
                COUNT(*),
                COALESCE(SUM(latency_micros), 0),
                COALESCE(MAX(latency_micros), 0),
                COALESCE(SUM(request_bytes), 0),
                COALESCE(SUM(response_bytes), 0),
                COALESCE(SUM(validation_failure), 0),
                COALESCE(SUM(retry_after_validation_failure), 0),
                COALESCE(SUM(core_reached), 0),
                COALESCE(SUM(core_committed), 0),
                COALESCE(SUM(replayed), 0)
           FROM diagnostic_events
          WHERE session_id = ?1 AND event_kind = 'mcp_tool_call' AND tool_name IS NOT NULL
          GROUP BY tool_name
          ORDER BY tool_name",
    )?;
    let rows = statement.query_map([session_id], |row| {
        Ok(DiagnosticToolAggregate {
            tool_name: row.get(0)?,
            call_count: row.get(1)?,
            latency_micros_total: row.get(2)?,
            latency_micros_max: row.get(3)?,
            request_bytes: row.get(4)?,
            response_bytes: row.get(5)?,
            validation_failures: row.get(6)?,
            retries_after_validation_failure: row.get(7)?,
            core_reached_count: row.get(8)?,
            core_committed_count: row.get(9)?,
            replayed_count: row.get(10)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn read_totals(conn: &Connection, session_id: &str) -> StoreResult<DiagnosticTotals> {
    conn.query_row(
        "SELECT COUNT(*),
                COALESCE(SUM(CASE WHEN event_kind = 'mcp_tool_call' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(validation_failure), 0),
                COALESCE(SUM(retry_after_validation_failure), 0),
                COALESCE(SUM(core_reached), 0),
                COALESCE(SUM(core_committed), 0),
                COALESCE(SUM(replayed), 0),
                COALESCE(SUM(product_file_write_count), 0),
                COALESCE(SUM(authoritative_refresh_failure), 0)
           FROM diagnostic_events
          WHERE session_id = ?1",
        [session_id],
        |row| {
            Ok(DiagnosticTotals {
                event_count: row.get(0)?,
                tool_call_count: row.get(1)?,
                validation_failures: row.get(2)?,
                retries_after_validation_failure: row.get(3)?,
                core_reached_count: row.get(4)?,
                core_committed_count: row.get(5)?,
                replayed_count: row.get(6)?,
                product_file_write_count: row.get(7)?,
                authoritative_refresh_failures: row.get(8)?,
            })
        },
    )
    .map_err(Into::into)
}

fn read_category_counts(
    conn: &Connection,
    session_id: &str,
    column: &'static str,
) -> StoreResult<BTreeMap<String, u64>> {
    // `column` is selected only by this module, never from runtime input.
    let sql = format!(
        "SELECT {column}, COUNT(*)
           FROM diagnostic_events
          WHERE session_id = ?1 AND {column} IS NOT NULL
          GROUP BY {column}
          ORDER BY {column}"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map([session_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
    })?;
    rows.collect::<Result<BTreeMap<_, _>, _>>()
        .map_err(Into::into)
}

fn validate_workflow_metric_event(input: &WorkflowMetricEvent) -> StoreResult<()> {
    validate_identifier("session_id", &input.session_id)?;

    if matches!(input.metric_kind, WorkflowMetricKind::McpMethodCall) != input.method_name.is_some()
    {
        return Err(invalid_workflow_metric_dimensions(
            "method_name is required only for mcp_method_call",
        ));
    }

    match input.metric_kind {
        WorkflowMetricKind::PreToolDecision => {
            if input.decision.is_none()
                || input.observation_confidence.is_none()
                || input.outcome.is_some()
            {
                return Err(invalid_workflow_metric_dimensions(
                    "pre_tool_decision requires decision and observation_confidence and disallows outcome",
                ));
            }
        }
        WorkflowMetricKind::ObservationAssessment => {
            if input.decision.is_some()
                || input.observation_confidence.is_none()
                || !input
                    .outcome
                    .is_some_and(WorkflowMetricOutcome::is_observation_effect)
            {
                return Err(invalid_workflow_metric_dimensions(
                    "observation_assessment requires observation_confidence and a bounded effect outcome",
                ));
            }
        }
        _ => {
            if input.decision.is_some()
                || input.observation_confidence.is_some()
                || input
                    .outcome
                    .is_some_and(WorkflowMetricOutcome::is_observation_effect)
            {
                return Err(invalid_workflow_metric_dimensions(
                    "decision, observation_confidence, or effect outcome is not applicable to this metric kind",
                ));
            }
        }
    }
    Ok(())
}

fn invalid_workflow_metric_dimensions(detail: &str) -> StoreError {
    StoreError::InvalidInput {
        detail: format!("invalid workflow metric dimensions: {detail}"),
    }
}

const fn observation_confidence_as_str(value: ObservationConfidence) -> &'static str {
    match value {
        ObservationConfidence::Confirmed => "confirmed",
        ObservationConfidence::Structured => "structured",
        ObservationConfidence::Heuristic => "heuristic",
        ObservationConfidence::Unknown => "unknown",
    }
}

fn validate_identifier(field: &'static str, value: &str) -> StoreResult<()> {
    let valid = !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'));
    if valid {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("diagnostics {field} must be a bounded identifier"),
        })
    }
}

fn validate_optional_identifier(field: &'static str, value: Option<&str>) -> StoreResult<()> {
    value.map_or(Ok(()), |value| validate_identifier(field, value))
}

fn validate_optional_tool_name(value: Option<&str>) -> StoreResult<()> {
    value.map_or(Ok(()), |value| validate_identifier("tool_name", value))
}

fn validate_build_value(field: &'static str, value: &str, max_len: usize) -> StoreResult<()> {
    let valid = !value.is_empty()
        && value.len() <= max_len
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+' | b';' | b'=')
        });
    if valid {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("diagnostics {field} is not a bounded build identifier"),
        })
    }
}

fn sqlite_integer(value: u64, field: &'static str) -> StoreResult<i64> {
    i64::try_from(value).map_err(|_| StoreError::InvalidInput {
        detail: format!("diagnostics {field} exceeds the supported range"),
    })
}

#[cfg(unix)]
fn harden_diagnostics_permissions(path: &Path) -> StoreResult<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn harden_diagnostics_permissions(_path: &Path) -> StoreResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use volicord_test_support::TempRuntimeHome;
    use volicord_types::managed_host_session_id;

    fn start<'a>(session_id: &'a str) -> DiagnosticSessionStart<'a> {
        DiagnosticSessionStart {
            session_id,
            connection_id: Some("connection_test"),
            project_id: Some("project_test"),
            transport: DiagnosticTransport::McpStdio,
            host_kind: Some(DiagnosticHostKind::Codex),
            package_version: "0.1.0",
            build_id: "0.1.0;git=unknown;tree=unknown",
        }
    }

    fn event<'a>(session_id: &'a str, tool_name: &'a str) -> DiagnosticEvent<'a> {
        DiagnosticEvent {
            session_id,
            event_kind: DiagnosticEventKind::McpToolCall,
            tool_name: Some(tool_name),
            latency_micros: 25,
            request_bytes: 100,
            response_bytes: 200,
            validation_failure: false,
            core_reached: true,
            core_committed: true,
            replayed: false,
            user_channel_kind: Some(DiagnosticUserChannelKind::McpElicitation),
            fallback_kind: None,
            product_file_write_count: 1,
            authoritative_refresh_failure: false,
            outcome: DiagnosticOutcome::Success,
        }
    }

    fn metric(
        session_id: &str,
        metric_kind: WorkflowMetricKind,
        value: u64,
    ) -> WorkflowMetricEvent {
        WorkflowMetricEvent {
            session_id: session_id.to_owned(),
            metric_kind,
            value,
            method_name: None,
            integration_profile: None,
            decision: None,
            observation_confidence: None,
            outcome: None,
        }
    }

    #[test]
    fn diagnostics_are_separate_bounded_and_aggregate_without_content_columns() {
        let fixture = TempRuntimeHome::new("diagnostics-bounded").expect("fixture");
        start_diagnostic_session(fixture.path(), start("session_test")).expect("start");
        record_diagnostic_event(
            fixture.path(),
            DiagnosticEvent {
                validation_failure: true,
                core_reached: false,
                core_committed: false,
                outcome: DiagnosticOutcome::ValidationFailure,
                ..event("session_test", "volicord.record_run")
            },
        )
        .expect("validation event");
        record_diagnostic_event(fixture.path(), event("session_test", "volicord.record_run"))
            .expect("retry event");

        let aggregate = read_diagnostic_session(fixture.path(), None)
            .expect("read")
            .expect("session");
        assert_eq!(aggregate.totals.event_count, 2);
        assert_eq!(aggregate.totals.validation_failures, 1);
        assert_eq!(aggregate.totals.retries_after_validation_failure, 1);
        assert_eq!(aggregate.totals.core_reached_count, 1);
        assert_eq!(aggregate.totals.core_committed_count, 1);
        assert_eq!(aggregate.totals.product_file_write_count, 2);
        assert_eq!(aggregate.tools[0].request_bytes, 200);
        assert_eq!(aggregate.user_channel_counts["mcp_elicitation"], 2);

        let conn = Connection::open(diagnostics_db_path(fixture.path())).expect("diagnostics db");
        let columns = conn
            .prepare("PRAGMA table_info(diagnostic_events)")
            .expect("columns")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("column rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("column names");
        for forbidden in [
            "prompt",
            "path",
            "secret",
            "judgment_text",
            "detail",
            "body",
        ] {
            assert!(
                columns.iter().all(|column| !column.contains(forbidden)),
                "forbidden content column {forbidden}: {columns:?}"
            );
        }
        assert!(!fixture.path().join("projects").exists());
        assert!(!fixture.registry_db_path().exists());
    }

    #[test]
    fn diagnostics_reject_content_bearing_identifiers() {
        let fixture = TempRuntimeHome::new("diagnostics-redaction").expect("fixture");
        let mut input = start("session_test");
        input.connection_id = Some("/home/user/private-file.txt");
        assert!(start_diagnostic_session(fixture.path(), input).is_err());

        let mut input = start("session_test");
        input.build_id = "0.2.0;target=/private/build/location";
        assert!(start_diagnostic_session(fixture.path(), input).is_err());

        start_diagnostic_session(fixture.path(), start("session_test")).expect("start");
        let error = record_diagnostic_event(
            fixture.path(),
            event("session_test", "prompt text with secret"),
        )
        .expect_err("content must not fit tool field");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
    }

    #[test]
    fn managed_session_diagnostics_are_coordinate_immutable_and_exactly_idempotent() {
        let fixture = TempRuntimeHome::new("diagnostics-managed-binding").expect("fixture");
        let session_id = managed_host_session_id("codex", "connection_test", "native-session")
            .expect("managed test coordinates should bind");
        start_diagnostic_session(fixture.path(), start(&session_id)).expect("initial start");
        record_diagnostic_event(fixture.path(), event(&session_id, "volicord.status"))
            .expect("initial event");

        start_diagnostic_session(fixture.path(), start(&session_id))
            .expect("an exact managed binding is idempotent");
        let exact = read_diagnostic_session(fixture.path(), Some(&session_id))
            .expect("read exact binding")
            .expect("managed diagnostics session");
        assert_eq!(exact.connection_id.as_deref(), Some("connection_test"));
        assert_eq!(exact.host_kind.as_deref(), Some("codex"));
        assert_eq!(exact.totals.event_count, 1);

        let mut cross_connection = start(&session_id);
        cross_connection.connection_id = Some("connection_other");
        let error = start_diagnostic_session(fixture.path(), cross_connection)
            .expect_err("cross-connection managed reuse must fail");
        assert!(matches!(error, StoreError::Conflict { .. }));

        let mut cross_host = start(&session_id);
        cross_host.host_kind = Some(DiagnosticHostKind::ClaudeCode);
        let error = start_diagnostic_session(fixture.path(), cross_host)
            .expect_err("cross-host managed reuse must fail");
        assert!(matches!(error, StoreError::Conflict { .. }));

        let unchanged = read_diagnostic_session(fixture.path(), Some(&session_id))
            .expect("read unchanged binding")
            .expect("managed diagnostics session");
        assert_eq!(unchanged.connection_id.as_deref(), Some("connection_test"));
        assert_eq!(unchanged.host_kind.as_deref(), Some("codex"));
        assert_eq!(unchanged.totals.event_count, 1);
    }

    #[test]
    fn managed_session_prefix_is_rejected_for_generic_or_unmanaged_diagnostics() {
        let fixture = TempRuntimeHome::new("diagnostics-managed-prefix").expect("fixture");
        let session_id = managed_host_session_id("codex", "connection_test", "native-session")
            .expect("managed test coordinates should bind");

        let mut generic = start(&session_id);
        generic.host_kind = Some(DiagnosticHostKind::Generic);
        assert!(matches!(
            start_diagnostic_session(fixture.path(), generic),
            Err(StoreError::InvalidInput { .. })
        ));

        let mut local_http = start(&session_id);
        local_http.transport = DiagnosticTransport::LocalHttp;
        assert!(matches!(
            start_diagnostic_session(fixture.path(), local_http),
            Err(StoreError::InvalidInput { .. })
        ));

        let malformed = start("mhs_not-a-canonical-binding");
        assert!(matches!(
            start_diagnostic_session(fixture.path(), malformed),
            Err(StoreError::InvalidInput { .. })
        ));
        assert!(!diagnostics_db_path(fixture.path()).exists());
    }

    #[test]
    fn per_session_event_retention_is_enforced() {
        let fixture = TempRuntimeHome::new("diagnostics-event-retention").expect("fixture");
        start_diagnostic_session(fixture.path(), start("session_test")).expect("start");
        for _ in 0..(DIAGNOSTICS_MAX_EVENTS_PER_SESSION + 3) {
            record_diagnostic_event(fixture.path(), event("session_test", "volicord.status"))
                .expect("event");
        }
        let aggregate = read_diagnostic_session(fixture.path(), Some("session_test"))
            .expect("read")
            .expect("session");
        assert_eq!(
            aggregate.totals.event_count,
            u64::from(DIAGNOSTICS_MAX_EVENTS_PER_SESSION)
        );
    }

    #[test]
    fn session_count_retention_is_enforced() {
        let fixture = TempRuntimeHome::new("diagnostics-session-retention").expect("fixture");
        for index in 0..(DIAGNOSTICS_MAX_SESSIONS + 3) {
            let session_id = format!("session_{index:03}");
            start_diagnostic_session(fixture.path(), start(&session_id)).expect("session");
        }
        let conn = Connection::open(diagnostics_db_path(fixture.path())).expect("diagnostics db");
        let count = conn
            .query_row("SELECT COUNT(*) FROM diagnostic_sessions", [], |row| {
                row.get::<_, u64>(0)
            })
            .expect("session count");
        assert_eq!(count, u64::from(DIAGNOSTICS_MAX_SESSIONS));
        assert!(read_diagnostic_session(fixture.path(), Some("session_000"))
            .expect("oldest session")
            .is_none());
        assert!(read_diagnostic_session(fixture.path(), Some("session_066"))
            .expect("latest session")
            .is_some());
    }

    #[test]
    fn age_retention_compares_iso_timestamps_as_time_values() {
        let fixture = TempRuntimeHome::new("diagnostics-age-retention").expect("fixture");
        start_diagnostic_session(fixture.path(), start("session_expired")).expect("expired");
        start_diagnostic_session(fixture.path(), start("session_recent")).expect("recent");
        let conn = Connection::open(diagnostics_db_path(fixture.path())).expect("diagnostics db");
        conn.execute(
            "UPDATE diagnostic_sessions
                SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-7 days', '-1 hour')
              WHERE session_id = 'session_expired'",
            [],
        )
        .expect("backdate expired session");
        conn.execute(
            "UPDATE diagnostic_sessions
                SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-6 days')
              WHERE session_id = 'session_recent'",
            [],
        )
        .expect("backdate recent session");
        drop(conn);

        start_diagnostic_session(fixture.path(), start("session_trigger")).expect("trigger prune");

        assert!(
            read_diagnostic_session(fixture.path(), Some("session_expired"))
                .expect("expired read")
                .is_none()
        );
        assert!(
            read_diagnostic_session(fixture.path(), Some("session_recent"))
                .expect("recent read")
                .is_some()
        );
    }

    #[test]
    fn workflow_metric_kind_and_payload_are_closed_and_content_free() {
        let kinds = WorkflowMetricKind::ALL.map(WorkflowMetricKind::as_str);
        assert_eq!(
            kinds,
            [
                "task_duration_micros",
                "first_product_write_duration_micros",
                "mcp_method_call",
                "status_reread",
                "authority_refresh",
                "write_ticket_issued",
                "write_ticket_reused",
                "write_ticket_reissued",
                "user_roundtrip",
                "stop_call",
                "stop_repeat",
                "tools_list_serialized_bytes",
                "pre_tool_decision",
                "observation_assessment",
                "confirmed_out_of_scope_write",
                "suspected_resolved_no_change",
                "confirmed_unrecorded_false_positive",
                "confirmed_structured_write_deny",
                "sensitive_approval_missing_block",
                "completion_claim_suppressed",
            ]
        );

        let valid = serde_json::json!({
            "session_id": "session_test",
            "metric_kind": "status_reread",
            "value": 1
        });
        serde_json::from_value::<WorkflowMetricEvent>(valid.clone()).expect("closed payload");
        for forbidden in ["prompt", "command", "path", "content", "user_answer"] {
            let mut payload = valid.clone();
            payload
                .as_object_mut()
                .expect("object")
                .insert(forbidden.to_owned(), serde_json::json!("private value"));
            assert!(
                serde_json::from_value::<WorkflowMetricEvent>(payload).is_err(),
                "content-bearing field {forbidden} must be rejected"
            );
        }
        let mut unknown_kind = valid;
        unknown_kind["metric_kind"] = serde_json::json!("custom_metric");
        assert!(serde_json::from_value::<WorkflowMetricEvent>(unknown_kind).is_err());
    }

    #[test]
    fn workflow_metric_dimensions_are_kind_specific() {
        let mut invalid = metric("session_test", WorkflowMetricKind::StatusReread, 1);
        invalid.method_name = Some(MethodName::Status);
        assert!(matches!(
            validate_workflow_metric_event(&invalid),
            Err(StoreError::InvalidInput { .. })
        ));

        invalid = metric("session_test", WorkflowMetricKind::McpMethodCall, 1);
        assert!(validate_workflow_metric_event(&invalid).is_err());

        invalid = metric("session_test", WorkflowMetricKind::PreToolDecision, 1);
        invalid.decision = Some(WorkflowMetricDecision::Allow);
        assert!(validate_workflow_metric_event(&invalid).is_err());

        invalid = metric("session_test", WorkflowMetricKind::ObservationAssessment, 1);
        invalid.observation_confidence = Some(ObservationConfidence::Heuristic);
        invalid.outcome = Some(WorkflowMetricOutcome::Success);
        assert!(validate_workflow_metric_event(&invalid).is_err());

        invalid = metric("session_test", WorkflowMetricKind::StopCall, 1);
        invalid.observation_confidence = Some(ObservationConfidence::Unknown);
        assert!(validate_workflow_metric_event(&invalid).is_err());
    }

    #[test]
    fn workflow_metrics_are_exposed_only_as_bounded_aggregate_rows() {
        let fixture = TempRuntimeHome::new("workflow-metric-aggregates").expect("fixture");
        start_diagnostic_session(fixture.path(), start("session_metrics")).expect("start");

        let mut method_call = metric("session_metrics", WorkflowMetricKind::McpMethodCall, 1);
        method_call.method_name = Some(MethodName::Status);
        method_call.integration_profile = Some(IntegrationProfile::Detective);
        method_call.outcome = Some(WorkflowMetricOutcome::Success);
        record_workflow_metric_event(fixture.path(), &method_call).expect("method call one");
        record_workflow_metric_event(fixture.path(), &method_call).expect("method call two");

        let mut pre_tool = metric("session_metrics", WorkflowMetricKind::PreToolDecision, 1);
        pre_tool.integration_profile = Some(IntegrationProfile::Detective);
        pre_tool.decision = Some(WorkflowMetricDecision::Allow);
        pre_tool.observation_confidence = Some(ObservationConfidence::Confirmed);
        record_workflow_metric_event(fixture.path(), &pre_tool).expect("pre-tool decision");

        let mut observation = metric(
            "session_metrics",
            WorkflowMetricKind::ObservationAssessment,
            3,
        );
        observation.integration_profile = Some(IntegrationProfile::Detective);
        observation.observation_confidence = Some(ObservationConfidence::Heuristic);
        observation.outcome = Some(WorkflowMetricOutcome::ReadOnly);
        record_workflow_metric_event(fixture.path(), &observation).expect("observation");

        let rows = read_workflow_metric_aggregates(fixture.path(), "project_test")
            .expect("aggregate rows");
        assert_eq!(rows.len(), 3);
        let calls = rows
            .iter()
            .find(|row| row.metric_kind == "mcp_method_call")
            .expect("method aggregate");
        assert_eq!(calls.method_name.as_deref(), Some("volicord.status"));
        assert_eq!(calls.host_kind.as_deref(), Some("codex"));
        assert_eq!(calls.integration_profile.as_deref(), Some("detective"));
        assert_eq!(calls.outcome.as_deref(), Some("success"));
        assert_eq!(calls.effect, None);
        assert_eq!(calls.sample_count, 2);
        assert_eq!(calls.value_total, 2);
        assert_eq!(calls.value_min, 1);
        assert_eq!(calls.value_max, 1);

        let assessed = rows
            .iter()
            .find(|row| row.metric_kind == "observation_assessment")
            .expect("observation aggregate");
        assert_eq!(assessed.effect.as_deref(), Some("read_only"));
        assert_eq!(assessed.outcome, None);
        assert_eq!(
            assessed.observation_confidence.as_deref(),
            Some("heuristic")
        );
        assert_eq!(assessed.value_total, 3);
        assert!(
            read_workflow_metric_aggregates(fixture.path(), "project_other")
                .expect("other project")
                .is_empty()
        );

        let conn = Connection::open(diagnostics_db_path(fixture.path())).expect("diagnostics db");
        let columns = table_columns(&conn, "workflow_metric_events").expect("columns");
        for forbidden in [
            "prompt",
            "command",
            "path",
            "content",
            "user_answer",
            "answer",
            "file_body",
            "error_detail",
            "secret",
        ] {
            assert!(
                columns.iter().all(|column| !column.contains(forbidden)),
                "forbidden workflow-metric column {forbidden}: {columns:?}"
            );
        }
    }

    #[test]
    fn diagnostics_v1_migrates_additively_to_workflow_metrics_v2() {
        let fixture = TempRuntimeHome::new("diagnostics-v1-migration").expect("fixture");
        let path = diagnostics_db_path(fixture.path());
        let conn = Connection::open(&path).expect("v1 diagnostics db");
        conn.execute_batch(DIAGNOSTICS_SCHEMA_V1_SQL)
            .expect("v1 schema");
        conn.pragma_update(None, "user_version", 1)
            .expect("v1 version");
        conn.execute(
            "INSERT INTO diagnostic_sessions (
                 session_id, connection_id, project_id, transport, host_kind,
                 package_version, build_id, started_at, updated_at
             ) VALUES (
                 'session_v1', 'connection_test', 'project_test', 'mcp_stdio',
                 'codex', '0.1.0', '0.1.0;git=unknown;tree=unknown',
                 '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z'
             )",
            [],
        )
        .expect("v1 session");
        conn.execute(
            "INSERT INTO diagnostic_events (
                 session_id, event_kind, tool_name, latency_micros,
                 request_bytes, response_bytes, validation_failure,
                 retry_after_validation_failure, core_reached, core_committed,
                 replayed, user_channel_kind, fallback_kind,
                 product_file_write_count, authoritative_refresh_failure,
                 outcome, occurred_at
             ) VALUES (
                 'session_v1', 'mcp_tool_call', 'volicord.status', 1,
                 2, 3, 0, 0, 1, 0, 0, NULL, NULL, 0, 0, 'success',
                 '2026-01-01T00:00:00.000Z'
             )",
            [],
        )
        .expect("v1 event");
        drop(conn);

        let status_reread = metric("session_v1", WorkflowMetricKind::StatusReread, 1);
        record_workflow_metric_event(fixture.path(), &status_reread).expect("migrated metric");

        let conn = Connection::open(&path).expect("migrated diagnostics db");
        let version = conn
            .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
            .expect("schema version");
        assert_eq!(version, DIAGNOSTICS_SCHEMA_VERSION);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM diagnostic_events", [], |row| {
                row.get::<_, u64>(0)
            })
            .expect("preserved v1 event"),
            1
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM workflow_metric_events", [], |row| row
                .get::<_, u64>(0),)
                .expect("new v2 event"),
            1
        );
    }

    #[test]
    fn workflow_metrics_share_the_per_session_event_retention_limit() {
        let fixture = TempRuntimeHome::new("workflow-metric-retention").expect("fixture");
        start_diagnostic_session(fixture.path(), start("session_metrics")).expect("start");
        record_diagnostic_event(fixture.path(), event("session_metrics", "volicord.status"))
            .expect("legacy event");
        let status_reread = metric("session_metrics", WorkflowMetricKind::StatusReread, 1);
        for _ in 0..DIAGNOSTICS_MAX_EVENTS_PER_SESSION {
            record_workflow_metric_event(fixture.path(), &status_reread).expect("workflow event");
        }

        let conn = Connection::open(diagnostics_db_path(fixture.path())).expect("diagnostics db");
        let legacy_count = conn
            .query_row(
                "SELECT COUNT(*) FROM diagnostic_events WHERE session_id = 'session_metrics'",
                [],
                |row| row.get::<_, u64>(0),
            )
            .expect("legacy count");
        let workflow_count = conn
            .query_row(
                "SELECT COUNT(*) FROM workflow_metric_events WHERE session_id = 'session_metrics'",
                [],
                |row| row.get::<_, u64>(0),
            )
            .expect("workflow count");
        assert_eq!(legacy_count + workflow_count, 1_024);
    }

    #[test]
    fn reads_do_not_create_a_diagnostics_database() {
        let fixture = TempRuntimeHome::new("diagnostics-read-only").expect("fixture");
        assert!(read_diagnostic_session(fixture.path(), None)
            .expect("empty read")
            .is_none());
        assert!(
            read_workflow_metric_aggregates(fixture.path(), "project_test")
                .expect("empty workflow read")
                .is_empty()
        );
        assert!(!diagnostics_db_path(fixture.path()).exists());
    }
}
