use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::{
    bootstrap::{validate_current_project_registration, validate_project_id, ProjectRecord},
    sqlite::{begin_immediate_transaction, open_registry_database, registry_db_path},
    StoreError, StoreResult,
};

/// Baseline-valid Codex host kind.
pub const HOST_KIND_CODEX: &str = "codex";
/// Baseline-valid Claude Code host kind.
pub const HOST_KIND_CLAUDE_CODE: &str = "claude_code";
/// Baseline-valid exported generic host kind.
pub const HOST_KIND_GENERIC: &str = "generic";

/// Baseline-valid user-scoped host configuration.
pub const HOST_SCOPE_USER: &str = "user";
/// Baseline-valid project-scoped host configuration.
pub const HOST_SCOPE_PROJECT: &str = "project";
/// Baseline-valid local host configuration.
pub const HOST_SCOPE_LOCAL: &str = "local";
/// Baseline-valid exported host configuration.
pub const HOST_SCOPE_EXPORT: &str = "export";

/// Agent Connection mode that allows read-only operations.
pub const CONNECTION_MODE_READ_ONLY: &str = "read_only";
/// Agent Connection mode that allows workflow operations.
pub const CONNECTION_MODE_WORKFLOW: &str = "workflow";

/// Agent Connection has not been checked.
pub const VERIFIED_STATUS_NOT_VERIFIED: &str = "not_verified";
/// Agent Connection verification completed.
pub const VERIFIED_STATUS_COMPLETE: &str = "complete";
/// Agent Connection needs a host-controlled action.
pub const VERIFIED_STATUS_ACTION_REQUIRED: &str = "action_required";
/// Agent Connection verification partly succeeded or partly failed.
pub const VERIFIED_STATUS_PARTIAL_FAILURE: &str = "partial_failure";
/// Agent Connection verification failed.
pub const VERIFIED_STATUS_FAILED: &str = "failed";

/// Agent Connection creation or compatible update input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConnectionRegistration {
    pub connection_id: String,
    pub host_kind: String,
    pub host_scope: String,
    pub server_name: String,
    pub config_target: String,
    pub mode: String,
    pub enabled: bool,
    pub managed_fingerprint: String,
    pub last_verified_status: String,
    pub metadata_json: String,
}

/// Agent Connection row stored in `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConnectionRecord {
    pub connection_id: String,
    pub host_kind: String,
    pub host_scope: String,
    pub server_name: String,
    pub config_target: String,
    pub mode: String,
    pub enabled: bool,
    pub managed_fingerprint: String,
    pub last_verified_status: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Explicit project allowlist row creation input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProjectRegistration {
    pub connection_id: String,
    pub project_id: String,
}

/// Explicit project allowlist row with current project registration facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProjectRecord {
    pub connection_id: String,
    pub project_id: String,
    pub created_at: String,
    pub project: ProjectRecord,
}

/// Current dynamic project-access facts for one connection/project pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConnectionProjectAccess {
    pub connection_id: String,
    pub project_id: String,
    pub connection_enabled: bool,
    pub project_allowed: bool,
    pub project: Option<ProjectRecord>,
}

/// Registers or updates one Agent Connection.
pub fn ensure_agent_connection(
    runtime_home: impl AsRef<Path>,
    registration: AgentConnectionRegistration,
) -> StoreResult<AgentConnectionRecord> {
    validate_agent_connection_registration(&registration)?;

    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;

    if let Some(existing_target_id) = connection_id_for_target(&tx, &registration)? {
        if existing_target_id != registration.connection_id {
            return Err(conflict(
                "agent_connection",
                &registration.connection_id,
                "host target is already managed by another connection_id",
            ));
        }
    }

    if let Some(existing) = agent_connection_record_from_conn(&tx, &registration.connection_id)? {
        if !connection_target_is_compatible(&existing, &registration) {
            return Err(conflict(
                "agent_connection",
                &registration.connection_id,
                "connection_id is already bound to a different host target",
            ));
        }
        tx.execute(
            "UPDATE agent_connections
                SET mode = ?2,
                    enabled = ?3,
                    managed_fingerprint = ?4,
                    last_verified_status = ?5,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    metadata_json = ?6
              WHERE connection_id = ?1",
            params![
                registration.connection_id,
                registration.mode,
                enabled_as_i64(registration.enabled),
                registration.managed_fingerprint,
                registration.last_verified_status,
                registration.metadata_json
            ],
        )?;
    } else {
        tx.execute(
            "INSERT INTO agent_connections (
                connection_id,
                host_kind,
                host_scope,
                server_name,
                config_target,
                mode,
                enabled,
                managed_fingerprint,
                last_verified_status,
                created_at,
                updated_at,
                metadata_json
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
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?10
            )",
            params![
                registration.connection_id,
                registration.host_kind,
                registration.host_scope,
                registration.server_name,
                registration.config_target,
                registration.mode,
                enabled_as_i64(registration.enabled),
                registration.managed_fingerprint,
                registration.last_verified_status,
                registration.metadata_json
            ],
        )?;
    }
    tx.commit()?;

    agent_connection_record_from_conn(&conn, &registration.connection_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "agent_connection",
            id: registration.connection_id,
        }
    })
}

