use std::{
    fs, io,
    path::{Path, PathBuf},
};

use rusqlite::{
    config::DbConfig,
    functions::{Context, FunctionFlags},
    Connection, OpenFlags, Transaction, TransactionBehavior,
};
use volicord_types::{canonical_json_string, StorageDatabaseKind, StorageManifest, UtcTimestamp};

use crate::{
    schema::{
        current_schema_facts, current_storage_manifest, current_storage_manifest_json,
        extract_schema_facts, initialize_project_state_schema, initialize_registry_schema,
        GeneratedSchemaFacts, PROJECT_STATE_DATABASE_KIND, REGISTRY_DATABASE_KIND,
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

const UTC_SECONDS_SQL_FUNCTION: &str = "volicord_utc_seconds";
const UTC_SUBSEC_NANOS_SQL_FUNCTION: &str = "volicord_utc_subsec_nanos";

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

/// Opens `registry.sqlite`, creating its canonical schema only when empty.
pub fn open_registry_database(path: impl AsRef<Path>) -> StoreResult<Connection> {
    let mut conn = open_sqlite_database(path)?;
    initialize_registry_schema(&mut conn)?;
    validate_registry_schema(&conn)?;
    Ok(conn)
}

/// Opens an existing `registry.sqlite` for read-only exact-contract validation.
pub fn open_registry_database_read_only(path: impl AsRef<Path>) -> StoreResult<Connection> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(StoreError::NotFound {
            entity: "runtime_home",
            id: path.display().to_string(),
        });
    }

    let conn = open_read_only_database(path)?;
    validate_registry_schema(&conn)?;
    Ok(conn)
}

/// Opens project `state.sqlite`, creating its canonical schema only when empty.
pub fn open_project_state_database(path: impl AsRef<Path>) -> StoreResult<Connection> {
    let mut conn = open_sqlite_database(path)?;
    initialize_project_state_schema(&mut conn)?;
    validate_project_state_schema(&conn)?;
    Ok(conn)
}

/// Opens an existing project `state.sqlite` for read-only exact-contract validation.
pub fn open_project_state_database_read_only(path: impl AsRef<Path>) -> StoreResult<Connection> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(StoreError::NotFound {
            entity: "project_state_database",
            id: path.display().to_string(),
        });
    }

    let conn = open_read_only_database(path)?;
    validate_project_state_schema(&conn)?;
    Ok(conn)
}

/// Opens an existing SQLite database for inspection without creating it.
pub fn open_read_only_database(path: impl AsRef<Path>) -> StoreResult<Connection> {
    let conn = Connection::open_with_flags(
        path.as_ref(),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    register_utc_order_functions(&conn)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)?;
    conn.pragma_update(None, "query_only", "ON")?;
    Ok(conn)
}

/// Probes whether an existing SQLite database can acquire a write transaction.
pub fn sqlite_database_write_capability(path: impl AsRef<Path>) -> StoreResult<bool> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(StoreError::NotFound {
            entity: "project_state_database",
            id: path.display().to_string(),
        });
    }

    let conn = match Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(conn) => conn,
        Err(error) if sqlite_write_probe_denied(&error) => return Ok(false),
        Err(error) => return Err(StoreError::from(error)),
    };
    enable_foreign_keys(&conn)?;
    match conn.execute_batch("BEGIN IMMEDIATE") {
        Ok(()) => match conn.execute_batch(
            "CREATE TABLE __volicord_write_probe_do_not_persist (probe INTEGER);
             DROP TABLE __volicord_write_probe_do_not_persist",
        ) {
            Ok(()) => {
                conn.execute_batch("ROLLBACK")?;
                Ok(true)
            }
            Err(error) if sqlite_write_probe_denied(&error) => {
                let _ = conn.execute_batch("ROLLBACK");
                Ok(false)
            }
            Err(error) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(StoreError::from(error))
            }
        },
        Err(error) if sqlite_write_probe_denied(&error) => Ok(false),
        Err(error) => Err(StoreError::from(error)),
    }
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

