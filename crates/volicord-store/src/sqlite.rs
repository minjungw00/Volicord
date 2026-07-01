use std::{
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{
    config::DbConfig, Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior,
};

use crate::{
    migrations::{
        apply_project_state_migrations, apply_registry_migrations,
        expected_project_state_migrations, expected_registry_migrations,
        PROJECT_STATE_DATABASE_KIND, PROJECT_STATE_SCHEMA_VERSION, REGISTRY_DATABASE_KIND,
        REGISTRY_SCHEMA_VERSION, STORAGE_PROFILE,
    },
    StoreError, StoreResult,
};

/// Placement marker for SQLite-backed store code.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SqliteStoreBoundary;

/// Runtime Home registry database filename.
pub const REGISTRY_DB_FILE: &str = "registry.sqlite";

/// Runtime Home project directory name.
pub const PROJECTS_DIR: &str = "projects";

/// Project-local state database filename.
pub const PROJECT_STATE_DB_FILE: &str = "state.sqlite";

/// Project artifact directory name.
pub const ARTIFACTS_DIR: &str = "artifacts";

/// Project transient artifact staging directory name.
pub const ARTIFACTS_TMP_DIR: &str = "tmp";

/// Returns the `registry.sqlite` path for a Runtime Home.
pub fn registry_db_path(runtime_home: impl AsRef<Path>) -> PathBuf {
    runtime_home.as_ref().join(REGISTRY_DB_FILE)
}

/// Returns the project home path under a Runtime Home.
pub fn project_home_path(runtime_home: impl AsRef<Path>, project_id: impl AsRef<str>) -> PathBuf {
    runtime_home
        .as_ref()
        .join(PROJECTS_DIR)
        .join(project_id.as_ref())
}

/// Returns the project-local `state.sqlite` path under a Runtime Home.
pub fn project_state_db_path(
    runtime_home: impl AsRef<Path>,
    project_id: impl AsRef<str>,
) -> PathBuf {
    project_home_path(runtime_home, project_id).join(PROJECT_STATE_DB_FILE)
}

/// Returns the transient artifact staging directory path for a project.
pub fn artifacts_tmp_path(runtime_home: impl AsRef<Path>, project_id: impl AsRef<str>) -> PathBuf {
    project_home_path(runtime_home, project_id)
        .join(ARTIFACTS_DIR)
        .join(ARTIFACTS_TMP_DIR)
}

/// Opens `registry.sqlite`, creating the parent directory and baseline schema.
pub fn open_registry_database(path: impl AsRef<Path>) -> StoreResult<Connection> {
    let mut conn = open_sqlite_database(path)?;
    apply_registry_migrations(&mut conn)?;
    validate_registry_schema(&conn)?;
    Ok(conn)
}

/// Opens project `state.sqlite`, creating the parent directory and baseline schema.
pub fn open_project_state_database(path: impl AsRef<Path>) -> StoreResult<Connection> {
    let mut conn = open_sqlite_database(path)?;
    apply_project_state_migrations(&mut conn)?;
    validate_project_state_schema(&conn)?;
    Ok(conn)
}

/// Opens an existing SQLite database for inspection without creating or migrating it.
pub fn open_read_only_database(path: impl AsRef<Path>) -> StoreResult<Connection> {
    let conn = Connection::open_with_flags(
        path.as_ref(),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)?;
    conn.pragma_update(None, "query_only", "ON")?;
    Ok(conn)
}

/// Enables SQLite foreign-key enforcement for a connection.
pub fn enable_foreign_keys(conn: &Connection) -> rusqlite::Result<()> {
    set_foreign_keys(conn, true)
}

/// Sets SQLite foreign-key enforcement for a connection.
pub fn set_foreign_keys(conn: &Connection, enabled: bool) -> rusqlite::Result<()> {
    conn.pragma_update(None, "foreign_keys", if enabled { "ON" } else { "OFF" })
}

/// Returns whether SQLite foreign-key enforcement is enabled.
pub fn foreign_keys_enabled(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row("PRAGMA foreign_keys", [], |row| {
        Ok(row.get::<_, i64>(0)? == 1)
    })
}

/// Begins a mutating transaction with a serialized SQLite write boundary.
pub fn begin_immediate_transaction(conn: &mut Connection) -> rusqlite::Result<Transaction<'_>> {
    enable_foreign_keys(conn)?;
    conn.transaction_with_behavior(TransactionBehavior::Immediate)
}