/// Reads one Agent Connection.
pub fn agent_connection_record(
    runtime_home: impl AsRef<Path>,
    connection_id: &str,
) -> StoreResult<Option<AgentConnectionRecord>> {
    validate_identifier("connection_id", connection_id)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database(registry_path)?;
    agent_connection_record_from_conn(&conn, connection_id)
}

/// Lists Agent Connections in deterministic order.
pub fn list_agent_connections(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<Vec<AgentConnectionRecord>> {
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(Vec::new());
    }

    let conn = open_registry_database(registry_path)?;
    let mut stmt = conn.prepare(
        "SELECT
            connection_id,
            host_kind,
            host_scope,
            server_name,
            config_target,
            mode,
            enabled,
            managed_fingerprint,
            last_verified_status,
            created_at,
            updated_at,
            metadata_json
         FROM agent_connections
         ORDER BY connection_id",
    )?;
    let mut rows = stmt.query([])?;
    let mut connections = Vec::new();
    while let Some(row) = rows.next()? {
        connections.push(agent_connection_record_from_row(row)?);
    }
    Ok(connections)
}

/// Enables or disables an Agent Connection.
pub fn set_connection_enabled(
    runtime_home: impl AsRef<Path>,
    connection_id: &str,
    enabled: bool,
) -> StoreResult<AgentConnectionRecord> {
    validate_identifier("connection_id", connection_id)?;
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let changed = tx.execute(
        "UPDATE agent_connections
            SET enabled = ?2,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE connection_id = ?1",
        params![connection_id, enabled_as_i64(enabled)],
    )?;
    if changed == 0 {
        return Err(StoreError::NotFound {
            entity: "agent_connection",
            id: connection_id.to_owned(),
        });
    }
    tx.commit()?;

    agent_connection_record_from_conn(&conn, connection_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "agent_connection",
        id: connection_id.to_owned(),
    })
}

/// Updates last-known Agent Connection verification state and fingerprint.
pub fn update_agent_connection_verification(
    runtime_home: impl AsRef<Path>,
    connection_id: &str,
    last_verified_status: &str,
    managed_fingerprint: &str,
) -> StoreResult<AgentConnectionRecord> {
    validate_identifier("connection_id", connection_id)?;
    validate_verification_status(last_verified_status)?;
    validate_nonempty("managed_fingerprint", managed_fingerprint)?;
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let changed = tx.execute(
        "UPDATE agent_connections
            SET managed_fingerprint = ?2,
                last_verified_status = ?3,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE connection_id = ?1",
        params![connection_id, managed_fingerprint, last_verified_status],
    )?;
    if changed == 0 {
        return Err(StoreError::NotFound {
            entity: "agent_connection",
            id: connection_id.to_owned(),
        });
    }
    tx.commit()?;

    agent_connection_record_from_conn(&conn, connection_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "agent_connection",
        id: connection_id.to_owned(),
    })
}

