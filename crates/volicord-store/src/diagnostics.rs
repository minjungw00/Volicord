//! Bounded local operability diagnostics stored outside authority databases.
//!
//! This store intentionally has no references to `registry.sqlite` or project
//! `state.sqlite`. Callers treat writes as best-effort observations; failure of
//! this store must never change a Core or User Channel result.

use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use volicord_platform_fs::{
    publish_file_no_replace, NoReplaceFilePublicationEffect, NoReplaceFilePublicationOutcome,
};
use volicord_types::canonical::canonical_json_bytes;
use volicord_types::managed_mcp_client_info::validate_project_agent_session_id;
use volicord_types::values::{IntegrationProfile, MethodName, ObservationConfidence, UtcTimestamp};

use crate::{
    mutation::RuntimeHomeMutationContext, sqlite::enable_foreign_keys, StoreError, StoreResult,
};

/// Runtime Home filename for the non-authoritative diagnostics store.
pub const DIAGNOSTICS_DB_FILE: &str = "diagnostics.sqlite";
/// Semantic identity of the one accepted local diagnostics storage contract.
pub const DIAGNOSTICS_CONTRACT_ID: &str = "volicord.sqlite.diagnostics";
/// Maximum age retained for diagnostic sessions.
pub const DIAGNOSTICS_RETENTION_DAYS: u32 = 7;
/// Maximum diagnostic sessions retained in one Runtime Home.
pub const DIAGNOSTICS_MAX_SESSIONS: u32 = 64;
/// Maximum diagnostic events retained for one session.
pub const DIAGNOSTICS_MAX_EVENTS_PER_SESSION: u32 = 1_024;
/// Maximum Core rejection observations retained in one Runtime Home.
pub const DIAGNOSTICS_MAX_CORE_REJECTIONS: u32 = 1_024;

const DATABASE_KIND: &str = "local_diagnostics";
const BUSY_TIMEOUT_MILLIS: u64 = 250;
const DIAGNOSTICS_STAGING_PREFIX: &str = ".volicord-diagnostics-staging-";
const DIAGNOSTICS_STAGING_ID_BYTES: usize = 16;
const DIAGNOSTICS_STAGING_CREATE_ATTEMPTS: usize = 8;
const SQLITE_SIDECAR_SUFFIXES: [&str; 3] = ["-journal", "-wal", "-shm"];

#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub enum DiagnosticsPublicationPhase {
    AfterStagingCreated,
    DuringSchemaInitialization,
    AfterStagingValidation,
    BeforePublication,
}

#[cfg(not(any(test, feature = "test-support")))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagnosticsPublicationPhase {
    AfterStagingCreated,
    DuringSchemaInitialization,
    AfterStagingValidation,
    BeforePublication,
}

#[cfg(any(test, feature = "test-support"))]
fn diagnostics_publication_hook(
    final_path: &Path,
    phase: DiagnosticsPublicationPhase,
) -> StoreResult<()> {
    diagnostics_publication_test_support::run_hook(final_path, phase)
}

#[cfg(not(any(test, feature = "test-support")))]
fn diagnostics_publication_hook(
    _final_path: &Path,
    _phase: DiagnosticsPublicationPhase,
) -> StoreResult<()> {
    Ok(())
}

/// Repository-owned deterministic coordination and fault support for
/// diagnostics-publication tests.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod diagnostics_publication_test_support {
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::{Arc, Barrier, Mutex, OnceLock},
    };

    use super::{DiagnosticsPublicationPhase, StoreError, StoreResult};

    #[derive(Clone)]
    struct PublicationHook {
        pause_at: Option<DiagnosticsPublicationPhase>,
        fail_at: Option<DiagnosticsPublicationPhase>,
        ready: Option<Arc<Barrier>>,
        resume: Option<Arc<Barrier>>,
    }

    /// Handle used by a test to observe and resume paused creators.
    pub struct DiagnosticsPublicationPause {
        ready: Arc<Barrier>,
        resume: Arc<Barrier>,
    }

    impl DiagnosticsPublicationPause {
        pub fn wait_until_all_creators_are_paused(&self) {
            self.ready.wait();
        }

        pub fn resume_all_creators(&self) {
            self.resume.wait();
        }
    }

    static HOOKS: OnceLock<Mutex<HashMap<PathBuf, PublicationHook>>> = OnceLock::new();

    fn hooks() -> &'static Mutex<HashMap<PathBuf, PublicationHook>> {
        HOOKS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    /// Pauses the requested number of creators at one deterministic phase.
    pub fn pause_creators(
        final_path: impl Into<PathBuf>,
        phase: DiagnosticsPublicationPhase,
        creator_count: usize,
    ) -> DiagnosticsPublicationPause {
        assert!(creator_count > 0, "at least one creator is required");
        let ready = Arc::new(Barrier::new(creator_count + 1));
        let resume = Arc::new(Barrier::new(creator_count + 1));
        let prior = hooks().lock().expect("hook lock").insert(
            final_path.into(),
            PublicationHook {
                pause_at: Some(phase),
                fail_at: None,
                ready: Some(Arc::clone(&ready)),
                resume: Some(Arc::clone(&resume)),
            },
        );
        assert!(prior.is_none(), "a diagnostics publication hook is active");
        DiagnosticsPublicationPause { ready, resume }
    }

    /// Fails one creator at a deterministic phase.
    pub fn fail_creator(final_path: impl Into<PathBuf>, phase: DiagnosticsPublicationPhase) {
        let prior = hooks().lock().expect("hook lock").insert(
            final_path.into(),
            PublicationHook {
                pause_at: None,
                fail_at: Some(phase),
                ready: None,
                resume: None,
            },
        );
        assert!(prior.is_none(), "a diagnostics publication hook is active");
    }

    /// Removes the hook for one exact diagnostics final path.
    pub fn clear(final_path: &Path) {
        hooks().lock().expect("hook lock").remove(final_path);
    }

    pub(super) fn run_hook(
        final_path: &Path,
        phase: DiagnosticsPublicationPhase,
    ) -> StoreResult<()> {
        let hook = hooks().lock().expect("hook lock").get(final_path).cloned();
        let Some(hook) = hook else {
            return Ok(());
        };
        if hook.pause_at == Some(phase) {
            hook.ready.expect("pause ready barrier").wait();
            hook.resume.expect("pause resume barrier").wait();
        }
        if hook.fail_at == Some(phase) {
            return Err(StoreError::Io(std::io::Error::other(format!(
                "injected diagnostics publication failure at {phase:?}"
            ))));
        }
        Ok(())
    }
}

const DIAGNOSTICS_SCHEMA_SQL: &str = r#"
CREATE TABLE diagnostics_manifest (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    contract_id TEXT NOT NULL,
    canonical_schema_digest TEXT NOT NULL
);

CREATE TABLE diagnostic_sessions (
    session_id TEXT PRIMARY KEY NOT NULL,
    connection_id TEXT,
    project_id TEXT,
    transport TEXT NOT NULL CHECK (transport IN ('mcp_stdio', 'guard_hook', 'cli_inbox')),
    host_kind TEXT CHECK (host_kind IS NULL OR host_kind = 'codex'),
    package_version TEXT NOT NULL,
    build_id TEXT NOT NULL,
    started_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE diagnostic_events (
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
        user_channel_kind IS NULL OR user_channel_kind IN ('prompt_capture', 'cli_inbox')
    ),
    fallback_kind TEXT CHECK (
        fallback_kind IS NULL OR fallback_kind = 'cli_inbox'
    ),
    product_file_write_count INTEGER NOT NULL CHECK (product_file_write_count >= 0),
    authoritative_refresh_failure INTEGER NOT NULL CHECK (authoritative_refresh_failure IN (0, 1)),
    outcome TEXT NOT NULL CHECK (
        outcome IN ('success', 'rejected', 'validation_failure', 'tool_error', 'transport_error', 'unavailable')
    ),
    occurred_at TEXT NOT NULL
);

CREATE INDEX idx_diagnostic_sessions_updated
    ON diagnostic_sessions(updated_at DESC, session_id DESC);
CREATE INDEX idx_diagnostic_events_session
    ON diagnostic_events(session_id, event_id);
CREATE INDEX idx_diagnostic_events_tool
    ON diagnostic_events(session_id, tool_name, event_id);

CREATE TABLE core_rejection_diagnostics (
    project_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    method_name TEXT NOT NULL CHECK (method_name = 'volicord.prepare_write'),
    reason TEXT NOT NULL CHECK (reason = 'current_change_unit_required'),
    occurred_at TEXT NOT NULL,
    PRIMARY KEY (project_id, task_id, method_name, reason)
);

