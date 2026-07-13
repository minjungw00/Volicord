use rusqlite::Connection;

use crate::{sqlite::begin_immediate_transaction, StoreError, StoreResult};

/// Baseline storage profile recorded by canonical storage records.
pub const STORAGE_PROFILE: &str = "baseline_sqlite_v5";

/// Storage database kind for `registry.sqlite` diagnostics.
pub const REGISTRY_DATABASE_KIND: &str = "registry";

/// Storage database kind for project `state.sqlite` diagnostics.
pub const PROJECT_STATE_DATABASE_KIND: &str = "project_state";

/// Canonical SQL source for `registry.sqlite`.
pub const REGISTRY_SCHEMA_SQL: &str = include_str!("schema/registry.sql");

/// Canonical SQL source for project `state.sqlite`.
pub const PROJECT_STATE_SCHEMA_SQL: &str = include_str!("schema/project.sql");

/// Initializes `registry.sqlite` from the canonical SQL source when it is empty.
pub fn initialize_registry_schema(conn: &mut Connection) -> StoreResult<()> {
    initialize_canonical_schema(conn, REGISTRY_DATABASE_KIND, REGISTRY_SCHEMA_SQL)
}

/// Initializes project `state.sqlite` from the canonical SQL source when it is empty.
pub fn initialize_project_state_schema(conn: &mut Connection) -> StoreResult<()> {
    initialize_canonical_schema(conn, PROJECT_STATE_DATABASE_KIND, PROJECT_STATE_SCHEMA_SQL)
}

fn initialize_canonical_schema(
    conn: &mut Connection,
    database_kind: &'static str,
    sql: &str,
) -> StoreResult<()> {
    if user_table_count(conn)? != 0 {
        return Ok(());
    }

    let tx = begin_immediate_transaction(conn)?;
    if user_table_count(&tx)? != 0 {
        tx.rollback()?;
        return Ok(());
    }
    tx.execute_batch(sql)?;
    if user_table_count(&tx)? == 0 {
        return Err(StoreError::schema_invariant(
            database_kind,
            "canonical schema initialization produced no tables",
        ));
    }
    tx.commit()?;

    Ok(())
}

fn user_table_count(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*)
           FROM sqlite_master
          WHERE type = 'table'
            AND name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    )
}