/// Validates the exact registry manifest, generated metadata, and physical schema.
pub fn validate_registry_schema(conn: &Connection) -> StoreResult<()> {
    validate_foreign_keys_enabled(conn, REGISTRY_DATABASE_KIND)?;
    validate_manifest_carrier(
        conn,
        REGISTRY_DATABASE_KIND,
        "SELECT storage_profile FROM runtime_home ORDER BY singleton_id",
    )?;
    validate_generated_schema(conn, REGISTRY_DATABASE_KIND, StorageDatabaseKind::Registry)?;
    validate_foreign_key_check(conn, REGISTRY_DATABASE_KIND)
}

/// Validates the exact project manifest, generated metadata, and physical schema.
pub fn validate_project_state_schema(conn: &Connection) -> StoreResult<()> {
    validate_foreign_keys_enabled(conn, PROJECT_STATE_DATABASE_KIND)?;
    validate_manifest_carrier(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "SELECT storage_profile FROM project_state ORDER BY project_id",
    )?;
    validate_generated_schema(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        StorageDatabaseKind::ProjectState,
    )?;
    validate_foreign_key_check(conn, PROJECT_STATE_DATABASE_KIND)
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
    register_utc_order_functions(&conn)?;
    enable_foreign_keys(&conn)?;
    Ok(conn)
}

fn register_utc_order_functions(conn: &Connection) -> rusqlite::Result<()> {
    let flags = FunctionFlags::SQLITE_UTF8
        | FunctionFlags::SQLITE_DETERMINISTIC
        | FunctionFlags::SQLITE_INNOCUOUS;
    conn.create_scalar_function(UTC_SECONDS_SQL_FUNCTION, 1, flags, |context| {
        Ok(strict_utc_order_timestamp(context)?
            .as_datetime()
            .timestamp())
    })?;
    conn.create_scalar_function(UTC_SUBSEC_NANOS_SQL_FUNCTION, 1, flags, |context| {
        Ok(i64::from(
            strict_utc_order_timestamp(context)?
                .as_datetime()
                .timestamp_subsec_nanos(),
        ))
    })?;
    Ok(())
}

fn strict_utc_order_timestamp(context: &Context<'_>) -> rusqlite::Result<UtcTimestamp> {
    let raw = context.get::<String>(0)?;
    let timestamp = UtcTimestamp::parse(&raw).map_err(|_| utc_order_function_error())?;
    timestamp
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| utc_order_function_error())?;
    Ok(timestamp)
}

fn utc_order_function_error() -> rusqlite::Error {
    rusqlite::Error::UserFunctionError(Box::new(io::Error::new(
        io::ErrorKind::InvalidData,
        "timestamp is not a canonical four-digit RFC 3339 UTC instant",
    )))
}

fn sqlite_write_probe_denied(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(sqlite_error, _)
            if matches!(
                sqlite_error.code,
                rusqlite::ErrorCode::CannotOpen
                    | rusqlite::ErrorCode::ReadOnly
                    | rusqlite::ErrorCode::PermissionDenied
            )
    )
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

fn validate_manifest_carrier(
    conn: &Connection,
    database_kind: &'static str,
    query: &str,
) -> StoreResult<()> {
    let mut statement = conn.prepare(query)?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let profiles = rows.collect::<rusqlite::Result<Vec<_>>>()?;

    // A newly applied empty schema is not yet a published storage instance. Once
    // its owner row exists, exactly one complete current manifest is mandatory.
    if profiles.is_empty() {
        return Ok(());
    }
    if profiles.len() != 1 {
        return Err(StoreError::schema_invariant(
            database_kind,
            "manifest carrier must contain exactly one owner row",
        ));
    }
    validate_persisted_manifest(database_kind, &profiles[0])
}

