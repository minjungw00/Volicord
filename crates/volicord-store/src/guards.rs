use std::{path::Path, str::FromStr};

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde_json::Value;
use volicord_types::{
    GuardDecision, GuardInstallationStatus, HostKind, IntegrationProfile, PromptCaptureStatus,
    UnrecordedChangeStatus,
};

use crate::{
    agent_connections::{
        agent_connection_record, is_agent_connection_project_allowed, AgentConnectionRecord,
    },
    bootstrap::{project_record_for_execution, raw_project_record_from_conn, ProjectRecord},
    sqlite::{
        begin_immediate_transaction, open_project_state_database, open_registry_database,
        registry_db_path,
    },
    StoreError, StoreResult,
};

const REQUIRED_GUARD_HOOK_PHASES: &[&str] = &[
    "session_start_hook",
    "pre_tool_hook",
    "post_tool_hook",
    "user_prompt_submit_hook",
    "stop_hook",
];

const KNOWN_GUARD_OBSERVATION_PHASES: &[&str] = &[
    "session_start",
    "pre_tool",
    "post_tool",
    "prompt_capture",
    "stop",
];

/// Guard installation creation or update input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardInstallationUpsert {
    pub guard_installation_id: String,
    pub connection_internal_id: String,
    pub project_id: Option<String>,
    pub host_kind: String,
    pub guard_mode: String,
    pub host_capability_json: String,
    pub installation_status: String,
    pub installed_at: Option<String>,
    pub last_checked_at: String,
    pub first_seen_at: Option<String>,
    pub last_seen_at: Option<String>,
    pub last_seen_phase: Option<String>,
    pub observed_host_kind: Option<String>,
    pub observed_policy_hash: Option<String>,
    pub observed_binary_version: Option<String>,
    pub metadata_json: String,
}

/// Guard installation row stored in `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardInstallationRecord {
    pub guard_installation_id: String,
    pub runtime_home_id: String,
    pub connection_internal_id: String,
    pub project_id: Option<String>,
    pub project_internal_id: Option<String>,
    pub host_kind: String,
    pub guard_mode: String,
    pub host_capability_json: String,
    pub installation_status: String,
    pub installed_at: Option<String>,
    pub last_checked_at: String,
    pub first_seen_at: Option<String>,
    pub last_seen_at: Option<String>,
    pub last_seen_phase: Option<String>,
    pub observed_host_kind: Option<String>,
    pub observed_policy_hash: Option<String>,
    pub observed_binary_version: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Validated guard hook observation for one registered installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardInstallationObservation {
    pub guard_installation_id: String,
    pub connection_internal_id: String,
    pub project_id: String,
    pub host_kind: String,
    pub guard_mode: String,
    pub observed_policy_hash: String,
    pub observed_binary_version: Option<String>,
    pub observed_phase: String,
    pub observed_at: String,
}

/// Agent Session insert input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionInsert {
    pub session_id: String,
    pub connection_internal_id: String,
    pub guard_installation_id: Option<String>,
    pub host_kind: String,
    pub guard_mode: String,
    pub started_at: String,
    pub metadata_json: String,
}

/// Agent Session row stored in project `state.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionRecord {
    pub project_id: String,
    pub session_id: String,
    pub connection_internal_id: String,
    pub guard_installation_id: Option<String>,
    pub host_kind: String,
    pub guard_mode: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub metadata_json: String,
}

/// Guard event insert input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardEventInsert {
    pub guard_event_id: String,
    pub session_id: Option<String>,
    pub connection_internal_id: String,
    pub guard_installation_id: Option<String>,
    pub event_kind: String,
    pub decision: String,
    pub subject_json: String,
    pub result_json: String,
    pub occurred_at: String,
    pub metadata_json: String,
}

/// Guard event row stored in project `state.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardEventRecord {
    pub project_id: String,
    pub guard_event_id: String,
    pub session_id: Option<String>,
    pub connection_internal_id: String,
    pub guard_installation_id: Option<String>,
    pub event_kind: String,
    pub decision: String,
    pub subject_json: String,
    pub result_json: String,
    pub occurred_at: String,
    pub metadata_json: String,
}

/// Prompt capture insert input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptCaptureInsert {
    pub prompt_capture_id: String,
    pub session_id: String,
    pub connection_internal_id: String,
    pub capture_kind: String,
    pub prompt_sha256: String,
    pub prompt_text: Option<String>,
    pub captured_at: String,
    pub metadata_json: String,
}

/// Prompt capture row stored in project `state.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptCaptureRecord {
    pub project_id: String,
    pub prompt_capture_id: String,
    pub session_id: String,
    pub connection_internal_id: String,
    pub capture_kind: String,
    pub prompt_sha256: String,
    pub prompt_text: Option<String>,
    pub captured_at: String,
    pub metadata_json: String,
}

/// Expected Product Repository write insert input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedWriteInsert {
    pub expected_write_id: String,
    pub session_id: Option<String>,
    pub connection_internal_id: String,
    pub guard_installation_id: Option<String>,
    pub pre_tool_guard_event_id: String,
    pub host_invocation_id: Option<String>,
    pub tool_name: Option<String>,
    pub command_kind: String,
    pub path_policy: String,
    pub expected_paths_json: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub write_ticket_ids_json: String,
    pub basis_state_version: u64,
    pub created_at: String,
    pub expires_at: String,
    pub metadata_json: String,
}

/// Expected Product Repository write match input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedWriteMatch {
    pub matched_post_tool_guard_event_id: String,
    pub matched_paths_json: String,
    pub matched_at: String,
}

/// Expected Product Repository write row stored in project `state.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedWriteRecord {
    pub project_id: String,
    pub expected_write_id: String,
    pub session_id: Option<String>,
    pub connection_internal_id: String,
    pub guard_installation_id: Option<String>,
    pub pre_tool_guard_event_id: String,
    pub host_invocation_id: Option<String>,
    pub tool_name: Option<String>,
    pub command_kind: String,
    pub path_policy: String,
    pub expected_paths_json: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub write_ticket_ids_json: String,
    pub basis_state_version: u64,
    pub status: String,
    pub matched_post_tool_guard_event_id: Option<String>,
    pub matched_paths_json: Option<String>,
    pub created_at: String,
    pub expires_at: String,
    pub matched_at: Option<String>,
    pub metadata_json: String,
}

/// Unrecorded Product Repository change insert input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecordedChangeInsert {
    pub unrecorded_change_id: String,
    pub session_id: Option<String>,
    pub connection_internal_id: String,
    pub task_id: Option<String>,
    pub summary: String,
    pub observed_paths_json: String,
    pub detection_json: String,
    pub detected_at: String,
    pub metadata_json: String,
}

/// Unrecorded Product Repository change resolution input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecordedChangeResolution {
    pub resolution_json: String,
    pub resolved_at: String,
    pub resolved_by_actor_source: String,
}

/// Unrecorded Product Repository change row stored in project `state.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecordedChangeRecord {
    pub project_id: String,
    pub unrecorded_change_id: String,
    pub session_id: Option<String>,
    pub connection_internal_id: String,
    pub task_id: Option<String>,
    pub status: String,
    pub summary: String,
    pub observed_paths_json: String,
    pub detection_json: String,
    pub resolution_json: Option<String>,
    pub detected_at: String,
    pub resolved_at: Option<String>,
    pub resolved_by_actor_source: Option<String>,
    pub metadata_json: String,
}

/// Read-only guard-health facts for one project and Agent Connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardHealthRecord {
    pub connection: Option<AgentConnectionRecord>,
    pub guard_installation: Option<GuardInstallationRecord>,
    pub latest_session: Option<AgentSessionRecord>,
    pub latest_event: Option<GuardEventRecord>,
    pub unresolved_unrecorded_changes: Vec<UnrecordedChangeRecord>,
}

/// Derived prompt-capture availability for one project and Agent Connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptCaptureAvailability {
    pub status: PromptCaptureStatus,
    pub host_supports_prompt_capture: bool,
    pub prompt_capture_configured: bool,
    pub policy_hash_matches_observation: bool,
}

impl PromptCaptureAvailability {
    pub fn can_use_chat_commands(&self) -> bool {
        self.status.allows_chat_judgment_commands()
    }
}

/// Creates or updates one guard installation in the Runtime Home registry.
pub fn upsert_guard_installation(
    runtime_home: impl AsRef<Path>,
    input: GuardInstallationUpsert,
) -> StoreResult<GuardInstallationRecord> {
    validate_guard_installation_upsert(&input)?;

    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let runtime_home_id = require_runtime_home_id(&conn)?;
    let connection_id = input.connection_internal_id.clone();
    require_connection(&conn, &connection_id)?;
    let project_internal_id = input
        .project_id
        .as_deref()
        .map(|project_id| {
            let project = raw_project_record_from_conn(&conn, project_id)?.ok_or_else(|| {
                StoreError::NotFound {
                    entity: "project",
                    id: project_id.to_owned(),
                }
            })?;
            require_connection_project_membership(
                &conn,
                &connection_id,
                &project.project_internal_id,
            )?;
            Ok::<String, StoreError>(project.project_internal_id)
        })
        .transpose()?;

    let tx = begin_immediate_transaction(&mut conn)?;
    if let Some(existing_id) = guard_installation_id_for_scope(
        &tx,
        &input.connection_internal_id,
        project_internal_id.as_deref(),
        &input.guard_mode,
    )? {
        if existing_id != input.guard_installation_id {
            return Err(StoreError::Conflict {
                entity: "guard_installation",
                id: input.guard_installation_id,
                detail: "connection/project/guard_mode scope is already recorded by another guard_installation_id".to_owned(),
            });
        }
    }
    tx.execute(
        "INSERT INTO guard_installations (
            guard_installation_id,
            runtime_home_id,
            connection_internal_id,
            project_internal_id,
            host_kind,
            guard_mode,
            host_capability_json,
            installation_status,
            installed_at,
            last_checked_at,
            first_seen_at,
            last_seen_at,
            last_seen_phase,
            observed_host_kind,
            observed_policy_hash,
            observed_binary_version,
            metadata_json,
            created_at,
            updated_at
        )
        VALUES (
            ?1,
            ?2,
            ?3,
            ?4,
            ?5,
            ?6,
            ?7,
            ?8,
            ?9,
            ?10,
            ?11,
            ?12,
            ?13,
            ?14,
            ?15,
            ?16,
            ?17,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        )
        ON CONFLICT(guard_installation_id) DO UPDATE SET
            runtime_home_id = excluded.runtime_home_id,
            connection_internal_id = excluded.connection_internal_id,
            project_internal_id = excluded.project_internal_id,
            host_kind = excluded.host_kind,
            guard_mode = excluded.guard_mode,
            host_capability_json = excluded.host_capability_json,
            installation_status = CASE
                WHEN guard_installations.installation_status = 'active'
                 AND excluded.installation_status = 'configured'
                 AND guard_installations.host_capability_json = excluded.host_capability_json
                 AND guard_installations.host_kind = excluded.host_kind
                 AND guard_installations.guard_mode = excluded.guard_mode
                THEN guard_installations.installation_status
                ELSE excluded.installation_status
            END,
            installed_at = excluded.installed_at,
            last_checked_at = excluded.last_checked_at,
            first_seen_at = COALESCE(excluded.first_seen_at, first_seen_at),
            last_seen_at = COALESCE(excluded.last_seen_at, last_seen_at),
            last_seen_phase = COALESCE(excluded.last_seen_phase, last_seen_phase),
            observed_host_kind = COALESCE(excluded.observed_host_kind, observed_host_kind),
            observed_policy_hash = COALESCE(excluded.observed_policy_hash, observed_policy_hash),
            observed_binary_version = COALESCE(excluded.observed_binary_version, observed_binary_version),
            metadata_json = excluded.metadata_json,
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        params![
            input.guard_installation_id,
            runtime_home_id,
            input.connection_internal_id,
            project_internal_id,
            input.host_kind,
            input.guard_mode,
            input.host_capability_json,
            input.installation_status,
            input.installed_at,
            input.last_checked_at,
            input.first_seen_at,
            input.last_seen_at,
            input.last_seen_phase,
            input.observed_host_kind,
            input.observed_policy_hash,
            input.observed_binary_version,
            input.metadata_json,
        ],
    )?;
    tx.commit()?;

    guard_installation(&runtime_home, &input.guard_installation_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "guard_installation",
            id: input.guard_installation_id,
        }
    })
}