/// Removes an Agent Connection only when no project memberships remain.
pub fn remove_agent_connection_if_unused(
    runtime_home: impl AsRef<Path>,
    connection_id: &str,
) -> StoreResult<bool> {
    validate_identifier("connection_id", connection_id)?;
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    require_agent_connection(&tx, connection_id)?;

    let membership_count: i64 = tx.query_row(
        "SELECT COUNT(*)
           FROM connection_projects
          WHERE connection_id = ?1",
        [connection_id],
        |row| row.get(0),
    )?;
    if membership_count != 0 {
        tx.commit()?;
        return Ok(false);
    }

    let changed = tx.execute(
        "DELETE FROM agent_connections WHERE connection_id = ?1",
        [connection_id],
    )?;
    tx.commit()?;
    Ok(changed > 0)
}

/// Adds a registered project to a connection allowlist.
pub fn add_connection_project(
    runtime_home: impl AsRef<Path>,
    registration: ConnectionProjectRegistration,
) -> StoreResult<ConnectionProjectRecord> {
    validate_connection_project_registration(&registration)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    require_agent_connection(&tx, &registration.connection_id)?;
    require_current_project_registration(&tx, &runtime_home, &registration.project_id)?;
    tx.execute(
        "INSERT OR IGNORE INTO connection_projects (
            connection_id,
            project_id,
            created_at
        )
        VALUES (
            ?1,
            ?2,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        )",
        params![registration.connection_id, registration.project_id],
    )?;
    tx.commit()?;

    connection_project_record_from_conn(
        &conn,
        &runtime_home,
        &registration.connection_id,
        &registration.project_id,
    )?
    .ok_or_else(|| StoreError::NotFound {
        entity: "connection_project",
        id: format!("{}/{}", registration.connection_id, registration.project_id),
    })
}

/// Removes one project from a connection allowlist.
pub fn remove_connection_project(
    runtime_home: impl AsRef<Path>,
    connection_id: &str,
    project_id: &str,
) -> StoreResult<bool> {
    validate_identifier("connection_id", connection_id)?;
    validate_project_id(project_id)?;
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_agent_connection(&tx, connection_id)?;
    let changed = tx.execute(
        "DELETE FROM connection_projects
          WHERE connection_id = ?1
            AND project_id = ?2",
        params![connection_id, project_id],
    )?;
    tx.commit()?;
    Ok(changed > 0)
}

/// Lists the explicitly allowed projects for one Agent Connection.
pub fn list_connection_projects(
    runtime_home: impl AsRef<Path>,
    connection_id: &str,
) -> StoreResult<Vec<ConnectionProjectRecord>> {
    validate_identifier("connection_id", connection_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Err(StoreError::NotFound {
            entity: "agent_connection",
            id: connection_id.to_owned(),
        });
    }

    let conn = open_registry_database(registry_path)?;
    require_agent_connection(&conn, connection_id)?;
    let mut stmt = conn.prepare(
        "SELECT
            cp.connection_id,
            cp.project_id,
            cp.created_at,
            p.runtime_home_id,
            p.repo_root,
            p.project_home,
            p.state_db_path,
            p.status,
            p.metadata_json
         FROM connection_projects AS cp
         JOIN projects AS p
           ON p.project_id = cp.project_id
        WHERE cp.connection_id = ?1
        ORDER BY cp.project_id",
    )?;
    let mut rows = stmt.query([connection_id])?;
    let mut projects = Vec::new();
    while let Some(row) = rows.next()? {
        let project = connection_project_record_from_row(row)?;
        projects.push(validate_connection_project_record(&runtime_home, project)?);
    }
    Ok(projects)
}