CREATE INDEX idx_core_rejection_diagnostics_time
    ON core_rejection_diagnostics(occurred_at DESC, project_id, task_id);

CREATE TABLE workflow_metric_events (
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
            'tools_list_serialized_bytes',
            'pre_tool_decision',
            'observation_assessment',
            'confirmed_out_of_scope_write',
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
        integration_profile IS NULL OR integration_profile = 'record'
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

CREATE INDEX idx_workflow_metric_events_session
    ON workflow_metric_events(session_id, workflow_metric_event_id);
CREATE INDEX idx_workflow_metric_events_aggregate
    ON workflow_metric_events(
        project_id, metric_kind, method_name, integration_profile,
        decision, observation_confidence, outcome
);
"#;

static DIAGNOSTICS_SCHEMA_METADATA: OnceLock<Result<DiagnosticsSchemaMetadata, String>> =
    OnceLock::new();
static DIAGNOSTICS_STORAGE_MANIFEST: OnceLock<Result<DiagnosticsStorageManifest, String>> =
    OnceLock::new();

/// Exact semantic manifest for the local, non-authority diagnostics database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticsStorageManifest {
    pub contract_id: String,
    pub canonical_schema_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DiagnosticsSchemaMetadata {
    relations: Vec<DiagnosticsRelation>,
    columns: Vec<DiagnosticsColumn>,
    indexes: Vec<DiagnosticsIndex>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct DiagnosticsRelation {
    object_type: String,
    name: String,
    canonical_sql: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct DiagnosticsColumn {
    relation: String,
    ordinal: u32,
    name: String,
    declared_type: String,
    not_null: bool,
    default_value: Option<String>,
    primary_key_ordinal: u32,
    hidden: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct DiagnosticsIndex {
    table: String,
    name: String,
    unique: bool,
    partial: bool,
    canonical_sql: String,
}

/// Returns the one exact manifest derived from the canonical diagnostics SQL.
pub fn current_diagnostics_storage_manifest() -> StoreResult<&'static DiagnosticsStorageManifest> {
    DIAGNOSTICS_STORAGE_MANIFEST
        .get_or_init(|| {
            let metadata = DIAGNOSTICS_SCHEMA_METADATA
                .get_or_init(build_diagnostics_schema_metadata)
                .as_ref()
                .map_err(Clone::clone)?;
            let bytes = canonical_json_bytes(metadata).map_err(|error| error.to_string())?;
            let digest = Sha256::digest(bytes);
            Ok(DiagnosticsStorageManifest {
                contract_id: DIAGNOSTICS_CONTRACT_ID.to_owned(),
                canonical_schema_digest: format!("sha256:{digest:x}"),
            })
        })
        .as_ref()
        .map_err(|detail| {
            StoreError::schema_invariant(
                DATABASE_KIND,
                format!("canonical diagnostics manifest is unavailable: {detail}"),
            )
        })
}

fn canonical_diagnostics_schema_metadata() -> StoreResult<&'static DiagnosticsSchemaMetadata> {
    DIAGNOSTICS_SCHEMA_METADATA
        .get_or_init(build_diagnostics_schema_metadata)
        .as_ref()
        .map_err(|detail| {
            StoreError::schema_invariant(
                DATABASE_KIND,
                format!("canonical diagnostics schema metadata is unavailable: {detail}"),
            )
        })
}

fn build_diagnostics_schema_metadata() -> Result<DiagnosticsSchemaMetadata, String> {
    let conn = Connection::open_in_memory().map_err(|error| error.to_string())?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|error| error.to_string())?;
    conn.execute_batch(DIAGNOSTICS_SCHEMA_SQL)
        .map_err(|error| error.to_string())?;
    extract_diagnostics_schema_metadata(&conn)
}

fn extract_diagnostics_schema_metadata(
    conn: &Connection,
) -> Result<DiagnosticsSchemaMetadata, String> {
    let mut relations = Vec::new();
    let mut relation_statement = conn
        .prepare(
            "SELECT type, name, sql
               FROM sqlite_schema
              WHERE type IN ('table', 'view', 'trigger')
                AND name NOT LIKE 'sqlite_%'
              ORDER BY type, name",
        )
        .map_err(|error| error.to_string())?;
    let relation_rows = relation_statement
        .query_map([], |row| {
            Ok(DiagnosticsRelation {
                object_type: row.get(0)?,
                name: row.get(1)?,
                canonical_sql: row.get(2)?,
            })
        })
        .map_err(|error| error.to_string())?;
    for row in relation_rows {
        relations.push(row.map_err(|error| error.to_string())?);
    }

    let mut columns = Vec::new();
    for relation in relations
        .iter()
        .filter(|relation| relation.object_type != "trigger")
    {
        columns.extend(read_diagnostics_columns(conn, &relation.name)?);
    }
    let mut indexes = read_diagnostics_indexes(conn)?;
    relations.sort();
    columns.sort();
    indexes.sort();
    Ok(DiagnosticsSchemaMetadata {
        relations,
        columns,
        indexes,
    })
}

fn read_diagnostics_columns(
    conn: &Connection,
    relation: &str,
) -> Result<Vec<DiagnosticsColumn>, String> {
    let sql = format!("PRAGMA table_xinfo({})", quote_sqlite_identifier(relation));
    let mut statement = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut columns = Vec::new();
    for row in rows {
        let (ordinal, name, declared_type, not_null, default_value, primary_key, hidden) =
            row.map_err(|error| error.to_string())?;
        columns.push(DiagnosticsColumn {
            relation: relation.to_owned(),
            ordinal: u32::try_from(ordinal).map_err(|error| error.to_string())?,
            name,
            declared_type,
            not_null: not_null != 0,
            default_value,
            primary_key_ordinal: u32::try_from(primary_key).map_err(|error| error.to_string())?,
            hidden: u32::try_from(hidden).map_err(|error| error.to_string())?,
        });
    }
    Ok(columns)
}

fn read_diagnostics_indexes(conn: &Connection) -> Result<Vec<DiagnosticsIndex>, String> {
    let mut statement = conn
        .prepare(
            "SELECT name, tbl_name, sql
               FROM sqlite_schema
              WHERE type = 'index'
                AND name NOT LIKE 'sqlite_%'
              ORDER BY name",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut indexes = Vec::new();
    for row in rows {
        let (name, table, canonical_sql) = row.map_err(|error| error.to_string())?;
        let (unique, partial) = diagnostics_index_flags(conn, &table, &name)?;
        indexes.push(DiagnosticsIndex {
            table,
            name,
            unique,
            partial,
            canonical_sql,
        });
    }
    Ok(indexes)
}

fn diagnostics_index_flags(
    conn: &Connection,
    table: &str,
    index: &str,
) -> Result<(bool, bool), String> {
    let sql = format!("PRAGMA index_list({})", quote_sqlite_identifier(table));
    let mut statement = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let mut rows = statement.query([]).map_err(|error| error.to_string())?;
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        if row.get::<_, String>(1).map_err(|error| error.to_string())? == index {
            return Ok((
                row.get::<_, i64>(2).map_err(|error| error.to_string())? != 0,
                row.get::<_, i64>(4).map_err(|error| error.to_string())? != 0,
            ));
        }
    }
    Err(format!("canonical diagnostics index {index} is missing"))
}

fn quote_sqlite_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

/// Returns the diagnostics database path for a Runtime Home.
pub fn diagnostics_db_path(runtime_home: impl AsRef<Path>) -> PathBuf {
    runtime_home.as_ref().join(DIAGNOSTICS_DB_FILE)
}

/// Controlled transport category for one diagnostic session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTransport {
    McpStdio,
    GuardHook,
    CliInbox,
}

impl DiagnosticTransport {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::McpStdio => "mcp_stdio",
            Self::GuardHook => "guard_hook",
            Self::CliInbox => "cli_inbox",
        }
    }
}

/// Controlled host category retained without host configuration content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticHostKind {
    Codex,
}

impl DiagnosticHostKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
        }
    }

    /// Maps a stored Agent Connection host kind to the bounded diagnostic set.
    pub fn from_connection_host_kind(value: &str) -> Option<Self> {
        (value == "codex").then_some(Self::Codex)
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
    PromptCapture,
    CliInbox,
}