pub(crate) fn validate_persisted_manifest(
    database_kind: &'static str,
    persisted: &str,
) -> StoreResult<()> {
    let expected = current_storage_manifest()?;
    let expected_json = current_storage_manifest_json()?;
    let decoded = serde_json::from_str::<StorageManifest>(persisted);
    match decoded {
        Ok(manifest) if &manifest == expected => {
            let canonical = canonical_json_string(&manifest).map_err(|error| {
                StoreError::schema_invariant(
                    database_kind,
                    format!("manifest canonical encoding failed: {error}"),
                )
            })?;
            if canonical == persisted {
                Ok(())
            } else {
                Err(StoreError::schema_invariant(
                    database_kind,
                    "current manifest carrier is not canonically encoded",
                ))
            }
        }
        Ok(manifest)
            if manifest.contract_id == volicord_types::STORAGE_CONTRACT_ID
                && manifest.canonical_ddl_digest
                    == "sha256:73ee4020f39d43134ce3dbce74c64243c6fc35c4f6910aa3b927bcf45de6e0d9"
                && manifest.integrity_constraints_digest
                    == "sha256:6f7cc21d2070888dbbe26803671644cbc3ad4bce52347e5015d4fb91fa4d1d9e"
                && manifest.enabled_capabilities == expected.enabled_capabilities =>
        {
            let actual = canonical_json_string(&manifest).map_err(|error| {
                StoreError::schema_invariant(
                    database_kind,
                    format!("prior manifest canonical encoding failed: {error}"),
                )
            })?;
            Err(StoreError::unsupported_storage_profile(
                database_kind,
                actual,
                expected_json,
            ))
        }
        Ok(manifest) if manifest.contract_id == volicord_types::STORAGE_CONTRACT_ID => {
            Err(StoreError::schema_invariant(
                database_kind,
                "current manifest digest or capabilities do not match",
            ))
        }
        Ok(manifest) => Err(StoreError::unsupported_storage_profile(
            database_kind,
            manifest.contract_id,
            expected_json,
        )),
        Err(error) => Err(StoreError::schema_invariant(
            database_kind,
            format!("persisted storage manifest is malformed: {error}"),
        )),
    }
}

fn validate_generated_schema(
    conn: &Connection,
    database_kind: &'static str,
    generated_database: StorageDatabaseKind,
) -> StoreResult<()> {
    let expected = current_schema_facts(generated_database)?;
    let actual = extract_schema_facts(conn, generated_database).map_err(|detail| {
        StoreError::schema_invariant(
            database_kind,
            format!("physical schema metadata extraction failed: {detail}"),
        )
    })?;
    if actual == expected {
        Ok(())
    } else {
        Err(schema_mismatch_error(database_kind, &expected, &actual))
    }
}

fn schema_mismatch_error(
    database_kind: &'static str,
    expected: &GeneratedSchemaFacts,
    actual: &GeneratedSchemaFacts,
) -> StoreError {
    let detail = if actual.tables != expected.tables {
        relation_mismatch_detail(expected, actual)
    } else if actual.columns != expected.columns {
        column_mismatch_detail(expected, actual)
    } else if actual.indexes != expected.indexes {
        index_mismatch_detail(expected, actual)
    } else {
        constraint_mismatch_detail(expected, actual)
    };
    StoreError::schema_invariant(database_kind, detail)
}

fn relation_mismatch_detail(
    expected: &GeneratedSchemaFacts,
    actual: &GeneratedSchemaFacts,
) -> String {
    if let Some(relation) = actual.tables.iter().find(|actual_relation| {
        !expected.tables.iter().any(|expected_relation| {
            expected_relation.database == actual_relation.database
                && expected_relation.relation_kind == actual_relation.relation_kind
                && expected_relation.name == actual_relation.name
        })
    }) {
        return format!(
            "unexpected SQLite relation {}; explicitly recreate the storage instance",
            relation.name
        );
    }
    if let Some(relation) = expected.tables.iter().find(|expected_relation| {
        !actual.tables.iter().any(|actual_relation| {
            actual_relation.database == expected_relation.database
                && actual_relation.relation_kind == expected_relation.relation_kind
                && actual_relation.name == expected_relation.name
        })
    }) {
        return format!("missing canonical SQLite relation {}", relation.name);
    }
    let relation = actual
        .tables
        .iter()
        .zip(&expected.tables)
        .find(|(actual_relation, expected_relation)| actual_relation != expected_relation)
        .map(|(actual_relation, _)| actual_relation.name.as_str())
        .unwrap_or("unknown");
    format!("canonical SQLite relation {relation} definition differs")
}

