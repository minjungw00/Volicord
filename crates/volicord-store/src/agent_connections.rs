use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};
use volicord_types::{
    guard_manifest_from_json, ConnectionIntegrationInstanceId, ConnectionVerificationReport,
    DurableIdGenerator, DurableIdKind, IntegrationRevision, RandomDurableIdGenerator, UtcTimestamp,
    DURABLE_ID_RETRY_LIMIT,
};

use crate::{
    bootstrap::{
        raw_project_record_from_conn, validate_current_project_registration, validate_project_id,
        ProjectRecord,
    },
    guards::{
        guard_installation_from_conn, upsert_guard_installation_in_transaction,
        validate_guard_installation_upsert_binding,
        validate_stored_guard_installation_manifest_binding, GuardInstallationRecord,
        GuardInstallationUpsert,
    },
    operational_sessions::connection_integration_revision,
    sqlite::{
        begin_immediate_transaction, open_registry_database_for_mutation,
        open_registry_database_read_only, registry_db_path,
    },
    RuntimeHomeMutationContext, StoreError, StoreResult,
};

/// Baseline-valid Codex host kind.
pub const HOST_KIND_CODEX: &str = "codex";

/// Baseline-valid user-scoped host configuration.
pub const HOST_SCOPE_USER: &str = "user";
/// Baseline-valid project-scoped host configuration.
pub const HOST_SCOPE_PROJECT: &str = "project";

/// Personal Agent Connection intent.
pub const CONNECTION_INTENT_PERSONAL: &str = "personal";
/// Shared Agent Connection intent.
pub const CONNECTION_INTENT_SHARED: &str = "shared";

/// Agent Connection mode that allows read-only operations.
pub const CONNECTION_MODE_READ_ONLY: &str = "read_only";
/// Agent Connection mode that allows workflow operations.
pub const CONNECTION_MODE_WORKFLOW: &str = "workflow";

const PENDING_HOST_CLEANUP_METADATA_KEY: &str = "pending_host_cleanup";

/// Agent Connection creation or compatible update input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConnectionRegistration {
    pub connection_internal_id: String,
    pub host_kind: String,
    pub intent: String,
    pub host_scope: String,
    pub server_name: String,
    pub config_target: String,
    pub mode: String,
    pub enabled: bool,
    pub managed_fingerprint: String,
    pub metadata_json: String,
}

/// Agent Connection ensure input keyed by host target and optional project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConnectionNaturalKeyRegistration {
    pub host_kind: String,
    pub intent: String,
    pub host_scope: String,
    pub project_ref: Option<String>,
    pub server_name: String,
    pub config_target: String,
    pub mode: String,
    pub enabled: bool,
    pub managed_fingerprint: String,
    pub metadata_json: String,
}

/// Natural key for looking up one Agent Connection without an ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConnectionNaturalKey {
    pub host_kind: String,
    pub intent: String,
    pub host_scope: String,
    pub project_ref: Option<String>,
    pub server_name: String,
    pub config_target: String,
}

/// Agent Connection row stored in `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConnectionRecord {
    pub connection_internal_id: String,
    pub integration_instance_id: ConnectionIntegrationInstanceId,
    pub host_kind: String,
    pub intent: String,
    pub host_scope: String,
    pub project_internal_id: Option<String>,
    pub server_name: String,
    pub config_target: String,
    pub mode: String,
    pub enabled: bool,
    pub managed_fingerprint: String,
    pub integration_generation: i64,
    pub verification_report_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

impl AgentConnectionRecord {
    /// Decodes the stored canonical report, if verification has run.
    pub fn verification_report(&self) -> StoreResult<Option<ConnectionVerificationReport>> {
        self.verification_report_json
            .as_deref()
            .map(|text| {
                let report =
                    serde_json::from_str::<ConnectionVerificationReport>(text).map_err(|_| {
                        StoreError::CorruptOwnerStateJson {
                            database_kind: "registry",
                            table: "agent_connections",
                            record_ref: self.connection_internal_id.clone(),
                            logical_column: "verification_report_json",
                        }
                    })?;
                if serde_json::to_string(&report).ok().as_deref() != Some(text) {
                    return Err(StoreError::CorruptOwnerStateJson {
                        database_kind: "registry",
                        table: "agent_connections",
                        record_ref: self.connection_internal_id.clone(),
                        logical_column: "verification_report_json",
                    });
                }
                Ok(report)
            })
            .transpose()
    }

    /// Returns the stored report or the read-only projection for an unverified connection.
    pub fn effective_verification_report(
        &self,
        checked_at: UtcTimestamp,
    ) -> StoreResult<ConnectionVerificationReport> {
        self.verification_report()?.map_or_else(
            || {
                ConnectionVerificationReport::verification_not_run(checked_at).map_err(|error| {
                    StoreError::InvalidInput {
                        detail: format!(
                            "could not synthesize the unverified connection report: {error}"
                        ),
                    }
                })
            },
            Ok,
        )
    }
}

/// Explicit project allowlist row creation input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProjectRegistration {
    pub connection_internal_id: String,
    pub project_id: String,
}

/// One superseded connection/project pair retired by an atomic activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupersededConnectionProject {
    pub connection_internal_id: String,
    pub project_id: String,
}

/// Transactionally classified Registry state for a staged connection migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StagedConnectionMigrationState {
    /// The requested project membership is still inactive and may be staged.
    Staged,
    /// Another attempt completed the Registry switch and host cleanup may resume.
    CleanupResume { pending_connection_ids: Vec<String> },
}

/// Failure while completing durable host-cleanup inventory after a connection
/// switch.
#[derive(Debug)]
pub enum PendingHostCleanupError<E> {
    Store(StoreError),
    Host(E),
}

impl<E> From<StoreError> for PendingHostCleanupError<E> {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl<E> From<rusqlite::Error> for PendingHostCleanupError<E> {
    fn from(error: rusqlite::Error) -> Self {
        Self::Store(StoreError::from(error))
    }
}

impl<E: std::fmt::Display> std::fmt::Display for PendingHostCleanupError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(error) => error.fmt(formatter),
            Self::Host(error) => error.fmt(formatter),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for PendingHostCleanupError<E> {}

/// Explicit project allowlist row with current project registration facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProjectRecord {
    pub connection_internal_id: String,
    pub project_internal_id: String,
    pub project_id: String,
    pub created_at: String,
    pub project: ProjectRecord,
}

/// Result of one atomic Connection Project removal transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProjectRemovalOutcome {
    pub membership_removed: bool,
    pub connection_removed: bool,
    pub remaining_project_count: usize,
}

/// One prevalidated Guard manifest rebind in a Connection mode transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionModeGuardManifestRebind {
    pub guard_installation_id: String,
    pub project_id: String,
    pub expected_manifest_json: String,
    pub manifest_json: String,
}

/// Store-owned input for one atomic Connection mode revision transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionModeTransition {
    pub connection_internal_id: String,
    pub expected_mode: String,
    pub expected_integration_revision: IntegrationRevision,
    pub mode: String,
    pub guard_manifests: Vec<ConnectionModeGuardManifestRebind>,
}

/// Whether an atomic Connection mode request changed durable state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionModeTransitionKind {
    Unchanged,
    Updated,
}

/// Result of one atomic Connection mode revision transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionModeTransitionOutcome {
    pub kind: ConnectionModeTransitionKind,
    pub connection: AgentConnectionRecord,
    pub previous_integration_revision: IntegrationRevision,
    pub current_integration_revision: IntegrationRevision,
    pub rebound_guard_installation_ids: Vec<String>,
}

/// Current dynamic project-access facts for one connection/project pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConnectionProjectAccess {
    pub connection_internal_id: String,
    pub project_id: String,
    pub connection_enabled: bool,
    pub project_allowed: bool,
    pub project: Option<ProjectRecord>,
}

/// Returns whether connection metadata records one exact pending host cleanup.
pub fn connection_metadata_has_pending_host_cleanup(
    metadata_json: &str,
    project_id: &str,
    replacement_connection_id: &str,
) -> bool {
    connection_metadata_pending_host_cleanup_replacement(metadata_json, project_id).as_deref()
        == Some(replacement_connection_id)
}

/// Returns the replacement connection named by valid pending host-cleanup
/// metadata for one project.
pub fn connection_metadata_pending_host_cleanup_replacement(
    metadata_json: &str,
    project_id: &str,
) -> Option<String> {
    let metadata = match serde_json::from_str::<Value>(metadata_json) {
        Ok(metadata) => metadata,
        Err(_) => return None,
    };
    metadata
        .get(PENDING_HOST_CLEANUP_METADATA_KEY)
        .cloned()
        .and_then(|pending| {
            let pending = pending.as_object()?;
            if pending.len() != 2
                || !pending.contains_key("project_id")
                || !pending.contains_key("replacement_connection_id")
                || pending["project_id"].as_str() != Some(project_id)
            {
                return None;
            }
            pending["replacement_connection_id"]
                .as_str()
                .filter(|replacement| !replacement.is_empty())
                .map(str::to_owned)
        })
}

/// Returns whether connection metadata records pending host cleanup for one
/// project, irrespective of the currently recorded replacement connection.
pub fn connection_metadata_has_pending_host_cleanup_for_project(
    metadata_json: &str,
    project_id: &str,
) -> bool {
    connection_metadata_pending_host_cleanup_replacement(metadata_json, project_id).is_some()
}

/// Returns whether connection metadata contains the Store-owned cleanup key,
/// including a malformed value that must not be treated as resumable cleanup.
pub fn connection_metadata_contains_pending_host_cleanup_key(metadata_json: &str) -> bool {
    match serde_json::from_str::<Value>(metadata_json) {
        Ok(Value::Object(metadata)) => metadata.contains_key(PENDING_HOST_CLEANUP_METADATA_KEY),
        Ok(_) | Err(_) => true,
    }
}

fn reject_pending_host_cleanup_metadata(metadata_json: &str) -> StoreResult<()> {
    if connection_metadata_contains_pending_host_cleanup_key(metadata_json) {
        Err(StoreError::InvalidInput {
            detail: format!(
                "agent_connections.metadata_json reserves {PENDING_HOST_CLEANUP_METADATA_KEY} for Store-owned migration recovery"
            ),
        })
    } else {
        Ok(())
    }
}

fn reject_generic_pending_host_cleanup_mutation(
    connection: &AgentConnectionRecord,
) -> StoreResult<()> {
    if connection_metadata_contains_pending_host_cleanup_key(&connection.metadata_json) {
        Err(StoreError::Conflict {
            entity: "agent_connection",
            id: connection.connection_internal_id.clone(),
            detail: "pending host cleanup must be completed by the migration recovery path"
                .to_owned(),
        })
    } else {
        Ok(())
    }
}

fn require_rebasable_pending_host_cleanup_metadata(
    connection: &AgentConnectionRecord,
    project_id: &str,
) -> StoreResult<()> {
    if !connection_metadata_contains_pending_host_cleanup_key(&connection.metadata_json)
        || connection_metadata_has_pending_host_cleanup_for_project(
            &connection.metadata_json,
            project_id,
        )
    {
        return Ok(());
    }

    Err(StoreError::Conflict {
        entity: "agent_connection",
        id: connection.connection_internal_id.clone(),
        detail: "pending host cleanup must be an exact valid marker for the migration project before it can be rebound"
            .to_owned(),
    })
}

fn metadata_with_pending_host_cleanup(
    metadata_json: &str,
    project_id: &str,
    replacement_connection_id: &str,
) -> StoreResult<String> {
    let mut metadata = serde_json::from_str::<Value>(metadata_json).map_err(|_| {
        StoreError::corrupt_stored_json("registry", "agent_connections.metadata_json")
    })?;
    let object = metadata.as_object_mut().ok_or_else(|| {
        StoreError::corrupt_stored_json("registry", "agent_connections.metadata_json")
    })?;
    object.insert(
        PENDING_HOST_CLEANUP_METADATA_KEY.to_owned(),
        serde_json::json!({
            "project_id": project_id,
            "replacement_connection_id": replacement_connection_id,
        }),
    );
    serde_json::to_string(&metadata)
        .map_err(|_| StoreError::corrupt_stored_json("registry", "agent_connections.metadata_json"))
}

fn metadata_without_pending_host_cleanup(metadata_json: &str) -> StoreResult<String> {
    let mut metadata = serde_json::from_str::<Value>(metadata_json).map_err(|_| {
        StoreError::corrupt_stored_json("registry", "agent_connections.metadata_json")
    })?;
    let object = metadata.as_object_mut().ok_or_else(|| {
        StoreError::corrupt_stored_json("registry", "agent_connections.metadata_json")
    })?;
    object.remove(PENDING_HOST_CLEANUP_METADATA_KEY);
    serde_json::to_string(&metadata)
        .map_err(|_| StoreError::corrupt_stored_json("registry", "agent_connections.metadata_json"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentConnectionWriteRegistration {
    connection_internal_id: String,
    host_kind: String,
    intent: String,
    host_scope: String,
    project_internal_id: Option<String>,
    server_name: String,
    config_target: String,
    mode: String,
    enabled: bool,
    managed_fingerprint: String,
    metadata_json: String,
}

/// Registers or updates one Agent Connection.
pub fn ensure_agent_connection(
    context: &RuntimeHomeMutationContext<'_>,
    registration: AgentConnectionRegistration,
) -> StoreResult<AgentConnectionRecord> {
    validate_agent_connection_registration(&registration)?;

    write_agent_connection(
        context,
        AgentConnectionWriteRegistration {
            connection_internal_id: registration.connection_internal_id,
            host_kind: registration.host_kind,
            intent: registration.intent,
            host_scope: registration.host_scope,
            project_internal_id: None,
            server_name: registration.server_name,
            config_target: registration.config_target,
            mode: registration.mode,
            enabled: registration.enabled,
            managed_fingerprint: registration.managed_fingerprint,
            metadata_json: registration.metadata_json,
        },
        false,
    )
}

/// Registers or updates a migration target while preserving a concurrently
/// enabled existing connection. New targets remain disabled until activation.
pub fn ensure_staged_agent_connection(
    context: &RuntimeHomeMutationContext<'_>,
    registration: AgentConnectionRegistration,
) -> StoreResult<AgentConnectionRecord> {
    validate_agent_connection_registration(&registration)?;
    if registration.enabled {
        return Err(StoreError::InvalidInput {
            detail: "staged Agent Connection registration must request enabled=false".to_owned(),
        });
    }
    write_agent_connection(
        context,
        AgentConnectionWriteRegistration {
            connection_internal_id: registration.connection_internal_id,
            host_kind: registration.host_kind,
            intent: registration.intent,
            host_scope: registration.host_scope,
            project_internal_id: None,
            server_name: registration.server_name,
            config_target: registration.config_target,
            mode: registration.mode,
            enabled: false,
            managed_fingerprint: registration.managed_fingerprint,
            metadata_json: registration.metadata_json,
        },
        true,
    )
}

/// Ensures an Agent Connection by its natural host target.
pub fn ensure_agent_connection_for_target(
    context: &RuntimeHomeMutationContext<'_>,
    registration: AgentConnectionNaturalKeyRegistration,
) -> StoreResult<AgentConnectionRecord> {
    validate_agent_connection_natural_key_registration(&registration)?;
    let runtime_home = context.runtime_home().as_path().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let conn = open_registry_database_read_only(&registry_path)?;
    let project_internal_id = registration
        .project_ref
        .as_deref()
        .map(|project_ref| {
            raw_project_record_from_conn(&conn, project_ref)?.ok_or_else(|| StoreError::NotFound {
                entity: "project",
                id: project_ref.to_owned(),
            })
        })
        .transpose()?
        .map(|project| project.project_internal_id);
    drop(conn);
    let connection_internal_id = connection_internal_id_for_target(
        &registration.host_kind,
        &registration.intent,
        &registration.host_scope,
        project_internal_id.as_deref(),
        &registration.config_target,
        &registration.server_name,
    );

    write_agent_connection(
        context,
        AgentConnectionWriteRegistration {
            connection_internal_id,
            host_kind: registration.host_kind,
            intent: registration.intent,
            host_scope: registration.host_scope,
            project_internal_id,
            server_name: registration.server_name,
            config_target: registration.config_target,
            mode: registration.mode,
            enabled: registration.enabled,
            managed_fingerprint: registration.managed_fingerprint,
            metadata_json: registration.metadata_json,
        },
        false,
    )
}

/// Looks up one Agent Connection by host target and optional project reference.
pub fn agent_connection_record_for_target(
    runtime_home: impl AsRef<Path>,
    key: AgentConnectionNaturalKey,
) -> StoreResult<Option<AgentConnectionRecord>> {
    validate_agent_connection_natural_key(&key)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database_read_only(&registry_path)?;
    let project_internal_id = key
        .project_ref
        .as_deref()
        .map(|project_ref| {
            raw_project_record_from_conn(&conn, project_ref)?.ok_or_else(|| StoreError::NotFound {
                entity: "project",
                id: project_ref.to_owned(),
            })
        })
        .transpose()?
        .map(|project| project.project_internal_id);
    let connection_internal_id = connection_internal_id_for_target(
        &key.host_kind,
        &key.intent,
        &key.host_scope,
        project_internal_id.as_deref(),
        &key.config_target,
        &key.server_name,
    );
    agent_connection_record_from_conn(&conn, &connection_internal_id)
}

fn write_agent_connection(
    context: &RuntimeHomeMutationContext<'_>,
    registration: AgentConnectionWriteRegistration,
    preserve_existing_enabled: bool,
) -> StoreResult<AgentConnectionRecord> {
    validate_agent_connection_write_registration(&registration)?;

    let registry_path = registry_db_path(context.runtime_home().as_path());
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;

    if let Some(existing_target_id) =
        existing_connection_internal_id_for_target(&tx, &registration)?
    {
        require_agent_connection(&tx, &existing_target_id)?;
        if existing_target_id != registration.connection_internal_id {
            return Err(conflict(
                "agent_connection",
                &registration.connection_internal_id,
                "host target is already managed by another connection_internal_id",
            ));
        }
    }

    if let Some(existing) =
        agent_connection_record_from_conn(&tx, &registration.connection_internal_id)?
    {
        reject_generic_pending_host_cleanup_mutation(&existing)?;
        if !connection_target_is_compatible(&existing, &registration) {
            return Err(conflict(
                "agent_connection",
                &registration.connection_internal_id,
                "connection_internal_id is already bound to a different host target",
            ));
        }
        if existing.mode != registration.mode {
            return Err(conflict(
                "agent_connection",
                &registration.connection_internal_id,
                "an established Connection mode can change only through the atomic mode-transition API",
            ));
        }
        let enabled = registration.enabled || (preserve_existing_enabled && existing.enabled);
        tx.execute(
            "UPDATE agent_connections
                SET enabled = ?2,
                    managed_fingerprint = ?3,
                    verification_report_json = CASE
                        WHEN managed_fingerprint <> ?3 THEN NULL
                        ELSE verification_report_json
                    END,
                    metadata_json = ?4,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    project_internal_id = ?5
              WHERE connection_internal_id = ?1",
            params![
                registration.connection_internal_id,
                enabled_as_i64(enabled),
                registration.managed_fingerprint,
                registration.metadata_json,
                registration.project_internal_id
            ],
        )?;
    } else {
        let generator = RandomDurableIdGenerator;
        let mut inserted = false;
        for _ in 0..DURABLE_ID_RETRY_LIMIT {
            let integration_instance_id = generator
                .generate(DurableIdKind::ConnectionIntegrationInstance)
                .map_err(|error| StoreError::InvalidInput {
                    detail: format!(
                        "could not generate Agent Connection integration-instance id: {error}"
                    ),
                })?;
            let integration_instance_id = ConnectionIntegrationInstanceId::parse(
                integration_instance_id,
            )
            .map_err(|error| StoreError::InvalidInput {
                detail: format!(
                    "generated Agent Connection integration-instance id was invalid: {error}"
                ),
            })?;
            let changed = tx.execute(
                "INSERT OR IGNORE INTO agent_connections (
                connection_internal_id,
                integration_instance_id,
                host_kind,
                intent,
                host_scope,
                project_internal_id,
                server_name,
                config_target,
                mode,
                enabled,
                managed_fingerprint,
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
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )",
                params![
                    registration.connection_internal_id,
                    integration_instance_id.as_str(),
                    registration.host_kind,
                    registration.intent,
                    registration.host_scope,
                    registration.project_internal_id,
                    registration.server_name,
                    registration.config_target,
                    registration.mode,
                    enabled_as_i64(registration.enabled),
                    registration.managed_fingerprint,
                    registration.metadata_json
                ],
            )?;
            if changed == 1 {
                inserted = true;
                break;
            }
        }
        if !inserted {
            return Err(StoreError::Conflict {
                entity: "agent_connection",
                id: registration.connection_internal_id.clone(),
                detail: "integration-instance durable id collision retry limit was exhausted"
                    .to_owned(),
            });
        }
    }
    let connection = agent_connection_record_from_conn(&tx, &registration.connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: registration.connection_internal_id.clone(),
        })?;
    tx.commit()?;
    Ok(connection)
}

/// Reads one Agent Connection.
pub fn agent_connection_record(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Option<AgentConnectionRecord>> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database_read_only(registry_path)?;
    agent_connection_record_from_conn(&conn, connection_internal_id)
}

/// Reads one Agent Connection without creating, migrating, or writing registry state.
pub fn agent_connection_record_read_only(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Option<AgentConnectionRecord>> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database_read_only(registry_path)?;
    agent_connection_record_from_conn(&conn, connection_internal_id)
}

/// Reads one Agent Connection as raw diagnostic state without validating its
/// persisted JSON owner fields. This read never creates or writes registry state.
pub fn agent_connection_record_for_diagnostics(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Option<AgentConnectionRecord>> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database_read_only(registry_path)?;
    raw_agent_connection_record_from_conn(&conn, connection_internal_id)
}

/// Lists Agent Connections in deterministic order.
pub fn list_agent_connections(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<Vec<AgentConnectionRecord>> {
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(Vec::new());
    }

    let conn = open_registry_database_read_only(registry_path)?;
    list_agent_connections_from_conn(&conn)
}

/// Lists Agent Connections without creating, migrating, or writing registry state.
pub fn list_agent_connections_read_only(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<Vec<AgentConnectionRecord>> {
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database_read_only(registry_path)?;
    list_agent_connections_from_conn(&conn)
}

/// Lists raw Agent Connection diagnostic state without validating persisted
/// JSON owner fields. This read never creates or writes registry state.
pub fn list_agent_connections_for_diagnostics(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<Vec<AgentConnectionRecord>> {
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database_read_only(registry_path)?;
    list_raw_agent_connections_from_conn(&conn)
}

fn list_agent_connections_from_conn(conn: &Connection) -> StoreResult<Vec<AgentConnectionRecord>> {
    list_raw_agent_connections_from_conn(conn)?
        .into_iter()
        .map(validate_stored_agent_connection)
        .collect()
}

fn list_raw_agent_connections_from_conn(
    conn: &Connection,
) -> StoreResult<Vec<AgentConnectionRecord>> {
    let mut stmt = conn.prepare(
        "SELECT
            connection_internal_id,
            integration_instance_id,
            host_kind,
            intent,
            host_scope,
            project_internal_id,
            server_name,
            config_target,
            mode,
            enabled,
            managed_fingerprint,
            integration_generation,
            verification_report_json,
            created_at,
            updated_at,
            metadata_json
         FROM agent_connections
         ORDER BY host_kind, intent, host_scope, server_name, connection_internal_id",
    )?;
    let mut rows = stmt.query([])?;
    let mut connections = Vec::new();
    while let Some(row) = rows.next()? {
        connections.push(decode_agent_connection_record(
            agent_connection_record_from_row(row)?,
        )?);
    }
    Ok(connections)
}

/// Enables or disables an Agent Connection.
pub fn set_connection_enabled(
    context: &RuntimeHomeMutationContext<'_>,
    connection_internal_id: &str,
    enabled: bool,
) -> StoreResult<AgentConnectionRecord> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    let registry_path = registry_db_path(context.runtime_home().as_path());
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let connection = require_agent_connection(&tx, connection_internal_id)?;
    reject_generic_pending_host_cleanup_mutation(&connection)?;
    let changed = tx.execute(
        "UPDATE agent_connections
            SET enabled = ?2,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE connection_internal_id = ?1",
        params![connection_internal_id, enabled_as_i64(enabled)],
    )?;
    if changed == 0 {
        return Err(StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        });
    }
    tx.commit()?;

    agent_connection_record_from_conn(&conn, connection_internal_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        }
    })
}

