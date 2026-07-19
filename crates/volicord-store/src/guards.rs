use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    str::FromStr,
};

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Deserialize;
use serde_json::Value;
use volicord_platform_fs::resolve_git_worktree_layout;
use volicord_types::{
    canonical_json_sha256, guard_manifest_from_json, guard_manifest_matches_owner_binding,
    project_agent_session_id, validate_managed_host_native_session_id, GuardDecision,
    GuardHookContractStatus, GuardHookPhase, GuardManifestOwnerBinding, IntegrationRevision,
    ProjectIntegrationRevisionBasis, PromptCaptureStatus, UnrecordedChangeStatus, UtcTimestamp,
};

use crate::{
    agent_connections::{
        agent_connection_record_from_conn, agent_connection_record_read_only,
        is_agent_connection_project_allowed, AgentConnectionRecord,
    },
    bootstrap::{
        project_record_for_execution, project_record_for_execution_read_only,
        raw_project_record_from_conn, ProjectRecord,
    },
    operational_sessions::{
        connection_integration_revision, current_managed_mcp_runtime_session_for_connection,
        reserve_mcp_runtime_project_session, McpRuntimeProjectSessionReservation,
    },
    sqlite::{
        begin_immediate_transaction, open_project_state_database,
        open_project_state_database_read_only, open_registry_database,
        open_registry_database_read_only, registry_db_path,
    },
    StoreError, StoreResult,
};

/// Maximum prior post-tool Guard events considered for one exact correlation window.
pub const POST_TOOL_CORRELATION_EVENT_LIMIT: usize = 512;

/// Guard installation creation or update input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardInstallationUpsert {
    pub guard_installation_id: String,
    pub connection_internal_id: String,
    pub project_id: String,
    pub manifest_json: String,
}

/// Guard installation row stored in `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardInstallationRecord {
    pub guard_installation_id: String,
    pub runtime_home_id: String,
    pub connection_internal_id: String,
    pub project_id: String,
    pub project_internal_id: String,
    pub manifest_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Idempotent project Agent Session observation from Guard metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionObservation {
    pub connection_internal_id: String,
    pub guard_installation_id: Option<String>,
    pub host_session_id: String,
    pub host_thread_id: String,
    pub host_turn_id: String,
    pub observed_at: String,
}

/// Required managed-runtime attachment input for one project Agent Session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionRuntimeBinding {
    pub runtime_session_id: String,
    pub connection_internal_id: String,
    pub guard_installation_id: Option<String>,
    pub host_session_id: String,
    pub host_thread_id: String,
    pub host_turn_id: String,
    pub observed_at: String,
}

/// Store-derived current project Agent Session identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectAgentSessionIdentity {
    pub session_id: String,
    pub project_integration_revision: String,
    pub guard_installation_id: Option<String>,
}

/// Agent Session row stored in project `state.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionRecord {
    pub project_id: String,
    pub session_id: String,
    pub runtime_session_id: Option<String>,
    pub connection_internal_id: String,
    pub project_integration_revision: String,
    pub host_session_id: String,
    pub host_thread_id: String,
    pub last_host_turn_id: String,
    pub first_observed_at: String,
    pub last_observed_at: String,
}

/// Guard event insert input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardEventInsert {
    pub guard_event_id: String,
    pub session_id: Option<String>,
    pub connection_internal_id: String,
    pub guard_installation_id: String,
    pub policy_hash: String,
    pub integration_revision: String,
    pub event_kind: String,
    pub contract_status: String,
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
    pub guard_installation_id: String,
    pub policy_hash: String,
    pub integration_revision: String,
    pub event_kind: String,
    pub contract_status: String,
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
    pub observation: Option<GuardObservationSummary>,
    pub unresolved_unrecorded_changes: Vec<UnrecordedChangeRecord>,
}

/// Guard events projected only for the current installation manifest ownership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardObservationSummary {
    pub required_phases: Vec<String>,
    pub observed_phases: Vec<String>,
    pub incompatible_event_ids: Vec<String>,
    pub last_observed_at: Option<String>,
}

impl GuardObservationSummary {
    pub fn all_required_phases_observed(&self) -> bool {
        self.incompatible_event_ids.is_empty()
            && self
                .required_phases
                .iter()
                .all(|phase| self.observed_phases.contains(phase))
    }