/// Runs a closure inside `BEGIN IMMEDIATE` and commits it on success.
pub fn with_immediate_transaction<T>(
    conn: &mut Connection,
    work: impl FnOnce(&Transaction<'_>) -> rusqlite::Result<T>,
) -> rusqlite::Result<T> {
    let tx = begin_immediate_transaction(conn)?;
    let output = work(&tx)?;
    tx.commit()?;
    Ok(output)
}

/// Validates baseline registry schema invariants after migration.
pub fn validate_registry_schema(conn: &Connection) -> StoreResult<()> {
    validate_foreign_keys_enabled(conn, REGISTRY_DATABASE_KIND)?;
    validate_latest_migration(conn, REGISTRY_DATABASE_KIND, REGISTRY_SCHEMA_VERSION)?;
    validate_migration_history(
        conn,
        REGISTRY_DATABASE_KIND,
        &expected_registry_migrations(),
    )?;
    require_tables(
        conn,
        REGISTRY_DATABASE_KIND,
        &[
            "schema_migrations",
            "runtime_home",
            "installation_profile",
            "projects",
            "project_aliases",
            "agent_connections",
            "connection_projects",
            "guard_installations",
            "local_web_consent_tokens",
        ],
    )?;
    require_indexes(
        conn,
        REGISTRY_DATABASE_KIND,
        &[
            "idx_projects_repo_root",
            "idx_projects_status",
            "idx_project_aliases_project",
            "idx_connection_projects_project",
            "idx_agent_connections_enabled",
            "idx_agent_connections_project",
            "idx_agent_connections_target_project",
            "idx_agent_connections_target_global",
            "idx_guard_installations_connection",
            "idx_guard_installations_project",
            "idx_guard_installations_status",
            "idx_guard_installations_scope_project",
            "idx_guard_installations_scope_global",
            "idx_local_web_consent_tokens_judgment",
            "idx_local_web_consent_tokens_connection",
            "idx_local_web_consent_tokens_expiry",
        ],
    )?;
    require_column_spec(
        conn,
        REGISTRY_DATABASE_KIND,
        "runtime_home",
        ColumnSpec {
            name: "runtime_home_path",
            type_name: "TEXT",
            not_null: true,
            default_value: None,
            primary_key_position: 0,
        },
    )?;
    require_column_spec(
        conn,
        REGISTRY_DATABASE_KIND,
        "runtime_home",
        ColumnSpec {
            name: "registry_db_path",
            type_name: "TEXT",
            not_null: true,
            default_value: None,
            primary_key_position: 0,
        },
    )?;
    require_column_spec(
        conn,
        REGISTRY_DATABASE_KIND,
        "projects",
        ColumnSpec {
            name: "project_internal_id",
            type_name: "TEXT",
            not_null: false,
            default_value: None,
            primary_key_position: 1,
        },
    )?;
    require_column_spec(
        conn,
        REGISTRY_DATABASE_KIND,
        "project_aliases",
        ColumnSpec {
            name: "alias",
            type_name: "TEXT",
            not_null: false,
            default_value: None,
            primary_key_position: 1,
        },
    )?;
    require_column_spec(
        conn,
        REGISTRY_DATABASE_KIND,
        "agent_connections",
        ColumnSpec {
            name: "connection_internal_id",
            type_name: "TEXT",
            not_null: false,
            default_value: None,
            primary_key_position: 1,
        },
    )?;
    require_column_spec(
        conn,
        REGISTRY_DATABASE_KIND,
        "agent_connections",
        ColumnSpec {
            name: "intent",
            type_name: "TEXT",
            not_null: true,
            default_value: None,
            primary_key_position: 0,
        },
    )?;
    require_column_spec(
        conn,
        REGISTRY_DATABASE_KIND,
        "agent_connections",
        ColumnSpec {
            name: "enabled",
            type_name: "INTEGER",
            not_null: true,
            default_value: Some("1"),
            primary_key_position: 0,
        },
    )?;
    require_column_spec(
        conn,
        REGISTRY_DATABASE_KIND,
        "agent_connections",
        ColumnSpec {
            name: "metadata_json",
            type_name: "TEXT",
            not_null: true,
            default_value: Some("'{}'"),
            primary_key_position: 0,
        },
    )?;
    require_column_spec(
        conn,
        REGISTRY_DATABASE_KIND,
        "agent_connections",
        ColumnSpec {
            name: "mode",
            type_name: "TEXT",
            not_null: true,
            default_value: None,
            primary_key_position: 0,
        },
    )?;
    require_column_spec(
        conn,
        REGISTRY_DATABASE_KIND,
        "connection_projects",
        ColumnSpec {
            name: "connection_internal_id",
            type_name: "TEXT",
            not_null: true,
            default_value: None,
            primary_key_position: 1,
        },
    )?;
    require_column_spec(
        conn,
        REGISTRY_DATABASE_KIND,
        "connection_projects",
        ColumnSpec {
            name: "project_internal_id",
            type_name: "TEXT",
            not_null: true,
            default_value: None,
            primary_key_position: 2,
        },
    )?;
    require_column_spec(
        conn,
        REGISTRY_DATABASE_KIND,
        "agent_connections",
        ColumnSpec {
            name: "last_verification_status",
            type_name: "TEXT",
            not_null: true,
            default_value: Some("'not_verified'"),
            primary_key_position: 0,
        },
    )?;
    require_column_spec(
        conn,
        REGISTRY_DATABASE_KIND,
        "agent_connections",
        ColumnSpec {
            name: "last_verification_report_json",
            type_name: "TEXT",
            not_null: true,
            default_value: Some("'{}'"),
            primary_key_position: 0,
        },
    )?;
    require_column_spec(
        conn,
        REGISTRY_DATABASE_KIND,
        "agent_connections",
        ColumnSpec {
            name: "last_user_actions_json",
            type_name: "TEXT",
            not_null: true,
            default_value: Some("'[]'"),
            primary_key_position: 0,
        },
    )?;
    for column in [
        "guard_installation_id",
        "runtime_home_id",
        "connection_internal_id",
        "project_internal_id",
        "host_kind",
        "guard_mode",
        "host_capability_json",
        "installation_status",
        "installed_at",
        "last_checked_at",
        "first_seen_at",
        "last_seen_at",
        "last_seen_phase",
        "observed_host_kind",
        "observed_policy_hash",
        "observed_binary_version",
        "metadata_json",
        "created_at",
        "updated_at",
    ] {
        require_column(conn, REGISTRY_DATABASE_KIND, "guard_installations", column)?;
    }
    validate_guard_installations_constraints(conn)?;
    validate_local_web_consent_tokens(conn)?;
    validate_registry_versions(conn)?;
    validate_foreign_key_check(conn, REGISTRY_DATABASE_KIND)?;
    Ok(())
}

/// Validates baseline project-state schema invariants after migration.
pub fn validate_project_state_schema(conn: &Connection) -> StoreResult<()> {
    validate_foreign_keys_enabled(conn, PROJECT_STATE_DATABASE_KIND)?;
    validate_latest_migration(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        PROJECT_STATE_SCHEMA_VERSION,
    )?;
    validate_migration_history(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        &expected_project_state_migrations(),
    )?;
    require_tables(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        &[
            "schema_migrations",
            "project_state",
            "tasks",
            "change_units",
            "user_judgments",
            "project_continuity_records",
            "write_checks",
            "runs",
            "artifact_staging",
            "artifacts",
            "artifact_links",
            "evidence_summaries",
            "evidence_observations",
            "blockers",
            "task_events",
            "tool_invocations",
            "agent_sessions",
            "guard_events",
            "prompt_captures",
            "expected_writes",
            "unrecorded_changes",
            "session_watch_baselines",
            "session_watch_observations",
        ],
    )?;
    require_indexes(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        &[
            "idx_change_units_one_current_active",
            "idx_write_checks_consumed_run",
            "idx_runs_write_check",
            "idx_artifact_staging_promoted_artifact",
            "idx_artifacts_source_staging",
            "idx_project_state_active_task",
            "idx_tasks_lifecycle",
            "idx_tasks_current_change_unit",
            "idx_change_units_task_status",
            "idx_user_judgments_task_status",
            "idx_project_continuity_records_status",
            "idx_project_continuity_records_source_task",
            "idx_write_checks_task_status",
            "idx_runs_task_created",
            "idx_artifact_staging_task_status",
            "idx_artifact_staging_actor_source",
            "idx_artifacts_task_status",
            "idx_artifact_links_owner",
            "idx_evidence_summaries_task_status",
            "idx_evidence_observations_task_claim",
            "idx_evidence_observations_run",
            "idx_blockers_task_status",
            "idx_task_events_task_seq",
            "idx_agent_sessions_connection",
            "idx_agent_sessions_open",
            "idx_guard_events_session",
            "idx_guard_events_connection",
            "idx_guard_events_decision",
            "idx_prompt_captures_session",
            "idx_prompt_captures_connection",
            "idx_expected_writes_pending_connection",
            "idx_expected_writes_session",
            "idx_expected_writes_host_invocation",
            "idx_expected_writes_task",
            "idx_unrecorded_changes_status",
            "idx_unrecorded_changes_connection",
            "idx_unrecorded_changes_task",
            "idx_session_watch_baselines_session",
            "idx_session_watch_baselines_status",
            "idx_session_watch_observations_unresolved",
            "idx_session_watch_observations_baseline",
            "idx_session_watch_observations_expected_write",
            "idx_session_watch_observations_unrecorded_change",
        ],
    )?;
    require_column(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "project_state",
        "state_version",
    )?;
    require_column_spec(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "project_state",
        ColumnSpec {
            name: "enforcement_profile_json",
            type_name: "TEXT",
            not_null: true,
            default_value: Some("'{\"profile_id\":\"baseline_cooperative\",\"guarantee_level\":\"cooperative\",\"enabled_mechanisms\":[],\"source\":\"baseline_scope\",\"status\":\"active\"}'"),
            primary_key_position: 0,
        },
    )?;
    require_column_spec(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "tasks",
        ColumnSpec {
            name: "scope_revision",
            type_name: "INTEGER",
            not_null: true,
            default_value: Some("0"),
            primary_key_position: 0,
        },
    )?;
    reject_column(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "change_units",
        "close_basis_json",
    )?;
    require_column_spec(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "tasks",
        ColumnSpec {
            name: "close_basis_revision",
            type_name: "INTEGER",
            not_null: true,
            default_value: Some("0"),
            primary_key_position: 0,
        },
    )?;
    require_column_spec(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "tasks",
        ColumnSpec {
            name: "close_basis_json",
            type_name: "TEXT",
            not_null: false,
            default_value: None,
            primary_key_position: 0,
        },
    )?;
    require_column_spec(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "runs",
        ColumnSpec {
            name: "scope_revision",
            type_name: "INTEGER",
            not_null: true,
            default_value: None,
            primary_key_position: 0,
        },
    )?;
    require_column_spec(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "user_judgments",
        ColumnSpec {
            name: "basis_json",
            type_name: "TEXT",
            not_null: true,
            default_value: None,
            primary_key_position: 0,
        },
    )?;
    require_column_spec(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "user_judgments",
        ColumnSpec {
            name: "basis_status",
            type_name: "TEXT",
            not_null: true,
            default_value: Some("'current'"),
            primary_key_position: 0,
        },
    )?;
    validate_user_judgments_basis_status_constraint(conn)?;
    require_column_spec(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "user_judgments",
        ColumnSpec {
            name: "status",
            type_name: "TEXT",
            not_null: true,
            default_value: None,
            primary_key_position: 0,
        },
    )?;
    validate_user_judgments_status_constraint(conn)?;
    require_column_spec(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "user_judgments",
        ColumnSpec {
            name: "resolution_outcome",
            type_name: "TEXT",
            not_null: false,
            default_value: None,
            primary_key_position: 0,
        },
    )?;
    validate_user_judgments_resolution_outcome_constraint(conn)?;
    require_column_spec(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "user_judgments",
        ColumnSpec {
            name: "resolution_machine_action",
            type_name: "TEXT",
            not_null: false,
            default_value: None,
            primary_key_position: 0,
        },
    )?;
    validate_user_judgments_resolution_machine_action_constraint(conn)?;
    validate_user_judgments_resolution_group_constraint(conn)?;
    for column in [
        "requested_by_actor_source",
        "resolved_by_actor_source",
        "resolved_verification_basis",
        "resolved_assurance_level",
    ] {
        require_column(conn, PROJECT_STATE_DATABASE_KIND, "user_judgments", column)?;
    }
    require_column_spec(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "project_continuity_records",
        ColumnSpec {
            name: "continuity_record_id",
            type_name: "TEXT",
            not_null: true,
            default_value: None,
            primary_key_position: 2,
        },
    )?;
    require_column_spec(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "project_continuity_records",
        ColumnSpec {
            name: "kind",
            type_name: "TEXT",
            not_null: true,
            default_value: None,
            primary_key_position: 0,
        },
    )?;
    require_column_spec(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "project_continuity_records",
        ColumnSpec {
            name: "status",
            type_name: "TEXT",
            not_null: true,
            default_value: None,
            primary_key_position: 0,
        },
    )?;
    for column in [
        "source_task_id",
        "source_change_unit_id",
        "title",
        "summary",
        "rationale",
        "applies_to_paths_json",
        "applies_to_refs_json",
        "source_refs_json",
        "artifact_refs_json",
        "supersedes_refs_json",
        "review_triggers_json",
        "created_at",
        "updated_at",
        "metadata_json",
    ] {
        require_column(
            conn,
            PROJECT_STATE_DATABASE_KIND,
            "project_continuity_records",
            column,
        )?;
    }
    validate_project_continuity_records_constraints(conn)?;
    reject_column(conn, PROJECT_STATE_DATABASE_KIND, "tasks", "state_version")?;
    require_column(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "tool_invocations",
        "request_hash",
    )?;
    for column in ["actor_source", "operation_category"] {
        require_column_spec(
            conn,
            PROJECT_STATE_DATABASE_KIND,
            "tool_invocations",
            ColumnSpec {
                name: column,
                type_name: "TEXT",
                not_null: true,
                default_value: None,
                primary_key_position: 0,
            },
        )?;
    }
    require_column(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "tool_invocations",
        "verification_basis",
    )?;
    validate_tool_invocations_columns(conn)?;
    validate_tool_invocations_primary_key(conn)?;
    validate_tool_invocations_operation_category_constraint(conn)?;
    require_column_spec(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "artifacts",
        ColumnSpec {
            name: "integrity_status",
            type_name: "TEXT",
            not_null: true,
            default_value: Some("'verified'"),
            primary_key_position: 0,
        },
    )?;
    validate_artifacts_integrity_status_constraint(conn)?;
    validate_artifacts_body_path_constraint(conn)?;
    validate_guard_project_record_tables(conn)?;
    validate_project_state_versions(conn)?;
    validate_foreign_key_check(conn, PROJECT_STATE_DATABASE_KIND)?;
    Ok(())
}

fn open_sqlite_database(path: impl AsRef<Path>) -> StoreResult<Connection> {
    let path = path.as_ref();
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(path)?;
    enable_foreign_keys(&conn)?;
    Ok(conn)
}

fn validate_foreign_keys_enabled(
    conn: &Connection,
    database_kind: &'static str,
) -> StoreResult<()> {
    if foreign_keys_enabled(conn)? {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            database_kind,
            "PRAGMA foreign_keys is not enabled",
        ))
    }
}