/// Rebinds one Connection and every owned Guard manifest as one mode revision transition.
pub fn transition_connection_mode(
    context: &RuntimeHomeMutationContext<'_>,
    input: ConnectionModeTransition,
) -> StoreResult<ConnectionModeTransitionOutcome> {
    validate_identifier("connection_internal_id", &input.connection_internal_id)?;
    validate_connection_mode(&input.expected_mode)?;
    validate_connection_mode(&input.mode)?;
    let mut guard_ids = BTreeSet::new();
    let mut project_ids = BTreeSet::new();
    for rebind in &input.guard_manifests {
        validate_identifier("guard_installation_id", &rebind.guard_installation_id)?;
        validate_project_id(&rebind.project_id)?;
        if !guard_ids.insert(rebind.guard_installation_id.clone()) {
            return Err(StoreError::InvalidInput {
                detail: format!(
                    "duplicate Guard Installation in mode-transition inventory: {}",
                    rebind.guard_installation_id
                ),
            });
        }
        if !project_ids.insert(rebind.project_id.clone()) {
            return Err(StoreError::InvalidInput {
                detail: format!(
                    "duplicate project in mode-transition inventory: {}",
                    rebind.project_id
                ),
            });
        }
    }

    let runtime_home = context.runtime_home().as_path().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let connection = require_agent_connection(&tx, &input.connection_internal_id)?;
    reject_generic_pending_host_cleanup_mutation(&connection)?;
    let previous_revision = connection_integration_revision(&connection)?;
    if connection.mode != input.expected_mode
        || previous_revision != input.expected_integration_revision
    {
        return Err(StoreError::Conflict {
            entity: "agent_connection",
            id: input.connection_internal_id,
            detail:
                "Connection mode or integration revision changed after mode-transition preflight"
                    .to_owned(),
        });
    }

    if connection.mode == input.mode {
        if !input.guard_manifests.is_empty() {
            return Err(StoreError::InvalidInput {
                detail: "a no-op mode transition must not include Guard manifest mutations"
                    .to_owned(),
            });
        }
        return Ok(ConnectionModeTransitionOutcome {
            kind: ConnectionModeTransitionKind::Unchanged,
            connection,
            previous_integration_revision: previous_revision.clone(),
            current_integration_revision: previous_revision,
            rebound_guard_installation_ids: Vec::new(),
        });
    }

    let memberships =
        list_connection_projects_from_conn(&tx, &runtime_home, &input.connection_internal_id)?;
    let memberships_by_project = memberships
        .iter()
        .map(|membership| (membership.project_id.as_str(), membership))
        .collect::<BTreeMap<_, _>>();

    let stored_guard_ids = {
        let mut statement = tx.prepare(
            "SELECT guard_installation_id
               FROM guard_installations
              WHERE connection_internal_id = ?1
              ORDER BY guard_installation_id",
        )?;
        let rows = statement.query_map([&input.connection_internal_id], |row| row.get(0))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }
        ids
    };
    if stored_guard_ids.len() != memberships.len()
        || input.guard_manifests.len() != memberships.len()
        || stored_guard_ids.iter().collect::<BTreeSet<_>>()
            != input
                .guard_manifests
                .iter()
                .map(|rebind| &rebind.guard_installation_id)
                .collect::<BTreeSet<_>>()
        || memberships_by_project
            .keys()
            .copied()
            .collect::<BTreeSet<_>>()
            != input
                .guard_manifests
                .iter()
                .map(|rebind| rebind.project_id.as_str())
                .collect::<BTreeSet<_>>()
    {
        return Err(StoreError::Conflict {
            entity: "guard_installation",
            id: input.connection_internal_id.clone(),
            detail: "mode-transition inventory must contain exactly one current Guard Installation for every Connection Project"
                .to_owned(),
        });
    }

    let mut candidate_connection = connection.clone();
    candidate_connection.mode.clone_from(&input.mode);
    candidate_connection.integration_generation = candidate_connection
        .integration_generation
        .checked_add(1)
        .ok_or_else(|| StoreError::InvalidInput {
            detail: "Agent Connection integration generation is exhausted".to_owned(),
        })?;
    let candidate_revision = connection_integration_revision(&candidate_connection)?;
    let rebinds_by_id = input
        .guard_manifests
        .iter()
        .map(|rebind| (rebind.guard_installation_id.as_str(), rebind))
        .collect::<BTreeMap<_, _>>();

    for guard_installation_id in &stored_guard_ids {
        let rebind = rebinds_by_id
            .get(guard_installation_id.as_str())
            .expect("complete inventory was checked");
        let membership = memberships_by_project
            .get(rebind.project_id.as_str())
            .expect("complete project inventory was checked");
        let installation =
            guard_installation_from_conn(&tx, guard_installation_id)?.ok_or_else(|| {
                StoreError::NotFound {
                    entity: "guard_installation",
                    id: guard_installation_id.clone(),
                }
            })?;
        if installation.project_internal_id != membership.project_internal_id
            || installation.project_id != membership.project_id
            || installation.manifest_json != rebind.expected_manifest_json
        {
            return Err(StoreError::Conflict {
                entity: "guard_installation",
                id: guard_installation_id.clone(),
                detail:
                    "Guard Installation owner or manifest changed after mode-transition preflight"
                        .to_owned(),
            });
        }
        validate_stored_guard_installation_manifest_binding(
            &installation,
            &connection,
            &membership.project.repo_root,
        )?;

        let mut expected_manifest =
            guard_manifest_from_json(&installation.manifest_json).map_err(|_| {
                StoreError::corrupt_owner_state_json(
                    "guard_installations",
                    guard_installation_id.clone(),
                    "manifest_json",
                )
            })?;
        expected_manifest.integration_revision = candidate_revision.clone();
        let expected_candidate_json =
            serde_json::to_string(&expected_manifest).map_err(|error| {
                StoreError::InvalidInput {
                    detail: format!("candidate Guard manifest could not be serialized: {error}"),
                }
            })?;
        if rebind.manifest_json != expected_candidate_json {
            return Err(StoreError::InvalidInput {
                detail: format!(
                    "candidate Guard manifest {} must replace only the Connection integration revision",
                    guard_installation_id
                ),
            });
        }
        validate_guard_installation_upsert_binding(
            &GuardInstallationUpsert {
                guard_installation_id: rebind.guard_installation_id.clone(),
                connection_internal_id: input.connection_internal_id.clone(),
                project_id: rebind.project_id.clone(),
                manifest_json: rebind.manifest_json.clone(),
            },
            &candidate_connection,
            &membership.project,
        )?;
    }

    let changed = tx.execute(
        "UPDATE agent_connections
            SET verification_report_json = NULL,
                mode = ?2,
                integration_generation = integration_generation + 1,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE connection_internal_id = ?1
            AND mode = ?3
            AND integration_generation = ?4",
        params![
            input.connection_internal_id,
            input.mode,
            input.expected_mode,
            connection.integration_generation,
        ],
    )?;
    if changed != 1 {
        return Err(StoreError::Conflict {
            entity: "agent_connection",
            id: input.connection_internal_id,
            detail: "Connection mode changed during the atomic transition".to_owned(),
        });
    }

    for guard_installation_id in &stored_guard_ids {
        let rebind = rebinds_by_id
            .get(guard_installation_id.as_str())
            .expect("complete inventory was checked");
        let membership = memberships_by_project
            .get(rebind.project_id.as_str())
            .expect("complete project inventory was checked");
        let changed = tx.execute(
            "UPDATE guard_installations
                SET manifest_json = ?2,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE guard_installation_id = ?1
                AND connection_internal_id = ?3
                AND project_internal_id = ?4
                AND manifest_json = ?5",
            params![
                rebind.guard_installation_id,
                rebind.manifest_json,
                input.connection_internal_id,
                membership.project_internal_id,
                rebind.expected_manifest_json,
            ],
        )?;
        if changed != 1 {
            return Err(StoreError::Conflict {
                entity: "guard_installation",
                id: guard_installation_id.clone(),
                detail: "Guard manifest changed during the atomic mode transition".to_owned(),
            });
        }
    }

    let connection = require_agent_connection(&tx, &input.connection_internal_id)?;
    let current_revision = connection_integration_revision(&connection)?;
    if current_revision != candidate_revision {
        return Err(StoreError::Conflict {
            entity: "agent_connection",
            id: input.connection_internal_id,
            detail: "committed candidate Connection revision did not match preflight".to_owned(),
        });
    }
    for guard_installation_id in &stored_guard_ids {
        let rebind = rebinds_by_id
            .get(guard_installation_id.as_str())
            .expect("complete inventory was checked");
        let membership = memberships_by_project
            .get(rebind.project_id.as_str())
            .expect("complete project inventory was checked");
        let installation =
            guard_installation_from_conn(&tx, guard_installation_id)?.ok_or_else(|| {
                StoreError::NotFound {
                    entity: "guard_installation",
                    id: guard_installation_id.clone(),
                }
            })?;
        validate_stored_guard_installation_manifest_binding(
            &installation,
            &connection,
            &membership.project.repo_root,
        )?;
    }

    tx.commit()?;
    Ok(ConnectionModeTransitionOutcome {
        kind: ConnectionModeTransitionKind::Updated,
        connection,
        previous_integration_revision: previous_revision,
        current_integration_revision: current_revision,
        rebound_guard_installation_ids: stored_guard_ids,
    })
}

/// Replaces only the canonical verification report when the Connection
/// integration revision still matches the revision that was verified.
pub fn replace_agent_connection_verification_report_if_revision(
    context: &RuntimeHomeMutationContext<'_>,
    connection_internal_id: &str,
    expected_integration_revision: &IntegrationRevision,
    verification_report: Option<&ConnectionVerificationReport>,
) -> StoreResult<AgentConnectionRecord> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    let verification_report_json = verification_report
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("verification_report could not be serialized: {error}"),
        })?;
    let registry_path = registry_db_path(context.runtime_home().as_path());
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let existing =
        raw_agent_connection_record_from_conn(&tx, connection_internal_id)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "agent_connection",
                id: connection_internal_id.to_owned(),
            }
        })?;
    validate_stored_agent_connection_json_object(
        connection_internal_id,
        "metadata_json",
        &existing.metadata_json,
    )?;
    let current_integration_revision = connection_integration_revision(&existing)?;
    if current_integration_revision != *expected_integration_revision {
        return Err(StoreError::Conflict {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
            detail:
                "Connection integration revision changed before verification report persistence"
                    .to_owned(),
        });
    }
    let changed = tx.execute(
        "UPDATE agent_connections
            SET verification_report_json = ?2,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE connection_internal_id = ?1",
        params![connection_internal_id, verification_report_json],
    )?;
    if changed == 0 {
        return Err(StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        });
    }
    let connection =
        agent_connection_record_from_conn(&tx, connection_internal_id)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "agent_connection",
                id: connection_internal_id.to_owned(),
            }
        })?;
    tx.commit()?;
    Ok(connection)
}

/// Adds a registered project to a connection allowlist.
pub fn add_connection_project(
    context: &RuntimeHomeMutationContext<'_>,
    registration: ConnectionProjectRegistration,
) -> StoreResult<ConnectionProjectRecord> {
    validate_connection_project_registration(&registration)?;
    let runtime_home = context.runtime_home().as_path().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let connection = require_agent_connection(&tx, &registration.connection_internal_id)?;
    reject_generic_pending_host_cleanup_mutation(&connection)?;
    let project =
        require_current_project_registration(&tx, &runtime_home, &registration.project_id)?;
    tx.execute(
        "INSERT OR IGNORE INTO connection_projects (
            connection_internal_id,
            project_internal_id,
            created_at
        )
        VALUES (
            ?1,
            ?2,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        )",
        params![
            connection.connection_internal_id,
            project.project_internal_id
        ],
    )?;
    tx.commit()?;

    connection_project_record_from_conn(
        &conn,
        &runtime_home,
        &connection.connection_internal_id,
        &project.project_internal_id,
    )?
    .ok_or_else(|| StoreError::NotFound {
        entity: "connection_project",
        id: format!(
            "{}/{}",
            connection.connection_internal_id, project.project_internal_id
        ),
    })
}

/// Atomically removes one Connection Project and its connection-owned Registry
/// integration state.
pub fn remove_connection_project(
    context: &RuntimeHomeMutationContext<'_>,
    connection_internal_id: &str,
    project_id: &str,
) -> StoreResult<ConnectionProjectRemovalOutcome> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    validate_project_id(project_id)?;
    let runtime_home = context.runtime_home().as_path().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let connection = require_agent_connection(&tx, connection_internal_id)?;
    reject_generic_pending_host_cleanup_mutation(&connection)?;
    let project = require_current_project_registration(&tx, &runtime_home, project_id)?;
    retire_connection_project_state_in_transaction(
        &tx,
        &connection.connection_internal_id,
        &project.project_internal_id,
        project_id,
    )?;

    let remaining_project_count: i64 = tx.query_row(
        "SELECT COUNT(*)
           FROM connection_projects
          WHERE connection_internal_id = ?1",
        [&connection.connection_internal_id],
        |row| row.get(0),
    )?;
    let remaining_project_count =
        usize::try_from(remaining_project_count).map_err(|_| StoreError::SchemaInvariant {
            database_kind: "registry",
            detail: "connection membership count cannot be represented as usize".to_owned(),
        })?;

    let connection_removed = remaining_project_count == 0;
    if connection_removed {
        tx.execute(
            "DELETE FROM mcp_runtime_project_session_bindings
              WHERE connection_internal_id = ?1",
            [&connection.connection_internal_id],
        )?;
        tx.execute(
            "DELETE FROM guard_installations
              WHERE connection_internal_id = ?1",
            [&connection.connection_internal_id],
        )?;
        tx.execute(
            "DELETE FROM mcp_runtime_sessions
              WHERE connection_internal_id = ?1",
            [&connection.connection_internal_id],
        )?;
        tx.execute(
            "DELETE FROM managed_mcp_launch_leases
              WHERE connection_internal_id = ?1",
            [&connection.connection_internal_id],
        )?;
        let removed = tx.execute(
            "DELETE FROM agent_connections
              WHERE connection_internal_id = ?1",
            [&connection.connection_internal_id],
        )?;
        if removed != 1 {
            return Err(StoreError::NotFound {
                entity: "agent_connection",
                id: connection_internal_id.to_owned(),
            });
        }
    }

    tx.commit()?;
    Ok(ConnectionProjectRemovalOutcome {
        membership_removed: true,
        connection_removed,
        remaining_project_count,
    })
}

