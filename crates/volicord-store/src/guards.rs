use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Deserialize;
use serde_json::Value;
use volicord_platform_fs::resolve_git_worktree_layout;
use volicord_types::{
    host_hook_capability_has_exact_current_shape, host_hook_capability_matches_owner_binding,
    GuardDecision, GuardInstallationStatus, HostHookCapabilityOwnerBinding, HostKind,
    IntegrationProfile, PromptCaptureStatus, UnrecordedChangeStatus, UtcTimestamp,
    HOST_HOOK_CAPABILITY_SCHEMA,
};

use crate::{
    agent_connections::{
        agent_connection_record_read_only, is_agent_connection_project_allowed,
        AgentConnectionRecord,
    },
    bootstrap::{
        project_record_for_execution, project_record_for_execution_read_only,
        raw_project_record_from_conn, ProjectRecord,
    },
    sqlite::{
        begin_immediate_transaction, open_project_state_database,
        open_project_state_database_read_only, open_registry_database,
        open_registry_database_read_only, registry_db_path,
    },
    StoreError, StoreResult,
};

const KNOWN_GUARD_OBSERVATION_PHASES: &[&str] = &["pre_tool", "post_tool", "prompt_capture"];

/// Maximum prior post-tool Guard events considered for one exact correlation window.
pub const POST_TOOL_CORRELATION_EVENT_LIMIT: usize = 512;

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

/// Stored guard-observation facts evaluated against the current installation capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardObservationMatch<'a> {
    pub guard_installation_id: &'a str,
    pub host_kind: &'a str,
    pub host_capability_json: &'a str,
    pub last_seen_at: Option<&'a str>,
    pub last_seen_phase: Option<&'a str>,
    pub observed_host_kind: Option<&'a str>,
    pub observed_policy_hash: Option<&'a str>,
}

impl<'a> From<&'a GuardInstallationRecord> for GuardObservationMatch<'a> {
    fn from(installation: &'a GuardInstallationRecord) -> Self {
        Self {
            guard_installation_id: &installation.guard_installation_id,
            host_kind: &installation.host_kind,
            host_capability_json: &installation.host_capability_json,
            last_seen_at: installation.last_seen_at.as_deref(),
            last_seen_phase: installation.last_seen_phase.as_deref(),
            observed_host_kind: installation.observed_host_kind.as_deref(),
            observed_policy_hash: installation.observed_policy_hash.as_deref(),
        }
    }
}

/// Returns whether persisted observation metadata matches the exact canonical capability.
///
/// Invalid timestamps, stale host or policy identity, unknown lifecycle phases, and phases not
/// configured by the current capability all fail closed.
pub fn guard_observation_matches_current_capability(
    observation: GuardObservationMatch<'_>,
) -> StoreResult<bool> {
    let GuardObservationMatch {
        guard_installation_id,
        host_kind,
        host_capability_json,
        last_seen_at,
        last_seen_phase,
        observed_host_kind,
        observed_policy_hash,
    } = observation;
    let Some(last_seen_at) = last_seen_at else {
        return Ok(false);
    };
    UtcTimestamp::from_str(last_seen_at).map_err(|_| {
        StoreError::corrupt_owner_state_json(
            "guard_installations",
            guard_installation_id.to_owned(),
            "last_seen_at",
        )
    })?;
    if observed_host_kind != Some(host_kind) {
        return Ok(false);
    }
    let capability = serde_json::from_str::<Value>(host_capability_json).map_err(|_| {
        StoreError::corrupt_owner_state_json(
            "guard_installations",
            guard_installation_id.to_owned(),
            "host_capability_json",
        )
    })?;
    if !host_hook_capability_has_exact_current_shape(&capability) {
        return Err(StoreError::corrupt_owner_state_value(
            "guard_installations",
            guard_installation_id.to_owned(),
            "host_capability_json",
        ));
    }
    if observed_policy_hash != capability["policy_hash"].as_str() {
        return Ok(false);
    }
    let Some(last_seen_phase) = last_seen_phase else {
        return Ok(false);
    };
    Ok(KNOWN_GUARD_OBSERVATION_PHASES.contains(&last_seen_phase)
        && capability["commands"]
            .as_object()
            .is_some_and(|commands| commands.contains_key(last_seen_phase)))
}

/// Returns whether one stored installation has a current matching hook observation.
pub fn guard_installation_observation_is_current(
    installation: &GuardInstallationRecord,
) -> StoreResult<bool> {
    guard_observation_matches_current_capability(installation.into())
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

/// Strictly decoded Run-side confirmation that one Write Ticket was consumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedRunWriteTicketConsumption {
    pub run_id: String,
    pub write_ticket_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRunWriteTicketEffect {
    write_ticket_id: Option<String>,
    effect: StoredRunWriteTicketEffectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StoredRunWriteTicketEffectKind {
    None,
    Consumed,
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
    pub expected_paths: Vec<String>,
    pub task_id: String,
    pub change_unit_id: String,
    pub write_ticket_ids: Vec<String>,
    pub basis_state_version: u64,
    pub created_at: String,
    pub expires_at: String,
    pub metadata_json: String,
}

/// Expected Product Repository write match input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedWriteMatch {
    pub matched_post_tool_guard_event_id: String,
    pub matched_paths: Vec<String>,
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
    pub expected_paths: Vec<String>,
    pub task_id: String,
    pub change_unit_id: String,
    pub write_ticket_ids: Vec<String>,
    pub basis_state_version: u64,
    pub status: String,
    pub matched_post_tool_guard_event_id: Option<String>,
    pub matched_paths: Option<Vec<String>>,
    pub created_at: String,
    pub expires_at: String,
    pub matched_at: Option<String>,
    pub metadata_json: String,
}

#[derive(Debug)]
struct ExpectedWriteRaw {
    project_id: String,
    expected_write_id: String,
    session_id: Option<String>,
    connection_internal_id: String,
    guard_installation_id: Option<String>,
    pre_tool_guard_event_id: String,
    host_invocation_id: Option<String>,
    tool_name: Option<String>,
    command_kind: String,
    path_policy: String,
    expected_paths_json: String,
    task_id: String,
    change_unit_id: Option<String>,
    write_ticket_ids_json: String,
    basis_state_version: u64,
    status: String,
    matched_post_tool_guard_event_id: Option<String>,
    matched_paths_json: Option<String>,
    created_at: String,
    expires_at: String,
    matched_at: Option<String>,
    metadata_json: String,
}

/// Unrecorded Product Repository change insert input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecordedChangeInsert {
    pub unrecorded_change_id: String,
    pub session_id: Option<String>,
    pub connection_internal_id: String,
    pub task_id: Option<String>,
    pub confidence: String,
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

/// Deterministic observation used to promote one unresolved suspected change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecordedChangePromotion {
    pub observed_paths_json: String,
    pub detection_json: String,
    pub confirmed_at: String,
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
    pub confidence: String,
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
    pub project_repo_root: std::path::PathBuf,
    pub connection: Option<AgentConnectionRecord>,
    pub guard_installation: Option<GuardInstallationRecord>,
    pub latest_session: Option<AgentSessionRecord>,
    pub latest_event: Option<GuardEventRecord>,
    /// Every guard event at the exact greatest observed UTC instant.
    pub co_latest_events: Vec<GuardEventRecord>,
    pub unresolved_unrecorded_changes: Vec<UnrecordedChangeRecord>,
}

