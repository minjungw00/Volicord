use std::{
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::{
    migrations::{
        apply_project_state_migrations, apply_registry_migrations, PROJECT_STATE_DATABASE_KIND,
        REGISTRY_DATABASE_KIND,
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

/// Enables SQLite foreign-key enforcement for a connection.
pub fn enable_foreign_keys(conn: &Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")
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
    require_tables(
        conn,
        REGISTRY_DATABASE_KIND,
        &["schema_migrations", "runtime_home", "projects"],
    )?;
    require_indexes(
        conn,
        REGISTRY_DATABASE_KIND,
        &["idx_projects_repo_root", "idx_projects_status"],
    )?;
    validate_foreign_key_check(conn, REGISTRY_DATABASE_KIND)?;
    Ok(())
}

/// Validates baseline project-state schema invariants after migration.
pub fn validate_project_state_schema(conn: &Connection) -> StoreResult<()> {
    validate_foreign_keys_enabled(conn, PROJECT_STATE_DATABASE_KIND)?;
    require_tables(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        &[
            "schema_migrations",
            "project_state",
            "surfaces",
            "tasks",
            "change_units",
            "user_judgments",
            "write_authorizations",
            "runs",
            "artifact_staging",
            "artifacts",
            "artifact_links",
            "evidence_summaries",
            "blockers",
            "task_events",
            "tool_invocations",
        ],
    )?;
    require_indexes(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        &[
            "idx_change_units_one_current_active",
            "idx_write_authorizations_consumed_run",
            "idx_runs_write_authorization",
            "idx_artifact_staging_promoted_artifact",
            "idx_artifacts_source_staging",
            "idx_project_state_active_task",
            "idx_surfaces_last_seen",
            "idx_tasks_lifecycle",
            "idx_tasks_current_change_unit",
            "idx_change_units_task_status",
            "idx_user_judgments_task_status",
            "idx_write_authorizations_task_status",
            "idx_runs_task_created",
            "idx_artifact_staging_task_status",
            "idx_artifact_staging_surface",
            "idx_artifacts_task_status",
            "idx_artifact_links_owner",
            "idx_evidence_summaries_task_status",
            "idx_blockers_task_status",
            "idx_task_events_task_seq",
        ],
    )?;
    require_column(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "project_state",
        "state_version",
    )?;
    reject_column(conn, PROJECT_STATE_DATABASE_KIND, "tasks", "state_version")?;
    require_column(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "tool_invocations",
        "request_hash",
    )?;
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

    use harness_test_support::TempRuntimeHome;
    use rusqlite::{params, Error, ErrorCode};

    use super::*;
    use crate::migrations::{BASELINE_SCHEMA_VERSION, STORAGE_PROFILE};

    #[test]
    fn registry_migrations_are_idempotent() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("registry-idempotent")?;
        let path = registry_db_path(runtime_home.path());

        let conn = open_registry_database(&path)?;
        assert_eq!(migration_count(&conn)?, 1);
        drop(conn);

        let conn = open_registry_database(&path)?;
        assert_eq!(migration_count(&conn)?, 1);
        assert!(foreign_keys_enabled(&conn)?);
        assert!(sqlite_object_exists(&conn, "table", "runtime_home")?);
        Ok(())
    }

    #[test]
    fn project_state_migrations_are_idempotent() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("project-state-idempotent")?;
        let path = project_state_db_path(runtime_home.path(), "PRJ-0001");

        let conn = open_project_state_database(&path)?;
        assert_eq!(migration_count(&conn)?, 1);
        drop(conn);

        let conn = open_project_state_database(&path)?;
        assert_eq!(migration_count(&conn)?, 1);
        assert!(foreign_keys_enabled(&conn)?);
        assert!(sqlite_object_exists(&conn, "table", "tool_invocations")?);
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
                "INSERT INTO surfaces (
                    project_id,
                    surface_id,
                    surface_instance_id,
                    surface_kind,
                    registered_at
                )
                VALUES ('missing-project', 'surface-main', 'surface-instance-1', 'cli', 't0')",
                [],
            )
            .expect_err("surface insert without project_state row must fail");
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
        insert_project_state(&conn)?;

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
                    response_json,
                    created_at
                )
                VALUES (
                    'project_a',
                    'harness.intake',
                    'idem_same',
                    'sha256:second',
                    0,
                    3,
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
                params!["project_tx", STORAGE_PROFILE, BASELINE_SCHEMA_VERSION],
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
            params!["project_a", STORAGE_PROFILE, BASELINE_SCHEMA_VERSION],
        )?;
        Ok(())
    }

    fn insert_minimal_project_task(conn: &Connection) -> rusqlite::Result<()> {
        insert_project_state(conn)?;
        conn.execute(
            "INSERT INTO surfaces (
                project_id,
                surface_id,
                surface_instance_id,
                surface_kind,
                registered_at
            )
            VALUES ('project_a', 'surface_main', 'surface_instance_1', 'cli', 't0')",
            [],
        )?;
        conn.execute(
            "INSERT INTO tasks (
                project_id,
                task_id,
                created_by_surface_id,
                created_by_surface_instance_id,
                mode,
                lifecycle_phase,
                created_at,
                updated_at
            )
            VALUES (
                'project_a',
                'task_a',
                'surface_main',
                'surface_instance_1',
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
                response_json,
                created_at
            )
            VALUES (
                'project_a',
                'harness.intake',
                ?1,
                ?2,
                0,
                ?3,
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