fn require_tables(
    conn: &Connection,
    database_kind: &'static str,
    names: &[&str],
) -> StoreResult<()> {
    for name in names {
        if !sqlite_object_exists(conn, "table", name)? {
            return Err(StoreError::schema_invariant(
                database_kind,
                format!("missing table {name}"),
            ));
        }
    }

    Ok(())
}

fn require_indexes(
    conn: &Connection,
    database_kind: &'static str,
    names: &[&str],
) -> StoreResult<()> {
    for name in names {
        if !sqlite_object_exists(conn, "index", name)? {
            return Err(StoreError::schema_invariant(
                database_kind,
                format!("missing index {name}"),
            ));
        }
    }

    Ok(())
}

fn sqlite_object_exists(
    conn: &Connection,
    object_type: &str,
    name: &str,
) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT COUNT(*)
           FROM sqlite_master
          WHERE type = ?1 AND name = ?2",
        [object_type, name],
        |row| Ok(row.get::<_, i64>(0)? > 0),
    )
}

fn validate_latest_migration(
    conn: &Connection,
    database_kind: &'static str,
    latest_version: i64,
) -> StoreResult<()> {
    let version = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0)
               FROM schema_migrations
              WHERE database_kind = ?1",
            [database_kind],
            |row| row.get::<_, i64>(0),
        )
        .map_err(StoreError::from)?;
    if version == latest_version {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            database_kind,
            format!("latest migration is {version}, expected {latest_version}"),
        ))
    }
}