/// Derived prompt-observation availability for one project and Agent Connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptCaptureAvailability {
    pub status: PromptCaptureStatus,
    pub host_supports_prompt_capture: bool,
    pub prompt_capture_configured: bool,
    pub policy_hash_matches_observation: bool,
}

impl PromptCaptureAvailability {
    pub fn is_operational(&self) -> bool {
        self.status.is_operational()
    }
}

/// Creates or updates one guard installation in the Runtime Home registry.
pub fn upsert_guard_installation(
    runtime_home: impl AsRef<Path>,
    input: GuardInstallationUpsert,
) -> StoreResult<GuardInstallationRecord> {
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    upsert_guard_installation_in_transaction(&tx, &input)?;
    tx.commit()?;

    guard_installation(&runtime_home, &input.guard_installation_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "guard_installation",
            id: input.guard_installation_id,
        }
    })
}

pub(crate) fn upsert_guard_installation_in_transaction(
    conn: &Connection,
    input: &GuardInstallationUpsert,
) -> StoreResult<()> {
    validate_guard_installation_upsert(input)?;
    let runtime_home_id = require_runtime_home_id(conn)?;
    let connection_id = input.connection_internal_id.as_str();
    let connection = require_connection(conn, connection_id)?;
    let project = input
        .project_id
        .as_deref()
        .map(|project_id| {
            let project = raw_project_record_from_conn(conn, project_id)?.ok_or_else(|| {
                StoreError::NotFound {
                    entity: "project",
                    id: project_id.to_owned(),
                }
            })?;
            require_connection_project_membership(
                conn,
                connection_id,
                &project.project_internal_id,
            )?;
            Ok::<ProjectRecord, StoreError>(project)
        })
        .transpose()?;
    validate_guard_installation_binding(
        input,
        &connection,
        project.as_ref().map(|project| project.repo_root.as_path()),
    )?;
    let project_internal_id = project
        .as_ref()
        .map(|project| project.project_internal_id.clone());

    if let Some(existing_id) = guard_installation_id_for_scope(
        conn,
        &input.connection_internal_id,
        project_internal_id.as_deref(),
        &input.guard_mode,
    )? {
        if existing_id != input.guard_installation_id {
            return Err(StoreError::Conflict {
                entity: "guard_installation",
                id: input.guard_installation_id.clone(),
                detail: "connection/project/guard_mode scope is already recorded by another guard_installation_id".to_owned(),
            });
        }
    }
    conn.execute(
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
            &input.guard_installation_id,
            runtime_home_id,
            &input.connection_internal_id,
            project_internal_id.as_deref(),
            &input.host_kind,
            &input.guard_mode,
            &input.host_capability_json,
            &input.installation_status,
            input.installed_at.as_deref(),
            &input.last_checked_at,
            input.first_seen_at.as_deref(),
            input.last_seen_at.as_deref(),
            input.last_seen_phase.as_deref(),
            input.observed_host_kind.as_deref(),
            input.observed_policy_hash.as_deref(),
            input.observed_binary_version.as_deref(),
            &input.metadata_json,
        ],
    )?;
    Ok(())
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

    let conn = open_registry_database_read_only(registry_path)?;
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
    let conn = open_registry_database_read_only(&registry_path)?;
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
    collect_rows(rows)?
        .into_iter()
        .map(validate_decoded_guard_installation)
        .collect()
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
    let owning_project = existing
        .project_id
        .as_deref()
        .map(|project_id| raw_project_record_from_conn(&conn, project_id))
        .transpose()?
        .flatten()
        .ok_or_else(|| {
            StoreError::corrupt_owner_state_json(
                "guard_installations",
                existing.guard_installation_id.clone(),
                "host_capability_json",
            )
        })?;
    let connection = require_connection(&conn, &input.connection_internal_id)?;
    validate_stored_guard_installation_binding_fields(
        &existing,
        &connection.host_kind,
        &connection.intent,
        &owning_project.repo_root,
    )?;
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