fn column_mismatch_detail(
    expected: &GeneratedSchemaFacts,
    actual: &GeneratedSchemaFacts,
) -> String {
    if let Some(column) = actual.columns.iter().find(|actual_column| {
        !expected.columns.iter().any(|expected_column| {
            expected_column.database == actual_column.database
                && expected_column.table == actual_column.table
                && expected_column.name == actual_column.name
        })
    }) {
        return format!("unexpected SQLite column {}.{}", column.table, column.name);
    }
    if let Some(column) = expected.columns.iter().find(|expected_column| {
        !actual.columns.iter().any(|actual_column| {
            actual_column.database == expected_column.database
                && actual_column.table == expected_column.table
                && actual_column.name == expected_column.name
        })
    }) {
        return format!(
            "missing canonical SQLite column {}.{}",
            column.table, column.name
        );
    }
    let column = actual
        .columns
        .iter()
        .zip(&expected.columns)
        .find(|(actual_column, expected_column)| actual_column != expected_column)
        .map(|(actual_column, _)| format!("{}.{}", actual_column.table, actual_column.name))
        .unwrap_or_else(|| "unknown".to_owned());
    format!("canonical SQLite column {column} definition differs")
}

fn index_mismatch_detail(expected: &GeneratedSchemaFacts, actual: &GeneratedSchemaFacts) -> String {
    if let Some(index) = actual.indexes.iter().find(|actual_index| {
        !expected.indexes.iter().any(|expected_index| {
            expected_index.database == actual_index.database
                && expected_index.name == actual_index.name
        })
    }) {
        return format!("unexpected SQLite index {}", index.name);
    }
    if let Some(index) = expected.indexes.iter().find(|expected_index| {
        !actual.indexes.iter().any(|actual_index| {
            actual_index.database == expected_index.database
                && actual_index.name == expected_index.name
        })
    }) {
        return format!("missing canonical SQLite index {}", index.name);
    }
    let index = actual
        .indexes
        .iter()
        .zip(&expected.indexes)
        .find(|(actual_index, expected_index)| actual_index != expected_index)
        .map(|(actual_index, _)| actual_index.name.as_str())
        .unwrap_or("unknown");
    format!("canonical SQLite index {index} definition differs")
}

fn constraint_mismatch_detail(
    expected: &GeneratedSchemaFacts,
    actual: &GeneratedSchemaFacts,
) -> String {
    let table = actual
        .constraints
        .iter()
        .zip(&expected.constraints)
        .find(|(actual_constraint, expected_constraint)| actual_constraint != expected_constraint)
        .map(|(actual_constraint, _)| actual_constraint.table.as_str())
        .unwrap_or("unknown");
    format!("canonical SQLite integrity constraints differ for {table}")
}