fn validate_migration_history(
    conn: &Connection,
    database_kind: &'static str,
    expected: &[crate::migrations::ExpectedMigration],
) -> StoreResult<()> {
    let mut stmt = conn.prepare(
        "SELECT version, name, storage_profile
           FROM schema_migrations
          WHERE database_kind = ?1
          ORDER BY version",
    )?;
    let rows = stmt.query_map([database_kind], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut actual = Vec::new();
    for row in rows {
        actual.push(row?);
    }

    if actual.len() != expected.len() {
        return Err(StoreError::schema_invariant(
            database_kind,
            format!(
                "migration history has {} rows, expected {}",
                actual.len(),
                expected.len()
            ),
        ));
    }

    for (index, (actual_version, actual_name, actual_profile)) in actual.iter().enumerate() {
        let expected_row = expected[index];
        if expected_row.database_kind != database_kind
            || *actual_version != expected_row.version
            || actual_name != expected_row.name
            || actual_profile != STORAGE_PROFILE
        {
            return Err(StoreError::schema_invariant(
                database_kind,
                format!(
                    "migration row {index} is version={actual_version} name={actual_name} profile={actual_profile}, expected version={} name={} profile={}",
                    expected_row.version,
                    expected_row.name,
                    STORAGE_PROFILE
                ),
            ));
        }
    }

    Ok(())
}

fn validate_project_state_versions(conn: &Connection) -> StoreResult<()> {
    if let Some(actual_storage_profile) = conn
        .query_row(
            "SELECT storage_profile
               FROM project_state
              WHERE storage_profile != ?1
              LIMIT 1",
            [STORAGE_PROFILE],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        return Err(StoreError::unsupported_storage_profile(
            PROJECT_STATE_DATABASE_KIND,
            actual_storage_profile,
            STORAGE_PROFILE,
        ));
    }

    let stale_count: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM project_state
          WHERE schema_version != ?1",
        [PROJECT_STATE_SCHEMA_VERSION],
        |row| row.get(0),
    )?;
    if stale_count == 0 {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "project_state.schema_version does not match the latest applied migration",
        ))
    }
}

fn validate_registry_versions(conn: &Connection) -> StoreResult<()> {
    if let Some(actual_storage_profile) = conn
        .query_row(
            "SELECT storage_profile
               FROM runtime_home
              WHERE storage_profile != ?1
              LIMIT 1",
            [STORAGE_PROFILE],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        return Err(StoreError::unsupported_storage_profile(
            REGISTRY_DATABASE_KIND,
            actual_storage_profile,
            STORAGE_PROFILE,
        ));
    }

    let stale_count: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM runtime_home
          WHERE schema_version != ?1",
        [REGISTRY_SCHEMA_VERSION],
        |row| row.get(0),
    )?;
    if stale_count == 0 {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            REGISTRY_DATABASE_KIND,
            "runtime_home.schema_version does not match the latest applied migration",
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ColumnSpec {
    name: &'static str,
    type_name: &'static str,
    not_null: bool,
    default_value: Option<&'static str>,
    primary_key_position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ColumnInfo {
    type_name: String,
    not_null: bool,
    default_value: Option<String>,
    primary_key_position: i64,
}

fn require_column_spec(
    conn: &Connection,
    database_kind: &'static str,
    table: &str,
    expected: ColumnSpec,
) -> StoreResult<()> {
    let info = column_info(conn, table, expected.name)?.ok_or_else(|| {
        StoreError::schema_invariant(
            database_kind,
            format!("missing column {table}.{}", expected.name),
        )
    })?;

    if info.type_name.eq_ignore_ascii_case(expected.type_name)
        && info.not_null == expected.not_null
        && info.default_value.as_deref() == expected.default_value
        && info.primary_key_position == expected.primary_key_position
    {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            database_kind,
            format!(
                "column {table}.{} has type={} not_null={} default={:?} pk={}, expected type={} not_null={} default={:?} pk={}",
                expected.name,
                info.type_name,
                info.not_null,
                info.default_value,
                info.primary_key_position,
                expected.type_name,
                expected.not_null,
                expected.default_value,
                expected.primary_key_position
            ),
        ))
    }
}

fn column_info(
    conn: &Connection,
    table: &str,
    column: &str,
) -> rusqlite::Result<Option<ColumnInfo>> {
    let escaped_table = table.replace('"', "\"\"");
    let sql = format!("PRAGMA table_info(\"{escaped_table}\")");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(Some(ColumnInfo {
                type_name: row.get(2)?,
                not_null: row.get::<_, i64>(3)? != 0,
                default_value: row.get(4)?,
                primary_key_position: row.get(5)?,
            }));
        }
    }

    Ok(None)
}

fn table_column_names(conn: &Connection, table: &str) -> rusqlite::Result<Vec<String>> {
    let escaped_table = table.replace('"', "\"\"");
    let sql = format!("PRAGMA table_info(\"{escaped_table}\")");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut columns = Vec::new();
    for row in rows {
        columns.push(row?);
    }
    columns.sort_by_key(|(position, _)| *position);
    Ok(columns.into_iter().map(|(_, name)| name).collect())
}

fn validate_user_judgments_basis_status_constraint(conn: &Connection) -> StoreResult<()> {
    let table_sql: String = conn.query_row(
        "SELECT sql
           FROM sqlite_master
          WHERE type = 'table'
            AND name = 'user_judgments'",
        [],
        |row| row.get(0),
    )?;
    let normalized = table_sql
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let has_constraint = normalized.contains("basis_status in ('current', 'stale', 'superseded')")
        || normalized.contains("basis_status in('current', 'stale', 'superseded')");
    if has_constraint {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "user_judgments.basis_status constraint is missing or malformed",
        ))
    }
}

fn validate_user_judgments_status_constraint(conn: &Connection) -> StoreResult<()> {
    let table_sql = normalized_table_sql(conn, "user_judgments")?;
    let has_constraint = table_sql
        .contains("status in ('pending', 'resolved', 'stale', 'superseded', 'expired')")
        || table_sql.contains("status in('pending', 'resolved', 'stale', 'superseded', 'expired')");
    if has_constraint {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "user_judgments.status constraint is missing or malformed",
        ))
    }
}

fn validate_user_judgments_resolution_outcome_constraint(conn: &Connection) -> StoreResult<()> {
    let table_sql: String = conn.query_row(
        "SELECT sql
           FROM sqlite_master
          WHERE type = 'table'
            AND name = 'user_judgments'",
        [],
        |row| row.get(0),
    )?;
    let normalized = table_sql
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let has_constraint = normalized.contains(
        "resolution_outcome is null or resolution_outcome in ('accepted', 'rejected', 'deferred')",
    ) || normalized.contains(
        "resolution_outcome is null or resolution_outcome in('accepted', 'rejected', 'deferred')",
    );
    if has_constraint {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "user_judgments.resolution_outcome constraint is missing or malformed",
        ))
    }
}

fn validate_user_judgments_resolution_machine_action_constraint(
    conn: &Connection,
) -> StoreResult<()> {
    let table_sql = normalized_table_sql(conn, "user_judgments")?;
    let has_constraint = table_sql.contains(
        "resolution_machine_action is null or resolution_machine_action in ('accept', 'reject', 'defer')",
    ) || table_sql.contains(
        "resolution_machine_action is null or resolution_machine_action in('accept', 'reject', 'defer')",
    );
    if has_constraint {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "user_judgments.resolution_machine_action constraint is missing or malformed",
        ))
    }
}

