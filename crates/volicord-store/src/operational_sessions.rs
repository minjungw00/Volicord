//! Authoritative MCP runtime-session lifecycle records.

use std::{path::Path, str::FromStr};

use rusqlite::{params, Connection, OptionalExtension, Row};
use volicord_types::{
    project_agent_session_id, validate_managed_host_native_session_id,
    ConnectionIntegrationRevisionBasis, DurableIdGenerator, DurableIdKind, IntegrationRevision,
    ManagedMcpClientInfo, McpRuntimeSessionSource, RandomDurableIdGenerator, UtcTimestamp,
    DURABLE_ID_RETRY_LIMIT,
};

use crate::{
    agent_connections::{raw_agent_connection_record_from_conn, AgentConnectionRecord},
    bootstrap::raw_project_record_from_conn,
    sqlite::{
        begin_immediate_transaction, open_registry_database, open_registry_database_read_only,
        registry_db_path,
    },
    StoreError, StoreResult,
};

const MAX_DIAGNOSTIC_FIELD_BYTES: usize = 1024;
const MAX_PROTOCOL_FIELD_BYTES: usize = 256;
const MAX_FAILURE_CODE_BYTES: usize = 128;
const MAX_FAILURE_DETAILS_BYTES: usize = 4096;

/// MCP process-start facts used to create an authoritative runtime session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRuntimeSessionStart {
    pub connection_internal_id: String,
    pub session_source: McpRuntimeSessionSource,
    pub observed_host_executable_version: Option<String>,
    pub process_id: u32,
    pub process_started_at: String,
}

/// Canonical Registry row for one MCP process launch and its observed milestones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRuntimeSessionRecord {
    pub runtime_session_id: String,
    pub connection_internal_id: String,
    pub session_source: McpRuntimeSessionSource,
    pub connection_integration_revision: String,
    pub observed_host_executable_version: Option<String>,
    pub client_name: Option<String>,
    pub client_version: Option<String>,
    pub negotiated_protocol_version: Option<String>,
    pub process_id: u32,
    pub process_started_at: String,
    pub initialize_completed_at: Option<String>,
    pub initialized_notification_at: Option<String>,
    pub tools_list_observed_at: Option<String>,
    pub required_tools_present: Option<bool>,
    pub last_safe_read_only_tool_call_at: Option<String>,
    pub last_observed_at: String,
    pub terminal_protocol_failure_code: Option<String>,
    pub terminal_protocol_failure_details: Option<String>,
    pub graceful_close_at: Option<String>,
}

/// Registry reservation joining one managed runtime to one project Agent Session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRuntimeProjectSessionBindingRecord {
    pub runtime_session_id: String,
    pub connection_internal_id: String,
    pub project_id: String,
    pub session_id: String,
    pub project_integration_revision: String,
    pub host_session_id: String,
    pub bound_at: String,
}

/// Derives the deterministic current integration revision for one Agent Connection.
pub fn connection_integration_revision(
    connection: &AgentConnectionRecord,
) -> StoreResult<IntegrationRevision> {
    IntegrationRevision::for_connection(ConnectionIntegrationRevisionBasis {
        connection_internal_id: &connection.connection_internal_id,
        integration_instance_id: &connection.integration_instance_id,
        host_kind: &connection.host_kind,
        intent: &connection.intent,
        host_scope: &connection.host_scope,
        mode: &connection.mode,
        server_name: &connection.server_name,
        config_target: &connection.config_target,
        managed_configuration_fingerprint: &connection.managed_fingerprint,
        integration_generation: connection.integration_generation,
    })
    .map_err(|error| StoreError::InvalidInput {
        detail: format!("Agent Connection cannot produce an integration revision: {error}"),
    })
}