impl DiagnosticUserChannelKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::PromptCapture => "prompt_capture",
            Self::CliInbox => "cli_inbox",
        }
    }
}

/// Controlled pending-user-action fallback category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticFallbackKind {
    CliInbox,
}

impl DiagnosticFallbackKind {
    const fn as_str(self) -> &'static str {
        match self {
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
    ToolsListSerializedBytes,
    PreToolDecision,
    ObservationAssessment,
    ConfirmedOutOfScopeWrite,
    ConfirmedStructuredWriteDeny,
    SensitiveApprovalMissingBlock,
    CompletionClaimSuppressed,
}

impl WorkflowMetricKind {
    /// All supported workflow metric kinds in stable contract order.
    pub const ALL: [Self; 16] = [
        Self::TaskDurationMicros,
        Self::FirstProductWriteDurationMicros,
        Self::McpMethodCall,
        Self::StatusReread,
        Self::AuthorityRefresh,
        Self::WriteTicketIssued,
        Self::WriteTicketReused,
        Self::WriteTicketReissued,
        Self::UserRoundtrip,
        Self::ToolsListSerializedBytes,
        Self::PreToolDecision,
        Self::ObservationAssessment,
        Self::ConfirmedOutOfScopeWrite,
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
            Self::ToolsListSerializedBytes => "tools_list_serialized_bytes",
            Self::PreToolDecision => "pre_tool_decision",
            Self::ObservationAssessment => "observation_assessment",
            Self::ConfirmedOutOfScopeWrite => "confirmed_out_of_scope_write",
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

/// Closed reason set for bounded Core structural-rejection observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreRejectionReason {
    /// Write preparation resolved a Task without a current Change Unit.
    CurrentChangeUnitRequired,
}

impl CoreRejectionReason {
    /// Returns the stable machine-readable reason.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CurrentChangeUnitRequired => "current_change_unit_required",
        }
    }
}

/// Content-free bounded observation of one Core structural rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreRejectionDiagnostic<'a> {
    pub project_id: &'a str,
    pub task_id: &'a str,
    pub method_name: MethodName,
    pub reason: CoreRejectionReason,
    pub occurred_at: &'a UtcTimestamp,
}

/// Stored projection of one bounded Core structural-rejection observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoreRejectionDiagnosticRecord {
    pub project_id: String,
    pub task_id: String,
    pub method_name: String,
    pub reason: String,
    pub occurred_at: String,
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
    context: &RuntimeHomeMutationContext<'_>,
    input: DiagnosticSessionStart<'_>,
) -> StoreResult<()> {
    validate_diagnostic_session_start_shape(&input)?;

    let mut conn = open_diagnostics_database(context)?;
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
    match input.transport {
        DiagnosticTransport::McpStdio | DiagnosticTransport::GuardHook
            if input.connection_id.is_none()
                || input.host_kind != Some(DiagnosticHostKind::Codex) =>
        {
            return Err(StoreError::InvalidInput {
                detail: "managed diagnostics require a Codex Agent Connection".to_owned(),
            });
        }
        DiagnosticTransport::CliInbox if input.host_kind.is_some() => {
            return Err(StoreError::InvalidInput {
                detail: "CLI inbox diagnostics are not host-bound".to_owned(),
            });
        }
        _ => {}
    }
    if input.transport == DiagnosticTransport::GuardHook {
        validate_project_agent_session_id(input.session_id).map_err(|error| {
            StoreError::InvalidInput {
                detail: error.to_string(),
            }
        })?;
    }
    Ok(())
}

fn validate_managed_diagnostic_session_binding(
    conn: &Connection,
    input: &DiagnosticSessionStart<'_>,
) -> StoreResult<()> {
    if !matches!(
        input.transport,
        DiagnosticTransport::McpStdio | DiagnosticTransport::GuardHook
    ) {
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
    context: &RuntimeHomeMutationContext<'_>,
    input: DiagnosticEvent<'_>,
) -> StoreResult<()> {
    validate_identifier("session_id", input.session_id)?;
    validate_optional_tool_name(input.tool_name)?;
    let latency_micros = sqlite_integer(input.latency_micros, "latency_micros")?;
    let request_bytes = sqlite_integer(input.request_bytes, "request_bytes")?;
    let response_bytes = sqlite_integer(input.response_bytes, "response_bytes")?;
    let product_file_write_count =
        sqlite_integer(input.product_file_write_count, "product_file_write_count")?;

    let mut conn = open_diagnostics_database(context)?;
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

/// Records one bounded Core structural rejection outside authority storage.
pub fn record_core_rejection_diagnostic(
    context: &RuntimeHomeMutationContext<'_>,
    input: CoreRejectionDiagnostic<'_>,
) -> StoreResult<()> {
    validate_identifier("project_id", input.project_id)?;
    validate_identifier("task_id", input.task_id)?;
    if input.method_name != MethodName::PrepareWrite
        || input.reason != CoreRejectionReason::CurrentChangeUnitRequired
    {
        return Err(StoreError::InvalidInput {
            detail: "diagnostics Core rejection method and reason are not a supported pair"
                .to_owned(),
        });
    }
    input
        .occurred_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::InvalidInput {
            detail: "diagnostics occurred_at must be canonical RFC 3339 UTC".to_owned(),
        })?;

    let mut conn = open_diagnostics_database(context)?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute(
        "INSERT INTO core_rejection_diagnostics (
             project_id, task_id, method_name, reason, occurred_at
         ) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(project_id, task_id, method_name, reason) DO UPDATE SET
             occurred_at = excluded.occurred_at",
        params![
            input.project_id,
            input.task_id,
            input.method_name.as_str(),
            input.reason.as_str(),
            input.occurred_at.to_canonical_string(),
        ],
    )?;
    prune_diagnostics(&tx)?;
    tx.commit()?;
    Ok(())
}

/// Reads the bounded Core rejection observations without creating storage.
pub fn read_core_rejection_diagnostics(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<Vec<CoreRejectionDiagnosticRecord>> {
    let path = diagnostics_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_diagnostics_database_read_only(&path)?;
    let mut statement = conn.prepare(
        "SELECT project_id, task_id, method_name, reason, occurred_at
           FROM core_rejection_diagnostics
          ORDER BY project_id, task_id, method_name, reason",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(CoreRejectionDiagnosticRecord {
            project_id: row.get(0)?,
            task_id: row.get(1)?,
            method_name: row.get(2)?,
            reason: row.get(3)?,
            occurred_at: row.get(4)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Records one privacy-bounded workflow metric and enforces shared event retention.
pub fn record_workflow_metric_event(
    context: &RuntimeHomeMutationContext<'_>,
    input: &WorkflowMetricEvent,
) -> StoreResult<()> {
    validate_workflow_metric_event(input)?;
    let value = sqlite_integer(input.value, "workflow metric value")?;

    let mut conn = open_diagnostics_database(context)?;
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

fn open_diagnostics_database(context: &RuntimeHomeMutationContext<'_>) -> StoreResult<Connection> {
    let path = diagnostics_db_path(context.runtime_home().as_path());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    match fs::symlink_metadata(&path) {
        Ok(_) => return open_existing_diagnostics_database(&path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(StoreError::Io(error)),
    }

    let parent = path.parent().ok_or_else(|| StoreError::InvalidInput {
        detail: "diagnostics database path has no parent directory".to_owned(),
    })?;
    let mut staging = DiagnosticsStagingDatabase::create(parent)?;
    diagnostics_publication_hook(&path, DiagnosticsPublicationPhase::AfterStagingCreated)?;
    prepare_diagnostics_staging_database(staging.path(), &path)?;
    diagnostics_publication_hook(&path, DiagnosticsPublicationPhase::BeforePublication)?;

    match publish_file_no_replace(staging.path(), &path) {
        Ok(NoReplaceFilePublicationOutcome::Published { .. }) => {
            staging.cleanup()?;
            open_existing_diagnostics_database(&path)
        }
        Ok(NoReplaceFilePublicationOutcome::DestinationExists) => {
            staging.cleanup()?;
            open_existing_diagnostics_database(&path)
        }
        Err(error) => {
            if error.effect == NoReplaceFilePublicationEffect::Unknown {
                staging.preserve();
            }
            Err(StoreError::Io(io::Error::new(
                error.io_error().kind(),
                error,
            )))
        }
    }
}

fn open_existing_diagnostics_database(path: &Path) -> StoreResult<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MILLIS))?;
    enable_foreign_keys(&conn)?;
    validate_diagnostics_schema(&conn)?;
    harden_diagnostics_permissions(path)?;
    Ok(conn)
}

fn prepare_diagnostics_staging_database(staging_path: &Path, final_path: &Path) -> StoreResult<()> {
    let mut conn = Connection::open_with_flags(
        staging_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MILLIS))?;
    enable_foreign_keys(&conn)?;
    conn.pragma_update(None, "journal_mode", "DELETE")?;
    initialize_diagnostics_database(&mut conn, final_path)?;
    validate_diagnostics_schema(&conn)?;
    conn.close()
        .map_err(|(_, error)| StoreError::Sqlite(error))?;

    ensure_diagnostics_staging_has_no_sidecars(staging_path)?;
    harden_diagnostics_permissions(staging_path)?;
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(staging_path)?
        .sync_all()?;
    diagnostics_publication_hook(
        final_path,
        DiagnosticsPublicationPhase::AfterStagingValidation,
    )?;
    Ok(())
}

fn initialize_diagnostics_database(conn: &mut Connection, final_path: &Path) -> StoreResult<()> {
    let manifest = current_diagnostics_storage_manifest()?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute_batch(DIAGNOSTICS_SCHEMA_SQL)?;
    diagnostics_publication_hook(
        final_path,
        DiagnosticsPublicationPhase::DuringSchemaInitialization,
    )?;
    tx.execute(
        "INSERT INTO diagnostics_manifest (
             singleton_id, contract_id, canonical_schema_digest
         ) VALUES (1, ?1, ?2)",
        params![&manifest.contract_id, &manifest.canonical_schema_digest],
    )?;
    validate_diagnostics_schema(&tx)?;
    tx.commit()?;
    Ok(())
}