fn validate_user_judgments_resolution_group_constraint(conn: &Connection) -> StoreResult<()> {
    let table_sql = normalized_table_sql(conn, "user_judgments")?;
    let has_resolved_requirement = table_sql.contains("status = 'resolved'")
        && table_sql.contains("resolution_outcome is not null")
        && table_sql.contains("resolution_machine_action is not null")
        && table_sql.contains("resolution_json is not null")
        && table_sql.contains("resolved_by_actor_source is not null")
        && table_sql.contains("resolved_verification_basis is not null")
        && table_sql.contains("resolved_assurance_level is not null")
        && table_sql.contains("resolved_at is not null");
    let has_unresolved_requirement = table_sql.contains("status in ('pending', 'expired')")
        && table_sql.contains("resolution_outcome is null")
        && table_sql.contains("resolution_machine_action is null")
        && table_sql.contains("resolution_json is null")
        && table_sql.contains("resolved_by_actor_source is null")
        && table_sql.contains("resolved_verification_basis is null")
        && table_sql.contains("resolved_assurance_level is null")
        && table_sql.contains("resolved_at is null");
    if has_resolved_requirement && has_unresolved_requirement {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "user_judgments resolution completeness constraint is missing or malformed",
        ))
    }
}

fn validate_project_continuity_records_constraints(conn: &Connection) -> StoreResult<()> {
    let table_sql = normalized_table_sql(conn, "project_continuity_records")?;
    let required_fragments = [
        "kind in ('decision', 'obligation', 'known_limit', 'accepted_risk', 'constraint')",
        "length(trim(title)) > 0",
        "length(trim(summary)) > 0",
        "rationale is null or length(trim(rationale)) > 0",
        "status in ('active', 'superseded', 'closed')",
    ];
    for fragment in required_fragments {
        if !table_sql.contains(fragment) {
            return Err(StoreError::schema_invariant(
                PROJECT_STATE_DATABASE_KIND,
                "project_continuity_records constraints are missing or malformed",
            ));
        }
    }
    Ok(())
}

fn normalized_table_sql(conn: &Connection, table: &str) -> StoreResult<String> {
    let table_sql: String = conn.query_row(
        "SELECT sql
           FROM sqlite_master
          WHERE type = 'table'
            AND name = ?1",
        [table],
        |row| row.get(0),
    )?;
    Ok(table_sql
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase())
}

fn validate_tool_invocations_primary_key(conn: &Connection) -> StoreResult<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(tool_invocations)")?;
    let mut rows = stmt.query([])?;
    let mut primary_key_columns = Vec::new();

    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        let primary_key_position: i64 = row.get(5)?;
        if primary_key_position > 0 {
            primary_key_columns.push((primary_key_position, name));
        }
    }

    primary_key_columns.sort_by_key(|(position, _)| *position);
    let primary_key_columns = primary_key_columns
        .into_iter()
        .map(|(_, name)| name)
        .collect::<Vec<_>>();
    let expected = vec![
        "project_id".to_owned(),
        "tool_name".to_owned(),
        "idempotency_key".to_owned(),
    ];
    if primary_key_columns == expected {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            format!(
                "tool_invocations primary key is {:?}, expected {:?}",
                primary_key_columns, expected
            ),
        ))
    }
}

fn validate_tool_invocations_columns(conn: &Connection) -> StoreResult<()> {
    let actual = table_column_names(conn, "tool_invocations")?;
    let expected = [
        "project_id",
        "tool_name",
        "idempotency_key",
        "request_hash",
        "basis_state_version",
        "committed_state_version",
        "status",
        "actor_source",
        "operation_category",
        "verification_basis",
        "response_json",
        "created_at",
    ]
    .iter()
    .map(|name| (*name).to_owned())
    .collect::<Vec<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            format!(
                "tool_invocations columns are {:?}, expected {:?}",
                actual, expected
            ),
        ))
    }
}

fn validate_tool_invocations_operation_category_constraint(conn: &Connection) -> StoreResult<()> {
    let table_sql = normalized_table_sql(conn, "tool_invocations")?;
    let has_constraint = table_sql
        .contains(
            "operation_category in ('read', 'agent_workflow', 'user_only', 'admin_local', 'local_recovery')",
        )
        || table_sql.contains(
            "operation_category in('read', 'agent_workflow', 'user_only', 'admin_local', 'local_recovery')",
        );
    if has_constraint {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "tool_invocations.operation_category constraint is missing or malformed",
        ))
    }
}

fn validate_guard_installations_constraints(conn: &Connection) -> StoreResult<()> {
    let table_sql = normalized_table_sql(conn, "guard_installations")?;
    let required_fragments = [
        "length(trim(host_kind)) > 0",
        "guard_mode in ('mcp_only', 'guarded', 'managed')",
    ];
    for fragment in required_fragments {
        if !table_sql.contains(fragment) {
            return Err(StoreError::schema_invariant(
                REGISTRY_DATABASE_KIND,
                "guard_installations constraints are missing or malformed",
            ));
        }
    }
    let has_status_constraint = [
        "installation_status in ('absent', 'configured', 'reload_required', 'active', 'degraded', 'stale', 'broken')",
        "installation_status in ( 'absent', 'configured', 'reload_required', 'active', 'degraded', 'stale', 'broken' )",
    ]
    .iter()
    .any(|fragment| table_sql.contains(fragment));
    if !has_status_constraint {
        return Err(StoreError::schema_invariant(
            REGISTRY_DATABASE_KIND,
            "guard_installations constraints are missing or malformed",
        ));
    }
    Ok(())
}

fn validate_local_web_consent_tokens(conn: &Connection) -> StoreResult<()> {
    for (column, not_null) in [
        ("token_hash", true),
        ("project_internal_id", true),
        ("connection_internal_id", true),
        ("judgment_id", true),
        ("capture_basis", true),
        ("status", true),
        ("created_at", true),
        ("expires_at", true),
        ("consumed_at", false),
        ("completed_at", false),
        ("created_metadata_json", true),
        ("completion_metadata_json", true),
    ] {
        require_column_spec(
            conn,
            REGISTRY_DATABASE_KIND,
            "local_web_consent_tokens",
            ColumnSpec {
                name: column,
                type_name: "TEXT",
                not_null,
                default_value: match column {
                    "status" => Some("'pending'"),
                    "created_metadata_json" | "completion_metadata_json" => Some("'{}'"),
                    _ => None,
                },
                primary_key_position: if column == "token_hash" { 1 } else { 0 },
            },
        )?;
    }

    let table_sql = normalized_table_sql(conn, "local_web_consent_tokens")?;
    let required_fragments = [
        "length(token_hash) = 64",
        "status in ('pending', 'consumed', 'expired')",
        "status = 'pending'",
        "status = 'consumed'",
        "status = 'expired'",
        "consumed_at is null",
        "consumed_at is not null",
        "completed_at is null",
        "completed_at is not null",
    ];
    for fragment in required_fragments {
        if !table_sql.contains(fragment) {
            return Err(StoreError::schema_invariant(
                REGISTRY_DATABASE_KIND,
                "local_web_consent_tokens constraints are missing or malformed",
            ));
        }
    }
    Ok(())
}