/// Creates a new runtime session at MCP process startup.
pub fn start_mcp_runtime_session(
    runtime_home: impl AsRef<Path>,
    input: McpRuntimeSessionStart,
) -> StoreResult<McpRuntimeSessionRecord> {
    validate_start(&input)?;
    let registry_path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let generator = RandomDurableIdGenerator;
    for _ in 0..DURABLE_ID_RETRY_LIMIT {
        let runtime_session_id = generator
            .generate(DurableIdKind::McpRuntimeSession)
            .map_err(|error| StoreError::InvalidInput {
                detail: format!("could not generate MCP runtime session id: {error}"),
            })?;
        let tx = begin_immediate_transaction(&mut conn)?;
        let connection = raw_agent_connection_record_from_conn(&tx, &input.connection_internal_id)?
            .ok_or_else(|| StoreError::NotFound {
                entity: "agent_connection",
                id: input.connection_internal_id.clone(),
            })?;
        let revision = connection_integration_revision(&connection)?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO mcp_runtime_sessions (
                runtime_session_id, connection_internal_id, session_source,
                connection_integration_revision, observed_host_executable_version,
                process_id, process_started_at, last_observed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                runtime_session_id,
                input.connection_internal_id,
                input.session_source.as_str(),
                revision.as_str(),
                input.observed_host_executable_version,
                i64::from(input.process_id),
                input.process_started_at,
            ],
        )?;
        if inserted == 1 {
            let record = runtime_session_from_conn(&tx, &runtime_session_id)?.ok_or_else(|| {
                StoreError::NotFound {
                    entity: "mcp_runtime_session",
                    id: runtime_session_id.clone(),
                }
            })?;
            tx.commit()?;
            return Ok(record);
        }
        tx.rollback()?;
    }
    Err(StoreError::Conflict {
        entity: "mcp_runtime_session",
        id: "generated".to_owned(),
        detail: "durable id collision retry limit was exhausted".to_owned(),
    })
}

/// Reads one authoritative runtime session without consulting diagnostics.
pub fn mcp_runtime_session(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
) -> StoreResult<Option<McpRuntimeSessionRecord>> {
    validate_text(
        "runtime_session_id",
        runtime_session_id,
        MAX_DIAGNOSTIC_FIELD_BYTES,
    )?;
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    runtime_session_from_conn(&conn, runtime_session_id)
}

/// Resolves a managed runtime session only when it is still owned by the expected
/// Connection and represents that Connection's current integration revision.
pub fn current_managed_mcp_runtime_session_for_connection(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    connection_internal_id: &str,
) -> StoreResult<McpRuntimeSessionRecord> {
    let path = registry_db_path(runtime_home);
    let conn = open_registry_database_read_only(path)?;
    let session = runtime_session_from_conn(&conn, runtime_session_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
        }
    })?;
    if session.connection_internal_id != connection_internal_id
        || session.session_source != McpRuntimeSessionSource::ManagedHost
    {
        return Err(StoreError::Conflict {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
            detail: "runtime session is not a managed-host launch owned by the expected Agent Connection".to_owned(),
        });
    }
    let connection = raw_agent_connection_record_from_conn(&conn, connection_internal_id)?
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
    if connection_integration_revision(&connection)?.as_str()
        != session.connection_integration_revision
    {
        return Err(StoreError::Conflict {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
            detail:
                "runtime session does not represent the current Connection integration revision"
                    .to_owned(),
        });
    }
    Ok(session)
}