/// Reads one guard installation by id.
pub fn guard_installation(
    runtime_home: impl AsRef<Path>,
    guard_installation_id: &str,
) -> StoreResult<Option<GuardInstallationRecord>> {
    validate_identifier("guard_installation_id", guard_installation_id)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database(registry_path)?;
    guard_installation_from_conn(&conn, guard_installation_id)
}

/// Lists guard installations for one connection and optional project.
pub fn list_guard_installations(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
    project_id: Option<&str>,
) -> StoreResult<Vec<GuardInstallationRecord>> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    if let Some(project_id) = project_id {
        validate_identifier("project_id", project_id)?;
    }

    let registry_path = registry_db_path(runtime_home.as_ref());
    if !registry_path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database(&registry_path)?;
    let project_internal_id = project_id
        .map(|project_id| {
            raw_project_record_from_conn(&conn, project_id)?
                .map(|project| project.project_internal_id)
                .ok_or_else(|| StoreError::NotFound {
                    entity: "project",
                    id: project_id.to_owned(),
                })
        })
        .transpose()?;

    let mut stmt = conn.prepare(
        "SELECT
            gi.guard_installation_id,
            gi.runtime_home_id,
            gi.connection_internal_id,
            gi.project_internal_id,
            p.project_internal_id,
            gi.host_kind,
            gi.guard_mode,
            gi.host_capability_json,
            gi.installation_status,
            gi.installed_at,
            gi.last_checked_at,
            gi.first_seen_at,
            gi.last_seen_at,
            gi.last_seen_phase,
            gi.observed_host_kind,
            gi.observed_policy_hash,
            gi.observed_binary_version,
            gi.metadata_json,
            gi.created_at,
            gi.updated_at
         FROM guard_installations AS gi
         LEFT JOIN projects AS p
           ON p.project_internal_id = gi.project_internal_id
        WHERE gi.connection_internal_id = ?1
          AND (
            (?2 IS NULL AND gi.project_internal_id IS NULL)
            OR gi.project_internal_id = ?2
          )
        ORDER BY gi.guard_mode, gi.guard_installation_id",
    )?;
    let rows = stmt.query_map(
        params![connection_internal_id, project_internal_id],
        guard_installation_from_row,
    )?;
    collect_rows(rows)
}

/// Records a validated guard hook observation and promotes healthy configured installations.
pub fn observe_guard_installation(
    runtime_home: impl AsRef<Path>,
    input: GuardInstallationObservation,
) -> StoreResult<Option<GuardInstallationRecord>> {
    validate_guard_installation_observation(&input)?;

    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }
    let mut conn = open_registry_database(&registry_path)?;
    let Some(project) = raw_project_record_from_conn(&conn, &input.project_id)? else {
        return Ok(None);
    };
    let Some(existing) = guard_installation_from_conn(&conn, &input.guard_installation_id)? else {
        return Ok(None);
    };
    if existing.connection_internal_id != input.connection_internal_id
        || existing.project_internal_id.as_deref() != Some(project.project_internal_id.as_str())
        || existing.host_kind != input.host_kind
        || existing.guard_mode != input.guard_mode
        || expected_policy_hash(&existing.host_capability_json)?.as_deref()
            != Some(input.observed_policy_hash.as_str())
    {
        return Ok(None);
    }
    require_connection_project_membership(
        &conn,
        &input.connection_internal_id,
        &project.project_internal_id,
    )?;
    let next_installation_status = guard_status_after_observation(&existing)?;

    let tx = begin_immediate_transaction(&mut conn)?;
    tx.execute(
        "UPDATE guard_installations
            SET installation_status = ?2,
                first_seen_at = COALESCE(first_seen_at, ?3),
                last_seen_at = ?3,
                last_seen_phase = ?4,
                observed_host_kind = ?5,
                observed_policy_hash = ?6,
                observed_binary_version = ?7,
                last_checked_at = ?3,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE guard_installation_id = ?1",
        params![
            input.guard_installation_id,
            next_installation_status,
            input.observed_at,
            input.observed_phase,
            input.host_kind,
            input.observed_policy_hash,
            input.observed_binary_version,
        ],
    )?;
    tx.commit()?;
    guard_installation(&runtime_home, &input.guard_installation_id)
}

/// Inserts one project-scoped Agent Session row.
pub fn insert_agent_session(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    input: AgentSessionInsert,
) -> StoreResult<AgentSessionRecord> {
    validate_agent_session_insert(&input)?;
    let mut project = open_guard_project(runtime_home, project_id, &input.connection_internal_id)?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    tx.execute(
        "INSERT INTO agent_sessions (
            project_id,
            session_id,
            connection_internal_id,
            guard_installation_id,
            host_kind,
            guard_mode,
            started_at,
            metadata_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            project.project.project_id,
            input.session_id,
            input.connection_internal_id,
            input.guard_installation_id,
            input.host_kind,
            input.guard_mode,
            input.started_at,
            input.metadata_json
        ],
    )?;
    tx.commit()?;

    agent_session_by_conn(
        &project.conn,
        &project.project.project_id,
        &input.session_id,
    )
}

/// Reads one project-scoped Agent Session row.
pub fn agent_session(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    session_id: &str,
) -> StoreResult<Option<AgentSessionRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("session_id", session_id)?;
    let project = open_project_for_read(runtime_home, project_id)?;
    project
        .map(|project| {
            agent_session_from_conn(&project.conn, &project.project.project_id, session_id)
        })
        .transpose()
        .map(Option::flatten)
}

/// Marks one Agent Session ended.
pub fn end_agent_session(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    session_id: &str,
    ended_at: &str,
) -> StoreResult<AgentSessionRecord> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("session_id", session_id)?;
    validate_timestamp_text("ended_at", ended_at)?;
    let mut project = open_project_for_required_read(runtime_home, project_id)?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    let changed = tx.execute(
        "UPDATE agent_sessions
            SET ended_at = ?3
          WHERE project_id = ?1
            AND session_id = ?2",
        params![project.project.project_id, session_id, ended_at],
    )?;
    tx.commit()?;
    if changed == 0 {
        return Err(StoreError::NotFound {
            entity: "agent_session",
            id: session_id.to_owned(),
        });
    }

    agent_session_by_conn(&project.conn, &project.project.project_id, session_id)
}

/// Inserts one project-scoped guard event row.
pub fn insert_guard_event(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    input: GuardEventInsert,
) -> StoreResult<GuardEventRecord> {
    validate_guard_event_insert(&input)?;
    let mut project = open_guard_project(runtime_home, project_id, &input.connection_internal_id)?;
    validate_optional_session_scope(
        &project.conn,
        &project.project.project_id,
        input.session_id.as_deref(),
        &input.connection_internal_id,
    )?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    tx.execute(
        "INSERT INTO guard_events (
            project_id,
            guard_event_id,
            session_id,
            connection_internal_id,
            guard_installation_id,
            event_kind,
            decision,
            subject_json,
            result_json,
            occurred_at,
            metadata_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            project.project.project_id,
            input.guard_event_id,
            input.session_id,
            input.connection_internal_id,
            input.guard_installation_id,
            input.event_kind,
            input.decision,
            input.subject_json,
            input.result_json,
            input.occurred_at,
            input.metadata_json
        ],
    )?;
    tx.commit()?;

    guard_event_by_conn(
        &project.conn,
        &project.project.project_id,
        &input.guard_event_id,
    )
}

/// Reads one project-scoped guard event row.
pub fn guard_event(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    guard_event_id: &str,
) -> StoreResult<Option<GuardEventRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("guard_event_id", guard_event_id)?;
    let project = open_project_for_read(runtime_home, project_id)?;
    project
        .map(|project| {
            guard_event_from_conn(&project.conn, &project.project.project_id, guard_event_id)
        })
        .transpose()
        .map(Option::flatten)
}

/// Inserts one project-scoped prompt capture row.
pub fn insert_prompt_capture(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    input: PromptCaptureInsert,
) -> StoreResult<PromptCaptureRecord> {
    validate_prompt_capture_insert(&input)?;
    let mut project = open_guard_project(runtime_home, project_id, &input.connection_internal_id)?;
    validate_session_scope(
        &project.conn,
        &project.project.project_id,
        &input.session_id,
        &input.connection_internal_id,
    )?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    tx.execute(
        "INSERT INTO prompt_captures (
            project_id,
            prompt_capture_id,
            session_id,
            connection_internal_id,
            capture_kind,
            prompt_sha256,
            prompt_text,
            captured_at,
            metadata_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            project.project.project_id,
            input.prompt_capture_id,
            input.session_id,
            input.connection_internal_id,
            input.capture_kind,
            input.prompt_sha256,
            input.prompt_text,
            input.captured_at,
            input.metadata_json
        ],
    )?;
    tx.commit()?;

    prompt_capture_by_conn(
        &project.conn,
        &project.project.project_id,
        &input.prompt_capture_id,
    )
}

/// Reads one project-scoped prompt capture row.
pub fn prompt_capture(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    prompt_capture_id: &str,
) -> StoreResult<Option<PromptCaptureRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("prompt_capture_id", prompt_capture_id)?;
    let project = open_project_for_read(runtime_home, project_id)?;
    project
        .map(|project| {
            prompt_capture_from_conn(
                &project.conn,
                &project.project.project_id,
                prompt_capture_id,
            )
        })
        .transpose()
        .map(Option::flatten)
}

/// Inserts one project-scoped expected-write row or returns the existing row.
pub fn insert_expected_write(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    input: ExpectedWriteInsert,
) -> StoreResult<ExpectedWriteRecord> {
    validate_expected_write_insert(&input)?;
    let mut project = open_guard_project(runtime_home, project_id, &input.connection_internal_id)?;
    validate_optional_session_scope(
        &project.conn,
        &project.project.project_id,
        input.session_id.as_deref(),
        &input.connection_internal_id,
    )?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    tx.execute(
        "INSERT OR IGNORE INTO expected_writes (
            project_id,
            expected_write_id,
            session_id,
            connection_internal_id,
            guard_installation_id,
            pre_tool_guard_event_id,
            host_invocation_id,
            tool_name,
            command_kind,
            path_policy,
            expected_paths_json,
            task_id,
            change_unit_id,
            write_ticket_ids_json,
            basis_state_version,
            status,
            created_at,
            expires_at,
            metadata_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 'pending', ?16, ?17, ?18)",
        params![
            project.project.project_id,
            input.expected_write_id,
            input.session_id,
            input.connection_internal_id,
            input.guard_installation_id,
            input.pre_tool_guard_event_id,
            input.host_invocation_id,
            input.tool_name,
            input.command_kind,
            input.path_policy,
            input.expected_paths_json,
            input.task_id,
            input.change_unit_id,
            input.write_ticket_ids_json,
            input.basis_state_version,
            input.created_at,
            input.expires_at,
            input.metadata_json,
        ],
    )?;
    tx.commit()?;

    expected_write_by_conn(
        &project.conn,
        &project.project.project_id,
        &input.expected_write_id,
    )
}

