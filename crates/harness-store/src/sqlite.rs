use std::{
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::{
    migrations::{
        apply_project_state_migrations, apply_registry_migrations, PROJECT_STATE_DATABASE_KIND,
        PROJECT_STATE_SCHEMA_VERSION, REGISTRY_DATABASE_KIND, REGISTRY_SCHEMA_VERSION,
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
    validate_latest_migration(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        PROJECT_STATE_SCHEMA_VERSION,
    )?;
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
    for column in [
        "surface_id",
        "surface_instance_id",
        "access_class",
        "verification_basis",
        "replay_context_status",
    ] {
        require_column(
            conn,
            PROJECT_STATE_DATABASE_KIND,
            "tool_invocations",
            column,
        )?;
    }
    validate_tool_invocations_primary_key(conn)?;
    validate_tool_invocations_replay_surface_foreign_key(conn)?;
    require_triggers(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        &[
            "tool_invocations_verified_context_insert",
            "tool_invocations_verified_context_update",
        ],
    )?;
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

fn require_triggers(
    conn: &Connection,
    database_kind: &'static str,
    names: &[&str],
) -> StoreResult<()> {
    for name in names {
        if !sqlite_object_exists(conn, "trigger", name)? {
            return Err(StoreError::schema_invariant(
                database_kind,
                format!("missing trigger {name}"),
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

fn validate_project_state_versions(conn: &Connection) -> StoreResult<()> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ForeignKeyListRow {
    id: i64,
    seq: i64,
    parent_table: String,
    from_column: String,
    to_column: String,
    on_delete: String,
}

fn validate_tool_invocations_replay_surface_foreign_key(conn: &Connection) -> StoreResult<()> {
    let mut stmt = conn.prepare("PRAGMA foreign_key_list(tool_invocations)")?;
    let rows = stmt.query_map([], |row| {
        Ok(ForeignKeyListRow {
            id: row.get(0)?,
            seq: row.get(1)?,
            parent_table: row.get(2)?,
            from_column: row.get(3)?,
            to_column: row.get(4)?,
            on_delete: row.get(6)?,
        })
    })?;

    let mut rows_by_id = Vec::<ForeignKeyListRow>::new();
    for row in rows {
        rows_by_id.push(row?);
    }

    let expected_columns = [
        ("project_id", "project_id"),
        ("surface_id", "surface_id"),
        ("surface_instance_id", "surface_instance_id"),
    ];

    for id in rows_by_id.iter().map(|row| row.id) {
        let mut candidate = rows_by_id
            .iter()
            .filter(|row| row.id == id)
            .cloned()
            .collect::<Vec<_>>();
        candidate.sort_by_key(|row| row.seq);

        if candidate.len() != expected_columns.len() {
            continue;
        }
        if !candidate.iter().all(|row| row.parent_table == "surfaces") {
            continue;
        }
        if !candidate.iter().all(|row| row.on_delete == "RESTRICT") {
            continue;
        }

        let actual_columns = candidate
            .iter()
            .map(|row| (row.from_column.as_str(), row.to_column.as_str()))
            .collect::<Vec<_>>();
        if actual_columns == expected_columns {
            return Ok(());
        }
    }

    Err(StoreError::schema_invariant(
        PROJECT_STATE_DATABASE_KIND,
        "tool_invocations replay surface foreign key is missing or malformed",
    ))
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
    use crate::migrations::{
        BASELINE_SCHEMA_VERSION, PROJECT_STATE_SCHEMA_VERSION, STORAGE_PROFILE,
    };

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
        assert_eq!(migration_count(&conn)?, 3);
        assert_eq!(
            latest_migration_version(&conn, PROJECT_STATE_DATABASE_KIND)?,
            PROJECT_STATE_SCHEMA_VERSION
        );
        assert!(migration_exists(
            &conn,
            PROJECT_STATE_DATABASE_KIND,
            BASELINE_SCHEMA_VERSION,
            "project_state_baseline_v1"
        )?);
        assert!(migration_exists(
            &conn,
            PROJECT_STATE_DATABASE_KIND,
            PROJECT_STATE_SCHEMA_VERSION,
            "project_state_replay_surface_fk_v3"
        )?);
        drop(conn);

        let conn = open_project_state_database(&path)?;
        assert_eq!(migration_count(&conn)?, 3);
        assert!(foreign_keys_enabled(&conn)?);
        assert!(sqlite_object_exists(&conn, "table", "tool_invocations")?);
        assert!(column_exists(
            &conn,
            "tool_invocations",
            "replay_context_status"
        )?);
        validate_tool_invocations_replay_surface_foreign_key(&conn)?;
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
                    surface_id,
                    surface_instance_id,
                    access_class,
                    replay_context_status,
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
                    'surface_main',
                    'surface_instance_1',
                    'core_mutation',
                    'verified',
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
    fn verified_tool_invocation_requires_existing_surface() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("tool-invocations-surface-fk")?;
        let conn =
            open_project_state_database(runtime_home.project_state_db_path("PRJ-tools-surface"))?;
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
                    surface_id,
                    surface_instance_id,
                    access_class,
                    replay_context_status,
                    response_json,
                    created_at
                )
                VALUES (
                    'project_a',
                    'harness.intake',
                    'idem_missing_surface',
                    'sha256:first',
                    0,
                    1,
                    'missing_surface',
                    'missing_surface_instance',
                    'core_mutation',
                    'verified',
                    '{}',
                    't0'
                )",
                [],
            )
            .expect_err("verified replay context must reference a registered surface");
        assert_foreign_key_constraint_error(err);
        Ok(())
    }

    #[test]
    fn verified_tool_invocation_restricts_surface_deletion() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("tool-invocations-surface-delete")?;
        let conn =
            open_project_state_database(runtime_home.project_state_db_path("PRJ-tools-delete"))?;
        insert_project_state(&conn)?;
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
        insert_tool_invocation(&conn, "idem_surface_delete", "sha256:first", 1)?;

        let err = conn
            .execute(
                "DELETE FROM surfaces
                  WHERE project_id = 'project_a'
                    AND surface_id = 'surface_main'
                    AND surface_instance_id = 'surface_instance_1'",
                [],
            )
            .expect_err("surface deletion must be restricted while replay rows reference it");
        assert_restrictive_delete_constraint_error(err);
        Ok(())
    }

    #[test]
    fn verified_tool_invocation_requires_complete_replay_context() -> StoreResult<()> {
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
                    replay_context_status,
                    response_json,
                    created_at
                )
                VALUES (
                    'project_a',
                    'harness.intake',
                    'idem_missing_context',
                    'sha256:first',
                    0,
                    1,
                    'verified',
                    '{}',
                    't0'
                )",
                [],
            )
            .expect_err("verified replay context must include identity fields");
        assert_constraint_error(err);
        Ok(())
    }

    #[test]
    fn project_state_schema_validation_routes_missing_replay_context_trigger() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("schema-validation-trigger")?;
        let conn =
            open_project_state_database(runtime_home.project_state_db_path("PRJ-validation"))?;
        conn.execute("DROP TRIGGER tool_invocations_verified_context_insert", [])?;

        let error = validate_project_state_schema(&conn)
            .expect_err("missing replay context trigger should fail schema validation");
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
                surface_id,
                surface_instance_id,
                access_class,
                replay_context_status,
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
                'surface_main',
                'surface_instance_1',
                'core_mutation',
                'verified',
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

    fn assert_foreign_key_constraint_error(err: Error) {
        match err {
            Error::SqliteFailure(error, _) => {
                assert_eq!(error.code, ErrorCode::ConstraintViolation);
                assert_eq!(
                    error.extended_code,
                    rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY
                );
            }
            other => panic!("expected SQLite foreign-key constraint error, got {other:?}"),
        }
    }

    fn assert_restrictive_delete_constraint_error(err: Error) {
        match err {
            Error::SqliteFailure(error, _) => {
                assert_eq!(error.code, ErrorCode::ConstraintViolation);
                assert!(
                    matches!(
                        error.extended_code,
                        rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY
                            | rusqlite::ffi::SQLITE_CONSTRAINT_TRIGGER
                    ),
                    "expected foreign-key or restrictive-delete trigger constraint, got {}",
                    error.extended_code
                );
            }
            other => panic!("expected SQLite restrictive-delete constraint error, got {other:?}"),
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