    pub fn prompt_capture_observed(&self) -> bool {
        self.observed_phases
            .iter()
            .any(|phase| phase == GuardHookPhase::PromptCapture.as_str())
    }
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
    let runtime_home_id = require_runtime_home_id(conn)?;
    let connection_id = input.connection_internal_id.as_str();
    let connection = agent_connection_record_from_conn(conn, connection_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "agent_connection",
            id: connection_id.to_owned(),
        }
    })?;
    let project = raw_project_record_from_conn(conn, &input.project_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "project",
            id: input.project_id.clone(),
        }
    })?;
    require_connection_project_membership(conn, connection_id, &project.project_internal_id)?;
    validate_guard_installation_upsert_binding(input, &connection, &project)?;

    if let Some(existing_id) = guard_installation_id_for_scope(
        conn,
        &input.connection_internal_id,
        &project.project_internal_id,
    )? {
        if existing_id != input.guard_installation_id {
            return Err(StoreError::Conflict {
                entity: "guard_installation",
                id: input.guard_installation_id.clone(),
                detail:
                    "connection/project scope is already recorded by another guard_installation_id"
                        .to_owned(),
            });
        }
    }
    conn.execute(
        "INSERT INTO guard_installations (
            guard_installation_id,
            runtime_home_id,
            connection_internal_id,
            project_internal_id,
            manifest_json,
            created_at,
            updated_at
        )
        VALUES (
            ?1,
            ?2,
            ?3,
            ?4,
            ?5,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        )
        ON CONFLICT(guard_installation_id) DO UPDATE SET
            runtime_home_id = excluded.runtime_home_id,
            connection_internal_id = excluded.connection_internal_id,
            project_internal_id = excluded.project_internal_id,
            manifest_json = excluded.manifest_json,
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        params![
            &input.guard_installation_id,
            runtime_home_id,
            &input.connection_internal_id,
            &project.project_internal_id,
            &input.manifest_json,
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
            json_extract(gi.manifest_json, '$.project_id'),
            gi.manifest_json,
            gi.created_at,
            gi.updated_at
         FROM guard_installations AS gi
        WHERE gi.connection_internal_id = ?1
          AND (?2 IS NULL OR gi.project_internal_id = ?2)
        ORDER BY gi.guard_installation_id",
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

/// Derives the only current project Agent Session identity for native host metadata.
pub fn current_project_agent_session_identity(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    connection_internal_id: &str,
    asserted_guard_installation_id: Option<&str>,
    host_session_id: &str,
) -> StoreResult<ProjectAgentSessionIdentity> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("connection_internal_id", connection_internal_id)?;
    if let Some(guard_installation_id) = asserted_guard_installation_id {
        validate_identifier("guard_installation_id", guard_installation_id)?;
    }
    validate_managed_host_native_session_id(host_session_id).map_err(|_| {
        StoreError::InvalidInput {
            detail: "host_session_id must be valid managed-host identity metadata".to_owned(),
        }
    })?;
    let runtime_home = runtime_home.as_ref();
    let connection = agent_connection_record_read_only(runtime_home, connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        })?;
    if !connection.enabled {
        return Err(StoreError::Conflict {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
            detail: "Agent Connection is disabled".to_owned(),
        });
    }
    if !is_agent_connection_project_allowed(runtime_home, connection_internal_id, project_id)? {
        return Err(StoreError::NotFound {
            entity: "connection_project",
            id: format!("{connection_internal_id}/{project_id}"),
        });
    }
    let project =
        open_project_for_read(runtime_home, project_id)?.ok_or_else(|| StoreError::NotFound {
            entity: "project",
            id: project_id.to_owned(),
        })?;
    let installations =
        list_guard_installations(runtime_home, connection_internal_id, Some(project_id))?;
    let guard_ownership = match installations.as_slice() {
        [] => None,
        [installation] => {
            if asserted_guard_installation_id
                .is_some_and(|asserted| asserted != installation.guard_installation_id)
            {
                return Err(StoreError::Conflict {
                    entity: "guard_installation",
                    id: asserted_guard_installation_id
                        .unwrap_or_default()
                        .to_owned(),
                    detail: "asserted Guard installation is not the current project installation"
                        .to_owned(),
                });
            }
            validate_stored_guard_installation_manifest_binding(
                installation,
                &connection,
                &project.project.repo_root,
            )?;
            let manifest = current_guard_manifest(installation)?;
            Some((
                installation.guard_installation_id.clone(),
                manifest.policy_hash.into_inner(),
            ))
        }
        _ => {
            return Err(StoreError::Conflict {
                entity: "guard_installation",
                id: format!("{connection_internal_id}/{project_id}"),
                detail: "project has multiple current Guard installations".to_owned(),
            })
        }
    };
    if asserted_guard_installation_id.is_some() && guard_ownership.is_none() {
        return Err(StoreError::Conflict {
            entity: "guard_installation",
            id: asserted_guard_installation_id
                .unwrap_or_default()
                .to_owned(),
            detail: "asserted Guard installation is not current for the project".to_owned(),
        });
    }
    let policy_fingerprint = project
        .conn
        .query_row(
            "SELECT policy_fingerprint FROM project_workflow_policies WHERE project_id = ?1",
            [&project.project.project_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .map(Ok)
        .unwrap_or_else(|| {
            canonical_json_sha256(&serde_json::json!({"project_policy": "repository_default"}))
                .map(|digest| digest.into_inner())
                .map_err(|_| StoreError::InvalidInput {
                    detail: "default project policy revision could not be derived".to_owned(),
                })
        })?;
    let connection_revision = connection_integration_revision(&connection)?;
    let project_revision = IntegrationRevision::for_project(ProjectIntegrationRevisionBasis {
        connection_integration_revision: connection_revision.as_str(),
        project_id: &project.project.project_id,
        policy_fingerprint: &policy_fingerprint,
        guard_installation_id: guard_ownership.as_ref().map(|value| value.0.as_str()),
        guard_policy_hash: guard_ownership.as_ref().map(|value| value.1.as_str()),
    })
    .map_err(|error| StoreError::InvalidInput {
        detail: format!("project integration revision could not be derived: {error}"),
    })?;
    let session_id = project_agent_session_id(
        connection_internal_id,
        project_revision.as_str(),
        host_session_id,
    )
    .map_err(|error| StoreError::InvalidInput {
        detail: format!("project Agent Session identity could not be derived: {error}"),
    })?;
    Ok(ProjectAgentSessionIdentity {
        session_id,
        project_integration_revision: project_revision.into_inner(),
        guard_installation_id: guard_ownership.map(|value| value.0),
    })
}

/// Creates or updates one unbound project Agent Session from Guard observation metadata.
pub fn observe_agent_session(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    input: AgentSessionObservation,
) -> StoreResult<AgentSessionRecord> {
    validate_agent_session_observation(&input)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let observed_at = canonical_agent_session_observed_at(&input.observed_at)?;
    let identity = current_project_agent_session_identity(
        &runtime_home,
        project_id,
        &input.connection_internal_id,
        input.guard_installation_id.as_deref(),
        &input.host_session_id,
    )?;
    establish_agent_session_anchor(
        &runtime_home,
        project_id,
        &identity,
        AgentSessionAnchorInput {
            requested_runtime_session_id: None,
            connection_internal_id: &input.connection_internal_id,
            host_session_id: &input.host_session_id,
            host_thread_id: &input.host_thread_id,
            host_turn_id: &input.host_turn_id,
            observed_at: &observed_at,
        },
    )
}

/// Validates, reserves, and attaches one managed runtime to a project Agent Session.
pub fn bind_agent_session_runtime(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    input: AgentSessionRuntimeBinding,
) -> StoreResult<AgentSessionRecord> {
    validate_agent_session_runtime_binding(&input)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let observed_at = canonical_agent_session_observed_at(&input.observed_at)?;

    // Phase 0: validate current managed-runtime facts without project mutation.
    let runtime = current_managed_mcp_runtime_session_for_connection(
        &runtime_home,
        &input.runtime_session_id,
        &input.connection_internal_id,
    )?;
    let process_started_at = UtcTimestamp::parse(&runtime.process_started_at).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "mcp_runtime_sessions",
            runtime.runtime_session_id.clone(),
            "process_started_at",
        )
    })?;
    let observation = UtcTimestamp::parse(&observed_at).expect("validated observation");
    if observation < process_started_at {
        return Err(StoreError::InvalidInput {
            detail: "runtime-bound Agent Session observation cannot precede MCP process start"
                .to_owned(),
        });
    }
    let identity = current_project_agent_session_identity(
        &runtime_home,
        project_id,
        &input.connection_internal_id,
        input.guard_installation_id.as_deref(),
        &input.host_session_id,
    )?;

    // Phase 1: establish the exact project-local anchor without attaching runtime authority.
    establish_agent_session_anchor(
        &runtime_home,
        project_id,
        &identity,
        AgentSessionAnchorInput {
            requested_runtime_session_id: Some(&input.runtime_session_id),
            connection_internal_id: &input.connection_internal_id,
            host_session_id: &input.host_session_id,
            host_thread_id: &input.host_thread_id,
            host_turn_id: &input.host_turn_id,
            observed_at: &observed_at,
        },
    )?;

    // Phase 2: reserve only the exact current identity validated by Phase 1.
    reserve_mcp_runtime_project_session(
        &runtime_home,
        McpRuntimeProjectSessionReservation {
            runtime_session_id: &input.runtime_session_id,
            connection_internal_id: &input.connection_internal_id,
            project_id,
            asserted_guard_installation_id: input.guard_installation_id.as_deref(),
            expected_identity: &identity,
            host_session_id: &input.host_session_id,
            bound_at: &observed_at,
        },
    )?;

    // Phase 3: attach only after the authoritative Registry reservation exists.
    attach_agent_session_runtime(
        &runtime_home,
        project_id,
        &identity,
        &input.runtime_session_id,
        &input.connection_internal_id,
        &input.host_session_id,
        &input.host_thread_id,
    )
}

#[derive(Debug, Clone, Copy)]
struct AgentSessionAnchorInput<'a> {
    requested_runtime_session_id: Option<&'a str>,
    connection_internal_id: &'a str,
    host_session_id: &'a str,
    host_thread_id: &'a str,
    host_turn_id: &'a str,
    observed_at: &'a str,
}