/// Reserves one runtime/host session for exactly one registered project.
/// The Registry reservation supplies the cross-database uniqueness boundary
/// that a project-local SQLite foreign key cannot express.
pub fn bind_mcp_runtime_project_session(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    connection_internal_id: &str,
    project_id: &str,
    asserted_guard_installation_id: Option<&str>,
    host_session_id: &str,
    bound_at: &str,
) -> StoreResult<McpRuntimeProjectSessionBindingRecord> {
    validate_timestamp("bound_at", bound_at)?;
    for (field, value) in [
        ("runtime_session_id", runtime_session_id),
        ("connection_internal_id", connection_internal_id),
        ("project_id", project_id),
        ("host_session_id", host_session_id),
    ] {
        validate_text(field, value, MAX_DIAGNOSTIC_FIELD_BYTES)?;
    }
    validate_managed_host_native_session_id(host_session_id).map_err(|_| {
        StoreError::InvalidInput {
            detail: "host_session_id must be valid managed-host identity metadata".to_owned(),
        }
    })?;
    let runtime_home = runtime_home.as_ref();
    let identity = crate::guards::current_project_agent_session_identity(
        runtime_home,
        project_id,
        connection_internal_id,
        asserted_guard_installation_id,
        host_session_id,
    )?;
    let session_id = identity.session_id.as_str();
    let path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let runtime = runtime_session_from_conn(&tx, runtime_session_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
        }
    })?;
    let connection = raw_agent_connection_record_from_conn(&tx, connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        })?;
    if !connection.enabled
        || runtime.connection_internal_id != connection_internal_id
        || runtime.session_source != McpRuntimeSessionSource::ManagedHost
        || runtime.connection_integration_revision
            != connection_integration_revision(&connection)?.as_str()
    {
        return Err(StoreError::Conflict {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
            detail: "runtime session is not a current managed-host launch owned by an enabled Agent Connection".to_owned(),
        });
    }
    let project =
        raw_project_record_from_conn(&tx, project_id)?.ok_or_else(|| StoreError::NotFound {
            entity: "project",
            id: project_id.to_owned(),
        })?;
    let membership: i64 = tx.query_row(
        "SELECT COUNT(*) FROM connection_projects
          WHERE connection_internal_id = ?1 AND project_internal_id = ?2",
        params![connection_internal_id, project.project_internal_id],
        |row| row.get(0),
    )?;
    if membership != 1 {
        return Err(StoreError::Conflict {
            entity: "connection_project",
            id: format!("{connection_internal_id}/{project_id}"),
            detail: "project is not a current member of the Agent Connection".to_owned(),
        });
    }
    let existing_project_session = tx
        .query_row(
            "SELECT runtime_session_id, connection_internal_id,
                    project_integration_revision, host_session_id
               FROM mcp_runtime_project_session_bindings
              WHERE project_internal_id = ?1 AND session_id = ?2",
            params![project.project_internal_id, session_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    if let Some((existing_runtime, existing_connection, existing_revision, existing_host_session)) =
        existing_project_session
    {
        if existing_runtime == runtime_session_id
            && existing_connection == connection_internal_id
            && existing_revision == identity.project_integration_revision
            && existing_host_session == host_session_id
        {
            tx.commit()?;
            return Ok(McpRuntimeProjectSessionBindingRecord {
                runtime_session_id: runtime_session_id.to_owned(),
                connection_internal_id: connection_internal_id.to_owned(),
                project_id: project.project_internal_id,
                session_id: identity.session_id,
                project_integration_revision: identity.project_integration_revision,
                host_session_id: host_session_id.to_owned(),
                bound_at: bound_at.to_owned(),
            });
        }
        return Err(StoreError::Conflict {
            entity: "agent_session",
            id: session_id.to_owned(),
            detail: "project Agent Session is already reserved for another runtime, Connection, or host session".to_owned(),
        });
    }
    let existing_runtime_host = tx
        .query_row(
            "SELECT connection_internal_id, project_internal_id, session_id
               FROM mcp_runtime_project_session_bindings
              WHERE runtime_session_id = ?1 AND host_session_id = ?2",
            params![runtime_session_id, host_session_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    if existing_runtime_host.is_some() {
        return Err(StoreError::Conflict {
            entity: "agent_session",
            id: session_id.to_owned(),
            detail: "runtime host session is already reserved for another project or Connection"
                .to_owned(),
        });
    }
    tx.execute(
        "INSERT INTO mcp_runtime_project_session_bindings (
            runtime_session_id, connection_internal_id, project_internal_id,
            session_id, project_integration_revision, host_session_id, bound_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            runtime_session_id,
            connection_internal_id,
            project.project_internal_id,
            session_id,
            identity.project_integration_revision,
            host_session_id,
            bound_at
        ],
    )?;
    tx.commit()?;
    Ok(McpRuntimeProjectSessionBindingRecord {
        runtime_session_id: runtime_session_id.to_owned(),
        connection_internal_id: connection_internal_id.to_owned(),
        project_id: project.project_internal_id,
        session_id: identity.session_id,
        project_integration_revision: identity.project_integration_revision,
        host_session_id: host_session_id.to_owned(),
        bound_at: bound_at.to_owned(),
    })
}

/// Reads the exact Registry reservation for one project Agent Session.
pub fn mcp_runtime_project_session_binding(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
    session_id: &str,
) -> StoreResult<Option<McpRuntimeProjectSessionBindingRecord>> {
    validate_text("project_id", project_id, MAX_DIAGNOSTIC_FIELD_BYTES)?;
    validate_text("session_id", session_id, MAX_DIAGNOSTIC_FIELD_BYTES)?;
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    let record = conn
        .query_row(
            "SELECT b.runtime_session_id, b.connection_internal_id, p.project_internal_id,
                    b.session_id, b.project_integration_revision, b.host_session_id, b.bound_at
               FROM mcp_runtime_project_session_bindings AS b
               JOIN projects AS p ON p.project_internal_id = b.project_internal_id
              WHERE p.project_internal_id = ?1 AND b.session_id = ?2",
            params![project_id, session_id],
            |row| {
                Ok(McpRuntimeProjectSessionBindingRecord {
                    runtime_session_id: row.get(0)?,
                    connection_internal_id: row.get(1)?,
                    project_id: row.get(2)?,
                    session_id: row.get(3)?,
                    project_integration_revision: row.get(4)?,
                    host_session_id: row.get(5)?,
                    bound_at: row.get(6)?,
                })
            },
        )
        .optional()?;
    record
        .map(|record| {
            let corrupt = |field| {
                StoreError::corrupt_owner_state_value(
                    "mcp_runtime_project_session_bindings",
                    record.session_id.clone(),
                    field,
                )
            };
            validate_timestamp("bound_at", &record.bound_at).map_err(|_| corrupt("bound_at"))?;
            for (field, value) in [
                ("runtime_session_id", record.runtime_session_id.as_str()),
                (
                    "connection_internal_id",
                    record.connection_internal_id.as_str(),
                ),
                ("project_id", record.project_id.as_str()),
                ("session_id", record.session_id.as_str()),
                (
                    "project_integration_revision",
                    record.project_integration_revision.as_str(),
                ),
            ] {
                validate_text(field, value, MAX_DIAGNOSTIC_FIELD_BYTES)
                    .map_err(|_| corrupt(field))?;
            }
            validate_managed_host_native_session_id(&record.host_session_id)
                .map_err(|_| corrupt("host_session_id"))?;
            if project_agent_session_id(
                &record.connection_internal_id,
                &record.project_integration_revision,
                &record.host_session_id,
            )
            .map_err(|_| corrupt("session_id"))?
                != record.session_id
            {
                return Err(corrupt("session_id"));
            }
            Ok(record)
        })
        .transpose()
}

/// Records successful MCP initialize completion before its response is emitted.
pub fn record_mcp_initialize(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    client_info: &ManagedMcpClientInfo,
    negotiated_protocol_version: &str,
    observed_at: &str,
) -> StoreResult<McpRuntimeSessionRecord> {
    validate_text(
        "negotiated_protocol_version",
        negotiated_protocol_version,
        MAX_PROTOCOL_FIELD_BYTES,
    )?;
    validate_timestamp("initialize_completed_at", observed_at)?;
    update_session(runtime_home, runtime_session_id, |tx, prior| {
        require_observation_time(prior, observed_at)?;
        if prior.initialize_completed_at.is_some() {
            return Err(StoreError::Conflict {
                entity: "mcp_runtime_session",
                id: runtime_session_id.to_owned(),
                detail: "initialize has already completed".to_owned(),
            });
        }
        tx.execute(
            "UPDATE mcp_runtime_sessions
                SET client_name = ?2, client_version = ?3,
                    negotiated_protocol_version = ?4,
                    initialize_completed_at = ?5, last_observed_at = ?5
              WHERE runtime_session_id = ?1",
            params![
                runtime_session_id,
                client_info.name(),
                client_info.version(),
                negotiated_protocol_version,
                observed_at
            ],
        )?;
        Ok(())
    })
}

/// Records the initialized notification. A duplicate valid notification is idempotent.
pub fn record_mcp_initialized_notification(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    observed_at: &str,
) -> StoreResult<McpRuntimeSessionRecord> {
    validate_timestamp("initialized_notification_at", observed_at)?;
    update_session(runtime_home, runtime_session_id, |tx, prior| {
        require_observation_time(prior, observed_at)?;
        if prior.initialize_completed_at.is_none() {
            return Err(milestone_order(
                runtime_session_id,
                "initialized notification requires initialize completion",
            ));
        }
        tx.execute(
            "UPDATE mcp_runtime_sessions
                SET initialized_notification_at = COALESCE(initialized_notification_at, ?2),
                    last_observed_at = ?2
              WHERE runtime_session_id = ?1",
            params![runtime_session_id, observed_at],
        )?;
        Ok(())
    })
}

/// Records one actual tools/list response and its required-tool-set fact.
pub fn record_mcp_tools_list(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    required_tools_present: bool,
    observed_at: &str,
) -> StoreResult<McpRuntimeSessionRecord> {
    validate_timestamp("tools_list_observed_at", observed_at)?;
    update_session(runtime_home, runtime_session_id, |tx, prior| {
        require_observation_time(prior, observed_at)?;
        if prior.initialize_completed_at.is_none() {
            return Err(milestone_order(
                runtime_session_id,
                "tools/list requires initialize completion",
            ));
        }
        tx.execute(
            "UPDATE mcp_runtime_sessions
                SET tools_list_observed_at = ?2, required_tools_present = ?3,
                    last_observed_at = ?2
              WHERE runtime_session_id = ?1",
            params![
                runtime_session_id,
                observed_at,
                bool_i64(required_tools_present)
            ],
        )?;
        Ok(())
    })
}

/// Records successful completion of a designated safe/read-only Volicord tool.
pub fn record_mcp_safe_read_only_tool_call(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    observed_at: &str,
) -> StoreResult<McpRuntimeSessionRecord> {
    validate_timestamp("last_safe_read_only_tool_call_at", observed_at)?;
    update_session(runtime_home, runtime_session_id, |tx, prior| {
        require_observation_time(prior, observed_at)?;
        if prior.initialized_notification_at.is_none() {
            return Err(milestone_order(
                runtime_session_id,
                "safe tool success requires the initialized notification",
            ));
        }
        tx.execute(
            "UPDATE mcp_runtime_sessions
                SET last_safe_read_only_tool_call_at = ?2, last_observed_at = ?2
              WHERE runtime_session_id = ?1",
            params![runtime_session_id, observed_at],
        )?;
        Ok(())
    })
}

/// Records an observable terminal protocol failure.
pub fn record_mcp_terminal_protocol_failure(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    code: &str,
    details: Option<&str>,
    observed_at: &str,
) -> StoreResult<McpRuntimeSessionRecord> {
    validate_text(
        "terminal_protocol_failure_code",
        code,
        MAX_FAILURE_CODE_BYTES,
    )?;
    if let Some(details) = details {
        validate_text(
            "terminal_protocol_failure_details",
            details,
            MAX_FAILURE_DETAILS_BYTES,
        )?;
    }
    validate_timestamp("terminal_failure_at", observed_at)?;
    update_session(runtime_home, runtime_session_id, |tx, prior| {
        require_observation_time(prior, observed_at)?;
        if prior.graceful_close_at.is_some() {
            return Err(milestone_order(
                runtime_session_id,
                "terminal failure cannot follow graceful close",
            ));
        }
        if let Some(existing) = prior.terminal_protocol_failure_code.as_deref() {
            if existing == code && prior.terminal_protocol_failure_details.as_deref() == details {
                return Ok(());
            }
            return Err(milestone_order(
                runtime_session_id,
                "terminal failure is already recorded",
            ));
        }
        tx.execute(
            "UPDATE mcp_runtime_sessions
                SET terminal_protocol_failure_code = ?2,
                    terminal_protocol_failure_details = ?3,
                    last_observed_at = ?4
              WHERE runtime_session_id = ?1",
            params![runtime_session_id, code, details, observed_at],
        )?;
        Ok(())
    })
}

/// Records observable graceful transport close. A duplicate close is idempotent.
pub fn record_mcp_graceful_close(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    observed_at: &str,
) -> StoreResult<McpRuntimeSessionRecord> {
    validate_timestamp("graceful_close_at", observed_at)?;
    update_session(runtime_home, runtime_session_id, |tx, prior| {
        require_observation_time(prior, observed_at)?;
        if prior.terminal_protocol_failure_code.is_some() {
            return Err(milestone_order(
                runtime_session_id,
                "graceful close cannot follow terminal failure",
            ));
        }
        tx.execute(
            "UPDATE mcp_runtime_sessions
                SET graceful_close_at = COALESCE(graceful_close_at, ?2), last_observed_at = ?2
              WHERE runtime_session_id = ?1",
            params![runtime_session_id, observed_at],
        )?;
        Ok(())
    })
}

/// Returns the latest successful managed-host observation for the current revision.
/// CLI preflight sessions and diagnostics are structurally outside this lookup.
pub fn latest_successful_managed_runtime_session(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Option<McpRuntimeSessionRecord>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    let connection = raw_agent_connection_record_from_conn(&conn, connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        })?;
    let revision = connection_integration_revision(&connection)?;
    conn.query_row(
        &format!(
            "{RUNTIME_SESSION_SELECT}
          WHERE connection_internal_id = ?1
            AND session_source = 'managed_host'
            AND connection_integration_revision = ?2
            AND initialize_completed_at IS NOT NULL
            AND initialized_notification_at IS NOT NULL
            AND tools_list_observed_at IS NOT NULL
            AND required_tools_present = 1
            AND last_safe_read_only_tool_call_at IS NOT NULL
          ORDER BY last_safe_read_only_tool_call_at DESC, runtime_session_id DESC
          LIMIT 1"
        ),
        params![connection_internal_id, revision.as_str()],
        runtime_session_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|record| record.map(validate_runtime_session).transpose())
}

/// Returns the latest managed-host observation for a Connection, including an
/// older integration revision or a session that stopped before all milestones.
/// CLI preflight sessions are structurally excluded.
pub fn latest_managed_runtime_session(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Option<McpRuntimeSessionRecord>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    conn.query_row(
        &format!(
            "{RUNTIME_SESSION_SELECT}
          WHERE connection_internal_id = ?1
            AND session_source = 'managed_host'
          ORDER BY last_observed_at DESC, runtime_session_id DESC
          LIMIT 1"
        ),
        params![connection_internal_id],
        runtime_session_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|record| record.map(validate_runtime_session).transpose())
}

/// Returns the latest managed-host observation for the Connection's current
/// integration revision, whether complete, in progress, or terminally failed.
/// CLI preflight sessions are structurally excluded.
pub fn latest_current_managed_runtime_session(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Option<McpRuntimeSessionRecord>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    let connection = raw_agent_connection_record_from_conn(&conn, connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        })?;
    let revision = connection_integration_revision(&connection)?;
    conn.query_row(
        &format!(
            "{RUNTIME_SESSION_SELECT}
          WHERE connection_internal_id = ?1
            AND session_source = 'managed_host'
            AND connection_integration_revision = ?2
          ORDER BY last_observed_at DESC, runtime_session_id DESC
          LIMIT 1"
        ),
        params![connection_internal_id, revision.as_str()],
        runtime_session_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|record| record.map(validate_runtime_session).transpose())
}

