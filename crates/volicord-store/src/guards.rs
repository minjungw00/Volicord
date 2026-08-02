use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    str::FromStr,
};

use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use serde_json::Value;
use volicord_host_contract::{
    CanonicalToolName, CodexGuardToolEffectContract, CodexHookPromptCorrelation,
    CodexHookToolCorrelation, CodexMcpCorrelation, HostContractProfileId, HostNativeCorrelation,
    HostSessionId, HostToolUseId, HostTurnId,
};
use volicord_platform_fs::{
    resolve_git_worktree_layout, ObserverLimits, RepositoryDelta, SemanticObserverContractDigest,
};
use volicord_types::canonical::{
    canonical_json_sha256, canonical_json_string, is_canonical_sha256_digest,
};
use volicord_types::guard_manifest::{
    guard_manifest_from_json, guard_manifest_matches_owner_binding, GuardManifestOwnerBinding,
};
use volicord_types::integration_revision::{IntegrationRevision, ProjectIntegrationRevisionBasis};
use volicord_types::managed_mcp_client_info::project_agent_session_id;
use volicord_types::product_path::ProductRelativePath;
use volicord_types::schema::JsonObject;
use volicord_types::values::{
    ActorSource, GuardDecision, GuardHookContractStatus, GuardHookPhase, PromptCaptureStatus,
    UnrecordedChangeStatus, UtcTimestamp,
};

use crate::{
    agent_connections::{
        agent_connection_record_from_conn, agent_connection_record_read_only,
        is_agent_connection_project_allowed, AgentConnectionRecord,
    },
    bootstrap::{
        project_record_for_execution_admitted, project_record_for_execution_read_only,
        raw_project_record_from_conn, ProjectRecord,
    },
    operational_sessions::{
        connection_integration_revision, current_managed_mcp_runtime_session_for_connection,
        reserve_mcp_runtime_project_session, McpRuntimeProjectSessionReservation,
    },
    sqlite::{
        begin_immediate_transaction, open_project_state_database_for_mutation,
        open_project_state_database_read_only, open_registry_database_for_mutation,
        open_registry_database_read_only, registry_db_path,
    },
    RuntimeHomeMutationContext, StoreError, StoreResult,
};

mod repository_observation;
pub use repository_observation::*;

#[cfg(test)]
use crate::bootstrap::project_record_for_execution;

const GUARD_INTEGRATION_VERIFICATION_EVENT_LIMIT: usize = 512;

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

/// Idempotent normalized host-correlation observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCorrelationObservation {
    pub connection_internal_id: String,
    pub guard_installation_id: Option<String>,
    pub correlation: HostNativeCorrelation,
    pub observed_at: String,
}

/// Required managed-runtime attachment input for one project Agent Session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionRuntimeBinding {
    pub runtime_session_id: String,
    pub connection_internal_id: String,
    pub guard_installation_id: Option<String>,
    pub correlation: CodexMcpCorrelation,
    pub observed_at: String,
}

/// Store-derived current project Agent Session lifecycle coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectAgentSessionCoordinates {
    pub session_id: String,
    pub project_integration_revision: String,
    pub guard_installation_id: Option<String>,
}

/// Normalized host session row shared by hook and MCP observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostSessionRecord {
    pub project_id: String,
    pub session_id: String,
    pub connection_internal_id: String,
    pub project_integration_revision: String,
    pub host_session_id: String,
    pub first_observed_at: String,
    pub last_observed_at: String,
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
    pub correlation: Option<HostNativeCorrelation>,
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
    pub correlation: Option<HostNativeCorrelation>,
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

/// Prompt capture insert input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptCaptureInsert {
    pub prompt_capture_id: String,
    pub correlation: HostNativeCorrelation,
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
    pub correlation: HostNativeCorrelation,
    pub connection_internal_id: String,
    pub capture_kind: String,
    pub prompt_sha256: String,
    pub prompt_text: Option<String>,
    pub captured_at: String,
    pub metadata_json: String,
}

/// One accepted prompt capture and the prior-turn observations it atomically terminalized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptCaptureAggregateRecord {
    pub prompt_capture: PromptCaptureRecord,
    pub terminalized_repository_observations: Vec<RepositoryObservationRecord>,
}

/// Unrecorded Product Repository change resolution input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecordedChangeResolution {
    pub resolution: JsonObject,
    pub resolved_at: UtcTimestamp,
    pub resolved_by_actor_source: ActorSource,
}

/// Unrecorded Product Repository change row stored in project `state.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecordedChangeRecord {
    pub project_id: String,
    pub unrecorded_change_id: String,
    pub repository_observation_id: String,
    pub session_id: String,
    pub correlation: HostNativeCorrelation,
    pub connection_internal_id: String,
    pub task_id: Option<String>,
    pub status: UnrecordedChangeStatus,
    pub summary: String,
    pub observed_paths: Vec<ProductRelativePath>,
    pub unmatched_delta_digest: String,
    pub detection: JsonObject,
    pub resolution: Option<JsonObject>,
    pub detected_at: UtcTimestamp,
    pub resolved_at: Option<UtcTimestamp>,
    pub resolved_by_actor_source: Option<ActorSource>,
    pub metadata: JsonObject,
}

#[derive(Debug)]
struct UnrecordedChangeRaw {
    project_id: String,
    unrecorded_change_id: String,
    repository_observation_id: String,
    session_id: Option<String>,
    connection_internal_id: Option<String>,
    host_session_id: Option<String>,
    host_turn_id: Option<String>,
    host_tool_use_id: Option<String>,
    host_tool_name: Option<String>,
    observation_state: Option<String>,
    observation_delta_json: Option<String>,
    observation_delta_digest: Option<String>,
    observation_completed_at: Option<String>,
    observation_terminal_result_json: Option<String>,
    task_id: Option<String>,
    status: String,
    summary: String,
    observed_paths_json: String,
    unmatched_delta_digest: String,
    detection_json: String,
    resolution_json: Option<String>,
    detected_at: String,
    resolved_at: Option<String>,
    resolved_by_actor_source: Option<String>,
    metadata_json: String,
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
    context: &RuntimeHomeMutationContext<'_>,
    input: GuardInstallationUpsert,
) -> StoreResult<GuardInstallationRecord> {
    let runtime_home = context.runtime_home().as_path();
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    upsert_guard_installation_in_transaction(&tx, &input)?;
    tx.commit()?;

    guard_installation(runtime_home, &input.guard_installation_id)?.ok_or_else(|| {
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
            updated_at = CASE
                WHEN guard_installations.manifest_json <> excluded.manifest_json
                THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                ELSE guard_installations.updated_at
            END",
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

/// Derives the only current project Agent Session coordinates for host-native correlation metadata.
pub fn current_project_agent_session_coordinates(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    connection_internal_id: &str,
    asserted_guard_installation_id: Option<&str>,
    correlation: &HostNativeCorrelation,
) -> StoreResult<ProjectAgentSessionCoordinates> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("connection_internal_id", connection_internal_id)?;
    if let Some(guard_installation_id) = asserted_guard_installation_id {
        validate_identifier("guard_installation_id", guard_installation_id)?;
    }
    let host_session_id = correlation.session_id().as_str();
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
    let observer_contract_digest =
        SemanticObserverContractDigest::for_limits(&ObserverLimits::default());
    let effect_catalog_digest = CodexGuardToolEffectContract::semantic_digest();
    let guidance_digest = volicord_types::managed_guidance::managed_guidance_semantic_digest();
    let project_revision = IntegrationRevision::for_project(ProjectIntegrationRevisionBasis {
        connection_integration_revision: connection_revision.as_str(),
        project_id: &project.project.project_id,
        policy_fingerprint: &policy_fingerprint,
        guard_installation_id: guard_ownership.as_ref().map(|value| value.0.as_str()),
        guard_policy_hash: guard_ownership.as_ref().map(|value| value.1.as_str()),
        repository_observer_contract_digest: observer_contract_digest.as_str(),
        product_repository_effect_catalog_digest: effect_catalog_digest.as_str(),
        managed_guidance_semantic_digest: guidance_digest.as_str(),
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
        detail: format!("project Agent Session coordinates could not be derived: {error}"),
    })?;
    Ok(ProjectAgentSessionCoordinates {
        session_id,
        project_integration_revision: project_revision.into_inner(),
        guard_installation_id: guard_ownership.map(|value| value.0),
    })
}

/// Creates or updates normalized host session, turn, and optional tool records.
pub fn observe_host_correlation(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    input: HostCorrelationObservation,
) -> StoreResult<HostSessionRecord> {
    validate_host_correlation_observation(&input)?;
    let runtime_home = context.runtime_home().as_path().to_path_buf();
    let observed_at = canonical_agent_session_observed_at(&input.observed_at)?;
    let coordinates = current_project_agent_session_coordinates(
        runtime_home,
        project_id,
        &input.connection_internal_id,
        input.guard_installation_id.as_deref(),
        &input.correlation,
    )?;
    establish_host_correlation(
        context,
        project_id,
        &coordinates,
        &input.connection_internal_id,
        &input.correlation,
        &observed_at,
    )
}

/// Validates, reserves, and attaches one managed runtime to a project Agent Session.
pub fn bind_agent_session_runtime(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    input: AgentSessionRuntimeBinding,
) -> StoreResult<AgentSessionRecord> {
    validate_agent_session_runtime_binding(&input)?;
    let correlation = &input.correlation;
    let runtime_home = context.runtime_home().as_path();
    let observed_at = canonical_agent_session_observed_at(&input.observed_at)?;

    // Phase 0: validate current managed-runtime facts without project mutation.
    let runtime = current_managed_mcp_runtime_session_for_connection(
        runtime_home,
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
    let coordinates = current_project_agent_session_coordinates(
        runtime_home,
        project_id,
        &input.connection_internal_id,
        input.guard_installation_id.as_deref(),
        &HostNativeCorrelation::CodexMcp(input.correlation.clone()),
    )?;

    // Phase 1: establish normalized host records and the exact MCP-only anchor.
    establish_host_correlation(
        context,
        project_id,
        &coordinates,
        &input.connection_internal_id,
        &HostNativeCorrelation::CodexMcp(input.correlation.clone()),
        &observed_at,
    )?;
    establish_agent_session_anchor(
        context,
        project_id,
        &coordinates,
        AgentSessionAnchorInput {
            requested_runtime_session_id: Some(&input.runtime_session_id),
            connection_internal_id: &input.connection_internal_id,
            correlation,
            observed_at: &observed_at,
        },
    )?;

    // Phase 2: reserve only the exact current coordinates validated by Phase 1.
    reserve_mcp_runtime_project_session(
        context,
        McpRuntimeProjectSessionReservation {
            runtime_session_id: &input.runtime_session_id,
            connection_internal_id: &input.connection_internal_id,
            project_id,
            asserted_guard_installation_id: input.guard_installation_id.as_deref(),
            expected_coordinates: &coordinates,
            correlation,
            bound_at: &observed_at,
        },
    )?;

    // Phase 3: attach only after the authoritative Registry reservation exists.
    attach_agent_session_runtime(
        context,
        project_id,
        &coordinates,
        &input.runtime_session_id,
        &input.connection_internal_id,
        correlation,
    )
}

#[derive(Debug, Clone, Copy)]
struct AgentSessionAnchorInput<'a> {
    requested_runtime_session_id: Option<&'a str>,
    connection_internal_id: &'a str,
    correlation: &'a CodexMcpCorrelation,
    observed_at: &'a str,
}

fn establish_host_correlation(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    coordinates: &ProjectAgentSessionCoordinates,
    connection_internal_id: &str,
    correlation: &HostNativeCorrelation,
    observed_at: &str,
) -> StoreResult<HostSessionRecord> {
    let mut project = open_guard_project(context, project_id, connection_internal_id)?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    establish_host_correlation_in_transaction(
        &tx,
        &project.project.project_id,
        &coordinates.session_id,
        &coordinates.project_integration_revision,
        connection_internal_id,
        correlation,
        observed_at,
    )?;
    tx.commit()?;
    host_session_by_conn(
        &project.conn,
        &project.project.project_id,
        &coordinates.session_id,
    )
}

fn establish_host_correlation_in_transaction(
    tx: &Transaction<'_>,
    project_id: &str,
    session_id: &str,
    project_integration_revision: &str,
    connection_internal_id: &str,
    correlation: &HostNativeCorrelation,
    observed_at: &str,
) -> StoreResult<()> {
    let host_session_id = correlation.session_id().as_str();
    if let Some(existing) = host_session_from_conn(tx, project_id, session_id)? {
        if existing.connection_internal_id != connection_internal_id
            || existing.project_integration_revision != project_integration_revision
            || existing.host_session_id != host_session_id
        {
            return Err(StoreError::Conflict {
                entity: "host_session",
                id: session_id.to_owned(),
                detail: "host session is already bound to different owner coordinates".to_owned(),
            });
        }
        let existing_first = strict_stored_timestamp(
            "host_sessions",
            &existing.session_id,
            "first_observed_at",
            &existing.first_observed_at,
        )?;
        let existing_last = strict_stored_timestamp(
            "host_sessions",
            &existing.session_id,
            "last_observed_at",
            &existing.last_observed_at,
        )?;
        let observation = UtcTimestamp::parse(observed_at).expect("validated observation");
        let first = if observation < existing_first {
            observed_at
        } else {
            existing.first_observed_at.as_str()
        };
        let last = if observation >= existing_last {
            observed_at
        } else {
            existing.last_observed_at.as_str()
        };
        tx.execute(
            "UPDATE host_sessions
                SET first_observed_at = ?3, last_observed_at = ?4
              WHERE project_id = ?1 AND session_id = ?2",
            params![project_id, session_id, first, last],
        )?;
    } else {
        tx.execute(
            "INSERT INTO host_sessions (
                project_id, session_id, connection_internal_id,
                project_integration_revision, host_session_id,
                first_observed_at, last_observed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![
                project_id,
                session_id,
                connection_internal_id,
                project_integration_revision,
                host_session_id,
                observed_at,
            ],
        )?;
    }

    let existing_turn: Option<(String, String, String)> = tx
        .query_row(
            "SELECT connection_internal_id, first_observed_at, last_observed_at
              FROM host_turns
              WHERE project_id = ?1 AND session_id = ?2 AND host_turn_id = ?3",
            params![project_id, session_id, correlation.turn_id().as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    if let Some((existing_connection, existing_first, existing_last)) = existing_turn {
        if existing_connection != connection_internal_id {
            return Err(StoreError::Conflict {
                entity: "host_turn",
                id: correlation.turn_id().as_str().to_owned(),
                detail: "host turn is already bound to another Connection".to_owned(),
            });
        }
        let first = earlier_timestamp(
            "host_turns",
            correlation.turn_id().as_str(),
            &existing_first,
            observed_at,
        )?;
        let last = later_timestamp(
            "host_turns",
            correlation.turn_id().as_str(),
            &existing_last,
            observed_at,
        )?;
        tx.execute(
            "UPDATE host_turns
                SET first_observed_at = ?4, last_observed_at = ?5
              WHERE project_id = ?1 AND session_id = ?2 AND host_turn_id = ?3",
            params![
                project_id,
                session_id,
                correlation.turn_id().as_str(),
                first,
                last,
            ],
        )?;
    } else {
        tx.execute(
            "INSERT INTO host_turns (
                project_id, session_id, connection_internal_id, host_turn_id,
                first_observed_at, last_observed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![
                project_id,
                session_id,
                connection_internal_id,
                correlation.turn_id().as_str(),
                observed_at,
            ],
        )?;
    }

    if let HostNativeCorrelation::CodexHookTool(tool) = correlation {
        let existing_tool: Option<(String, String, String)> = tx
            .query_row(
                "SELECT host_turn_id, host_tool_name, last_observed_at
                  FROM host_tool_invocations
                  WHERE project_id = ?1 AND session_id = ?2 AND host_tool_use_id = ?3",
                params![project_id, session_id, tool.tool_use_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        if let Some((turn_id, tool_name, last_observed_at)) = existing_tool {
            if turn_id != tool.turn_id.as_str() || tool_name != tool.tool_name.as_str() {
                return Err(StoreError::Conflict {
                    entity: "host_tool_invocation",
                    id: tool.tool_use_id.as_str().to_owned(),
                    detail: "tool-use ID is already bound to a different turn or tool name"
                        .to_owned(),
                });
            }
            let last = later_timestamp(
                "host_tool_invocations",
                tool.tool_use_id.as_str(),
                &last_observed_at,
                observed_at,
            )?;
            tx.execute(
                "UPDATE host_tool_invocations
                    SET last_observed_at = ?4
                  WHERE project_id = ?1 AND session_id = ?2 AND host_tool_use_id = ?3",
                params![project_id, session_id, tool.tool_use_id.as_str(), last,],
            )?;
        } else {
            tx.execute(
                "INSERT INTO host_tool_invocations (
                    project_id, session_id, connection_internal_id, host_turn_id,
                    host_tool_use_id, host_tool_name, first_observed_at, last_observed_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    project_id,
                    session_id,
                    connection_internal_id,
                    tool.turn_id.as_str(),
                    tool.tool_use_id.as_str(),
                    tool.tool_name.as_str(),
                    observed_at,
                ],
            )?;
        }
    }
    Ok(())
}

fn earlier_timestamp<'a>(
    entity: &'static str,
    id: &str,
    existing: &'a str,
    candidate: &'a str,
) -> StoreResult<&'a str> {
    let existing_value = strict_stored_timestamp(entity, id, "first_observed_at", existing)?;
    let candidate_value = UtcTimestamp::parse(candidate).expect("validated observation");
    Ok(if candidate_value < existing_value {
        candidate
    } else {
        existing
    })
}

fn later_timestamp<'a>(
    entity: &'static str,
    id: &str,
    existing: &'a str,
    candidate: &'a str,
) -> StoreResult<&'a str> {
    let existing_value = strict_stored_timestamp(entity, id, "last_observed_at", existing)?;
    let candidate_value = UtcTimestamp::parse(candidate).expect("validated observation");
    Ok(if candidate_value >= existing_value {
        candidate
    } else {
        existing
    })
}

fn establish_agent_session_anchor(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    coordinates: &ProjectAgentSessionCoordinates,
    input: AgentSessionAnchorInput<'_>,
) -> StoreResult<AgentSessionRecord> {
    let mut project = open_guard_project(context, project_id, input.connection_internal_id)?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    if let Some(runtime_session_id) = input.requested_runtime_session_id {
        let attached_session_id = tx
            .query_row(
                "SELECT session_id
                   FROM managed_mcp_sessions
                  WHERE project_id = ?1 AND runtime_session_id = ?2",
                params![project.project.project_id, runtime_session_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if attached_session_id
            .as_deref()
            .is_some_and(|session_id| session_id != coordinates.session_id)
        {
            return Err(StoreError::Conflict {
                entity: "agent_session",
                id: coordinates.session_id.clone(),
                detail: "managed runtime is already attached to another project Agent Session"
                    .to_owned(),
            });
        }
    }
    if let Some(existing) =
        agent_session_from_conn(&tx, &project.project.project_id, &coordinates.session_id)?
    {
        if existing.connection_internal_id != input.connection_internal_id
            || existing.host_session_id != input.correlation.session_id.as_str()
            || existing.host_thread_id != input.correlation.thread_id.as_str()
        {
            return Err(StoreError::Conflict {
                entity: "agent_session",
                id: coordinates.session_id.clone(),
                detail: "Agent Session is already bound to another Connection or host-native session correlation"
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
                    id: coordinates.session_id.clone(),
                    detail: "Agent Session is already attached to another runtime".to_owned(),
                });
            }
        }
        if existing.project_integration_revision != coordinates.project_integration_revision {
            return Err(StoreError::Conflict {
                entity: "agent_session",
                id: coordinates.session_id.clone(),
                detail: "Agent Session integration revision is immutable".to_owned(),
            });
        }
        let existing_first = strict_stored_timestamp(
            "managed_mcp_sessions",
            &existing.session_id,
            "first_observed_at",
            &existing.first_observed_at,
        )?;
        let existing_last = strict_stored_timestamp(
            "managed_mcp_sessions",
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
            (input.correlation.turn_id.as_str(), input.observed_at)
        } else {
            (
                existing.last_host_turn_id.as_str(),
                existing.last_observed_at.as_str(),
            )
        };
        tx.execute(
            "UPDATE managed_mcp_sessions
                SET last_host_turn_id = ?3,
                    first_observed_at = ?4,
                    last_observed_at = ?5
              WHERE project_id = ?1 AND session_id = ?2",
            params![
                project.project.project_id,
                coordinates.session_id,
                last_host_turn_id,
                first_observed_at,
                last_observed_at,
            ],
        )?;
    } else {
        tx.execute(
            "INSERT INTO managed_mcp_sessions (
                project_id, session_id, runtime_session_id, connection_internal_id,
                host_thread_id, last_host_turn_id, first_observed_at, last_observed_at
            ) VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?6)",
            params![
                project.project.project_id,
                coordinates.session_id,
                input.connection_internal_id,
                input.correlation.thread_id.as_str(),
                input.correlation.turn_id.as_str(),
                input.observed_at,
            ],
        )?;
    }
    tx.commit()?;

    agent_session_by_conn(
        &project.conn,
        &project.project.project_id,
        &coordinates.session_id,
    )
}