fn establish_agent_session_anchor(
    runtime_home: &Path,
    project_id: &str,
    identity: &ProjectAgentSessionIdentity,
    input: AgentSessionAnchorInput<'_>,
) -> StoreResult<AgentSessionRecord> {
    let mut project = open_guard_project(runtime_home, project_id, input.connection_internal_id)?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    if let Some(runtime_session_id) = input.requested_runtime_session_id {
        let attached_session_id = tx
            .query_row(
                "SELECT session_id
                   FROM agent_sessions
                  WHERE project_id = ?1 AND runtime_session_id = ?2",
                params![project.project.project_id, runtime_session_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if attached_session_id
            .as_deref()
            .is_some_and(|session_id| session_id != identity.session_id)
        {
            return Err(StoreError::Conflict {
                entity: "agent_session",
                id: identity.session_id.clone(),
                detail: "managed runtime is already attached to another project Agent Session"
                    .to_owned(),
            });
        }
    }
    if let Some(existing) =
        agent_session_from_conn(&tx, &project.project.project_id, &identity.session_id)?
    {
        if existing.connection_internal_id != input.connection_internal_id
            || existing.host_session_id != input.host_session_id
            || existing.host_thread_id != input.host_thread_id
        {
            return Err(StoreError::Conflict {
                entity: "agent_session",
                id: identity.session_id.clone(),
                detail: "Agent Session is already bound to another Connection or host identity"
                    .to_owned(),
            });
        }
        if let (Some(existing_runtime), Some(requested_runtime)) = (
            existing.runtime_session_id.as_deref(),
            input.requested_runtime_session_id,
        ) {
            if existing_runtime != requested_runtime {
                return Err(StoreError::Conflict {
                    entity: "agent_session",
                    id: identity.session_id.clone(),
                    detail: "Agent Session is already attached to another runtime".to_owned(),
                });
            }
        }
        if existing.project_integration_revision != identity.project_integration_revision {
            return Err(StoreError::Conflict {
                entity: "agent_session",
                id: identity.session_id.clone(),
                detail: "Agent Session integration revision is immutable".to_owned(),
            });
        }
        let existing_first = strict_stored_timestamp(
            "agent_sessions",
            &existing.session_id,
            "first_observed_at",
            &existing.first_observed_at,
        )?;
        let existing_last = strict_stored_timestamp(
            "agent_sessions",
            &existing.session_id,
            "last_observed_at",
            &existing.last_observed_at,
        )?;
        let observation = UtcTimestamp::parse(input.observed_at).expect("validated observation");
        let first_observed_at = if observation < existing_first {
            input.observed_at
        } else {
            existing.first_observed_at.as_str()
        };
        let (last_host_turn_id, last_observed_at) = if observation >= existing_last {
            (input.host_turn_id, input.observed_at)
        } else {
            (
                existing.last_host_turn_id.as_str(),
                existing.last_observed_at.as_str(),
            )
        };
        tx.execute(
            "UPDATE agent_sessions
                SET last_host_turn_id = ?3,
                    first_observed_at = ?4,
                    last_observed_at = ?5
              WHERE project_id = ?1 AND session_id = ?2",
            params![
                project.project.project_id,
                identity.session_id,
                last_host_turn_id,
                first_observed_at,
                last_observed_at,
            ],
        )?;
    } else {
        tx.execute(
            "INSERT INTO agent_sessions (
                project_id, session_id, runtime_session_id, connection_internal_id,
                project_integration_revision, host_session_id, host_thread_id,
                last_host_turn_id, first_observed_at, last_observed_at
            ) VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![
                project.project.project_id,
                identity.session_id,
                input.connection_internal_id,
                identity.project_integration_revision,
                input.host_session_id,
                input.host_thread_id,
                input.host_turn_id,
                input.observed_at,
            ],
        )?;
    }
    tx.commit()?;

    agent_session_by_conn(
        &project.conn,
        &project.project.project_id,
        &identity.session_id,
    )
}

fn attach_agent_session_runtime(
    runtime_home: &Path,
    project_id: &str,
    identity: &ProjectAgentSessionIdentity,
    runtime_session_id: &str,
    connection_internal_id: &str,
    host_session_id: &str,
    host_thread_id: &str,
) -> StoreResult<AgentSessionRecord> {
    let mut project = open_guard_project(runtime_home, project_id, connection_internal_id)?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    let existing = agent_session_from_conn(&tx, &project.project.project_id, &identity.session_id)?
        .ok_or_else(|| StoreError::Conflict {
            entity: "agent_session",
            id: identity.session_id.clone(),
            detail: "validated project Agent Session anchor is no longer present".to_owned(),
        })?;
    if existing.connection_internal_id != connection_internal_id
        || existing.host_session_id != host_session_id
        || existing.host_thread_id != host_thread_id
        || existing.project_integration_revision != identity.project_integration_revision
    {
        return Err(StoreError::Conflict {
            entity: "agent_session",
            id: identity.session_id.clone(),
            detail: "project Agent Session anchor ownership changed before runtime attachment"
                .to_owned(),
        });
    }
    match existing.runtime_session_id.as_deref() {
        Some(existing_runtime) if existing_runtime == runtime_session_id => {}
        Some(_) => {
            return Err(StoreError::Conflict {
                entity: "agent_session",
                id: identity.session_id.clone(),
                detail: "Agent Session is already attached to another runtime".to_owned(),
            })
        }
        None => {
            tx.execute(
                "UPDATE agent_sessions
                    SET runtime_session_id = ?3
                  WHERE project_id = ?1 AND session_id = ?2 AND runtime_session_id IS NULL",
                params![
                    project.project.project_id,
                    identity.session_id,
                    runtime_session_id
                ],
            )?;
        }
    }
    tx.commit()?;
    agent_session_by_conn(
        &project.conn,
        &project.project.project_id,
        &identity.session_id,
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

/// Verifies that a project Agent Session still matches the current Connection,
/// project policy, and selected Guard ownership revision.
pub fn agent_session_matches_current_integration(
    runtime_home: impl AsRef<Path>,
    session: &AgentSessionRecord,
    guard_installation_id: Option<&str>,
) -> StoreResult<bool> {
    let runtime_home = runtime_home.as_ref();
    let Some(runtime_session_id) = session.runtime_session_id.as_deref() else {
        return Ok(false);
    };
    match current_managed_mcp_runtime_session_for_connection(
        runtime_home,
        runtime_session_id,
        &session.connection_internal_id,
    ) {
        Ok(_) => {}
        Err(StoreError::Conflict { .. } | StoreError::NotFound { .. }) => return Ok(false),
        Err(error) => return Err(error),
    };
    let identity = match current_project_agent_session_identity(
        runtime_home,
        &session.project_id,
        &session.connection_internal_id,
        guard_installation_id,
        &session.host_session_id,
    ) {
        Ok(identity) => identity,
        Err(StoreError::Conflict { .. } | StoreError::NotFound { .. }) => return Ok(false),
        Err(error) => return Err(error),
    };
    Ok(identity.session_id == session.session_id
        && identity.project_integration_revision == session.project_integration_revision)
}

/// Inserts one project-scoped guard event row.
pub fn insert_guard_event(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    input: GuardEventInsert,
) -> StoreResult<GuardEventRecord> {
    validate_guard_event_insert(&input)?;
    let runtime_home = runtime_home.as_ref();
    let installation =
        guard_installation(runtime_home, &input.guard_installation_id)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "guard_installation",
                id: input.guard_installation_id.clone(),
            }
        })?;
    let manifest = current_guard_manifest(&installation)?;
    if installation.connection_internal_id != input.connection_internal_id
        || installation.project_id != project_id
        || manifest.policy_hash.as_str() != input.policy_hash
        || manifest.integration_revision.as_str() != input.integration_revision
    {
        return Err(StoreError::Conflict {
            entity: "guard_event",
            id: input.guard_event_id,
            detail: "Guard event ownership does not match the current installation manifest"
                .to_owned(),
        });
    }
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
            policy_hash,
            integration_revision,
            event_kind,
            contract_status,
            decision,
            subject_json,
            result_json,
            occurred_at,
            metadata_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            project.project.project_id,
            input.guard_event_id,
            input.session_id,
            input.connection_internal_id,
            input.guard_installation_id,
            input.policy_hash,
            input.integration_revision,
            input.event_kind,
            input.contract_status,
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
    let observation = guard_installation
        .as_ref()
        .map(|installation| guard_observation_summary(&runtime_home, project_id, installation))
        .transpose()?;
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
        observation,
        unresolved_unrecorded_changes,
    })
}