/// Returns every managed-host observation for the Connection's current revision.
/// CLI preflight sessions are structurally excluded.
pub fn current_managed_runtime_sessions(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Vec<McpRuntimeSessionRecord>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database_read_only(path)?;
    let connection = raw_agent_connection_record_from_conn(&conn, connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        })?;
    let revision = connection_integration_revision(&connection)?;
    let mut stmt = conn.prepare(&format!(
        "{RUNTIME_SESSION_SELECT}
          WHERE connection_internal_id = ?1
            AND session_source = 'managed_host'
            AND connection_integration_revision = ?2
          ORDER BY last_observed_at DESC, runtime_session_id DESC"
    ))?;
    let rows = stmt.query_map(
        params![connection_internal_id, revision.as_str()],
        runtime_session_from_row,
    )?;
    rows.collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(validate_runtime_session)
        .collect()
}

fn update_session<F>(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    update: F,
) -> StoreResult<McpRuntimeSessionRecord>
where
    F: FnOnce(&rusqlite::Transaction<'_>, &McpRuntimeSessionRecord) -> StoreResult<()>,
{
    validate_text(
        "runtime_session_id",
        runtime_session_id,
        MAX_DIAGNOSTIC_FIELD_BYTES,
    )?;
    let path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let prior = runtime_session_from_conn(&tx, runtime_session_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
        }
    })?;
    update(&tx, &prior)?;
    let record = runtime_session_from_conn(&tx, runtime_session_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
        }
    })?;
    tx.commit()?;
    Ok(record)
}