/// Adds one staged project binding and guard installation, retires or disables
/// superseded bindings, and activates the requested connection in one registry
/// transaction. A disabled last-project binding remains as durable pending
/// host-cleanup inventory until the caller completes that cleanup.
pub fn staged_connection_migration_state(
    context: &RuntimeHomeMutationContext<'_>,
    connection_internal_id: &str,
    project_id: &str,
    expected_superseded: &[SupersededConnectionProject],
) -> StoreResult<(AgentConnectionRecord, StagedConnectionMigrationState)> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    validate_project_id(project_id)?;
    let expected_ids = expected_superseded
        .iter()
        .map(|superseded| {
            validate_identifier(
                "superseded.connection_internal_id",
                &superseded.connection_internal_id,
            )?;
            validate_project_id(&superseded.project_id)?;
            if superseded.project_id != project_id {
                return Err(StoreError::InvalidInput {
                    detail: "superseded project must match the staged migration project".to_owned(),
                });
            }
            Ok(superseded.connection_internal_id.clone())
        })
        .collect::<StoreResult<BTreeSet<_>>>()?;
    if expected_ids.len() != expected_superseded.len() || expected_ids.is_empty() {
        return Err(StoreError::InvalidInput {
            detail: "staged migration requires unique superseded connection bindings".to_owned(),
        });
    }

    let runtime_home = context.runtime_home().as_path().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let project = require_current_project_registration(&tx, &runtime_home, project_id)?;
    let requested = require_agent_connection(&tx, connection_internal_id)?;
    reject_generic_pending_host_cleanup_mutation(&requested)?;
    let requested_membership_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM connection_projects
          WHERE connection_internal_id = ?1
            AND project_internal_id = ?2",
        params![connection_internal_id, project.project_internal_id],
        |row| row.get(0),
    )?;
    if requested_membership_count == 0 {
        tx.commit()?;
        return Ok((requested, StagedConnectionMigrationState::Staged));
    }
    if requested_membership_count != 1 || !requested.enabled {
        return Err(StoreError::Conflict {
            entity: "connection_project",
            id: format!("{connection_internal_id}/{project_id}"),
            detail: "requested migration membership is active without a resumable Registry switch"
                .to_owned(),
        });
    }

    let mut stmt = tx.prepare(
        "SELECT ac.connection_internal_id
           FROM connection_projects AS cp
           JOIN agent_connections AS ac
             ON ac.connection_internal_id = cp.connection_internal_id
          WHERE cp.project_internal_id = ?1
            AND ac.connection_internal_id <> ?2
            AND ac.host_kind = 'codex'
            AND ac.intent IN ('personal', 'shared')
          ORDER BY ac.connection_internal_id",
    )?;
    let rows = stmt.query_map(
        params![project.project_internal_id, connection_internal_id],
        |row| row.get::<_, String>(0),
    )?;
    let mut pending_ids = BTreeSet::new();
    for row in rows {
        let candidate_id = row?;
        let candidate = require_agent_connection(&tx, &candidate_id)?;
        if connection_metadata_has_pending_host_cleanup(
            &candidate.metadata_json,
            project_id,
            connection_internal_id,
        ) {
            if candidate.enabled {
                return Err(StoreError::Conflict {
                    entity: "agent_connection",
                    id: candidate_id,
                    detail: "pending host cleanup connection became enabled".to_owned(),
                });
            }
            pending_ids.insert(candidate_id);
        } else if candidate.enabled
            || connection_metadata_has_pending_host_cleanup_for_project(
                &candidate.metadata_json,
                project_id,
            )
        {
            return Err(StoreError::Conflict {
                entity: "connection_project",
                id: project_id.to_owned(),
                detail: "active migration inventory does not match the requested replacement"
                    .to_owned(),
            });
        }
    }
    drop(stmt);
    if pending_ids != expected_ids {
        return Err(StoreError::Conflict {
            entity: "connection_project",
            id: project_id.to_owned(),
            detail: "requested migration membership changed while staging was in progress"
                .to_owned(),
        });
    }
    let pending_connection_ids = pending_ids.into_iter().collect();
    tx.commit()?;
    Ok((
        requested,
        StagedConnectionMigrationState::CleanupResume {
            pending_connection_ids,
        },
    ))
}

pub fn activate_staged_connection(
    context: &RuntimeHomeMutationContext<'_>,
    connection_internal_id: &str,
    project_id: &str,
    superseded: &[SupersededConnectionProject],
    guard_upsert: GuardInstallationUpsert,
) -> StoreResult<(AgentConnectionRecord, GuardInstallationRecord, Vec<String>)> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    validate_project_id(project_id)?;
    if guard_upsert.connection_internal_id != connection_internal_id
        || guard_upsert.project_id != project_id
    {
        return Err(StoreError::InvalidInput {
            detail: "the staged guard installation must match the requested connection and project"
                .to_owned(),
        });
    }
    for retired in superseded {
        validate_identifier(
            "superseded.connection_internal_id",
            &retired.connection_internal_id,
        )?;
        validate_project_id(&retired.project_id)?;
        if retired.connection_internal_id == connection_internal_id {
            return Err(StoreError::InvalidInput {
                detail: "the staged connection cannot supersede itself".to_owned(),
            });
        }
        if retired.project_id != project_id {
            return Err(StoreError::InvalidInput {
                detail: "every superseded binding must belong to the requested project".to_owned(),
            });
        }
    }

    let runtime_home = context.runtime_home().as_path().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let staged_connection = require_agent_connection(&tx, connection_internal_id)?;
    reject_generic_pending_host_cleanup_mutation(&staged_connection)?;
    let project = require_current_project_registration(&tx, &runtime_home, project_id)?;
    let expected_superseded = superseded
        .iter()
        .map(|retired| retired.connection_internal_id.clone())
        .collect::<BTreeSet<_>>();
    if expected_superseded.len() != superseded.len() {
        return Err(StoreError::InvalidInput {
            detail: "superseded connection bindings must not be duplicated".to_owned(),
        });
    }
    let mut current_stmt = tx.prepare(
        "SELECT ac.connection_internal_id
           FROM connection_projects AS cp
           JOIN agent_connections AS ac
             ON ac.connection_internal_id = cp.connection_internal_id
          WHERE cp.project_internal_id = ?1
            AND ac.connection_internal_id <> ?2
            AND ac.host_kind = 'codex'
            AND ac.intent IN ('personal', 'shared')
          ORDER BY ac.connection_internal_id",
    )?;
    let current_rows = current_stmt.query_map(
        params![project.project_internal_id, connection_internal_id],
        |row| row.get::<_, String>(0),
    )?;
    let mut current_superseded = BTreeSet::new();
    for row in current_rows {
        let current_connection_id = row?;
        let current_connection = require_agent_connection(&tx, &current_connection_id)?;
        if current_connection.enabled
            || expected_superseded.contains(&current_connection_id)
            || connection_metadata_has_pending_host_cleanup_for_project(
                &current_connection.metadata_json,
                project_id,
            )
        {
            current_superseded.insert(current_connection_id);
        }
    }
    drop(current_stmt);
    if current_superseded != expected_superseded {
        return Err(StoreError::Conflict {
            entity: "connection_project",
            id: project_id.to_owned(),
            detail: "the supported integration membership inventory changed while the migration was staged"
                .to_owned(),
        });
    }
    let target_membership_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM connection_projects
          WHERE connection_internal_id = ?1
            AND project_internal_id = ?2",
        params![connection_internal_id, project.project_internal_id],
        |row| row.get(0),
    )?;
    if target_membership_count != 0 {
        return Err(StoreError::InvalidInput {
            detail: "the requested project membership must remain inactive while staged".to_owned(),
        });
    }
    tx.execute(
        "INSERT INTO connection_projects (
            connection_internal_id,
            project_internal_id,
            created_at
        ) VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        params![
            staged_connection.connection_internal_id,
            project.project_internal_id
        ],
    )?;

    let mut pending_host_cleanup_connections = Vec::new();
    for retired in superseded {
        let retired_connection = require_agent_connection(&tx, &retired.connection_internal_id)?;
        require_rebasable_pending_host_cleanup_metadata(&retired_connection, project_id)?;
        let project_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM connection_projects WHERE connection_internal_id = ?1",
            [&retired_connection.connection_internal_id],
            |row| row.get(0),
        )?;
        if project_count == 1 {
            let metadata_json = metadata_with_pending_host_cleanup(
                &retired_connection.metadata_json,
                project_id,
                connection_internal_id,
            )?;
            tx.execute(
                "UPDATE agent_connections
                    SET enabled = 0,
                        metadata_json = ?2,
                        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                  WHERE connection_internal_id = ?1",
                params![retired_connection.connection_internal_id, metadata_json],
            )?;
            pending_host_cleanup_connections
                .push(retired_connection.connection_internal_id.clone());
        } else if connection_metadata_has_pending_host_cleanup_for_project(
            &retired_connection.metadata_json,
            project_id,
        ) {
            return Err(StoreError::Conflict {
                entity: "agent_connection",
                id: retired_connection.connection_internal_id,
                detail: "pending host cleanup gained another project membership".to_owned(),
            });
        } else {
            retire_connection_project_state_in_transaction(
                &tx,
                &retired_connection.connection_internal_id,
                &project.project_internal_id,
                project_id,
            )?;
        }
    }

    tx.execute(
        "UPDATE agent_connections
            SET enabled = 1,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE connection_internal_id = ?1",
        [connection_internal_id],
    )?;
    upsert_guard_installation_in_transaction(&tx, &guard_upsert)?;
    let connection =
        agent_connection_record_from_conn(&tx, connection_internal_id)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "agent_connection",
                id: connection_internal_id.to_owned(),
            }
        })?;
    let installation = guard_installation_from_conn(&tx, &guard_upsert.guard_installation_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "guard_installation",
            id: guard_upsert.guard_installation_id.clone(),
        })?;
    tx.commit()?;
    Ok((connection, installation, pending_host_cleanup_connections))
}

/// Removes durable disabled memberships after caller-owned host cleanup.
///
/// The callback runs after an initial durable-state validation and before the
/// final Registry transaction. Generic connection APIs cannot mutate marked
/// rows, and the final transaction revalidates every marker before removing it.
/// External callback effects cannot be rolled back if the later Registry
/// commit fails; the retained disabled memberships make cleanup retryable.
pub fn complete_pending_host_cleanup<E>(
    context: &RuntimeHomeMutationContext<'_>,
    project_id: &str,
    replacement_connection_id: &str,
    pending_connection_ids: &[String],
    cleanup_host_configuration: impl FnOnce(&[String]) -> Result<(), E>,
) -> Result<(), PendingHostCleanupError<E>> {
    validate_project_id(project_id)?;
    validate_identifier(
        "replacement_connection_internal_id",
        replacement_connection_id,
    )?;
    let unique_ids = pending_connection_ids.iter().collect::<BTreeSet<_>>();
    if unique_ids.len() != pending_connection_ids.len() {
        return Err(StoreError::InvalidInput {
            detail: "pending host-cleanup connection ids must not be duplicated".to_owned(),
        }
        .into());
    }
    for connection_id in pending_connection_ids {
        validate_identifier("pending.connection_internal_id", connection_id)?;
    }

    let runtime_home = context.runtime_home().as_path().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    validate_pending_host_cleanup_inventory(
        &runtime_home,
        &registry_path,
        project_id,
        replacement_connection_id,
        pending_connection_ids,
    )?;
    cleanup_host_configuration(pending_connection_ids).map_err(PendingHostCleanupError::Host)?;

    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let project = require_current_project_registration(&tx, &runtime_home, project_id)?;
    validate_pending_host_cleanup_inventory_in_transaction(
        &tx,
        &project,
        project_id,
        replacement_connection_id,
        pending_connection_ids,
    )?;
    for connection_id in pending_connection_ids {
        retire_connection_project_state_in_transaction(
            &tx,
            connection_id,
            &project.project_internal_id,
            project_id,
        )?;
        let connection = require_agent_connection(&tx, connection_id)?;
        let metadata_json = metadata_without_pending_host_cleanup(&connection.metadata_json)?;
        let updated = tx.execute(
            "UPDATE agent_connections
                SET metadata_json = ?2,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE connection_internal_id = ?1",
            params![connection_id, metadata_json],
        )?;
        if updated != 1 {
            return Err(StoreError::NotFound {
                entity: "agent_connection",
                id: connection_id.clone(),
            }
            .into());
        }
    }
    tx.commit()?;
    Ok(())
}

fn validate_pending_host_cleanup_inventory<E>(
    runtime_home: &Path,
    registry_path: &Path,
    project_id: &str,
    replacement_connection_id: &str,
    pending_connection_ids: &[String],
) -> Result<(), PendingHostCleanupError<E>> {
    let conn = open_registry_database_read_only(registry_path)?;
    require_runtime_home(&conn, registry_path)?;
    let project = require_current_project_registration(&conn, runtime_home, project_id)?;
    validate_pending_host_cleanup_inventory_in_transaction(
        &conn,
        &project,
        project_id,
        replacement_connection_id,
        pending_connection_ids,
    )?;
    Ok(())
}

fn validate_pending_host_cleanup_inventory_in_transaction<E>(
    tx: &Connection,
    project: &ProjectRecord,
    project_id: &str,
    replacement_connection_id: &str,
    pending_connection_ids: &[String],
) -> Result<(), PendingHostCleanupError<E>> {
    let replacement = require_agent_connection(tx, replacement_connection_id)?;
    let replacement_project_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM connection_projects
          WHERE connection_internal_id = ?1
            AND project_internal_id = ?2",
        params![replacement_connection_id, project.project_internal_id],
        |row| row.get(0),
    )?;
    if !replacement.enabled
        || replacement_project_count != 1
        || connection_metadata_contains_pending_host_cleanup_key(&replacement.metadata_json)
    {
        return Err(StoreError::Conflict {
            entity: "connection_project",
            id: format!("{replacement_connection_id}/{project_id}"),
            detail: "pending host cleanup replacement is no longer one enabled current membership"
                .to_owned(),
        }
        .into());
    }

    for connection_id in pending_connection_ids {
        let connection = require_agent_connection(tx, connection_id)?;
        let total_project_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM connection_projects WHERE connection_internal_id = ?1",
            [connection_id],
            |row| row.get(0),
        )?;
        let target_project_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM connection_projects
              WHERE connection_internal_id = ?1
                AND project_internal_id = ?2",
            params![connection_id, project.project_internal_id],
            |row| row.get(0),
        )?;
        if connection.enabled
            || total_project_count != 1
            || target_project_count != 1
            || !connection_metadata_has_pending_host_cleanup(
                &connection.metadata_json,
                project_id,
                replacement_connection_id,
            )
        {
            return Err(StoreError::Conflict {
                entity: "connection_project",
                id: format!("{connection_id}/{project_id}"),
                detail: "pending host cleanup no longer has one disabled retained membership"
                    .to_owned(),
            }
            .into());
        }
    }
    Ok(())
}

fn retire_connection_project_state_in_transaction(
    tx: &Connection,
    connection_internal_id: &str,
    project_internal_id: &str,
    project_id: &str,
) -> StoreResult<()> {
    let membership_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM connection_projects
          WHERE connection_internal_id = ?1
            AND project_internal_id = ?2",
        params![connection_internal_id, project_internal_id],
        |row| row.get(0),
    )?;
    if membership_count != 1 {
        return Err(StoreError::NotFound {
            entity: "connection_project",
            id: format!("{connection_internal_id}/{project_id}"),
        });
    }

    tx.execute(
        "DELETE FROM mcp_runtime_project_session_bindings
          WHERE connection_internal_id = ?1
            AND project_internal_id = ?2",
        params![connection_internal_id, project_internal_id],
    )?;
    tx.execute(
        "DELETE FROM guard_integration_verification_runs
          WHERE connection_internal_id = ?1
            AND project_internal_id = ?2",
        params![connection_internal_id, project_internal_id],
    )?;
    tx.execute(
        "DELETE FROM guard_installations
          WHERE connection_internal_id = ?1
            AND project_internal_id = ?2",
        params![connection_internal_id, project_internal_id],
    )?;
    let membership_removed = tx.execute(
        "DELETE FROM connection_projects
          WHERE connection_internal_id = ?1
            AND project_internal_id = ?2",
        params![connection_internal_id, project_internal_id],
    )?;
    if membership_removed != 1 {
        return Err(StoreError::SchemaInvariant {
            database_kind: "registry",
            detail: format!(
                "connection project retirement removed {membership_removed} memberships for {connection_internal_id}/{project_id}"
            ),
        });
    }
    Ok(())
}

/// Lists the explicitly allowed projects for one Agent Connection.
pub fn list_connection_projects(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Vec<ConnectionProjectRecord>> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Err(StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        });
    }

    let conn = open_registry_database_read_only(registry_path)?;
    require_agent_connection(&conn, connection_internal_id)?;
    list_connection_projects_from_conn(&conn, &runtime_home, connection_internal_id)
}

/// Lists explicitly allowed projects without creating, migrating, or writing registry state.
pub fn list_connection_projects_read_only(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Vec<ConnectionProjectRecord>> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Err(StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        });
    }

    let conn = open_registry_database_read_only(registry_path)?;
    require_agent_connection(&conn, connection_internal_id)?;
    list_connection_projects_from_conn(&conn, &runtime_home, connection_internal_id)
}

/// Lists project memberships for raw Agent Connection diagnostic state without
/// validating the connection's persisted JSON owner fields.
pub fn list_connection_projects_for_diagnostics(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Vec<ConnectionProjectRecord>> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Err(StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        });
    }

    let conn = open_registry_database_read_only(registry_path)?;
    if raw_agent_connection_record_from_conn(&conn, connection_internal_id)?.is_none() {
        return Err(StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        });
    }
    list_connection_projects_from_conn(&conn, &runtime_home, connection_internal_id)
}

fn list_connection_projects_from_conn(
    conn: &Connection,
    runtime_home: &Path,
    connection_internal_id: &str,
) -> StoreResult<Vec<ConnectionProjectRecord>> {
    let mut stmt = conn.prepare(
        "SELECT
            cp.connection_internal_id,
            cp.project_internal_id,
            cp.created_at,
            p.project_name,
            p.project_alias,
            p.runtime_home_id,
            p.repo_root,
            p.project_home,
            p.state_db_path,
            p.status,
            p.metadata_json
         FROM connection_projects AS cp
         JOIN projects AS p
           ON p.project_internal_id = cp.project_internal_id
        WHERE cp.connection_internal_id = ?1
        ORDER BY p.project_name, cp.project_internal_id",
    )?;
    let mut rows = stmt.query([connection_internal_id])?;
    let mut projects = Vec::new();
    while let Some(row) = rows.next()? {
        let project = connection_project_record_from_row(row)?;
        projects.push(validate_connection_project_record(runtime_home, project)?);
    }
    Ok(projects)
}

/// Returns current access facts for a connection/project pair.
pub fn agent_connection_project_access(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
    project_id: &str,
) -> StoreResult<Option<AgentConnectionProjectAccess>> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    validate_project_id(project_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database_read_only(registry_path)?;
    let Some(connection) = agent_connection_record_from_conn(&conn, connection_internal_id)? else {
        return Ok(None);
    };
    let project = raw_project_record_from_conn(&conn, project_id)?;
    let project_allowed = if let Some(project) = &project {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*)
               FROM connection_projects
              WHERE connection_internal_id = ?1
                AND project_internal_id = ?2",
            params![
                connection.connection_internal_id,
                project.project_internal_id
            ],
            |row| row.get(0),
        )?;
        count > 0
    } else {
        false
    };
    let project = project
        .map(|project| validate_current_project_registration(&runtime_home, &project))
        .transpose()?;
    let resolved_project_id = project
        .as_ref()
        .map(|project| project.project_id.clone())
        .unwrap_or_else(|| project_id.to_owned());

    Ok(Some(AgentConnectionProjectAccess {
        connection_internal_id: connection.connection_internal_id,
        project_id: resolved_project_id,
        connection_enabled: connection.enabled,
        project_allowed,
        project,
    }))
}