fn validate_foreign_key_check(conn: &Connection, database_kind: &'static str) -> StoreResult<()> {
    let mut statement = conn.prepare("PRAGMA foreign_key_check")?;
    let mut rows = statement.query([])?;
    if rows.next()?.is_some() {
        Err(StoreError::schema_invariant(
            database_kind,
            "PRAGMA foreign_key_check reported a violation",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rusqlite::{params, Error, ErrorCode};
    use serde_json::Value;
    use volicord_test_support::TempRuntimeHome;

    use super::*;
    use crate::schema::current_storage_manifest_json;

    #[test]
    fn canonical_schema_initialization_is_idempotent() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("canonical-schema-idempotent")?;
        let registry_path = registry_db_path(runtime_home.path());
        open_registry_database(&registry_path)?;
        let registry = open_registry_database(&registry_path)?;
        validate_registry_schema(&registry)?;

        let project_path = project_state_db_path(runtime_home.path(), "PRJ-0001");
        open_project_state_database(&project_path)?;
        let project = open_project_state_database(&project_path)?;
        validate_project_state_schema(&project)
    }

    #[test]
    fn exact_validator_rejects_extra_table_column_and_index() -> StoreResult<()> {
        let cases = [
            "CREATE TABLE runtime_extension (id TEXT PRIMARY KEY)",
            "ALTER TABLE tasks ADD COLUMN extension_metadata TEXT",
            "CREATE INDEX idx_tasks_extension_metadata ON tasks (metadata_json)",
        ];
        for (index, statement) in cases.into_iter().enumerate() {
            let runtime_home = TempRuntimeHome::new(&format!("schema-mismatch-{index}"))?;
            let path = project_state_db_path(runtime_home.path(), "PRJ-mismatch");
            let conn = open_project_state_database(&path)?;
            conn.execute_batch(statement)?;
            let error = validate_project_state_schema(&conn)
                .expect_err("physical schema mismatch must fail closed");
            assert!(matches!(error, StoreError::SchemaInvariant { .. }));
        }
        Ok(())
    }

    #[test]
    fn current_manifest_requires_canonical_exact_encoding() -> StoreResult<()> {
        let manifest = current_storage_manifest_json()?;
        let parsed: Value = serde_json::from_str(manifest).expect("manifest JSON");
        assert_eq!(
            parsed["contract_id"],
            Value::String(volicord_types::STORAGE_CONTRACT_ID.to_owned())
        );
        assert_eq!(
            canonical_json_string(&parsed).expect("canonical JSON"),
            manifest
        );
        Ok(())
    }

    #[test]
    fn persisted_manifest_distinguishes_unsupported_from_corrupt() -> StoreResult<()> {
        let current = current_storage_manifest()?;
        let unsupported = StorageManifest::new(
            "unknown_storage_contract",
            current.canonical_ddl_digest.clone(),
            current.integrity_constraints_digest.clone(),
            current.enabled_capabilities.clone(),
        )
        .expect("well-formed non-current manifest");
        let unsupported = canonical_json_string(&unsupported).expect("canonical manifest JSON");
        let unsupported_error = validate_persisted_manifest(REGISTRY_DATABASE_KIND, &unsupported)
            .expect_err("well-formed non-current manifest must be unsupported");
        assert!(matches!(
            unsupported_error,
            StoreError::UnsupportedStorageProfile { .. }
        ));

        let malformed_current = format!(
            r#"{{"contract_id":"{}"}}"#,
            volicord_types::STORAGE_CONTRACT_ID
        );
        for corrupt in ["not-json", malformed_current.as_str()] {
            let error = validate_persisted_manifest(REGISTRY_DATABASE_KIND, corrupt)
                .expect_err("malformed persisted manifest must be corrupt");
            assert!(matches!(error, StoreError::SchemaInvariant { .. }));
            assert_eq!(
                error.classification().route,
                crate::StoreFailureRoute::PersistedDataCorrupt
            );
        }
        Ok(())
    }

    #[test]
    fn persisted_current_manifest_requires_canonical_encoding() -> StoreResult<()> {
        let current = current_storage_manifest_json()?;
        let value: Value = serde_json::from_str(current).expect("manifest JSON");
        let noncanonical = serde_json::to_string_pretty(&value).expect("pretty manifest JSON");

        let error = validate_persisted_manifest(PROJECT_STATE_DATABASE_KIND, &noncanonical)
            .expect_err("non-canonical current manifest must be corrupt");

        assert!(matches!(error, StoreError::SchemaInvariant { .. }));
        assert_eq!(
            error.classification().route,
            crate::StoreFailureRoute::PersistedDataCorrupt
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

        let transaction = begin_immediate_transaction(&mut first)?;
        let error = begin_immediate_transaction(&mut second)
            .expect_err("second immediate writer must not enter concurrently");
        assert!(matches!(
            error,
            Error::SqliteFailure(sqlite_error, _)
                if matches!(sqlite_error.code, ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
        ));
        transaction.rollback()?;
        Ok(())
    }

    #[test]
    fn foreign_keys_are_enabled_and_checked() -> StoreResult<()> {
        let runtime_home = TempRuntimeHome::new("foreign-keys")?;
        let conn = open_project_state_database(runtime_home.project_state_db_path("PRJ-fk"))?;
        assert!(foreign_keys_enabled(&conn)?);
        conn.execute(
            "INSERT INTO tasks (
                project_id, task_id, created_by_actor_source, mode,
                requested_control_level, effective_control_level, control_level_reason,
                work_phase, acceptance_policy, acceptance_policy_reason, carry_forward_json,
                lifecycle_phase, created_at, updated_at
             ) VALUES (
                'missing', 'task_missing', 'agent_connection:conn_main', 'work',
                'tracked', 'tracked', 'fixture', 'shaping', 'required', 'fixture', '[]',
                'shaping', 't0', 't0'
             )",
            params![],
        )
        .expect_err("foreign-key violation must fail");
        Ok(())
    }
}