fn attach_agent_session_runtime(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    coordinates: &ProjectAgentSessionCoordinates,
    runtime_session_id: &str,
    connection_internal_id: &str,
    correlation: &CodexMcpCorrelation,
) -> StoreResult<AgentSessionRecord> {
    let mut project = open_guard_project(context, project_id, connection_internal_id)?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    let existing =
        agent_session_from_conn(&tx, &project.project.project_id, &coordinates.session_id)?
            .ok_or_else(|| StoreError::Conflict {
                entity: "agent_session",
                id: coordinates.session_id.clone(),
                detail: "validated project Agent Session anchor is no longer present".to_owned(),
            })?;
    if existing.connection_internal_id != connection_internal_id
        || existing.host_session_id != correlation.session_id.as_str()
        || existing.host_thread_id != correlation.thread_id.as_str()
        || existing.project_integration_revision != coordinates.project_integration_revision
    {
        return Err(StoreError::Conflict {
            entity: "agent_session",
            id: coordinates.session_id.clone(),
            detail: "project Agent Session anchor ownership changed before runtime attachment"
                .to_owned(),
        });
    }
    match existing.runtime_session_id.as_deref() {
        Some(existing_runtime) if existing_runtime == runtime_session_id => {}
        Some(_) => {
            return Err(StoreError::Conflict {
                entity: "agent_session",
                id: coordinates.session_id.clone(),
                detail: "Agent Session is already attached to another runtime".to_owned(),
            })
        }
        None => {
            tx.execute(
                "UPDATE managed_mcp_sessions
                    SET runtime_session_id = ?3
                  WHERE project_id = ?1 AND session_id = ?2 AND runtime_session_id IS NULL",
                params![
                    project.project.project_id,
                    coordinates.session_id,
                    runtime_session_id
                ],
            )?;
        }
    }
    tx.commit()?;
    agent_session_by_conn(
        &project.conn,
        &project.project.project_id,
        &coordinates.session_id,
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
    let correlation = decoded_codex_mcp_correlation(session)?;
    let coordinates = match current_project_agent_session_coordinates(
        runtime_home,
        &session.project_id,
        &session.connection_internal_id,
        guard_installation_id,
        &correlation,
    ) {
        Ok(coordinates) => coordinates,
        Err(StoreError::Conflict { .. } | StoreError::NotFound { .. }) => return Ok(false),
        Err(error) => return Err(error),
    };
    Ok(coordinates.session_id == session.session_id
        && coordinates.project_integration_revision == session.project_integration_revision)
}

/// Inserts one project-scoped guard event row.
pub fn insert_guard_event(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    input: GuardEventInsert,
) -> StoreResult<GuardEventRecord> {
    validate_guard_event_insert(&input)?;
    let runtime_home = context.runtime_home().as_path();
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
    let correlation_fields = input
        .correlation
        .as_ref()
        .map(|correlation| {
            guard_correlation_fields(
                runtime_home,
                project_id,
                &input.connection_internal_id,
                Some(&input.guard_installation_id),
                correlation,
            )
        })
        .transpose()?;
    let mut project = open_guard_project(context, project_id, &input.connection_internal_id)?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    tx.execute(
        "INSERT INTO guard_events (
            project_id,
            guard_event_id,
            session_id,
            connection_internal_id,
            correlation_kind,
            host_turn_id,
            host_tool_use_id,
            host_tool_name,
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
        VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18
        )",
        params![
            project.project.project_id,
            input.guard_event_id,
            correlation_fields
                .as_ref()
                .map(|fields| fields.session_id.as_str()),
            input.connection_internal_id,
            input.correlation.as_ref().map(HostNativeCorrelation::kind),
            correlation_fields
                .as_ref()
                .map(|fields| fields.host_turn_id.as_str()),
            correlation_fields
                .as_ref()
                .and_then(|fields| fields.host_tool_use_id.as_deref()),
            correlation_fields
                .as_ref()
                .and_then(|fields| fields.host_tool_name.as_deref()),
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

#[derive(Debug)]
struct GuardCorrelationFields {
    session_id: String,
    project_integration_revision: String,
    host_turn_id: String,
    host_tool_use_id: Option<String>,
    host_tool_name: Option<String>,
}

fn guard_correlation_fields(
    runtime_home: &Path,
    project_id: &str,
    connection_internal_id: &str,
    guard_installation_id: Option<&str>,
    correlation: &HostNativeCorrelation,
) -> StoreResult<GuardCorrelationFields> {
    let (host_turn_id, host_tool_use_id, host_tool_name) = match correlation {
        HostNativeCorrelation::CodexHookPrompt(value) => (value.turn_id.as_str(), None, None),
        HostNativeCorrelation::CodexHookTool(value) => (
            value.turn_id.as_str(),
            Some(value.tool_use_id.as_str()),
            Some(value.tool_name.as_str()),
        ),
        HostNativeCorrelation::CodexMcp(_) => {
            return Err(StoreError::InvalidInput {
                detail: "Guard records require Codex hook correlation".to_owned(),
            })
        }
    };
    let coordinates = current_project_agent_session_coordinates(
        runtime_home,
        project_id,
        connection_internal_id,
        guard_installation_id,
        correlation,
    )?;
    Ok(GuardCorrelationFields {
        session_id: coordinates.session_id,
        project_integration_revision: coordinates.project_integration_revision,
        host_turn_id: host_turn_id.to_owned(),
        host_tool_use_id: host_tool_use_id.map(str::to_owned),
        host_tool_name: host_tool_name.map(str::to_owned),
    })
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

/// Exact owner and native-turn coordinate for bounded integration-verification events.
#[derive(Debug, Clone, Copy)]
pub struct GuardIntegrationVerificationEventQuery<'a> {
    pub project_id: &'a str,
    pub connection_internal_id: &'a str,
    pub session_id: &'a str,
    pub host_turn_id: &'a str,
    pub guard_installation_id: &'a str,
    pub policy_hash: &'a str,
    pub integration_revision: &'a str,
}

/// Reads the bounded Guard events eligible for one exact integration-verification coordinate.
pub fn guard_events_for_integration_verification(
    runtime_home: impl AsRef<Path>,
    query: GuardIntegrationVerificationEventQuery<'_>,
) -> StoreResult<Vec<GuardEventRecord>> {
    for (field, value) in [
        ("project_id", query.project_id),
        ("connection_internal_id", query.connection_internal_id),
        ("session_id", query.session_id),
        ("host_turn_id", query.host_turn_id),
        ("guard_installation_id", query.guard_installation_id),
        ("policy_hash", query.policy_hash),
        ("integration_revision", query.integration_revision),
    ] {
        validate_identifier(field, value)?;
    }
    let Some(project) = open_project_for_read(runtime_home, query.project_id)? else {
        return Ok(Vec::new());
    };
    let mut stmt = project.conn.prepare(
        "SELECT
            e.project_id, e.guard_event_id, e.session_id,
            e.connection_internal_id, e.correlation_kind, h.host_session_id,
            e.host_turn_id, e.host_tool_use_id, e.host_tool_name,
            e.guard_installation_id, e.policy_hash, e.integration_revision,
            e.event_kind, e.contract_status, e.decision, e.subject_json,
            e.result_json, e.occurred_at, e.metadata_json
           FROM guard_events AS e
           LEFT JOIN host_sessions AS h
             ON h.project_id = e.project_id
            AND h.session_id = e.session_id
            AND h.connection_internal_id = e.connection_internal_id
          WHERE e.project_id = ?1
            AND e.connection_internal_id = ?2
            AND e.session_id = ?3
            AND e.host_turn_id = ?4
            AND e.guard_installation_id = ?5
            AND e.policy_hash = ?6
            AND e.integration_revision = ?7
          ORDER BY volicord_utc_seconds(e.occurred_at),
                   volicord_utc_subsec_nanos(e.occurred_at),
                   e.guard_event_id
          LIMIT 513",
    )?;
    let rows = stmt.query_map(
        params![
            project.project.project_id,
            query.connection_internal_id,
            query.session_id,
            query.host_turn_id,
            query.guard_installation_id,
            query.policy_hash,
            query.integration_revision,
        ],
        guard_event_from_row,
    )?;
    let records = collect_rows(rows)?;
    if records.len() > GUARD_INTEGRATION_VERIFICATION_EVENT_LIMIT {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "integration-verification event window exceeds the bounded event limit of {}",
                GUARD_INTEGRATION_VERIFICATION_EVENT_LIMIT
            ),
        });
    }
    Ok(records)
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
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    input: PromptCaptureInsert,
) -> StoreResult<PromptCaptureRecord> {
    insert_prompt_capture_and_terminalize_prior_turn_observations(context, project_id, input)
        .map(|aggregate| aggregate.prompt_capture)
}

/// Atomically records an accepted prompt and closes prior-turn open observations.
pub fn insert_prompt_capture_and_terminalize_prior_turn_observations(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    input: PromptCaptureInsert,
) -> StoreResult<PromptCaptureAggregateRecord> {
    validate_prompt_capture_insert(&input)?;
    let runtime_home = context.runtime_home().as_path();
    let fields = guard_correlation_fields(
        runtime_home,
        project_id,
        &input.connection_internal_id,
        None,
        &input.correlation,
    )?;
    let mut project = open_guard_project(context, project_id, &input.connection_internal_id)?;
    let tx = begin_immediate_transaction(&mut project.conn)?;
    let existing =
        prompt_capture_from_conn(&tx, &project.project.project_id, &input.prompt_capture_id)?;
    if let Some(existing) = &existing {
        let expected_correlation = &input.correlation;
        if existing.session_id != fields.session_id
            || &existing.correlation != expected_correlation
            || existing.connection_internal_id != input.connection_internal_id
            || existing.capture_kind != input.capture_kind
            || existing.prompt_sha256 != input.prompt_sha256
            || existing.prompt_text != input.prompt_text
            || existing.captured_at != input.captured_at
            || existing.metadata_json != input.metadata_json
        {
            return Err(StoreError::Conflict {
                entity: "prompt_capture",
                id: input.prompt_capture_id.clone(),
                detail: "prompt capture replay does not match the stored accepted prompt"
                    .to_owned(),
            });
        }
    } else {
        tx.execute(
            "INSERT INTO prompt_captures (
                project_id,
                prompt_capture_id,
                session_id,
                connection_internal_id,
                host_turn_id,
                capture_kind,
                prompt_sha256,
                prompt_text,
                captured_at,
                metadata_json
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                project.project.project_id,
                input.prompt_capture_id,
                fields.session_id,
                input.connection_internal_id,
                fields.host_turn_id,
                input.capture_kind,
                input.prompt_sha256,
                input.prompt_text,
                input.captured_at,
                input.metadata_json
            ],
        )?;
    }
    let completed_at =
        UtcTimestamp::parse(&input.captured_at).map_err(|_| StoreError::InvalidInput {
            detail: "prompt capture time must be a canonical RFC 3339 timestamp".to_owned(),
        })?;
    let current_turn_id = match &input.correlation {
        HostNativeCorrelation::CodexHookPrompt(correlation) => correlation.turn_id.clone(),
        _ => unreachable!("prompt correlation was validated before transaction admission"),
    };
    let terminalized_repository_observations =
        terminalize_open_repository_observations_in_transaction(
            &tx,
            &project.project.project_id,
            &TerminalizeOpenRepositoryObservations {
                connection_internal_id: input.connection_internal_id.clone(),
                session_id: fields.session_id,
                scope: OpenRepositoryObservationTerminalizationScope::EarlierTurns {
                    current_turn_id,
                },
                reason: RepositoryObservationUnavailableReason::PostToolNotObserved,
                completed_at,
            },
        )?;
    let prompt_capture =
        prompt_capture_by_conn(&tx, &project.project.project_id, &input.prompt_capture_id)?;
    tx.commit()?;
    Ok(PromptCaptureAggregateRecord {
        prompt_capture,
        terminalized_repository_observations,
    })
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
    unresolved_unrecorded_changes_from_conn(
        &project.conn,
        &project.project.project_id,
        connection_internal_id,
    )
}

pub(crate) fn unresolved_unrecorded_changes_from_conn(
    conn: &Connection,
    project_id: &str,
    connection_internal_id: Option<&str>,
) -> StoreResult<Vec<UnrecordedChangeRecord>> {
    validate_identifier("project_id", project_id)?;
    if let Some(connection_internal_id) = connection_internal_id {
        validate_identifier("connection_internal_id", connection_internal_id)?;
    }
    let mut stmt = conn.prepare(
        "SELECT
            u.project_id, u.unrecorded_change_id, u.repository_observation_id,
            o.session_id, o.connection_internal_id, h.host_session_id,
            o.host_turn_id, o.host_tool_use_id, o.host_tool_name, o.state,
            o.delta_json, o.delta_digest, o.completed_at, o.terminal_result_json,
            u.task_id, u.status, u.summary, u.observed_paths_json,
            u.unmatched_delta_digest, u.detection_json, u.resolution_json,
            u.detected_at, u.resolved_at, u.resolved_by_actor_source,
            u.metadata_json
         FROM unrecorded_changes AS u
         LEFT JOIN repository_observations AS o
           ON o.project_id = u.project_id
          AND o.repository_observation_id = u.repository_observation_id
         LEFT JOIN host_sessions AS h
           ON h.project_id = o.project_id
          AND h.session_id = o.session_id
          AND h.connection_internal_id = o.connection_internal_id
        WHERE u.project_id = ?1
        ORDER BY volicord_utc_seconds(u.detected_at),
                 volicord_utc_subsec_nanos(u.detected_at),
                 u.unrecorded_change_id",
    )?;
    let rows = stmt.query_map([project_id], unrecorded_change_raw_from_row)?;
    let mut records = Vec::new();
    for row in rows {
        let record = decode_unrecorded_change(row?)?;
        if record.status == UnrecordedChangeStatus::Unresolved
            && connection_internal_id
                .is_none_or(|connection| record.connection_internal_id == connection)
        {
            records.push(record);
        }
    }
    Ok(records)
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
            AND (
              volicord_utc_seconds(occurred_at)
                  > volicord_utc_seconds(?6)
              OR (
                volicord_utc_seconds(occurred_at)
                    = volicord_utc_seconds(?6)
                AND volicord_utc_subsec_nanos(occurred_at)
                    >= volicord_utc_subsec_nanos(?6)
              )
            )
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
            installation.updated_at,
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
                m.project_id,
                m.session_id,
                m.runtime_session_id,
                m.connection_internal_id,
                h.project_integration_revision,
                h.host_session_id,
                m.host_thread_id,
                m.last_host_turn_id,
                min(h.first_observed_at, m.first_observed_at),
                max(h.last_observed_at, m.last_observed_at)
             FROM managed_mcp_sessions AS m
             JOIN host_sessions AS h
               ON h.project_id = m.project_id
              AND h.session_id = m.session_id
              AND h.connection_internal_id = m.connection_internal_id
            WHERE m.project_id = ?1
              AND m.connection_internal_id = ?2
            ORDER BY volicord_utc_seconds(max(h.last_observed_at, m.last_observed_at)) DESC,
                     volicord_utc_subsec_nanos(max(h.last_observed_at, m.last_observed_at)) DESC,
                     m.session_id DESC
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
            "managed_mcp_sessions",
            &records[0].session_id,
            "last_observed_at",
            &records[0].last_observed_at,
        )?;
        let second = strict_stored_timestamp(
            "managed_mcp_sessions",
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
                correlation_kind,
                host_turn_id,
                host_tool_use_id,
                host_tool_name,
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
            c.project_id, c.guard_event_id, c.session_id, c.connection_internal_id,
            c.correlation_kind, h.host_session_id, c.host_turn_id,
            c.host_tool_use_id, c.host_tool_name, c.guard_installation_id,
            c.policy_hash, c.integration_revision, c.event_kind, c.contract_status,
            c.decision, c.subject_json, c.result_json, c.occurred_at, c.metadata_json
          FROM candidates AS c
          LEFT JOIN host_sessions AS h
            ON h.project_id = c.project_id
           AND h.session_id = c.session_id
           AND h.connection_internal_id = c.connection_internal_id,
               latest_seconds, latest_instant
         WHERE c.utc_seconds = latest_seconds.value
           AND c.utc_subsec_nanos = latest_instant.value
         ORDER BY c.guard_event_id DESC",
    )?;
    let rows = stmt.query_map(
        params![project.project.project_id, connection_internal_id],
        guard_event_from_row,
    )?;
    collect_rows(rows)
}