/// Returns current access facts without creating, migrating, or writing registry state.
pub fn agent_connection_project_access_read_only(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
    project_id: &str,
) -> StoreResult<Option<AgentConnectionProjectAccess>> {
    validate_identifier("connection_internal_id", connection_internal_id)?;
    validate_project_id(project_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database_read_only(registry_path)?;
    let Some(connection) = agent_connection_record_from_conn(&conn, connection_internal_id)? else {
        return Ok(None);
    };
    let project = raw_project_record_from_conn(&conn, project_id)?;
    let project_allowed = if let Some(project) = &project {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*)
               FROM connection_projects
              WHERE connection_internal_id = ?1
                AND project_internal_id = ?2",
            params![
                connection.connection_internal_id,
                project.project_internal_id
            ],
            |row| row.get(0),
        )?;
        count > 0
    } else {
        false
    };
    let project = project
        .map(|project| validate_current_project_registration(&runtime_home, &project))
        .transpose()?;
    let resolved_project_id = project
        .as_ref()
        .map(|project| project.project_id.clone())
        .unwrap_or_else(|| project_id.to_owned());

    Ok(Some(AgentConnectionProjectAccess {
        connection_internal_id: connection.connection_internal_id,
        project_id: resolved_project_id,
        connection_enabled: connection.enabled,
        project_allowed,
        project,
    }))
}

/// Returns whether the connection is enabled and the project is allowlisted.
pub fn is_agent_connection_project_allowed(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
    project_id: &str,
) -> StoreResult<bool> {
    Ok(
        agent_connection_project_access(runtime_home, connection_internal_id, project_id)?
            .is_some_and(|access| access.connection_enabled && access.project_allowed),
    )
}

fn validate_agent_connection_registration(
    registration: &AgentConnectionRegistration,
) -> StoreResult<()> {
    validate_identifier(
        "connection_internal_id",
        &registration.connection_internal_id,
    )?;
    validate_host_kind_scope(&registration.host_kind, &registration.host_scope)?;
    validate_connection_intent(&registration.intent)?;
    validate_nonempty("server_name", &registration.server_name)?;
    validate_nonempty("config_target", &registration.config_target)?;
    validate_connection_mode(&registration.mode)?;
    validate_nonempty("managed_fingerprint", &registration.managed_fingerprint)?;
    validate_json_object(
        "agent_connections.metadata_json",
        &registration.metadata_json,
    )?;
    reject_pending_host_cleanup_metadata(&registration.metadata_json)
}

fn validate_agent_connection_natural_key_registration(
    registration: &AgentConnectionNaturalKeyRegistration,
) -> StoreResult<()> {
    validate_host_kind_scope(&registration.host_kind, &registration.host_scope)?;
    validate_connection_intent(&registration.intent)?;
    if let Some(project_ref) = &registration.project_ref {
        validate_project_id(project_ref)?;
    }
    validate_nonempty("server_name", &registration.server_name)?;
    validate_nonempty("config_target", &registration.config_target)?;
    validate_connection_mode(&registration.mode)?;
    validate_nonempty("managed_fingerprint", &registration.managed_fingerprint)?;
    validate_json_object(
        "agent_connections.metadata_json",
        &registration.metadata_json,
    )?;
    reject_pending_host_cleanup_metadata(&registration.metadata_json)
}

fn validate_agent_connection_natural_key(key: &AgentConnectionNaturalKey) -> StoreResult<()> {
    validate_host_kind_scope(&key.host_kind, &key.host_scope)?;
    validate_connection_intent(&key.intent)?;
    if let Some(project_ref) = &key.project_ref {
        validate_project_id(project_ref)?;
    }
    validate_nonempty("server_name", &key.server_name)?;
    validate_nonempty("config_target", &key.config_target)
}

fn validate_agent_connection_write_registration(
    registration: &AgentConnectionWriteRegistration,
) -> StoreResult<()> {
    validate_identifier(
        "connection_internal_id",
        &registration.connection_internal_id,
    )?;
    validate_host_kind_scope(&registration.host_kind, &registration.host_scope)?;
    validate_connection_intent(&registration.intent)?;
    if let Some(project_internal_id) = &registration.project_internal_id {
        validate_project_id(project_internal_id)?;
    }
    validate_nonempty("server_name", &registration.server_name)?;
    validate_nonempty("config_target", &registration.config_target)?;
    validate_connection_mode(&registration.mode)?;
    validate_nonempty("managed_fingerprint", &registration.managed_fingerprint)?;
    validate_json_object(
        "agent_connections.metadata_json",
        &registration.metadata_json,
    )?;
    reject_pending_host_cleanup_metadata(&registration.metadata_json)
}

fn validate_connection_project_registration(
    registration: &ConnectionProjectRegistration,
) -> StoreResult<()> {
    validate_identifier(
        "connection_internal_id",
        &registration.connection_internal_id,
    )?;
    validate_project_id(&registration.project_id)
}

fn validate_identifier(field: &'static str, value: &str) -> StoreResult<()> {
    validate_nonempty(field, value)?;
    if value.contains('\0') {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not contain NUL bytes"),
        })
    } else {
        Ok(())
    }
}

fn validate_nonempty(field: &'static str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}

fn validate_host_kind_scope(host_kind: &str, host_scope: &str) -> StoreResult<()> {
    let valid = matches!(
        (host_kind, host_scope),
        (HOST_KIND_CODEX, HOST_SCOPE_USER) | (HOST_KIND_CODEX, HOST_SCOPE_PROJECT)
    );
    if valid {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "host_kind and host_scope must match the supported matrix".to_owned(),
        })
    }
}

fn validate_connection_intent(intent: &str) -> StoreResult<()> {
    if matches!(
        intent,
        CONNECTION_INTENT_PERSONAL | CONNECTION_INTENT_SHARED
    ) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "intent must be personal or shared".to_owned(),
        })
    }
}

fn validate_connection_mode(mode: &str) -> StoreResult<()> {
    if matches!(mode, CONNECTION_MODE_READ_ONLY | CONNECTION_MODE_WORKFLOW) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "mode must be read_only or workflow".to_owned(),
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

fn enabled_as_i64(enabled: bool) -> i64 {
    if enabled {
        1
    } else {
        0
    }
}

fn require_runtime_home(conn: &Connection, registry_path: &Path) -> StoreResult<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM runtime_home", [], |row| row.get(0))?;
    if count == 1 {
        Ok(())
    } else {
        Err(StoreError::NotFound {
            entity: "runtime_home",
            id: registry_path.display().to_string(),
        })
    }
}

fn require_agent_connection(
    conn: &Connection,
    connection_internal_id: &str,
) -> StoreResult<AgentConnectionRecord> {
    agent_connection_record_from_conn(conn, connection_internal_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        }
    })
}

fn require_current_project_registration(
    conn: &Connection,
    runtime_home: &Path,
    project_id: &str,
) -> StoreResult<ProjectRecord> {
    project_record_from_conn(conn, runtime_home, project_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "project",
        id: project_id.to_owned(),
    })
}

pub(crate) fn agent_connection_record_from_conn(
    conn: &Connection,
    connection_internal_id: &str,
) -> StoreResult<Option<AgentConnectionRecord>> {
    raw_agent_connection_record_from_conn(conn, connection_internal_id)?
        .map(validate_stored_agent_connection)
        .transpose()
}

pub(crate) fn raw_agent_connection_record_from_conn(
    conn: &Connection,
    connection_internal_id: &str,
) -> StoreResult<Option<AgentConnectionRecord>> {
    conn.query_row(
        "SELECT
            connection_internal_id,
            integration_instance_id,
            host_kind,
            intent,
            host_scope,
            project_internal_id,
            server_name,
            config_target,
            mode,
            enabled,
            managed_fingerprint,
            integration_generation,
            verification_report_json,
            created_at,
            updated_at,
            metadata_json
         FROM agent_connections
         WHERE connection_internal_id = ?1",
        [connection_internal_id],
        agent_connection_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)?
    .map(decode_agent_connection_record)
    .transpose()
}

#[derive(Debug)]
struct RawAgentConnectionRecord {
    connection_internal_id: String,
    integration_instance_id: String,
    host_kind: String,
    intent: String,
    host_scope: String,
    project_internal_id: Option<String>,
    server_name: String,
    config_target: String,
    mode: String,
    enabled: bool,
    managed_fingerprint: String,
    integration_generation: i64,
    verification_report_json: Option<String>,
    created_at: String,
    updated_at: String,
    metadata_json: String,
}

fn agent_connection_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RawAgentConnectionRecord> {
    let connection_internal_id = row.get::<_, String>(0)?;
    Ok(RawAgentConnectionRecord {
        connection_internal_id,
        integration_instance_id: row.get(1)?,
        host_kind: row.get(2)?,
        intent: row.get(3)?,
        host_scope: row.get(4)?,
        project_internal_id: row.get(5)?,
        server_name: row.get(6)?,
        config_target: row.get(7)?,
        mode: row.get(8)?,
        enabled: row.get::<_, i64>(9)? == 1,
        managed_fingerprint: row.get(10)?,
        integration_generation: row.get(11)?,
        verification_report_json: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        metadata_json: row.get(15)?,
    })
}

fn decode_agent_connection_record(
    raw: RawAgentConnectionRecord,
) -> StoreResult<AgentConnectionRecord> {
    let integration_instance_id =
        ConnectionIntegrationInstanceId::parse(raw.integration_instance_id).map_err(|_| {
            StoreError::CorruptOwnerStateValue {
                database_kind: "registry",
                table: "agent_connections",
                record_ref: raw.connection_internal_id.clone(),
                logical_column: "integration_instance_id",
            }
        })?;
    Ok(AgentConnectionRecord {
        connection_internal_id: raw.connection_internal_id,
        integration_instance_id,
        host_kind: raw.host_kind,
        intent: raw.intent,
        host_scope: raw.host_scope,
        project_internal_id: raw.project_internal_id,
        server_name: raw.server_name,
        config_target: raw.config_target,
        mode: raw.mode,
        enabled: raw.enabled,
        managed_fingerprint: raw.managed_fingerprint,
        integration_generation: raw.integration_generation,
        verification_report_json: raw.verification_report_json,
        created_at: raw.created_at,
        updated_at: raw.updated_at,
        metadata_json: raw.metadata_json,
    })
}

fn validate_stored_agent_connection(
    connection: AgentConnectionRecord,
) -> StoreResult<AgentConnectionRecord> {
    if connection.integration_generation < 0 {
        return Err(StoreError::CorruptOwnerStateJson {
            database_kind: "registry",
            table: "agent_connections",
            record_ref: connection.connection_internal_id.clone(),
            logical_column: "integration_generation",
        });
    }
    connection.verification_report()?;
    validate_stored_agent_connection_json_object(
        &connection.connection_internal_id,
        "metadata_json",
        &connection.metadata_json,
    )?;
    Ok(connection)
}

fn validate_stored_agent_connection_json_object(
    connection_internal_id: &str,
    logical_column: &'static str,
    text: &str,
) -> StoreResult<()> {
    if matches!(serde_json::from_str::<Value>(text), Ok(Value::Object(_))) {
        Ok(())
    } else {
        Err(StoreError::CorruptOwnerStateJson {
            database_kind: "registry",
            table: "agent_connections",
            record_ref: connection_internal_id.to_owned(),
            logical_column,
        })
    }
}

fn connection_project_record_from_conn(
    conn: &Connection,
    runtime_home: &Path,
    connection_internal_id: &str,
    project_id: &str,
) -> StoreResult<Option<ConnectionProjectRecord>> {
    let record = conn
        .query_row(
            "SELECT
                cp.connection_internal_id,
                cp.project_internal_id,
                cp.created_at,
                p.project_name,
                p.project_alias,
                p.runtime_home_id,
                p.repo_root,
                p.project_home,
                p.state_db_path,
                p.status,
                p.metadata_json
             FROM connection_projects AS cp
             JOIN projects AS p
               ON p.project_internal_id = cp.project_internal_id
            WHERE cp.connection_internal_id = ?1
              AND cp.project_internal_id = ?2",
            params![connection_internal_id, project_id],
            connection_project_record_from_row,
        )
        .optional()
        .map_err(StoreError::from)?;
    record
        .map(|record| validate_connection_project_record(runtime_home, record))
        .transpose()
}

fn project_record_from_conn(
    conn: &Connection,
    runtime_home: &Path,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    let project = raw_project_record_from_conn(conn, project_id)?;
    project
        .map(|project| validate_current_project_registration(runtime_home, &project))
        .transpose()
}

fn validate_connection_project_record(
    runtime_home: &Path,
    mut record: ConnectionProjectRecord,
) -> StoreResult<ConnectionProjectRecord> {
    record.project = validate_current_project_registration(runtime_home, &record.project)?;
    Ok(record)
}

fn connection_project_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ConnectionProjectRecord> {
    let project_id = row.get::<_, String>(1)?;
    let connection_internal_id = row.get::<_, String>(0)?;
    Ok(ConnectionProjectRecord {
        connection_internal_id,
        project_id: project_id.clone(),
        project_internal_id: project_id.clone(),
        created_at: row.get(2)?,
        project: ProjectRecord {
            project_id: project_id.clone(),
            project_internal_id: project_id,
            project_name: row.get(3)?,
            project_alias: row.get(4)?,
            runtime_home_id: row.get(5)?,
            repo_root: PathBuf::from(row.get::<_, String>(6)?),
            project_home: PathBuf::from(row.get::<_, String>(7)?),
            state_db_path: PathBuf::from(row.get::<_, String>(8)?),
            status: row.get(9)?,
            metadata_json: row.get(10)?,
        },
    })
}

fn existing_connection_internal_id_for_target(
    conn: &Connection,
    registration: &AgentConnectionWriteRegistration,
) -> StoreResult<Option<String>> {
    conn.query_row(
        "SELECT connection_internal_id
           FROM agent_connections
          WHERE host_kind = ?1
            AND intent = ?2
            AND host_scope = ?3
            AND (
                (project_internal_id IS NULL AND ?4 IS NULL)
                OR project_internal_id = ?4
            )
            AND config_target = ?5
            AND server_name = ?6",
        params![
            registration.host_kind,
            registration.intent,
            registration.host_scope,
            registration.project_internal_id,
            registration.config_target,
            registration.server_name
        ],
        |row| row.get(0),
    )
    .optional()
    .map_err(StoreError::from)
}

fn connection_target_is_compatible(
    existing: &AgentConnectionRecord,
    registration: &AgentConnectionWriteRegistration,
) -> bool {
    existing.host_kind == registration.host_kind
        && existing.intent == registration.intent
        && existing.host_scope == registration.host_scope
        && existing.project_internal_id == registration.project_internal_id
        && existing.server_name == registration.server_name
        && existing.config_target == registration.config_target
}

fn conflict(entity: &'static str, id: &str, detail: impl Into<String>) -> StoreError {
    StoreError::Conflict {
        entity,
        id: id.to_owned(),
        detail: detail.into(),
    }
}

fn connection_internal_id_for_target(
    host_kind: &str,
    intent: &str,
    host_scope: &str,
    project_internal_id: Option<&str>,
    config_target: &str,
    server_name: &str,
) -> String {
    stable_internal_id(
        "conn",
        &format!(
            "{host_kind}\n{intent}\n{host_scope}\n{}\n{config_target}\n{server_name}",
            project_internal_id.unwrap_or("")
        ),
    )
}