struct DiagnosticsStagingDatabase {
    path: PathBuf,
    cleanup_on_drop: bool,
}

impl DiagnosticsStagingDatabase {
    fn create(parent: &Path) -> StoreResult<Self> {
        for _ in 0..DIAGNOSTICS_STAGING_CREATE_ATTEMPTS {
            let identity = diagnostics_staging_identity()?;
            let path = parent.join(format!("{DIAGNOSTICS_STAGING_PREFIX}{identity}"));
            match OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(file) => {
                    drop(file);
                    return Ok(Self {
                        path,
                        cleanup_on_drop: true,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(StoreError::Io(error)),
            }
        }
        Err(StoreError::Io(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a unique diagnostics staging file",
        )))
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn cleanup(&mut self) -> StoreResult<()> {
        remove_owned_diagnostics_staging_files(&self.path)?;
        self.cleanup_on_drop = false;
        Ok(())
    }

    fn preserve(&mut self) {
        self.cleanup_on_drop = false;
    }
}

impl Drop for DiagnosticsStagingDatabase {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            let _ = remove_owned_diagnostics_staging_files(&self.path);
        }
    }
}

fn diagnostics_staging_identity() -> StoreResult<String> {
    use std::fmt::Write as _;

    let mut bytes = [0_u8; DIAGNOSTICS_STAGING_ID_BYTES];
    getrandom::fill(&mut bytes)
        .map_err(|error| StoreError::Io(io::Error::other(error.to_string())))?;
    let mut identity = String::with_capacity(DIAGNOSTICS_STAGING_ID_BYTES * 2);
    for byte in bytes {
        write!(&mut identity, "{byte:02x}")
            .expect("writing hexadecimal bytes to String cannot fail");
    }
    Ok(identity)
}

fn diagnostics_sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut sidecar = path.as_os_str().to_os_string();
    sidecar.push(suffix);
    PathBuf::from(sidecar)
}

fn ensure_diagnostics_staging_has_no_sidecars(path: &Path) -> StoreResult<()> {
    for suffix in SQLITE_SIDECAR_SUFFIXES {
        let sidecar = diagnostics_sqlite_sidecar_path(path, suffix);
        match fs::symlink_metadata(&sidecar) {
            Ok(_) => {
                return Err(StoreError::schema_invariant(
                    DATABASE_KIND,
                    format!("diagnostics staging database still requires SQLite sidecar {suffix}"),
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(StoreError::Io(error)),
        }
    }
    Ok(())
}

fn remove_owned_diagnostics_staging_files(path: &Path) -> StoreResult<()> {
    for owned_path in SQLITE_SIDECAR_SUFFIXES
        .into_iter()
        .map(|suffix| diagnostics_sqlite_sidecar_path(path, suffix))
        .chain(std::iter::once(path.to_path_buf()))
    {
        match fs::remove_file(owned_path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(StoreError::Io(error)),
        }
    }
    Ok(())
}

fn open_diagnostics_database_read_only(path: &Path) -> StoreResult<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MILLIS))?;
    enable_foreign_keys(&conn)?;
    conn.pragma_update(None, "query_only", "ON")?;
    validate_diagnostics_schema(&conn)?;
    Ok(conn)
}

fn validate_diagnostics_schema(conn: &Connection) -> StoreResult<()> {
    validate_diagnostics_manifest(conn)?;
    let expected = canonical_diagnostics_schema_metadata()?;
    let actual = extract_diagnostics_schema_metadata(conn).map_err(|detail| {
        StoreError::schema_invariant(
            DATABASE_KIND,
            format!("diagnostics schema inventory could not be read: {detail}"),
        )
    })?;
    if actual != *expected {
        return Err(StoreError::schema_invariant(
            DATABASE_KIND,
            diagnostics_schema_difference(expected, &actual),
        ));
    }
    Ok(())
}

fn validate_diagnostics_manifest(conn: &Connection) -> StoreResult<()> {
    let current = current_diagnostics_storage_manifest()?;
    let carrier_exists = conn
        .query_row(
            "SELECT 1
               FROM sqlite_schema
              WHERE type = 'table' AND name = 'diagnostics_manifest'",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !carrier_exists {
        return Err(StoreError::unsupported_storage_profile(
            DATABASE_KIND,
            "missing_diagnostics_manifest",
            DIAGNOSTICS_CONTRACT_ID,
        ));
    }

    let mut statement = conn.prepare(
        "SELECT singleton_id, contract_id, canonical_schema_digest
           FROM diagnostics_manifest
          ORDER BY singleton_id",
    )?;
    let mut rows = statement.query([])?;
    let first = rows.next()?.ok_or_else(|| {
        StoreError::unsupported_storage_profile(
            DATABASE_KIND,
            "missing_diagnostics_manifest_row",
            DIAGNOSTICS_CONTRACT_ID,
        )
    })?;
    let singleton_id = first.get::<_, i64>(0)?;
    let contract_id = first.get::<_, String>(1)?;
    let schema_digest = first.get::<_, String>(2)?;
    if rows.next()?.is_some() {
        return Err(StoreError::schema_invariant(
            DATABASE_KIND,
            "diagnostics manifest contains more than one row",
        ));
    }
    if singleton_id != 1 {
        return Err(StoreError::schema_invariant(
            DATABASE_KIND,
            "diagnostics manifest singleton identity is invalid",
        ));
    }
    if contract_id != current.contract_id {
        return Err(StoreError::unsupported_storage_profile(
            DATABASE_KIND,
            contract_id,
            DIAGNOSTICS_CONTRACT_ID,
        ));
    }
    if schema_digest != current.canonical_schema_digest {
        return Err(StoreError::schema_invariant(
            DATABASE_KIND,
            "diagnostics manifest schema digest does not match canonical SQL",
        ));
    }
    Ok(())
}

fn diagnostics_schema_difference(
    expected: &DiagnosticsSchemaMetadata,
    actual: &DiagnosticsSchemaMetadata,
) -> String {
    for relation in &actual.relations {
        if !expected.relations.iter().any(|expected_relation| {
            expected_relation.object_type == relation.object_type
                && expected_relation.name == relation.name
        }) {
            return format!(
                "unexpected diagnostics SQLite {} {}",
                relation.object_type, relation.name
            );
        }
    }
    for relation in &expected.relations {
        let Some(actual_relation) = actual.relations.iter().find(|actual_relation| {
            actual_relation.object_type == relation.object_type
                && actual_relation.name == relation.name
        }) else {
            return format!(
                "missing canonical diagnostics SQLite {} {}",
                relation.object_type, relation.name
            );
        };
        if actual_relation != relation {
            return format!(
                "diagnostics SQLite {} {} definition does not match canonical SQL",
                relation.object_type, relation.name
            );
        }
    }
    if let Some(column) = actual
        .columns
        .iter()
        .find(|column| !expected.columns.contains(column))
    {
        return format!(
            "unexpected or changed diagnostics column {}.{}",
            column.relation, column.name
        );
    }
    if let Some(column) = expected
        .columns
        .iter()
        .find(|column| !actual.columns.contains(column))
    {
        return format!(
            "missing canonical diagnostics column {}.{}",
            column.relation, column.name
        );
    }
    if let Some(index) = actual
        .indexes
        .iter()
        .find(|index| !expected.indexes.contains(index))
    {
        return format!("unexpected or changed diagnostics index {}", index.name);
    }
    if let Some(index) = expected
        .indexes
        .iter()
        .find(|index| !actual.indexes.contains(index))
    {
        return format!("missing canonical diagnostics index {}", index.name);
    }
    "diagnostics SQLite schema does not match canonical metadata".to_owned()
}