/// Reports whether an earlier event of one kind exists for the exact managed session.
///
/// The query is intentionally existence-only and excludes the current event so a first
/// delivery cannot be mistaken for evidence about a later host retry.
pub fn prior_guard_event_exists_for_session_kind(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    session_id: &str,
    connection_internal_id: &str,
    event_kind: &str,
    current_guard_event_id: &str,
) -> StoreResult<bool> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("session_id", session_id)?;
    validate_identifier("connection_internal_id", connection_internal_id)?;
    validate_guard_hook_phase("event_kind", event_kind)?;
    validate_identifier("current_guard_event_id", current_guard_event_id)?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(false);
    };
    project
        .conn
        .query_row(
            "SELECT 1
               FROM guard_events
              WHERE project_id = ?1
                AND session_id = ?2
                AND connection_internal_id = ?3
                AND event_kind = ?4
                AND guard_event_id <> ?5
              LIMIT 1",
            params![
                project.project.project_id,
                session_id,
                connection_internal_id,
                event_kind,
                current_guard_event_id
            ],
            |_| Ok(true),
        )
        .optional()
        .map(Option::unwrap_or_default)
        .map_err(Into::into)
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
    let expected_paths_json =
        serde_json::to_string(&input.expected_paths).map_err(|error| StoreError::InvalidInput {
            detail: format!("expected paths cannot be serialized: {error}"),
        })?;
    let write_ticket_ids_json =
        serde_json::to_string(&input.write_ticket_ids).map_err(|error| {
            StoreError::InvalidInput {
                detail: format!("write-ticket IDs cannot be serialized: {error}"),
            }
        })?;
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
            expected_paths_json,
            input.task_id,
            input.change_unit_id,
            write_ticket_ids_json,
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
        ORDER BY volicord_utc_seconds(created_at) DESC,
                 volicord_utc_subsec_nanos(created_at) DESC,
                 expected_write_id DESC",
    )?;
    let rows = stmt.query_map(
        params![project.project.project_id, connection_internal_id],
        expected_write_raw_from_row,
    )?;
    collect_rows(rows)?
        .into_iter()
        .map(expected_write_from_raw)
        .collect()
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
        ORDER BY volicord_utc_seconds(created_at) DESC,
                 volicord_utc_subsec_nanos(created_at) DESC,
                 expected_write_id DESC",
    )?;
    let rows = stmt.query_map(
        params![project.project.project_id, connection_internal_id],
        expected_write_raw_from_row,
    )?;
    collect_rows(rows)?
        .into_iter()
        .map(expected_write_from_raw)
        .collect()
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
        ORDER BY volicord_utc_seconds(matched_at) DESC,
                 volicord_utc_subsec_nanos(matched_at) DESC,
                 expected_write_id DESC",
    )?;
    let rows = stmt.query_map(
        params![
            project.project.project_id,
            connection_internal_id,
            post_tool_guard_event_id
        ],
        expected_write_raw_from_row,
    )?;
    collect_rows(rows)?
        .into_iter()
        .map(expected_write_from_raw)
        .collect()
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
    let matched_paths_json =
        serde_json::to_string(&input.matched_paths).map_err(|error| StoreError::InvalidInput {
            detail: format!("matched paths cannot be serialized: {error}"),
        })?;
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
            matched_paths_json,
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
            confidence,
            summary,
            observed_paths_json,
            detection_json,
            detected_at,
            metadata_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, 'unresolved', ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            project.project.project_id,
            input.unrecorded_change_id,
            input.session_id,
            input.connection_internal_id,
            input.task_id,
            input.confidence,
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

/// Promotes one unresolved suspected change after deterministic observation.
pub fn promote_suspected_unrecorded_change(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    unrecorded_change_id: &str,
    promotion: UnrecordedChangePromotion,
) -> StoreResult<UnrecordedChangeRecord> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("unrecorded_change_id", unrecorded_change_id)?;
    validate_json_array(
        "unrecorded_changes.observed_paths_json",
        &promotion.observed_paths_json,
    )?;
    validate_json_object(
        "unrecorded_changes.detection_json",
        &promotion.detection_json,
    )?;
    validate_timestamp_text("confirmed_at", &promotion.confirmed_at)?;

    let mut project = open_project_for_required_read(runtime_home, project_id)?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    let changed = tx.execute(
        "UPDATE unrecorded_changes
            SET confidence = 'confirmed',
                observed_paths_json = ?3,
                detection_json = ?4
          WHERE project_id = ?1
            AND unrecorded_change_id = ?2
            AND status = 'unresolved'
            AND confidence = 'suspected'",
        params![
            project.project.project_id,
            unrecorded_change_id,
            promotion.observed_paths_json,
            promotion.detection_json,
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
            detail: "unrecorded change is not an unresolved suspected observation".to_owned(),
        });
    }

    unrecorded_change_by_conn(
        &project.conn,
        &project.project.project_id,
        unrecorded_change_id,
    )
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
            confidence,
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
        ORDER BY volicord_utc_seconds(detected_at),
                 volicord_utc_subsec_nanos(detected_at),
                 unrecorded_change_id",
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
    let project =
        project_record_for_execution_read_only(&runtime_home, project_id)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "project",
                id: project_id.to_owned(),
            }
        })?;
    let connection = agent_connection_record_read_only(&runtime_home, connection_internal_id)?;
    let guard_installation =
        selected_guard_installation(&runtime_home, project_id, connection_internal_id)?;
    let latest_session = latest_agent_session(&runtime_home, project_id, connection_internal_id)?;
    let co_latest_events = latest_guard_events(&runtime_home, project_id, connection_internal_id)?;
    let latest_event = co_latest_events.first().cloned();
    let unresolved_unrecorded_changes = list_unresolved_unrecorded_changes(
        &runtime_home,
        project_id,
        Some(connection_internal_id),
    )?;
    Ok(GuardHealthRecord {
        project_repo_root: project.repo_root,
        connection,
        guard_installation,
        latest_session,
        latest_event,
        co_latest_events,
        unresolved_unrecorded_changes,
    })
}

/// Reads post-tool Guard events for one exact session and connection at or after a timestamp.
///
/// The 513th matching row is a fail-closed overflow probe; callers must not treat a truncated
/// correlation window as complete.
pub fn post_tool_guard_events_for_session_since(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    session_id: &str,
    connection_internal_id: &str,
    not_before: &str,
) -> StoreResult<Vec<GuardEventRecord>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("session_id", session_id)?;
    validate_identifier("connection_internal_id", connection_internal_id)?;
    validate_timestamp_text("not_before", not_before)?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(Vec::new());
    };
    let mut stmt = project.conn.prepare(
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
            AND session_id = ?2
            AND connection_internal_id = ?3
            AND event_kind = 'post_tool'
            AND (
              volicord_utc_seconds(occurred_at) > volicord_utc_seconds(?4)
              OR (
                volicord_utc_seconds(occurred_at) = volicord_utc_seconds(?4)
                AND volicord_utc_subsec_nanos(occurred_at)
                    >= volicord_utc_subsec_nanos(?4)
              )
            )
          ORDER BY volicord_utc_seconds(occurred_at) DESC,
                   volicord_utc_subsec_nanos(occurred_at) DESC,
                   guard_event_id DESC
          LIMIT ?5",
    )?;
    let rows = stmt.query_map(
        params![
            project.project.project_id,
            session_id,
            connection_internal_id,
            not_before,
            (POST_TOOL_CORRELATION_EVENT_LIMIT + 1) as i64
        ],
        guard_event_from_row,
    )?;
    let records = collect_rows(rows)?;
    if records.len() > POST_TOOL_CORRELATION_EVENT_LIMIT {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "post-tool correlation window exceeds the bounded event limit of {}",
                POST_TOOL_CORRELATION_EVENT_LIMIT
            ),
        });
    }
    Ok(records)
}

