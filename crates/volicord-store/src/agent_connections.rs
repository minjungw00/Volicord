use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    bootstrap::{
        raw_project_record_from_conn, validate_current_project_registration, validate_project_id,
        ProjectRecord,
    },
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

/// Personal Agent Connection intent.
pub const CONNECTION_INTENT_PERSONAL: &str = "personal";
/// Shared Agent Connection intent.
pub const CONNECTION_INTENT_SHARED: &str = "shared";
/// Global Agent Connection intent.
pub const CONNECTION_INTENT_GLOBAL: &str = "global";

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
    pub last_verification_status: String,
    pub last_verification_report_json: String,
    pub last_user_actions_json: String,
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
    pub connection_id: String,
    pub host_kind: String,
    pub intent: String,
    pub host_scope: String,
    pub project_internal_id: Option<String>,
    pub server_name: String,
    pub config_target: String,
    pub mode: String,
    pub enabled: bool,
    pub managed_fingerprint: String,
    pub last_verification_status: String,
    pub last_verification_report_json: String,
    pub last_user_actions_json: String,
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
    pub connection_internal_id: String,
    pub connection_id: String,
    pub project_internal_id: String,
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
    last_verification_status: String,
    last_verification_report_json: String,
    last_user_actions_json: String,
    metadata_json: String,
}

/// Registers or updates one Agent Connection.
pub fn ensure_agent_connection(
    runtime_home: impl AsRef<Path>,
    registration: AgentConnectionRegistration,
) -> StoreResult<AgentConnectionRecord> {
    validate_agent_connection_registration(&registration)?;

    write_agent_connection(
        runtime_home,
        AgentConnectionWriteRegistration {
            connection_internal_id: registration.connection_id,
            host_kind: registration.host_kind,
            intent: CONNECTION_INTENT_PERSONAL.to_owned(),
            host_scope: registration.host_scope,
            project_internal_id: None,
            server_name: registration.server_name,
            config_target: registration.config_target,
            mode: registration.mode,
            enabled: registration.enabled,
            managed_fingerprint: registration.managed_fingerprint,
            last_verification_status: registration.last_verified_status,
            last_verification_report_json: "{}".to_owned(),
            last_user_actions_json: "[]".to_owned(),
            metadata_json: registration.metadata_json,
        },
    )
}

/// Ensures an Agent Connection by its natural host target.
pub fn ensure_agent_connection_for_target(
    runtime_home: impl AsRef<Path>,
    registration: AgentConnectionNaturalKeyRegistration,
) -> StoreResult<AgentConnectionRecord> {
    validate_agent_connection_natural_key_registration(&registration)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let conn = open_registry_database(&registry_path)?;
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
        &runtime_home,
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
            last_verification_status: registration.last_verification_status,
            last_verification_report_json: registration.last_verification_report_json,
            last_user_actions_json: registration.last_user_actions_json,
            metadata_json: registration.metadata_json,
        },
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

    let conn = open_registry_database(&registry_path)?;
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
    runtime_home: impl AsRef<Path>,
    registration: AgentConnectionWriteRegistration,
) -> StoreResult<AgentConnectionRecord> {
    validate_agent_connection_write_registration(&registration)?;

    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;

    if let Some(existing_target_id) = connection_id_for_target(&tx, &registration)? {
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
        if !connection_target_is_compatible(&existing, &registration) {
            return Err(conflict(
                "agent_connection",
                &registration.connection_internal_id,
                "connection_internal_id is already bound to a different host target",
            ));
        }
        tx.execute(
            "UPDATE agent_connections
                SET mode = ?2,
                    enabled = ?3,
                    managed_fingerprint = ?4,
                    last_verification_status = ?5,
                    last_verification_report_json = ?6,
                    last_user_actions_json = ?7,
                    metadata_json = ?8,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    project_internal_id = ?9
              WHERE connection_internal_id = ?1",
            params![
                registration.connection_internal_id,
                registration.mode,
                enabled_as_i64(registration.enabled),
                registration.managed_fingerprint,
                registration.last_verification_status,
                registration.last_verification_report_json,
                registration.last_user_actions_json,
                registration.metadata_json,
                registration.project_internal_id
            ],
        )?;
    } else {
        tx.execute(
            "INSERT INTO agent_connections (
                connection_internal_id,
                host_kind,
                intent,
                host_scope,
                project_internal_id,
                server_name,
                config_target,
                mode,
                enabled,
                managed_fingerprint,
                last_verification_status,
                last_verification_report_json,
                last_user_actions_json,
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
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )",
            params![
                registration.connection_internal_id,
                registration.host_kind,
                registration.intent,
                registration.host_scope,
                registration.project_internal_id,
                registration.server_name,
                registration.config_target,
                registration.mode,
                enabled_as_i64(registration.enabled),
                registration.managed_fingerprint,
                registration.last_verification_status,
                registration.last_verification_report_json,
                registration.last_user_actions_json,
                registration.metadata_json
            ],
        )?;
    }
    tx.commit()?;

    agent_connection_record_from_conn(&conn, &registration.connection_internal_id)?.ok_or_else(
        || StoreError::NotFound {
            entity: "agent_connection",
            id: registration.connection_internal_id,
        },
    )
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
            connection_internal_id,
            host_kind,
            intent,
            host_scope,
            project_internal_id,
            server_name,
            config_target,
            mode,
            enabled,
            managed_fingerprint,
            last_verification_status,
            last_verification_report_json,
            last_user_actions_json,
            created_at,
            updated_at,
            metadata_json
         FROM agent_connections
         ORDER BY host_kind, intent, host_scope, server_name, connection_internal_id",
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
          WHERE connection_internal_id = ?1",
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