const RUNTIME_SESSION_SELECT: &str = "SELECT
    runtime_session_id, connection_internal_id, session_source,
    connection_integration_revision, observed_host_executable_version,
    client_name, client_version, negotiated_protocol_version, process_id,
    process_started_at, initialize_completed_at, initialized_notification_at,
    tools_list_observed_at, required_tools_present,
    last_safe_read_only_tool_call_at, last_observed_at,
    terminal_protocol_failure_code, terminal_protocol_failure_details, graceful_close_at
  FROM mcp_runtime_sessions";

pub(crate) fn runtime_session_from_conn(
    conn: &Connection,
    runtime_session_id: &str,
) -> StoreResult<Option<McpRuntimeSessionRecord>> {
    conn.query_row(
        &format!("{RUNTIME_SESSION_SELECT} WHERE runtime_session_id = ?1"),
        [runtime_session_id],
        runtime_session_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|record| record.map(validate_runtime_session).transpose())
}

fn runtime_session_from_row(row: &Row<'_>) -> rusqlite::Result<McpRuntimeSessionRecord> {
    let source = match row.get::<_, String>(2)?.as_str() {
        "managed_host" => McpRuntimeSessionSource::ManagedHost,
        "cli_preflight" => McpRuntimeSessionSource::CliPreflight,
        value => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                value.to_owned().into(),
            ))
        }
    };
    let process_id = u32::try_from(row.get::<_, i64>(8)?).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            8,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })?;
    Ok(McpRuntimeSessionRecord {
        runtime_session_id: row.get(0)?,
        connection_internal_id: row.get(1)?,
        session_source: source,
        connection_integration_revision: row.get(3)?,
        observed_host_executable_version: row.get(4)?,
        client_name: row.get(5)?,
        client_version: row.get(6)?,
        negotiated_protocol_version: row.get(7)?,
        process_id,
        process_started_at: row.get(9)?,
        initialize_completed_at: row.get(10)?,
        initialized_notification_at: row.get(11)?,
        tools_list_observed_at: row.get(12)?,
        required_tools_present: row.get::<_, Option<i64>>(13)?.map(|value| value == 1),
        last_safe_read_only_tool_call_at: row.get(14)?,
        last_observed_at: row.get(15)?,
        terminal_protocol_failure_code: row.get(16)?,
        terminal_protocol_failure_details: row.get(17)?,
        graceful_close_at: row.get(18)?,
    })
}