/// Reads and strictly validates the Run-side half of one ticket-consumption link.
pub fn recorded_run_write_ticket_consumption(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    run_id: &str,
) -> StoreResult<Option<RecordedRunWriteTicketConsumption>> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("run_id", run_id)?;
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(None);
    };
    let row = project
        .conn
        .query_row(
            "SELECT status, write_ticket_effect_json
               FROM runs
              WHERE project_id = ?1
                AND run_id = ?2",
            params![project.project.project_id, run_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((status, raw_effect)) = row else {
        return Ok(None);
    };
    if status != "recorded" {
        return Ok(None);
    }
    let effect: StoredRunWriteTicketEffect = serde_json::from_str(&raw_effect).map_err(|_| {
        StoreError::corrupt_owner_state_json("runs", run_id.to_owned(), "write_ticket_effect_json")
    })?;
    match (effect.effect, effect.write_ticket_id) {
        (StoredRunWriteTicketEffectKind::Consumed, Some(write_ticket_id)) => {
            validate_identifier("write_ticket_id", &write_ticket_id)?;
            Ok(Some(RecordedRunWriteTicketConsumption {
                run_id: run_id.to_owned(),
                write_ticket_id,
            }))
        }
        (StoredRunWriteTicketEffectKind::None, None) => Ok(None),
        _ => Err(StoreError::corrupt_owner_state_json(
            "runs",
            run_id.to_owned(),
            "write_ticket_effect_json",
        )),
    }
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
    let connection = record.connection.as_ref().ok_or_else(|| {
        StoreError::corrupt_owner_state_json(
            "guard_installations",
            installation.guard_installation_id.clone(),
            "host_capability_json",
        )
    })?;
    validate_stored_guard_installation_capability_binding(
        installation,
        connection,
        &record.project_repo_root,
    )?;
    let facts = prompt_capture_capability_facts(&installation.host_capability_json)?;
    let policy_hash_matches_observation = installation
        .observed_policy_hash
        .as_deref()
        .zip(facts.expected_policy_hash.as_deref())
        .is_some_and(|(observed, expected)| observed == expected);
    let observation_is_current = guard_installation_observation_is_current(installation)?;
    let status = if !facts.host_supports_prompt_capture {
        PromptCaptureStatus::UnsupportedByHost
    } else if !facts.prompt_capture_configured {
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
        && observation_is_current
        && installation.last_seen_phase.as_deref() == Some("prompt_capture")
    {
        PromptCaptureStatus::Active
    } else if installation.installation_status == GuardInstallationStatus::Active.as_str()
        && observation_is_current
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
}

fn prompt_capture_capability_facts(
    host_capability_json: &str,
) -> StoreResult<PromptCaptureCapabilityFacts> {
    let value = current_host_capability_value(host_capability_json)?;
    let expected_policy_hash = Some(
        value
            .get("policy_hash")
            .and_then(Value::as_str)
            .expect("validated host capability has a policy_hash")
            .to_owned(),
    );
    let host_supports_prompt_capture = value
        .get("host_capabilities")
        .and_then(|capabilities| capabilities.get("user_prompt_submit_hook"))
        .and_then(Value::as_bool)
        .expect("validated host capability has user_prompt_submit_hook");
    let prompt_capture_configured = value
        .get("commands")
        .and_then(Value::as_object)
        .is_some_and(|commands| commands.contains_key("prompt_capture"));
    Ok(PromptCaptureCapabilityFacts {
        expected_policy_hash,
        host_supports_prompt_capture,
        prompt_capture_configured,
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
    Ok(records.pop())
}

fn latest_agent_session(
    runtime_home: &Path,
    project_id: &str,
    connection_internal_id: &str,
) -> StoreResult<Option<AgentSessionRecord>> {
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(None);
    };
    let mut stmt = project.conn.prepare(
        "SELECT
                project_id,
                session_id,
                connection_internal_id,
                guard_installation_id,
                host_kind,
                guard_mode,
                started_at,
                metadata_json
             FROM agent_sessions
            WHERE project_id = ?1
              AND connection_internal_id = ?2
            ORDER BY volicord_utc_seconds(started_at) DESC,
                     volicord_utc_subsec_nanos(started_at) DESC,
                     session_id DESC
            LIMIT 2",
    )?;
    let rows = stmt.query_map(
        params![project.project.project_id, connection_internal_id],
        agent_session_from_row,
    )?;
    let records = collect_rows(rows)?;
    if records.len() > 1 {
        let first = strict_stored_timestamp(
            "agent_sessions",
            &records[0].session_id,
            "started_at",
            &records[0].started_at,
        )?;
        let second = strict_stored_timestamp(
            "agent_sessions",
            &records[1].session_id,
            "started_at",
            &records[1].started_at,
        )?;
        if first == second {
            return Err(StoreError::schema_invariant(
                "project_state",
                format!(
                    "ambiguous co-latest agent_sessions for connection {connection_internal_id}"
                ),
            ));
        }
    }
    Ok(records.into_iter().next())
}

fn latest_guard_events(
    runtime_home: &Path,
    project_id: &str,
    connection_internal_id: &str,
) -> StoreResult<Vec<GuardEventRecord>> {
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(Vec::new());
    };
    let mut stmt = project.conn.prepare(
        "WITH candidates AS (
            SELECT
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
                metadata_json,
                volicord_utc_seconds(occurred_at) AS utc_seconds,
                volicord_utc_subsec_nanos(occurred_at) AS utc_subsec_nanos
             FROM guard_events
            WHERE project_id = ?1
              AND connection_internal_id = ?2
        ), latest_seconds AS (
            SELECT MAX(utc_seconds) AS value FROM candidates
        ), latest_instant AS (
            SELECT MAX(utc_subsec_nanos) AS value
              FROM candidates, latest_seconds
             WHERE utc_seconds = latest_seconds.value
        )
        SELECT
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
          FROM candidates, latest_seconds, latest_instant
         WHERE utc_seconds = latest_seconds.value
           AND utc_subsec_nanos = latest_instant.value
         ORDER BY guard_event_id DESC",
    )?;
    let rows = stmt.query_map(
        params![project.project.project_id, connection_internal_id],
        guard_event_from_row,
    )?;
    collect_rows(rows)
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
    let Some(project) = project_record_for_execution_read_only(runtime_home, project_id)? else {
        return Ok(None);
    };
    let conn = open_project_state_database_read_only(&project.state_db_path)?;
    Ok(Some(OpenGuardProject { project, conn }))
}

fn open_project_for_required_read(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<OpenGuardProject> {
    let Some(project) = open_project_for_write(runtime_home, project_id)? else {
        return Err(StoreError::NotFound {
            entity: "project",
            id: project_id.to_owned(),
        });
    };
    Ok(project)
}

fn open_project_for_write(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<Option<OpenGuardProject>> {
    let Some(project) = project_record_for_execution(runtime_home, project_id)? else {
        return Ok(None);
    };
    let conn = open_project_state_database(&project.state_db_path)?;
    Ok(Some(OpenGuardProject { project, conn }))
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

struct GuardConnectionBinding {
    host_kind: String,
    intent: String,
}

fn require_connection(
    conn: &Connection,
    connection_internal_id: &str,
) -> StoreResult<GuardConnectionBinding> {
    conn.query_row(
        "SELECT host_kind, intent
           FROM agent_connections
          WHERE connection_internal_id = ?1",
        [connection_internal_id],
        |row| {
            Ok(GuardConnectionBinding {
                host_kind: row.get(0)?,
                intent: row.get(1)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| StoreError::NotFound {
        entity: "agent_connection",
        id: connection_internal_id.to_owned(),
    })
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
    validate_host_hook_capability_json(
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

fn validate_guard_installation_binding(
    input: &GuardInstallationUpsert,
    connection: &GuardConnectionBinding,
    project_repo_root: Option<&Path>,
) -> StoreResult<()> {
    let capability = serde_json::from_str::<Value>(&input.host_capability_json).map_err(|_| {
        StoreError::InvalidInput {
            detail: "guard_installations.host_capability_json must be current capability JSON"
                .to_owned(),
        }
    })?;
    let reject = |detail: &str| StoreError::InvalidInput {
        detail: format!("guard installation capability binding mismatch: {detail}"),
    };
    let project_git_info_exclude_path = project_repo_root
        .map(project_git_info_exclude_path)
        .transpose()
        .map_err(|_| reject("owning project Git layout is not safely resolvable"))?
        .flatten();
    if !host_hook_capability_matches_owner_binding(
        &capability,
        HostHookCapabilityOwnerBinding {
            row_host_kind: &input.host_kind,
            row_guard_mode: &input.guard_mode,
            row_guard_installation_id: &input.guard_installation_id,
            connection_internal_id: &input.connection_internal_id,
            connection_host_kind: &connection.host_kind,
            connection_intent: &connection.intent,
            project_repo_root,
            project_git_info_exclude_path: project_git_info_exclude_path.as_deref(),
        },
    ) {
        return Err(reject(
            "capability facts must match the row and owning Agent Connection",
        ));
    }
    Ok(())
}

/// Validates that a stored exact canonical capability is bound to its owner row and
/// owning Agent Connection before any capability facts are consumed.
pub fn validate_stored_guard_installation_capability_binding(
    installation: &GuardInstallationRecord,
    connection: &AgentConnectionRecord,
    project_repo_root: &Path,
) -> StoreResult<()> {
    if installation.connection_internal_id != connection.connection_internal_id {
        return Err(StoreError::corrupt_owner_state_json(
            "guard_installations",
            installation.guard_installation_id.clone(),
            "host_capability_json",
        ));
    }
    validate_stored_guard_installation_binding_fields(
        installation,
        &connection.host_kind,
        &connection.intent,
        project_repo_root,
    )
}

fn validate_stored_guard_installation_binding_fields(
    installation: &GuardInstallationRecord,
    connection_host_kind: &str,
    connection_intent: &str,
    project_repo_root: &Path,
) -> StoreResult<()> {
    let corrupt_capability = || {
        StoreError::corrupt_owner_state_json(
            "guard_installations",
            installation.guard_installation_id.clone(),
            "host_capability_json",
        )
    };
    let capability = current_host_capability_value(&installation.host_capability_json)
        .map_err(|_| corrupt_capability())?;
    let project_git_info_exclude_path =
        project_git_info_exclude_path(project_repo_root).map_err(|_| corrupt_capability())?;
    if !host_hook_capability_matches_owner_binding(
        &capability,
        HostHookCapabilityOwnerBinding {
            row_host_kind: &installation.host_kind,
            row_guard_mode: &installation.guard_mode,
            row_guard_installation_id: &installation.guard_installation_id,
            connection_internal_id: &installation.connection_internal_id,
            connection_host_kind,
            connection_intent,
            project_repo_root: Some(project_repo_root),
            project_git_info_exclude_path: project_git_info_exclude_path.as_deref(),
        },
    ) {
        return Err(corrupt_capability());
    }
    Ok(())
}

fn project_git_info_exclude_path(repo_root: &Path) -> std::io::Result<Option<PathBuf>> {
    resolve_git_worktree_layout(repo_root)
        .map(|layout| layout.map(|layout| layout.common_dir.join("info").join("exclude")))
}

fn validate_guard_installation_observation(
    input: &GuardInstallationObservation,
) -> StoreResult<()> {
    validate_identifier("guard_installation_id", &input.guard_installation_id)?;
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    validate_identifier("project_id", &input.project_id)?;
    validate_host_kind(&input.host_kind)?;
    validate_guard_mode(&input.guard_mode)?;
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
    validate_string_items("expected_writes.expected_paths", &input.expected_paths)?;
    validate_identifier("task_id", &input.task_id)?;
    validate_identifier("change_unit_id", &input.change_unit_id)?;
    validate_string_items("expected_writes.write_ticket_ids", &input.write_ticket_ids)?;
    validate_timestamp_text("created_at", &input.created_at)?;
    validate_timestamp_text("expires_at", &input.expires_at)?;
    validate_json_object("expected_writes.metadata_json", &input.metadata_json)
}

fn validate_expected_write_match(input: &ExpectedWriteMatch) -> StoreResult<()> {
    validate_identifier(
        "matched_post_tool_guard_event_id",
        &input.matched_post_tool_guard_event_id,
    )?;
    validate_string_items("expected_writes.matched_paths", &input.matched_paths)?;
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
    if !matches!(input.confidence.as_str(), "confirmed" | "suspected") {
        return Err(StoreError::InvalidInput {
            detail: "confidence must be confirmed or suspected".to_owned(),
        });
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
    validate_identifier(field, value)?;
    UtcTimestamp::parse(value)
        .and_then(|timestamp| {
            timestamp
                .ensure_canonical_rfc3339_representable()
                .map_err(|_| volicord_types::UtcTimestampParseError)
        })
        .map_err(|_| StoreError::InvalidInput {
            detail: format!(
                "{field} must be a canonical four-digit RFC 3339 timestamp with an explicit offset"
            ),
        })
}

fn strict_stored_timestamp(
    table: &'static str,
    record_ref: &str,
    field: &'static str,
    value: &str,
) -> StoreResult<UtcTimestamp> {
    let timestamp = UtcTimestamp::parse(value)
        .map_err(|_| StoreError::corrupt_owner_state_value(table, record_ref, field))?;
    timestamp
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::corrupt_owner_state_value(table, record_ref, field))?;
    Ok(timestamp)
}

fn validate_host_kind(value: &str) -> StoreResult<()> {
    HostKind::from_str(value)
        .map(|_| ())
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("host_kind is not usable: {error}"),
        })
}

fn validate_guard_mode(value: &str) -> StoreResult<()> {
    if value == IntegrationProfile::Record.as_str() {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "integration profile must be record".to_owned(),
        })
    }
}

fn validate_guard_hook_phase(field: &'static str, value: &str) -> StoreResult<()> {
    validate_identifier(field, value)?;
    if KNOWN_GUARD_OBSERVATION_PHASES.contains(&value) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be pre_tool, post_tool, or prompt_capture"),
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

fn validate_host_hook_capability_json(field: &'static str, text: &str) -> StoreResult<()> {
    let value = serde_json::from_str::<Value>(text).map_err(|_| StoreError::InvalidInput {
        detail: format!("{field} must be exact current capability JSON"),
    })?;
    if host_hook_capability_has_exact_current_shape(&value) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must match {HOST_HOOK_CAPABILITY_SCHEMA}"),
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

fn validate_string_items(field: &'static str, values: &[String]) -> StoreResult<()> {
    if values.iter().all(|value| !value.trim().is_empty()) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must contain only non-empty strings"),
        })
    }
}

fn decode_canonical_string_array(text: &str) -> Result<Vec<String>, ()> {
    let values = serde_json::from_str::<Vec<String>>(text).map_err(|_| ())?;
    if values.iter().any(|value| value.trim().is_empty())
        || serde_json::to_string(&values).map_err(|_| ())? != text
    {
        return Err(());
    }
    Ok(values)
}

fn expected_policy_hash(host_capability_json: &str) -> StoreResult<Option<String>> {
    let value = current_host_capability_value(host_capability_json)?;
    Ok(Some(
        value["policy_hash"]
            .as_str()
            .expect("validated host capability has a policy_hash")
            .to_owned(),
    ))
}

fn guard_status_after_observation(installation: &GuardInstallationRecord) -> StoreResult<String> {
    current_host_capability_value(&installation.host_capability_json)?;
    let status = match installation.installation_status.as_str() {
        "configured" | "reload_required" | "active" => GuardInstallationStatus::Active.as_str(),
        _ => installation.installation_status.as_str(),
    };
    Ok(status.to_owned())
}

fn current_host_capability_value(host_capability_json: &str) -> StoreResult<Value> {
    let value = serde_json::from_str::<Value>(host_capability_json).map_err(|_| {
        StoreError::InvalidInput {
            detail: "guard_installations.host_capability_json must be current capability JSON"
                .to_owned(),
        }
    })?;
    if !host_hook_capability_has_exact_current_shape(&value) {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "guard_installations.host_capability_json must use {HOST_HOOK_CAPABILITY_SCHEMA}"
            ),
        });
    }
    Ok(value)
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

pub(crate) fn guard_installation_from_conn(
    conn: &Connection,
    guard_installation_id: &str,
) -> StoreResult<Option<GuardInstallationRecord>> {
    let record = conn
        .query_row(
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
        .optional()?;
    record.map(validate_decoded_guard_installation).transpose()
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

fn validate_decoded_guard_installation(
    installation: GuardInstallationRecord,
) -> StoreResult<GuardInstallationRecord> {
    current_host_capability_value(&installation.host_capability_json).map_err(|_| {
        StoreError::corrupt_owner_state_json(
            "guard_installations",
            installation.guard_installation_id.clone(),
            "host_capability_json",
        )
    })?;
    Ok(installation)
}

pub(crate) fn agent_session_from_conn(
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
        metadata_json: row.get(7)?,
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
    let raw = conn
        .query_row(
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
            expected_write_raw_from_row,
        )
        .optional()?;
    raw.map(expected_write_from_raw).transpose()
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

fn expected_write_raw_from_row(row: &Row<'_>) -> rusqlite::Result<ExpectedWriteRaw> {
    Ok(ExpectedWriteRaw {
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

fn expected_write_from_raw(raw: ExpectedWriteRaw) -> StoreResult<ExpectedWriteRecord> {
    let corrupt = |field| {
        StoreError::corrupt_owner_state_json(
            "expected_writes",
            raw.expected_write_id.clone(),
            field,
        )
    };
    let expected_paths = decode_canonical_string_array(&raw.expected_paths_json)
        .map_err(|_| corrupt("expected_paths_json"))?;
    let write_ticket_ids = decode_canonical_string_array(&raw.write_ticket_ids_json)
        .map_err(|_| corrupt("write_ticket_ids_json"))?;
    let change_unit_id = raw
        .change_unit_id
        .ok_or_else(|| corrupt("change_unit_id"))?;
    let matched_paths = raw
        .matched_paths_json
        .as_deref()
        .map(decode_canonical_string_array)
        .transpose()
        .map_err(|_| corrupt("matched_paths_json"))?;
    Ok(ExpectedWriteRecord {
        project_id: raw.project_id,
        expected_write_id: raw.expected_write_id,
        session_id: raw.session_id,
        connection_internal_id: raw.connection_internal_id,
        guard_installation_id: raw.guard_installation_id,
        pre_tool_guard_event_id: raw.pre_tool_guard_event_id,
        host_invocation_id: raw.host_invocation_id,
        tool_name: raw.tool_name,
        command_kind: raw.command_kind,
        path_policy: raw.path_policy,
        expected_paths,
        task_id: raw.task_id,
        change_unit_id,
        write_ticket_ids,
        basis_state_version: raw.basis_state_version,
        status: raw.status,
        matched_post_tool_guard_event_id: raw.matched_post_tool_guard_event_id,
        matched_paths,
        created_at: raw.created_at,
        expires_at: raw.expires_at,
        matched_at: raw.matched_at,
        metadata_json: raw.metadata_json,
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
            confidence,
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
        confidence: row.get(6)?,
        summary: row.get(7)?,
        observed_paths_json: row.get(8)?,
        detection_json: row.get(9)?,
        resolution_json: row.get(10)?,
        detected_at: row.get(11)?,
        resolved_at: row.get(12)?,
        resolved_by_actor_source: row.get(13)?,
        metadata_json: row.get(14)?,
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
    use std::{error::Error, path::Path};

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

    const TEST_POLICY_HASH: &str =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    const TEST_CONTENT_HASH: &str =
        "sha256:1111111111111111111111111111111111111111111111111111111111111111";

    #[test]
    fn capability_consumers_reject_unsupported_input_without_inference() {
        for capability in [
            r#"{"schema":"unsupported-host-capability","policy_hash":"sha256:old"}"#,
            r#"{"policy_hash":"sha256:missing-schema"}"#,
        ] {
            assert!(prompt_capture_capability_facts(capability).is_err());
            assert!(expected_policy_hash(capability).is_err());
        }
    }

    #[test]
    fn persisted_capability_corruption_is_not_reported_as_a_stale_observation() {
        for (capability, expected_json_error) in [
            ("not-json", true),
            (r#"{"schema":"unsupported-host-capability"}"#, false),
        ] {
            let error = guard_observation_matches_current_capability(GuardObservationMatch {
                guard_installation_id: "guard_corrupt",
                host_kind: "codex",
                host_capability_json: capability,
                last_seen_at: Some("2026-06-30T00:01:00Z"),
                last_seen_phase: Some("pre_tool"),
                observed_host_kind: Some("codex"),
                observed_policy_hash: Some(TEST_POLICY_HASH),
            })
            .expect_err("corrupt persisted capability must fail closed");
            assert_eq!(
                matches!(error, StoreError::CorruptOwnerStateJson { .. }),
                expected_json_error
            );
            assert_eq!(
                error.classification().route,
                crate::StoreFailureRoute::PersistedDataCorrupt
            );
        }
    }

    fn test_host_capability(
        policy_hash: &str,
        repo_root: &Path,
        connection_id: &str,
        guard_installation_id: &str,
    ) -> String {
        let command = |phase: &str| {
            serde_json::json!({
                "command": repo_root.join(".volicord/bin/volicord"),
                "args": [
                    "_hook", phase,
                    "--repo", repo_root,
                    "--connection", connection_id,
                    "--guard-installation", guard_installation_id,
                    "--host", "codex",
                    "--integration-profile", "record",
                    "--policy-hash", policy_hash,
                    "--host-output", "codex",
                ],
            })
        };
        let wrapper = |phase: &str, command_name: &str| {
            serde_json::json!({
                "kind": "host_hook_wrapper",
                "path": repo_root.join(format!(".codex/hooks/volicord-{command_name}.sh")),
                "status": "unchanged",
                "content_hash": TEST_CONTENT_HASH,
                "ownership": "managed_script",
                "managed_marker": "VOLICORD_MANAGED_HOOK_WRAPPER",
                "executable_required": true,
                "managed_script_command": "exec volicord",
                "host_kind": "codex",
                "phase": phase,
                "purpose": "guard",
                "connection_id": connection_id,
                "guard_installation_id": guard_installation_id,
                "policy_hash": policy_hash,
                "host_output": "codex",
            })
        };
        serde_json::json!({
            "schema": HOST_HOOK_CAPABILITY_SCHEMA,
            "policy_hash": policy_hash,
            "selected_profile": "record",
            "connection_intent": "shared",
            "direct_file_write_matcher_coverage": true,
            "host_capabilities": {
                "stdio_mcp": true,
                "pre_tool_hook": true,
                "post_tool_hook": true,
                "user_prompt_submit_hook": true,
                "rule_file_support": true,
                "project_local_configuration": true,
            },
            "files": [
                {
                    "kind": "agents_managed_block",
                    "path": repo_root.join("AGENTS.md"),
                    "status": "unchanged",
                    "content_hash": TEST_CONTENT_HASH,
                    "ownership": "managed_block",
                    "managed_marker_start": "# BEGIN VOLICORD MANAGED AGENT GUIDANCE",
                    "managed_marker_end": "# END VOLICORD MANAGED AGENT GUIDANCE",
                },
                {
                    "kind": "volicord_policy",
                    "path": repo_root.join(".volicord/policy.json"),
                    "status": "unchanged",
                    "content_hash": TEST_CONTENT_HASH,
                    "ownership": "managed_json",
                },
                {
                    "kind": "host_hook_config",
                    "path": repo_root.join(".codex/hooks.json"),
                    "status": "unchanged",
                    "content_hash": TEST_CONTENT_HASH,
                    "ownership": "managed_json",
                },
                {
                    "kind": "host_hook_dispatch",
                    "path": repo_root.join(".codex/hooks/volicord-dispatch.sh"),
                    "status": "unchanged",
                    "content_hash": TEST_CONTENT_HASH,
                    "ownership": "managed_script",
                    "managed_marker": "VOLICORD_MANAGED_HOOK_WRAPPER",
                    "executable_required": true,
                    "managed_script_role": "codex_dispatch",
                    "host_kind": "codex",
                    "phase": "dispatch",
                },
                wrapper("pre_tool", "pre-tool"),
                wrapper("post_tool", "post-tool"),
                wrapper("prompt_capture", "prompt-capture"),
                {
                    "kind": "host_rule_instruction",
                    "path": repo_root.join(".codex/rules/volicord.rules"),
                    "status": "unchanged",
                    "content_hash": TEST_CONTENT_HASH,
                    "ownership": "managed_block",
                    "managed_marker_start": "# BEGIN VOLICORD MANAGED CODEX RULES",
                    "managed_marker_end": "# END VOLICORD MANAGED CODEX RULES",
                },
            ],
            "commands": {
                "pre_tool": command("pre-tool"),
                "post_tool": command("post-tool"),
                "prompt_capture": command("prompt-capture"),
            },
        })
        .to_string()
    }
    #[test]
    fn guard_records_round_trip_and_unrecorded_changes_resolve() -> Result<(), Box<dyn Error>> {
        let fixture = GuardFixture::new("guard-round-trip")?;
        fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;
        let repo_root =
            project_record_for_execution(fixture.runtime_home.path(), "project_guard_a")?
                .expect("fixture project should exist")
                .repo_root;

        let installation = upsert_guard_installation(
            fixture.runtime_home.path(),
            GuardInstallationUpsert {
                guard_installation_id: "guard_installation_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: Some("project_guard_a".to_owned()),
                host_kind: "codex".to_owned(),
                guard_mode: "record".to_owned(),
                host_capability_json: test_host_capability(
                    TEST_POLICY_HASH,
                    &repo_root,
                    "conn_guard_a",
                    "guard_installation_a",
                ),
                installation_status: "active".to_owned(),
                installed_at: Some("2026-06-30T00:00:00Z".to_owned()),
                last_checked_at: "2026-06-30T00:01:00Z".to_owned(),
                first_seen_at: Some("2026-06-30T00:01:00Z".to_owned()),
                last_seen_at: Some("2026-06-30T00:01:00Z".to_owned()),
                last_seen_phase: Some("pre_tool".to_owned()),
                observed_host_kind: Some("codex".to_owned()),
                observed_policy_hash: Some(TEST_POLICY_HASH.to_owned()),
                observed_binary_version: Some("test".to_owned()),
                metadata_json: "{}".to_owned(),
            },
        )?;
        assert_eq!(installation.project_id.as_deref(), Some("project_guard_a"));
        assert_eq!(installation.guard_mode, "record");

        let session = insert_agent_session(
            fixture.runtime_home.path(),
            "project_guard_a",
            AgentSessionInsert {
                session_id: "session_guard_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: Some("guard_installation_a".to_owned()),
                host_kind: "codex".to_owned(),
                guard_mode: "record".to_owned(),
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
                expected_paths: vec!["src/lib.rs".to_owned()],
                task_id: "task_guard_a".to_owned(),
                change_unit_id: "change_unit_guard_a".to_owned(),
                write_ticket_ids: vec!["write_ticket_a".to_owned()],
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
                matched_paths: vec!["src/lib.rs".to_owned()],
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
                confidence: "confirmed".to_owned(),
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

        let project = project_record_for_execution(fixture.runtime_home.path(), "project_guard_a")?
            .expect("fixture project should exist");
        let conn = open_project_state_database(&project.state_db_path)?;
        for (column, corrupt_text, restored_text) in [
            (
                "expected_paths_json",
                r#"{"not":"paths"}"#,
                r#"["src/lib.rs"]"#,
            ),
            ("write_ticket_ids_json", r#"[1]"#, r#"["write_ticket_a"]"#),
            ("matched_paths_json", r#"[""]"#, r#"["src/lib.rs"]"#),
        ] {
            conn.execute(
                &format!(
                    "UPDATE expected_writes SET {column} = ?1 WHERE expected_write_id = 'expected_write_a'"
                ),
                [corrupt_text],
            )?;
            let error = expected_write(
                fixture.runtime_home.path(),
                "project_guard_a",
                "expected_write_a",
            )
            .expect_err("malformed persisted string arrays must fail closed");
            assert!(matches!(error, StoreError::CorruptOwnerStateJson { .. }));
            conn.execute(
                &format!(
                    "UPDATE expected_writes SET {column} = ?1 WHERE expected_write_id = 'expected_write_a'"
                ),
                [restored_text],
            )?;
        }
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
                guard_mode: "record".to_owned(),
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
                confidence: "confirmed".to_owned(),
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
                guard_mode: "record".to_owned(),
                host_capability_json: test_host_capability(
                    TEST_POLICY_HASH,
                    &project_record_for_execution(fixture.runtime_home.path(), "project_guard_b")?
                        .expect("fixture project B should exist")
                        .repo_root,
                    "conn_guard_a",
                    "guard_installation_cross",
                ),
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
                    requested_control_level,
                    effective_control_level,
                    control_level_reason,
                    work_phase,
                    acceptance_policy,
                    acceptance_policy_reason,
                    carry_forward_json,
                    lifecycle_phase,
                    created_at,
                    updated_at
                )
                VALUES (
                    ?1, ?2, 'agent_connection:conn_guard_a', 'work',
                    'tracked', 'tracked', 'Guard fixture control.',
                    'shaping', 'required', 'Guard fixture requires acceptance.', '[]',
                    'shaping', 't0', 't0'
                )",
                params![project_id, task_id],
            )?;
            Ok(())
        }
    }
}