/// Returns current access facts for a connection/project pair.
pub fn agent_connection_project_access(
    runtime_home: impl AsRef<Path>,
    connection_id: &str,
    project_id: &str,
) -> StoreResult<Option<AgentConnectionProjectAccess>> {
    validate_identifier("connection_id", connection_id)?;
    validate_project_id(project_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database(registry_path)?;
    let access = conn
        .query_row(
            "SELECT
            ac.enabled,
            CASE WHEN cp.project_id IS NULL THEN 0 ELSE 1 END AS project_allowed,
            p.project_id,
            p.runtime_home_id,
            p.repo_root,
            p.project_home,
            p.state_db_path,
            p.status,
            p.metadata_json
         FROM agent_connections AS ac
         LEFT JOIN connection_projects AS cp
           ON cp.connection_id = ac.connection_id
          AND cp.project_id = ?2
         LEFT JOIN projects AS p
           ON p.project_id = ?2
        WHERE ac.connection_id = ?1",
            params![connection_id, project_id],
            |row| {
                let project_id_value = row.get::<_, Option<String>>(2)?;
                let project = if let Some(project_id_value) = project_id_value {
                    Some(ProjectRecord {
                        project_id: project_id_value,
                        runtime_home_id: row.get(3)?,
                        repo_root: row.get::<_, String>(4)?.into(),
                        project_home: row.get::<_, String>(5)?.into(),
                        state_db_path: row.get::<_, String>(6)?.into(),
                        status: row.get(7)?,
                        metadata_json: row.get(8)?,
                    })
                } else {
                    None
                };
                Ok(AgentConnectionProjectAccess {
                    connection_id: connection_id.to_owned(),
                    project_id: project_id.to_owned(),
                    connection_enabled: row.get::<_, i64>(0)? == 1,
                    project_allowed: row.get::<_, i64>(1)? == 1,
                    project,
                })
            },
        )
        .optional()
        .map_err(StoreError::from)?;
    access
        .map(|mut access| {
            if let Some(project) = access.project.take() {
                access.project = Some(validate_current_project_registration(
                    &runtime_home,
                    &project,
                )?);
            }
            Ok(access)
        })
        .transpose()
}

/// Returns whether the connection is enabled and the project is allowlisted.
pub fn is_agent_connection_project_allowed(
    runtime_home: impl AsRef<Path>,
    connection_id: &str,
    project_id: &str,
) -> StoreResult<bool> {
    Ok(
        agent_connection_project_access(runtime_home, connection_id, project_id)?
            .is_some_and(|access| access.connection_enabled && access.project_allowed),
    )
}

fn validate_agent_connection_registration(
    registration: &AgentConnectionRegistration,
) -> StoreResult<()> {
    validate_identifier("connection_id", &registration.connection_id)?;
    validate_host_kind_scope(&registration.host_kind, &registration.host_scope)?;
    validate_nonempty("server_name", &registration.server_name)?;
    validate_nonempty("config_target", &registration.config_target)?;
    validate_connection_mode(&registration.mode)?;
    validate_nonempty("managed_fingerprint", &registration.managed_fingerprint)?;
    validate_verification_status(&registration.last_verified_status)?;
    validate_json_object(
        "agent_connections.metadata_json",
        &registration.metadata_json,
    )
}

fn validate_connection_project_registration(
    registration: &ConnectionProjectRegistration,
) -> StoreResult<()> {
    validate_identifier("connection_id", &registration.connection_id)?;
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
        (HOST_KIND_CODEX, HOST_SCOPE_USER)
            | (HOST_KIND_CODEX, HOST_SCOPE_PROJECT)
            | (HOST_KIND_CLAUDE_CODE, HOST_SCOPE_LOCAL)
            | (HOST_KIND_CLAUDE_CODE, HOST_SCOPE_PROJECT)
            | (HOST_KIND_CLAUDE_CODE, HOST_SCOPE_USER)
            | (HOST_KIND_GENERIC, HOST_SCOPE_EXPORT)
    );
    if valid {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "host_kind and host_scope must match the supported matrix".to_owned(),
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

fn validate_verification_status(status: &str) -> StoreResult<()> {
    if matches!(
        status,
        VERIFIED_STATUS_NOT_VERIFIED
            | VERIFIED_STATUS_COMPLETE
            | VERIFIED_STATUS_ACTION_REQUIRED
            | VERIFIED_STATUS_PARTIAL_FAILURE
            | VERIFIED_STATUS_FAILED
    ) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "last_verified_status is not supported".to_owned(),
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
    connection_id: &str,
) -> StoreResult<AgentConnectionRecord> {
    agent_connection_record_from_conn(conn, connection_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "agent_connection",
        id: connection_id.to_owned(),
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

fn agent_connection_record_from_conn(
    conn: &Connection,
    connection_id: &str,
) -> StoreResult<Option<AgentConnectionRecord>> {
    conn.query_row(
        "SELECT
            connection_id,
            host_kind,
            host_scope,
            server_name,
            config_target,
            mode,
            enabled,
            managed_fingerprint,
            last_verified_status,
            created_at,
            updated_at,
            metadata_json
         FROM agent_connections
         WHERE connection_id = ?1",
        [connection_id],
        agent_connection_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn agent_connection_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<AgentConnectionRecord> {
    Ok(AgentConnectionRecord {
        connection_id: row.get(0)?,
        host_kind: row.get(1)?,
        host_scope: row.get(2)?,
        server_name: row.get(3)?,
        config_target: row.get(4)?,
        mode: row.get(5)?,
        enabled: row.get::<_, i64>(6)? == 1,
        managed_fingerprint: row.get(7)?,
        last_verified_status: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        metadata_json: row.get(11)?,
    })
}

fn connection_project_record_from_conn(
    conn: &Connection,
    runtime_home: &Path,
    connection_id: &str,
    project_id: &str,
) -> StoreResult<Option<ConnectionProjectRecord>> {
    let record = conn
        .query_row(
            "SELECT
                cp.connection_id,
                cp.project_id,
                cp.created_at,
                p.runtime_home_id,
                p.repo_root,
                p.project_home,
                p.state_db_path,
                p.status,
                p.metadata_json
             FROM connection_projects AS cp
             JOIN projects AS p
               ON p.project_id = cp.project_id
            WHERE cp.connection_id = ?1
              AND cp.project_id = ?2",
            params![connection_id, project_id],
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
    let project = conn
        .query_row(
            "SELECT
                project_id,
                runtime_home_id,
                repo_root,
                project_home,
                state_db_path,
                status,
                metadata_json
             FROM projects
             WHERE project_id = ?1",
            params![project_id],
            |row| {
                Ok(ProjectRecord {
                    project_id: row.get(0)?,
                    runtime_home_id: row.get(1)?,
                    repo_root: PathBuf::from(row.get::<_, String>(2)?),
                    project_home: PathBuf::from(row.get::<_, String>(3)?),
                    state_db_path: PathBuf::from(row.get::<_, String>(4)?),
                    status: row.get(5)?,
                    metadata_json: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(StoreError::from)?;
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
    Ok(ConnectionProjectRecord {
        connection_id: row.get(0)?,
        project_id: project_id.clone(),
        created_at: row.get(2)?,
        project: ProjectRecord {
            project_id,
            runtime_home_id: row.get(3)?,
            repo_root: PathBuf::from(row.get::<_, String>(4)?),
            project_home: PathBuf::from(row.get::<_, String>(5)?),
            state_db_path: PathBuf::from(row.get::<_, String>(6)?),
            status: row.get(7)?,
            metadata_json: row.get(8)?,
        },
    })
}

fn connection_id_for_target(
    conn: &Connection,
    registration: &AgentConnectionRegistration,
) -> StoreResult<Option<String>> {
    conn.query_row(
        "SELECT connection_id
           FROM agent_connections
          WHERE host_kind = ?1
            AND host_scope = ?2
            AND config_target = ?3
            AND server_name = ?4",
        params![
            registration.host_kind,
            registration.host_scope,
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
    registration: &AgentConnectionRegistration,
) -> bool {
    existing.host_kind == registration.host_kind
        && existing.host_scope == registration.host_scope
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

#[cfg(test)]
mod tests {
    use std::error::Error;

    use volicord_test_support::TempRuntimeHome;

    use super::*;
    use crate::bootstrap::{
        initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
    };

    const PROJECT_ID: &str = "project_a";

    #[test]
    fn agent_connection_registration_updates_and_lists() -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-register")?;

        let created = ensure_agent_connection(fixture.runtime_home.path(), connection("conn_a"))?;
        let updated = ensure_agent_connection(
            fixture.runtime_home.path(),
            AgentConnectionRegistration {
                mode: CONNECTION_MODE_READ_ONLY.to_owned(),
                enabled: false,
                managed_fingerprint: "fingerprint-updated".to_owned(),
                last_verified_status: VERIFIED_STATUS_COMPLETE.to_owned(),
                metadata_json: r#"{"updated":true}"#.to_owned(),
                ..connection("conn_a")
            },
        )?;
        let read = agent_connection_record(fixture.runtime_home.path(), "conn_a")?
            .expect("connection should be readable");
        let listed = list_agent_connections(fixture.runtime_home.path())?;

        assert_eq!(created.connection_id, "conn_a");
        assert_eq!(updated.mode, CONNECTION_MODE_READ_ONLY);
        assert!(!updated.enabled);
        assert_eq!(updated.managed_fingerprint, "fingerprint-updated");
        assert_eq!(read, updated);
        assert_eq!(listed, vec![updated]);
        Ok(())
    }

    #[test]
    fn agent_connection_rejects_conflicting_target() -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-conflict")?;
        ensure_agent_connection(fixture.runtime_home.path(), connection("conn_a"))?;

        let error = ensure_agent_connection(
            fixture.runtime_home.path(),
            AgentConnectionRegistration {
                connection_id: "conn_b".to_owned(),
                ..connection("conn_a")
            },
        )
        .expect_err("duplicate target should be rejected");

        assert!(matches!(error, StoreError::Conflict { .. }));
        Ok(())
    }

    #[test]
    fn connection_projects_gate_current_project_access() -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-projects")?;
        ensure_agent_connection(fixture.runtime_home.path(), connection("conn_project"))?;
        assert!(!is_agent_connection_project_allowed(
            fixture.runtime_home.path(),
            "conn_project",
            PROJECT_ID
        )?);

        let added = add_connection_project(
            fixture.runtime_home.path(),
            ConnectionProjectRegistration {
                connection_id: "conn_project".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;
        let repeated = add_connection_project(
            fixture.runtime_home.path(),
            ConnectionProjectRegistration {
                connection_id: "conn_project".to_owned(),
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

        set_connection_enabled(fixture.runtime_home.path(), "conn_project", false)?;
        assert!(!is_agent_connection_project_allowed(
            fixture.runtime_home.path(),
            "conn_project",
            PROJECT_ID
        )?);

        assert!(remove_connection_project(
            fixture.runtime_home.path(),
            "conn_project",
            PROJECT_ID
        )?);
        assert!(remove_agent_connection_if_unused(
            fixture.runtime_home.path(),
            "conn_project"
        )?);
        Ok(())
    }

    #[test]
    fn connection_cannot_be_removed_while_projects_remain() -> Result<(), Box<dyn Error>> {
        let fixture = registry_fixture("connection-remove-blocked")?;
        ensure_agent_connection(fixture.runtime_home.path(), connection("conn_blocked"))?;
        add_connection_project(
            fixture.runtime_home.path(),
            ConnectionProjectRegistration {
                connection_id: "conn_blocked".to_owned(),
                project_id: PROJECT_ID.to_owned(),
            },
        )?;

        assert!(!remove_agent_connection_if_unused(
            fixture.runtime_home.path(),
            "conn_blocked"
        )?);
        assert!(agent_connection_record(fixture.runtime_home.path(), "conn_blocked")?.is_some());
        Ok(())
    }

    struct RegistryFixture {
        runtime_home: TempRuntimeHome,
    }

    fn registry_fixture(name: &str) -> Result<RegistryFixture, Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new(name)?;
        initialize_runtime_home(runtime_home.path(), "runtime_home_test", "{}")?;
        register_project(
            runtime_home.path(),
            ProjectRegistration {
                project_id: PROJECT_ID.to_owned(),
                repo_root: runtime_home.create_product_repo("repo")?,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        Ok(RegistryFixture { runtime_home })
    }

    fn connection(connection_id: &str) -> AgentConnectionRegistration {
        AgentConnectionRegistration {
            connection_id: connection_id.to_owned(),
            host_kind: HOST_KIND_CODEX.to_owned(),
            host_scope: HOST_SCOPE_USER.to_owned(),
            server_name: "volicord".to_owned(),
            config_target: "/tmp/volicord-test-config.toml".to_owned(),
            mode: CONNECTION_MODE_WORKFLOW.to_owned(),
            enabled: true,
            managed_fingerprint: "fingerprint".to_owned(),
            last_verified_status: VERIFIED_STATUS_NOT_VERIFIED.to_owned(),
            metadata_json: "{}".to_owned(),
        }
    }
}