fn stable_internal_id(prefix: &str, input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let mut suffix = String::with_capacity(24);
    for byte in digest.iter().take(12) {
        suffix.push_str(&format!("{byte:02x}"));
    }
    format!("{prefix}_{suffix}")
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use volicord_test_support::TempRuntimeHome;
    use volicord_types::McpRuntimeSessionSource;

    use super::*;
    use crate::bootstrap::{
        initialize_runtime_home, project_record_for_execution, register_project,
        ProjectRegistration, ACTIVE_PROJECT_STATUS,
    };
    use crate::mutation::{with_test_runtime_home_setup, TestRuntimeHomeAdmission};
    use crate::operational_sessions::{
        mcp_runtime_session, start_mcp_runtime_session_for_test, McpRuntimeSessionRecord,
        McpRuntimeSessionStart,
    };
    use crate::sqlite::open_registry_database_for_test;

    const PROJECT_ID: &str = "project_a";
    const PRIOR_OTHER_PROJECT_ID: &str = "project_b";
    const TARGET_OTHER_PROJECT_ID: &str = "project_c";
    const TEST_POLICY_HASH: &str =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn agent_connection_registration_updates_and_lists() -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-register")?;

        let created = ensure_agent_connection(&fixture.context()?, connection("conn_a"))?;
        assert!(matches!(
            ensure_agent_connection(
                &fixture.context()?,
                AgentConnectionRegistration {
                    mode: CONNECTION_MODE_READ_ONLY.to_owned(),
                    ..connection("conn_a")
                },
            ),
            Err(StoreError::Conflict { .. })
        ));
        let updated = ensure_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                enabled: false,
                managed_fingerprint: "fingerprint-updated".to_owned(),
                metadata_json: r#"{"updated":true}"#.to_owned(),
                ..connection("conn_a")
            },
        )?;
        let read = agent_connection_record(fixture.runtime_home.path(), "conn_a")?
            .expect("connection should be readable");
        let listed = list_agent_connections(fixture.runtime_home.path())?;

        assert_eq!(created.connection_internal_id, "conn_a");
        assert_eq!(updated.mode, CONNECTION_MODE_WORKFLOW);
        assert!(!updated.enabled);
        assert_eq!(updated.managed_fingerprint, "fingerprint-updated");
        assert_eq!(read, updated);
        assert_eq!(listed, vec![updated.clone()]);
        assert_eq!(
            agent_connection_record_for_diagnostics(fixture.runtime_home.path(), "conn_a")?
                .expect("diagnostic connection")
                .integration_instance_id,
            updated.integration_instance_id
        );
        Ok(())
    }

    #[test]
    fn connection_instances_are_unique_and_stable_across_compatible_updates(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-instance-stability")?;
        let first = ensure_agent_connection(&fixture.context()?, connection("conn_first"))?;
        let second = ensure_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                config_target: "/tmp/volicord-test-second-config.toml".to_owned(),
                ..connection("conn_second")
            },
        )?;
        assert_ne!(
            first.integration_instance_id,
            second.integration_instance_id
        );
        assert!(ConnectionIntegrationInstanceId::parse(
            first.integration_instance_id.as_str().to_owned()
        )
        .is_ok());

        let initial_revision = connection_integration_revision(&first)?;
        let replay = ensure_agent_connection(&fixture.context()?, connection("conn_first"))?;
        assert_eq!(
            replay.integration_instance_id,
            first.integration_instance_id
        );
        assert_eq!(connection_integration_revision(&replay)?, initial_revision);

        let disabled = set_connection_enabled(&fixture.context()?, "conn_first", false)?;
        assert_eq!(
            disabled.integration_instance_id,
            first.integration_instance_id
        );
        assert_eq!(
            connection_integration_revision(&disabled)?,
            initial_revision
        );

        let verified = replace_agent_connection_verification_report_if_revision(
            &fixture.context()?,
            "conn_first",
            &initial_revision,
            Some(&verification_report()),
        )?;
        assert_eq!(
            verified.integration_instance_id,
            first.integration_instance_id
        );
        assert_eq!(
            connection_integration_revision(&verified)?,
            initial_revision
        );
        Ok(())
    }

    #[test]
    fn connection_instance_is_sql_immutable_and_malformed_storage_is_corrupt(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-instance-storage")?;
        let created = ensure_agent_connection(&fixture.context()?, connection("conn_instance"))?;
        let registry_path = registry_db_path(fixture.runtime_home.path());
        let conn = open_registry_database_for_test(&registry_path)?;
        let replacement = "connection_instance_11223344-5566-4abb-8cdd-eeff10203040";
        assert!(conn
            .execute(
                "UPDATE agent_connections
                    SET integration_instance_id = ?2
                  WHERE connection_internal_id = ?1",
                params!["conn_instance", replacement],
            )
            .is_err());
        assert_eq!(
            agent_connection_record(fixture.runtime_home.path(), "conn_instance")?
                .expect("connection after rejected overwrite")
                .integration_instance_id,
            created.integration_instance_id
        );

        let immutable_trigger_sql: String = conn.query_row(
            "SELECT sql FROM sqlite_master
              WHERE type = 'trigger'
                AND name = 'agent_connections_integration_instance_immutable'",
            [],
            |row| row.get(0),
        )?;
        conn.execute_batch(
            "DROP TRIGGER agent_connections_integration_instance_immutable;
             PRAGMA ignore_check_constraints = ON;",
        )?;
        assert_eq!(
            conn.execute(
                "UPDATE agent_connections
                    SET integration_instance_id = 'not-a-connection-instance'
                  WHERE connection_internal_id = 'conn_instance'",
                [],
            )?,
            1
        );
        conn.execute_batch("PRAGMA ignore_check_constraints = OFF;")?;
        conn.execute_batch(&immutable_trigger_sql)?;
        drop(conn);

        for error in [
            agent_connection_record(fixture.runtime_home.path(), "conn_instance")
                .expect_err("strict read must reject a malformed integration instance"),
            agent_connection_record_for_diagnostics(fixture.runtime_home.path(), "conn_instance")
                .expect_err("diagnostic read must retain typed instance validation"),
            list_agent_connections(fixture.runtime_home.path())
                .expect_err("strict list must reject a malformed integration instance"),
        ] {
            assert_connection_owner_value_corrupt(
                error,
                "conn_instance",
                "integration_instance_id",
            );
        }
        Ok(())
    }

    #[test]
    fn report_replacement_changes_only_the_report_and_row_timestamp() -> Result<(), Box<dyn Error>>
    {
        let fixture = registry_fixture("connection-report-only")?;
        let connection = ensure_agent_connection(&fixture.context()?, connection("conn_report"))?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_report".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        crate::guards::upsert_guard_installation(
            &fixture.context()?,
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_report",
                "conn_report",
                PROJECT_ID,
            ),
        )?;
        let registry_path = registry_db_path(fixture.runtime_home.path());
        open_registry_database_for_test(&registry_path)?.execute(
            "UPDATE agent_connections
                SET updated_at = '2000-01-01T00:00:00.000Z'
              WHERE connection_internal_id = 'conn_report'",
            [],
        )?;
        let before = agent_connection_record(fixture.runtime_home.path(), "conn_report")?
            .expect("connection before report replacement");
        let before_revision = connection_integration_revision(&before)?;
        let memberships = list_connection_projects(fixture.runtime_home.path(), "conn_report")?;
        let guard = crate::guards::guard_installation(fixture.runtime_home.path(), "guard_report")?
            .expect("Guard Installation before report replacement");

        let replaced = replace_agent_connection_verification_report_if_revision(
            &fixture.context()?,
            "conn_report",
            &before_revision,
            Some(&verification_report()),
        )?;

        assert_eq!(replaced.verification_report()?, Some(verification_report()));
        assert_ne!(replaced.updated_at, before.updated_at);
        let mut expected_replaced = before.clone();
        expected_replaced.verification_report_json = replaced.verification_report_json.clone();
        expected_replaced.updated_at = replaced.updated_at.clone();
        assert_eq!(replaced, expected_replaced);
        assert_eq!(replaced.managed_fingerprint, connection.managed_fingerprint);
        assert_eq!(
            replaced.integration_instance_id,
            before.integration_instance_id
        );
        assert_eq!(
            replaced.integration_generation,
            before.integration_generation
        );
        assert_eq!(replaced.mode, before.mode);
        assert_eq!(replaced.enabled, before.enabled);
        assert_eq!(replaced.metadata_json, before.metadata_json);
        assert_eq!(connection_integration_revision(&replaced)?, before_revision);
        assert_eq!(
            list_connection_projects(fixture.runtime_home.path(), "conn_report")?,
            memberships
        );
        assert_eq!(
            crate::guards::guard_installation(fixture.runtime_home.path(), "guard_report",)?
                .expect("Guard Installation after report replacement"),
            guard
        );
        Ok(())
    }

    #[test]
    fn missing_report_synthesizes_action_required_without_writing() -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-report-missing")?;
        let connection = ensure_agent_connection(&fixture.context()?, connection("conn_missing"))?;
        assert!(connection.verification_report_json.is_none());

        let projected = connection.effective_verification_report(test_timestamp())?;
        assert_eq!(
            projected.status(),
            volicord_types::ConnectionStatus::ActionRequired
        );
        assert_eq!(projected.checks()[0].id().as_str(), "verification_not_run");
        assert_eq!(
            projected.activation_plan().required_steps()[0]
                .id()
                .as_str(),
            "request_integration_verification"
        );
        assert_eq!(
            projected.activation_plan().optional_diagnostics()[0]
                .id()
                .as_str(),
            "run_optional_active_diagnostics"
        );
        assert!(
            agent_connection_record(fixture.runtime_home.path(), "conn_missing")?
                .expect("projection must not remove the connection")
                .verification_report_json
                .is_none()
        );
        Ok(())
    }

    #[test]
    fn applied_fingerprint_change_clears_the_prior_report_and_changes_revision(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-fingerprint-report-invalidation")?;
        let initial = ensure_agent_connection(&fixture.context()?, connection("conn_fingerprint"))?;
        let initial_revision = connection_integration_revision(&initial)?;
        let verified = replace_agent_connection_verification_report_if_revision(
            &fixture.context()?,
            "conn_fingerprint",
            &initial_revision,
            Some(&verification_report()),
        )?;
        assert!(verified.verification_report_json.is_some());

        let changed = ensure_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                managed_fingerprint: "fingerprint-next".to_owned(),
                ..connection("conn_fingerprint")
            },
        )?;
        let changed_revision = connection_integration_revision(&changed)?;
        assert_eq!(changed.managed_fingerprint, "fingerprint-next");
        assert!(changed.verification_report_json.is_none());
        assert_ne!(changed_revision, initial_revision);

        let reverified = replace_agent_connection_verification_report_if_revision(
            &fixture.context()?,
            "conn_fingerprint",
            &changed_revision,
            Some(&verification_report()),
        )?;
        let compatible_replay = ensure_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                managed_fingerprint: "fingerprint-next".to_owned(),
                ..connection("conn_fingerprint")
            },
        )?;
        assert_eq!(
            compatible_replay.verification_report_json,
            reverified.verification_report_json
        );
        assert_eq!(
            connection_integration_revision(&compatible_replay)?,
            changed_revision
        );
        Ok(())
    }

    #[test]
    fn damaged_stored_verification_report_is_raw_only_until_explicit_replacement(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-report-stored")?;
        ensure_agent_connection(&fixture.context()?, connection("conn_report"))?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_report".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        let registry_path = registry_db_path(fixture.runtime_home.path());
        let conn = open_registry_database_for_test(&registry_path)?;
        let expected_revision = connection_integration_revision(
            &agent_connection_record(fixture.runtime_home.path(), "conn_report")?
                .expect("connection before report corruption"),
        )?;

        let mut noncanonical = report_json(&verification_report())?;
        noncanonical.push(' ');
        for damaged in ["[", "[]", "null", noncanonical.as_str()] {
            conn.execute(
                "UPDATE agent_connections
                    SET verification_report_json = ?2
                  WHERE connection_internal_id = ?1",
                params!["conn_report", damaged],
            )?;
            assert_connection_owner_json_corrupt(
                agent_connection_record(fixture.runtime_home.path(), "conn_report")
                    .expect_err("strict record read must reject a damaged report"),
                "conn_report",
                "verification_report_json",
            );
            assert_connection_owner_json_corrupt(
                agent_connection_record_read_only(fixture.runtime_home.path(), "conn_report")
                    .expect_err("strict read-only record must reject a damaged report"),
                "conn_report",
                "verification_report_json",
            );
            assert_connection_owner_json_corrupt(
                list_agent_connections(fixture.runtime_home.path())
                    .expect_err("strict list must reject a damaged report"),
                "conn_report",
                "verification_report_json",
            );
            assert_connection_owner_json_corrupt(
                list_agent_connections_read_only(fixture.runtime_home.path())
                    .expect_err("strict read-only list must reject a damaged report"),
                "conn_report",
                "verification_report_json",
            );
            assert_connection_owner_json_corrupt(
                list_connection_projects(fixture.runtime_home.path(), "conn_report")
                    .expect_err("strict membership read must reject a damaged report"),
                "conn_report",
                "verification_report_json",
            );
            assert_connection_owner_json_corrupt(
                set_connection_enabled(&fixture.context()?, "conn_report", false)
                    .expect_err("mutation must reject a damaged report before effects"),
                "conn_report",
                "verification_report_json",
            );

            let diagnostic = agent_connection_record_for_diagnostics(
                fixture.runtime_home.path(),
                "conn_report",
            )?
            .expect("diagnostic read should preserve the connection");
            assert_eq!(
                diagnostic.verification_report_json.as_deref(),
                Some(damaged)
            );
            assert!(diagnostic.enabled);
            assert_eq!(
                list_agent_connections_for_diagnostics(fixture.runtime_home.path())?[0]
                    .verification_report_json
                    .as_deref(),
                Some(damaged)
            );
            assert_eq!(
                list_connection_projects_for_diagnostics(
                    fixture.runtime_home.path(),
                    "conn_report",
                )?
                .len(),
                1
            );

            replace_agent_connection_verification_report_if_revision(
                &fixture.context()?,
                "conn_report",
                &expected_revision,
                Some(&verification_report()),
            )?;
            let repaired = agent_connection_record(fixture.runtime_home.path(), "conn_report")?
                .expect("explicit replacement should repair the report");
            assert_eq!(
                repaired.verification_report_json,
                Some(report_json(&verification_report())?)
            );
            assert_eq!(repaired.managed_fingerprint, "fingerprint");
            assert_eq!(
                connection_integration_revision(&repaired)?,
                expected_revision
            );
        }
        Ok(())
    }

    #[test]
    fn damaged_stored_metadata_is_raw_only_and_blocks_mutation() -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-metadata-stored")?;
        ensure_agent_connection(&fixture.context()?, connection("conn_metadata"))?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_metadata".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        let registry_path = registry_db_path(fixture.runtime_home.path());
        let conn = open_registry_database_for_test(&registry_path)?;
        let expected_revision = connection_integration_revision(
            &agent_connection_record(fixture.runtime_home.path(), "conn_metadata")?
                .expect("connection before metadata corruption"),
        )?;

        for damaged in ["[", "[]", "null"] {
            conn.execute(
                "UPDATE agent_connections
                    SET metadata_json = ?2
                  WHERE connection_internal_id = ?1",
                params!["conn_metadata", damaged],
            )?;
            for error in [
                agent_connection_record(fixture.runtime_home.path(), "conn_metadata")
                    .expect_err("strict record read must reject damaged metadata"),
                agent_connection_record_read_only(fixture.runtime_home.path(), "conn_metadata")
                    .expect_err("strict read-only record must reject damaged metadata"),
                list_agent_connections(fixture.runtime_home.path())
                    .expect_err("strict list must reject damaged metadata"),
                list_agent_connections_read_only(fixture.runtime_home.path())
                    .expect_err("strict read-only list must reject damaged metadata"),
                set_connection_enabled(&fixture.context()?, "conn_metadata", false)
                    .expect_err("mutation must reject damaged metadata before effects"),
                replace_agent_connection_verification_report_if_revision(
                    &fixture.context()?,
                    "conn_metadata",
                    &expected_revision,
                    Some(&verification_report()),
                )
                .expect_err("verification replacement cannot repair unrelated metadata"),
            ] {
                assert_connection_owner_json_corrupt(error, "conn_metadata", "metadata_json");
            }

            let diagnostic = agent_connection_record_for_diagnostics(
                fixture.runtime_home.path(),
                "conn_metadata",
            )?
            .expect("diagnostic read should preserve the connection");
            assert_eq!(diagnostic.metadata_json, damaged);
            assert!(diagnostic.enabled);
            assert_eq!(
                list_agent_connections_for_diagnostics(fixture.runtime_home.path())?[0]
                    .metadata_json,
                damaged
            );
            assert_eq!(
                list_connection_projects_for_diagnostics(
                    fixture.runtime_home.path(),
                    "conn_metadata",
                )?
                .len(),
                1
            );

            conn.execute(
                "UPDATE agent_connections
                    SET metadata_json = '{}'
                  WHERE connection_internal_id = ?1",
                ["conn_metadata"],
            )?;
        }
        Ok(())
    }

    fn assert_connection_owner_json_corrupt(
        error: StoreError,
        connection_internal_id: &str,
        logical_column: &'static str,
    ) {
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateJson {
                database_kind: "registry",
                table: "agent_connections",
                ref record_ref,
                logical_column: actual_column,
            } if record_ref == connection_internal_id && actual_column == logical_column
        ));
    }

    fn assert_connection_owner_value_corrupt(
        error: StoreError,
        connection_internal_id: &str,
        logical_column: &'static str,
    ) {
        assert!(
            matches!(
                &error,
                StoreError::CorruptOwnerStateValue {
                    database_kind: "registry",
                    table: "agent_connections",
                    record_ref,
                    logical_column: actual_column,
                } if record_ref == connection_internal_id && *actual_column == logical_column
            ),
            "{error:?}"
        );
    }

    #[test]
    fn agent_connection_rejects_conflicting_target() -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-conflict")?;
        ensure_agent_connection(&fixture.context()?, connection("conn_a"))?;

        let error = ensure_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                connection_internal_id: "conn_b".to_owned(),
                ..connection("conn_a")
            },
        )
        .expect_err("duplicate target should be rejected");

        assert!(matches!(error, StoreError::Conflict { .. }));
        Ok(())
    }

    #[test]
    fn staged_registration_preserves_a_concurrently_enabled_target() -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-staged-upsert-race")?;
        let staged = ensure_staged_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                enabled: false,
                ..connection("conn_staged_race")
            },
        )?;
        assert!(!staged.enabled);
        set_connection_enabled(&fixture.context()?, "conn_staged_race", true)?;

        let refreshed = ensure_staged_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                enabled: false,
                managed_fingerprint: "refreshed-staging-plan".to_owned(),
                ..connection("conn_staged_race")
            },
        )?;

        assert!(refreshed.enabled);
        assert_eq!(refreshed.managed_fingerprint, "refreshed-staging-plan");
        Ok(())
    }

    #[test]
    fn generic_registration_rejects_store_owned_cleanup_metadata() -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-reserved-cleanup-metadata")?;
        let error = ensure_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                metadata_json: format!(
                    r#"{{"{PENDING_HOST_CLEANUP_METADATA_KEY}":{{"project_id":"{PROJECT_ID}","replacement_connection_id":"conn_next"}}}}"#
                ),
                ..connection("conn_forged")
            },
        )
        .expect_err("generic registration must not forge Store-owned cleanup state");

        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert!(agent_connection_record(fixture.runtime_home.path(), "conn_forged")?.is_none());
        Ok(())
    }

    #[test]
    fn connection_projects_gate_current_project_access() -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-projects")?;
        ensure_agent_connection(&fixture.context()?, connection("conn_project"))?;
        assert!(!is_agent_connection_project_allowed(
            fixture.runtime_home.path(),
            "conn_project",
            PROJECT_ID
        )?);

        let added = add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_project".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        let repeated = add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_project".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        let listed = list_connection_projects(fixture.runtime_home.path(), "conn_project")?;
        let access = agent_connection_project_access(
            fixture.runtime_home.path(),
            "conn_project",
            PROJECT_ID,
        )?
        .expect("connection should exist");

        assert_eq!(added.project_id, PROJECT_ID);
        assert_eq!(repeated.project_id, PROJECT_ID);
        assert_eq!(listed.len(), 1);
        assert!(access.connection_enabled);
        assert!(access.project_allowed);
        assert!(access.project.is_some());
        assert!(is_agent_connection_project_allowed(
            fixture.runtime_home.path(),
            "conn_project",
            PROJECT_ID
        )?);

        set_connection_enabled(&fixture.context()?, "conn_project", false)?;
        assert!(!is_agent_connection_project_allowed(
            fixture.runtime_home.path(),
            "conn_project",
            PROJECT_ID
        )?);

        let removal = remove_connection_project(&fixture.context()?, "conn_project", PROJECT_ID)?;
        assert_eq!(
            removal,
            ConnectionProjectRemovalOutcome {
                membership_removed: true,
                connection_removed: true,
                remaining_project_count: 0,
            }
        );
        assert!(agent_connection_record(fixture.runtime_home.path(), "conn_project")?.is_none());
        Ok(())
    }

    #[test]
    fn intentionally_disabled_membership_is_not_pending_host_cleanup() -> Result<(), Box<dyn Error>>
    {
        let fixture = registry_fixture("connection-disabled-not-cleanup")?;
        ensure_agent_connection(&fixture.context()?, connection("conn_disabled"))?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_disabled".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        set_connection_enabled(&fixture.context()?, "conn_disabled", false)?;
        ensure_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                config_target: "/tmp/conn-replacement-config.toml".to_owned(),
                ..connection("conn_replacement")
            },
        )?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_replacement".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;

        let error = complete_pending_host_cleanup(
            &fixture.context()?,
            PROJECT_ID,
            "conn_replacement",
            &["conn_disabled".to_owned()],
            |_| -> Result<(), StoreError> {
                panic!("unmarked disabled connection must not reach host cleanup")
            },
        )
        .expect_err("an explicit cleanup marker is required");

        assert!(matches!(
            error,
            PendingHostCleanupError::Store(StoreError::Conflict { .. })
        ));
        assert_eq!(
            list_connection_projects(fixture.runtime_home.path(), "conn_disabled")?.len(),
            1
        );
        Ok(())
    }

    #[test]
    fn staged_connection_activation_retires_superseded_binding_atomically(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-staged-activation")?;
        register_project(
            &fixture.context()?,
            ProjectRegistration {
                project_id: PRIOR_OTHER_PROJECT_ID.to_owned(),
                repo_root: fixture
                    .runtime_home
                    .create_product_repo("prior-other-repo")?,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        register_project(
            &fixture.context()?,
            ProjectRegistration {
                project_id: TARGET_OTHER_PROJECT_ID.to_owned(),
                repo_root: fixture
                    .runtime_home
                    .create_product_repo("target-other-repo")?,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        ensure_agent_connection(&fixture.context()?, connection("conn_prior"))?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PRIOR_OTHER_PROJECT_ID.to_owned(),
            },
        )?;
        for (guard_id, project_id) in [
            ("guard_prior_selected", PROJECT_ID),
            ("guard_prior_retained", PRIOR_OTHER_PROJECT_ID),
        ] {
            crate::guards::upsert_guard_installation(
                &fixture.context()?,
                guard_installation_upsert(
                    fixture.runtime_home.path(),
                    guard_id,
                    "conn_prior",
                    project_id,
                ),
            )?;
        }
        let prior_runtime = start_test_runtime_session(
            &fixture,
            "conn_prior",
            McpRuntimeSessionSource::ManagedHost,
            46,
        )?;
        let mut prior_project_sessions = Vec::new();
        for (project_id, guard_id, host_session_id) in [
            (PROJECT_ID, "guard_prior_selected", "host_prior_selected"),
            (
                PRIOR_OTHER_PROJECT_ID,
                "guard_prior_retained",
                "host_prior_retained",
            ),
        ] {
            prior_project_sessions.push(upsert_test_agent_session(
                &fixture,
                &prior_runtime.runtime_session_id,
                "conn_prior",
                project_id,
                guard_id,
                host_session_id,
                "2026-07-19T00:00:01Z",
            )?);
        }
        ensure_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                connection_internal_id: "conn_staged".to_owned(),
                config_target: "/tmp/volicord-test-staged.toml".to_owned(),
                ..connection("conn_staged")
            },
        )?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_staged".to_owned(),
                project_id: TARGET_OTHER_PROJECT_ID.to_owned(),
            },
        )?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_staged".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        let invalid = activate_staged_connection(
            &fixture.context()?,
            "conn_staged",
            PROJECT_ID,
            &[SupersededConnectionProject {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            }],
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_staged",
                "conn_staged",
                PROJECT_ID,
            ),
        )
        .expect_err("an active target membership must not bypass staged activation");
        assert!(matches!(invalid, StoreError::InvalidInput { .. }));
        assert!(
            agent_connection_record(fixture.runtime_home.path(), "conn_prior")?
                .expect("prior connection")
                .enabled
        );
        assert_eq!(
            list_connection_projects(fixture.runtime_home.path(), "conn_prior")?.len(),
            2
        );
        crate::guards::upsert_guard_installation(
            &fixture.context()?,
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_staged",
                "conn_staged",
                PROJECT_ID,
            ),
        )?;
        delete_connection_project_membership_for_test(
            fixture.runtime_home.path(),
            "conn_staged",
            PROJECT_ID,
        )?;
        let conflict = activate_staged_connection(
            &fixture.context()?,
            "conn_staged",
            PROJECT_ID,
            &[SupersededConnectionProject {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            }],
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_conflicting",
                "conn_staged",
                PROJECT_ID,
            ),
        )
        .expect_err("a guard scope conflict must roll back the registry transition");
        assert!(matches!(conflict, StoreError::Conflict { .. }));
        assert_eq!(
            list_connection_projects(fixture.runtime_home.path(), "conn_prior")?.len(),
            2
        );
        assert_eq!(
            list_connection_projects(fixture.runtime_home.path(), "conn_staged")?.len(),
            1
        );
        assert!(crate::guards::guard_installation(
            fixture.runtime_home.path(),
            "guard_conflicting"
        )?
        .is_none());
        ensure_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                connection_internal_id: "conn_competing".to_owned(),
                config_target: "/tmp/volicord-test-competing.toml".to_owned(),
                ..connection("conn_competing")
            },
        )?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_competing".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        let stale_inventory = activate_staged_connection(
            &fixture.context()?,
            "conn_staged",
            PROJECT_ID,
            &[SupersededConnectionProject {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            }],
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_staged",
                "conn_staged",
                PROJECT_ID,
            ),
        )
        .expect_err("a competing project binding must invalidate the staged inventory");
        assert!(matches!(stale_inventory, StoreError::Conflict { .. }));
        set_connection_enabled(&fixture.context()?, "conn_competing", false)?;

        let (activated, installation, disabled_superseded) = activate_staged_connection(
            &fixture.context()?,
            "conn_staged",
            PROJECT_ID,
            &[SupersededConnectionProject {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            }],
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_staged",
                "conn_staged",
                PROJECT_ID,
            ),
        )?;

        assert!(activated.enabled);
        assert!(disabled_superseded.is_empty());
        assert_eq!(installation.connection_internal_id, "conn_staged");
        assert_eq!(installation.project_id, PROJECT_ID);
        let prior = agent_connection_record(fixture.runtime_home.path(), "conn_prior")?
            .expect("prior connection remains as history");
        assert!(prior.enabled);
        let prior_projects = list_connection_projects(fixture.runtime_home.path(), "conn_prior")?;
        assert_eq!(prior_projects.len(), 1);
        assert_eq!(prior_projects[0].project_id, PRIOR_OTHER_PROJECT_ID);
        assert!(mcp_runtime_session(
            fixture.runtime_home.path(),
            &prior_runtime.runtime_session_id
        )?
        .is_some());
        for session in prior_project_sessions {
            assert!(crate::guards::agent_session(
                fixture.runtime_home.path(),
                if session.host_session_id == "host_prior_selected" {
                    PROJECT_ID
                } else {
                    PRIOR_OTHER_PROJECT_ID
                },
                &session.session_id,
            )?
            .is_some());
        }
        assert_eq!(
            registry_connection_project_row_count(
                &fixture,
                "mcp_runtime_project_session_bindings",
                "conn_prior",
                PROJECT_ID,
            )?,
            0
        );
        assert_eq!(
            registry_connection_project_row_count(
                &fixture,
                "mcp_runtime_project_session_bindings",
                "conn_prior",
                PRIOR_OTHER_PROJECT_ID,
            )?,
            1
        );
        assert!(crate::guards::guard_installation(
            fixture.runtime_home.path(),
            "guard_prior_selected"
        )?
        .is_none());
        assert!(crate::guards::guard_installation(
            fixture.runtime_home.path(),
            "guard_prior_retained"
        )?
        .is_some());
        let staged_projects = list_connection_projects(fixture.runtime_home.path(), "conn_staged")?;
        assert_eq!(staged_projects.len(), 2);
        assert!(staged_projects
            .iter()
            .any(|membership| membership.project_id == PROJECT_ID));
        assert!(staged_projects
            .iter()
            .any(|membership| membership.project_id == TARGET_OTHER_PROJECT_ID));
        assert_eq!(
            list_connection_projects(fixture.runtime_home.path(), "conn_competing")?.len(),
            1,
            "an unrelated disabled alternative must not enter migration inventory"
        );
        Ok(())
    }

    #[test]
    fn staged_connection_activation_rolls_back_flags_on_late_guard_conflict(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-staged-activation-rollback")?;
        ensure_agent_connection(&fixture.context()?, connection("conn_prior"))?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        crate::guards::upsert_guard_installation(
            &fixture.context()?,
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_prior_pending",
                "conn_prior",
                PROJECT_ID,
            ),
        )?;
        let prior_runtime = start_test_runtime_session(
            &fixture,
            "conn_prior",
            McpRuntimeSessionSource::ManagedHost,
            47,
        )?;
        let prior_agent_session = upsert_test_agent_session(
            &fixture,
            &prior_runtime.runtime_session_id,
            "conn_prior",
            PROJECT_ID,
            "guard_prior_pending",
            "host_prior_pending",
            "2026-07-19T00:00:01Z",
        )?;
        let staged_registration = AgentConnectionRegistration {
            connection_internal_id: "conn_staged".to_owned(),
            config_target: "/tmp/volicord-test-staged-disabled.toml".to_owned(),
            enabled: false,
            ..connection("conn_staged")
        };
        let staged =
            ensure_staged_agent_connection(&fixture.context()?, staged_registration.clone())?;
        let staged_revision = connection_integration_revision(&staged)?;
        let staged_replay =
            ensure_staged_agent_connection(&fixture.context()?, staged_registration)?;
        assert_eq!(
            staged_replay.integration_instance_id,
            staged.integration_instance_id
        );
        assert_eq!(
            connection_integration_revision(&staged_replay)?,
            staged_revision
        );
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_staged".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        crate::guards::upsert_guard_installation(
            &fixture.context()?,
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_existing",
                "conn_staged",
                PROJECT_ID,
            ),
        )?;
        delete_connection_project_membership_for_test(
            fixture.runtime_home.path(),
            "conn_staged",
            PROJECT_ID,
        )?;

        let conflict = activate_staged_connection(
            &fixture.context()?,
            "conn_staged",
            PROJECT_ID,
            &[SupersededConnectionProject {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            }],
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_conflicting",
                "conn_staged",
                PROJECT_ID,
            ),
        )
        .expect_err("the late guard conflict must roll back connection flags and memberships");

        assert!(matches!(conflict, StoreError::Conflict { .. }));
        assert!(
            agent_connection_record(fixture.runtime_home.path(), "conn_prior")?
                .expect("prior connection")
                .enabled
        );
        assert_eq!(
            list_connection_projects(fixture.runtime_home.path(), "conn_prior")?.len(),
            1
        );
        assert!(
            !agent_connection_record(fixture.runtime_home.path(), "conn_staged")?
                .expect("staged connection")
                .enabled
        );
        assert!(list_connection_projects(fixture.runtime_home.path(), "conn_staged")?.is_empty());
        assert!(
            crate::guards::guard_installation(fixture.runtime_home.path(), "guard_existing")?
                .is_some()
        );
        assert!(crate::guards::guard_installation(
            fixture.runtime_home.path(),
            "guard_conflicting"
        )?
        .is_none());
        let (activated_connection, _, pending_host_cleanup) = activate_staged_connection(
            &fixture.context()?,
            "conn_staged",
            PROJECT_ID,
            &[SupersededConnectionProject {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            }],
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_existing",
                "conn_staged",
                PROJECT_ID,
            ),
        )?;
        assert_eq!(pending_host_cleanup, vec!["conn_prior"]);
        assert_eq!(
            activated_connection.integration_instance_id,
            staged.integration_instance_id
        );
        assert_eq!(
            connection_integration_revision(&activated_connection)?,
            staged_revision
        );
        assert!(
            !agent_connection_record(fixture.runtime_home.path(), "conn_prior")?
                .expect("prior connection")
                .enabled
        );
        assert_eq!(
            list_connection_projects(fixture.runtime_home.path(), "conn_prior")?.len(),
            1
        );
        assert_eq!(
            registry_connection_project_row_count(
                &fixture,
                "mcp_runtime_project_session_bindings",
                "conn_prior",
                PROJECT_ID,
            )?,
            1
        );
        assert!(crate::guards::guard_installation(
            fixture.runtime_home.path(),
            "guard_prior_pending"
        )?
        .is_some());
        assert!(
            agent_connection_record(fixture.runtime_home.path(), "conn_staged")?
                .expect("activated connection")
                .enabled
        );
        assert_eq!(
            list_connection_projects(fixture.runtime_home.path(), "conn_staged")?.len(),
            1
        );
        let (resumed_connection, resumed_state) = staged_connection_migration_state(
            &fixture.context()?,
            "conn_staged",
            PROJECT_ID,
            &[SupersededConnectionProject {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            }],
        )?;
        assert!(resumed_connection.enabled);
        assert_eq!(
            resumed_connection.integration_instance_id,
            staged.integration_instance_id
        );
        assert_eq!(
            connection_integration_revision(&resumed_connection)?,
            staged_revision
        );
        assert_eq!(
            resumed_state,
            StagedConnectionMigrationState::CleanupResume {
                pending_connection_ids: vec!["conn_prior".to_owned()]
            },
            "a stale second migration snapshot must classify the completed switch as cleanup resume"
        );

        register_project(
            &fixture.context()?,
            ProjectRegistration {
                project_id: TARGET_OTHER_PROJECT_ID.to_owned(),
                repo_root: fixture
                    .runtime_home
                    .create_product_repo("marked-target-other-repo")?,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        let marked_target = activate_staged_connection(
            &fixture.context()?,
            "conn_prior",
            TARGET_OTHER_PROJECT_ID,
            &[],
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_marked_target",
                "conn_prior",
                TARGET_OTHER_PROJECT_ID,
            ),
        )
        .expect_err("a pending-cleanup row must not be activated for another project");
        assert!(matches!(marked_target, StoreError::Conflict { .. }));

        let generic_update = ensure_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                enabled: false,
                ..connection("conn_prior")
            },
        )
        .expect_err("generic ensure must not overwrite a cleanup marker");
        assert!(matches!(generic_update, StoreError::Conflict { .. }));
        let generic_enable = set_connection_enabled(&fixture.context()?, "conn_prior", true)
            .expect_err("generic enable must not bypass cleanup recovery");
        assert!(matches!(generic_enable, StoreError::Conflict { .. }));
        let generic_add = add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )
        .expect_err("generic membership addition must not mutate cleanup inventory");
        assert!(matches!(generic_add, StoreError::Conflict { .. }));
        let generic_remove =
            remove_connection_project(&fixture.context()?, "conn_prior", PROJECT_ID)
                .expect_err("generic membership removal must not orphan cleanup inventory");
        assert!(matches!(generic_remove, StoreError::Conflict { .. }));

        let cleanup_failure = complete_pending_host_cleanup(
            &fixture.context()?,
            PROJECT_ID,
            "conn_staged",
            &pending_host_cleanup,
            |connection_ids| {
                assert_eq!(connection_ids, ["conn_prior"]);
                Err(StoreError::Conflict {
                    entity: "host_configuration",
                    id: "conn_prior".to_owned(),
                    detail: "fixture cleanup failure".to_owned(),
                })
            },
        )
        .expect_err("failed external cleanup must retain durable cleanup inventory");
        assert!(matches!(
            cleanup_failure,
            PendingHostCleanupError::Host(StoreError::Conflict { .. })
        ));
        assert_eq!(
            list_connection_projects(fixture.runtime_home.path(), "conn_prior")?.len(),
            1
        );
        assert_eq!(
            registry_connection_project_row_count(
                &fixture,
                "mcp_runtime_project_session_bindings",
                "conn_prior",
                PROJECT_ID,
            )?,
            1
        );
        assert!(crate::guards::guard_installation(
            fixture.runtime_home.path(),
            "guard_prior_pending"
        )?
        .is_some());
        let revalidation_failure = complete_pending_host_cleanup(
            &fixture.context()?,
            PROJECT_ID,
            "conn_staged",
            &pending_host_cleanup,
            |_| {
                let registry_path = registry_db_path(fixture.runtime_home.path());
                let conn = open_registry_database_for_test(&registry_path)?;
                let prior = require_agent_connection(&conn, "conn_prior")?;
                let rebased_metadata = metadata_with_pending_host_cleanup(
                    &prior.metadata_json,
                    PROJECT_ID,
                    "conn_newer",
                )?;
                conn.execute(
                    "UPDATE agent_connections SET metadata_json = ?2 WHERE connection_internal_id = ?1",
                    params!["conn_prior", rebased_metadata],
                )?;
                Ok::<(), StoreError>(())
            },
        )
        .expect_err("final cleanup must revalidate a marker changed during host cleanup");
        assert!(matches!(
            revalidation_failure,
            PendingHostCleanupError::Store(StoreError::Conflict { .. })
        ));
        assert_eq!(
            list_connection_projects(fixture.runtime_home.path(), "conn_prior")?.len(),
            1
        );
        assert_eq!(
            registry_connection_project_row_count(
                &fixture,
                "mcp_runtime_project_session_bindings",
                "conn_prior",
                PROJECT_ID,
            )?,
            1
        );
        assert!(crate::guards::guard_installation(
            fixture.runtime_home.path(),
            "guard_prior_pending"
        )?
        .is_some());
        let registry_path = registry_db_path(fixture.runtime_home.path());
        let registry = open_registry_database_for_test(&registry_path)?;
        let prior = require_agent_connection(&registry, "conn_prior")?;
        let restored_metadata =
            metadata_with_pending_host_cleanup(&prior.metadata_json, PROJECT_ID, "conn_staged")?;
        registry.execute(
            "UPDATE agent_connections SET metadata_json = ?2 WHERE connection_internal_id = ?1",
            params!["conn_prior", restored_metadata],
        )?;
        drop(registry);
        complete_pending_host_cleanup(
            &fixture.context()?,
            PROJECT_ID,
            "conn_staged",
            &pending_host_cleanup,
            |_| {
                let transition = mode_transition_input(
                    fixture.runtime_home.path(),
                    "conn_staged",
                    CONNECTION_MODE_READ_ONLY,
                )?;
                transition_connection_mode(&fixture.context()?, transition)?;
                Ok::<(), StoreError>(())
            },
        )?;
        assert!(list_connection_projects(fixture.runtime_home.path(), "conn_prior")?.is_empty());
        assert_eq!(
            registry_connection_project_row_count(
                &fixture,
                "mcp_runtime_project_session_bindings",
                "conn_prior",
                PROJECT_ID,
            )?,
            0
        );
        assert!(crate::guards::guard_installation(
            fixture.runtime_home.path(),
            "guard_prior_pending"
        )?
        .is_none());
        assert!(mcp_runtime_session(
            fixture.runtime_home.path(),
            &prior_runtime.runtime_session_id
        )?
        .is_some());
        assert!(crate::guards::agent_session(
            fixture.runtime_home.path(),
            PROJECT_ID,
            &prior_agent_session.session_id,
        )?
        .is_some());
        let retired = agent_connection_record(fixture.runtime_home.path(), "conn_prior")?
            .expect("disabled zero-membership connection remains as history");
        assert!(!retired.enabled);
        assert!(!connection_metadata_contains_pending_host_cleanup_key(
            &retired.metadata_json
        ));
        let after_cleanup = agent_connection_record(fixture.runtime_home.path(), "conn_staged")?
            .expect("replacement connection after cleanup");
        assert_eq!(
            after_cleanup.integration_instance_id,
            staged.integration_instance_id
        );
        Ok(())
    }

    #[test]
    fn staged_activation_preserves_a_malformed_pending_cleanup_marker() -> Result<(), Box<dyn Error>>
    {
        assert_staged_activation_rejects_non_rebasable_marker(
            "connection-cleanup-marker-malformed",
            r#"{"preserved":true,"pending_host_cleanup":{"project_id":"project_a","replacement_connection_id":"conn_old","unexpected":true}}"#,
        )
    }

    #[test]
    fn staged_activation_preserves_a_foreign_project_pending_cleanup_marker(
    ) -> Result<(), Box<dyn Error>> {
        assert_staged_activation_rejects_non_rebasable_marker(
            "connection-cleanup-marker-foreign-project",
            r#"{"preserved":true,"pending_host_cleanup":{"project_id":"project_b","replacement_connection_id":"conn_foreign"}}"#,
        )
    }

    #[test]
    fn staged_activation_rebases_older_pending_cleanup_to_the_new_replacement(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-cleanup-marker-chain")?;
        ensure_agent_connection(&fixture.context()?, connection("conn_prior"))?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        crate::guards::upsert_guard_installation(
            &fixture.context()?,
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_prior_chain",
                "conn_prior",
                PROJECT_ID,
            ),
        )?;
        let prior_runtime = start_test_runtime_session(
            &fixture,
            "conn_prior",
            McpRuntimeSessionSource::ManagedHost,
            48,
        )?;
        let prior_agent_session = upsert_test_agent_session(
            &fixture,
            &prior_runtime.runtime_session_id,
            "conn_prior",
            PROJECT_ID,
            "guard_prior_chain",
            "host_prior_chain",
            "2026-07-19T00:00:01Z",
        )?;
        for (connection_id, target) in [
            ("conn_middle", "/tmp/volicord-test-middle.toml"),
            ("conn_next", "/tmp/volicord-test-next.toml"),
        ] {
            ensure_agent_connection(
                &fixture.context()?,
                AgentConnectionRegistration {
                    connection_internal_id: connection_id.to_owned(),
                    config_target: target.to_owned(),
                    enabled: false,
                    ..connection(connection_id)
                },
            )?;
        }
        let (_, _, first_pending) = activate_staged_connection(
            &fixture.context()?,
            "conn_middle",
            PROJECT_ID,
            &[SupersededConnectionProject {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            }],
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_middle",
                "conn_middle",
                PROJECT_ID,
            ),
        )?;
        assert_eq!(first_pending, ["conn_prior"]);
        let middle_runtime = start_test_runtime_session(
            &fixture,
            "conn_middle",
            McpRuntimeSessionSource::ManagedHost,
            49,
        )?;
        let middle_agent_session = upsert_test_agent_session(
            &fixture,
            &middle_runtime.runtime_session_id,
            "conn_middle",
            PROJECT_ID,
            "guard_middle",
            "host_middle_chain",
            "2026-07-19T00:00:02Z",
        )?;

        let (_, _, rebased_pending) = activate_staged_connection(
            &fixture.context()?,
            "conn_next",
            PROJECT_ID,
            &[
                SupersededConnectionProject {
                    connection_internal_id: "conn_prior".to_owned(),
                    project_id: PROJECT_ID.to_owned(),
                },
                SupersededConnectionProject {
                    connection_internal_id: "conn_middle".to_owned(),
                    project_id: PROJECT_ID.to_owned(),
                },
            ],
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_next",
                "conn_next",
                PROJECT_ID,
            ),
        )?;

        assert_eq!(
            rebased_pending.into_iter().collect::<BTreeSet<_>>(),
            BTreeSet::from(["conn_middle".to_owned(), "conn_prior".to_owned()])
        );
        for connection_id in ["conn_prior", "conn_middle"] {
            let connection = agent_connection_record(fixture.runtime_home.path(), connection_id)?
                .expect("superseded connection");
            assert!(!connection.enabled);
            assert!(connection_metadata_has_pending_host_cleanup(
                &connection.metadata_json,
                PROJECT_ID,
                "conn_next"
            ));
        }
        complete_pending_host_cleanup(
            &fixture.context()?,
            PROJECT_ID,
            "conn_next",
            &["conn_prior".to_owned(), "conn_middle".to_owned()],
            |_| Ok::<(), StoreError>(()),
        )?;
        for connection_id in ["conn_prior", "conn_middle"] {
            assert!(
                list_connection_projects(fixture.runtime_home.path(), connection_id)?.is_empty()
            );
            assert_eq!(
                registry_connection_project_row_count(
                    &fixture,
                    "mcp_runtime_project_session_bindings",
                    connection_id,
                    PROJECT_ID,
                )?,
                0
            );
        }
        assert!(crate::guards::guard_installation(
            fixture.runtime_home.path(),
            "guard_prior_chain"
        )?
        .is_none());
        assert!(
            crate::guards::guard_installation(fixture.runtime_home.path(), "guard_middle")?
                .is_none()
        );
        assert!(mcp_runtime_session(
            fixture.runtime_home.path(),
            &prior_runtime.runtime_session_id
        )?
        .is_some());
        assert!(mcp_runtime_session(
            fixture.runtime_home.path(),
            &middle_runtime.runtime_session_id
        )?
        .is_some());
        for session in [prior_agent_session, middle_agent_session] {
            assert!(crate::guards::agent_session(
                fixture.runtime_home.path(),
                PROJECT_ID,
                &session.session_id,
            )?
            .is_some());
        }
        Ok(())
    }

    #[test]
    fn last_membership_removal_cleans_initialized_registry_state() -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-remove-initialized")?;
        ensure_agent_connection(&fixture.context()?, connection("conn_initialized"))?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_initialized".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        crate::guards::upsert_guard_installation(
            &fixture.context()?,
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_initialized",
                "conn_initialized",
                PROJECT_ID,
            ),
        )?;
        let cli_session = start_test_runtime_session(
            &fixture,
            "conn_initialized",
            McpRuntimeSessionSource::CliPreflight,
            41,
        )?;

        let outcome =
            remove_connection_project(&fixture.context()?, "conn_initialized", PROJECT_ID)?;

        assert_eq!(
            outcome,
            ConnectionProjectRemovalOutcome {
                membership_removed: true,
                connection_removed: true,
                remaining_project_count: 0,
            }
        );
        assert!(
            agent_connection_record(fixture.runtime_home.path(), "conn_initialized")?.is_none()
        );
        assert!(crate::guards::guard_installation(
            fixture.runtime_home.path(),
            "guard_initialized"
        )?
        .is_none());
        assert!(
            mcp_runtime_session(fixture.runtime_home.path(), &cli_session.runtime_session_id)?
                .is_none()
        );
        assert_eq!(
            registry_connection_row_count(&fixture, "connection_projects", "conn_initialized")?,
            0
        );
        Ok(())
    }

    #[test]
    fn physical_recreation_reuses_connection_id_with_a_new_instance_and_revision(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-instance-recreation")?;
        let first = ensure_agent_connection_for_target(&fixture.context()?, natural_connection())?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: first.connection_internal_id.clone(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        let first_revision = connection_integration_revision(&first)?;
        let outcome = remove_connection_project(
            &fixture.context()?,
            &first.connection_internal_id,
            PROJECT_ID,
        )?;
        assert!(outcome.connection_removed);
        assert!(agent_connection_record(
            fixture.runtime_home.path(),
            &first.connection_internal_id
        )?
        .is_none());

        let recreated =
            ensure_agent_connection_for_target(&fixture.context()?, natural_connection())?;
        assert_eq!(
            recreated.connection_internal_id,
            first.connection_internal_id
        );
        assert_ne!(
            recreated.integration_instance_id,
            first.integration_instance_id
        );
        assert_eq!(recreated.integration_generation, 0);
        assert_ne!(connection_integration_revision(&recreated)?, first_revision);
        Ok(())
    }

    #[test]
    fn last_membership_removal_cleans_bound_runtime_state_and_retains_unrelated_rows(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-remove-bound-runtime")?;
        register_additional_project(
            &fixture,
            PRIOR_OTHER_PROJECT_ID,
            "remove-bound-runtime-other-repo",
        )?;
        ensure_agent_connection(&fixture.context()?, connection("conn_selected"))?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_selected".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        crate::guards::upsert_guard_installation(
            &fixture.context()?,
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_selected",
                "conn_selected",
                PROJECT_ID,
            ),
        )?;
        let selected_session = start_test_runtime_session(
            &fixture,
            "conn_selected",
            McpRuntimeSessionSource::ManagedHost,
            42,
        )?;
        upsert_test_agent_session(
            &fixture,
            &selected_session.runtime_session_id,
            "conn_selected",
            PROJECT_ID,
            "guard_selected",
            "host_selected",
            "2026-07-19T00:00:01Z",
        )?;

        ensure_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                config_target: "/tmp/volicord-test-unrelated.toml".to_owned(),
                ..connection("conn_unrelated")
            },
        )?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_unrelated".to_owned(),
                project_id: PRIOR_OTHER_PROJECT_ID.to_owned(),
            },
        )?;
        crate::guards::upsert_guard_installation(
            &fixture.context()?,
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_unrelated",
                "conn_unrelated",
                PRIOR_OTHER_PROJECT_ID,
            ),
        )?;
        let unrelated_session = start_test_runtime_session(
            &fixture,
            "conn_unrelated",
            McpRuntimeSessionSource::ManagedHost,
            43,
        )?;
        upsert_test_agent_session(
            &fixture,
            &unrelated_session.runtime_session_id,
            "conn_unrelated",
            PRIOR_OTHER_PROJECT_ID,
            "guard_unrelated",
            "host_unrelated",
            "2026-07-19T00:00:01Z",
        )?;

        let outcome = remove_connection_project(&fixture.context()?, "conn_selected", PROJECT_ID)?;

        assert!(outcome.connection_removed);
        assert!(mcp_runtime_session(
            fixture.runtime_home.path(),
            &selected_session.runtime_session_id
        )?
        .is_none());
        assert_eq!(
            registry_connection_row_count(
                &fixture,
                "mcp_runtime_project_session_bindings",
                "conn_selected"
            )?,
            0
        );
        assert_eq!(
            registry_connection_row_count(&fixture, "guard_installations", "conn_selected")?,
            0
        );
        assert!(agent_connection_record(fixture.runtime_home.path(), "conn_unrelated")?.is_some());
        assert!(mcp_runtime_session(
            fixture.runtime_home.path(),
            &unrelated_session.runtime_session_id
        )?
        .is_some());
        assert_eq!(
            registry_connection_row_count(&fixture, "connection_projects", "conn_unrelated")?,
            1
        );
        assert_eq!(
            registry_connection_row_count(
                &fixture,
                "mcp_runtime_project_session_bindings",
                "conn_unrelated"
            )?,
            1
        );
        assert_eq!(
            registry_connection_row_count(&fixture, "guard_installations", "conn_unrelated")?,
            1
        );
        assert!(
            project_record_for_execution(fixture.runtime_home.path(), PRIOR_OTHER_PROJECT_ID)?
                .is_some()
        );
        Ok(())
    }

    #[test]
    fn membership_only_removal_keeps_connection_wide_and_other_project_state(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-remove-one-membership")?;
        register_additional_project(
            &fixture,
            PRIOR_OTHER_PROJECT_ID,
            "remove-one-membership-other-repo",
        )?;
        ensure_agent_connection(&fixture.context()?, connection("conn_multi"))?;
        for project_id in [PROJECT_ID, PRIOR_OTHER_PROJECT_ID] {
            add_connection_project(
                &fixture.context()?,
                ConnectionProjectRegistration {
                    connection_internal_id: "conn_multi".to_owned(),
                    project_id: project_id.to_owned(),
                },
            )?;
        }
        for (guard_id, project_id) in [
            ("guard_multi_selected", PROJECT_ID),
            ("guard_multi_retained", PRIOR_OTHER_PROJECT_ID),
        ] {
            crate::guards::upsert_guard_installation(
                &fixture.context()?,
                guard_installation_upsert(
                    fixture.runtime_home.path(),
                    guard_id,
                    "conn_multi",
                    project_id,
                ),
            )?;
        }
        let runtime_session = start_test_runtime_session(
            &fixture,
            "conn_multi",
            McpRuntimeSessionSource::ManagedHost,
            44,
        )?;
        for (project_id, guard_id, host_session_id) in [
            (PROJECT_ID, "guard_multi_selected", "host_multi_selected"),
            (
                PRIOR_OTHER_PROJECT_ID,
                "guard_multi_retained",
                "host_multi_retained",
            ),
        ] {
            upsert_test_agent_session(
                &fixture,
                &runtime_session.runtime_session_id,
                "conn_multi",
                project_id,
                guard_id,
                host_session_id,
                "2026-07-19T00:00:01Z",
            )?;
        }

        let outcome = remove_connection_project(&fixture.context()?, "conn_multi", PROJECT_ID)?;

        assert_eq!(
            outcome,
            ConnectionProjectRemovalOutcome {
                membership_removed: true,
                connection_removed: false,
                remaining_project_count: 1,
            }
        );
        assert!(agent_connection_record(fixture.runtime_home.path(), "conn_multi")?.is_some());
        assert!(mcp_runtime_session(
            fixture.runtime_home.path(),
            &runtime_session.runtime_session_id
        )?
        .is_some());
        assert_eq!(
            registry_connection_row_count(&fixture, "connection_projects", "conn_multi")?,
            1
        );
        assert_eq!(
            registry_connection_row_count(
                &fixture,
                "mcp_runtime_project_session_bindings",
                "conn_multi"
            )?,
            1
        );
        assert_eq!(
            registry_connection_row_count(&fixture, "guard_installations", "conn_multi")?,
            1
        );
        assert!(crate::guards::guard_installation(
            fixture.runtime_home.path(),
            "guard_multi_selected"
        )?
        .is_none());
        assert!(crate::guards::guard_installation(
            fixture.runtime_home.path(),
            "guard_multi_retained"
        )?
        .is_some());
        let remaining = list_connection_projects(fixture.runtime_home.path(), "conn_multi")?;
        assert_eq!(remaining[0].project_id, PRIOR_OTHER_PROJECT_ID);
        Ok(())
    }

    #[test]
    fn pending_cleanup_conflict_preserves_all_removal_inputs() -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-remove-pending-conflict")?;
        ensure_agent_connection(&fixture.context()?, connection("conn_pending"))?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_pending".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        crate::guards::upsert_guard_installation(
            &fixture.context()?,
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_pending",
                "conn_pending",
                PROJECT_ID,
            ),
        )?;
        let runtime_session = start_test_runtime_session(
            &fixture,
            "conn_pending",
            McpRuntimeSessionSource::ManagedHost,
            45,
        )?;
        upsert_test_agent_session(
            &fixture,
            &runtime_session.runtime_session_id,
            "conn_pending",
            PROJECT_ID,
            "guard_pending",
            "host_pending",
            "2026-07-19T00:00:01Z",
        )?;
        let registry_path = registry_db_path(fixture.runtime_home.path());
        let conn = open_registry_database_for_test(&registry_path)?;
        conn.execute(
            "UPDATE agent_connections SET metadata_json = ?2 WHERE connection_internal_id = ?1",
            params![
                "conn_pending",
                r#"{"pending_host_cleanup":{"project_id":"project_b","replacement_connection_id":"conn_replacement"}}"#
            ],
        )?;
        drop(conn);

        let error = remove_connection_project(&fixture.context()?, "conn_pending", PROJECT_ID)
            .expect_err("pending host cleanup must block generic removal");

        assert!(matches!(error, StoreError::Conflict { .. }));
        assert_eq!(
            registry_connection_row_count(&fixture, "connection_projects", "conn_pending")?,
            1
        );
        assert_eq!(
            registry_connection_row_count(
                &fixture,
                "mcp_runtime_project_session_bindings",
                "conn_pending"
            )?,
            1
        );
        assert_eq!(
            registry_connection_row_count(&fixture, "guard_installations", "conn_pending")?,
            1
        );
        assert_eq!(
            registry_connection_row_count(&fixture, "mcp_runtime_sessions", "conn_pending")?,
            1
        );
        assert!(agent_connection_record_for_diagnostics(
            fixture.runtime_home.path(),
            "conn_pending"
        )?
        .is_some());
        Ok(())
    }

    #[test]
    fn missing_connection_project_removal_is_explicit_and_has_no_effect(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-remove-missing-membership")?;
        ensure_agent_connection(&fixture.context()?, connection("conn_blocked"))?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_blocked".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        register_additional_project(
            &fixture,
            PRIOR_OTHER_PROJECT_ID,
            "remove-missing-membership-other-repo",
        )?;

        let error =
            remove_connection_project(&fixture.context()?, "conn_blocked", PRIOR_OTHER_PROJECT_ID)
                .expect_err("a missing selected membership must not be reported as removed");
        assert!(matches!(
            error,
            StoreError::NotFound {
                entity: "connection_project",
                ..
            }
        ));
        assert!(agent_connection_record(fixture.runtime_home.path(), "conn_blocked")?.is_some());
        assert_eq!(
            list_connection_projects(fixture.runtime_home.path(), "conn_blocked")?.len(),
            1
        );
        Ok(())
    }

    #[test]
    fn mode_transition_rebinds_guard_manifest_and_no_op_preserves_state(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-mode-transition")?;
        let report = report_json(&verification_report())?;
        ensure_agent_connection(&fixture.context()?, connection("conn_mode"))?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_mode".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        crate::guards::upsert_guard_installation(
            &fixture.context()?,
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_mode",
                "conn_mode",
                PROJECT_ID,
            ),
        )?;
        let connection_before_report =
            agent_connection_record(fixture.runtime_home.path(), "conn_mode")?
                .expect("connection before report");
        replace_agent_connection_verification_report_if_revision(
            &fixture.context()?,
            "conn_mode",
            &connection_integration_revision(&connection_before_report)?,
            Some(&verification_report()),
        )?;

        let before_connection =
            agent_connection_record(fixture.runtime_home.path(), "conn_mode")?.expect("connection");
        let before_guard =
            crate::guards::guard_installation(fixture.runtime_home.path(), "guard_mode")?
                .expect("Guard Installation");
        let before_manifest = guard_manifest_from_json(&before_guard.manifest_json)?;
        let transition = mode_transition_input(
            fixture.runtime_home.path(),
            "conn_mode",
            CONNECTION_MODE_READ_ONLY,
        )?;
        let outcome = transition_connection_mode(&fixture.context()?, transition)?;

        assert_eq!(outcome.kind, ConnectionModeTransitionKind::Updated);
        assert_eq!(outcome.connection.mode, CONNECTION_MODE_READ_ONLY);
        assert_eq!(
            outcome.connection.integration_instance_id,
            before_connection.integration_instance_id
        );
        assert_eq!(
            outcome.connection.integration_generation,
            before_connection.integration_generation + 1
        );
        assert!(outcome.connection.verification_report_json.is_none());
        assert_ne!(
            outcome.previous_integration_revision,
            outcome.current_integration_revision
        );
        assert_eq!(
            outcome.rebound_guard_installation_ids,
            ["guard_mode".to_owned()]
        );
        let after_guard =
            crate::guards::guard_installation(fixture.runtime_home.path(), "guard_mode")?
                .expect("Guard Installation");
        let after_manifest = guard_manifest_from_json(&after_guard.manifest_json)?;
        let mut expected_manifest = before_manifest.clone();
        expected_manifest.integration_revision = outcome.current_integration_revision.clone();
        assert_eq!(after_manifest, expected_manifest);
        assert_eq!(before_manifest.policy_hash, after_manifest.policy_hash);
        assert_eq!(
            before_manifest.runtime_commands,
            after_manifest.runtime_commands
        );
        assert_eq!(before_manifest.managed_files, after_manifest.managed_files);
        assert_eq!(
            before_manifest.required_hook_phases,
            after_manifest.required_hook_phases
        );

        let stale_error = replace_agent_connection_verification_report_if_revision(
            &fixture.context()?,
            "conn_mode",
            &outcome.previous_integration_revision,
            Some(&verification_report()),
        )
        .expect_err("the prior revision must not accept a stale verification report");
        assert!(matches!(stale_error, StoreError::Conflict { .. }));
        assert_eq!(
            agent_connection_record(fixture.runtime_home.path(), "conn_mode")?
                .expect("R2 Connection after stale report rejection"),
            outcome.connection
        );
        assert_eq!(
            crate::guards::guard_installation(fixture.runtime_home.path(), "guard_mode",)?
                .expect("R2 Guard Installation after stale report rejection"),
            after_guard
        );

        let connection_with_report = replace_agent_connection_verification_report_if_revision(
            &fixture.context()?,
            "conn_mode",
            &outcome.current_integration_revision,
            Some(&verification_report()),
        )?;
        let guard_before_no_op =
            crate::guards::guard_installation(fixture.runtime_home.path(), "guard_mode")?
                .expect("Guard Installation");
        let no_op = transition_connection_mode(
            &fixture.context()?,
            ConnectionModeTransition {
                connection_internal_id: "conn_mode".to_owned(),
                expected_mode: CONNECTION_MODE_READ_ONLY.to_owned(),
                expected_integration_revision: connection_integration_revision(
                    &connection_with_report,
                )?,
                mode: CONNECTION_MODE_READ_ONLY.to_owned(),
                guard_manifests: Vec::new(),
            },
        )?;
        assert_eq!(no_op.kind, ConnectionModeTransitionKind::Unchanged);
        assert_eq!(
            no_op.connection.integration_instance_id,
            connection_with_report.integration_instance_id
        );
        assert_eq!(
            no_op.connection.integration_generation,
            connection_with_report.integration_generation
        );
        assert_eq!(
            no_op.connection.updated_at,
            connection_with_report.updated_at
        );
        assert_eq!(
            no_op.connection.verification_report_json.as_deref(),
            Some(report.as_str())
        );
        assert_eq!(
            no_op.previous_integration_revision,
            no_op.current_integration_revision
        );
        assert_eq!(
            no_op.current_integration_revision,
            connection_integration_revision(&connection_with_report)?
        );
        assert_eq!(
            crate::guards::guard_installation(fixture.runtime_home.path(), "guard_mode")?
                .expect("Guard Installation"),
            guard_before_no_op
        );
        assert_eq!(before_connection.mode, CONNECTION_MODE_WORKFLOW);
        Ok(())
    }

    #[test]
    fn mode_transition_updates_every_project_or_rolls_back_all_candidates(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-mode-multi-project")?;
        register_additional_project(
            &fixture,
            PRIOR_OTHER_PROJECT_ID,
            "mode-multi-project-other-repo",
        )?;
        ensure_agent_connection(&fixture.context()?, connection("conn_multi_mode"))?;
        for (project_id, guard_id, policy_hash) in [
            (PROJECT_ID, "guard_mode_a", TEST_POLICY_HASH),
            (
                PRIOR_OTHER_PROJECT_ID,
                "guard_mode_b",
                "sha256:2222222222222222222222222222222222222222222222222222222222222222",
            ),
        ] {
            add_connection_project(
                &fixture.context()?,
                ConnectionProjectRegistration {
                    connection_internal_id: "conn_multi_mode".to_owned(),
                    project_id: project_id.to_owned(),
                },
            )?;
            crate::guards::upsert_guard_installation(
                &fixture.context()?,
                guard_installation_upsert_with_policy_hash(
                    fixture.runtime_home.path(),
                    guard_id,
                    "conn_multi_mode",
                    project_id,
                    policy_hash,
                ),
            )?;
        }

        let transition = mode_transition_input(
            fixture.runtime_home.path(),
            "conn_multi_mode",
            CONNECTION_MODE_READ_ONLY,
        )?;
        let before = transition
            .guard_manifests
            .iter()
            .map(|rebind| {
                (
                    rebind.guard_installation_id.clone(),
                    guard_manifest_from_json(&rebind.expected_manifest_json)
                        .expect("canonical manifest"),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let outcome = transition_connection_mode(&fixture.context()?, transition)?;
        assert_eq!(outcome.rebound_guard_installation_ids.len(), 2);
        for guard_id in ["guard_mode_a", "guard_mode_b"] {
            let manifest = guard_manifest_from_json(
                &crate::guards::guard_installation(fixture.runtime_home.path(), guard_id)?
                    .expect("Guard Installation")
                    .manifest_json,
            )?;
            let original = &before[guard_id];
            assert_eq!(
                manifest.integration_revision,
                outcome.current_integration_revision
            );
            assert_eq!(manifest.project_id, original.project_id);
            assert_eq!(manifest.policy_hash, original.policy_hash);
            assert_eq!(manifest.runtime_commands, original.runtime_commands);
            assert_eq!(manifest.managed_files, original.managed_files);
        }

        let rollback_fixture = registry_fixture("connection-mode-multi-rollback")?;
        register_additional_project(
            &rollback_fixture,
            PRIOR_OTHER_PROJECT_ID,
            "mode-multi-rollback-other-repo",
        )?;
        ensure_agent_connection(
            &rollback_fixture.context()?,
            connection("conn_multi_rollback"),
        )?;
        for (project_id, guard_id) in [
            (PROJECT_ID, "guard_rollback_a"),
            (PRIOR_OTHER_PROJECT_ID, "guard_rollback_b"),
        ] {
            add_connection_project(
                &rollback_fixture.context()?,
                ConnectionProjectRegistration {
                    connection_internal_id: "conn_multi_rollback".to_owned(),
                    project_id: project_id.to_owned(),
                },
            )?;
            crate::guards::upsert_guard_installation(
                &rollback_fixture.context()?,
                guard_installation_upsert(
                    rollback_fixture.runtime_home.path(),
                    guard_id,
                    "conn_multi_rollback",
                    project_id,
                ),
            )?;
        }
        let mut invalid = mode_transition_input(
            rollback_fixture.runtime_home.path(),
            "conn_multi_rollback",
            CONNECTION_MODE_READ_ONLY,
        )?;
        invalid.guard_manifests[1].manifest_json =
            invalid.guard_manifests[1].expected_manifest_json.clone();
        let before_connection =
            agent_connection_record(rollback_fixture.runtime_home.path(), "conn_multi_rollback")?
                .expect("connection");
        let before_manifests = invalid
            .guard_manifests
            .iter()
            .map(|rebind| {
                (
                    rebind.guard_installation_id.clone(),
                    rebind.expected_manifest_json.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert!(transition_connection_mode(&rollback_fixture.context()?, invalid).is_err());
        assert_eq!(
            agent_connection_record(rollback_fixture.runtime_home.path(), "conn_multi_rollback")?
                .expect("connection"),
            before_connection
        );
        for (guard_id, manifest_json) in before_manifests {
            assert_eq!(
                crate::guards::guard_installation(rollback_fixture.runtime_home.path(), &guard_id)?
                    .expect("Guard Installation")
                    .manifest_json,
                manifest_json
            );
        }
        Ok(())
    }

    #[test]
    fn mode_transition_rejects_missing_stale_duplicate_and_owner_mismatched_inventory(
    ) -> Result<(), Box<dyn Error>> {
        let missing = registry_fixture("connection-mode-missing-guard")?;
        ensure_agent_connection(&missing.context()?, connection("conn_missing_guard"))?;
        add_connection_project(
            &missing.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_missing_guard".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        let missing_connection =
            agent_connection_record(missing.runtime_home.path(), "conn_missing_guard")?
                .expect("connection");
        let missing_error = transition_connection_mode(
            &missing.context()?,
            ConnectionModeTransition {
                connection_internal_id: "conn_missing_guard".to_owned(),
                expected_mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                expected_integration_revision: connection_integration_revision(
                    &missing_connection,
                )?,
                mode: CONNECTION_MODE_READ_ONLY.to_owned(),
                guard_manifests: Vec::new(),
            },
        )
        .expect_err("missing Guard Installation must fail");
        assert!(matches!(missing_error, StoreError::Conflict { .. }));
        assert_eq!(
            agent_connection_record(missing.runtime_home.path(), "conn_missing_guard")?
                .expect("connection")
                .mode,
            CONNECTION_MODE_WORKFLOW
        );

        let fixture = registry_fixture("connection-mode-invalid-inventory")?;
        ensure_agent_connection(&fixture.context()?, connection("conn_invalid_mode"))?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_invalid_mode".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        crate::guards::upsert_guard_installation(
            &fixture.context()?,
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_invalid_mode",
                "conn_invalid_mode",
                PROJECT_ID,
            ),
        )?;
        let valid = mode_transition_input(
            fixture.runtime_home.path(),
            "conn_invalid_mode",
            CONNECTION_MODE_READ_ONLY,
        )?;

        let mut incomplete = valid.clone();
        incomplete.guard_manifests.clear();
        assert!(matches!(
            transition_connection_mode(&fixture.context()?, incomplete),
            Err(StoreError::Conflict { .. })
        ));

        let mut stale = valid.clone();
        stale.expected_integration_revision =
            IntegrationRevision::parse(format!("sha256:{}", "f".repeat(64)))?;
        assert!(matches!(
            transition_connection_mode(&fixture.context()?, stale),
            Err(StoreError::Conflict { .. })
        ));

        let mut duplicate = valid.clone();
        duplicate
            .guard_manifests
            .push(duplicate.guard_manifests[0].clone());
        assert!(matches!(
            transition_connection_mode(&fixture.context()?, duplicate),
            Err(StoreError::InvalidInput { .. })
        ));

        let installation =
            crate::guards::guard_installation(fixture.runtime_home.path(), "guard_invalid_mode")?
                .expect("Guard Installation");
        let mut mismatched = guard_manifest_from_json(&installation.manifest_json)?;
        mismatched.connection_id = volicord_types::AgentConnectionId::new("conn_other");
        let registry =
            open_registry_database_for_test(registry_db_path(fixture.runtime_home.path()))?;
        registry.execute(
            "UPDATE guard_installations SET manifest_json = ?2 WHERE guard_installation_id = ?1",
            params!["guard_invalid_mode", serde_json::to_string(&mismatched)?],
        )?;
        drop(registry);
        assert!(transition_connection_mode(&fixture.context()?, valid.clone()).is_err());
        assert_eq!(
            agent_connection_record(fixture.runtime_home.path(), "conn_invalid_mode")?
                .expect("connection")
                .mode,
            CONNECTION_MODE_WORKFLOW
        );

        let registry =
            open_registry_database_for_test(registry_db_path(fixture.runtime_home.path()))?;
        registry.execute(
            "UPDATE guard_installations SET manifest_json = '{}' WHERE guard_installation_id = ?1",
            ["guard_invalid_mode"],
        )?;
        drop(registry);
        let mut malformed = valid;
        malformed.guard_manifests[0].expected_manifest_json = "{}".to_owned();
        assert!(transition_connection_mode(&fixture.context()?, malformed).is_err());
        assert_eq!(
            agent_connection_record(fixture.runtime_home.path(), "conn_invalid_mode")?
                .expect("connection")
                .mode,
            CONNECTION_MODE_WORKFLOW
        );
        Ok(())
    }

    struct RegistryFixture {
        mutation: TestRuntimeHomeAdmission,
        runtime_home: TempRuntimeHome,
    }

    fn registry_fixture(name: &str) -> Result<RegistryFixture, Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new(name)?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        with_test_runtime_home_setup(runtime_home.path(), |context| {
            initialize_runtime_home(context, runtime_home.path(), "runtime_home_test", "{}")?;
            register_project(
                context,
                ProjectRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    repo_root,
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            Ok(())
        })?;
        let mutation = TestRuntimeHomeAdmission::shared(runtime_home.path())?;
        Ok(RegistryFixture {
            mutation,
            runtime_home,
        })
    }

    impl RegistryFixture {
        fn context(&self) -> StoreResult<RuntimeHomeMutationContext<'_>> {
            self.mutation.context()
        }
    }

    fn register_additional_project(
        fixture: &RegistryFixture,
        project_id: &str,
        repo_name: &str,
    ) -> StoreResult<ProjectRecord> {
        register_project(
            &fixture.context()?,
            ProjectRegistration {
                project_id: project_id.to_owned(),
                repo_root: fixture.runtime_home.create_product_repo(repo_name)?,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
    }

    fn start_test_runtime_session(
        fixture: &RegistryFixture,
        connection_internal_id: &str,
        session_source: McpRuntimeSessionSource,
        process_id: u32,
    ) -> StoreResult<McpRuntimeSessionRecord> {
        start_mcp_runtime_session_for_test(
            &fixture.context()?,
            McpRuntimeSessionStart {
                connection_internal_id: connection_internal_id.to_owned(),
                session_source,
                observed_host_executable_version: None,
                process_id,
                process_started_at: "2026-07-19T00:00:00Z".to_owned(),
            },
        )
    }

    fn upsert_test_agent_session(
        fixture: &RegistryFixture,
        runtime_session_id: &str,
        connection_internal_id: &str,
        project_id: &str,
        guard_installation_id: &str,
        host_session_id: &str,
        observed_at: &str,
    ) -> StoreResult<crate::guards::AgentSessionRecord> {
        crate::guards::bind_agent_session_runtime(
            &fixture.context()?,
            project_id,
            crate::guards::AgentSessionRuntimeBinding {
                runtime_session_id: runtime_session_id.to_owned(),
                connection_internal_id: connection_internal_id.to_owned(),
                guard_installation_id: Some(guard_installation_id.to_owned()),
                correlation: volicord_host_contract::CodexMcpCorrelation {
                    session_id: volicord_host_contract::HostSessionId::parse(host_session_id)
                        .expect("valid test host session"),
                    thread_id: volicord_host_contract::HostThreadId::parse("native.thread.fixture")
                        .expect("valid test host thread"),
                    turn_id: volicord_host_contract::HostTurnId::parse("native.turn.fixture")
                        .expect("valid test host turn"),
                },
                observed_at: observed_at.to_owned(),
            },
        )
    }

    fn registry_connection_row_count(
        fixture: &RegistryFixture,
        table: &str,
        connection_internal_id: &str,
    ) -> StoreResult<i64> {
        let registry_path = registry_db_path(fixture.runtime_home.path());
        let conn = open_registry_database_read_only(&registry_path)?;
        conn.query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE connection_internal_id = ?1"),
            [connection_internal_id],
            |row| row.get(0),
        )
        .map_err(StoreError::from)
    }

    fn registry_connection_project_row_count(
        fixture: &RegistryFixture,
        table: &str,
        connection_internal_id: &str,
        project_id: &str,
    ) -> StoreResult<i64> {
        let registry_path = registry_db_path(fixture.runtime_home.path());
        let conn = open_registry_database_read_only(&registry_path)?;
        let project = raw_project_record_from_conn(&conn, project_id)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "project",
                id: project_id.to_owned(),
            }
        })?;
        conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM {table} WHERE connection_internal_id = ?1 AND project_internal_id = ?2"
            ),
            params![connection_internal_id, project.project_internal_id],
            |row| row.get(0),
        )
        .map_err(StoreError::from)
    }

    fn delete_connection_project_membership_for_test(
        runtime_home: &Path,
        connection_internal_id: &str,
        project_id: &str,
    ) -> StoreResult<()> {
        let registry_path = registry_db_path(runtime_home);
        let conn = open_registry_database_for_test(&registry_path)?;
        let project = raw_project_record_from_conn(&conn, project_id)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "project",
                id: project_id.to_owned(),
            }
        })?;
        let changed = conn.execute(
            "DELETE FROM connection_projects
              WHERE connection_internal_id = ?1
                AND project_internal_id = ?2",
            params![connection_internal_id, project.project_internal_id],
        )?;
        if changed != 1 {
            return Err(StoreError::NotFound {
                entity: "connection_project",
                id: format!("{connection_internal_id}/{project_id}"),
            });
        }
        Ok(())
    }

    fn assert_staged_activation_rejects_non_rebasable_marker(
        fixture_name: &str,
        marker_metadata: &str,
    ) -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture(fixture_name)?;
        ensure_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                enabled: false,
                ..connection("conn_prior")
            },
        )?;
        add_connection_project(
            &fixture.context()?,
            ConnectionProjectRegistration {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        ensure_agent_connection(
            &fixture.context()?,
            AgentConnectionRegistration {
                connection_internal_id: "conn_staged".to_owned(),
                config_target: "/tmp/volicord-test-non-rebasable-marker.toml".to_owned(),
                enabled: false,
                ..connection("conn_staged")
            },
        )?;

        let registry_path = registry_db_path(fixture.runtime_home.path());
        let conn = open_registry_database_for_test(&registry_path)?;
        assert_eq!(
            conn.execute(
                "UPDATE agent_connections
                    SET metadata_json = ?2
                  WHERE connection_internal_id = ?1",
                params!["conn_prior", marker_metadata],
            )?,
            1
        );
        drop(conn);

        let error = activate_staged_connection(
            &fixture.context()?,
            "conn_staged",
            PROJECT_ID,
            &[SupersededConnectionProject {
                connection_internal_id: "conn_prior".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            }],
            guard_installation_upsert(
                fixture.runtime_home.path(),
                "guard_staged",
                "conn_staged",
                PROJECT_ID,
            ),
        )
        .expect_err("non-rebasable cleanup metadata must block staged activation");

        assert!(matches!(
            error,
            StoreError::Conflict {
                entity: "agent_connection",
                ref id,
                ..
            } if id == "conn_prior"
        ));
        let prior = agent_connection_record(fixture.runtime_home.path(), "conn_prior")?
            .expect("superseded connection must remain present");
        assert!(!prior.enabled);
        assert_eq!(prior.metadata_json, marker_metadata);
        assert_eq!(
            list_connection_projects(fixture.runtime_home.path(), "conn_prior")?.len(),
            1
        );
        let staged = agent_connection_record(fixture.runtime_home.path(), "conn_staged")?
            .expect("staged connection must remain present");
        assert!(!staged.enabled);
        assert!(list_connection_projects(fixture.runtime_home.path(), "conn_staged")?.is_empty());
        assert!(
            crate::guards::guard_installation(fixture.runtime_home.path(), "guard_staged")?
                .is_none()
        );
        Ok(())
    }

    fn connection(connection_internal_id: &str) -> AgentConnectionRegistration {
        AgentConnectionRegistration {
            connection_internal_id: connection_internal_id.to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            intent: CONNECTION_INTENT_PERSONAL.to_owned(),
            host_scope: HOST_SCOPE_USER.to_owned(),
            server_name: "volicord".to_owned(),
            config_target: "/tmp/volicord-test-config.toml".to_owned(),
            mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            enabled: true,
            managed_fingerprint: "fingerprint".to_owned(),
            metadata_json: "{}".to_owned(),
        }
    }

    fn natural_connection() -> AgentConnectionNaturalKeyRegistration {
        AgentConnectionNaturalKeyRegistration {
            host_kind: HOST_KIND_CODEX.to_owned(),
            intent: CONNECTION_INTENT_PERSONAL.to_owned(),
            host_scope: HOST_SCOPE_USER.to_owned(),
            project_ref: None,
            server_name: "volicord".to_owned(),
            config_target: "/tmp/volicord-test-natural-config.toml".to_owned(),
            mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            enabled: true,
            managed_fingerprint: "fingerprint".to_owned(),
            metadata_json: "{}".to_owned(),
        }
    }

    fn test_timestamp() -> UtcTimestamp {
        UtcTimestamp::parse("2026-07-18T00:00:00Z").expect("test timestamp")
    }

    fn verification_report() -> ConnectionVerificationReport {
        ConnectionVerificationReport::verification_not_run(test_timestamp())
            .expect("canonical test report")
    }

    fn report_json(report: &ConnectionVerificationReport) -> Result<String, serde_json::Error> {
        serde_json::to_string(report)
    }

    fn guard_installation_upsert(
        runtime_home: &Path,
        guard_installation_id: &str,
        connection_internal_id: &str,
        project_id: &str,
    ) -> GuardInstallationUpsert {
        guard_installation_upsert_with_policy_hash(
            runtime_home,
            guard_installation_id,
            connection_internal_id,
            project_id,
            TEST_POLICY_HASH,
        )
    }

    fn guard_installation_upsert_with_policy_hash(
        runtime_home: &Path,
        guard_installation_id: &str,
        connection_internal_id: &str,
        project_id: &str,
        policy_hash: &str,
    ) -> GuardInstallationUpsert {
        let repo_root = project_record_for_execution(runtime_home, project_id)
            .expect("fixture project lookup")
            .expect("fixture project")
            .repo_root;
        let connection = agent_connection_record_read_only(runtime_home, connection_internal_id)
            .expect("fixture connection lookup")
            .expect("fixture connection");
        GuardInstallationUpsert {
            guard_installation_id: guard_installation_id.to_owned(),
            connection_internal_id: connection_internal_id.to_owned(),
            project_id: project_id.to_owned(),
            manifest_json: crate::guards::test_guard_manifest_json(
                &connection,
                project_id,
                &repo_root,
                guard_installation_id,
                policy_hash,
            ),
        }
    }

    fn mode_transition_input(
        runtime_home: &Path,
        connection_internal_id: &str,
        mode: &str,
    ) -> StoreResult<ConnectionModeTransition> {
        let connection = agent_connection_record(runtime_home, connection_internal_id)?
            .expect("fixture connection");
        let expected_revision = connection_integration_revision(&connection)?;
        let mut candidate_connection = connection.clone();
        candidate_connection.mode = mode.to_owned();
        candidate_connection.integration_generation += 1;
        let candidate_revision = connection_integration_revision(&candidate_connection)?;
        let guard_manifests =
            crate::guards::list_guard_installations(runtime_home, connection_internal_id, None)?
                .into_iter()
                .map(|installation| {
                    let mut manifest = guard_manifest_from_json(&installation.manifest_json)
                        .map_err(|_| {
                            StoreError::corrupt_owner_state_json(
                                "guard_installations",
                                installation.guard_installation_id.clone(),
                                "manifest_json",
                            )
                        })?;
                    manifest.integration_revision = candidate_revision.clone();
                    Ok(ConnectionModeGuardManifestRebind {
                        guard_installation_id: installation.guard_installation_id,
                        project_id: installation.project_id,
                        expected_manifest_json: installation.manifest_json,
                        manifest_json: serde_json::to_string(&manifest).map_err(|error| {
                            StoreError::InvalidInput {
                                detail: format!("fixture manifest serialization failed: {error}"),
                            }
                        })?,
                    })
                })
                .collect::<StoreResult<Vec<_>>>()?;
        Ok(ConnectionModeTransition {
            connection_internal_id: connection_internal_id.to_owned(),
            expected_mode: connection.mode,
            expected_integration_revision: expected_revision,
            mode: mode.to_owned(),
            guard_manifests,
        })
    }
}