fn validate_guard_project_record_tables(conn: &Connection) -> StoreResult<()> {
    for (table, columns) in [
        (
            "agent_sessions",
            &[
                "project_id",
                "session_id",
                "connection_internal_id",
                "guard_installation_id",
                "host_kind",
                "guard_mode",
                "started_at",
                "ended_at",
                "metadata_json",
            ][..],
        ),
        (
            "guard_events",
            &[
                "project_id",
                "guard_event_id",
                "session_id",
                "connection_internal_id",
                "guard_installation_id",
                "event_kind",
                "decision",
                "subject_json",
                "result_json",
                "occurred_at",
                "metadata_json",
            ][..],
        ),
        (
            "prompt_captures",
            &[
                "project_id",
                "prompt_capture_id",
                "session_id",
                "connection_internal_id",
                "capture_kind",
                "prompt_sha256",
                "prompt_text",
                "captured_at",
                "metadata_json",
            ][..],
        ),
        (
            "expected_writes",
            &[
                "project_id",
                "expected_write_id",
                "session_id",
                "connection_internal_id",
                "guard_installation_id",
                "pre_tool_guard_event_id",
                "host_invocation_id",
                "tool_name",
                "command_kind",
                "path_policy",
                "expected_paths_json",
                "task_id",
                "change_unit_id",
                "write_check_ids_json",
                "basis_state_version",
                "status",
                "matched_post_tool_guard_event_id",
                "matched_paths_json",
                "created_at",
                "expires_at",
                "matched_at",
                "metadata_json",
            ][..],
        ),
        (
            "unrecorded_changes",
            &[
                "project_id",
                "unrecorded_change_id",
                "session_id",
                "connection_internal_id",
                "task_id",
                "status",
                "summary",
                "observed_paths_json",
                "detection_json",
                "resolution_json",
                "detected_at",
                "resolved_at",
                "resolved_by_actor_source",
                "metadata_json",
            ][..],
        ),
        (
            "session_watch_baselines",
            &[
                "project_id",
                "watch_baseline_id",
                "session_id",
                "connection_internal_id",
                "guard_installation_id",
                "status",
                "scope_kind",
                "repo_root",
                "watched_paths_json",
                "exclusions_json",
                "snapshot_algorithm",
                "snapshot_digest",
                "snapshot_entries_json",
                "created_at",
                "updated_at",
                "metadata_json",
            ][..],
        ),
        (
            "session_watch_observations",
            &[
                "project_id",
                "watch_observation_id",
                "watch_baseline_id",
                "session_id",
                "connection_internal_id",
                "expected_write_id",
                "unrecorded_change_id",
                "observation_status",
                "observed_paths_json",
                "change_summary_json",
                "snapshot_algorithm",
                "snapshot_digest",
                "snapshot_entries_json",
                "observed_at",
                "linked_at",
                "metadata_json",
            ][..],
        ),
    ] {
        for column in columns {
            require_column(conn, PROJECT_STATE_DATABASE_KIND, table, column)?;
        }
    }

    let expected_writes_sql = normalized_table_sql(conn, "expected_writes")?;
    let has_status_values = expected_writes_sql.contains("status in ('pending', 'matched')");
    let has_path_policy = expected_writes_sql.contains("path_policy in ('exact_paths')");
    let has_pending_group = expected_writes_sql.contains("status = 'pending'")
        && expected_writes_sql.contains("matched_post_tool_guard_event_id is null")
        && expected_writes_sql.contains("matched_paths_json is null")
        && expected_writes_sql.contains("matched_at is null");
    let has_matched_group = expected_writes_sql.contains("status = 'matched'")
        && expected_writes_sql.contains("matched_post_tool_guard_event_id is not null")
        && expected_writes_sql.contains("matched_paths_json is not null")
        && expected_writes_sql.contains("matched_at is not null");
    if !has_status_values || !has_path_policy || !has_pending_group || !has_matched_group {
        return Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "expected_writes constraints are missing or malformed",
        ));
    }

    let sessions_sql = normalized_table_sql(conn, "agent_sessions")?;
    if !sessions_sql.contains("guard_mode in ('mcp_only', 'guarded', 'managed')")
        || !sessions_sql.contains("length(trim(host_kind)) > 0")
    {
        return Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "agent_sessions constraints are missing or malformed",
        ));
    }

    let guard_events_sql = normalized_table_sql(conn, "guard_events")?;
    if !guard_events_sql.contains("decision in ('allow', 'deny', 'warn', 'inject_context')") {
        return Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "guard_events.decision constraint is missing or malformed",
        ));
    }

    let unrecorded_changes_sql = normalized_table_sql(conn, "unrecorded_changes")?;
    let has_status_values = unrecorded_changes_sql.contains("status in ('unresolved', 'resolved')");
    let has_unresolved_group = unrecorded_changes_sql.contains("status = 'unresolved'")
        && unrecorded_changes_sql.contains("resolution_json is null")
        && unrecorded_changes_sql.contains("resolved_at is null")
        && unrecorded_changes_sql.contains("resolved_by_actor_source is null");
    let has_resolved_group = unrecorded_changes_sql.contains("status = 'resolved'")
        && unrecorded_changes_sql.contains("resolution_json is not null")
        && unrecorded_changes_sql.contains("resolved_at is not null")
        && unrecorded_changes_sql.contains("resolved_by_actor_source is not null");
    if !has_status_values || !has_unresolved_group || !has_resolved_group {
        return Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "unrecorded_changes resolution constraints are missing or malformed",
        ));
    }

    let baselines_sql = normalized_table_sql(conn, "session_watch_baselines")?;
    if !baselines_sql.contains("status in ('disabled', 'active', 'degraded', 'unavailable')")
        || !baselines_sql.contains("scope_kind in ('repository', 'path_set')")
    {
        return Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "session_watch_baselines constraints are missing or malformed",
        ));
    }

    let observations_sql = normalized_table_sql(conn, "session_watch_observations")?;
    let has_status_values =
        observations_sql.contains("observation_status in ('unresolved', 'linked')");
    let has_unresolved_group = observations_sql.contains("observation_status = 'unresolved'")
        && observations_sql.contains("unrecorded_change_id is null")
        && observations_sql.contains("linked_at is null");
    let has_linked_group = observations_sql.contains("observation_status = 'linked'")
        && observations_sql.contains("unrecorded_change_id is not null")
        && observations_sql.contains("linked_at is not null");
    if !has_status_values || !has_unresolved_group || !has_linked_group {
        return Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "session_watch_observations link constraints are missing or malformed",
        ));
    }

    Ok(())
}

fn validate_artifacts_integrity_status_constraint(conn: &Connection) -> StoreResult<()> {
    let table_sql: String = conn.query_row(
        "SELECT sql
           FROM sqlite_master
          WHERE type = 'table'
            AND name = 'artifacts'",
        [],
        |row| row.get(0),
    )?;
    let normalized = table_sql
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let has_status_values = normalized.contains("integrity_status in ('verified', 'corrupt')")
        || normalized.contains("integrity_status in('verified', 'corrupt')");
    let has_verified_requirement = normalized.contains("integrity_status <> 'verified'")
        && normalized.contains("length(sha256) = 64")
        && normalized.contains("sha256 not glob '*[^0-9a-f]*'")
        && normalized.contains("size_bytes is not null")
        && normalized.contains("content_type is not null");
    if has_status_values && has_verified_requirement {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "artifacts.integrity_status constraint is missing or malformed",
        ))
    }
}