/// Resolves one unresolved unrecorded-change row.
pub fn resolve_unrecorded_change(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    unrecorded_change_id: &str,
    resolution: UnrecordedChangeResolution,
) -> StoreResult<UnrecordedChangeRecord> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("unrecorded_change_id", unrecorded_change_id)?;
    validate_unrecorded_change_resolution(&resolution)?;
    let resolution_json =
        canonical_json_string(&resolution.resolution).map_err(|_| StoreError::InvalidInput {
            detail: "unrecorded change resolution cannot be serialized".to_owned(),
        })?;
    let mut project =
        open_project_for_mutation(context, project_id)?.ok_or_else(|| StoreError::NotFound {
            entity: "project",
            id: project_id.to_owned(),
        })?;
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
            resolution_json,
            resolution.resolved_at.to_string(),
            resolution.resolved_by_actor_source.to_canonical_string()
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
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    connection_internal_id: &str,
) -> StoreResult<OpenGuardProject> {
    validate_identifier("project_id", project_id)?;
    validate_identifier("connection_internal_id", connection_internal_id)?;
    if !is_agent_connection_project_allowed(
        context.runtime_home().as_path(),
        connection_internal_id,
        project_id,
    )? {
        return Err(StoreError::NotFound {
            entity: "connection_project",
            id: format!("{connection_internal_id}/{project_id}"),
        });
    }
    open_project_for_mutation(context, project_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "project",
        id: project_id.to_owned(),
    })
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

fn open_project_for_mutation(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
) -> StoreResult<Option<OpenGuardProject>> {
    let Some(project) = project_record_for_execution_admitted(context, project_id)? else {
        return Ok(None);
    };
    let conn = open_project_state_database_for_mutation(context, &project)?;
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
    let manifest =
        guard_manifest_from_json(&input.manifest_json).map_err(|_| StoreError::InvalidInput {
            detail:
                "guard_installations.manifest_json must be one canonical current Guard manifest"
                    .to_owned(),
        })?;
    validate_current_guard_host_contract(&manifest)?;
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
    validate_current_guard_host_contract(&manifest)?;
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

fn validate_host_correlation_observation(input: &HostCorrelationObservation) -> StoreResult<()> {
    if matches!(&input.correlation, HostNativeCorrelation::CodexMcp(_)) {
        return Err(StoreError::InvalidInput {
            detail: "hook observation cannot use managed MCP correlation".to_owned(),
        });
    }
    validate_host_correlation_metadata(
        &input.connection_internal_id,
        input.guard_installation_id.as_deref(),
        &input.observed_at,
    )
}

fn validate_agent_session_runtime_binding(input: &AgentSessionRuntimeBinding) -> StoreResult<()> {
    validate_identifier("runtime_session_id", &input.runtime_session_id)?;
    validate_host_correlation_metadata(
        &input.connection_internal_id,
        input.guard_installation_id.as_deref(),
        &input.observed_at,
    )
}

fn validate_host_correlation_metadata(
    connection_internal_id: &str,
    guard_installation_id: Option<&str>,
    observed_at: &str,
) -> StoreResult<()> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    if let Some(guard_installation_id) = guard_installation_id {
        validate_identifier("guard_installation_id", guard_installation_id)?;
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
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    validate_identifier("guard_installation_id", &input.guard_installation_id)?;
    volicord_types::guard_manifest::PolicyHash::parse(input.policy_hash.clone()).map_err(|_| {
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
    let correlation_matches_phase = matches!(
        (input.event_kind.as_str(), input.correlation.as_ref()),
        (
            "prompt_capture",
            Some(HostNativeCorrelation::CodexHookPrompt(_))
        ) | (
            "pre_tool" | "post_tool",
            Some(HostNativeCorrelation::CodexHookTool(_))
        )
    );
    if input.correlation.is_some() && !correlation_matches_phase {
        return Err(StoreError::InvalidInput {
            detail: "Guard event phase and host correlation kind do not match".to_owned(),
        });
    }
    if input.contract_status == GuardHookContractStatus::Compatible.as_str()
        && !correlation_matches_phase
    {
        return Err(StoreError::InvalidInput {
            detail: "compatible Guard event requires phase-specific hook correlation".to_owned(),
        });
    }
    validate_guard_decision(&input.decision)?;
    validate_json_object("guard_events.subject_json", &input.subject_json)?;
    validate_json_object("guard_events.result_json", &input.result_json)?;
    validate_timestamp_text("occurred_at", &input.occurred_at)?;
    validate_json_object("guard_events.metadata_json", &input.metadata_json)
}

fn validate_prompt_capture_insert(input: &PromptCaptureInsert) -> StoreResult<()> {
    validate_identifier("prompt_capture_id", &input.prompt_capture_id)?;
    if !matches!(
        &input.correlation,
        HostNativeCorrelation::CodexHookPrompt(_)
    ) {
        return Err(StoreError::InvalidInput {
            detail: "prompt capture requires Codex prompt-hook correlation".to_owned(),
        });
    }
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    validate_identifier("capture_kind", &input.capture_kind)?;
    validate_identifier("prompt_sha256", &input.prompt_sha256)?;
    if let Some(prompt_text) = &input.prompt_text {
        validate_text("prompt_text", prompt_text)?;
    }
    validate_timestamp_text("captured_at", &input.captured_at)?;
    validate_json_object("prompt_captures.metadata_json", &input.metadata_json)
}

fn validate_unrecorded_change_resolution(
    resolution: &UnrecordedChangeResolution,
) -> StoreResult<()> {
    resolution
        .resolved_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::InvalidInput {
            detail: "resolved_at must be a canonical four-digit RFC 3339 timestamp".to_owned(),
        })
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
                .map_err(|_| volicord_types::values::UtcTimestampParseError)
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

fn validate_string_items(field: &'static str, values: &[String]) -> StoreResult<()> {
    if values.iter().all(|value| !value.trim().is_empty()) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must contain only non-empty strings"),
        })
    }
}

fn current_guard_manifest(
    installation: &GuardInstallationRecord,
) -> StoreResult<volicord_types::guard_manifest::GuardManifest> {
    let manifest = guard_manifest_from_json(&installation.manifest_json).map_err(|_| {
        StoreError::InvalidInput {
            detail:
                "guard_installations.manifest_json must be one canonical current Guard manifest"
                    .to_owned(),
        }
    })?;
    validate_current_guard_host_contract(&manifest)?;
    Ok(manifest)
}

fn validate_current_guard_host_contract(
    manifest: &volicord_types::guard_manifest::GuardManifest,
) -> StoreResult<()> {
    let profile = HostContractProfileId::CodexCommandHooks;
    if manifest.host_contract_profile == profile.as_str()
        && manifest.host_contract_digest == profile.contract_digest()
    {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "guard_installations.manifest_json selects a stale host contract".to_owned(),
        })
    }
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
            m.project_id,
            m.session_id,
            m.runtime_session_id,
            m.connection_internal_id,
            h.project_integration_revision,
            h.host_session_id,
            m.host_thread_id,
            m.last_host_turn_id,
            min(h.first_observed_at, m.first_observed_at),
            max(h.last_observed_at, m.last_observed_at)
         FROM managed_mcp_sessions AS m
         JOIN host_sessions AS h
           ON h.project_id = m.project_id
          AND h.session_id = m.session_id
          AND h.connection_internal_id = m.connection_internal_id
        WHERE m.project_id = ?1
          AND m.session_id = ?2",
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
        StoreError::corrupt_owner_state_value(
            "managed_mcp_sessions",
            session.session_id.clone(),
            field,
        )
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
    decoded_codex_mcp_correlation(&session)?;
    let first = strict_stored_timestamp(
        "managed_mcp_sessions",
        &session.session_id,
        "first_observed_at",
        &session.first_observed_at,
    )?;
    let last = strict_stored_timestamp(
        "managed_mcp_sessions",
        &session.session_id,
        "last_observed_at",
        &session.last_observed_at,
    )?;
    if last < first {
        return Err(corrupt("last_observed_at"));
    }
    Ok(session)
}

fn decoded_codex_mcp_correlation(
    session: &AgentSessionRecord,
) -> StoreResult<HostNativeCorrelation> {
    let corrupt = |field| {
        StoreError::corrupt_owner_state_value(
            "managed_mcp_sessions",
            session.session_id.clone(),
            field,
        )
    };
    Ok(HostNativeCorrelation::CodexMcp(CodexMcpCorrelation {
        session_id: HostSessionId::parse(session.host_session_id.clone())
            .map_err(|_| corrupt("host_session_id"))?,
        thread_id: volicord_host_contract::HostThreadId::parse(session.host_thread_id.clone())
            .map_err(|_| corrupt("host_thread_id"))?,
        turn_id: HostTurnId::parse(session.last_host_turn_id.clone())
            .map_err(|_| corrupt("last_host_turn_id"))?,
    }))
}

fn host_session_from_conn(
    conn: &Connection,
    project_id: &str,
    session_id: &str,
) -> StoreResult<Option<HostSessionRecord>> {
    conn.query_row(
        "SELECT project_id, session_id, connection_internal_id,
                project_integration_revision, host_session_id,
                first_observed_at, last_observed_at
           FROM host_sessions
          WHERE project_id = ?1 AND session_id = ?2",
        params![project_id, session_id],
        |row| {
            Ok(HostSessionRecord {
                project_id: row.get(0)?,
                session_id: row.get(1)?,
                connection_internal_id: row.get(2)?,
                project_integration_revision: row.get(3)?,
                host_session_id: row.get(4)?,
                first_observed_at: row.get(5)?,
                last_observed_at: row.get(6)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|record| record.map(validate_decoded_host_session).transpose())
}

fn host_session_by_conn(
    conn: &Connection,
    project_id: &str,
    session_id: &str,
) -> StoreResult<HostSessionRecord> {
    host_session_from_conn(conn, project_id, session_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "host_session",
        id: session_id.to_owned(),
    })
}

fn validate_decoded_host_session(session: HostSessionRecord) -> StoreResult<HostSessionRecord> {
    let corrupt = |field| {
        StoreError::corrupt_owner_state_value("host_sessions", session.session_id.clone(), field)
    };
    validate_identifier("project_id", &session.project_id).map_err(|_| corrupt("project_id"))?;
    validate_identifier("session_id", &session.session_id).map_err(|_| corrupt("session_id"))?;
    validate_identifier("connection_internal_id", &session.connection_internal_id)
        .map_err(|_| corrupt("connection_internal_id"))?;
    IntegrationRevision::parse(session.project_integration_revision.clone())
        .map_err(|_| corrupt("project_integration_revision"))?;
    HostSessionId::parse(session.host_session_id.clone())
        .map_err(|_| corrupt("host_session_id"))?;
    let expected = project_agent_session_id(
        &session.connection_internal_id,
        &session.project_integration_revision,
        &session.host_session_id,
    )
    .map_err(|_| corrupt("session_id"))?;
    if expected != session.session_id {
        return Err(corrupt("session_id"));
    }
    let first = strict_stored_timestamp(
        "host_sessions",
        &session.session_id,
        "first_observed_at",
        &session.first_observed_at,
    )?;
    let last = strict_stored_timestamp(
        "host_sessions",
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
            e.project_id,
            e.guard_event_id,
            e.session_id,
            e.connection_internal_id,
            e.correlation_kind,
            h.host_session_id,
            e.host_turn_id,
            e.host_tool_use_id,
            e.host_tool_name,
            e.guard_installation_id,
            e.policy_hash,
            e.integration_revision,
            e.event_kind,
            e.contract_status,
            e.decision,
            e.subject_json,
            e.result_json,
            e.occurred_at,
            e.metadata_json
         FROM guard_events AS e
         LEFT JOIN host_sessions AS h
           ON h.project_id = e.project_id
          AND h.session_id = e.session_id
          AND h.connection_internal_id = e.connection_internal_id
        WHERE e.project_id = ?1
          AND e.guard_event_id = ?2",
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
    let correlation = decode_hook_correlation(
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
    )?;
    Ok(GuardEventRecord {
        project_id: row.get(0)?,
        guard_event_id: row.get(1)?,
        session_id: row.get(2)?,
        connection_internal_id: row.get(3)?,
        correlation,
        guard_installation_id: row.get(9)?,
        policy_hash: row.get(10)?,
        integration_revision: row.get(11)?,
        event_kind: row.get(12)?,
        contract_status: row.get(13)?,
        decision: row.get(14)?,
        subject_json: row.get(15)?,
        result_json: row.get(16)?,
        occurred_at: row.get(17)?,
        metadata_json: row.get(18)?,
    })
}

fn decode_hook_correlation(
    kind: Option<String>,
    host_session_id: Option<String>,
    host_turn_id: Option<String>,
    host_tool_use_id: Option<String>,
    host_tool_name: Option<String>,
) -> rusqlite::Result<Option<HostNativeCorrelation>> {
    let invalid = || {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid persisted host correlation",
            )
            .into(),
        )
    };
    match (
        kind.as_deref(),
        host_session_id,
        host_turn_id,
        host_tool_use_id,
        host_tool_name,
    ) {
        (None, None, None, None, None) => Ok(None),
        (Some("codex_hook_prompt"), Some(session), Some(turn), None, None) => Ok(Some(
            HostNativeCorrelation::CodexHookPrompt(CodexHookPromptCorrelation {
                session_id: HostSessionId::parse(session).map_err(|_| invalid())?,
                turn_id: HostTurnId::parse(turn).map_err(|_| invalid())?,
            }),
        )),
        (Some("codex_hook_tool"), Some(session), Some(turn), Some(tool_use), Some(tool_name)) => {
            Ok(Some(HostNativeCorrelation::CodexHookTool(
                CodexHookToolCorrelation {
                    session_id: HostSessionId::parse(session).map_err(|_| invalid())?,
                    turn_id: HostTurnId::parse(turn).map_err(|_| invalid())?,
                    tool_use_id: HostToolUseId::parse(tool_use).map_err(|_| invalid())?,
                    tool_name: CanonicalToolName::parse(tool_name).map_err(|_| invalid())?,
                },
            )))
        }
        _ => Err(invalid()),
    }
}

fn prompt_capture_from_conn(
    conn: &Connection,
    project_id: &str,
    prompt_capture_id: &str,
) -> StoreResult<Option<PromptCaptureRecord>> {
    conn.query_row(
        "SELECT
            p.project_id,
            p.prompt_capture_id,
            p.session_id,
            p.connection_internal_id,
            h.host_session_id,
            p.host_turn_id,
            p.capture_kind,
            p.prompt_sha256,
            p.prompt_text,
            p.captured_at,
            p.metadata_json
         FROM prompt_captures AS p
         JOIN host_sessions AS h
           ON h.project_id = p.project_id
          AND h.session_id = p.session_id
          AND h.connection_internal_id = p.connection_internal_id
        WHERE p.project_id = ?1
          AND p.prompt_capture_id = ?2",
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
    let correlation = decode_hook_correlation(
        Some("codex_hook_prompt".to_owned()),
        Some(row.get(4)?),
        Some(row.get(5)?),
        None,
        None,
    )?
    .expect("prompt capture correlation is required by SQL");
    Ok(PromptCaptureRecord {
        project_id: row.get(0)?,
        prompt_capture_id: row.get(1)?,
        session_id: row.get(2)?,
        connection_internal_id: row.get(3)?,
        correlation,
        capture_kind: row.get(6)?,
        prompt_sha256: row.get(7)?,
        prompt_text: row.get(8)?,
        captured_at: row.get(9)?,
        metadata_json: row.get(10)?,
    })
}

fn unrecorded_change_from_conn(
    conn: &Connection,
    project_id: &str,
    unrecorded_change_id: &str,
) -> StoreResult<Option<UnrecordedChangeRecord>> {
    let raw = conn
        .query_row(
            "SELECT
            u.project_id, u.unrecorded_change_id, u.repository_observation_id,
            o.session_id, o.connection_internal_id, h.host_session_id,
            o.host_turn_id, o.host_tool_use_id, o.host_tool_name, o.state,
            o.delta_json, o.delta_digest, o.completed_at, o.terminal_result_json,
            u.task_id, u.status, u.summary, u.observed_paths_json,
            u.unmatched_delta_digest, u.detection_json, u.resolution_json,
            u.detected_at, u.resolved_at, u.resolved_by_actor_source,
            u.metadata_json
         FROM unrecorded_changes AS u
         LEFT JOIN repository_observations AS o
           ON o.project_id = u.project_id
          AND o.repository_observation_id = u.repository_observation_id
         LEFT JOIN host_sessions AS h
           ON h.project_id = o.project_id
          AND h.session_id = o.session_id
          AND h.connection_internal_id = o.connection_internal_id
        WHERE u.project_id = ?1
          AND u.unrecorded_change_id = ?2",
            params![project_id, unrecorded_change_id],
            unrecorded_change_raw_from_row,
        )
        .optional()
        .map_err(StoreError::from)?;
    raw.map(decode_unrecorded_change).transpose()
}

fn unrecorded_change_by_conn(
    conn: &Connection,
    project_id: &str,
    unrecorded_change_id: &str,
) -> StoreResult<UnrecordedChangeRecord> {
    unrecorded_change_from_conn(conn, project_id, unrecorded_change_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "unrecorded_change",
            id: unrecorded_change_id.to_owned(),
        }
    })
}