#[cfg(test)]
fn table_columns(conn: &Connection, table: &str) -> StoreResult<Vec<String>> {
    let mut statement = conn.prepare(&format!(
        "PRAGMA table_info({})",
        quote_sqlite_identifier(table)
    ))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn prune_diagnostics(conn: &Connection) -> rusqlite::Result<()> {
    prune_core_rejection_diagnostics(conn)?;
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

fn prune_core_rejection_diagnostics(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM core_rejection_diagnostics
          WHERE julianday(occurred_at) < julianday('now', ?1)",
        [format!("-{} days", DIAGNOSTICS_RETENTION_DAYS)],
    )?;
    conn.execute(
        "DELETE FROM core_rejection_diagnostics
          WHERE rowid NOT IN (
              SELECT rowid
                FROM core_rejection_diagnostics
               ORDER BY julianday(occurred_at) DESC,
                        occurred_at DESC,
                        project_id DESC,
                        task_id DESC
               LIMIT ?1
          )",
        [DIAGNOSTICS_MAX_CORE_REJECTIONS],
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
    use crate::mutation::TestRuntimeHomeAdmission;
    use rusqlite::Connection;
    use std::thread;
    use volicord_test_support::TempRuntimeHome;

    fn managed_session_id(native_session_id: &str) -> String {
        format!("mcp_runtime_{native_session_id}")
    }

    fn start(session_id: &str) -> DiagnosticSessionStart<'_> {
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
            user_channel_kind: Some(DiagnosticUserChannelKind::CliInbox),
            fallback_kind: None,
            product_file_write_count: 1,
            authoritative_refresh_failure: false,
            outcome: DiagnosticOutcome::Success,
        }
    }

    fn diagnostics_staging_entries(runtime_home: &Path) -> Vec<PathBuf> {
        let mut entries = fs::read_dir(runtime_home)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(DIAGNOSTICS_STAGING_PREFIX))
            })
            .collect::<Vec<_>>();
        entries.sort();
        entries
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
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let session_id = managed_session_id("session_test");
        start_diagnostic_session(&context, start(&session_id)).expect("start");
        record_diagnostic_event(
            &context,
            DiagnosticEvent {
                validation_failure: true,
                core_reached: false,
                core_committed: false,
                outcome: DiagnosticOutcome::ValidationFailure,
                ..event(&session_id, "volicord.record_run")
            },
        )
        .expect("validation event");
        record_diagnostic_event(&context, event(&session_id, "volicord.record_run"))
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
        assert_eq!(aggregate.user_channel_counts["cli_inbox"], 2);

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
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let session_id = managed_session_id("session_test");
        let mut input = start(&session_id);
        input.connection_id = Some("/home/user/private-file.txt");
        assert!(start_diagnostic_session(&context, input).is_err());

        let mut input = start(&session_id);
        input.build_id = "0.2.0;target=/private/build/location";
        assert!(start_diagnostic_session(&context, input).is_err());

        start_diagnostic_session(&context, start(&session_id)).expect("start");
        let error =
            record_diagnostic_event(&context, event(&session_id, "prompt text with secret"))
                .expect_err("content must not fit tool field");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
    }

    #[test]
    fn managed_session_diagnostics_bind_native_identity_to_the_connection() {
        let fixture = TempRuntimeHome::new("diagnostics-managed-binding").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let session_id = managed_session_id("native-session");
        start_diagnostic_session(&context, start(&session_id)).expect("initial start");
        record_diagnostic_event(&context, event(&session_id, "volicord.status"))
            .expect("initial event");

        start_diagnostic_session(&context, start(&session_id))
            .expect("an exact managed binding is idempotent");
        let exact = read_diagnostic_session(fixture.path(), Some(&session_id))
            .expect("read exact binding")
            .expect("managed diagnostics session");
        assert_eq!(exact.connection_id.as_deref(), Some("connection_test"));
        assert_eq!(exact.host_kind.as_deref(), Some("codex"));
        assert_eq!(exact.totals.event_count, 1);

        let mut cross_connection = start(&session_id);
        cross_connection.connection_id = Some("connection_other");
        let error = start_diagnostic_session(&context, cross_connection)
            .expect_err("cross-connection managed reuse must fail");
        assert!(matches!(error, StoreError::Conflict { .. }));

        let unchanged = read_diagnostic_session(fixture.path(), Some(&session_id))
            .expect("read unchanged binding")
            .expect("managed diagnostics session");
        assert_eq!(unchanged.connection_id.as_deref(), Some("connection_test"));
        assert_eq!(unchanged.host_kind.as_deref(), Some("codex"));
        assert_eq!(unchanged.totals.event_count, 1);
    }

    #[test]
    fn managed_transport_requires_bound_valid_native_session_correlation() {
        let fixture = TempRuntimeHome::new("diagnostics-managed-native-session").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let session_id = managed_session_id("native-session");

        let mut unbound = start(&session_id);
        unbound.host_kind = None;
        assert!(matches!(
            start_diagnostic_session(&context, unbound),
            Err(StoreError::InvalidInput { .. })
        ));

        let mut cli_inbox = start(&session_id);
        cli_inbox.transport = DiagnosticTransport::CliInbox;
        cli_inbox.host_kind = None;
        start_diagnostic_session(&context, cli_inbox)
            .expect("CLI inbox sessions do not require host-native correlation");

        let malformed = start("not a native id");
        assert!(matches!(
            start_diagnostic_session(&context, malformed),
            Err(StoreError::InvalidInput { .. })
        ));
    }

    #[test]
    fn per_session_event_retention_is_enforced() {
        let fixture = TempRuntimeHome::new("diagnostics-event-retention").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let session_id = managed_session_id("session_test");
        start_diagnostic_session(&context, start(&session_id)).expect("start");
        for _ in 0..(DIAGNOSTICS_MAX_EVENTS_PER_SESSION + 3) {
            record_diagnostic_event(&context, event(&session_id, "volicord.status"))
                .expect("event");
        }
        let aggregate = read_diagnostic_session(fixture.path(), Some(&session_id))
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
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let mut session_ids = (0..(DIAGNOSTICS_MAX_SESSIONS + 3))
            .map(|index| managed_session_id(&format!("session_{index:03}")))
            .collect::<Vec<_>>();
        session_ids.sort();
        for session_id in &session_ids {
            start_diagnostic_session(&context, start(session_id)).expect("session");
        }
        let conn = Connection::open(diagnostics_db_path(fixture.path())).expect("diagnostics db");
        let count = conn
            .query_row("SELECT COUNT(*) FROM diagnostic_sessions", [], |row| {
                row.get::<_, u64>(0)
            })
            .expect("session count");
        assert_eq!(count, u64::from(DIAGNOSTICS_MAX_SESSIONS));
        assert!(
            read_diagnostic_session(fixture.path(), Some(&session_ids[0]))
                .expect("oldest session")
                .is_none()
        );
        assert!(read_diagnostic_session(
            fixture.path(),
            Some(session_ids.last().expect("latest session id"))
        )
        .expect("latest session")
        .is_some());
    }

    #[test]
    fn age_retention_compares_iso_timestamps_as_time_values() {
        let fixture = TempRuntimeHome::new("diagnostics-age-retention").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let expired_session_id = managed_session_id("session_expired");
        let recent_session_id = managed_session_id("session_recent");
        let trigger_session_id = managed_session_id("session_trigger");
        start_diagnostic_session(&context, start(&expired_session_id)).expect("expired");
        start_diagnostic_session(&context, start(&recent_session_id)).expect("recent");
        let conn = Connection::open(diagnostics_db_path(fixture.path())).expect("diagnostics db");
        conn.execute(
            "UPDATE diagnostic_sessions
                SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-7 days', '-1 hour')
              WHERE session_id = ?1",
            [&expired_session_id],
        )
        .expect("backdate expired session");
        conn.execute(
            "UPDATE diagnostic_sessions
                SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-6 days')
              WHERE session_id = ?1",
            [&recent_session_id],
        )
        .expect("backdate recent session");
        drop(conn);

        start_diagnostic_session(&context, start(&trigger_session_id)).expect("trigger prune");

        assert!(
            read_diagnostic_session(fixture.path(), Some(&expired_session_id))
                .expect("expired read")
                .is_none()
        );
        assert!(
            read_diagnostic_session(fixture.path(), Some(&recent_session_id))
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
                "tools_list_serialized_bytes",
                "pre_tool_decision",
                "observation_assessment",
                "confirmed_out_of_scope_write",
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

        invalid = metric("session_test", WorkflowMetricKind::StatusReread, 1);
        invalid.observation_confidence = Some(ObservationConfidence::Unknown);
        assert!(validate_workflow_metric_event(&invalid).is_err());
    }

    #[test]
    fn workflow_metrics_are_exposed_only_as_bounded_aggregate_rows() {
        let fixture = TempRuntimeHome::new("workflow-metric-aggregates").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let session_id = managed_session_id("session_metrics");
        start_diagnostic_session(&context, start(&session_id)).expect("start");

        let mut method_call = metric(&session_id, WorkflowMetricKind::McpMethodCall, 1);
        method_call.method_name = Some(MethodName::Status);
        method_call.integration_profile = Some(IntegrationProfile::Record);
        method_call.outcome = Some(WorkflowMetricOutcome::Success);
        record_workflow_metric_event(&context, &method_call).expect("method call one");
        record_workflow_metric_event(&context, &method_call).expect("method call two");

        let mut pre_tool = metric(&session_id, WorkflowMetricKind::PreToolDecision, 1);
        pre_tool.integration_profile = Some(IntegrationProfile::Record);
        pre_tool.decision = Some(WorkflowMetricDecision::Allow);
        pre_tool.observation_confidence = Some(ObservationConfidence::Confirmed);
        record_workflow_metric_event(&context, &pre_tool).expect("pre-tool decision");

        let mut observation = metric(&session_id, WorkflowMetricKind::ObservationAssessment, 3);
        observation.integration_profile = Some(IntegrationProfile::Record);
        observation.observation_confidence = Some(ObservationConfidence::Heuristic);
        observation.outcome = Some(WorkflowMetricOutcome::ReadOnly);
        record_workflow_metric_event(&context, &observation).expect("observation");

        let rows = read_workflow_metric_aggregates(fixture.path(), "project_test")
            .expect("aggregate rows");
        assert_eq!(rows.len(), 3);
        let calls = rows
            .iter()
            .find(|row| row.metric_kind == "mcp_method_call")
            .expect("method aggregate");
        assert_eq!(calls.method_name.as_deref(), Some("volicord.status"));
        assert_eq!(calls.host_kind.as_deref(), Some("codex"));
        assert_eq!(calls.integration_profile.as_deref(), Some("record"));
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
    fn workflow_metrics_share_the_per_session_event_retention_limit() {
        let fixture = TempRuntimeHome::new("workflow-metric-retention").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let session_id = managed_session_id("session_metrics");
        start_diagnostic_session(&context, start(&session_id)).expect("start");
        record_diagnostic_event(&context, event(&session_id, "volicord.status"))
            .expect("diagnostic event");
        let status_reread = metric(&session_id, WorkflowMetricKind::StatusReread, 1);
        for _ in 0..DIAGNOSTICS_MAX_EVENTS_PER_SESSION {
            record_workflow_metric_event(&context, &status_reread).expect("workflow event");
        }

        let conn = Connection::open(diagnostics_db_path(fixture.path())).expect("diagnostics db");
        let diagnostic_count = conn
            .query_row(
                "SELECT COUNT(*) FROM diagnostic_events WHERE session_id = ?1",
                [&session_id],
                |row| row.get::<_, u64>(0),
            )
            .expect("diagnostic count");
        let workflow_count = conn
            .query_row(
                "SELECT COUNT(*) FROM workflow_metric_events WHERE session_id = ?1",
                [&session_id],
                |row| row.get::<_, u64>(0),
            )
            .expect("workflow count");
        assert_eq!(diagnostic_count + workflow_count, 1_024);
    }

    #[test]
    fn core_rejection_diagnostics_are_exact_bounded_upserts() {
        let fixture = TempRuntimeHome::new("core-rejection-diagnostic").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let first = UtcTimestamp::parse("2099-07-17T01:02:03Z").expect("first timestamp");
        let second = UtcTimestamp::parse("2099-07-17T01:03:04Z").expect("second timestamp");
        let input = |occurred_at| CoreRejectionDiagnostic {
            project_id: "project_test",
            task_id: "task_test",
            method_name: MethodName::PrepareWrite,
            reason: CoreRejectionReason::CurrentChangeUnitRequired,
            occurred_at,
        };
        record_core_rejection_diagnostic(&context, input(&first)).expect("first observation");
        record_core_rejection_diagnostic(&context, input(&second)).expect("updated observation");

        let records = read_core_rejection_diagnostics(fixture.path()).expect("records");
        assert_eq!(
            records,
            vec![CoreRejectionDiagnosticRecord {
                project_id: "project_test".to_owned(),
                task_id: "task_test".to_owned(),
                method_name: "volicord.prepare_write".to_owned(),
                reason: "current_change_unit_required".to_owned(),
                occurred_at: "2099-07-17T01:03:04Z".to_owned(),
            }]
        );

        let invalid = CoreRejectionDiagnostic {
            method_name: MethodName::Status,
            ..input(&second)
        };
        assert!(record_core_rejection_diagnostic(&context, invalid).is_err());
    }

    #[test]
    fn core_rejection_diagnostics_enforce_global_retention() {
        let fixture = TempRuntimeHome::new("core-rejection-retention").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let now = UtcTimestamp::parse("2099-07-17T01:02:03Z").expect("timestamp");
        record_core_rejection_diagnostic(
            &context,
            CoreRejectionDiagnostic {
                project_id: "project_seed",
                task_id: "task_seed",
                method_name: MethodName::PrepareWrite,
                reason: CoreRejectionReason::CurrentChangeUnitRequired,
                occurred_at: &now,
            },
        )
        .expect("seed");
        let mut conn = Connection::open(diagnostics_db_path(fixture.path())).expect("database");
        let tx = conn.transaction().expect("transaction");
        for index in 0..=DIAGNOSTICS_MAX_CORE_REJECTIONS {
            tx.execute(
                "INSERT OR REPLACE INTO core_rejection_diagnostics (
                     project_id, task_id, method_name, reason, occurred_at
                 ) VALUES (?1, ?2, 'volicord.prepare_write',
                           'current_change_unit_required', ?3)",
                params![
                    format!("project_{index}"),
                    format!("task_{index}"),
                    now.to_canonical_string()
                ],
            )
            .expect("insert");
        }
        tx.commit().expect("commit");
        drop(conn);

        record_core_rejection_diagnostic(
            &context,
            CoreRejectionDiagnostic {
                project_id: "project_final",
                task_id: "task_final",
                method_name: MethodName::PrepareWrite,
                reason: CoreRejectionReason::CurrentChangeUnitRequired,
                occurred_at: &now,
            },
        )
        .expect("prune");
        assert_eq!(
            read_core_rejection_diagnostics(fixture.path())
                .expect("bounded records")
                .len(),
            DIAGNOSTICS_MAX_CORE_REJECTIONS as usize
        );
    }

    #[test]
    fn diagnostics_manifest_is_semantic_and_derived_from_canonical_sql() {
        let fixture = TempRuntimeHome::new("diagnostics-manifest").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let session_id = managed_session_id("session_manifest");
        start_diagnostic_session(&context, start(&session_id)).expect("initialize");

        let current = current_diagnostics_storage_manifest().expect("current manifest");
        assert_eq!(current.contract_id, DIAGNOSTICS_CONTRACT_ID);
        assert!(current.canonical_schema_digest.starts_with("sha256:"));
        assert_eq!(current.canonical_schema_digest.len(), 71);
        let conn = Connection::open(diagnostics_db_path(fixture.path())).expect("database");
        let persisted = conn
            .query_row(
                "SELECT contract_id, canonical_schema_digest
                   FROM diagnostics_manifest
                  WHERE singleton_id = 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .expect("persisted manifest");
        assert_eq!(persisted.0, current.contract_id);
        assert_eq!(persisted.1, current.canonical_schema_digest);
    }

    #[test]
    fn diagnostics_final_path_is_absent_until_the_complete_carrier_is_published() {
        let fixture = TempRuntimeHome::new("diagnostics-atomic-visibility").expect("fixture");
        let runtime_home = fixture.path().to_path_buf();
        let final_path = diagnostics_db_path(&runtime_home);
        let pause = diagnostics_publication_test_support::pause_creators(
            final_path.clone(),
            DiagnosticsPublicationPhase::BeforePublication,
            1,
        );
        let creator_home = runtime_home.clone();
        let creator = thread::spawn(move || {
            let mutation = TestRuntimeHomeAdmission::shared(&creator_home)
                .map_err(|error| error.to_string())?;
            let context = mutation.context().map_err(|error| error.to_string())?;
            start_diagnostic_session(&context, start(&managed_session_id("atomic_visibility")))
                .map_err(|error| error.to_string())
        });

        pause.wait_until_all_creators_are_paused();
        assert!(!final_path.exists());
        assert_eq!(diagnostics_staging_entries(&runtime_home).len(), 1);
        pause.resume_all_creators();
        creator
            .join()
            .expect("creator thread")
            .expect("publication");
        diagnostics_publication_test_support::clear(&final_path);

        assert!(final_path.is_file());
        open_diagnostics_database_read_only(&final_path).expect("complete final database");
        assert!(
            diagnostics_staging_entries(&runtime_home).is_empty(),
            "successful publication must leave no staging files"
        );
    }

    #[test]
    fn concurrent_shared_writers_publish_once_and_persist_both_sessions() {
        let fixture = TempRuntimeHome::new("diagnostics-concurrent-publish").expect("fixture");
        let runtime_home = fixture.path().to_path_buf();
        let final_path = diagnostics_db_path(&runtime_home);
        let pause = diagnostics_publication_test_support::pause_creators(
            final_path.clone(),
            DiagnosticsPublicationPhase::BeforePublication,
            2,
        );
        let creators = ["concurrent_first", "concurrent_second"].map(|native_session_id| {
            let creator_home = runtime_home.clone();
            thread::spawn(move || {
                let mutation = TestRuntimeHomeAdmission::shared(&creator_home)
                    .map_err(|error| error.to_string())?;
                let context = mutation.context().map_err(|error| error.to_string())?;
                start_diagnostic_session(&context, start(&managed_session_id(native_session_id)))
                    .map_err(|error| error.to_string())
            })
        });

        pause.wait_until_all_creators_are_paused();
        assert!(!final_path.exists());
        assert_eq!(diagnostics_staging_entries(&runtime_home).len(), 2);
        pause.resume_all_creators();
        for creator in creators {
            creator
                .join()
                .expect("creator thread")
                .expect("concurrent diagnostics start");
        }
        diagnostics_publication_test_support::clear(&final_path);

        let conn =
            open_diagnostics_database_read_only(&final_path).expect("validated final database");
        let session_ids = conn
            .prepare("SELECT session_id FROM diagnostic_sessions ORDER BY session_id")
            .expect("session query")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("session rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("session values");
        assert_eq!(
            session_ids,
            vec![
                managed_session_id("concurrent_first"),
                managed_session_id("concurrent_second")
            ]
        );
        assert!(diagnostics_staging_entries(&runtime_home).is_empty());
    }

    #[test]
    fn concurrent_transport_writers_converge_on_one_diagnostics_carrier() {
        let fixture = TempRuntimeHome::new("diagnostics-transport-convergence").expect("fixture");
        let runtime_home = fixture.path().to_path_buf();
        let final_path = diagnostics_db_path(&runtime_home);
        let pause = diagnostics_publication_test_support::pause_creators(
            final_path.clone(),
            DiagnosticsPublicationPhase::BeforePublication,
            3,
        );

        let mcp_home = runtime_home.clone();
        let mcp = thread::spawn(move || {
            let mutation =
                TestRuntimeHomeAdmission::shared(&mcp_home).map_err(|error| error.to_string())?;
            let context = mutation.context().map_err(|error| error.to_string())?;
            start_diagnostic_session(
                &context,
                DiagnosticSessionStart {
                    session_id: "mcp_transport_concurrent",
                    connection_id: Some("connection_test"),
                    project_id: Some("project_test"),
                    transport: DiagnosticTransport::McpStdio,
                    host_kind: Some(DiagnosticHostKind::Codex),
                    package_version: "0.1.0",
                    build_id: "test",
                },
            )
            .map_err(|error| error.to_string())
        });
        let guard_home = runtime_home.clone();
        let guard = thread::spawn(move || {
            let mutation =
                TestRuntimeHomeAdmission::shared(&guard_home).map_err(|error| error.to_string())?;
            let context = mutation.context().map_err(|error| error.to_string())?;
            start_diagnostic_session(
                &context,
                DiagnosticSessionStart {
                    session_id:
                        "agent_session_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                    connection_id: Some("connection_test"),
                    project_id: Some("project_test"),
                    transport: DiagnosticTransport::GuardHook,
                    host_kind: Some(DiagnosticHostKind::Codex),
                    package_version: "0.1.0",
                    build_id: "test",
                },
            )
            .map_err(|error| error.to_string())
        });
        let cli_home = runtime_home.clone();
        let cli = thread::spawn(move || {
            let mutation =
                TestRuntimeHomeAdmission::shared(&cli_home).map_err(|error| error.to_string())?;
            let context = mutation.context().map_err(|error| error.to_string())?;
            start_diagnostic_session(
                &context,
                DiagnosticSessionStart {
                    session_id: "cli_inbox_concurrent",
                    connection_id: None,
                    project_id: Some("project_test"),
                    transport: DiagnosticTransport::CliInbox,
                    host_kind: None,
                    package_version: "0.1.0",
                    build_id: "test",
                },
            )
            .map_err(|error| error.to_string())
        });

        pause.wait_until_all_creators_are_paused();
        pause.resume_all_creators();
        for creator in [mcp, guard, cli] {
            creator
                .join()
                .expect("creator thread")
                .expect("transport diagnostics start");
        }
        diagnostics_publication_test_support::clear(&final_path);

        let conn =
            open_diagnostics_database_read_only(&final_path).expect("validated final database");
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM diagnostic_sessions", [], |row| {
                row.get::<_, u64>(0)
            })
            .expect("session count"),
            3
        );
        assert!(diagnostics_staging_entries(&runtime_home).is_empty());
    }

    #[test]
    fn diagnostics_creation_faults_remove_only_owned_staging_files() {
        for (label, phase) in [
            (
                "diagnostics-initialization-fault",
                DiagnosticsPublicationPhase::DuringSchemaInitialization,
            ),
            (
                "diagnostics-post-validation-fault",
                DiagnosticsPublicationPhase::AfterStagingValidation,
            ),
        ] {
            let fixture = TempRuntimeHome::new(label).expect("fixture");
            let mutation =
                TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
            let context = mutation.context().expect("mutation context");
            let final_path = diagnostics_db_path(fixture.path());
            diagnostics_publication_test_support::fail_creator(final_path.clone(), phase);

            start_diagnostic_session(&context, start("diagnostics_fault_session"))
                .expect_err("injected creation fault must fail");
            diagnostics_publication_test_support::clear(&final_path);

            assert!(!final_path.exists());
            assert!(
                diagnostics_staging_entries(fixture.path()).is_empty(),
                "failed creator must remove its staging database and sidecars"
            );
        }
    }

    #[test]
    fn concurrent_external_empty_final_is_preserved_and_rejected() {
        let fixture = TempRuntimeHome::new("diagnostics-external-empty").expect("fixture");
        let runtime_home = fixture.path().to_path_buf();
        let final_path = diagnostics_db_path(&runtime_home);
        let pause = diagnostics_publication_test_support::pause_creators(
            final_path.clone(),
            DiagnosticsPublicationPhase::BeforePublication,
            1,
        );
        let creator_home = runtime_home.clone();
        let creator = thread::spawn(move || {
            let mutation = TestRuntimeHomeAdmission::shared(&creator_home)
                .map_err(|error| error.to_string())?;
            let context = mutation.context().map_err(|error| error.to_string())?;
            start_diagnostic_session(&context, start("external_empty_creator"))
                .map_err(|error| error.to_string())
        });

        pause.wait_until_all_creators_are_paused();
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&final_path)
            .expect("external empty final");
        let original = fs::read(&final_path).expect("original final bytes");
        pause.resume_all_creators();
        let error = creator
            .join()
            .expect("creator thread")
            .expect_err("empty concurrent winner must fail exact validation");
        diagnostics_publication_test_support::clear(&final_path);

        assert!(error.contains("storage profile"));
        assert_eq!(fs::read(&final_path).expect("preserved bytes"), original);
        assert!(diagnostics_staging_entries(&runtime_home).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn parent_sync_failure_keeps_the_complete_published_database() {
        use volicord_platform_fs::no_replace_file_publication_test_support::{
            fail_next_no_replace_file_publication, NoReplaceFilePublicationFault,
        };

        let fixture = TempRuntimeHome::new("diagnostics-parent-sync-failure").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let final_path = diagnostics_db_path(fixture.path());
        fail_next_no_replace_file_publication(
            NoReplaceFilePublicationFault::ParentDirectorySynchronizationFailure,
        );

        start_diagnostic_session(&context, start("parent_sync_first"))
            .expect_err("parent synchronization failure must be reported");
        assert!(final_path.is_file());
        let conn = open_diagnostics_database_read_only(&final_path)
            .expect("published carrier must remain complete");
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM diagnostic_sessions", [], |row| {
                row.get::<_, u64>(0)
            })
            .expect("session count"),
            0,
            "publication does not insert the caller's session into staging"
        );
        drop(conn);
        assert!(diagnostics_staging_entries(fixture.path()).is_empty());

        start_diagnostic_session(&context, start("parent_sync_retry"))
            .expect("the next caller can use the complete final database");
        assert!(
            read_diagnostic_session(fixture.path(), Some("parent_sync_retry"))
                .expect("read session")
                .is_some()
        );
    }

    #[cfg(unix)]
    #[test]
    fn diagnostics_file_permissions_are_hardened_after_publication() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = TempRuntimeHome::new("diagnostics-file-permissions").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        start_diagnostic_session(&context, start("permission_session")).expect("start");

        let mode = fs::metadata(diagnostics_db_path(fixture.path()))
            .expect("diagnostics metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn existing_empty_diagnostics_database_is_rejected_without_initialization() {
        let fixture = TempRuntimeHome::new("diagnostics-existing-empty").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        fs::create_dir_all(fixture.path()).expect("runtime home directory");
        let path = diagnostics_db_path(fixture.path());
        Connection::open(&path).expect("empty database");

        let error = start_diagnostic_session(
            &context,
            start(&managed_session_id("session_existing_empty")),
        )
        .expect_err("existing empty database must not be initialized");
        assert!(matches!(
            error,
            StoreError::UnsupportedStorageProfile { .. }
        ));
        let conn = Connection::open(path).expect("unchanged database");
        let object_count = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get::<_, u64>(0),
            )
            .expect("object count");
        assert_eq!(object_count, 0);
    }

    #[test]
    fn missing_diagnostics_manifest_row_is_rejected_without_repair() {
        let fixture = TempRuntimeHome::new("diagnostics-missing-manifest-row").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let session_id = managed_session_id("session_missing_manifest");
        start_diagnostic_session(&context, start(&session_id)).expect("initialize");
        let path = diagnostics_db_path(fixture.path());
        let conn = Connection::open(&path).expect("database");
        conn.execute("DELETE FROM diagnostics_manifest", [])
            .expect("remove manifest row");
        drop(conn);

        let error = read_diagnostic_session(fixture.path(), None)
            .expect_err("missing manifest row must fail closed");
        assert!(matches!(
            error,
            StoreError::UnsupportedStorageProfile { .. }
        ));
        let conn = Connection::open(path).expect("database remains inspectable");
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM diagnostics_manifest", [], |row| {
                row.get::<_, u64>(0)
            })
            .expect("manifest count"),
            0
        );
    }

    #[test]
    fn extra_diagnostics_manifest_row_is_rejected() {
        let fixture = TempRuntimeHome::new("diagnostics-extra-manifest-row").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let session_id = managed_session_id("session_extra_manifest");
        start_diagnostic_session(&context, start(&session_id)).expect("initialize");
        let conn = Connection::open(diagnostics_db_path(fixture.path())).expect("database");
        conn.pragma_update(None, "ignore_check_constraints", "ON")
            .expect("test-only constraint bypass");
        let current = current_diagnostics_storage_manifest().expect("manifest");
        conn.execute(
            "INSERT INTO diagnostics_manifest (
                 singleton_id, contract_id, canonical_schema_digest
             ) VALUES (2, ?1, ?2)",
            params![&current.contract_id, &current.canonical_schema_digest],
        )
        .expect("extra manifest row");
        drop(conn);

        let error = read_diagnostic_session(fixture.path(), None)
            .expect_err("extra manifest row must fail closed");
        assert!(matches!(error, StoreError::SchemaInvariant { .. }));
    }

    #[test]
    fn unknown_diagnostics_manifest_contract_is_rejected() {
        let fixture = TempRuntimeHome::new("diagnostics-unknown-manifest").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let session_id = managed_session_id("session_unknown_manifest");
        start_diagnostic_session(&context, start(&session_id)).expect("initialize");
        let conn = Connection::open(diagnostics_db_path(fixture.path())).expect("database");
        conn.execute(
            "UPDATE diagnostics_manifest SET contract_id = 'unknown.diagnostics.contract'",
            [],
        )
        .expect("replace contract identity");
        drop(conn);

        let error = read_diagnostic_session(fixture.path(), None)
            .expect_err("unknown manifest contract must fail closed");
        assert!(matches!(
            error,
            StoreError::UnsupportedStorageProfile { .. }
        ));
    }

    #[test]
    fn missing_canonical_diagnostics_index_is_rejected() {
        let fixture = TempRuntimeHome::new("diagnostics-missing-index").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let session_id = managed_session_id("session_missing_index");
        start_diagnostic_session(&context, start(&session_id)).expect("initialize");
        let conn = Connection::open(diagnostics_db_path(fixture.path())).expect("database");
        conn.execute("DROP INDEX idx_diagnostic_events_tool", [])
            .expect("remove canonical index");
        drop(conn);

        let error = read_diagnostic_session(fixture.path(), None)
            .expect_err("missing canonical index must fail closed");
        assert!(matches!(error, StoreError::SchemaInvariant { .. }));
    }

    #[test]
    fn unexpected_diagnostics_schema_objects_are_rejected() {
        let fixture = TempRuntimeHome::new("diagnostics-unexpected-object").expect("fixture");
        let mutation =
            TestRuntimeHomeAdmission::shared(fixture.path()).expect("mutation admission");
        let context = mutation.context().expect("mutation context");
        let session_id = managed_session_id("session_unexpected_object");
        start_diagnostic_session(&context, start(&session_id)).expect("initialize");
        let conn = Connection::open(diagnostics_db_path(fixture.path())).expect("database");
        conn.execute("CREATE TABLE unexpected_diagnostics_state (value TEXT)", [])
            .expect("unexpected object");
        drop(conn);

        let error = read_diagnostic_session(fixture.path(), None)
            .expect_err("unexpected object must fail closed");
        assert!(matches!(error, StoreError::SchemaInvariant { .. }));
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
        assert!(read_core_rejection_diagnostics(fixture.path())
            .expect("empty Core rejection read")
            .is_empty());
        assert!(!diagnostics_db_path(fixture.path()).exists());
        assert!(
            diagnostics_staging_entries(fixture.path()).is_empty(),
            "read-only diagnostics operations must not create staging storage"
        );

        fs::create_dir_all(fixture.path()).expect("runtime home directory");
        let unrelated_staging = fixture
            .path()
            .join(format!("{DIAGNOSTICS_STAGING_PREFIX}opaque-test-identity"));
        fs::write(
            &unrelated_staging,
            b"not an authoritative diagnostics carrier",
        )
        .expect("staging fixture");
        assert!(read_diagnostic_session(fixture.path(), None)
            .expect("read must ignore staging")
            .is_none());
        assert!(
            read_workflow_metric_aggregates(fixture.path(), "project_test")
                .expect("workflow read must ignore staging")
                .is_empty()
        );
        assert!(!diagnostics_db_path(fixture.path()).exists());
        assert_eq!(
            fs::read(unrelated_staging).expect("staging bytes"),
            b"not an authoritative diagnostics carrier"
        );
    }
}