fn validate_artifacts_body_path_constraint(conn: &Connection) -> StoreResult<()> {
    let table_sql: String = conn.query_row(
        "SELECT sql
           FROM sqlite_master
          WHERE type = 'table'
            AND name = 'artifacts'",
        [],
        |row| row.get(0),
    )?;
    let normalized = table_sql
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let has_body_path_shape = normalized.contains("body_path is null")
        && normalized.contains("length(trim(body_path)) > 0")
        && normalized.contains("body_path not glob '/*'")
        && normalized.contains("body_path not glob '[a-za-z]:*'")
        && normalized.contains(r"and instr(body_path, '\') = 0")
        && normalized.contains("body_path <> '..'")
        && normalized.contains("body_path not glob '../*'")
        && normalized.contains("body_path not glob '*/../*'")
        && normalized.contains("body_path not glob '*/..'")
        && normalized.contains("body_path <> 'artifacts'")
        && normalized.contains("body_path not glob 'artifacts/*'");
    if has_body_path_shape {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            "artifacts.body_path constraint is missing or malformed",
        ))
    }
}

fn require_column(
    conn: &Connection,
    database_kind: &'static str,
    table: &str,
    column: &str,
) -> StoreResult<()> {
    if column_exists(conn, table, column)? {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            database_kind,
            format!("missing column {table}.{column}"),
        ))
    }
}

fn reject_column(
    conn: &Connection,
    database_kind: &'static str,
    table: &str,
    column: &str,
) -> StoreResult<()> {
    if column_exists(conn, table, column)? {
        Err(StoreError::schema_invariant(
            database_kind,
            format!("forbidden column {table}.{column}"),
        ))
    } else {
        Ok(())
    }
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let escaped_table = table.replace('"', "\"\"");
    let sql = format!("PRAGMA table_info(\"{escaped_table}\")");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }

    Ok(false)
}