/// Updates one Agent Connection mode.
pub fn set_connection_mode(
    runtime_home: impl AsRef<Path>,
    connection_id: &str,
    mode: &str,
) -> StoreResult<AgentConnectionRecord> {
    validate_identifier("connection_id", connection_id)?;
    validate_connection_mode(mode)?;
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let changed = tx.execute(
        "UPDATE agent_connections
            SET mode = ?2,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE connection_internal_id = ?1",
        params![connection_id, mode],
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
                last_verification_status = ?3,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE connection_internal_id = ?1",
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

/// Updates verification status, full report JSON, user actions JSON, and fingerprint.
pub fn update_agent_connection_verification_report(
    runtime_home: impl AsRef<Path>,
    connection_id: &str,
    last_verification_status: &str,
    managed_fingerprint: &str,
    last_verification_report_json: &str,
    last_user_actions_json: &str,
) -> StoreResult<AgentConnectionRecord> {
    validate_identifier("connection_id", connection_id)?;
    validate_verification_status(last_verification_status)?;
    validate_nonempty("managed_fingerprint", managed_fingerprint)?;
    validate_json_object(
        "agent_connections.last_verification_report_json",
        last_verification_report_json,
    )?;
    validate_json_array(
        "agent_connections.last_user_actions_json",
        last_user_actions_json,
    )?;
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    require_runtime_home(&tx, &registry_path)?;
    let changed = tx.execute(
        "UPDATE agent_connections
            SET managed_fingerprint = ?2,
                last_verification_status = ?3,
                last_verification_report_json = ?4,
                last_user_actions_json = ?5,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE connection_internal_id = ?1",
        params![
            connection_id,
            managed_fingerprint,
            last_verification_status,
            last_verification_report_json,
            last_user_actions_json
        ],
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
          WHERE connection_internal_id = ?1",
        [connection_id],
        |row| row.get(0),
    )?;
    if membership_count != 0 {
        tx.commit()?;
        return Ok(false);
    }

    let changed = tx.execute(
        "DELETE FROM agent_connections WHERE connection_internal_id = ?1",
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
    let connection = require_agent_connection(&tx, &registration.connection_id)?;
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

/// Removes one project from a connection allowlist.
pub fn remove_connection_project(
    runtime_home: impl AsRef<Path>,
    connection_id: &str,
    project_id: &str,
) -> StoreResult<bool> {
    validate_identifier("connection_id", connection_id)?;
    validate_project_id(project_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let connection = require_agent_connection(&tx, connection_id)?;
    let Some(project) = raw_project_record_from_conn(&tx, project_id)? else {
        tx.commit()?;
        return Ok(false);
    };
    let changed = tx.execute(
        "DELETE FROM connection_projects
          WHERE connection_internal_id = ?1
            AND project_internal_id = ?2",
        params![
            connection.connection_internal_id,
            project.project_internal_id
        ],
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
    let Some(connection) = agent_connection_record_from_conn(&conn, connection_id)? else {
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
        connection_id: connection.connection_id,
        project_id: resolved_project_id,
        connection_enabled: connection.enabled,
        project_allowed,
        project,
    }))
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
    validate_verification_status(&registration.last_verification_status)?;
    validate_json_object(
        "agent_connections.last_verification_report_json",
        &registration.last_verification_report_json,
    )?;
    validate_json_array(
        "agent_connections.last_user_actions_json",
        &registration.last_user_actions_json,
    )?;
    validate_json_object(
        "agent_connections.metadata_json",
        &registration.metadata_json,
    )
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
    validate_verification_status(&registration.last_verification_status)?;
    validate_json_object(
        "agent_connections.last_verification_report_json",
        &registration.last_verification_report_json,
    )?;
    validate_json_array(
        "agent_connections.last_user_actions_json",
        &registration.last_user_actions_json,
    )?;
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

fn validate_connection_intent(intent: &str) -> StoreResult<()> {
    if matches!(
        intent,
        CONNECTION_INTENT_PERSONAL | CONNECTION_INTENT_SHARED | CONNECTION_INTENT_GLOBAL
    ) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "intent must be personal, shared, or global".to_owned(),
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
            connection_internal_id,
            host_kind,
            intent,
            host_scope,
            project_internal_id,
            server_name,
            config_target,
            mode,
            enabled,
            managed_fingerprint,
            last_verification_status,
            last_verification_report_json,
            last_user_actions_json,
            created_at,
            updated_at,
            metadata_json
         FROM agent_connections
         WHERE connection_internal_id = ?1",
        [connection_id],
        agent_connection_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn agent_connection_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<AgentConnectionRecord> {
    let connection_internal_id = row.get::<_, String>(0)?;
    let last_verification_status = row.get::<_, String>(10)?;
    Ok(AgentConnectionRecord {
        connection_id: connection_internal_id.clone(),
        connection_internal_id,
        host_kind: row.get(1)?,
        intent: row.get(2)?,
        host_scope: row.get(3)?,
        project_internal_id: row.get(4)?,
        server_name: row.get(5)?,
        config_target: row.get(6)?,
        mode: row.get(7)?,
        enabled: row.get::<_, i64>(8)? == 1,
        managed_fingerprint: row.get(9)?,
        last_verified_status: last_verification_status.clone(),
        last_verification_status,
        last_verification_report_json: row.get(11)?,
        last_user_actions_json: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        metadata_json: row.get(15)?,
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
    let connection_id = row.get::<_, String>(0)?;
    Ok(ConnectionProjectRecord {
        connection_id: connection_id.clone(),
        connection_internal_id: connection_id,
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

fn connection_id_for_target(
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