/// Reads one project-scoped expected-write row.
pub fn expected_write(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    expected_write_id: &str,
) -> StoreResult<Option<ExpectedWriteRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("expected_write_id", expected_write_id)?;
    let project = open_project_for_read(runtime_home, project_id)?;
    project
        .map(|project| {
            expected_write_from_conn(
                &project.conn,
                &project.project.project_id,
                expected_write_id,
            )
        })
        .transpose()
        .map(Option::flatten)
}

/// Lists pending expected writes for one project and Agent Connection.
pub fn list_pending_expected_writes(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    connection_internal_id: &str,
) -> StoreResult<Vec<ExpectedWriteRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("connection_internal_id", connection_internal_id)?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(Vec::new());
    };
    let mut stmt = project.conn.prepare(
        "SELECT
            project_id,
            expected_write_id,
            session_id,
            connection_internal_id,
            guard_installation_id,
            pre_tool_guard_event_id,
            host_invocation_id,
            tool_name,
            command_kind,
            path_policy,
            expected_paths_json,
            task_id,
            change_unit_id,
            write_ticket_ids_json,
            basis_state_version,
            status,
            matched_post_tool_guard_event_id,
            matched_paths_json,
            created_at,
            expires_at,
            matched_at,
            metadata_json
         FROM expected_writes
        WHERE project_id = ?1
          AND connection_internal_id = ?2
          AND status = 'pending'
        ORDER BY created_at DESC, expected_write_id DESC",
    )?;
    let rows = stmt.query_map(
        params![project.project.project_id, connection_internal_id],
        expected_write_from_row,
    )?;
    collect_rows(rows)
}

/// Lists all expected writes for one project and Agent Connection.
pub fn list_expected_writes_for_connection(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    connection_internal_id: &str,
) -> StoreResult<Vec<ExpectedWriteRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("connection_internal_id", connection_internal_id)?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(Vec::new());
    };
    let mut stmt = project.conn.prepare(
        "SELECT
            project_id,
            expected_write_id,
            session_id,
            connection_internal_id,
            guard_installation_id,
            pre_tool_guard_event_id,
            host_invocation_id,
            tool_name,
            command_kind,
            path_policy,
            expected_paths_json,
            task_id,
            change_unit_id,
            write_ticket_ids_json,
            basis_state_version,
            status,
            matched_post_tool_guard_event_id,
            matched_paths_json,
            created_at,
            expires_at,
            matched_at,
            metadata_json
         FROM expected_writes
        WHERE project_id = ?1
          AND connection_internal_id = ?2
        ORDER BY created_at DESC, expected_write_id DESC",
    )?;
    let rows = stmt.query_map(
        params![project.project.project_id, connection_internal_id],
        expected_write_from_row,
    )?;
    collect_rows(rows)
}

/// Lists expected writes already matched by one post-tool guard event.
pub fn list_expected_writes_matched_by_post_event(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    connection_internal_id: &str,
    post_tool_guard_event_id: &str,
) -> StoreResult<Vec<ExpectedWriteRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("connection_internal_id", connection_internal_id)?;
    validate_identifier("post_tool_guard_event_id", post_tool_guard_event_id)?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(Vec::new());
    };
    let mut stmt = project.conn.prepare(
        "SELECT
            project_id,
            expected_write_id,
            session_id,
            connection_internal_id,
            guard_installation_id,
            pre_tool_guard_event_id,
            host_invocation_id,
            tool_name,
            command_kind,
            path_policy,
            expected_paths_json,
            task_id,
            change_unit_id,
            write_ticket_ids_json,
            basis_state_version,
            status,
            matched_post_tool_guard_event_id,
            matched_paths_json,
            created_at,
            expires_at,
            matched_at,
            metadata_json
         FROM expected_writes
        WHERE project_id = ?1
          AND connection_internal_id = ?2
          AND status = 'matched'
          AND matched_post_tool_guard_event_id = ?3
        ORDER BY matched_at DESC, expected_write_id DESC",
    )?;
    let rows = stmt.query_map(
        params![
            project.project.project_id,
            connection_internal_id,
            post_tool_guard_event_id
        ],
        expected_write_from_row,
    )?;
    collect_rows(rows)
}

/// Marks one pending expected-write row matched by a post-tool observation.
pub fn mark_expected_write_matched(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    expected_write_id: &str,
    input: ExpectedWriteMatch,
) -> StoreResult<ExpectedWriteRecord> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("expected_write_id", expected_write_id)?;
    validate_expected_write_match(&input)?;
    let mut project = open_project_for_required_read(runtime_home, project_id)?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    let changed = tx.execute(
        "UPDATE expected_writes
            SET status = 'matched',
                matched_post_tool_guard_event_id = ?3,
                matched_paths_json = ?4,
                matched_at = ?5
          WHERE project_id = ?1
            AND expected_write_id = ?2
            AND status = 'pending'",
        params![
            project.project.project_id,
            expected_write_id,
            input.matched_post_tool_guard_event_id,
            input.matched_paths_json,
            input.matched_at,
        ],
    )?;
    tx.commit()?;
    if changed == 0 {
        let Some(existing) = expected_write_from_conn(
            &project.conn,
            &project.project.project_id,
            expected_write_id,
        )?
        else {
            return Err(StoreError::NotFound {
                entity: "expected_write",
                id: expected_write_id.to_owned(),
            });
        };
        return Err(StoreError::Conflict {
            entity: "expected_write",
            id: existing.expected_write_id,
            detail: "expected write is already matched".to_owned(),
        });
    }

    expected_write_by_conn(
        &project.conn,
        &project.project.project_id,
        expected_write_id,
    )
}

/// Inserts one unresolved unrecorded-change row.
pub fn insert_unrecorded_change(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    input: UnrecordedChangeInsert,
) -> StoreResult<UnrecordedChangeRecord> {
    validate_unrecorded_change_insert(&input)?;
    let mut project = open_guard_project(runtime_home, project_id, &input.connection_internal_id)?;
    validate_optional_session_scope(
        &project.conn,
        &project.project.project_id,
        input.session_id.as_deref(),
        &input.connection_internal_id,
    )?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    tx.execute(
        "INSERT INTO unrecorded_changes (
            project_id,
            unrecorded_change_id,
            session_id,
            connection_internal_id,
            task_id,
            status,
            summary,
            observed_paths_json,
            detection_json,
            detected_at,
            metadata_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, 'unresolved', ?6, ?7, ?8, ?9, ?10)",
        params![
            project.project.project_id,
            input.unrecorded_change_id,
            input.session_id,
            input.connection_internal_id,
            input.task_id,
            input.summary,
            input.observed_paths_json,
            input.detection_json,
            input.detected_at,
            input.metadata_json
        ],
    )?;
    tx.commit()?;

    unrecorded_change_by_conn(
        &project.conn,
        &project.project.project_id,
        &input.unrecorded_change_id,
    )
}

/// Reads one project-scoped unrecorded-change row.
pub fn unrecorded_change(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    unrecorded_change_id: &str,
) -> StoreResult<Option<UnrecordedChangeRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("unrecorded_change_id", unrecorded_change_id)?;
    let project = open_project_for_read(runtime_home, project_id)?;
    project
        .map(|project| {
            unrecorded_change_from_conn(
                &project.conn,
                &project.project.project_id,
                unrecorded_change_id,
            )
        })
        .transpose()
        .map(Option::flatten)
}

/// Lists unresolved unrecorded changes for a project, optionally narrowed by connection.
pub fn list_unresolved_unrecorded_changes(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    connection_internal_id: Option<&str>,
) -> StoreResult<Vec<UnrecordedChangeRecord>> {
    validate_identifier("project_id", project_id)?;
    if let Some(connection_internal_id) = connection_internal_id {
        validate_identifier("connection_internal_id", connection_internal_id)?;
    }
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(Vec::new());
    };
    let mut stmt = project.conn.prepare(
        "SELECT
            project_id,
            unrecorded_change_id,
            session_id,
            connection_internal_id,
            task_id,
            status,
            summary,
            observed_paths_json,
            detection_json,
            resolution_json,
            detected_at,
            resolved_at,
            resolved_by_actor_source,
            metadata_json
         FROM unrecorded_changes
        WHERE project_id = ?1
          AND status = 'unresolved'
          AND (?2 IS NULL OR connection_internal_id = ?2)
        ORDER BY detected_at, unrecorded_change_id",
    )?;
    let rows = stmt.query_map(
        params![project.project.project_id, connection_internal_id],
        unrecorded_change_from_row,
    )?;
    collect_rows(rows)
}

/// Reads compact guard-health facts for one project and Agent Connection.
pub fn guard_health_record(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    connection_internal_id: &str,
) -> StoreResult<GuardHealthRecord> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("connection_internal_id", connection_internal_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let connection = agent_connection_record(&runtime_home, connection_internal_id)?;
    let guard_installation =
        selected_guard_installation(&runtime_home, project_id, connection_internal_id)?;
    let latest_session = latest_agent_session(&runtime_home, project_id, connection_internal_id)?;
    let latest_event = latest_guard_event(&runtime_home, project_id, connection_internal_id)?;
    let unresolved_unrecorded_changes = list_unresolved_unrecorded_changes(
        &runtime_home,
        project_id,
        Some(connection_internal_id),
    )?;
    Ok(GuardHealthRecord {
        connection,
        guard_installation,
        latest_session,
        latest_event,
        unresolved_unrecorded_changes,
    })
}