/// Derives current Guard observation facts from events with exact manifest ownership.
pub fn guard_observation_summary(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    installation: &GuardInstallationRecord,
) -> StoreResult<GuardObservationSummary> {
    validate_identifier("project_id", project_id)?;
    let manifest = current_guard_manifest(installation)?;
    let required_phases = manifest
        .required_hook_phases
        .iter()
        .map(|phase| phase.as_str().to_owned())
        .collect::<Vec<_>>();
    let Some(project) = open_project_for_read(runtime_home, project_id)? else {
        return Ok(GuardObservationSummary {
            required_phases,
            observed_phases: Vec::new(),
            incompatible_event_ids: Vec::new(),
            last_observed_at: None,
        });
    };
    let mut stmt = project.conn.prepare(
        "SELECT guard_event_id, event_kind, contract_status, occurred_at
           FROM guard_events
          WHERE project_id = ?1
            AND connection_internal_id = ?2
            AND guard_installation_id = ?3
            AND policy_hash = ?4
            AND integration_revision = ?5
          ORDER BY volicord_utc_seconds(occurred_at),
                   volicord_utc_subsec_nanos(occurred_at),
                   guard_event_id",
    )?;
    let rows = stmt.query_map(
        params![
            project.project.project_id,
            installation.connection_internal_id,
            installation.guard_installation_id,
            manifest.policy_hash.as_str(),
            manifest.integration_revision.as_str(),
        ],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    )?;
    let mut observed_phases = BTreeSet::new();
    let mut incompatible_event_ids = Vec::new();
    let mut last_observed_at = None;
    for row in rows {
        let (event_id, phase, contract_status, occurred_at) = row?;
        GuardHookPhase::from_str(&phase).map_err(|_| {
            StoreError::corrupt_owner_state_value("guard_events", event_id.clone(), "event_kind")
        })?;
        if contract_status == GuardHookContractStatus::Compatible.as_str() {
            observed_phases.insert(phase);
        } else if contract_status == GuardHookContractStatus::Malformed.as_str()
            || contract_status == GuardHookContractStatus::Incompatible.as_str()
        {
            incompatible_event_ids.push(event_id);
        } else {
            return Err(StoreError::corrupt_owner_state_value(
                "guard_events",
                event_id,
                "contract_status",
            ));
        }
        last_observed_at = Some(occurred_at);
    }
    Ok(GuardObservationSummary {
        required_phases,
        observed_phases: observed_phases.into_iter().collect(),
        incompatible_event_ids,
        last_observed_at,
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
                policy_hash,
                integration_revision,
                event_kind,
                contract_status,
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
            "manifest_json",
        )
    })?;
    validate_stored_guard_installation_manifest_binding(
        installation,
        connection,
        &record.project_repo_root,
    )?;
    let manifest = current_guard_manifest(installation)?;
    let prompt_capture_configured = manifest
        .required_hook_phases
        .contains(&GuardHookPhase::PromptCapture);
    let observation = record.observation.as_ref();
    let policy_hash_matches_observation =
        observation.is_some_and(|summary| !summary.observed_phases.is_empty());
    let status = if !prompt_capture_configured {
        PromptCaptureStatus::NotConfigured
    } else if observation.is_some_and(|summary| !summary.incompatible_event_ids.is_empty()) {
        PromptCaptureStatus::Degraded
    } else if observation.is_some_and(GuardObservationSummary::prompt_capture_observed) {
        PromptCaptureStatus::Active
    } else if observation.is_some_and(|summary| !summary.observed_phases.is_empty()) {
        PromptCaptureStatus::Observed
    } else {
        PromptCaptureStatus::Configured
    };
    Ok(PromptCaptureAvailability {
        status,
        host_supports_prompt_capture: true,
        prompt_capture_configured,
        policy_hash_matches_observation,
    })
}