fn validate_foreign_key_check(conn: &Connection, database_kind: &'static str) -> StoreResult<()> {
    let mut stmt = conn.prepare("PRAGMA foreign_key_check")?;
    let mut rows = stmt.query([])?;

    if rows.next()?.is_some() {
        return Err(StoreError::schema_invariant(
            database_kind,
            "PRAGMA foreign_key_check reported a violation",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rusqlite::{params, Error, ErrorCode};
    use volicord_test_support::TempRuntimeHome;

    use super::*;
    use crate::migrations::{
        PROJECT_STATE_SCHEMA_VERSION, REGISTRY_SCHEMA_VERSION, STORAGE_PROFILE,
    };

    #[test]
    fn registry_migrations_are_idempotent() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("registry-idempotent")?;
        let path = registry_db_path(runtime_home.path());

        let conn = open_registry_database(&path)?;
        assert_eq!(migration_count(&conn)?, REGISTRY_SCHEMA_VERSION);
        assert_eq!(
            latest_migration_version(&conn, REGISTRY_DATABASE_KIND)?,
            REGISTRY_SCHEMA_VERSION
        );
        drop(conn);

        let conn = open_registry_database(&path)?;
        assert_eq!(migration_count(&conn)?, REGISTRY_SCHEMA_VERSION);
        assert!(foreign_keys_enabled(&conn)?);
        assert!(sqlite_object_exists(&conn, "table", "runtime_home")?);
        assert!(sqlite_object_exists(&conn, "table", "agent_connections")?);
        assert!(sqlite_object_exists(&conn, "table", "connection_projects")?);
        assert!(sqlite_object_exists(&conn, "table", "guard_installations")?);
        Ok(())
    }

    #[test]
    fn project_state_migrations_are_idempotent() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("project-state-idempotent")?;
        let path = project_state_db_path(runtime_home.path(), "PRJ-0001");

        let conn = open_project_state_database(&path)?;
        assert_eq!(migration_count(&conn)?, PROJECT_STATE_SCHEMA_VERSION);
        assert_eq!(
            latest_migration_version(&conn, PROJECT_STATE_DATABASE_KIND)?,
            PROJECT_STATE_SCHEMA_VERSION
        );
        assert!(migration_exists(
            &conn,
            PROJECT_STATE_DATABASE_KIND,
            1,
            "project_state_initial_v1"
        )?);
        assert!(migration_exists(
            &conn,
            PROJECT_STATE_DATABASE_KIND,
            PROJECT_STATE_SCHEMA_VERSION,
            "project_state_session_watch_v5"
        )?);
        drop(conn);

        let conn = open_project_state_database(&path)?;
        assert_eq!(migration_count(&conn)?, PROJECT_STATE_SCHEMA_VERSION);
        assert!(foreign_keys_enabled(&conn)?);
        assert!(sqlite_object_exists(&conn, "table", "tool_invocations")?);
        assert!(sqlite_object_exists(&conn, "table", "agent_sessions")?);
        assert!(sqlite_object_exists(&conn, "table", "expected_writes")?);
        assert!(sqlite_object_exists(&conn, "table", "unrecorded_changes")?);
        assert!(sqlite_object_exists(
            &conn,
            "table",
            "session_watch_baselines"
        )?);
        assert!(sqlite_object_exists(
            &conn,
            "table",
            "session_watch_observations"
        )?);
        validate_tool_invocations_columns(&conn)?;
        validate_tool_invocations_operation_category_constraint(&conn)?;
        Ok(())
    }

    #[test]
    fn project_state_schema_has_single_public_clock_column() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("single-clock")?;
        let conn = open_project_state_database(runtime_home.project_state_db_path("PRJ-clock"))?;

        assert!(column_exists(&conn, "project_state", "state_version")?);
        assert!(!column_exists(&conn, "tasks", "state_version")?);
        Ok(())
    }

    #[test]
    fn foreign_keys_are_enforced() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("foreign-keys")?;
        let conn = open_project_state_database(runtime_home.project_state_db_path("PRJ-fk"))?;

        let err = conn
            .execute(
                "INSERT INTO tasks (
                    project_id,
                    task_id,
                    created_by_actor_source,
                    mode,
                    lifecycle_phase,
                    created_at,
                    updated_at
                )
                VALUES (
                    'missing-project',
                    'task_missing',
                    'agent_connection:conn_main',
                    'work',
                    'shaping',
                    't0',
                    't0'
                )",
                [],
            )
            .expect_err("task insert without project_state row must fail");
        assert_constraint_error(err);
        Ok(())
    }

    #[test]
    fn one_active_current_change_unit_is_allowed_per_task() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("current-change-unit")?;
        let conn =
            open_project_state_database(runtime_home.project_state_db_path("PRJ-change-unit"))?;
        insert_minimal_project_task(&conn)?;

        conn.execute(
            "INSERT INTO change_units (
                project_id,
                change_unit_id,
                task_id,
                status,
                is_current,
                created_at,
                updated_at
            )
            VALUES ('project_a', 'cu_1', 'task_a', 'active', 1, 't0', 't0')",
            [],
        )?;

        let err = conn
            .execute(
                "INSERT INTO change_units (
                    project_id,
                    change_unit_id,
                    task_id,
                    status,
                    is_current,
                    created_at,
                    updated_at
                )
                VALUES ('project_a', 'cu_2', 'task_a', 'active', 1, 't1', 't1')",
                [],
            )
            .expect_err("second active current Change Unit must fail");
        assert_constraint_error(err);
        Ok(())
    }

    #[test]
    fn tool_invocations_key_does_not_include_request_hash() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("tool-invocations")?;
        let conn = open_project_state_database(runtime_home.project_state_db_path("PRJ-tools"))?;
        insert_minimal_project_task(&conn)?;

        insert_tool_invocation(&conn, "idem_same", "sha256:first", 1)?;
        insert_tool_invocation(&conn, "idem_other", "sha256:first", 2)?;

        let err = conn
            .execute(
                "INSERT INTO tool_invocations (
                    project_id,
                    tool_name,
                    idempotency_key,
                    request_hash,
                    basis_state_version,
                    committed_state_version,
                    actor_source,
                    operation_category,
                    response_json,
                    created_at
                )
                VALUES (
                    'project_a',
                    'volicord.intake',
                    'idem_same',
                    'sha256:second',
                    0,
                    3,
                    'agent_connection:conn_main',
                    'agent_workflow',
                    '{}',
                    't2'
                )",
                [],
            )
            .expect_err("same project/tool/idempotency key must be unique");
        assert_constraint_error(err);
        Ok(())
    }

    #[test]
    fn verified_tool_invocation_rejects_invalid_operation_category() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("tool-invocations-operation-category")?;
        let conn =
            open_project_state_database(runtime_home.project_state_db_path("PRJ-tools-category"))?;
        insert_project_state(&conn)?;

        let err = conn
            .execute(
                "INSERT INTO tool_invocations (
                    project_id,
                    tool_name,
                    idempotency_key,
                    request_hash,
                    basis_state_version,
                    committed_state_version,
                    actor_source,
                    operation_category,
                    response_json,
                    created_at
                )
                VALUES (
                    'project_a',
                    'volicord.intake',
                    'idem_invalid_category',
                    'sha256:first',
                    0,
                    1,
                    'agent_connection:conn_main',
                    'core_mutation',
                    '{}',
                    't0'
                )",
                [],
            )
            .expect_err("verified replay context must use a supported operation category");
        assert_constraint_error(err);
        Ok(())
    }

    #[test]
    fn tool_invocation_requires_complete_replay_context() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("tool-invocations-context")?;
        let conn =
            open_project_state_database(runtime_home.project_state_db_path("PRJ-tools-context"))?;
        insert_project_state(&conn)?;

        let err = conn
            .execute(
                "INSERT INTO tool_invocations (
                    project_id,
                    tool_name,
                    idempotency_key,
                    request_hash,
                    basis_state_version,
                    committed_state_version,
                    response_json,
                    created_at
                )
                VALUES (
                    'project_a',
                    'volicord.intake',
                    'idem_missing_context',
                    'sha256:first',
                    0,
                    1,
                    '{}',
                    't0'
                )",
                [],
            )
            .expect_err("replay context must include identity fields");
        assert_constraint_error(err);
        Ok(())
    }

    #[test]
    fn project_state_schema_validation_rejects_replay_context_status_column() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("schema-validation-replay-status")?;
        let conn =
            open_project_state_database(runtime_home.project_state_db_path("PRJ-validation"))?;
        conn.execute(
            "ALTER TABLE tool_invocations ADD COLUMN replay_context_status TEXT",
            [],
        )?;

        let error = validate_project_state_schema(&conn)
            .expect_err("legacy replay context status column should fail schema validation");
        let classification = error.classification();

        assert!(matches!(error, StoreError::SchemaInvariant { .. }));
        assert!(matches!(
            classification.route,
            crate::StoreFailureRoute::OperationalUnavailable
        ));
        assert_eq!(classification.category, "schema_invariant");
        assert_eq!(
            classification.database_kind,
            Some(PROJECT_STATE_DATABASE_KIND)
        );
        Ok(())
    }

    #[test]
    fn immediate_transaction_serializes_writers() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("immediate-transaction")?;
        let path = runtime_home.project_state_db_path("PRJ-tx");
        let mut first = open_project_state_database(&path)?;
        let mut second = open_project_state_database(&path)?;
        first.busy_timeout(Duration::from_millis(0))?;
        second.busy_timeout(Duration::from_millis(0))?;

        let tx = begin_immediate_transaction(&mut first)?;
        let err = begin_immediate_transaction(&mut second)
            .expect_err("second immediate writer should wait or fail while first is open");
        assert_locked_error(err);
        tx.rollback()?;
        Ok(())
    }

    #[test]
    fn immediate_transaction_helper_commits_on_success() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("immediate-helper")?;
        let mut conn =
            open_project_state_database(runtime_home.project_state_db_path("PRJ-helper"))?;

        with_immediate_transaction(&mut conn, |tx| {
            tx.execute(
                "INSERT INTO project_state (
                    project_id,
                    storage_profile,
                    schema_version,
                    created_at,
                    updated_at
                )
                VALUES (?1, ?2, ?3, 't0', 't0')",
                params!["project_tx", STORAGE_PROFILE, PROJECT_STATE_SCHEMA_VERSION],
            )?;
            Ok(())
        })?;

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM project_state WHERE project_id = 'project_tx'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(count, 1);
        Ok(())
    }

    fn migration_count(conn: &Connection) -> rusqlite::Result<i64> {
        conn.query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
    }

    fn latest_migration_version(conn: &Connection, database_kind: &str) -> rusqlite::Result<i64> {
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0)
               FROM schema_migrations
              WHERE database_kind = ?1",
            [database_kind],
            |row| row.get(0),
        )
    }

    fn migration_exists(
        conn: &Connection,
        database_kind: &str,
        version: i64,
        name: &str,
    ) -> rusqlite::Result<bool> {
        conn.query_row(
            "SELECT COUNT(*)
               FROM schema_migrations
              WHERE database_kind = ?1
                AND version = ?2
                AND name = ?3",
            params![database_kind, version, name],
            |row| Ok(row.get::<_, i64>(0)? == 1),
        )
    }

    fn insert_project_state(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO project_state (
                project_id,
                storage_profile,
                schema_version,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, 't0', 't0')",
            params!["project_a", STORAGE_PROFILE, PROJECT_STATE_SCHEMA_VERSION],
        )?;
        Ok(())
    }

    fn insert_minimal_project_task(conn: &Connection) -> rusqlite::Result<()> {
        insert_project_state(conn)?;
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
            VALUES (
                'project_a',
                'task_a',
                'agent_connection:conn_main',
                'work',
                'shaping',
                't0',
                't0'
            )",
            [],
        )?;
        Ok(())
    }

    fn insert_tool_invocation(
        conn: &Connection,
        idempotency_key: &str,
        request_hash: &str,
        committed_state_version: i64,
    ) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO tool_invocations (
                project_id,
                tool_name,
                idempotency_key,
                request_hash,
                basis_state_version,
                committed_state_version,
                actor_source,
                operation_category,
                response_json,
                created_at
            )
            VALUES (
                'project_a',
                'volicord.intake',
                ?1,
                ?2,
                0,
                ?3,
                'agent_connection:conn_main',
                'agent_workflow',
                '{}',
                't0'
            )",
            params![idempotency_key, request_hash, committed_state_version],
        )?;
        Ok(())
    }

    fn assert_constraint_error(err: Error) {
        match err {
            Error::SqliteFailure(error, _) => {
                assert_eq!(error.code, ErrorCode::ConstraintViolation);
            }
            other => panic!("expected SQLite constraint error, got {other:?}"),
        }
    }

    fn assert_locked_error(err: Error) {
        match err {
            Error::SqliteFailure(error, _) => {
                assert!(
                    matches!(
                        error.code,
                        ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
                    ),
                    "expected busy or locked error, got {:?}",
                    error.code
                );
            }
            other => panic!("expected SQLite lock error, got {other:?}"),
        }
    }
}