/// Derives prompt-capture availability from the selected guard-health record.
pub fn prompt_capture_availability(
    record: &GuardHealthRecord,
) -> StoreResult<PromptCaptureAvailability> {
    let Some(installation) = record.guard_installation.as_ref() else {
        return Ok(PromptCaptureAvailability {
            status: PromptCaptureStatus::Unavailable,
            host_supports_prompt_capture: false,
            prompt_capture_configured: false,
            policy_hash_matches_observation: false,
        });
    };
    let facts = prompt_capture_capability_facts(&installation.host_capability_json)?;
    let policy_hash_matches_observation = installation
        .observed_policy_hash
        .as_deref()
        .zip(facts.expected_policy_hash.as_deref())
        .is_some_and(|(observed, expected)| observed == expected);
    let status = if installation.guard_mode == IntegrationProfile::Record.as_str() {
        PromptCaptureStatus::NotConfigured
    } else if !facts.host_supports_prompt_capture {
        PromptCaptureStatus::UnsupportedByHost
    } else if !facts.prompt_capture_configured || facts.prompt_capture_hook_missing {
        PromptCaptureStatus::NotConfigured
    } else if matches!(
        installation.installation_status.as_str(),
        "broken" | "stale" | "degraded"
    ) {
        PromptCaptureStatus::Degraded
    } else if installation.installation_status == GuardInstallationStatus::ReloadRequired.as_str()
        || (installation.observed_policy_hash.is_some() && !policy_hash_matches_observation)
    {
        PromptCaptureStatus::ReloadRequired
    } else if installation.installation_status == GuardInstallationStatus::Active.as_str()
        && installation.last_seen_phase.as_deref() == Some("prompt_capture")
    {
        PromptCaptureStatus::Active
    } else if installation.installation_status == GuardInstallationStatus::Active.as_str()
        && installation.last_seen_at.is_some()
    {
        PromptCaptureStatus::Observed
    } else if matches!(
        installation.installation_status.as_str(),
        "configured" | "active"
    ) {
        PromptCaptureStatus::Configured
    } else {
        PromptCaptureStatus::Unavailable
    };
    Ok(PromptCaptureAvailability {
        status,
        host_supports_prompt_capture: facts.host_supports_prompt_capture,
        prompt_capture_configured: facts.prompt_capture_configured,
        policy_hash_matches_observation,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromptCaptureCapabilityFacts {
    expected_policy_hash: Option<String>,
    host_supports_prompt_capture: bool,
    prompt_capture_configured: bool,
    prompt_capture_hook_missing: bool,
}

fn prompt_capture_capability_facts(
    host_capability_json: &str,
) -> StoreResult<PromptCaptureCapabilityFacts> {
    let value = serde_json::from_str::<Value>(host_capability_json).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("guard_installations.host_capability_json must be JSON: {error}"),
        }
    })?;
    let expected_policy_hash = value
        .get("policy_hash")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned);
    let host_supports_prompt_capture = value
        .get("host_capabilities")
        .and_then(|capabilities| capabilities.get("user_prompt_submit_hook"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let prompt_capture_configured = value
        .get("prompt_capture")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let prompt_capture_hook_missing = value
        .get("missing_required_hooks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|phase| phase == "user_prompt_submit_hook");
    Ok(PromptCaptureCapabilityFacts {
        expected_policy_hash,
        host_supports_prompt_capture,
        prompt_capture_configured,
        prompt_capture_hook_missing,
    })
}

fn selected_guard_installation(
    runtime_home: &Path,
    project_id: &str,
    connection_internal_id: &str,
) -> StoreResult<Option<GuardInstallationRecord>> {
    let mut records =
        list_guard_installations(runtime_home, connection_internal_id, Some(project_id))?;
    if records.is_empty() {
        records = list_guard_installations(runtime_home, connection_internal_id, None)?;
    }
    records.sort_by_key(|record| guard_mode_priority(&record.guard_mode));
    Ok(records.pop())
}

fn guard_mode_priority(value: &str) -> u8 {
    match value {
        "detective" => 2,
        "record" => 1,
        _ => 0,
    }
}

fn latest_agent_session(
    runtime_home: &Path,
    project_id: &str,
    connection_internal_id: &str,
) -> StoreResult<Option<AgentSessionRecord>> {
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(None);
    };
    project
        .conn
        .query_row(
            "SELECT
                project_id,
                session_id,
                connection_internal_id,
                guard_installation_id,
                host_kind,
                guard_mode,
                started_at,
                ended_at,
                metadata_json
             FROM agent_sessions
            WHERE project_id = ?1
              AND connection_internal_id = ?2
            ORDER BY started_at DESC, session_id DESC
            LIMIT 1",
            params![project.project.project_id, connection_internal_id],
            agent_session_from_row,
        )
        .optional()
        .map_err(StoreError::from)
}

fn latest_guard_event(
    runtime_home: &Path,
    project_id: &str,
    connection_internal_id: &str,
) -> StoreResult<Option<GuardEventRecord>> {
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(None);
    };
    project
        .conn
        .query_row(
            "SELECT
                project_id,
                guard_event_id,
                session_id,
                connection_internal_id,
                guard_installation_id,
                event_kind,
                decision,
                subject_json,
                result_json,
                occurred_at,
                metadata_json
             FROM guard_events
            WHERE project_id = ?1
              AND connection_internal_id = ?2
            ORDER BY occurred_at DESC, guard_event_id DESC
            LIMIT 1",
            params![project.project.project_id, connection_internal_id],
            guard_event_from_row,
        )
        .optional()
        .map_err(StoreError::from)
}

/// Resolves one unresolved unrecorded-change row.
pub fn resolve_unrecorded_change(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    unrecorded_change_id: &str,
    resolution: UnrecordedChangeResolution,
) -> StoreResult<UnrecordedChangeRecord> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("unrecorded_change_id", unrecorded_change_id)?;
    validate_unrecorded_change_resolution(&resolution)?;
    let mut project = open_project_for_required_read(runtime_home, project_id)?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    let changed = tx.execute(
        "UPDATE unrecorded_changes
            SET status = 'resolved',
                resolution_json = ?3,
                resolved_at = ?4,
                resolved_by_actor_source = ?5
          WHERE project_id = ?1
            AND unrecorded_change_id = ?2
            AND status = 'unresolved'",
        params![
            project.project.project_id,
            unrecorded_change_id,
            resolution.resolution_json,
            resolution.resolved_at,
            resolution.resolved_by_actor_source
        ],
    )?;
    tx.commit()?;
    if changed == 0 {
        let Some(existing) = unrecorded_change_from_conn(
            &project.conn,
            &project.project.project_id,
            unrecorded_change_id,
        )?
        else {
            return Err(StoreError::NotFound {
                entity: "unrecorded_change",
                id: unrecorded_change_id.to_owned(),
            });
        };
        return Err(StoreError::Conflict {
            entity: "unrecorded_change",
            id: existing.unrecorded_change_id,
            detail: "unrecorded change is already resolved".to_owned(),
        });
    }

    unrecorded_change_by_conn(
        &project.conn,
        &project.project.project_id,
        unrecorded_change_id,
    )
}

struct OpenGuardProject {
    project: ProjectRecord,
    conn: Connection,
}

fn open_guard_project(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    connection_internal_id: &str,
) -> StoreResult<OpenGuardProject> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("connection_internal_id", connection_internal_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    if !is_agent_connection_project_allowed(&runtime_home, connection_internal_id, project_id)? {
        return Err(StoreError::NotFound {
            entity: "connection_project",
            id: format!("{connection_internal_id}/{project_id}"),
        });
    }
    open_project_for_required_read(runtime_home, project_id)
}

fn open_project_for_read(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<Option<OpenGuardProject>> {
    let Some(project) = project_record_for_execution(runtime_home, project_id)? else {
        return Ok(None);
    };
    let conn = open_project_state_database(&project.state_db_path)?;
    Ok(Some(OpenGuardProject { project, conn }))
}

fn open_project_for_required_read(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<OpenGuardProject> {
    open_project_for_read(runtime_home, project_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "project",
        id: project_id.to_owned(),
    })
}

fn require_runtime_home_id(conn: &Connection) -> StoreResult<String> {
    conn.query_row(
        "SELECT runtime_home_id FROM runtime_home WHERE singleton_id = 1",
        [],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| StoreError::NotFound {
        entity: "runtime_home",
        id: "singleton".to_owned(),
    })
}

fn require_connection(conn: &Connection, connection_internal_id: &str) -> StoreResult<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM agent_connections
          WHERE connection_internal_id = ?1",
        [connection_internal_id],
        |row| row.get(0),
    )?;
    if count == 1 {
        Ok(())
    } else {
        Err(StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        })
    }
}

fn require_connection_project_membership(
    conn: &Connection,
    connection_internal_id: &str,
    project_internal_id: &str,
) -> StoreResult<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM connection_projects
          WHERE connection_internal_id = ?1
            AND project_internal_id = ?2",
        params![connection_internal_id, project_internal_id],
        |row| row.get(0),
    )?;
    if count == 1 {
        Ok(())
    } else {
        Err(StoreError::NotFound {
            entity: "connection_project",
            id: format!("{connection_internal_id}/{project_internal_id}"),
        })
    }
}

fn guard_installation_id_for_scope(
    conn: &Connection,
    connection_internal_id: &str,
    project_internal_id: Option<&str>,
    guard_mode: &str,
) -> StoreResult<Option<String>> {
    conn.query_row(
        "SELECT guard_installation_id
           FROM guard_installations
          WHERE connection_internal_id = ?1
            AND guard_mode = ?2
            AND (
                (?3 IS NULL AND project_internal_id IS NULL)
                OR project_internal_id = ?3
            )",
        params![connection_internal_id, guard_mode, project_internal_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(StoreError::from)
}

fn validate_guard_installation_upsert(input: &GuardInstallationUpsert) -> StoreResult<()> {
    validate_identifier("guard_installation_id", &input.guard_installation_id)?;
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    if let Some(project_id) = &input.project_id {
        validate_identifier("project_id", project_id)?;
    }
    validate_host_kind(&input.host_kind)?;
    validate_guard_mode(&input.guard_mode)?;
    validate_guard_installation_status(&input.installation_status)?;
    validate_json_object(
        "guard_installations.host_capability_json",
        &input.host_capability_json,
    )?;
    if let Some(installed_at) = &input.installed_at {
        validate_timestamp_text("installed_at", installed_at)?;
    }
    validate_timestamp_text("last_checked_at", &input.last_checked_at)?;
    if let Some(first_seen_at) = &input.first_seen_at {
        validate_timestamp_text("first_seen_at", first_seen_at)?;
    }
    if let Some(last_seen_at) = &input.last_seen_at {
        validate_timestamp_text("last_seen_at", last_seen_at)?;
    }
    if let Some(last_seen_phase) = &input.last_seen_phase {
        validate_guard_hook_phase("last_seen_phase", last_seen_phase)?;
    }
    if let Some(observed_host_kind) = &input.observed_host_kind {
        validate_host_kind(observed_host_kind)?;
    }
    if let Some(observed_policy_hash) = &input.observed_policy_hash {
        validate_identifier("observed_policy_hash", observed_policy_hash)?;
    }
    if let Some(observed_binary_version) = &input.observed_binary_version {
        validate_identifier("observed_binary_version", observed_binary_version)?;
    }
    validate_json_object("guard_installations.metadata_json", &input.metadata_json)
}

fn validate_guard_installation_observation(
    input: &GuardInstallationObservation,
) -> StoreResult<()> {
    validate_identifier("guard_installation_id", &input.guard_installation_id)?;
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    validate_identifier("project_id", &input.project_id)?;
    validate_host_kind(&input.host_kind)?;
    validate_guard_mode(&input.guard_mode)?;
    if input.guard_mode == IntegrationProfile::Record.as_str() {
        return Err(StoreError::InvalidInput {
            detail: "host hook observation requires detective host-hook integration profile"
                .to_owned(),
        });
    }
    validate_identifier("observed_policy_hash", &input.observed_policy_hash)?;
    if let Some(version) = &input.observed_binary_version {
        validate_identifier("observed_binary_version", version)?;
    }
    validate_guard_hook_phase("observed_phase", &input.observed_phase)?;
    validate_timestamp_text("observed_at", &input.observed_at)
}

fn validate_agent_session_insert(input: &AgentSessionInsert) -> StoreResult<()> {
    validate_identifier("session_id", &input.session_id)?;
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    if let Some(guard_installation_id) = &input.guard_installation_id {
        validate_identifier("guard_installation_id", guard_installation_id)?;
    }
    validate_host_kind(&input.host_kind)?;
    validate_guard_mode(&input.guard_mode)?;
    validate_timestamp_text("started_at", &input.started_at)?;
    validate_json_object("agent_sessions.metadata_json", &input.metadata_json)
}

fn validate_guard_event_insert(input: &GuardEventInsert) -> StoreResult<()> {
    validate_identifier("guard_event_id", &input.guard_event_id)?;
    if let Some(session_id) = &input.session_id {
        validate_identifier("session_id", session_id)?;
    }
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    if let Some(guard_installation_id) = &input.guard_installation_id {
        validate_identifier("guard_installation_id", guard_installation_id)?;
    }
    validate_identifier("event_kind", &input.event_kind)?;
    validate_guard_decision(&input.decision)?;
    validate_json_object("guard_events.subject_json", &input.subject_json)?;
    validate_json_object("guard_events.result_json", &input.result_json)?;
    validate_timestamp_text("occurred_at", &input.occurred_at)?;
    validate_json_object("guard_events.metadata_json", &input.metadata_json)
}

fn validate_prompt_capture_insert(input: &PromptCaptureInsert) -> StoreResult<()> {
    validate_identifier("prompt_capture_id", &input.prompt_capture_id)?;
    validate_identifier("session_id", &input.session_id)?;
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    validate_identifier("capture_kind", &input.capture_kind)?;
    validate_identifier("prompt_sha256", &input.prompt_sha256)?;
    if let Some(prompt_text) = &input.prompt_text {
        validate_text("prompt_text", prompt_text)?;
    }
    validate_timestamp_text("captured_at", &input.captured_at)?;
    validate_json_object("prompt_captures.metadata_json", &input.metadata_json)
}

fn validate_expected_write_insert(input: &ExpectedWriteInsert) -> StoreResult<()> {
    validate_identifier("expected_write_id", &input.expected_write_id)?;
    if let Some(session_id) = &input.session_id {
        validate_identifier("session_id", session_id)?;
    }
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    if let Some(guard_installation_id) = &input.guard_installation_id {
        validate_identifier("guard_installation_id", guard_installation_id)?;
    }
    validate_identifier("pre_tool_guard_event_id", &input.pre_tool_guard_event_id)?;
    if let Some(host_invocation_id) = &input.host_invocation_id {
        validate_identifier("host_invocation_id", host_invocation_id)?;
    }
    if let Some(tool_name) = &input.tool_name {
        validate_identifier("tool_name", tool_name)?;
    }
    validate_identifier("command_kind", &input.command_kind)?;
    validate_expected_write_path_policy(&input.path_policy)?;
    validate_json_array(
        "expected_writes.expected_paths_json",
        &input.expected_paths_json,
    )?;
    validate_identifier("task_id", &input.task_id)?;
    if let Some(change_unit_id) = &input.change_unit_id {
        validate_identifier("change_unit_id", change_unit_id)?;
    }
    validate_json_array(
        "expected_writes.write_ticket_ids_json",
        &input.write_ticket_ids_json,
    )?;
    validate_timestamp_text("created_at", &input.created_at)?;
    validate_timestamp_text("expires_at", &input.expires_at)?;
    validate_json_object("expected_writes.metadata_json", &input.metadata_json)
}

fn validate_expected_write_match(input: &ExpectedWriteMatch) -> StoreResult<()> {
    validate_identifier(
        "matched_post_tool_guard_event_id",
        &input.matched_post_tool_guard_event_id,
    )?;
    validate_json_array(
        "expected_writes.matched_paths_json",
        &input.matched_paths_json,
    )?;
    validate_timestamp_text("matched_at", &input.matched_at)
}

fn validate_expected_write_path_policy(value: &str) -> StoreResult<()> {
    if value == "exact_paths" {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "path_policy must be exact_paths".to_owned(),
        })
    }
}