fn unrecorded_change_raw_from_row(row: &Row<'_>) -> rusqlite::Result<UnrecordedChangeRaw> {
    Ok(UnrecordedChangeRaw {
        project_id: row.get(0)?,
        unrecorded_change_id: row.get(1)?,
        repository_observation_id: row.get(2)?,
        session_id: row.get(3)?,
        connection_internal_id: row.get(4)?,
        host_session_id: row.get(5)?,
        host_turn_id: row.get(6)?,
        host_tool_use_id: row.get(7)?,
        host_tool_name: row.get(8)?,
        observation_state: row.get(9)?,
        observation_delta_json: row.get(10)?,
        observation_delta_digest: row.get(11)?,
        observation_completed_at: row.get(12)?,
        observation_terminal_result_json: row.get(13)?,
        task_id: row.get(14)?,
        status: row.get(15)?,
        summary: row.get(16)?,
        observed_paths_json: row.get(17)?,
        unmatched_delta_digest: row.get(18)?,
        detection_json: row.get(19)?,
        resolution_json: row.get(20)?,
        detected_at: row.get(21)?,
        resolved_at: row.get(22)?,
        resolved_by_actor_source: row.get(23)?,
        metadata_json: row.get(24)?,
    })
}

fn decode_unrecorded_change(raw: UnrecordedChangeRaw) -> StoreResult<UnrecordedChangeRecord> {
    let record_ref = raw.unrecorded_change_id.clone();
    let corrupt_value = |field| {
        StoreError::corrupt_owner_state_value("unrecorded_changes", record_ref.clone(), field)
    };
    let corrupt_json = |field| {
        StoreError::corrupt_owner_state_json("unrecorded_changes", record_ref.clone(), field)
    };
    for (field, value) in [
        ("project_id", raw.project_id.as_str()),
        (
            "repository_observation_id",
            raw.repository_observation_id.as_str(),
        ),
        ("unrecorded_change_id", raw.unrecorded_change_id.as_str()),
    ] {
        validate_identifier(field, value).map_err(|_| corrupt_value(field))?;
    }
    if raw.summary.trim().is_empty() {
        return Err(corrupt_value("summary"));
    }
    if let Some(task_id) = raw.task_id.as_deref() {
        validate_identifier("task_id", task_id).map_err(|_| corrupt_value("task_id"))?;
    }
    let session_id = raw
        .session_id
        .ok_or_else(|| corrupt_value("repository_observation_id"))?;
    let connection_internal_id = raw
        .connection_internal_id
        .ok_or_else(|| corrupt_value("repository_observation_id"))?;
    let correlation = HostNativeCorrelation::CodexHookTool(CodexHookToolCorrelation {
        session_id: HostSessionId::parse(
            raw.host_session_id
                .ok_or_else(|| corrupt_value("repository_observation_id"))?,
        )
        .map_err(|_| corrupt_value("repository_observation_id"))?,
        turn_id: HostTurnId::parse(
            raw.host_turn_id
                .ok_or_else(|| corrupt_value("repository_observation_id"))?,
        )
        .map_err(|_| corrupt_value("repository_observation_id"))?,
        tool_use_id: HostToolUseId::parse(
            raw.host_tool_use_id
                .ok_or_else(|| corrupt_value("repository_observation_id"))?,
        )
        .map_err(|_| corrupt_value("repository_observation_id"))?,
        tool_name: CanonicalToolName::parse(
            raw.host_tool_name
                .ok_or_else(|| corrupt_value("repository_observation_id"))?,
        )
        .map_err(|_| corrupt_value("repository_observation_id"))?,
    });
    let expected_observation_id = repository_observation_id(
        &raw.project_id,
        &connection_internal_id,
        &session_id,
        &correlation,
    )
    .map_err(|_| corrupt_value("repository_observation_id"))?;
    if expected_observation_id != raw.repository_observation_id
        || raw.observation_state.as_deref() != Some("complete")
    {
        return Err(corrupt_value("repository_observation_id"));
    }
    let delta_json = raw
        .observation_delta_json
        .as_deref()
        .ok_or_else(|| corrupt_value("repository_observation_id"))?;
    let delta_digest = raw
        .observation_delta_digest
        .as_deref()
        .ok_or_else(|| corrupt_value("repository_observation_id"))?;
    let delta = serde_json::from_str::<RepositoryDelta>(delta_json)
        .map_err(|_| corrupt_value("repository_observation_id"))?;
    if canonical_json_string(&delta).ok().as_deref() != Some(delta_json)
        || delta.digest().as_str() != delta_digest
    {
        return Err(corrupt_value("repository_observation_id"));
    }
    let status = serde_json::from_value::<UnrecordedChangeStatus>(Value::String(raw.status))
        .map_err(|_| corrupt_value("status"))?;
    let observed_paths = serde_json::from_str::<Vec<ProductRelativePath>>(&raw.observed_paths_json)
        .map_err(|_| corrupt_json("observed_paths_json"))?;
    if observed_paths.is_empty()
        || observed_paths.windows(2).any(|pair| pair[0] >= pair[1])
        || canonical_json_string(&observed_paths).ok().as_deref() != Some(&raw.observed_paths_json)
    {
        return Err(corrupt_json("observed_paths_json"));
    }
    if !is_canonical_sha256_digest(&raw.unmatched_delta_digest) {
        return Err(corrupt_value("unmatched_delta_digest"));
    }
    let detection = serde_json::from_str::<JsonObject>(&raw.detection_json)
        .map_err(|_| corrupt_json("detection_json"))?;
    let expected_detection = serde_json::from_value::<JsonObject>(serde_json::json!({
        "repository_observation_id": raw.repository_observation_id,
        "transition_semantics": "net_product_repository_transition_during_invocation",
        "unmatched_delta_digest": raw.unmatched_delta_digest,
    }))
    .expect("object literal");
    if canonical_json_string(&detection).ok().as_deref() != Some(&raw.detection_json)
        || detection != expected_detection
    {
        return Err(corrupt_json("detection_json"));
    }
    let selected_paths = observed_paths.iter().cloned().collect::<BTreeSet<_>>();
    let delta_paths = delta
        .transitions()
        .iter()
        .map(|transition| transition.path().clone())
        .collect::<BTreeSet<_>>();
    if !selected_paths.is_subset(&delta_paths)
        || delta.restricted_to(&selected_paths).digest().as_str() != raw.unmatched_delta_digest
        || repository_observation::stable_unrecorded_change_id(
            &raw.project_id,
            &raw.repository_observation_id,
            &raw.unmatched_delta_digest,
        ) != raw.unrecorded_change_id
    {
        return Err(corrupt_value("unmatched_delta_digest"));
    }
    let resolution = raw
        .resolution_json
        .as_deref()
        .map(|value| {
            let object = serde_json::from_str::<JsonObject>(value)
                .map_err(|_| corrupt_json("resolution_json"))?;
            if canonical_json_string(&object).ok().as_deref() != Some(value) {
                return Err(corrupt_json("resolution_json"));
            }
            Ok(object)
        })
        .transpose()?;
    let detected_at = strict_stored_timestamp(
        "unrecorded_changes",
        &record_ref,
        "detected_at",
        &raw.detected_at,
    )?;
    let resolved_at = raw
        .resolved_at
        .as_deref()
        .map(|value| {
            strict_stored_timestamp("unrecorded_changes", &record_ref, "resolved_at", value)
        })
        .transpose()?;
    let resolved_by_actor_source = raw
        .resolved_by_actor_source
        .map(|value| {
            serde_json::from_value::<ActorSource>(Value::String(value))
                .map_err(|_| corrupt_value("resolved_by_actor_source"))
        })
        .transpose()?;
    let metadata = serde_json::from_str::<JsonObject>(&raw.metadata_json)
        .map_err(|_| corrupt_json("metadata_json"))?;
    if canonical_json_string(&metadata).ok().as_deref() != Some(&raw.metadata_json) {
        return Err(corrupt_json("metadata_json"));
    }
    let observation_completed_at = strict_stored_timestamp(
        "repository_observations",
        &raw.repository_observation_id,
        "completed_at",
        raw.observation_completed_at
            .as_deref()
            .ok_or_else(|| corrupt_value("repository_observation_id"))?,
    )
    .map_err(|_| corrupt_value("repository_observation_id"))?;
    if detected_at != observation_completed_at
        || resolved_at
            .as_ref()
            .is_some_and(|resolved| resolved < &detected_at)
    {
        return Err(corrupt_value("detected_at"));
    }
    let resolution_fields_valid = match status {
        UnrecordedChangeStatus::Unresolved => {
            resolution.is_none() && resolved_at.is_none() && resolved_by_actor_source.is_none()
        }
        UnrecordedChangeStatus::Resolved => {
            resolution.is_some() && resolved_at.is_some() && resolved_by_actor_source.is_some()
        }
    };
    if !resolution_fields_valid {
        return Err(corrupt_value("status"));
    }
    let terminal_result_json = raw
        .observation_terminal_result_json
        .as_deref()
        .ok_or_else(|| corrupt_value("repository_observation_id"))?;
    let terminal_result = serde_json::from_str::<RepositoryObservationResult>(terminal_result_json)
        .map_err(|_| corrupt_value("repository_observation_id"))?;
    let expected_result = RepositoryUnrecordedChangeResult {
        unrecorded_change_id: raw.unrecorded_change_id.clone(),
        unmatched_delta_digest: raw.unmatched_delta_digest.clone(),
        observed_paths: observed_paths.clone(),
    };
    if canonical_json_string(&terminal_result).ok().as_deref() != Some(terminal_result_json)
        || terminal_result.repository_observation_id != raw.repository_observation_id
        || !terminal_result
            .unrecorded_changes
            .contains(&expected_result)
    {
        return Err(corrupt_value("repository_observation_id"));
    }
    Ok(UnrecordedChangeRecord {
        project_id: raw.project_id,
        unrecorded_change_id: raw.unrecorded_change_id,
        repository_observation_id: raw.repository_observation_id,
        session_id,
        correlation,
        connection_internal_id,
        task_id: raw.task_id,
        status,
        summary: raw.summary,
        observed_paths,
        unmatched_delta_digest: raw.unmatched_delta_digest,
        detection,
        resolution,
        detected_at,
        resolved_at,
        resolved_by_actor_source,
        metadata,
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
    use volicord_types::guard_manifest::{
        GuardArtifactContentHash, GuardCommandAbsolutePath, GuardCommandInvocationSet,
        GuardCommandProjection, GuardManagedArtifact, GuardManifest, ManagedFileExpectation,
        PolicyHash, GUARD_MANIFEST_SCHEMA,
    };
    use volicord_types::ids::{AgentConnectionId, GuardInstallationId, ProjectId};
    use volicord_types::values::{GuardHookPhase, HostKind, IntegrationProfile};

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
    let content_hash = || GuardArtifactContentHash::parse(hash).expect("test content hash");
    let artifact_path = |artifact: GuardManagedArtifact| {
        artifact
            .expected_path(repo_root, None)
            .expect("test repository-owned Guard artifact path")
    };
    let block = |artifact, path| {
        ManagedFileExpectation::managed_block(
            artifact,
            path,
            content_hash(),
            "VOLICORD_START",
            "VOLICORD_END",
        )
        .expect("test managed block")
    };
    let json_file = |artifact, path| {
        ManagedFileExpectation::managed_json(artifact, path, content_hash())
            .expect("test managed JSON")
    };
    let script = |phase: GuardHookPhase| {
        ManagedFileExpectation::hook_wrapper(
            phase,
            GuardManagedArtifact::HostHookWrapper(phase)
                .expected_path(repo_root, None)
                .expect("test wrapper path"),
            content_hash(),
            "VOLICORD_MANAGED_HOOK_WRAPPER",
            "/test/volicord _hook",
            AgentConnectionId::new(&connection.connection_internal_id),
            GuardInstallationId::new(guard_installation_id),
            typed_policy_hash.clone(),
        )
    };
    let mut files = vec![
        block(
            GuardManagedArtifact::AgentsManagedBlock,
            artifact_path(GuardManagedArtifact::AgentsManagedBlock),
        ),
        json_file(
            GuardManagedArtifact::VolicordPolicy,
            artifact_path(GuardManagedArtifact::VolicordPolicy),
        ),
        json_file(
            GuardManagedArtifact::HostHookConfig,
            artifact_path(GuardManagedArtifact::HostHookConfig),
        ),
        ManagedFileExpectation::codex_dispatch_script(
            artifact_path(GuardManagedArtifact::HostHookDispatch),
            content_hash(),
            "VOLICORD_MANAGED_HOOK_WRAPPER",
        ),
        block(
            GuardManagedArtifact::HostRuleInstruction,
            artifact_path(GuardManagedArtifact::HostRuleInstruction),
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
        host_contract_profile: HostContractProfileId::CodexCommandHooks.as_str().to_owned(),
        host_contract_digest: HostContractProfileId::CodexCommandHooks.contract_digest(),
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
    use std::{
        error::Error,
        fs,
        process::Command,
        sync::{Arc, Barrier},
        thread,
    };

    use volicord_platform_fs::{
        InvocationObservationPaths, ObserverLimits, ProductPathState, RegularFileContentEvidence,
        RepositoryObservationCheckpoint, RepositoryObserver,
    };
    use volicord_test_support::{IsolatedGitRepository, TempRuntimeHome};

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
        mutation::{with_test_runtime_home_setup, TestRuntimeHomeAdmission},
        operational_sessions::{
            mcp_runtime_session, record_mcp_graceful_close, record_mcp_terminal_finding,
            recover_terminal_managed_runtime_repository_observations,
            repository_observation_diagnostics_for_runtime_session,
            start_mcp_runtime_session_for_test, McpRuntimeSessionStart,
        },
        sqlite::{open_project_state_database_for_test, open_registry_database_for_test},
    };
    use volicord_types::diagnostics::{
        DiagnosticCode, DiagnosticDomain, DiagnosticFacts, DiagnosticFindingData,
        DiagnosticSeverity, DiagnosticSource, DiagnosticStage, DiagnosticSubject,
        OccurrenceDiagnosticFinding,
    };
    use volicord_types::ids::{AgentConnectionId, AgentRuntimeSessionId};
    use volicord_types::integration_revision::McpRuntimeSessionSource;

    const TEST_POLICY_HASH: &str =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    fn mcp_correlation(session: &str, thread: &str, turn: &str) -> CodexMcpCorrelation {
        CodexMcpCorrelation {
            session_id: HostSessionId::parse(session).expect("valid test session"),
            thread_id: volicord_host_contract::HostThreadId::parse(thread)
                .expect("valid test thread"),
            turn_id: HostTurnId::parse(turn).expect("valid test turn"),
        }
    }

    fn prompt_correlation(session: &str, turn: &str) -> HostNativeCorrelation {
        HostNativeCorrelation::CodexHookPrompt(CodexHookPromptCorrelation {
            session_id: HostSessionId::parse(session).expect("valid test session"),
            turn_id: HostTurnId::parse(turn).expect("valid test turn"),
        })
    }

    fn tool_correlation(
        session: &str,
        turn: &str,
        tool_use: &str,
        tool_name: &str,
    ) -> HostNativeCorrelation {
        HostNativeCorrelation::CodexHookTool(CodexHookToolCorrelation {
            session_id: HostSessionId::parse(session).expect("valid test session"),
            turn_id: HostTurnId::parse(turn).expect("valid test turn"),
            tool_use_id: HostToolUseId::parse(tool_use).expect("valid test tool use"),
            tool_name: CanonicalToolName::parse(tool_name).expect("valid test tool name"),
        })
    }

    fn start_guard_runtime(
        context: &RuntimeHomeMutationContext<'_>,
        connection_id: &str,
        started_at: &str,
    ) -> StoreResult<String> {
        Ok(start_mcp_runtime_session_for_test(
            context,
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
    fn rust_and_sql_reject_invalid_guard_correlation_shapes() -> Result<(), Box<dyn Error>> {
        let prompt = prompt_correlation("session_shape", "turn_shape");
        let tool = tool_correlation("session_shape", "turn_shape", "tool_use_shape", "Bash");
        let event =
            |event_kind: &str, correlation: Option<HostNativeCorrelation>| GuardEventInsert {
                guard_event_id: "guard_event_shape".to_owned(),
                correlation,
                connection_internal_id: "conn_shape".to_owned(),
                guard_installation_id: "guard_shape".to_owned(),
                policy_hash: TEST_POLICY_HASH.to_owned(),
                integration_revision: TEST_POLICY_HASH.to_owned(),
                event_kind: event_kind.to_owned(),
                contract_status: GuardHookContractStatus::Compatible.as_str().to_owned(),
                decision: GuardDecision::Allow.as_str().to_owned(),
                subject_json: "{}".to_owned(),
                result_json: "{}".to_owned(),
                occurred_at: "2026-07-23T00:00:00Z".to_owned(),
                metadata_json: "{}".to_owned(),
            };
        assert!(validate_guard_event_insert(&event("pre_tool", Some(prompt))).is_err());
        assert!(validate_guard_event_insert(&event("prompt_capture", Some(tool))).is_err());
        assert!(validate_guard_event_insert(&event("pre_tool", None)).is_err());

        let fixture = GuardFixture::new("guard-correlation-sql-shape")?;
        fixture.add_project_connection("project_shape", "conn_shape", "repo-shape")?;
        let project = project_record_for_execution(fixture.runtime_home.path(), "project_shape")?
            .expect("fixture project");
        let conn = open_project_state_database_for_test(&project.state_db_path)?;
        let invalid = conn.execute(
            "INSERT INTO guard_events (
                project_id, guard_event_id, session_id, connection_internal_id,
                correlation_kind, host_turn_id, host_tool_use_id, host_tool_name,
                guard_installation_id, policy_hash, integration_revision, event_kind,
                contract_status, decision, subject_json, result_json, occurred_at, metadata_json
             ) VALUES (
                'project_shape', 'guard_event_invalid_shape', NULL, 'conn_shape',
                'codex_hook_tool', NULL, NULL, NULL,
                'guard_shape', ?1, ?1, 'prompt_capture',
                'compatible', 'allow', '{}', '{}', '2026-07-23T00:00:00Z', '{}'
             )",
            [TEST_POLICY_HASH],
        );
        assert!(
            invalid.is_err(),
            "SQL must reject an invalid nullable correlation shape"
        );
        Ok(())
    }

    #[test]
    fn reused_tool_use_id_cannot_change_turn_or_tool_name() -> Result<(), Box<dyn Error>> {
        let fixture = GuardFixture::new("guard-tool-correlation-conflict")?;
        fixture.add_project_connection("project_tool", "conn_tool", "repo-tool")?;
        let observe = |correlation| {
            observe_host_correlation(
                &fixture.context()?,
                "project_tool",
                HostCorrelationObservation {
                    connection_internal_id: "conn_tool".to_owned(),
                    guard_installation_id: None,
                    correlation,
                    observed_at: "2026-07-23T00:00:00Z".to_owned(),
                },
            )
        };
        observe(tool_correlation(
            "session_tool",
            "turn_tool",
            "tool_use_reused",
            "Bash",
        ))?;
        let error = observe(tool_correlation(
            "session_tool",
            "turn_tool",
            "tool_use_reused",
            "apply_patch",
        ))
        .expect_err("one tool-use ID cannot change tool name");
        assert!(matches!(error, StoreError::Conflict { .. }));
        Ok(())
    }

    #[test]
    fn exact_replay_finishes_an_interrupted_final_runtime_attachment() -> Result<(), Box<dyn Error>>
    {
        let fixture = GuardFixture::new("guard-runtime-attach-replay")?;
        fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;
        let runtime_session_id =
            start_guard_runtime(&fixture.context()?, "conn_guard_a", "2026-07-19T00:00:00Z")?;
        let observed_at = "2026-07-19T00:00:01Z";
        let correlation = mcp_correlation("session_guard_a", "thread_guard_a", "turn_guard_a");
        let coordinates = current_project_agent_session_coordinates(
            fixture.runtime_home.path(),
            "project_guard_a",
            "conn_guard_a",
            None,
            &HostNativeCorrelation::CodexMcp(correlation.clone()),
        )?;

        establish_host_correlation(
            &fixture.context()?,
            "project_guard_a",
            &coordinates,
            "conn_guard_a",
            &HostNativeCorrelation::CodexMcp(correlation.clone()),
            observed_at,
        )?;

        establish_agent_session_anchor(
            &fixture.context()?,
            "project_guard_a",
            &coordinates,
            AgentSessionAnchorInput {
                requested_runtime_session_id: Some(&runtime_session_id),
                connection_internal_id: "conn_guard_a",
                correlation: &correlation,
                observed_at,
            },
        )?;
        reserve_mcp_runtime_project_session(
            &fixture.context()?,
            McpRuntimeProjectSessionReservation {
                runtime_session_id: &runtime_session_id,
                connection_internal_id: "conn_guard_a",
                project_id: "project_guard_a",
                asserted_guard_installation_id: None,
                expected_coordinates: &coordinates,
                correlation: &correlation,
                bound_at: observed_at,
            },
        )?;

        let unbound = agent_session(
            fixture.runtime_home.path(),
            "project_guard_a",
            &coordinates.session_id,
        )?
        .expect("Phase 1 anchor");
        assert!(unbound.runtime_session_id.is_none());
        assert!(
            crate::operational_sessions::mcp_runtime_project_session_binding(
                fixture.runtime_home.path(),
                "project_guard_a",
                &coordinates.session_id,
            )?
            .is_some()
        );

        let attached = bind_agent_session_runtime(
            &fixture.context()?,
            "project_guard_a",
            AgentSessionRuntimeBinding {
                runtime_session_id: runtime_session_id.clone(),
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: None,
                correlation: correlation.clone(),
                observed_at: observed_at.to_owned(),
            },
        )?;
        assert_eq!(
            attached.runtime_session_id.as_deref(),
            Some(runtime_session_id.as_str())
        );

        let project = project_record_for_execution(fixture.runtime_home.path(), "project_guard_a")?
            .expect("fixture project");
        let project_conn = open_project_state_database_for_test(&project.state_db_path)?;
        let project_count: i64 = project_conn.query_row(
            "SELECT COUNT(*) FROM managed_mcp_sessions WHERE session_id = ?1",
            [&coordinates.session_id],
            |row| row.get(0),
        )?;
        assert_eq!(project_count, 1);
        let registry_conn =
            open_registry_database_for_test(registry_db_path(fixture.runtime_home.path()))?;
        let binding_count: i64 = registry_conn.query_row(
            "SELECT COUNT(*) FROM mcp_runtime_project_session_bindings WHERE session_id = ?1",
            [&coordinates.session_id],
            |row| row.get(0),
        )?;
        assert_eq!(binding_count, 1);

        let changed_owner = bind_agent_session_runtime(
            &fixture.context()?,
            "project_guard_a",
            AgentSessionRuntimeBinding {
                runtime_session_id,
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: None,
                correlation: mcp_correlation(
                    "session_guard_a",
                    "thread_guard_changed",
                    "turn_guard_changed",
                ),
                observed_at: "2026-07-19T00:00:02Z".to_owned(),
            },
        )
        .expect_err("changed owner input cannot claim an existing reservation");
        assert!(matches!(changed_owner, StoreError::Conflict { .. }));
        let binding_count_after: i64 = registry_conn.query_row(
            "SELECT COUNT(*) FROM mcp_runtime_project_session_bindings WHERE session_id = ?1",
            [&coordinates.session_id],
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
        let runtime_session_id =
            start_guard_runtime(&fixture.context()?, "conn_guard_a", "2026-07-19T00:00:00Z")?;
        let observed_at = "2026-07-19T00:00:01Z";
        let correlation = mcp_correlation("session_guard_a", "thread_guard_a", "turn_guard_a");
        let prior_coordinates = current_project_agent_session_coordinates(
            fixture.runtime_home.path(),
            "project_guard_a",
            "conn_guard_a",
            None,
            &HostNativeCorrelation::CodexMcp(correlation.clone()),
        )?;
        establish_host_correlation(
            &fixture.context()?,
            "project_guard_a",
            &prior_coordinates,
            "conn_guard_a",
            &HostNativeCorrelation::CodexMcp(correlation.clone()),
            observed_at,
        )?;
        establish_agent_session_anchor(
            &fixture.context()?,
            "project_guard_a",
            &prior_coordinates,
            AgentSessionAnchorInput {
                requested_runtime_session_id: Some(&runtime_session_id),
                connection_internal_id: "conn_guard_a",
                correlation: &correlation,
                observed_at,
            },
        )?;

        let project = project_record_for_execution(fixture.runtime_home.path(), "project_guard_a")?
            .expect("fixture project");
        let project_conn = open_project_state_database_for_test(&project.state_db_path)?;
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
            &fixture.context()?,
            McpRuntimeProjectSessionReservation {
                runtime_session_id: &runtime_session_id,
                connection_internal_id: "conn_guard_a",
                project_id: "project_guard_a",
                asserted_guard_installation_id: None,
                expected_coordinates: &prior_coordinates,
                correlation: &correlation,
                bound_at: observed_at,
            },
        )
        .expect_err("a changed project revision cannot reserve the old anchor");
        assert!(matches!(error, StoreError::Conflict { .. }));
        assert!(
            crate::operational_sessions::mcp_runtime_project_session_binding(
                fixture.runtime_home.path(),
                "project_guard_a",
                &prior_coordinates.session_id,
            )?
            .is_none()
        );
        let old_anchor = agent_session(
            fixture.runtime_home.path(),
            "project_guard_a",
            &prior_coordinates.session_id,
        )?
        .expect("historical unbound anchor");
        assert!(old_anchor.runtime_session_id.is_none());

        let current = bind_agent_session_runtime(
            &fixture.context()?,
            "project_guard_a",
            AgentSessionRuntimeBinding {
                runtime_session_id,
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: None,
                correlation: mcp_correlation(
                    "session_guard_a",
                    "thread_guard_a",
                    "turn_guard_current",
                ),
                observed_at: "2026-07-19T00:00:02Z".to_owned(),
            },
        )?;
        assert_ne!(current.session_id, prior_coordinates.session_id);
        assert!(current.runtime_session_id.is_some());
        Ok(())
    }

    #[test]
    fn repository_observation_aggregate_records_only_complete_unmatched_delta(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = GuardFixture::new("repository-observation-aggregate")?;
        fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;
        fixture.insert_task("project_guard_a", "task_guard_a")?;
        let project = project_record_for_execution(fixture.runtime_home.path(), "project_guard_a")?
            .expect("fixture project");
        let connection =
            agent_connection_record_read_only(fixture.runtime_home.path(), "conn_guard_a")?
                .expect("fixture connection");
        let installation = upsert_guard_installation(
            &fixture.context()?,
            GuardInstallationUpsert {
                guard_installation_id: "guard_installation_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: "project_guard_a".to_owned(),
                manifest_json: test_guard_manifest_json(
                    &connection,
                    "project_guard_a",
                    &project.repo_root,
                    "guard_installation_a",
                    TEST_POLICY_HASH,
                ),
            },
        )?;
        let manifest = guard_manifest_from_json(&installation.manifest_json)?;
        let runtime_session_id =
            start_guard_runtime(&fixture.context()?, "conn_guard_a", "2026-07-30T00:00:00Z")?;
        bind_agent_session_runtime(
            &fixture.context()?,
            "project_guard_a",
            AgentSessionRuntimeBinding {
                runtime_session_id,
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: Some("guard_installation_a".to_owned()),
                correlation: mcp_correlation("session_guard_a", "thread_guard_a", "turn_guard_a"),
                observed_at: "2026-07-30T00:00:01Z".to_owned(),
            },
        )?;
        let correlation = tool_correlation(
            "session_guard_a",
            "turn_guard_tool_a",
            "tool_call_a",
            "apply_patch",
        );
        let host_session = observe_host_correlation(
            &fixture.context()?,
            "project_guard_a",
            HostCorrelationObservation {
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: Some("guard_installation_a".to_owned()),
                correlation: correlation.clone(),
                observed_at: "2026-07-30T00:00:02Z".to_owned(),
            },
        )?;
        let expected_path = ProductRelativePath::parse("src/lib.rs")?;
        let unmatched_path = ProductRelativePath::parse("src/extra.rs")?;
        let git_init = Command::new("git")
            .args([
                "-C",
                project.repo_root.to_str().expect("UTF-8 fixture path"),
                "init",
                "-q",
            ])
            .output()?;
        assert!(
            git_init.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&git_init.stderr)
        );
        let observer = RepositoryObserver::new(&project.repo_root, ObserverLimits::default())?;
        let before = observer.snapshot(&InvocationObservationPaths::new(
            vec![expected_path.clone(), unmatched_path.clone()],
            Vec::new(),
        ))?;
        let observation_id = repository_observation_id(
            "project_guard_a",
            "conn_guard_a",
            &host_session.session_id,
            &correlation,
        )?;
        record_pre_tool_repository_observation(
            &fixture.context()?,
            "project_guard_a",
            PreToolRepositoryObservationInsert {
                guard_event: GuardEventInsert {
                    guard_event_id: "guard_pre_a".to_owned(),
                    correlation: Some(correlation.clone()),
                    connection_internal_id: "conn_guard_a".to_owned(),
                    guard_installation_id: "guard_installation_a".to_owned(),
                    policy_hash: TEST_POLICY_HASH.to_owned(),
                    integration_revision: manifest.integration_revision.as_str().to_owned(),
                    event_kind: "pre_tool".to_owned(),
                    contract_status: "compatible".to_owned(),
                    decision: "allow".to_owned(),
                    subject_json: "{}".to_owned(),
                    result_json: r#"{"decision":"allow"}"#.to_owned(),
                    occurred_at: "2026-07-30T00:00:03Z".to_owned(),
                    metadata_json: "{}".to_owned(),
                },
                repository_observation_id: observation_id.clone(),
                observer_contract_digest: observer.contract_digest().as_str().to_owned(),
                checkpoint: Some(before.checkpoint()),
                unavailable_reason: None,
                expected_write: Some(RepositoryExpectedWriteInsert {
                    expected_write_id: "expected_write_a".to_owned(),
                    command_kind: "may_write_product".to_owned(),
                    expected_paths: vec![expected_path.clone()],
                    task_id: "task_guard_a".to_owned(),
                    change_unit_id: "change_unit_guard_a".to_owned(),
                    write_ticket_ids: vec!["write_ticket_a".to_owned()],
                    basis_state_version: 0,
                    created_at: UtcTimestamp::parse("2026-07-30T00:00:03Z")?,
                    metadata: JsonObject::new(),
                }),
                metadata: JsonObject::new(),
            },
        )?;
        fs::create_dir_all(project.repo_root.join("src"))?;
        fs::write(
            project.repo_root.join(expected_path.as_str()),
            b"expected\n",
        )?;
        fs::write(
            project.repo_root.join(unmatched_path.as_str()),
            b"unmatched\n",
        )?;
        let after = observer.snapshot(&InvocationObservationPaths::new(
            vec![expected_path.clone(), unmatched_path.clone()],
            Vec::new(),
        ))?;
        let delta = observer.delta(&observer.restore_checkpoint(before.checkpoint())?, &after)?;
        let stored = record_post_tool_repository_observation(
            &fixture.context()?,
            "project_guard_a",
            PostToolRepositoryObservationInsert {
                guard_event: GuardEventInsert {
                    guard_event_id: "guard_post_a".to_owned(),
                    correlation: Some(correlation.clone()),
                    connection_internal_id: "conn_guard_a".to_owned(),
                    guard_installation_id: "guard_installation_a".to_owned(),
                    policy_hash: TEST_POLICY_HASH.to_owned(),
                    integration_revision: manifest.integration_revision.as_str().to_owned(),
                    event_kind: "post_tool".to_owned(),
                    contract_status: "compatible".to_owned(),
                    decision: "allow".to_owned(),
                    subject_json: "{}".to_owned(),
                    result_json: r#"{"decision":"allow"}"#.to_owned(),
                    occurred_at: "2026-07-30T00:00:04Z".to_owned(),
                    metadata_json: "{}".to_owned(),
                },
                repository_observation_id: observation_id.clone(),
                observer_contract_digest: observer.contract_digest().as_str().to_owned(),
                outcome: PostToolRepositoryObservationOutcome::Complete {
                    post_snapshot: Box::new(after.checkpoint()),
                    delta,
                },
                task_id: Some("task_guard_a".to_owned()),
                metadata: JsonObject::new(),
            },
        )?;
        assert_eq!(
            stored.observation.state,
            RepositoryObservationState::Complete
        );
        assert_eq!(stored.result.expected_write_matches.len(), 1);
        assert_eq!(
            stored.result.expected_write_matches[0].matched_paths,
            vec![expected_path]
        );
        assert_eq!(stored.result.unrecorded_changes.len(), 1);
        assert_eq!(
            stored.result.unrecorded_changes[0].observed_paths,
            vec![unmatched_path]
        );
        assert_eq!(
            list_unresolved_unrecorded_changes(
                fixture.runtime_home.path(),
                "project_guard_a",
                Some("conn_guard_a"),
            )?
            .len(),
            1
        );
        let second = record_post_tool_repository_observation(
            &fixture.context()?,
            "project_guard_a",
            PostToolRepositoryObservationInsert {
                guard_event: GuardEventInsert {
                    guard_event_id: "guard_post_conflict".to_owned(),
                    correlation: Some(correlation),
                    connection_internal_id: "conn_guard_a".to_owned(),
                    guard_installation_id: "guard_installation_a".to_owned(),
                    policy_hash: TEST_POLICY_HASH.to_owned(),
                    integration_revision: manifest.integration_revision.as_str().to_owned(),
                    event_kind: "post_tool".to_owned(),
                    contract_status: "compatible".to_owned(),
                    decision: "allow".to_owned(),
                    subject_json: "{}".to_owned(),
                    result_json: r#"{"decision":"allow"}"#.to_owned(),
                    occurred_at: "2026-07-30T00:00:05Z".to_owned(),
                    metadata_json: "{}".to_owned(),
                },
                repository_observation_id: observation_id,
                observer_contract_digest: observer.contract_digest().as_str().to_owned(),
                outcome: PostToolRepositoryObservationOutcome::Unavailable {
                    reason: RepositoryObservationUnavailableReason::MissingOpenObservation,
                },
                task_id: None,
                metadata: JsonObject::new(),
            },
        )
        .expect_err("a second PostToolUse event must conflict");
        assert!(matches!(second, StoreError::Conflict { .. }));
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
                &fixture.context()?,
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
            let correlation = match phase {
                GuardHookPhase::PromptCapture => {
                    prompt_correlation("session_guard_observation", &format!("turn_{suffix}"))
                }
                GuardHookPhase::PreTool | GuardHookPhase::PostTool => tool_correlation(
                    "session_guard_observation",
                    &format!("turn_{suffix}"),
                    &format!("tool_use_{suffix}"),
                    "Read",
                ),
            };
            observe_host_correlation(
                &fixture.context()?,
                "project_guard_a",
                HostCorrelationObservation {
                    connection_internal_id: "conn_guard_a".to_owned(),
                    guard_installation_id: Some("guard_installation_a".to_owned()),
                    correlation: correlation.clone(),
                    observed_at: "2000-01-02T00:00:00Z".to_owned(),
                },
            )?;
            insert_guard_event(
                &fixture.context()?,
                "project_guard_a",
                GuardEventInsert {
                    guard_event_id: format!("guard_event_{suffix}"),
                    correlation: Some(correlation),
                    connection_internal_id: "conn_guard_a".to_owned(),
                    guard_installation_id: "guard_installation_a".to_owned(),
                    policy_hash: policy_hash.to_owned(),
                    integration_revision: revision.clone(),
                    event_kind: phase.as_str().to_owned(),
                    contract_status: contract_status.to_owned(),
                    decision: "allow".to_owned(),
                    subject_json: "{}".to_owned(),
                    result_json: "{}".to_owned(),
                    occurred_at: "2000-01-02T00:00:00Z".to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )
        };

        let old = upsert(OLD_POLICY_HASH)?;
        open_registry_database_for_test(registry_db_path(fixture.runtime_home.path()))?.execute(
            "UPDATE guard_installations
                SET updated_at = '2000-01-01T00:00:00Z'
              WHERE guard_installation_id = 'guard_installation_a'",
            [],
        )?;
        let old = guard_installation(fixture.runtime_home.path(), &old.guard_installation_id)?
            .expect("guard installation after timestamp fixture setup");
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
        let unchanged = upsert(OLD_POLICY_HASH)?;
        assert_eq!(unchanged.updated_at, old.updated_at);
        assert!(guard_observation_summary(
            fixture.runtime_home.path(),
            "project_guard_a",
            &unchanged,
        )?
        .all_required_phases_observed());

        let current = upsert(CURRENT_POLICY_HASH)?;
        let pending =
            guard_observation_summary(fixture.runtime_home.path(), "project_guard_a", &current)?;
        assert!(pending.observed_phases.is_empty());
        assert!(!pending.all_required_phases_observed());
        open_registry_database_for_test(registry_db_path(fixture.runtime_home.path()))?.execute(
            "UPDATE guard_installations
                SET updated_at = '2000-01-01T00:00:00Z'
              WHERE guard_installation_id = 'guard_installation_a'",
            [],
        )?;
        let current =
            guard_installation(fixture.runtime_home.path(), &current.guard_installation_id)?
                .expect("current guard installation after timestamp fixture setup");

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

        let changed_manifest = current.manifest_json.replace(
            "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            "sha256:4444444444444444444444444444444444444444444444444444444444444444",
        );
        assert_ne!(changed_manifest, current.manifest_json);
        let changed_definition = upsert_guard_installation(
            &fixture.context()?,
            GuardInstallationUpsert {
                guard_installation_id: "guard_installation_a".to_owned(),
                connection_internal_id: "conn_guard_a".to_owned(),
                project_id: "project_guard_a".to_owned(),
                manifest_json: changed_manifest,
            },
        )?;
        assert_ne!(changed_definition.updated_at, current.updated_at);
        let reset = guard_observation_summary(
            fixture.runtime_home.path(),
            "project_guard_a",
            &changed_definition,
        )?;
        assert!(reset.observed_phases.is_empty());
        assert!(reset.incompatible_event_ids.is_empty());
        assert!(!reset.all_required_phases_observed());
        Ok(())
    }

    struct PreparedRepositoryObservation {
        fixture: GuardFixture,
        git: IsolatedGitRepository,
        project: ProjectRecord,
        runtime_session_id: String,
        session_id: String,
        integration_revision: String,
        correlation: HostNativeCorrelation,
        observer: RepositoryObserver,
        before: RepositoryObservationCheckpoint,
        observation_id: String,
        expected_path: ProductRelativePath,
        unmatched_path: ProductRelativePath,
    }

    impl PreparedRepositoryObservation {
        fn new(prefix: &str, tool_use_id: &str) -> Result<Self, Box<dyn Error>> {
            let fixture = GuardFixture::new(prefix)?;
            fixture.add_project_connection("project_guard_a", "conn_guard_a", "repo-a")?;
            fixture.insert_task("project_guard_a", "task_guard_a")?;
            let project =
                project_record_for_execution(fixture.runtime_home.path(), "project_guard_a")?
                    .expect("fixture project");
            let connection =
                agent_connection_record_read_only(fixture.runtime_home.path(), "conn_guard_a")?
                    .expect("fixture connection");
            let installation = upsert_guard_installation(
                &fixture.context()?,
                GuardInstallationUpsert {
                    guard_installation_id: "guard_installation_a".to_owned(),
                    connection_internal_id: "conn_guard_a".to_owned(),
                    project_id: "project_guard_a".to_owned(),
                    manifest_json: test_guard_manifest_json(
                        &connection,
                        "project_guard_a",
                        &project.repo_root,
                        "guard_installation_a",
                        TEST_POLICY_HASH,
                    ),
                },
            )?;
            let manifest = guard_manifest_from_json(&installation.manifest_json)?;
            let runtime_session_id =
                start_guard_runtime(&fixture.context()?, "conn_guard_a", "2026-07-30T00:00:00Z")?;
            bind_agent_session_runtime(
                &fixture.context()?,
                "project_guard_a",
                AgentSessionRuntimeBinding {
                    runtime_session_id: runtime_session_id.clone(),
                    connection_internal_id: "conn_guard_a".to_owned(),
                    guard_installation_id: Some("guard_installation_a".to_owned()),
                    correlation: mcp_correlation(
                        "session_guard_a",
                        "thread_guard_a",
                        "turn_guard_a",
                    ),
                    observed_at: "2026-07-30T00:00:01Z".to_owned(),
                },
            )?;
            let correlation = tool_correlation(
                "session_guard_a",
                "turn_guard_tool_a",
                tool_use_id,
                "apply_patch",
            );
            let coordinates = current_project_agent_session_coordinates(
                fixture.runtime_home.path(),
                "project_guard_a",
                "conn_guard_a",
                Some("guard_installation_a"),
                &correlation,
            )?;
            let git = IsolatedGitRepository::initialize_at(&project.repo_root)?;
            let expected_path = ProductRelativePath::parse("src/lib.rs")?;
            let unmatched_path = ProductRelativePath::parse("src/extra.rs")?;
            let observer = RepositoryObserver::new(&project.repo_root, ObserverLimits::default())?;
            let before = observer
                .snapshot(&InvocationObservationPaths::new(
                    vec![expected_path.clone(), unmatched_path.clone()],
                    Vec::new(),
                ))?
                .checkpoint();
            let observation_id = repository_observation_id(
                "project_guard_a",
                "conn_guard_a",
                &coordinates.session_id,
                &correlation,
            )?;
            Ok(Self {
                fixture,
                git,
                project,
                runtime_session_id,
                session_id: coordinates.session_id,
                integration_revision: manifest.integration_revision.as_str().to_owned(),
                correlation,
                observer,
                before,
                observation_id,
                expected_path,
                unmatched_path,
            })
        }

        fn event(&self, event_id: &str, kind: &str, occurred_at: &str) -> GuardEventInsert {
            GuardEventInsert {
                guard_event_id: event_id.to_owned(),
                correlation: Some(self.correlation.clone()),
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: "guard_installation_a".to_owned(),
                policy_hash: TEST_POLICY_HASH.to_owned(),
                integration_revision: self.integration_revision.clone(),
                event_kind: kind.to_owned(),
                contract_status: "compatible".to_owned(),
                decision: "allow".to_owned(),
                subject_json: "{}".to_owned(),
                result_json: r#"{"decision":"allow"}"#.to_owned(),
                occurred_at: occurred_at.to_owned(),
                metadata_json: format!(r#"{{"phase":"{kind}"}}"#),
            }
        }

        fn pre_input(&self) -> PreToolRepositoryObservationInsert {
            PreToolRepositoryObservationInsert {
                guard_event: self.event("guard_pre_a", "pre_tool", "2026-07-30T00:00:03Z"),
                repository_observation_id: self.observation_id.clone(),
                observer_contract_digest: self.observer.contract_digest().as_str().to_owned(),
                checkpoint: Some(self.before.clone()),
                unavailable_reason: None,
                expected_write: Some(RepositoryExpectedWriteInsert {
                    expected_write_id: "expected_write_a".to_owned(),
                    command_kind: "may_write_product".to_owned(),
                    expected_paths: vec![self.expected_path.clone()],
                    task_id: "task_guard_a".to_owned(),
                    change_unit_id: "change_unit_guard_a".to_owned(),
                    write_ticket_ids: vec!["write_ticket_a".to_owned()],
                    basis_state_version: 0,
                    created_at: UtcTimestamp::parse("2026-07-30T00:00:03Z").expect("test time"),
                    metadata: JsonObject::new(),
                }),
                metadata: serde_json::from_value(serde_json::json!({"phase": "pre"}))
                    .expect("object"),
            }
        }

        fn post_input(&self) -> Result<PostToolRepositoryObservationInsert, Box<dyn Error>> {
            fs::create_dir_all(self.project.repo_root.join("src"))?;
            fs::write(
                self.project.repo_root.join(self.expected_path.as_str()),
                b"expected\n",
            )?;
            fs::write(
                self.project.repo_root.join(self.unmatched_path.as_str()),
                b"unmatched\n",
            )?;
            let after = self.observer.snapshot(&InvocationObservationPaths::new(
                vec![self.expected_path.clone(), self.unmatched_path.clone()],
                Vec::new(),
            ))?;
            let before = self.observer.restore_checkpoint(self.before.clone())?;
            let delta = self.observer.delta(&before, &after)?;
            Ok(PostToolRepositoryObservationInsert {
                guard_event: self.event("guard_post_a", "post_tool", "2026-07-30T00:00:04Z"),
                repository_observation_id: self.observation_id.clone(),
                observer_contract_digest: self.observer.contract_digest().as_str().to_owned(),
                outcome: PostToolRepositoryObservationOutcome::Complete {
                    post_snapshot: Box::new(after.checkpoint()),
                    delta,
                },
                task_id: Some("task_guard_a".to_owned()),
                metadata: serde_json::from_value(serde_json::json!({"phase": "post"}))
                    .expect("object"),
            })
        }

        fn project_connection(&self) -> StoreResult<Connection> {
            open_project_state_database_for_test(&self.project.state_db_path)
        }
    }

    fn record_additional_open_observation(
        prepared: &PreparedRepositoryObservation,
        turn_id: &str,
        tool_use_id: &str,
        guard_event_id: &str,
        occurred_at: &str,
    ) -> Result<String, Box<dyn Error>> {
        let correlation = tool_correlation("session_guard_a", turn_id, tool_use_id, "apply_patch");
        let observation_id = repository_observation_id(
            "project_guard_a",
            "conn_guard_a",
            &prepared.session_id,
            &correlation,
        )?;
        let mut input = prepared.pre_input();
        input.guard_event.guard_event_id = guard_event_id.to_owned();
        input.guard_event.correlation = Some(correlation);
        input.guard_event.occurred_at = occurred_at.to_owned();
        input.repository_observation_id = observation_id.clone();
        input.expected_write = None;
        record_pre_tool_repository_observation(
            &prepared.fixture.context()?,
            "project_guard_a",
            input,
        )?;
        Ok(observation_id)
    }

    fn accepted_prompt_input(
        prepared: &PreparedRepositoryObservation,
        turn_id: &str,
        prompt_capture_id: &str,
        captured_at: &str,
    ) -> Result<PromptCaptureInsert, Box<dyn Error>> {
        let correlation = prompt_correlation("session_guard_a", turn_id);
        observe_host_correlation(
            &prepared.fixture.context()?,
            "project_guard_a",
            HostCorrelationObservation {
                connection_internal_id: "conn_guard_a".to_owned(),
                guard_installation_id: Some("guard_installation_a".to_owned()),
                correlation: correlation.clone(),
                observed_at: captured_at.to_owned(),
            },
        )?;
        Ok(PromptCaptureInsert {
            prompt_capture_id: prompt_capture_id.to_owned(),
            correlation,
            connection_internal_id: "conn_guard_a".to_owned(),
            capture_kind: "user_prompt".to_owned(),
            prompt_sha256: format!("sha256:{}", "a".repeat(64)),
            prompt_text: None,
            captured_at: captured_at.to_owned(),
            metadata_json: "{}".to_owned(),
        })
    }

    #[test]
    fn accepted_prompt_terminalizes_only_different_turns_once_without_derived_effects(
    ) -> Result<(), Box<dyn Error>> {
        let prepared = PreparedRepositoryObservation::new(
            "repository-observation-prompt-terminalization",
            "tool_call_prior_turn",
        )?;
        record_pre_tool_repository_observation(
            &prepared.fixture.context()?,
            "project_guard_a",
            prepared.pre_input(),
        )?;
        let current_observation_id = record_additional_open_observation(
            &prepared,
            "turn_prompt_current",
            "tool_call_parallel_current",
            "guard_pre_parallel_current",
            "2026-07-30T00:00:04Z",
        )?;
        let prompt = accepted_prompt_input(
            &prepared,
            "turn_prompt_current",
            "prompt_capture_current",
            "2026-07-30T00:00:05Z",
        )?;
        let first = insert_prompt_capture_and_terminalize_prior_turn_observations(
            &prepared.fixture.context()?,
            "project_guard_a",
            prompt,
        )?;
        assert_eq!(first.terminalized_repository_observations.len(), 1);
        assert_eq!(
            first.terminalized_repository_observations[0].repository_observation_id,
            prepared.observation_id
        );
        let prior = repository_observation(
            prepared.fixture.runtime_home.path(),
            "project_guard_a",
            &prepared.observation_id,
        )?
        .expect("prior observation");
        assert_eq!(prior.state, RepositoryObservationState::Unavailable);
        assert_eq!(
            prior.unavailable_reason,
            Some(RepositoryObservationUnavailableReason::PostToolNotObserved)
        );
        assert!(prior.delta.is_none());
        let terminal_result = prior.terminal_result.expect("stable terminal result");
        assert!(terminal_result.delta.is_none());
        assert!(terminal_result.expected_write_matches.is_empty());
        assert!(terminal_result.unrecorded_changes.is_empty());
        assert_eq!(
            repository_observation(
                prepared.fixture.runtime_home.path(),
                "project_guard_a",
                &current_observation_id,
            )?
            .expect("same-turn parallel observation")
            .state,
            RepositoryObservationState::Open
        );

        let diagnostics = repository_observation_diagnostics_for_session(
            prepared.fixture.runtime_home.path(),
            "project_guard_a",
            "conn_guard_a",
            &prepared.session_id,
            false,
        )?;
        assert!(diagnostics.iter().any(|record| {
            record.repository_observation_id.as_deref() == Some(prepared.observation_id.as_str())
                && record.status
                    == RepositoryObservationDiagnosticStatus::UnavailablePostToolNotObserved
        }));
        assert!(diagnostics.iter().any(|record| {
            record.repository_observation_id.as_deref() == Some(current_observation_id.as_str())
                && record.status == RepositoryObservationDiagnosticStatus::OpenCurrentTurn
        }));
        let terminal_diagnostics = repository_observation_diagnostics_for_session(
            prepared.fixture.runtime_home.path(),
            "project_guard_a",
            "conn_guard_a",
            &prepared.session_id,
            true,
        )?;
        assert!(terminal_diagnostics.iter().any(|record| {
            record.repository_observation_id.as_deref() == Some(current_observation_id.as_str())
                && record.status == RepositoryObservationDiagnosticStatus::OrphanOpenTerminalSession
        }));

        let later_observation_id = record_additional_open_observation(
            &prepared,
            "turn_later_open",
            "tool_call_later_open",
            "guard_pre_later_open",
            "2026-07-30T00:00:06Z",
        )?;
        let later_prompt = accepted_prompt_input(
            &prepared,
            "turn_after_later",
            "prompt_capture_after_later",
            "2026-07-30T00:00:07Z",
        )?;
        let second = insert_prompt_capture_and_terminalize_prior_turn_observations(
            &prepared.fixture.context()?,
            "project_guard_a",
            later_prompt.clone(),
        )?;
        let mut expected_terminalized = vec![
            current_observation_id.as_str(),
            later_observation_id.as_str(),
        ];
        expected_terminalized.sort_unstable();
        assert_eq!(
            second
                .terminalized_repository_observations
                .iter()
                .map(|record| record.repository_observation_id.as_str())
                .collect::<Vec<_>>(),
            expected_terminalized
        );
        let replay = insert_prompt_capture_and_terminalize_prior_turn_observations(
            &prepared.fixture.context()?,
            "project_guard_a",
            later_prompt,
        )?;
        assert!(replay.terminalized_repository_observations.is_empty());
        for observation_id in [&current_observation_id, &later_observation_id] {
            let record = repository_observation(
                prepared.fixture.runtime_home.path(),
                "project_guard_a",
                observation_id,
            )?
            .expect("terminalized observation");
            assert_eq!(record.state, RepositoryObservationState::Unavailable);
            assert_eq!(
                record.unavailable_reason,
                Some(RepositoryObservationUnavailableReason::PostToolNotObserved)
            );
        }

        let project_conn = prepared.project_connection()?;
        assert_eq!(
            project_conn.query_row(
                "SELECT status FROM expected_writes WHERE expected_write_id = 'expected_write_a'",
                [],
                |row| row.get::<_, String>(0),
            )?,
            "pending"
        );
        assert_eq!(
            project_conn.query_row("SELECT count(*) FROM unrecorded_changes", [], |row| {
                row.get::<_, i64>(0)
            })?,
            0
        );
        assert_eq!(
            project_conn.query_row(
                "SELECT count(*) FROM guard_events WHERE event_kind = 'post_tool'",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            0
        );
        let registry = open_registry_database_for_test(registry_db_path(
            prepared.fixture.runtime_home.path(),
        ))?;
        assert_eq!(
            registry.query_row("SELECT count(*) FROM diagnostic_findings", [], |row| {
                row.get::<_, i64>(0)
            })?,
            0
        );
        Ok(())
    }

    #[test]
    fn prompt_terminalization_validation_failure_rolls_back_every_selected_row_and_capture(
    ) -> Result<(), Box<dyn Error>> {
        use repository_observation::{
            set_repository_observation_fault_point, RepositoryObservationFaultPoint,
        };

        let prepared = PreparedRepositoryObservation::new(
            "repository-observation-terminalization-rollback",
            "tool_call_rollback_first",
        )?;
        record_pre_tool_repository_observation(
            &prepared.fixture.context()?,
            "project_guard_a",
            prepared.pre_input(),
        )?;
        let second_observation_id = record_additional_open_observation(
            &prepared,
            "turn_rollback_second",
            "tool_call_rollback_second",
            "guard_pre_rollback_second",
            "2026-07-30T00:00:04Z",
        )?;
        let prompt = accepted_prompt_input(
            &prepared,
            "turn_rollback_prompt",
            "prompt_capture_rollback",
            "2026-07-30T00:00:05Z",
        )?;
        set_repository_observation_fault_point(Some(
            RepositoryObservationFaultPoint::TerminalizationAfterFirstUpdate,
        ));
        assert!(
            insert_prompt_capture_and_terminalize_prior_turn_observations(
                &prepared.fixture.context()?,
                "project_guard_a",
                prompt.clone(),
            )
            .is_err()
        );
        set_repository_observation_fault_point(None);
        for observation_id in [&prepared.observation_id, &second_observation_id] {
            assert_eq!(
                repository_observation(
                    prepared.fixture.runtime_home.path(),
                    "project_guard_a",
                    observation_id,
                )?
                .expect("rolled-back observation")
                .state,
                RepositoryObservationState::Open
            );
        }
        assert!(prompt_capture(
            prepared.fixture.runtime_home.path(),
            "project_guard_a",
            "prompt_capture_rollback",
        )?
        .is_none());
        let retry = insert_prompt_capture_and_terminalize_prior_turn_observations(
            &prepared.fixture.context()?,
            "project_guard_a",
            prompt,
        )?;
        assert_eq!(retry.terminalized_repository_observations.len(), 2);
        Ok(())
    }

    fn abrupt_runtime_finding(
        prepared: &PreparedRepositoryObservation,
        observed_at: &str,
    ) -> Result<OccurrenceDiagnosticFinding, Box<dyn Error>> {
        let runtime = mcp_runtime_session(
            prepared.fixture.runtime_home.path(),
            &prepared.runtime_session_id,
        )?
        .expect("runtime session");
        let data = DiagnosticFindingData::try_new(
            DiagnosticCode::parse("mcp.runtime_process_failed")?,
            DiagnosticDomain::parse("mcp")?,
            DiagnosticStage::parse("transport")?,
            DiagnosticSeverity::Error,
            DiagnosticSource::parse("store_test")?,
            DiagnosticSubject::try_new("runtime_session", &prepared.runtime_session_id)?,
            DiagnosticFacts::empty(),
            UtcTimestamp::parse(observed_at)?,
        )?
        .with_connection_id(AgentConnectionId::new(runtime.connection_internal_id))?
        .with_integration_revision(IntegrationRevision::parse(
            runtime.connection_integration_revision,
        )?);
        Ok(OccurrenceDiagnosticFinding::try_new(
            data,
            Some(AgentRuntimeSessionId::new(
                prepared.runtime_session_id.clone(),
            )),
        )?)
    }

    #[test]
    fn managed_runtime_graceful_and_abrupt_terminal_facts_close_bound_observations(
    ) -> Result<(), Box<dyn Error>> {
        for (prefix, abrupt) in [
            ("repository-observation-graceful-close", false),
            ("repository-observation-abrupt-close", true),
        ] {
            let prepared = PreparedRepositoryObservation::new(
                prefix,
                if abrupt {
                    "tool_call_abrupt_close"
                } else {
                    "tool_call_graceful_close"
                },
            )?;
            record_pre_tool_repository_observation(
                &prepared.fixture.context()?,
                "project_guard_a",
                prepared.pre_input(),
            )?;
            if abrupt {
                let finding = abrupt_runtime_finding(&prepared, "2026-07-30T00:00:05Z")?;
                record_mcp_terminal_finding(&prepared.fixture.context()?, &finding)?;
            } else {
                record_mcp_graceful_close(
                    &prepared.fixture.context()?,
                    &prepared.runtime_session_id,
                    "2026-07-30T00:00:05Z",
                )?;
            }
            let observation = repository_observation(
                prepared.fixture.runtime_home.path(),
                "project_guard_a",
                &prepared.observation_id,
            )?
            .expect("managed-session observation");
            assert_eq!(observation.state, RepositoryObservationState::Unavailable);
            assert_eq!(
                observation.unavailable_reason,
                Some(RepositoryObservationUnavailableReason::ManagedSessionTerminated)
            );
            assert!(observation.delta.is_none());
            assert!(observation
                .terminal_result
                .as_ref()
                .is_some_and(|result| result.delta.is_none()
                    && result.expected_write_matches.is_empty()
                    && result.unrecorded_changes.is_empty()));
            assert_eq!(
                prepared.project_connection()?.query_row(
                    "SELECT count(*) FROM repository_observations WHERE state = 'open'",
                    [],
                    |row| row.get::<_, i64>(0),
                )?,
                0,
                "completed disposable session left an open observation"
            );
            let diagnostics = repository_observation_diagnostics_for_runtime_session(
                prepared.fixture.runtime_home.path(),
                &prepared.runtime_session_id,
            )?;
            assert!(diagnostics.iter().any(|record| {
                record.repository_observation_id.as_deref()
                    == Some(prepared.observation_id.as_str())
                    && record.status
                        == RepositoryObservationDiagnosticStatus::UnavailableManagedSessionTerminated
            }));
        }
        Ok(())
    }

    #[test]
    fn runtime_recovery_closes_only_authoritatively_terminal_managed_sessions(
    ) -> Result<(), Box<dyn Error>> {
        let active = PreparedRepositoryObservation::new(
            "repository-observation-active-recovery",
            "tool_call_active_recovery",
        )?;
        record_pre_tool_repository_observation(
            &active.fixture.context()?,
            "project_guard_a",
            active.pre_input(),
        )?;
        assert_eq!(
            recover_terminal_managed_runtime_repository_observations(&active.fixture.context()?)?,
            0
        );
        assert_eq!(
            repository_observation(
                active.fixture.runtime_home.path(),
                "project_guard_a",
                &active.observation_id,
            )?
            .expect("active observation")
            .state,
            RepositoryObservationState::Open
        );

        let terminal = PreparedRepositoryObservation::new(
            "repository-observation-terminal-recovery",
            "tool_call_terminal_recovery",
        )?;
        record_pre_tool_repository_observation(
            &terminal.fixture.context()?,
            "project_guard_a",
            terminal.pre_input(),
        )?;
        let registry = open_registry_database_for_test(registry_db_path(
            terminal.fixture.runtime_home.path(),
        ))?;
        registry.execute(
            "UPDATE mcp_runtime_sessions
                SET graceful_close_at = ?2, last_observed_at = ?2
              WHERE runtime_session_id = ?1",
            params![terminal.runtime_session_id, "2026-07-30T00:00:05Z"],
        )?;
        drop(registry);
        assert_eq!(
            recover_terminal_managed_runtime_repository_observations(&terminal.fixture.context()?)?,
            1
        );
        assert_eq!(
            recover_terminal_managed_runtime_repository_observations(&terminal.fixture.context()?)?,
            0
        );
        let observation = repository_observation(
            terminal.fixture.runtime_home.path(),
            "project_guard_a",
            &terminal.observation_id,
        )?
        .expect("recovered observation");
        assert_eq!(observation.state, RepositoryObservationState::Unavailable);
        assert_eq!(
            observation.unavailable_reason,
            Some(RepositoryObservationUnavailableReason::ManagedSessionTerminated)
        );
        Ok(())
    }

    fn assert_corrupt_owner_state<T>(result: StoreResult<T>) {
        assert!(matches!(
            result,
            Err(StoreError::CorruptOwnerStateJson { .. })
                | Err(StoreError::CorruptOwnerStateValue { .. })
                | Err(StoreError::CorruptOwnerStateInvariant { .. })
        ));
    }

    #[test]
    fn repository_observation_exact_replay_is_stable_and_conflicts_are_explicit(
    ) -> Result<(), Box<dyn Error>> {
        let prepared = PreparedRepositoryObservation::new(
            "repository-observation-exact-replay",
            "tool_call_replay",
        )?;
        let pre = prepared.pre_input();
        let first = record_pre_tool_repository_observation(
            &prepared.fixture.context()?,
            "project_guard_a",
            pre.clone(),
        )?;
        assert!(!first.replayed);
        let replay = record_pre_tool_repository_observation(
            &prepared.fixture.context()?,
            "project_guard_a",
            pre.clone(),
        )?;
        assert!(replay.replayed);
        assert_eq!(
            first.guard_event.guard_event_id,
            replay.guard_event.guard_event_id
        );

        let mut conflicting_pre = pre;
        conflicting_pre.guard_event.subject_json = r#"{"changed":true}"#.to_owned();
        assert!(matches!(
            record_pre_tool_repository_observation(
                &prepared.fixture.context()?,
                "project_guard_a",
                conflicting_pre,
            ),
            Err(StoreError::Conflict { .. })
        ));

        let post = prepared.post_input()?;
        let first = record_post_tool_repository_observation(
            &prepared.fixture.context()?,
            "project_guard_a",
            post.clone(),
        )?;
        assert!(!first.replayed);
        let replay = record_post_tool_repository_observation(
            &prepared.fixture.context()?,
            "project_guard_a",
            post.clone(),
        )?;
        assert!(replay.replayed);
        assert_eq!(first.result, replay.result);

        let pre_after_post = record_pre_tool_repository_observation(
            &prepared.fixture.context()?,
            "project_guard_a",
            prepared.pre_input(),
        )?;
        assert!(pre_after_post.replayed);
        let mut conflicting_post = post;
        conflicting_post.guard_event.result_json =
            r#"{"decision":"allow","extra":true}"#.to_owned();
        assert!(matches!(
            record_post_tool_repository_observation(
                &prepared.fixture.context()?,
                "project_guard_a",
                conflicting_post,
            ),
            Err(StoreError::Conflict { .. })
        ));

        let conn = prepared.project_connection()?;
        for (table, expected) in [
            ("repository_observations", 1_i64),
            ("guard_events", 2),
            ("expected_writes", 1),
            ("unrecorded_changes", 1),
        ] {
            let count = conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get::<_, i64>(0)
            })?;
            assert_eq!(count, expected, "{table}");
        }
        Ok(())
    }

    #[test]
    fn repository_observation_store_decodes_direct_and_tree_regular_file_evidence(
    ) -> Result<(), Box<dyn Error>> {
        let prepared = PreparedRepositoryObservation::new(
            "repository-observation-regular-file-evidence",
            "tool_call_regular_file_evidence",
        )?;
        prepared
            .git
            .write(".gitattributes", b"src/*.rs text eol=crlf\n")?;
        prepared.git.write(
            prepared.expected_path.as_str(),
            b"record=expected-baseline\r\n",
        )?;
        prepared.git.write(
            prepared.unmatched_path.as_str(),
            b"record=unmatched-baseline\r\n",
        )?;
        prepared.git.commit_all("regular-file evidence baseline")?;
        prepared.git.write(
            prepared.expected_path.as_str(),
            b"record=expected-before\r\n",
        )?;
        prepared.git.write(
            prepared.unmatched_path.as_str(),
            b"record=unmatched-before\r\n",
        )?;
        let before = prepared
            .observer
            .snapshot(&InvocationObservationPaths::default())?;
        let mut pre = prepared.pre_input();
        pre.checkpoint = Some(before.checkpoint());
        record_pre_tool_repository_observation(
            &prepared.fixture.context()?,
            "project_guard_a",
            pre,
        )?;

        prepared.git.write(
            prepared.expected_path.as_str(),
            b"record=expected-after\r\n",
        )?;
        prepared.git.write(
            prepared.unmatched_path.as_str(),
            b"record=unmatched-after\r\n",
        )?;
        prepared.git.commit_all("regular-file evidence outcome")?;
        let after = prepared
            .observer
            .snapshot(&InvocationObservationPaths::default())?;
        let delta = prepared.observer.delta(&before, &after)?;
        assert_eq!(delta.transitions().len(), 2);
        for transition in delta.transitions() {
            let (
                ProductPathState::RegularFile {
                    content_evidence: before,
                    ..
                },
                ProductPathState::RegularFile {
                    content_evidence: after,
                    ..
                },
            ) = (transition.before(), transition.after())
            else {
                panic!("regular-file transition must remain typed");
            };
            assert!(matches!(
                before,
                RegularFileContentEvidence::Worktree { .. }
            ));
            assert!(matches!(after, RegularFileContentEvidence::GitTree { .. }));
            assert_ne!(before.canonical_git_blob(), after.canonical_git_blob());
        }
        let stored = record_post_tool_repository_observation(
            &prepared.fixture.context()?,
            "project_guard_a",
            PostToolRepositoryObservationInsert {
                guard_event: prepared.event("guard_post_a", "post_tool", "2026-07-30T00:00:04Z"),
                repository_observation_id: prepared.observation_id.clone(),
                observer_contract_digest: prepared.observer.contract_digest().as_str().to_owned(),
                outcome: PostToolRepositoryObservationOutcome::Complete {
                    post_snapshot: Box::new(after.checkpoint()),
                    delta,
                },
                task_id: Some("task_guard_a".to_owned()),
                metadata: serde_json::from_value(serde_json::json!({"phase": "post"}))
                    .expect("object"),
            },
        )?;
        let decoded = repository_observation(
            prepared.fixture.runtime_home.path(),
            "project_guard_a",
            &prepared.observation_id,
        )?
        .expect("stored repository observation");
        assert_eq!(decoded, stored.observation);
        let checkpoint_json = serde_json::to_string(
            decoded
                .pre_snapshot
                .as_ref()
                .expect("direct worktree checkpoint"),
        )?;
        assert!(checkpoint_json.contains("\"source\":\"worktree\""));
        let delta_json = serde_json::to_string(decoded.delta.as_ref().expect("stored delta"))?;
        assert!(delta_json.contains("\"source\":\"git_tree\""));
        Ok(())
    }

    #[test]
    fn unrecorded_change_identity_is_stable_across_path_iteration_order(
    ) -> Result<(), Box<dyn Error>> {
        let prepared = PreparedRepositoryObservation::new(
            "repository-observation-finding-order",
            "tool_call_finding_order",
        )?;
        let post = prepared.post_input()?;
        let PostToolRepositoryObservationOutcome::Complete { delta, .. } = &post.outcome else {
            panic!("fixture post outcome must be complete");
        };
        let forward = [
            prepared.expected_path.clone(),
            prepared.unmatched_path.clone(),
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        let reverse = [
            prepared.unmatched_path.clone(),
            prepared.expected_path.clone(),
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        let forward_digest = delta.restricted_to(&forward).digest();
        let reverse_digest = delta.restricted_to(&reverse).digest();
        assert_eq!(forward_digest, reverse_digest);
        assert_eq!(
            repository_observation::stable_unrecorded_change_id(
                "project_guard_a",
                &prepared.observation_id,
                forward_digest.as_str(),
            ),
            repository_observation::stable_unrecorded_change_id(
                "project_guard_a",
                &prepared.observation_id,
                reverse_digest.as_str(),
            )
        );
        Ok(())
    }

    #[test]
    fn repository_observation_faults_roll_back_every_pre_boundary() -> Result<(), Box<dyn Error>> {
        use repository_observation::{
            set_repository_observation_fault_point, RepositoryObservationFaultPoint,
        };
        for (index, point) in [
            RepositoryObservationFaultPoint::PreAfterCorrelation,
            RepositoryObservationFaultPoint::PreAfterGuardEvent,
            RepositoryObservationFaultPoint::PreAfterObservation,
            RepositoryObservationFaultPoint::PreAfterExpectedWrite,
        ]
        .into_iter()
        .enumerate()
        {
            let prepared = PreparedRepositoryObservation::new(
                &format!("repository-observation-pre-rollback-{index}"),
                &format!("tool_call_pre_rollback_{index}"),
            )?;
            set_repository_observation_fault_point(Some(point));
            assert!(record_pre_tool_repository_observation(
                &prepared.fixture.context()?,
                "project_guard_a",
                prepared.pre_input(),
            )
            .is_err());
            set_repository_observation_fault_point(None);
            let conn = prepared.project_connection()?;
            for table in ["guard_events", "repository_observations", "expected_writes"] {
                let count =
                    conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                        row.get::<_, i64>(0)
                    })?;
                assert_eq!(count, 0, "{point:?} left a {table} row");
            }
            let tool_count = conn.query_row(
                "SELECT count(*) FROM host_tool_invocations WHERE host_tool_use_id = ?1",
                [format!("tool_call_pre_rollback_{index}")],
                |row| row.get::<_, i64>(0),
            )?;
            assert_eq!(tool_count, 0, "{point:?} left normalized correlation");
            assert!(
                !record_pre_tool_repository_observation(
                    &prepared.fixture.context()?,
                    "project_guard_a",
                    prepared.pre_input(),
                )?
                .replayed
            );
        }
        Ok(())
    }

    #[test]
    fn repository_observation_faults_roll_back_every_post_boundary() -> Result<(), Box<dyn Error>> {
        use repository_observation::{
            set_repository_observation_fault_point, RepositoryObservationFaultPoint,
        };
        for (index, point) in [
            RepositoryObservationFaultPoint::PostAfterExpectedWriteReconciliation,
            RepositoryObservationFaultPoint::PostAfterUnrecordedChangeInsert,
            RepositoryObservationFaultPoint::PostAfterGuardEvent,
            RepositoryObservationFaultPoint::PostAfterObservation,
        ]
        .into_iter()
        .enumerate()
        {
            let prepared = PreparedRepositoryObservation::new(
                &format!("repository-observation-post-rollback-{index}"),
                &format!("tool_call_post_rollback_{index}"),
            )?;
            record_pre_tool_repository_observation(
                &prepared.fixture.context()?,
                "project_guard_a",
                prepared.pre_input(),
            )?;
            let post = prepared.post_input()?;
            set_repository_observation_fault_point(Some(point));
            assert!(record_post_tool_repository_observation(
                &prepared.fixture.context()?,
                "project_guard_a",
                post.clone(),
            )
            .is_err());
            set_repository_observation_fault_point(None);
            let conn = prepared.project_connection()?;
            let state = conn.query_row("SELECT state FROM repository_observations", [], |row| {
                row.get::<_, String>(0)
            })?;
            assert_eq!(state, "open", "{point:?}");
            let status = conn.query_row("SELECT status FROM expected_writes", [], |row| {
                row.get::<_, String>(0)
            })?;
            assert_eq!(status, "pending", "{point:?}");
            let post_events = conn.query_row(
                "SELECT count(*) FROM guard_events WHERE event_kind = 'post_tool'",
                [],
                |row| row.get::<_, i64>(0),
            )?;
            assert_eq!(post_events, 0, "{point:?}");
            let changes = conn.query_row("SELECT count(*) FROM unrecorded_changes", [], |row| {
                row.get::<_, i64>(0)
            })?;
            assert_eq!(changes, 0, "{point:?}");
            assert!(
                !record_post_tool_repository_observation(
                    &prepared.fixture.context()?,
                    "project_guard_a",
                    post,
                )?
                .replayed
            );
        }
        Ok(())
    }

    fn concurrent_pre_record(
        runtime_home: PathBuf,
        input: PreToolRepositoryObservationInsert,
        barrier: Arc<Barrier>,
    ) -> thread::JoinHandle<StoreResult<PreToolRepositoryObservationRecord>> {
        thread::spawn(move || {
            let admission = TestRuntimeHomeAdmission::shared(runtime_home)?;
            barrier.wait();
            record_pre_tool_repository_observation(&admission.context()?, "project_guard_a", input)
        })
    }

    fn concurrent_post_record(
        runtime_home: PathBuf,
        input: PostToolRepositoryObservationInsert,
        barrier: Arc<Barrier>,
    ) -> thread::JoinHandle<StoreResult<PostToolRepositoryObservationRecord>> {
        thread::spawn(move || {
            let admission = TestRuntimeHomeAdmission::shared(runtime_home)?;
            barrier.wait();
            record_post_tool_repository_observation(&admission.context()?, "project_guard_a", input)
        })
    }

    #[test]
    fn repository_observation_concurrency_deduplicates_exact_invocations_without_cross_match(
    ) -> Result<(), Box<dyn Error>> {
        let prepared = PreparedRepositoryObservation::new(
            "repository-observation-concurrency",
            "tool_call_concurrent",
        )?;
        let runtime_home = prepared.fixture.runtime_home.path().to_path_buf();
        let pre = prepared.pre_input();
        let barrier = Arc::new(Barrier::new(2));
        let first = concurrent_pre_record(runtime_home.clone(), pre.clone(), barrier.clone());
        let second = concurrent_pre_record(runtime_home.clone(), pre, barrier);
        let mut replay_states = [
            first.join().expect("first PreToolUse worker")?.replayed,
            second.join().expect("second PreToolUse worker")?.replayed,
        ];
        replay_states.sort_unstable();
        assert_eq!(replay_states, [false, true]);

        let post = prepared.post_input()?;
        let barrier = Arc::new(Barrier::new(2));
        let first = concurrent_post_record(runtime_home.clone(), post.clone(), barrier.clone());
        let second = concurrent_post_record(runtime_home.clone(), post, barrier);
        let mut replay_states = [
            first.join().expect("first PostToolUse worker")?.replayed,
            second.join().expect("second PostToolUse worker")?.replayed,
        ];
        replay_states.sort_unstable();
        assert_eq!(replay_states, [false, true]);

        let distinct = |suffix: &str| -> StoreResult<PreToolRepositoryObservationInsert> {
            let mut input = prepared.pre_input();
            input.guard_event.guard_event_id = format!("guard_pre_{suffix}");
            input.guard_event.correlation = Some(tool_correlation(
                "session_guard_a",
                "turn_guard_distinct",
                &format!("tool_call_{suffix}"),
                "apply_patch",
            ));
            input
                .expected_write
                .as_mut()
                .expect("expected write")
                .expected_write_id = format!("expected_write_{suffix}");
            let correlation = input.guard_event.correlation.as_ref().expect("correlation");
            let coordinates = current_project_agent_session_coordinates(
                prepared.fixture.runtime_home.path(),
                "project_guard_a",
                "conn_guard_a",
                Some("guard_installation_a"),
                correlation,
            )?;
            input.repository_observation_id = repository_observation_id(
                "project_guard_a",
                "conn_guard_a",
                &coordinates.session_id,
                correlation,
            )?;
            Ok(input)
        };
        let barrier = Arc::new(Barrier::new(2));
        let first = concurrent_pre_record(
            runtime_home.clone(),
            distinct("distinct_a")?,
            barrier.clone(),
        );
        let second = concurrent_pre_record(runtime_home, distinct("distinct_b")?, barrier);
        let first = first.join().expect("first distinct worker")?;
        let second = second.join().expect("second distinct worker")?;
        assert!(!first.replayed && !second.replayed);
        assert_ne!(
            first.observation.repository_observation_id,
            second.observation.repository_observation_id
        );
        assert_ne!(
            first.observation.correlation,
            second.observation.correlation
        );
        Ok(())
    }

    #[test]
    fn unavailable_and_missing_pre_observations_are_terminal_and_replayable(
    ) -> Result<(), Box<dyn Error>> {
        let unavailable = PreparedRepositoryObservation::new(
            "repository-observation-unavailable",
            "tool_call_unavailable",
        )?;
        let mut pre = unavailable.pre_input();
        pre.checkpoint = None;
        pre.expected_write = None;
        pre.unavailable_reason = Some(RepositoryObservationUnavailableReason::Observer(
            volicord_platform_fs::ObservationUnavailableReason::UnstableRepository,
        ));
        let stored = record_pre_tool_repository_observation(
            &unavailable.fixture.context()?,
            "project_guard_a",
            pre.clone(),
        )?;
        assert_eq!(
            stored.observation.state,
            RepositoryObservationState::Unavailable
        );
        assert!(
            record_pre_tool_repository_observation(
                &unavailable.fixture.context()?,
                "project_guard_a",
                pre,
            )?
            .replayed
        );
        let post = PostToolRepositoryObservationInsert {
            guard_event: unavailable.event("guard_post_a", "post_tool", "2026-07-30T00:00:04Z"),
            repository_observation_id: unavailable.observation_id.clone(),
            observer_contract_digest: unavailable.observer.contract_digest().as_str().to_owned(),
            outcome: PostToolRepositoryObservationOutcome::Unavailable {
                reason: RepositoryObservationUnavailableReason::Observer(
                    volicord_platform_fs::ObservationUnavailableReason::UnstableRepository,
                ),
            },
            task_id: None,
            metadata: serde_json::from_value(serde_json::json!({"phase": "post"})).expect("object"),
        };
        assert!(
            !record_post_tool_repository_observation(
                &unavailable.fixture.context()?,
                "project_guard_a",
                post.clone(),
            )?
            .replayed
        );
        assert!(
            record_post_tool_repository_observation(
                &unavailable.fixture.context()?,
                "project_guard_a",
                post,
            )?
            .replayed
        );

        let missing = PreparedRepositoryObservation::new(
            "repository-observation-missing-pre",
            "tool_call_missing_pre",
        )?;
        let post = PostToolRepositoryObservationInsert {
            guard_event: missing.event("guard_post_missing", "post_tool", "2026-07-30T00:00:04Z"),
            repository_observation_id: missing.observation_id.clone(),
            observer_contract_digest: missing.observer.contract_digest().as_str().to_owned(),
            outcome: PostToolRepositoryObservationOutcome::Unavailable {
                reason: RepositoryObservationUnavailableReason::MissingOpenObservation,
            },
            task_id: None,
            metadata: serde_json::from_value(serde_json::json!({"phase": "post"})).expect("object"),
        };
        let stored = record_post_tool_repository_observation(
            &missing.fixture.context()?,
            "project_guard_a",
            post.clone(),
        )?;
        assert_eq!(
            stored.observation.unavailable_reason,
            Some(RepositoryObservationUnavailableReason::MissingOpenObservation)
        );
        assert!(stored.observation.pre_tool_guard_event_id.is_none());
        assert!(stored.observation.post_tool_guard_event_id.is_some());
        assert!(
            record_post_tool_repository_observation(
                &missing.fixture.context()?,
                "project_guard_a",
                post,
            )?
            .replayed
        );
        let conn = missing.project_connection()?;
        for table in ["expected_writes", "unrecorded_changes"] {
            assert_eq!(
                conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get::<_, i64>(0)
                })?,
                0
            );
        }

        let late_failure = PreparedRepositoryObservation::new(
            "repository-observation-post-unavailable",
            "tool_call_post_unavailable",
        )?;
        record_pre_tool_repository_observation(
            &late_failure.fixture.context()?,
            "project_guard_a",
            late_failure.pre_input(),
        )?;
        let post = PostToolRepositoryObservationInsert {
            guard_event: late_failure.event("guard_post_a", "post_tool", "2026-07-30T00:00:04Z"),
            repository_observation_id: late_failure.observation_id.clone(),
            observer_contract_digest: late_failure.observer.contract_digest().as_str().to_owned(),
            outcome: PostToolRepositoryObservationOutcome::Unavailable {
                reason: RepositoryObservationUnavailableReason::Observer(
                    volicord_platform_fs::ObservationUnavailableReason::UnstableRepository,
                ),
            },
            task_id: None,
            metadata: serde_json::from_value(serde_json::json!({"phase": "post"})).expect("object"),
        };
        record_post_tool_repository_observation(
            &late_failure.fixture.context()?,
            "project_guard_a",
            post,
        )?;
        assert!(
            record_pre_tool_repository_observation(
                &late_failure.fixture.context()?,
                "project_guard_a",
                late_failure.pre_input(),
            )?
            .replayed
        );
        Ok(())
    }

    #[test]
    fn repository_observation_strict_reads_reject_corrupt_relationships(
    ) -> Result<(), Box<dyn Error>> {
        let prepared = PreparedRepositoryObservation::new(
            "repository-observation-corruption",
            "tool_call_corrupt",
        )?;
        record_pre_tool_repository_observation(
            &prepared.fixture.context()?,
            "project_guard_a",
            prepared.pre_input(),
        )?;
        let stored = record_post_tool_repository_observation(
            &prepared.fixture.context()?,
            "project_guard_a",
            prepared.post_input()?,
        )?;
        let change_id = stored.result.unrecorded_changes[0]
            .unrecorded_change_id
            .clone();
        let mut conn = prepared.project_connection()?;
        conn.execute_batch(
            "PRAGMA foreign_keys = OFF;
             PRAGMA ignore_check_constraints = ON;",
        )?;

        {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE repository_observations
                    SET pre_tool_guard_event_id = post_tool_guard_event_id
                  WHERE repository_observation_id = ?1",
                [&prepared.observation_id],
            )?;
            assert_corrupt_owner_state(repository_observation::repository_observation_from_conn(
                &tx,
                "project_guard_a",
                &prepared.observation_id,
            ));
            tx.rollback()?;
        }
        let post_snapshot_json = conn.query_row(
            "SELECT post_snapshot_json
               FROM repository_observations
              WHERE repository_observation_id = ?1",
            [&prepared.observation_id],
            |row| row.get::<_, String>(0),
        )?;
        for mutation in [
            "removed_regular_file_shape",
            "missing_canonical_identity",
            "malformed_canonical_identity",
            "invalid_tree_evidence_combination",
        ] {
            let mut checkpoint: Value = serde_json::from_str(&post_snapshot_json)?;
            let state = checkpoint
                .pointer_mut("/observed_states/src~1lib.rs")
                .and_then(Value::as_object_mut)
                .expect("regular-file checkpoint state");
            match mutation {
                "removed_regular_file_shape" => {
                    let exact = state
                        .get("content_evidence")
                        .and_then(|evidence| evidence.get("exact_worktree_bytes"))
                        .cloned()
                        .expect("exact worktree identity");
                    state.remove("content_evidence");
                    state.insert("content".to_owned(), exact);
                }
                "missing_canonical_identity" => {
                    state
                        .get_mut("content_evidence")
                        .and_then(Value::as_object_mut)
                        .expect("content evidence")
                        .remove("canonical_git_blob");
                }
                "malformed_canonical_identity" => {
                    state["content_evidence"]["canonical_git_blob"] =
                        Value::String("NOT-A-CANONICAL-GIT-OBJECT".to_owned());
                }
                "invalid_tree_evidence_combination" => {
                    state["content_evidence"]["source"] = Value::String("git_tree".to_owned());
                }
                _ => unreachable!("closed corruption fixture"),
            }
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE repository_observations
                    SET post_snapshot_json = ?2
                  WHERE repository_observation_id = ?1",
                params![prepared.observation_id, serde_json::to_string(&checkpoint)?],
            )?;
            assert_corrupt_owner_state(repository_observation::repository_observation_from_conn(
                &tx,
                "project_guard_a",
                &prepared.observation_id,
            ));
            tx.rollback()?;
        }
        {
            let delta_json = conn.query_row(
                "SELECT delta_json
                   FROM repository_observations
                  WHERE repository_observation_id = ?1",
                [&prepared.observation_id],
                |row| row.get::<_, String>(0),
            )?;
            let mut delta: Value = serde_json::from_str(&delta_json)?;
            delta["transitions"][0]["after"] = delta["transitions"][0]["before"].clone();
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE repository_observations
                    SET delta_json = ?2
                  WHERE repository_observation_id = ?1",
                params![prepared.observation_id, serde_json::to_string(&delta)?],
            )?;
            assert_corrupt_owner_state(repository_observation::repository_observation_from_conn(
                &tx,
                "project_guard_a",
                &prepared.observation_id,
            ));
            tx.rollback()?;
        }
        {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE repository_observations
                    SET observer_contract_digest = ?2
                  WHERE repository_observation_id = ?1",
                params![
                    prepared.observation_id,
                    format!("sha256:{}", "f".repeat(64))
                ],
            )?;
            assert_corrupt_owner_state(repository_observation::repository_observation_from_conn(
                &tx,
                "project_guard_a",
                &prepared.observation_id,
            ));
            tx.rollback()?;
        }
        {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE repository_observations
                    SET delta_digest = ?2
                  WHERE repository_observation_id = ?1",
                params![
                    prepared.observation_id,
                    format!("sha256:{}", "e".repeat(64))
                ],
            )?;
            assert_corrupt_owner_state(repository_observation::repository_observation_from_conn(
                &tx,
                "project_guard_a",
                &prepared.observation_id,
            ));
            tx.rollback()?;
        }
        {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE repository_observations
                    SET state = 'unknown'
                  WHERE repository_observation_id = ?1",
                [&prepared.observation_id],
            )?;
            assert_corrupt_owner_state(repository_observation::repository_observation_from_conn(
                &tx,
                "project_guard_a",
                &prepared.observation_id,
            ));
            tx.rollback()?;
        }
        {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE repository_observations
                    SET state = 'open',
                        pre_snapshot_json = NULL,
                        pre_snapshot_digest = NULL,
                        post_tool_guard_event_id = NULL,
                        post_snapshot_json = NULL,
                        post_snapshot_digest = NULL,
                        delta_json = NULL,
                        delta_digest = NULL,
                        unavailable_reason = NULL,
                        completed_at = NULL,
                        terminal_result_json = NULL
                  WHERE repository_observation_id = ?1",
                [&prepared.observation_id],
            )?;
            assert_corrupt_owner_state(repository_observation::repository_observation_from_conn(
                &tx,
                "project_guard_a",
                &prepared.observation_id,
            ));
            tx.rollback()?;
        }
        {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE repository_observations
                    SET post_snapshot_json = NULL,
                        post_snapshot_digest = NULL
                  WHERE repository_observation_id = ?1",
                [&prepared.observation_id],
            )?;
            assert_corrupt_owner_state(repository_observation::repository_observation_from_conn(
                &tx,
                "project_guard_a",
                &prepared.observation_id,
            ));
            tx.rollback()?;
        }
        {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE repository_observations
                    SET state = 'unavailable',
                        unavailable_reason = 'unstable_repository'
                  WHERE repository_observation_id = ?1",
                [&prepared.observation_id],
            )?;
            assert_corrupt_owner_state(repository_observation::repository_observation_from_conn(
                &tx,
                "project_guard_a",
                &prepared.observation_id,
            ));
            tx.rollback()?;
        }
        {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE repository_observations
                    SET pre_snapshot_json =
                        replace(pre_snapshot_json, '\"kind\":\"absent\"', '\"kind\":\"unknown\"')
                  WHERE repository_observation_id = ?1",
                [&prepared.observation_id],
            )?;
            assert_corrupt_owner_state(repository_observation::repository_observation_from_conn(
                &tx,
                "project_guard_a",
                &prepared.observation_id,
            ));
            tx.rollback()?;
        }
        {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE repository_observations
                    SET pre_snapshot_digest = ?2
                  WHERE repository_observation_id = ?1",
                params![
                    prepared.observation_id,
                    format!("sha256:{}", "d".repeat(64))
                ],
            )?;
            assert_corrupt_owner_state(repository_observation::repository_observation_from_conn(
                &tx,
                "project_guard_a",
                &prepared.observation_id,
            ));
            tx.rollback()?;
        }
        {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE repository_observations
                    SET metadata_json = '{\"phase\": \"post\"}'
                  WHERE repository_observation_id = ?1",
                [&prepared.observation_id],
            )?;
            assert_corrupt_owner_state(repository_observation::repository_observation_from_conn(
                &tx,
                "project_guard_a",
                &prepared.observation_id,
            ));
            tx.rollback()?;
        }
        {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE expected_writes
                    SET repository_observation_id = 'repository_observation_missing'
                  WHERE expected_write_id = 'expected_write_a'",
                [],
            )?;
            assert_corrupt_owner_state(repository_observation::repository_observation_from_conn(
                &tx,
                "project_guard_a",
                &prepared.observation_id,
            ));
            tx.rollback()?;
        }
        {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE unrecorded_changes
                    SET observed_paths_json = '[\"src/lib.rs\"]'
                  WHERE unrecorded_change_id = ?1",
                [&change_id],
            )?;
            assert_corrupt_owner_state(unrecorded_change_from_conn(
                &tx,
                "project_guard_a",
                &change_id,
            ));
            tx.rollback()?;
        }
        for observed_paths_json in [
            "[]",
            "[\"src/extra.rs\",\"src/extra.rs\"]",
            "[\"../escape\"]",
        ] {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE unrecorded_changes
                    SET observed_paths_json = ?2
                  WHERE unrecorded_change_id = ?1",
                params![change_id, observed_paths_json],
            )?;
            assert_corrupt_owner_state(unrecorded_change_from_conn(
                &tx,
                "project_guard_a",
                &change_id,
            ));
            tx.rollback()?;
        }
        {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE repository_observations
                    SET state = 'unavailable'
                  WHERE repository_observation_id = ?1",
                [&prepared.observation_id],
            )?;
            assert_corrupt_owner_state(unrecorded_change_from_conn(
                &tx,
                "project_guard_a",
                &change_id,
            ));
            tx.rollback()?;
        }
        {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE unrecorded_changes
                    SET repository_observation_id = 'repository_observation_missing'
                  WHERE unrecorded_change_id = ?1",
                [&change_id],
            )?;
            assert_corrupt_owner_state(unrecorded_change_from_conn(
                &tx,
                "project_guard_a",
                &change_id,
            ));
            tx.rollback()?;
        }
        {
            let tx = conn.transaction()?;
            tx.execute(
                "DELETE FROM host_sessions
                  WHERE project_id = 'project_guard_a'
                    AND session_id = (
                        SELECT session_id FROM repository_observations
                         WHERE repository_observation_id = ?1
                    )",
                [&prepared.observation_id],
            )?;
            assert_corrupt_owner_state(repository_observation::repository_observation_from_conn(
                &tx,
                "project_guard_a",
                &prepared.observation_id,
            ));
            tx.rollback()?;
        }
        Ok(())
    }

    struct GuardFixture {
        mutation: TestRuntimeHomeAdmission,
        runtime_home: TempRuntimeHome,
    }

    impl GuardFixture {
        fn new(prefix: &str) -> Result<Self, Box<dyn Error>> {
            let runtime_home = TempRuntimeHome::new(prefix)?;
            with_test_runtime_home_setup(runtime_home.path(), |context| {
                initialize_runtime_home(context, &format!("runtime_home_{prefix}"), "{}")?;
                Ok(())
            })?;
            let mutation = TestRuntimeHomeAdmission::shared(runtime_home.path())?;
            Ok(Self {
                mutation,
                runtime_home,
            })
        }

        fn context(&self) -> StoreResult<RuntimeHomeMutationContext<'_>> {
            self.mutation.context()
        }

        fn add_project_connection(
            &self,
            project_id: &str,
            connection_id: &str,
            repo_name: &str,
        ) -> Result<(), Box<dyn Error>> {
            let repo_root = self.runtime_home.create_product_repo(repo_name)?;
            register_project(
                &self.context()?,
                ProjectRegistration {
                    project_id: project_id.to_owned(),
                    repo_root,
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            ensure_agent_connection(
                &self.context()?,
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
                &self.context()?,
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
            let conn = open_project_state_database_for_test(&project.state_db_path)?;
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