fn validate_runtime_session(
    record: McpRuntimeSessionRecord,
) -> StoreResult<McpRuntimeSessionRecord> {
    IntegrationRevision::parse(record.connection_integration_revision.clone())
        .map_err(|_| corrupt(&record, "connection_integration_revision"))?;
    validate_timestamp("process_started_at", &record.process_started_at)
        .map_err(|_| corrupt(&record, "process_started_at"))?;
    validate_timestamp("last_observed_at", &record.last_observed_at)
        .map_err(|_| corrupt(&record, "last_observed_at"))?;
    Ok(record)
}

fn corrupt(record: &McpRuntimeSessionRecord, field: &'static str) -> StoreError {
    StoreError::CorruptOwnerStateValue {
        database_kind: "registry",
        table: "mcp_runtime_sessions",
        record_ref: record.runtime_session_id.clone(),
        logical_column: field,
    }
}

fn validate_start(input: &McpRuntimeSessionStart) -> StoreResult<()> {
    validate_text(
        "connection_internal_id",
        &input.connection_internal_id,
        MAX_DIAGNOSTIC_FIELD_BYTES,
    )?;
    if let Some(version) = input.observed_host_executable_version.as_deref() {
        validate_text(
            "observed_host_executable_version",
            version,
            MAX_DIAGNOSTIC_FIELD_BYTES,
        )?;
    }
    if input.process_id == 0 {
        return Err(StoreError::InvalidInput {
            detail: "process_id must be positive".to_owned(),
        });
    }
    validate_timestamp("process_started_at", &input.process_started_at)
}

fn validate_text(field: &'static str, value: &str, max: usize) -> StoreResult<()> {
    if value.is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must be 1 through {max} non-control UTF-8 bytes"),
        });
    }
    Ok(())
}

fn validate_timestamp(field: &'static str, value: &str) -> StoreResult<()> {
    UtcTimestamp::from_str(value)
        .map(|_| ())
        .map_err(|_| StoreError::InvalidInput {
            detail: format!("{field} must be an RFC 3339 timestamp"),
        })
}

fn require_observation_time(
    record: &McpRuntimeSessionRecord,
    observed_at: &str,
) -> StoreResult<()> {
    if observed_at < record.last_observed_at.as_str() {
        Err(milestone_order(
            &record.runtime_session_id,
            "milestone timestamp precedes the last observation",
        ))
    } else {
        Ok(())
    }
}

fn milestone_order(runtime_session_id: &str, detail: &str) -> StoreError {
    StoreError::Conflict {
        entity: "mcp_runtime_session",
        id: runtime_session_id.to_owned(),
        detail: detail.to_owned(),
    }
}

const fn bool_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}