fn validate_unrecorded_change_insert(input: &UnrecordedChangeInsert) -> StoreResult<()> {
    validate_identifier("unrecorded_change_id", &input.unrecorded_change_id)?;
    if let Some(session_id) = &input.session_id {
        validate_identifier("session_id", session_id)?;
    }
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    if let Some(task_id) = &input.task_id {
        validate_identifier("task_id", task_id)?;
    }
    validate_identifier("summary", &input.summary)?;
    validate_json_array(
        "unrecorded_changes.observed_paths_json",
        &input.observed_paths_json,
    )?;
    validate_json_object("unrecorded_changes.detection_json", &input.detection_json)?;
    validate_timestamp_text("detected_at", &input.detected_at)?;
    validate_json_object("unrecorded_changes.metadata_json", &input.metadata_json)
}

fn validate_unrecorded_change_resolution(
    resolution: &UnrecordedChangeResolution,
) -> StoreResult<()> {
    validate_json_object(
        "unrecorded_changes.resolution_json",
        &resolution.resolution_json,
    )?;
    validate_timestamp_text("resolved_at", &resolution.resolved_at)?;
    validate_identifier(
        "resolved_by_actor_source",
        &resolution.resolved_by_actor_source,
    )
}

fn validate_identifier(field: &'static str, value: &str) -> StoreResult<()> {
    validate_text(field, value)?;
    if value.trim().is_empty() {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}

fn validate_text(field: &'static str, value: &str) -> StoreResult<()> {
    if value.contains('\0') {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not contain NUL bytes"),
        })
    } else {
        Ok(())
    }
}

fn validate_timestamp_text(field: &'static str, value: &str) -> StoreResult<()> {
    validate_identifier(field, value)
}

fn validate_host_kind(value: &str) -> StoreResult<()> {
    HostKind::from_str(value)
        .map(|_| ())
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("host_kind is not usable: {error}"),
        })
}

fn validate_guard_mode(value: &str) -> StoreResult<()> {
    if [
        IntegrationProfile::Record.as_str(),
        IntegrationProfile::Detective.as_str(),
    ]
    .contains(&value)
    {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "integration profile must be record or detective".to_owned(),
        })
    }
}

fn validate_guard_hook_phase(field: &'static str, value: &str) -> StoreResult<()> {
    validate_identifier(field, value)?;
    if KNOWN_GUARD_OBSERVATION_PHASES.contains(&value) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!(
                "{field} must be session_start, pre_tool, post_tool, prompt_capture, or stop"
            ),
        })
    }
}

fn validate_guard_decision(value: &str) -> StoreResult<()> {
    if [
        GuardDecision::Allow.as_str(),
        GuardDecision::Deny.as_str(),
        GuardDecision::Warn.as_str(),
        GuardDecision::InjectContext.as_str(),
    ]
    .contains(&value)
    {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "decision must be allow, deny, warn, or inject_context".to_owned(),
        })
    }
}

fn validate_guard_installation_status(value: &str) -> StoreResult<()> {
    if [
        GuardInstallationStatus::Absent.as_str(),
        GuardInstallationStatus::Configured.as_str(),
        GuardInstallationStatus::ReloadRequired.as_str(),
        GuardInstallationStatus::Active.as_str(),
        GuardInstallationStatus::Degraded.as_str(),
        GuardInstallationStatus::Stale.as_str(),
        GuardInstallationStatus::Broken.as_str(),
    ]
    .contains(&value)
    {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "installation_status must be absent, configured, reload_required, active, degraded, stale, or broken".to_owned(),
        })
    }
}

fn validate_unrecorded_change_status(value: &str) -> StoreResult<()> {
    if [
        UnrecordedChangeStatus::Unresolved.as_str(),
        UnrecordedChangeStatus::Resolved.as_str(),
    ]
    .contains(&value)
    {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "unrecorded change status must be unresolved or resolved".to_owned(),
        })
    }
}

fn validate_json_object(field: &'static str, text: &str) -> StoreResult<()> {
    let value = serde_json::from_str::<Value>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be JSON object text: {error}"),
    })?;
    if value.is_object() {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be a JSON object"),
        })
    }
}

fn validate_json_array(field: &'static str, text: &str) -> StoreResult<()> {
    let value = serde_json::from_str::<Value>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be JSON array text: {error}"),
    })?;
    if value.is_array() {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be a JSON array"),
        })
    }
}

fn expected_policy_hash(host_capability_json: &str) -> StoreResult<Option<String>> {
    let value = serde_json::from_str::<Value>(host_capability_json).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("guard_installations.host_capability_json must be JSON: {error}"),
        }
    })?;
    Ok(value
        .get("policy_hash")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned))
}

fn guard_status_after_observation(installation: &GuardInstallationRecord) -> StoreResult<String> {
    if host_capability_has_missing_required_hooks(&installation.host_capability_json)? {
        return Ok(installation.installation_status.clone());
    }
    let status = match installation.installation_status.as_str() {
        "configured" | "reload_required" | "active" => GuardInstallationStatus::Active.as_str(),
        _ => installation.installation_status.as_str(),
    };
    Ok(status.to_owned())
}

fn host_capability_has_missing_required_hooks(host_capability_json: &str) -> StoreResult<bool> {
    let value = serde_json::from_str::<Value>(host_capability_json).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("guard_installations.host_capability_json must be JSON: {error}"),
        }
    })?;
    if value
        .get("missing_required_hooks")
        .and_then(Value::as_array)
        .is_some_and(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .any(|value| !value.trim().is_empty())
        })
    {
        return Ok(true);
    }
    let configured_phases = value
        .get("required_hook_phases")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(REQUIRED_GUARD_HOOK_PHASES
        .iter()
        .any(|required_phase| !configured_phases.contains(required_phase)))
}

fn validate_session_scope(
    conn: &Connection,
    project_id: &str,
    session_id: &str,
    connection_internal_id: &str,
) -> StoreResult<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM agent_sessions
          WHERE project_id = ?1
            AND session_id = ?2
            AND connection_internal_id = ?3",
        params![project_id, session_id, connection_internal_id],
        |row| row.get(0),
    )?;
    if count == 1 {
        Ok(())
    } else {
        Err(StoreError::NotFound {
            entity: "agent_session",
            id: session_id.to_owned(),
        })
    }
}

fn validate_optional_session_scope(
    conn: &Connection,
    project_id: &str,
    session_id: Option<&str>,
    connection_internal_id: &str,
) -> StoreResult<()> {
    if let Some(session_id) = session_id {
        validate_session_scope(conn, project_id, session_id, connection_internal_id)?;
    }
    Ok(())
}