fn selected_guard_installation(
    runtime_home: &Path,
    project_id: &str,
    connection_internal_id: &str,
) -> StoreResult<Option<GuardInstallationRecord>> {
    let mut records =
        list_guard_installations(runtime_home, connection_internal_id, Some(project_id))?;
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
                runtime_session_id,
                connection_internal_id,
                project_integration_revision,
                host_session_id,
                host_thread_id,
                last_host_turn_id,
                first_observed_at,
                last_observed_at
             FROM agent_sessions
            WHERE project_id = ?1
              AND connection_internal_id = ?2
            ORDER BY volicord_utc_seconds(last_observed_at) DESC,
                     volicord_utc_subsec_nanos(last_observed_at) DESC,
                     session_id DESC
            LIMIT 2",
    )?;
    let rows = stmt.query_map(
        params![project.project.project_id, connection_internal_id],
        agent_session_from_row,
    )?;
    let records = collect_rows(rows)?
        .into_iter()
        .map(validate_decoded_agent_session)
        .collect::<StoreResult<Vec<_>>>()?;
    if records.len() > 1 {
        let first = strict_stored_timestamp(
            "agent_sessions",
            &records[0].session_id,
            "last_observed_at",
            &records[0].last_observed_at,
        )?;
        let second = strict_stored_timestamp(
            "agent_sessions",
            &records[1].session_id,
            "last_observed_at",
            &records[1].last_observed_at,
        )?;
        if first == second {
            // Guard health does not require one session. Omit the singular
            // diagnostic instead of guessing between concurrent observations.
            return Ok(None);
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
                policy_hash,
                integration_revision,
                event_kind,
                contract_status,
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
            policy_hash,
            integration_revision,
            event_kind,
            contract_status,
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
    project_internal_id: &str,
) -> StoreResult<Option<String>> {
    conn.query_row(
        "SELECT guard_installation_id
           FROM guard_installations
          WHERE connection_internal_id = ?1
            AND project_internal_id = ?2",
        params![connection_internal_id, project_internal_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(StoreError::from)
}

fn validate_guard_installation_upsert(input: &GuardInstallationUpsert) -> StoreResult<()> {
    validate_identifier("guard_installation_id", &input.guard_installation_id)?;
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    validate_identifier("project_id", &input.project_id)?;
    guard_manifest_from_json(&input.manifest_json).map_err(|_| StoreError::InvalidInput {
        detail: "guard_installations.manifest_json must be one canonical current Guard manifest"
            .to_owned(),
    })?;
    Ok(())
}

pub(crate) fn validate_guard_installation_upsert_binding(
    input: &GuardInstallationUpsert,
    connection: &AgentConnectionRecord,
    project: &ProjectRecord,
) -> StoreResult<()> {
    validate_guard_installation_upsert(input)?;
    validate_guard_installation_binding(input, connection, project)
}

fn validate_guard_installation_binding(
    input: &GuardInstallationUpsert,
    connection: &AgentConnectionRecord,
    project: &ProjectRecord,
) -> StoreResult<()> {
    let manifest =
        guard_manifest_from_json(&input.manifest_json).map_err(|_| StoreError::InvalidInput {
            detail:
                "guard_installations.manifest_json must be one canonical current Guard manifest"
                    .to_owned(),
        })?;
    let manifest_value = serde_json::to_value(manifest).map_err(|_| StoreError::InvalidInput {
        detail: "Guard manifest cannot be represented as canonical JSON".to_owned(),
    })?;
    let reject = |detail: &str| StoreError::InvalidInput {
        detail: format!("Guard installation manifest binding mismatch: {detail}"),
    };
    let project_git_info_exclude_path = project_git_info_exclude_path(&project.repo_root)
        .map_err(|_| reject("owning project Git layout is not safely resolvable"))?;
    let integration_revision = connection_integration_revision(connection)?;
    if !guard_manifest_matches_owner_binding(
        &manifest_value,
        GuardManifestOwnerBinding {
            row_guard_installation_id: &input.guard_installation_id,
            row_connection_id: &input.connection_internal_id,
            row_project_id: &input.project_id,
            connection_host_kind: &connection.host_kind,
            connection_integration_revision: integration_revision.as_str(),
            project_repo_root: &project.repo_root,
            project_git_info_exclude_path: project_git_info_exclude_path.as_deref(),
        },
    ) {
        return Err(reject(
            "manifest facts must match the row and owning Agent Connection",
        ));
    }
    Ok(())
}

/// Validates that a stored canonical Guard manifest is bound to its owner row.
pub fn validate_stored_guard_installation_manifest_binding(
    installation: &GuardInstallationRecord,
    connection: &AgentConnectionRecord,
    project_repo_root: &Path,
) -> StoreResult<()> {
    if installation.connection_internal_id != connection.connection_internal_id {
        return Err(StoreError::corrupt_owner_state_json(
            "guard_installations",
            installation.guard_installation_id.clone(),
            "manifest_json",
        ));
    }
    let corrupt_manifest = || {
        StoreError::corrupt_owner_state_json(
            "guard_installations",
            installation.guard_installation_id.clone(),
            "manifest_json",
        )
    };
    let manifest = current_guard_manifest(installation).map_err(|_| corrupt_manifest())?;
    let manifest_value = serde_json::to_value(manifest).map_err(|_| corrupt_manifest())?;
    let project_git_info_exclude_path =
        project_git_info_exclude_path(project_repo_root).map_err(|_| corrupt_manifest())?;
    let integration_revision =
        connection_integration_revision(connection).map_err(|_| corrupt_manifest())?;
    if !guard_manifest_matches_owner_binding(
        &manifest_value,
        GuardManifestOwnerBinding {
            row_guard_installation_id: &installation.guard_installation_id,
            row_connection_id: &installation.connection_internal_id,
            row_project_id: &installation.project_id,
            connection_host_kind: &connection.host_kind,
            connection_integration_revision: integration_revision.as_str(),
            project_repo_root,
            project_git_info_exclude_path: project_git_info_exclude_path.as_deref(),
        },
    ) {
        return Err(corrupt_manifest());
    }
    Ok(())
}

fn project_git_info_exclude_path(repo_root: &Path) -> std::io::Result<Option<PathBuf>> {
    resolve_git_worktree_layout(repo_root)
        .map(|layout| layout.map(|layout| layout.common_dir.join("info").join("exclude")))
}

fn validate_agent_session_observation(input: &AgentSessionObservation) -> StoreResult<()> {
    validate_agent_session_metadata(
        &input.connection_internal_id,
        input.guard_installation_id.as_deref(),
        &input.host_session_id,
        &input.host_thread_id,
        &input.host_turn_id,
        &input.observed_at,
    )
}

fn validate_agent_session_runtime_binding(input: &AgentSessionRuntimeBinding) -> StoreResult<()> {
    validate_identifier("runtime_session_id", &input.runtime_session_id)?;
    validate_agent_session_metadata(
        &input.connection_internal_id,
        input.guard_installation_id.as_deref(),
        &input.host_session_id,
        &input.host_thread_id,
        &input.host_turn_id,
        &input.observed_at,
    )
}

fn validate_agent_session_metadata(
    connection_internal_id: &str,
    guard_installation_id: Option<&str>,
    host_session_id: &str,
    host_thread_id: &str,
    host_turn_id: &str,
    observed_at: &str,
) -> StoreResult<()> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    if let Some(guard_installation_id) = guard_installation_id {
        validate_identifier("guard_installation_id", guard_installation_id)?;
    }
    for (field, value) in [
        ("host_session_id", host_session_id),
        ("host_thread_id", host_thread_id),
        ("host_turn_id", host_turn_id),
    ] {
        validate_managed_host_native_session_id(value).map_err(|_| StoreError::InvalidInput {
            detail: format!("{field} must be valid managed-host identity metadata"),
        })?;
    }
    validate_timestamp_text("observed_at", observed_at)
}

fn canonical_agent_session_observed_at(observed_at: &str) -> StoreResult<String> {
    UtcTimestamp::parse(observed_at)
        .map_err(|_| StoreError::InvalidInput {
            detail: "observed_at must be a canonical RFC 3339 timestamp".to_owned(),
        })
        .map(|timestamp| timestamp.to_canonical_string())
}

fn validate_guard_event_insert(input: &GuardEventInsert) -> StoreResult<()> {
    validate_identifier("guard_event_id", &input.guard_event_id)?;
    if let Some(session_id) = &input.session_id {
        validate_identifier("session_id", session_id)?;
    }
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    validate_identifier("guard_installation_id", &input.guard_installation_id)?;
    volicord_types::PolicyHash::parse(input.policy_hash.clone()).map_err(|_| {
        StoreError::InvalidInput {
            detail: "guard_events.policy_hash must be canonical".to_owned(),
        }
    })?;
    IntegrationRevision::parse(input.integration_revision.clone()).map_err(|_| {
        StoreError::InvalidInput {
            detail: "guard_events.integration_revision must be canonical".to_owned(),
        }
    })?;
    validate_guard_hook_phase("event_kind", &input.event_kind)?;
    validate_guard_hook_contract_status(&input.contract_status)?;
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

fn validate_guard_hook_phase(field: &'static str, value: &str) -> StoreResult<()> {
    validate_identifier(field, value)?;
    if GuardHookPhase::from_str(value).is_ok() {
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

fn validate_guard_hook_contract_status(value: &str) -> StoreResult<()> {
    if [
        GuardHookContractStatus::Compatible.as_str(),
        GuardHookContractStatus::Malformed.as_str(),
        GuardHookContractStatus::Incompatible.as_str(),
    ]
    .contains(&value)
    {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "contract_status must be compatible, malformed, or incompatible".to_owned(),
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

fn current_guard_manifest(
    installation: &GuardInstallationRecord,
) -> StoreResult<volicord_types::GuardManifest> {
    guard_manifest_from_json(&installation.manifest_json).map_err(|_| StoreError::InvalidInput {
        detail: "guard_installations.manifest_json must be one canonical current Guard manifest"
            .to_owned(),
    })
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
            json_extract(gi.manifest_json, '$.project_id'),
            gi.manifest_json,
            gi.created_at,
            gi.updated_at
         FROM guard_installations AS gi
        WHERE gi.guard_installation_id = ?1",
            [guard_installation_id],
            guard_installation_from_row,
        )
        .optional()?;
    record.map(validate_decoded_guard_installation).transpose()
}

fn guard_installation_from_row(row: &Row<'_>) -> rusqlite::Result<GuardInstallationRecord> {
    Ok(GuardInstallationRecord {
        guard_installation_id: row.get(0)?,
        runtime_home_id: row.get(1)?,
        connection_internal_id: row.get(2)?,
        project_internal_id: row.get(3)?,
        project_id: row.get(4)?,
        manifest_json: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn validate_decoded_guard_installation(
    installation: GuardInstallationRecord,
) -> StoreResult<GuardInstallationRecord> {
    current_guard_manifest(&installation).map_err(|_| {
        StoreError::corrupt_owner_state_json(
            "guard_installations",
            installation.guard_installation_id.clone(),
            "manifest_json",
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
            runtime_session_id,
            connection_internal_id,
            project_integration_revision,
            host_session_id,
            host_thread_id,
            last_host_turn_id,
            first_observed_at,
            last_observed_at
         FROM agent_sessions
        WHERE project_id = ?1
          AND session_id = ?2",
        params![project_id, session_id],
        agent_session_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|record| record.map(validate_decoded_agent_session).transpose())
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
        runtime_session_id: row.get(2)?,
        connection_internal_id: row.get(3)?,
        project_integration_revision: row.get(4)?,
        host_session_id: row.get(5)?,
        host_thread_id: row.get(6)?,
        last_host_turn_id: row.get(7)?,
        first_observed_at: row.get(8)?,
        last_observed_at: row.get(9)?,
    })
}

fn validate_decoded_agent_session(session: AgentSessionRecord) -> StoreResult<AgentSessionRecord> {
    let corrupt = |field| {
        StoreError::corrupt_owner_state_value("agent_sessions", session.session_id.clone(), field)
    };
    for (field, value) in [
        ("project_id", session.project_id.as_str()),
        ("session_id", session.session_id.as_str()),
        (
            "connection_internal_id",
            session.connection_internal_id.as_str(),
        ),
    ] {
        validate_identifier(field, value).map_err(|_| corrupt(field))?;
    }
    if let Some(runtime_session_id) = session.runtime_session_id.as_deref() {
        validate_identifier("runtime_session_id", runtime_session_id)
            .map_err(|_| corrupt("runtime_session_id"))?;
    }
    let expected_session_id = project_agent_session_id(
        &session.connection_internal_id,
        &session.project_integration_revision,
        &session.host_session_id,
    )
    .map_err(|_| corrupt("session_id"))?;
    if expected_session_id != session.session_id {
        return Err(corrupt("session_id"));
    }
    IntegrationRevision::parse(session.project_integration_revision.clone())
        .map_err(|_| corrupt("project_integration_revision"))?;
    for (field, value) in [
        ("host_session_id", &session.host_session_id),
        ("host_thread_id", &session.host_thread_id),
        ("last_host_turn_id", &session.last_host_turn_id),
    ] {
        validate_managed_host_native_session_id(value).map_err(|_| corrupt(field))?;
    }
    let first = strict_stored_timestamp(
        "agent_sessions",
        &session.session_id,
        "first_observed_at",
        &session.first_observed_at,
    )?;
    let last = strict_stored_timestamp(
        "agent_sessions",
        &session.session_id,
        "last_observed_at",
        &session.last_observed_at,
    )?;
    if last < first {
        return Err(corrupt("last_observed_at"));
    }
    Ok(session)
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
            policy_hash,
            integration_revision,
            event_kind,
            contract_status,
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
        policy_hash: row.get(5)?,
        integration_revision: row.get(6)?,
        event_kind: row.get(7)?,
        contract_status: row.get(8)?,
        decision: row.get(9)?,
        subject_json: row.get(10)?,
        result_json: row.get(11)?,
        occurred_at: row.get(12)?,
        metadata_json: row.get(13)?,
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
pub(crate) fn test_guard_manifest_json(
    connection: &AgentConnectionRecord,
    project_id: &str,
    repo_root: &Path,
    guard_installation_id: &str,
    policy_hash: &str,
) -> String {
    use volicord_types::{
        AgentConnectionId, GuardCommandAbsolutePath, GuardCommandInvocationSet,
        GuardCommandProjection, GuardHookPhase, GuardInstallationId, GuardManifest, HostKind,
        IntegrationProfile, ManagedFileExpectation, PolicyHash, ProjectId, GUARD_MANIFEST_SCHEMA,
    };

    let integration_revision = connection_integration_revision(connection).expect("test revision");
    let typed_policy_hash = PolicyHash::parse(policy_hash).expect("test policy hash");
    let runtime_commands = GuardCommandInvocationSet::new(
        GuardCommandAbsolutePath::from_path(&repo_root.join("bin/volicord"))
            .expect("test command path"),
        GuardCommandAbsolutePath::from_path(repo_root).expect("test repository path"),
        AgentConnectionId::new(&connection.connection_internal_id),
        GuardInstallationId::new(guard_installation_id),
        HostKind::Codex,
        IntegrationProfile::Record,
        Some(typed_policy_hash.clone()),
        HostKind::Codex,
    )
    .expect("test Guard invocation set")
    .to_commands(GuardCommandProjection::Runtime)
    .expect("test runtime command projection");
    let hash = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
    let block = |kind: &str, path: PathBuf| ManagedFileExpectation {
        kind: kind.to_owned(),
        path: path.display().to_string(),
        content_hash: hash.to_owned(),
        ownership: "managed_block".to_owned(),
        managed_marker_start: Some("VOLICORD_START".to_owned()),
        managed_marker_end: Some("VOLICORD_END".to_owned()),
        managed_marker: None,
        executable_required: None,
        managed_script_role: None,
        managed_script_command: None,
        host_kind: None,
        phase: None,
        purpose: None,
        connection_id: None,
        guard_installation_id: None,
        policy_hash: None,
        host_output: None,
    };
    let json_file = |kind: &str, path: PathBuf| ManagedFileExpectation {
        kind: kind.to_owned(),
        path: path.display().to_string(),
        content_hash: hash.to_owned(),
        ownership: "managed_json".to_owned(),
        managed_marker_start: None,
        managed_marker_end: None,
        managed_marker: None,
        executable_required: None,
        managed_script_role: None,
        managed_script_command: None,
        host_kind: None,
        phase: None,
        purpose: None,
        connection_id: None,
        guard_installation_id: None,
        policy_hash: None,
        host_output: None,
    };
    let script = |phase: GuardHookPhase| ManagedFileExpectation {
        kind: "host_hook_wrapper".to_owned(),
        path: repo_root
            .join(format!(".codex/hooks/volicord-{}.sh", phase.command_name()))
            .display()
            .to_string(),
        content_hash: hash.to_owned(),
        ownership: "managed_script".to_owned(),
        managed_marker_start: None,
        managed_marker_end: None,
        managed_marker: Some("VOLICORD_MANAGED_HOOK_WRAPPER".to_owned()),
        executable_required: Some(true),
        managed_script_role: None,
        managed_script_command: Some("/test/volicord _hook".to_owned()),
        host_kind: Some("codex".to_owned()),
        phase: Some(phase.as_str().to_owned()),
        purpose: Some("guard".to_owned()),
        connection_id: Some(connection.connection_internal_id.clone()),
        guard_installation_id: Some(guard_installation_id.to_owned()),
        policy_hash: Some(policy_hash.to_owned()),
        host_output: Some("codex".to_owned()),
    };
    let mut files = vec![
        block("agents_managed_block", repo_root.join("AGENTS.md")),
        json_file("volicord_policy", repo_root.join(".volicord/policy.json")),
        json_file("host_hook_config", repo_root.join(".codex/hooks.json")),
        ManagedFileExpectation {
            kind: "host_hook_dispatch".to_owned(),
            path: repo_root
                .join(".codex/hooks/volicord-dispatch.sh")
                .display()
                .to_string(),
            content_hash: hash.to_owned(),
            ownership: "managed_script".to_owned(),
            managed_marker_start: None,
            managed_marker_end: None,
            managed_marker: Some("VOLICORD_MANAGED_HOOK_WRAPPER".to_owned()),
            executable_required: Some(true),
            managed_script_role: Some("codex_dispatch".to_owned()),
            managed_script_command: None,
            host_kind: Some("codex".to_owned()),
            phase: Some("dispatch".to_owned()),
            purpose: None,
            connection_id: None,
            guard_installation_id: None,
            policy_hash: None,
            host_output: None,
        },
        block(
            "host_rule_instruction",
            repo_root.join(".codex/rules/volicord.rules"),
        ),
    ];
    files.extend(GuardHookPhase::REQUIRED.into_iter().map(script));
    let manifest = GuardManifest {
        schema: GUARD_MANIFEST_SCHEMA.to_owned(),
        guard_installation_id: GuardInstallationId::new(guard_installation_id),
        connection_id: AgentConnectionId::new(&connection.connection_internal_id),
        project_id: ProjectId::new(project_id),
        host_kind: HostKind::Codex,
        integration_profile: IntegrationProfile::Record,
        policy_hash: typed_policy_hash,
        integration_revision,
        runtime_commands,
        managed_files: files,
        required_hook_phases: GuardHookPhase::REQUIRED.to_vec(),
    };
    serde_json::to_string(&manifest).expect("test manifest")
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
            HOST_KIND_CODEX, HOST_SCOPE_PROJECT,
        },
        bootstrap::{
            initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
        },
        operational_sessions::{start_mcp_runtime_session, McpRuntimeSessionStart},
    };
    use volicord_types::McpRuntimeSessionSource;

    const TEST_POLICY_HASH: &str =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    fn start_guard_runtime(
        runtime_home: &Path,
        connection_id: &str,
        started_at: &str,
    ) -> StoreResult<String> {
        Ok(start_mcp_runtime_session(
            runtime_home,
            McpRuntimeSessionStart {
                connection_internal_id: connection_id.to_owned(),
                session_source: McpRuntimeSessionSource::ManagedHost,
                observed_host_executable_version: None,
                process_id: 42,
                process_started_at: started_at.to_owned(),
            },
        )?
        .runtime_session_id)
    }

    #[test]
    fn exact_replay_finishes_an_interrupted_final_runtime_attachment() -> Result<(), Box<dyn Error>>
    {
        let fixture = GuardFixture::new("guard-runtime-attach-replay")?;
        fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;
        let runtime_session_id = start_guard_runtime(
            fixture.runtime_home.path(),
            "conn_guard_a",
            "2026-07-19T00:00:00Z",
        )?;
        let observed_at = "2026-07-19T00:00:01Z";
        let identity = current_project_agent_session_identity(
            fixture.runtime_home.path(),
            "project_guard_a",
            "conn_guard_a",
            None,
            "session_guard_a",
        )?;

        establish_agent_session_anchor(
            fixture.runtime_home.path(),
            "project_guard_a",
            &identity,
            AgentSessionAnchorInput {
                requested_runtime_session_id: Some(&runtime_session_id),
                connection_internal_id: "conn_guard_a",
                host_session_id: "session_guard_a",
                host_thread_id: "thread_guard_a",
                host_turn_id: "turn_guard_a",
                observed_at,
            },
        )?;
        reserve_mcp_runtime_project_session(
            fixture.runtime_home.path(),
            McpRuntimeProjectSessionReservation {
                runtime_session_id: &runtime_session_id,
                connection_internal_id: "conn_guard_a",
                project_id: "project_guard_a",
                asserted_guard_installation_id: None,
                expected_identity: &identity,
                host_session_id: "session_guard_a",
                bound_at: observed_at,
            },
        )?;

        let unbound = agent_session(
            fixture.runtime_home.path(),
            "project_guard_a",
            &identity.session_id,
        )?
        .expect("Phase 1 anchor");
        assert!(unbound.runtime_session_id.is_none());
        assert!(
            crate::operational_sessions::mcp_runtime_project_session_binding(
                fixture.runtime_home.path(),
                "project_guard_a",
                &identity.session_id,
            )?
            .is_some()
        );

        let attached = bind_agent_session_runtime(
            fixture.runtime_home.path(),
            "project_guard_a",
            AgentSessionRuntimeBinding {
                runtime_session_id: runtime_session_id.clone(),
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: None,
                host_session_id: "session_guard_a".to_owned(),
                host_thread_id: "thread_guard_a".to_owned(),
                host_turn_id: "turn_guard_a".to_owned(),
                observed_at: observed_at.to_owned(),
            },
        )?;
        assert_eq!(
            attached.runtime_session_id.as_deref(),
            Some(runtime_session_id.as_str())
        );

        let project = project_record_for_execution(fixture.runtime_home.path(), "project_guard_a")?
            .expect("fixture project");
        let project_conn = open_project_state_database(&project.state_db_path)?;
        let project_count: i64 = project_conn.query_row(
            "SELECT COUNT(*) FROM agent_sessions WHERE session_id = ?1",
            [&identity.session_id],
            |row| row.get(0),
        )?;
        assert_eq!(project_count, 1);
        let registry_conn = open_registry_database(registry_db_path(fixture.runtime_home.path()))?;
        let binding_count: i64 = registry_conn.query_row(
            "SELECT COUNT(*) FROM mcp_runtime_project_session_bindings WHERE session_id = ?1",
            [&identity.session_id],
            |row| row.get(0),
        )?;
        assert_eq!(binding_count, 1);

        let changed_owner = bind_agent_session_runtime(
            fixture.runtime_home.path(),
            "project_guard_a",
            AgentSessionRuntimeBinding {
                runtime_session_id,
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: None,
                host_session_id: "session_guard_a".to_owned(),
                host_thread_id: "thread_guard_changed".to_owned(),
                host_turn_id: "turn_guard_changed".to_owned(),
                observed_at: "2026-07-19T00:00:02Z".to_owned(),
            },
        )
        .expect_err("changed owner input cannot claim an existing reservation");
        assert!(matches!(changed_owner, StoreError::Conflict { .. }));
        let binding_count_after: i64 = registry_conn.query_row(
            "SELECT COUNT(*) FROM mcp_runtime_project_session_bindings WHERE session_id = ?1",
            [&identity.session_id],
            |row| row.get(0),
        )?;
        assert_eq!(binding_count_after, 1);
        Ok(())
    }

    #[test]
    fn project_revision_change_after_anchor_prevents_registry_reservation(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = GuardFixture::new("guard-runtime-anchor-revision-race")?;
        fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;
        let runtime_session_id = start_guard_runtime(
            fixture.runtime_home.path(),
            "conn_guard_a",
            "2026-07-19T00:00:00Z",
        )?;
        let observed_at = "2026-07-19T00:00:01Z";
        let old_identity = current_project_agent_session_identity(
            fixture.runtime_home.path(),
            "project_guard_a",
            "conn_guard_a",
            None,
            "session_guard_a",
        )?;
        establish_agent_session_anchor(
            fixture.runtime_home.path(),
            "project_guard_a",
            &old_identity,
            AgentSessionAnchorInput {
                requested_runtime_session_id: Some(&runtime_session_id),
                connection_internal_id: "conn_guard_a",
                host_session_id: "session_guard_a",
                host_thread_id: "thread_guard_a",
                host_turn_id: "turn_guard_a",
                observed_at,
            },
        )?;

        let project = project_record_for_execution(fixture.runtime_home.path(), "project_guard_a")?
            .expect("fixture project");
        let project_conn = open_project_state_database(&project.state_db_path)?;
        project_conn.execute(
            "INSERT INTO project_workflow_policies (
                project_id, policy_schema, policy_version, policy_json,
                policy_fingerprint, source, applied_at, created_at
             ) VALUES (?1, 'volicord.workflow_policy', 1, '{}', ?2, 'test', ?3, ?3)",
            params![
                "project_guard_a",
                format!("sha256:{}", "9".repeat(64)),
                "2026-07-19T00:00:01Z"
            ],
        )?;

        let error = reserve_mcp_runtime_project_session(
            fixture.runtime_home.path(),
            McpRuntimeProjectSessionReservation {
                runtime_session_id: &runtime_session_id,
                connection_internal_id: "conn_guard_a",
                project_id: "project_guard_a",
                asserted_guard_installation_id: None,
                expected_identity: &old_identity,
                host_session_id: "session_guard_a",
                bound_at: observed_at,
            },
        )
        .expect_err("a changed project revision cannot reserve the old anchor");
        assert!(matches!(error, StoreError::Conflict { .. }));
        assert!(
            crate::operational_sessions::mcp_runtime_project_session_binding(
                fixture.runtime_home.path(),
                "project_guard_a",
                &old_identity.session_id,
            )?
            .is_none()
        );
        let old_anchor = agent_session(
            fixture.runtime_home.path(),
            "project_guard_a",
            &old_identity.session_id,
        )?
        .expect("historical unbound anchor");
        assert!(old_anchor.runtime_session_id.is_none());

        let current = bind_agent_session_runtime(
            fixture.runtime_home.path(),
            "project_guard_a",
            AgentSessionRuntimeBinding {
                runtime_session_id,
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: None,
                host_session_id: "session_guard_a".to_owned(),
                host_thread_id: "thread_guard_a".to_owned(),
                host_turn_id: "turn_guard_current".to_owned(),
                observed_at: "2026-07-19T00:00:02Z".to_owned(),
            },
        )?;
        assert_ne!(current.session_id, old_identity.session_id);
        assert!(current.runtime_session_id.is_some());
        Ok(())
    }

    #[test]
    fn guard_records_round_trip_and_unrecorded_changes_resolve() -> Result<(), Box<dyn Error>> {
        let fixture = GuardFixture::new("guard-round-trip")?;
        fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;
        let repo_root =
            project_record_for_execution(fixture.runtime_home.path(), "project_guard_a")?
                .expect("fixture project should exist")
                .repo_root;
        let connection =
            agent_connection_record_read_only(fixture.runtime_home.path(), "conn_guard_a")?
                .expect("fixture connection should exist");
        let integration_revision = connection_integration_revision(&connection)?
            .as_str()
            .to_owned();

        let installation = upsert_guard_installation(
            fixture.runtime_home.path(),
            GuardInstallationUpsert {
                guard_installation_id: "guard_installation_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: "project_guard_a".to_owned(),
                manifest_json: test_guard_manifest_json(
                    &connection,
                    "project_guard_a",
                    &repo_root,
                    "guard_installation_a",
                    TEST_POLICY_HASH,
                ),
            },
        )?;
        assert_eq!(installation.project_id, "project_guard_a");
        let manifest = guard_manifest_from_json(&installation.manifest_json)?;
        assert_eq!(manifest.policy_hash.as_str(), TEST_POLICY_HASH);

        let runtime_session_id = start_guard_runtime(
            fixture.runtime_home.path(),
            "conn_guard_a",
            "2026-06-30T00:00:00Z",
        )?;
        let host_session_id = "session_guard_a";
        let session = bind_agent_session_runtime(
            fixture.runtime_home.path(),
            "project_guard_a",
            AgentSessionRuntimeBinding {
                runtime_session_id,
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: Some("guard_installation_a".to_owned()),
                host_session_id: host_session_id.to_owned(),
                host_thread_id: "thread_guard_a".to_owned(),
                host_turn_id: "turn_guard_a".to_owned(),
                observed_at: "2026-06-30T00:02:00Z".to_owned(),
            },
        )?;
        let session_id = session.session_id.clone();
        assert_eq!(session.session_id, session_id);

        let event = insert_guard_event(
            fixture.runtime_home.path(),
            "project_guard_a",
            GuardEventInsert {
                guard_event_id: "guard_event_a".to_owned(),
                session_id: Some(session_id.clone()),
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: "guard_installation_a".to_owned(),
                policy_hash: TEST_POLICY_HASH.to_owned(),
                integration_revision,
                event_kind: "pre_tool".to_owned(),
                contract_status: "compatible".to_owned(),
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
                session_id: session_id.clone(),
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
                session_id: Some(session_id.clone()),
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
                session_id: Some(session_id.clone()),
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
        let project_a =
            project_record_for_execution(fixture.runtime_home.path(), "project_guard_a")?
                .expect("fixture project A should exist");
        let connection_a =
            agent_connection_record_read_only(fixture.runtime_home.path(), "conn_guard_a")?
                .expect("fixture connection A should exist");
        let integration_revision = connection_integration_revision(&connection_a)?
            .as_str()
            .to_owned();
        upsert_guard_installation(
            fixture.runtime_home.path(),
            GuardInstallationUpsert {
                guard_installation_id: "guard_installation_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: "project_guard_a".to_owned(),
                manifest_json: test_guard_manifest_json(
                    &connection_a,
                    "project_guard_a",
                    &project_a.repo_root,
                    "guard_installation_a",
                    TEST_POLICY_HASH,
                ),
            },
        )?;

        let runtime_session_id = start_guard_runtime(
            fixture.runtime_home.path(),
            "conn_guard_a",
            "2026-06-30T00:59:00Z",
        )?;
        let session_id = bind_agent_session_runtime(
            fixture.runtime_home.path(),
            "project_guard_a",
            AgentSessionRuntimeBinding {
                runtime_session_id,
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: None,
                host_session_id: "session_guard_a".to_owned(),
                host_thread_id: "thread_guard_a".to_owned(),
                host_turn_id: "turn_guard_a".to_owned(),
                observed_at: "2026-06-30T01:00:00Z".to_owned(),
            },
        )?
        .session_id;
        insert_unrecorded_change(
            fixture.runtime_home.path(),
            "project_guard_a",
            UnrecordedChangeInsert {
                unrecorded_change_id: "unrecorded_change_a".to_owned(),
                session_id: Some(session_id.clone()),
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

        assert!(
            agent_session(fixture.runtime_home.path(), "project_guard_b", &session_id,)?.is_none()
        );
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
                guard_installation_id: "guard_installation_a".to_owned(),
                policy_hash: TEST_POLICY_HASH.to_owned(),
                integration_revision,
                event_kind: "pre_tool".to_owned(),
                contract_status: "compatible".to_owned(),
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
            StoreError::Conflict { .. }
                | StoreError::NotFound {
                    entity: "connection_project",
                    ..
                }
        ));

        let error = upsert_guard_installation(
            fixture.runtime_home.path(),
            GuardInstallationUpsert {
                guard_installation_id: "guard_installation_cross".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: "project_guard_b".to_owned(),
                manifest_json: test_guard_manifest_json(
                    &connection_a,
                    "project_guard_b",
                    &project_record_for_execution(fixture.runtime_home.path(), "project_guard_b")?
                        .expect("fixture project B should exist")
                        .repo_root,
                    "guard_installation_cross",
                    TEST_POLICY_HASH,
                ),
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
    fn guard_observation_is_owned_by_current_policy_hash_and_integration_revision(
    ) -> Result<(), Box<dyn Error>> {
        const OLD_POLICY_HASH: &str =
            "sha256:2222222222222222222222222222222222222222222222222222222222222222";
        const CURRENT_POLICY_HASH: &str =
            "sha256:3333333333333333333333333333333333333333333333333333333333333333";
        let fixture = GuardFixture::new("guard-current-observation")?;
        fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;
        let project = project_record_for_execution(fixture.runtime_home.path(), "project_guard_a")?
            .expect("fixture project");
        let connection =
            agent_connection_record_read_only(fixture.runtime_home.path(), "conn_guard_a")?
                .expect("fixture connection");
        let revision = connection_integration_revision(&connection)?
            .as_str()
            .to_owned();
        let upsert = |policy_hash: &str| {
            upsert_guard_installation(
                fixture.runtime_home.path(),
                GuardInstallationUpsert {
                    guard_installation_id: "guard_installation_a".to_owned(),
                    connection_internal_id: "conn_guard_a".to_owned(),
                    project_id: "project_guard_a".to_owned(),
                    manifest_json: test_guard_manifest_json(
                        &connection,
                        "project_guard_a",
                        &project.repo_root,
                        "guard_installation_a",
                        policy_hash,
                    ),
                },
            )
        };
        let insert_phase = |policy_hash: &str,
                            phase: GuardHookPhase,
                            suffix: &str,
                            contract_status: &str|
         -> StoreResult<GuardEventRecord> {
            insert_guard_event(
                fixture.runtime_home.path(),
                "project_guard_a",
                GuardEventInsert {
                    guard_event_id: format!("guard_event_{suffix}"),
                    session_id: None,
                    connection_internal_id: "conn_guard_a".to_owned(),
                    guard_installation_id: "guard_installation_a".to_owned(),
                    policy_hash: policy_hash.to_owned(),
                    integration_revision: revision.clone(),
                    event_kind: phase.as_str().to_owned(),
                    contract_status: contract_status.to_owned(),
                    decision: "allow".to_owned(),
                    subject_json: "{}".to_owned(),
                    result_json: "{}".to_owned(),
                    occurred_at: "2026-07-18T00:00:00Z".to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )
        };

        let old = upsert(OLD_POLICY_HASH)?;
        for phase in GuardHookPhase::REQUIRED {
            insert_phase(
                OLD_POLICY_HASH,
                phase,
                &format!("old_{}", phase.as_str()),
                GuardHookContractStatus::Compatible.as_str(),
            )?;
        }
        assert!(
            guard_observation_summary(fixture.runtime_home.path(), "project_guard_a", &old,)?
                .all_required_phases_observed()
        );

        let current = upsert(CURRENT_POLICY_HASH)?;
        let pending =
            guard_observation_summary(fixture.runtime_home.path(), "project_guard_a", &current)?;
        assert!(pending.observed_phases.is_empty());
        assert!(!pending.all_required_phases_observed());

        for phase in GuardHookPhase::REQUIRED {
            insert_phase(
                CURRENT_POLICY_HASH,
                phase,
                &format!("current_{}", phase.as_str()),
                GuardHookContractStatus::Compatible.as_str(),
            )?;
        }
        assert!(guard_observation_summary(
            fixture.runtime_home.path(),
            "project_guard_a",
            &current,
        )?
        .all_required_phases_observed());

        insert_phase(
            CURRENT_POLICY_HASH,
            GuardHookPhase::PreTool,
            "malformed_current",
            GuardHookContractStatus::Malformed.as_str(),
        )?;
        let failed =
            guard_observation_summary(fixture.runtime_home.path(), "project_guard_a", &current)?;
        assert_eq!(
            failed.incompatible_event_ids,
            ["guard_event_malformed_current"]
        );
        assert!(!failed.all_required_phases_observed());
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