fn guard_installation_from_conn(
    conn: &Connection,
    guard_installation_id: &str,
) -> StoreResult<Option<GuardInstallationRecord>> {
    conn.query_row(
        "SELECT
            gi.guard_installation_id,
            gi.runtime_home_id,
            gi.connection_internal_id,
            gi.project_internal_id,
            p.project_internal_id,
            gi.host_kind,
            gi.guard_mode,
            gi.host_capability_json,
            gi.installation_status,
            gi.installed_at,
            gi.last_checked_at,
            gi.first_seen_at,
            gi.last_seen_at,
            gi.last_seen_phase,
            gi.observed_host_kind,
            gi.observed_policy_hash,
            gi.observed_binary_version,
            gi.metadata_json,
            gi.created_at,
            gi.updated_at
         FROM guard_installations AS gi
         LEFT JOIN projects AS p
           ON p.project_internal_id = gi.project_internal_id
        WHERE gi.guard_installation_id = ?1",
        [guard_installation_id],
        guard_installation_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn guard_installation_from_row(row: &Row<'_>) -> rusqlite::Result<GuardInstallationRecord> {
    let project_internal_id = row.get::<_, Option<String>>(3)?;
    Ok(GuardInstallationRecord {
        guard_installation_id: row.get(0)?,
        runtime_home_id: row.get(1)?,
        connection_internal_id: row.get(2)?,
        project_id: row.get(4)?,
        project_internal_id,
        host_kind: row.get(5)?,
        guard_mode: row.get(6)?,
        host_capability_json: row.get(7)?,
        installation_status: row.get(8)?,
        installed_at: row.get(9)?,
        last_checked_at: row.get(10)?,
        first_seen_at: row.get(11)?,
        last_seen_at: row.get(12)?,
        last_seen_phase: row.get(13)?,
        observed_host_kind: row.get(14)?,
        observed_policy_hash: row.get(15)?,
        observed_binary_version: row.get(16)?,
        metadata_json: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

fn agent_session_from_conn(
    conn: &Connection,
    project_id: &str,
    session_id: &str,
) -> StoreResult<Option<AgentSessionRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            session_id,
            connection_internal_id,
            guard_installation_id,
            host_kind,
            guard_mode,
            started_at,
            ended_at,
            metadata_json
         FROM agent_sessions
        WHERE project_id = ?1
          AND session_id = ?2",
        params![project_id, session_id],
        agent_session_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn agent_session_by_conn(
    conn: &Connection,
    project_id: &str,
    session_id: &str,
) -> StoreResult<AgentSessionRecord> {
    agent_session_from_conn(conn, project_id, session_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "agent_session",
        id: session_id.to_owned(),
    })
}

fn agent_session_from_row(row: &Row<'_>) -> rusqlite::Result<AgentSessionRecord> {
    Ok(AgentSessionRecord {
        project_id: row.get(0)?,
        session_id: row.get(1)?,
        connection_internal_id: row.get(2)?,
        guard_installation_id: row.get(3)?,
        host_kind: row.get(4)?,
        guard_mode: row.get(5)?,
        started_at: row.get(6)?,
        ended_at: row.get(7)?,
        metadata_json: row.get(8)?,
    })
}

fn guard_event_from_conn(
    conn: &Connection,
    project_id: &str,
    guard_event_id: &str,
) -> StoreResult<Option<GuardEventRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            guard_event_id,
            session_id,
            connection_internal_id,
            guard_installation_id,
            event_kind,
            decision,
            subject_json,
            result_json,
            occurred_at,
            metadata_json
         FROM guard_events
        WHERE project_id = ?1
          AND guard_event_id = ?2",
        params![project_id, guard_event_id],
        guard_event_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn guard_event_by_conn(
    conn: &Connection,
    project_id: &str,
    guard_event_id: &str,
) -> StoreResult<GuardEventRecord> {
    guard_event_from_conn(conn, project_id, guard_event_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "guard_event",
        id: guard_event_id.to_owned(),
    })
}

fn guard_event_from_row(row: &Row<'_>) -> rusqlite::Result<GuardEventRecord> {
    Ok(GuardEventRecord {
        project_id: row.get(0)?,
        guard_event_id: row.get(1)?,
        session_id: row.get(2)?,
        connection_internal_id: row.get(3)?,
        guard_installation_id: row.get(4)?,
        event_kind: row.get(5)?,
        decision: row.get(6)?,
        subject_json: row.get(7)?,
        result_json: row.get(8)?,
        occurred_at: row.get(9)?,
        metadata_json: row.get(10)?,
    })
}

fn prompt_capture_from_conn(
    conn: &Connection,
    project_id: &str,
    prompt_capture_id: &str,
) -> StoreResult<Option<PromptCaptureRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            prompt_capture_id,
            session_id,
            connection_internal_id,
            capture_kind,
            prompt_sha256,
            prompt_text,
            captured_at,
            metadata_json
         FROM prompt_captures
        WHERE project_id = ?1
          AND prompt_capture_id = ?2",
        params![project_id, prompt_capture_id],
        prompt_capture_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn prompt_capture_by_conn(
    conn: &Connection,
    project_id: &str,
    prompt_capture_id: &str,
) -> StoreResult<PromptCaptureRecord> {
    prompt_capture_from_conn(conn, project_id, prompt_capture_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "prompt_capture",
            id: prompt_capture_id.to_owned(),
        }
    })
}

fn prompt_capture_from_row(row: &Row<'_>) -> rusqlite::Result<PromptCaptureRecord> {
    Ok(PromptCaptureRecord {
        project_id: row.get(0)?,
        prompt_capture_id: row.get(1)?,
        session_id: row.get(2)?,
        connection_internal_id: row.get(3)?,
        capture_kind: row.get(4)?,
        prompt_sha256: row.get(5)?,
        prompt_text: row.get(6)?,
        captured_at: row.get(7)?,
        metadata_json: row.get(8)?,
    })
}

fn expected_write_from_conn(
    conn: &Connection,
    project_id: &str,
    expected_write_id: &str,
) -> StoreResult<Option<ExpectedWriteRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            expected_write_id,
            session_id,
            connection_internal_id,
            guard_installation_id,
            pre_tool_guard_event_id,
            host_invocation_id,
            tool_name,
            command_kind,
            path_policy,
            expected_paths_json,
            task_id,
            change_unit_id,
            write_ticket_ids_json,
            basis_state_version,
            status,
            matched_post_tool_guard_event_id,
            matched_paths_json,
            created_at,
            expires_at,
            matched_at,
            metadata_json
         FROM expected_writes
        WHERE project_id = ?1
          AND expected_write_id = ?2",
        params![project_id, expected_write_id],
        expected_write_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn expected_write_by_conn(
    conn: &Connection,
    project_id: &str,
    expected_write_id: &str,
) -> StoreResult<ExpectedWriteRecord> {
    expected_write_from_conn(conn, project_id, expected_write_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "expected_write",
            id: expected_write_id.to_owned(),
        }
    })
}

fn expected_write_from_row(row: &Row<'_>) -> rusqlite::Result<ExpectedWriteRecord> {
    Ok(ExpectedWriteRecord {
        project_id: row.get(0)?,
        expected_write_id: row.get(1)?,
        session_id: row.get(2)?,
        connection_internal_id: row.get(3)?,
        guard_installation_id: row.get(4)?,
        pre_tool_guard_event_id: row.get(5)?,
        host_invocation_id: row.get(6)?,
        tool_name: row.get(7)?,
        command_kind: row.get(8)?,
        path_policy: row.get(9)?,
        expected_paths_json: row.get(10)?,
        task_id: row.get(11)?,
        change_unit_id: row.get(12)?,
        write_ticket_ids_json: row.get(13)?,
        basis_state_version: row.get(14)?,
        status: row.get(15)?,
        matched_post_tool_guard_event_id: row.get(16)?,
        matched_paths_json: row.get(17)?,
        created_at: row.get(18)?,
        expires_at: row.get(19)?,
        matched_at: row.get(20)?,
        metadata_json: row.get(21)?,
    })
}

fn unrecorded_change_from_conn(
    conn: &Connection,
    project_id: &str,
    unrecorded_change_id: &str,
) -> StoreResult<Option<UnrecordedChangeRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            unrecorded_change_id,
            session_id,
            connection_internal_id,
            task_id,
            status,
            summary,
            observed_paths_json,
            detection_json,
            resolution_json,
            detected_at,
            resolved_at,
            resolved_by_actor_source,
            metadata_json
         FROM unrecorded_changes
        WHERE project_id = ?1
          AND unrecorded_change_id = ?2",
        params![project_id, unrecorded_change_id],
        unrecorded_change_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn unrecorded_change_by_conn(
    conn: &Connection,
    project_id: &str,
    unrecorded_change_id: &str,
) -> StoreResult<UnrecordedChangeRecord> {
    let record =
        unrecorded_change_from_conn(conn, project_id, unrecorded_change_id)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "unrecorded_change",
                id: unrecorded_change_id.to_owned(),
            }
        })?;
    validate_unrecorded_change_status(&record.status)?;
    Ok(record)
}

fn unrecorded_change_from_row(row: &Row<'_>) -> rusqlite::Result<UnrecordedChangeRecord> {
    Ok(UnrecordedChangeRecord {
        project_id: row.get(0)?,
        unrecorded_change_id: row.get(1)?,
        session_id: row.get(2)?,
        connection_internal_id: row.get(3)?,
        task_id: row.get(4)?,
        status: row.get(5)?,
        summary: row.get(6)?,
        observed_paths_json: row.get(7)?,
        detection_json: row.get(8)?,
        resolution_json: row.get(9)?,
        detected_at: row.get(10)?,
        resolved_at: row.get(11)?,
        resolved_by_actor_source: row.get(12)?,
        metadata_json: row.get(13)?,
    })
}

fn collect_rows<T, F>(rows: rusqlite::MappedRows<'_, F>) -> StoreResult<Vec<T>>
where
    F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
{
    let mut values = Vec::new();
    for row in rows {
        values.push(row?);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use volicord_test_support::TempRuntimeHome;

    use super::*;
    use crate::{
        agent_connections::{
            add_connection_project, ensure_agent_connection, AgentConnectionRegistration,
            ConnectionProjectRegistration, CONNECTION_INTENT_SHARED, CONNECTION_MODE_WORKFLOW,
            HOST_KIND_CODEX, HOST_SCOPE_PROJECT, VERIFIED_STATUS_COMPLETE,
        },
        bootstrap::{
            initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
        },
    };

    #[test]
    fn guard_records_round_trip_and_unrecorded_changes_resolve() -> Result<(), Box<dyn Error>> {
        let fixture = GuardFixture::new("guard-round-trip")?;
        fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;

        let installation = upsert_guard_installation(
            fixture.runtime_home.path(),
            GuardInstallationUpsert {
                guard_installation_id: "guard_installation_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: Some("project_guard_a".to_owned()),
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                host_capability_json: r#"{"prompt_capture":true}"#.to_owned(),
                installation_status: "active".to_owned(),
                installed_at: Some("2026-06-30T00:00:00Z".to_owned()),
                last_checked_at: "2026-06-30T00:01:00Z".to_owned(),
                first_seen_at: Some("2026-06-30T00:01:00Z".to_owned()),
                last_seen_at: Some("2026-06-30T00:01:00Z".to_owned()),
                last_seen_phase: Some("session_start".to_owned()),
                observed_host_kind: Some("codex".to_owned()),
                observed_policy_hash: Some("sha256:test".to_owned()),
                observed_binary_version: Some("test".to_owned()),
                metadata_json: "{}".to_owned(),
            },
        )?;
        assert_eq!(installation.project_id.as_deref(), Some("project_guard_a"));
        assert_eq!(installation.guard_mode, "detective");

        let session = insert_agent_session(
            fixture.runtime_home.path(),
            "project_guard_a",
            AgentSessionInsert {
                session_id: "session_guard_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: Some("guard_installation_a".to_owned()),
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                started_at: "2026-06-30T00:02:00Z".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        assert_eq!(session.session_id, "session_guard_a");

        let event = insert_guard_event(
            fixture.runtime_home.path(),
            "project_guard_a",
            GuardEventInsert {
                guard_event_id: "guard_event_a".to_owned(),
                session_id: Some("session_guard_a".to_owned()),
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: Some("guard_installation_a".to_owned()),
                event_kind: "write_attempt".to_owned(),
                decision: "warn".to_owned(),
                subject_json: r#"{"path":"src/lib.rs"}"#.to_owned(),
                result_json: r#"{"message":"record context first"}"#.to_owned(),
                occurred_at: "2026-06-30T00:03:00Z".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        assert_eq!(event.decision, "warn");

        let capture = insert_prompt_capture(
            fixture.runtime_home.path(),
            "project_guard_a",
            PromptCaptureInsert {
                prompt_capture_id: "prompt_capture_a".to_owned(),
                session_id: "session_guard_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                capture_kind: "user_prompt".to_owned(),
                prompt_sha256: "sha256:abc123".to_owned(),
                prompt_text: Some("Please update the guard model.".to_owned()),
                captured_at: "2026-06-30T00:04:00Z".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        assert_eq!(
            capture.prompt_text.as_deref(),
            Some("Please update the guard model.")
        );

        fixture.insert_task("project_guard_a", "task_guard_a")?;
        let expected = insert_expected_write(
            fixture.runtime_home.path(),
            "project_guard_a",
            ExpectedWriteInsert {
                expected_write_id: "expected_write_a".to_owned(),
                session_id: Some("session_guard_a".to_owned()),
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: Some("guard_installation_a".to_owned()),
                pre_tool_guard_event_id: "guard_event_a".to_owned(),
                host_invocation_id: Some("tool_call_a".to_owned()),
                tool_name: Some("shell".to_owned()),
                command_kind: "mutating".to_owned(),
                path_policy: "exact_paths".to_owned(),
                expected_paths_json: r#"["src/lib.rs"]"#.to_owned(),
                task_id: "task_guard_a".to_owned(),
                change_unit_id: Some("change_unit_guard_a".to_owned()),
                write_ticket_ids_json: r#"["write_ticket_a"]"#.to_owned(),
                basis_state_version: 1,
                created_at: "2026-06-30T00:04:30Z".to_owned(),
                expires_at: "2026-06-30T00:19:30Z".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        assert_eq!(expected.status, "pending");
        assert_eq!(
            list_pending_expected_writes(
                fixture.runtime_home.path(),
                "project_guard_a",
                "conn_guard_a",
            )?
            .len(),
            1
        );
        let matched = mark_expected_write_matched(
            fixture.runtime_home.path(),
            "project_guard_a",
            "expected_write_a",
            ExpectedWriteMatch {
                matched_post_tool_guard_event_id: "guard_event_post_a".to_owned(),
                matched_paths_json: r#"["src/lib.rs"]"#.to_owned(),
                matched_at: "2026-06-30T00:05:00Z".to_owned(),
            },
        )?;
        assert_eq!(matched.status, "matched");
        assert!(list_pending_expected_writes(
            fixture.runtime_home.path(),
            "project_guard_a",
            "conn_guard_a",
        )?
        .is_empty());

        let change = insert_unrecorded_change(
            fixture.runtime_home.path(),
            "project_guard_a",
            UnrecordedChangeInsert {
                unrecorded_change_id: "unrecorded_change_a".to_owned(),
                session_id: Some("session_guard_a".to_owned()),
                connection_internal_id: "conn_guard_a".to_owned(),
                task_id: None,
                summary: "Product file changed without a matching Core run".to_owned(),
                observed_paths_json: r#"["src/lib.rs"]"#.to_owned(),
                detection_json: r#"{"source":"guard"}"#.to_owned(),
                detected_at: "2026-06-30T00:05:00Z".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        assert_eq!(change.status, "unresolved");

        assert_eq!(
            list_unresolved_unrecorded_changes(
                fixture.runtime_home.path(),
                "project_guard_a",
                Some("conn_guard_a"),
            )?
            .len(),
            1
        );

        let resolved = resolve_unrecorded_change(
            fixture.runtime_home.path(),
            "project_guard_a",
            "unrecorded_change_a",
            UnrecordedChangeResolution {
                resolution_json: r#"{"recorded_run_id":"run_guard_a"}"#.to_owned(),
                resolved_at: "2026-06-30T00:06:00Z".to_owned(),
                resolved_by_actor_source: "agent_connection:conn_guard_a".to_owned(),
            },
        )?;
        assert_eq!(resolved.status, "resolved");
        assert!(resolved.resolution_json.is_some());
        assert!(list_unresolved_unrecorded_changes(
            fixture.runtime_home.path(),
            "project_guard_a",
            Some("conn_guard_a"),
        )?
        .is_empty());

        let ended = end_agent_session(
            fixture.runtime_home.path(),
            "project_guard_a",
            "session_guard_a",
            "2026-06-30T00:07:00Z",
        )?;
        assert_eq!(ended.ended_at.as_deref(), Some("2026-06-30T00:07:00Z"));
        Ok(())
    }

    #[test]
    fn guard_records_are_project_and_connection_scoped() -> Result<(), Box<dyn Error>> {
        let fixture = GuardFixture::new("guard-scope")?;
        fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;
        fixture.add_project_connection("project_guard_b", "conn_guard_b", "repo-b")?;

        insert_agent_session(
            fixture.runtime_home.path(),
            "project_guard_a",
            AgentSessionInsert {
                session_id: "session_guard_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: None,
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                started_at: "2026-06-30T01:00:00Z".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        insert_unrecorded_change(
            fixture.runtime_home.path(),
            "project_guard_a",
            UnrecordedChangeInsert {
                unrecorded_change_id: "unrecorded_change_a".to_owned(),
                session_id: Some("session_guard_a".to_owned()),
                connection_internal_id: "conn_guard_a".to_owned(),
                task_id: None,
                summary: "Unrecorded change in project A".to_owned(),
                observed_paths_json: r#"["a.txt"]"#.to_owned(),
                detection_json: "{}".to_owned(),
                detected_at: "2026-06-30T01:01:00Z".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;

        assert!(agent_session(
            fixture.runtime_home.path(),
            "project_guard_b",
            "session_guard_a",
        )?
        .is_none());
        assert!(list_unresolved_unrecorded_changes(
            fixture.runtime_home.path(),
            "project_guard_a",
            Some("conn_guard_b"),
        )?
        .is_empty());

        let error = insert_guard_event(
            fixture.runtime_home.path(),
            "project_guard_b",
            GuardEventInsert {
                guard_event_id: "guard_event_cross".to_owned(),
                session_id: None,
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: None,
                event_kind: "cross_project_attempt".to_owned(),
                decision: "deny".to_owned(),
                subject_json: "{}".to_owned(),
                result_json: "{}".to_owned(),
                occurred_at: "2026-06-30T01:02:00Z".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("connection from project A must not write guard events into project B");
        assert!(matches!(
            error,
            StoreError::NotFound {
                entity: "connection_project",
                ..
            }
        ));

        let error = upsert_guard_installation(
            fixture.runtime_home.path(),
            GuardInstallationUpsert {
                guard_installation_id: "guard_installation_cross".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: Some("project_guard_b".to_owned()),
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                host_capability_json: "{}".to_owned(),
                installation_status: "active".to_owned(),
                installed_at: None,
                last_checked_at: "2026-06-30T01:03:00Z".to_owned(),
                first_seen_at: None,
                last_seen_at: None,
                last_seen_phase: None,
                observed_host_kind: None,
                observed_policy_hash: None,
                observed_binary_version: None,
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("connection from project A must not write project-B installation scope");
        assert!(matches!(
            error,
            StoreError::NotFound {
                entity: "connection_project",
                ..
            }
        ));

        Ok(())
    }

    #[test]
    fn guard_installation_observation_promotes_active() -> Result<(), Box<dyn Error>> {
        let fixture = GuardFixture::new("guard-detective-active")?;
        fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;
        fixture.upsert_observable_installation("guard_installation_a", "conn_guard_a")?;

        let observed = observe_guard_installation(
            fixture.runtime_home.path(),
            GuardInstallationObservation {
                guard_installation_id: "guard_installation_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: "project_guard_a".to_owned(),
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                observed_policy_hash: "sha256:policy-a".to_owned(),
                observed_binary_version: Some("1.2.3".to_owned()),
                observed_phase: "session_start".to_owned(),
                observed_at: "2026-06-30T02:00:00Z".to_owned(),
            },
        )?
        .expect("matching observation should promote installation");

        assert_eq!(observed.installation_status, "active");
        assert_eq!(
            observed.first_seen_at.as_deref(),
            Some("2026-06-30T02:00:00Z")
        );
        assert_eq!(
            observed.last_seen_at.as_deref(),
            Some("2026-06-30T02:00:00Z")
        );
        assert_eq!(observed.last_seen_phase.as_deref(), Some("session_start"));
        assert_eq!(observed.observed_host_kind.as_deref(), Some("codex"));
        assert_eq!(
            observed.observed_policy_hash.as_deref(),
            Some("sha256:policy-a")
        );
        assert_eq!(observed.observed_binary_version.as_deref(), Some("1.2.3"));

        let observed_again = observe_guard_installation(
            fixture.runtime_home.path(),
            GuardInstallationObservation {
                guard_installation_id: "guard_installation_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: "project_guard_a".to_owned(),
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                observed_policy_hash: "sha256:policy-a".to_owned(),
                observed_binary_version: Some("1.2.4".to_owned()),
                observed_phase: "pre_tool".to_owned(),
                observed_at: "2026-06-30T02:05:00Z".to_owned(),
            },
        )?
        .expect("later matching observation should update last-seen metadata");
        assert_eq!(
            observed_again.first_seen_at.as_deref(),
            Some("2026-06-30T02:00:00Z")
        );
        assert_eq!(
            observed_again.last_seen_at.as_deref(),
            Some("2026-06-30T02:05:00Z")
        );
        assert_eq!(observed_again.last_seen_phase.as_deref(), Some("pre_tool"));
        assert_eq!(
            observed_again.observed_binary_version.as_deref(),
            Some("1.2.4")
        );

        let idempotent_upsert = upsert_guard_installation(
            fixture.runtime_home.path(),
            GuardInstallationUpsert {
                guard_installation_id: "guard_installation_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: Some("project_guard_a".to_owned()),
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                host_capability_json: r#"{"policy_hash":"sha256:policy-a","required_hook_phases":["session_start_hook","pre_tool_hook","post_tool_hook","user_prompt_submit_hook","stop_hook"],"missing_required_hooks":[],"prompt_capture":true}"#.to_owned(),
                installation_status: "configured".to_owned(),
                installed_at: Some("2026-06-30T02:06:00Z".to_owned()),
                last_checked_at: "2026-06-30T02:06:00Z".to_owned(),
                first_seen_at: None,
                last_seen_at: None,
                last_seen_phase: None,
                observed_host_kind: None,
                observed_policy_hash: None,
                observed_binary_version: None,
                metadata_json: "{}".to_owned(),
            },
        )?;
        assert_eq!(idempotent_upsert.installation_status, "active");
        assert_eq!(
            idempotent_upsert.last_seen_at.as_deref(),
            Some("2026-06-30T02:05:00Z")
        );
        Ok(())
    }

    #[test]
    fn guard_installation_observation_records_metadata_without_promoting_degraded(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = GuardFixture::new("guard-detective-degraded")?;
        fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;
        upsert_guard_installation(
            fixture.runtime_home.path(),
            GuardInstallationUpsert {
                guard_installation_id: "guard_installation_degraded".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: Some("project_guard_a".to_owned()),
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                host_capability_json: r#"{"policy_hash":"sha256:policy-a","required_hook_phases":["session_start_hook","pre_tool_hook"],"missing_required_hooks":["pre_tool_hook"],"prompt_capture":true}"#.to_owned(),
                installation_status: "degraded".to_owned(),
                installed_at: Some("2026-06-30T01:59:00Z".to_owned()),
                last_checked_at: "2026-06-30T01:59:00Z".to_owned(),
                first_seen_at: None,
                last_seen_at: None,
                last_seen_phase: None,
                observed_host_kind: None,
                observed_policy_hash: None,
                observed_binary_version: None,
                metadata_json: "{}".to_owned(),
            },
        )?;

        let observed = observe_guard_installation(
            fixture.runtime_home.path(),
            GuardInstallationObservation {
                guard_installation_id: "guard_installation_degraded".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: "project_guard_a".to_owned(),
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                observed_policy_hash: "sha256:policy-a".to_owned(),
                observed_binary_version: Some("1.2.3".to_owned()),
                observed_phase: "session_start".to_owned(),
                observed_at: "2026-06-30T02:00:00Z".to_owned(),
            },
        )?
        .expect("matching degraded observation should be recorded");

        assert_eq!(observed.installation_status, "degraded");
        assert_eq!(
            observed.first_seen_at.as_deref(),
            Some("2026-06-30T02:00:00Z")
        );
        assert_eq!(
            observed.last_seen_at.as_deref(),
            Some("2026-06-30T02:00:00Z")
        );
        assert_eq!(observed.last_seen_phase.as_deref(), Some("session_start"));
        assert_eq!(
            observed.observed_policy_hash.as_deref(),
            Some("sha256:policy-a")
        );
        Ok(())
    }

    #[test]
    fn guard_installation_observation_does_not_promote_partial_required_phase_configuration(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = GuardFixture::new("guard-detective-partial-hooks")?;
        fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;
        upsert_guard_installation(
            fixture.runtime_home.path(),
            GuardInstallationUpsert {
                guard_installation_id: "guard_installation_partial".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: Some("project_guard_a".to_owned()),
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                host_capability_json: r#"{"policy_hash":"sha256:policy-a","required_hook_phases":["session_start_hook"],"missing_required_hooks":[],"prompt_capture":true}"#.to_owned(),
                installation_status: "configured".to_owned(),
                installed_at: Some("2026-06-30T01:59:00Z".to_owned()),
                last_checked_at: "2026-06-30T01:59:00Z".to_owned(),
                first_seen_at: None,
                last_seen_at: None,
                last_seen_phase: None,
                observed_host_kind: None,
                observed_policy_hash: None,
                observed_binary_version: None,
                metadata_json: "{}".to_owned(),
            },
        )?;

        let observed = observe_guard_installation(
            fixture.runtime_home.path(),
            GuardInstallationObservation {
                guard_installation_id: "guard_installation_partial".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: "project_guard_a".to_owned(),
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                observed_policy_hash: "sha256:policy-a".to_owned(),
                observed_binary_version: Some("1.2.3".to_owned()),
                observed_phase: "session_start".to_owned(),
                observed_at: "2026-06-30T02:00:00Z".to_owned(),
            },
        )?
        .expect("matching partial observation should be recorded");

        assert_eq!(observed.installation_status, "configured");
        assert_eq!(
            observed.last_seen_at.as_deref(),
            Some("2026-06-30T02:00:00Z")
        );
        assert_eq!(observed.last_seen_phase.as_deref(), Some("session_start"));
        Ok(())
    }

    #[test]
    fn guard_installation_observation_rejects_mismatched_identity_or_policy(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = GuardFixture::new("guard-detective-invalid")?;
        fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;
        fixture.add_project_connection("project_guard_b", "conn_guard_b", "repo-b")?;
        fixture.add_connection_to_existing_project("project_guard_a", "conn_guard_other")?;
        fixture.upsert_observable_installation("guard_installation_a", "conn_guard_a")?;

        for observation in [
            GuardInstallationObservation {
                guard_installation_id: "guard_installation_a".to_owned(),
                connection_internal_id: "conn_guard_other".to_owned(),
                project_id: "project_guard_a".to_owned(),
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                observed_policy_hash: "sha256:policy-a".to_owned(),
                observed_binary_version: Some("1.2.3".to_owned()),
                observed_phase: "session_start".to_owned(),
                observed_at: "2026-06-30T03:00:00Z".to_owned(),
            },
            GuardInstallationObservation {
                guard_installation_id: "guard_installation_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: "project_guard_b".to_owned(),
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                observed_policy_hash: "sha256:policy-a".to_owned(),
                observed_binary_version: Some("1.2.3".to_owned()),
                observed_phase: "pre_tool".to_owned(),
                observed_at: "2026-06-30T03:01:00Z".to_owned(),
            },
            GuardInstallationObservation {
                guard_installation_id: "guard_installation_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: "project_guard_a".to_owned(),
                host_kind: "claude_code".to_owned(),
                guard_mode: "detective".to_owned(),
                observed_policy_hash: "sha256:policy-a".to_owned(),
                observed_binary_version: Some("1.2.3".to_owned()),
                observed_phase: "post_tool".to_owned(),
                observed_at: "2026-06-30T03:02:00Z".to_owned(),
            },
            GuardInstallationObservation {
                guard_installation_id: "guard_installation_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: "project_guard_a".to_owned(),
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                observed_policy_hash: "sha256:other-policy".to_owned(),
                observed_binary_version: Some("1.2.3".to_owned()),
                observed_phase: "stop".to_owned(),
                observed_at: "2026-06-30T03:03:00Z".to_owned(),
            },
        ] {
            assert!(
                observe_guard_installation(fixture.runtime_home.path(), observation)?.is_none(),
                "mismatched observation must not promote installation"
            );
        }

        let stored = guard_installation(fixture.runtime_home.path(), "guard_installation_a")?
            .expect("installation should remain stored");
        assert_eq!(stored.installation_status, "reload_required");
        assert!(stored.first_seen_at.is_none());
        assert!(stored.last_seen_at.is_none());
        assert!(stored.last_seen_phase.is_none());
        assert!(stored.observed_policy_hash.is_none());
        Ok(())
    }

    #[test]
    fn guard_installation_observation_rejects_unknown_phase() -> Result<(), Box<dyn Error>> {
        let fixture = GuardFixture::new("guard-detective-unknown-phase")?;
        fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;
        fixture.upsert_observable_installation("guard_installation_a", "conn_guard_a")?;

        let error = observe_guard_installation(
            fixture.runtime_home.path(),
            GuardInstallationObservation {
                guard_installation_id: "guard_installation_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: "project_guard_a".to_owned(),
                host_kind: "codex".to_owned(),
                guard_mode: "detective".to_owned(),
                observed_policy_hash: "sha256:policy-a".to_owned(),
                observed_binary_version: Some("1.2.3".to_owned()),
                observed_phase: "unknown_phase".to_owned(),
                observed_at: "2026-06-30T03:30:00Z".to_owned(),
            },
        )
        .expect_err("unknown observation phase must be rejected");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        Ok(())
    }

    struct GuardFixture {
        runtime_home: TempRuntimeHome,
    }

    impl GuardFixture {
        fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
            let runtime_home = TempRuntimeHome::new(prefix)?;
            initialize_runtime_home(runtime_home.path(), &format!("runtime_home_{prefix}"), "{}")?;
            Ok(Self { runtime_home })
        }

        fn add_project_connection(
            &self,
            project_id: &str,
            connection_id: &str,
            repo_name: &str,
        ) -> Result<(), Box<dyn Error>> {
            let repo_root = self.runtime_home.create_product_repo(repo_name)?;
            register_project(
                self.runtime_home.path(),
                ProjectRegistration {
                    project_id: project_id.to_owned(),
                    repo_root,
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            ensure_agent_connection(
                self.runtime_home.path(),
                AgentConnectionRegistration {
                    connection_internal_id: connection_id.to_owned(),
                    host_kind: HOST_KIND_CODEX.to_owned(),
                    intent: CONNECTION_INTENT_SHARED.to_owned(),
                    host_scope: HOST_SCOPE_PROJECT.to_owned(),
                    server_name: format!("volicord-{connection_id}"),
                    config_target: self
                        .runtime_home
                        .path()
                        .join("agent-connections")
                        .join(connection_id)
                        .to_string_lossy()
                        .into_owned(),
                    mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                    enabled: true,
                    managed_fingerprint: format!("fingerprint:{connection_id}"),
                    last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
                    last_verification_report_json: "{}".to_owned(),
                    last_user_actions_json: "[]".to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            add_connection_project(
                self.runtime_home.path(),
                ConnectionProjectRegistration {
                    connection_internal_id: connection_id.to_owned(),
                    project_id: project_id.to_owned(),
                },
            )?;
            Ok(())
        }

        fn insert_task(&self, project_id: &str, task_id: &str) -> Result<(), Box<dyn Error>> {
            let project = project_record_for_execution(self.runtime_home.path(), project_id)?
                .expect("project should be registered");
            let conn = open_project_state_database(&project.state_db_path)?;
            conn.execute(
                "INSERT INTO tasks (
                    project_id,
                    task_id,
                    created_by_actor_source,
                    mode,
                    lifecycle_phase,
                    created_at,
                    updated_at
                )
                VALUES (?1, ?2, 'agent_connection:conn_guard_a', 'work', 'shaping', 't0', 't0')",
                params![project_id, task_id],
            )?;
            Ok(())
        }

        fn upsert_observable_installation(
            &self,
            guard_installation_id: &str,
            connection_id: &str,
        ) -> Result<(), Box<dyn Error>> {
            upsert_guard_installation(
                self.runtime_home.path(),
                GuardInstallationUpsert {
                    guard_installation_id: guard_installation_id.to_owned(),
                    connection_internal_id: connection_id.to_owned(),
                    project_id: Some("project_guard_a".to_owned()),
                    host_kind: "codex".to_owned(),
                    guard_mode: "detective".to_owned(),
                    host_capability_json: r#"{"policy_hash":"sha256:policy-a","required_hook_phases":["session_start_hook","pre_tool_hook","post_tool_hook","user_prompt_submit_hook","stop_hook"],"missing_required_hooks":[],"prompt_capture":true}"#.to_owned(),
                    installation_status: "reload_required".to_owned(),
                    installed_at: Some("2026-06-30T01:59:00Z".to_owned()),
                    last_checked_at: "2026-06-30T01:59:00Z".to_owned(),
                    first_seen_at: None,
                    last_seen_at: None,
                    last_seen_phase: None,
                    observed_host_kind: None,
                    observed_policy_hash: None,
                    observed_binary_version: None,
                    metadata_json: "{}".to_owned(),
                },
            )?;
            Ok(())
        }

        fn add_connection_to_existing_project(
            &self,
            project_id: &str,
            connection_id: &str,
        ) -> Result<(), Box<dyn Error>> {
            ensure_agent_connection(
                self.runtime_home.path(),
                AgentConnectionRegistration {
                    connection_internal_id: connection_id.to_owned(),
                    host_kind: HOST_KIND_CODEX.to_owned(),
                    intent: CONNECTION_INTENT_SHARED.to_owned(),
                    host_scope: HOST_SCOPE_PROJECT.to_owned(),
                    server_name: format!("volicord-{connection_id}"),
                    config_target: self
                        .runtime_home
                        .path()
                        .join("agent-connections")
                        .join(connection_id)
                        .to_string_lossy()
                        .into_owned(),
                    mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                    enabled: true,
                    managed_fingerprint: format!("fingerprint:{connection_id}"),
                    last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
                    last_verification_report_json: "{}".to_owned(),
                    last_user_actions_json: "[]".to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            add_connection_project(
                self.runtime_home.path(),
                ConnectionProjectRegistration {
                    connection_internal_id: connection_id.to_owned(),
                    project_id: project_id.to_owned(),
                },
            )?;
            Ok(())
        }
    }
}
