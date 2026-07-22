use std::sync::OnceLock;

use rusqlite::Connection;
use serde::Serialize;
use sha2::{Digest, Sha256};
use volicord_types::{
    canonical_json_bytes, canonical_json_string, GeneratedColumn, GeneratedConstraint,
    GeneratedIndex, GeneratedRelationKind, GeneratedSchemaMetadata, GeneratedTable,
    StorageDatabaseKind, StorageManifest,
};

use crate::{sqlite::begin_immediate_transaction, StoreError, StoreResult};

/// Storage database kind for `registry.sqlite` diagnostics.
pub const REGISTRY_DATABASE_KIND: &str = "registry";

/// Storage database kind for project `state.sqlite` diagnostics.
pub const PROJECT_STATE_DATABASE_KIND: &str = "project_state";

/// Canonical SQL source for `registry.sqlite`.
pub const REGISTRY_SCHEMA_SQL: &str = include_str!("schema/registry.sql");

/// Canonical SQL source for project `state.sqlite`.
pub const PROJECT_STATE_SCHEMA_SQL: &str = include_str!("schema/project.sql");

static GENERATED_SCHEMA_METADATA: OnceLock<Result<GeneratedSchemaMetadata, String>> =
    OnceLock::new();
static CURRENT_STORAGE_MANIFEST: OnceLock<Result<StorageManifest, String>> = OnceLock::new();
static CURRENT_STORAGE_MANIFEST_JSON: OnceLock<Result<String, String>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SchemaDigestInput<'a> {
    tables: &'a [GeneratedTable],
    columns: &'a [GeneratedColumn],
    indexes: &'a [GeneratedIndex],
    constraints: &'a [GeneratedConstraint],
}

/// Schema facts extracted from one SQLite database using the canonical extractor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeneratedSchemaFacts {
    pub tables: Vec<GeneratedTable>,
    pub columns: Vec<GeneratedColumn>,
    pub indexes: Vec<GeneratedIndex>,
    pub constraints: Vec<GeneratedConstraint>,
}

/// Returns the single deterministic metadata artifact derived from canonical SQL.
pub fn generated_schema_metadata() -> StoreResult<&'static GeneratedSchemaMetadata> {
    GENERATED_SCHEMA_METADATA
        .get_or_init(build_generated_schema_metadata)
        .as_ref()
        .map_err(|detail| {
            StoreError::schema_invariant(
                "canonical_sql",
                format!("generated schema metadata is unavailable: {detail}"),
            )
        })
}

/// Returns the complete current manifest constructed from generated metadata.
pub fn current_storage_manifest() -> StoreResult<&'static StorageManifest> {
    CURRENT_STORAGE_MANIFEST
        .get_or_init(|| {
            let metadata = GENERATED_SCHEMA_METADATA
                .get_or_init(build_generated_schema_metadata)
                .as_ref()
                .map_err(Clone::clone)?;
            StorageManifest::current(
                metadata.canonical_ddl_digest.clone(),
                metadata.integrity_constraints_digest.clone(),
            )
            .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|detail| {
            StoreError::schema_invariant(
                "canonical_sql",
                format!("current storage manifest is unavailable: {detail}"),
            )
        })
}

/// Returns the one canonical UTF-8 JSON encoding persisted in both carriers.
pub fn current_storage_manifest_json() -> StoreResult<&'static str> {
    CURRENT_STORAGE_MANIFEST_JSON
        .get_or_init(|| {
            let manifest = CURRENT_STORAGE_MANIFEST
                .get_or_init(|| {
                    let metadata = GENERATED_SCHEMA_METADATA
                        .get_or_init(build_generated_schema_metadata)
                        .as_ref()
                        .map_err(Clone::clone)?;
                    StorageManifest::current(
                        metadata.canonical_ddl_digest.clone(),
                        metadata.integrity_constraints_digest.clone(),
                    )
                    .map_err(|error| error.to_string())
                })
                .as_ref()
                .map_err(Clone::clone)?;
            canonical_json_string(manifest).map_err(|error| error.to_string())
        })
        .as_ref()
        .map(String::as_str)
        .map_err(|detail| {
            StoreError::schema_invariant(
                "canonical_sql",
                format!("canonical storage manifest encoding is unavailable: {detail}"),
            )
        })
}

pub(crate) fn current_schema_facts(
    database: StorageDatabaseKind,
) -> StoreResult<GeneratedSchemaFacts> {
    let metadata = generated_schema_metadata()?;
    Ok(GeneratedSchemaFacts {
        tables: metadata
            .tables
            .iter()
            .filter(|fact| fact.database == database)
            .cloned()
            .collect(),
        columns: metadata
            .columns
            .iter()
            .filter(|fact| fact.database == database)
            .cloned()
            .collect(),
        indexes: metadata
            .indexes
            .iter()
            .filter(|fact| fact.database == database)
            .cloned()
            .collect(),
        constraints: metadata
            .constraints
            .iter()
            .filter(|fact| fact.database == database)
            .cloned()
            .collect(),
    })
}

pub(crate) fn extract_schema_facts(
    conn: &Connection,
    database: StorageDatabaseKind,
) -> Result<GeneratedSchemaFacts, String> {
    let mut tables = Vec::new();
    let mut columns = Vec::new();
    let mut constraints = Vec::new();
    let mut statement = conn
        .prepare(
            "SELECT type, name, sql
               FROM sqlite_master
              WHERE type IN ('table', 'view', 'trigger')
                AND name NOT LIKE 'sqlite_%'
              ORDER BY type, name",
        )
        .map_err(|error| error.to_string())?;
    let relations = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    for relation in relations {
        let (object_type, name, canonical_sql) = relation.map_err(|error| error.to_string())?;
        let relation_kind = match object_type.as_str() {
            "table" => GeneratedRelationKind::Table,
            "view" => GeneratedRelationKind::View,
            "trigger" => GeneratedRelationKind::Trigger,
            _ => return Err(format!("unsupported SQLite object type {object_type}")),
        };
        tables.push(GeneratedTable {
            database,
            relation_kind,
            name: name.clone(),
            canonical_sql: canonical_sql.clone(),
        });

        if relation_kind != GeneratedRelationKind::Trigger {
            columns.extend(read_columns(conn, database, &name)?);
        }
        if relation_kind == GeneratedRelationKind::Table {
            constraints.push(GeneratedConstraint {
                database,
                table: name,
                canonical_table_sql: canonical_sql,
            });
        }
    }

    let mut indexes = read_indexes(conn, database)?;
    tables.sort();
    columns.sort();
    indexes.sort();
    constraints.sort();
    Ok(GeneratedSchemaFacts {
        tables,
        columns,
        indexes,
        constraints,
    })
}

fn build_generated_schema_metadata() -> Result<GeneratedSchemaMetadata, String> {
    validate_canonical_sql_source(REGISTRY_DATABASE_KIND, REGISTRY_SCHEMA_SQL)?;
    validate_canonical_sql_source(PROJECT_STATE_DATABASE_KIND, PROJECT_STATE_SCHEMA_SQL)?;
    extract_generated_schema_metadata(REGISTRY_SCHEMA_SQL, PROJECT_STATE_SCHEMA_SQL)
}

fn extract_generated_schema_metadata(
    registry_sql: &str,
    project_sql: &str,
) -> Result<GeneratedSchemaMetadata, String> {
    let registry = canonical_facts_from_sql(StorageDatabaseKind::Registry, registry_sql)?;
    let project = canonical_facts_from_sql(StorageDatabaseKind::ProjectState, project_sql)?;

    let mut tables = registry.tables;
    tables.extend(project.tables);
    let mut columns = registry.columns;
    columns.extend(project.columns);
    let mut indexes = registry.indexes;
    indexes.extend(project.indexes);
    let mut constraints = registry.constraints;
    constraints.extend(project.constraints);
    tables.sort();
    columns.sort();
    indexes.sort();
    constraints.sort();

    let digest_input = SchemaDigestInput {
        tables: &tables,
        columns: &columns,
        indexes: &indexes,
        constraints: &constraints,
    };
    let canonical_ddl_digest = sha256_canonical_json(&digest_input)?;
    let integrity_constraints_digest = sha256_canonical_json(&constraints)?;
    Ok(GeneratedSchemaMetadata {
        tables,
        columns,
        indexes,
        constraints,
        canonical_ddl_digest,
        integrity_constraints_digest,
    })
}

fn validate_canonical_sql_source(source: &str, sql: &str) -> Result<(), String> {
    let bytes = sql.as_bytes();
    if bytes.contains(&b'\r') {
        return Err(format!(
            "{source} canonical SQL source contains a carriage-return byte"
        ));
    }
    if !bytes.ends_with(b"\n") {
        return Err(format!(
            "{source} canonical SQL source must end with one final LF"
        ));
    }
    if bytes.ends_with(b"\n\n") {
        return Err(format!(
            "{source} canonical SQL source must not end with more than one final LF"
        ));
    }
    Ok(())
}

fn canonical_facts_from_sql(
    database: StorageDatabaseKind,
    sql: &str,
) -> Result<GeneratedSchemaFacts, String> {
    let conn = Connection::open_in_memory().map_err(|error| error.to_string())?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|error| error.to_string())?;
    conn.execute_batch(sql).map_err(|error| error.to_string())?;
    extract_schema_facts(&conn, database)
}

fn read_columns(
    conn: &Connection,
    database: StorageDatabaseKind,
    table: &str,
) -> Result<Vec<GeneratedColumn>, String> {
    let sql = format!("PRAGMA table_xinfo({})", quoted_identifier(table));
    let mut statement = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            let ordinal = u32::try_from(row.get::<_, i64>(0)?).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Integer,
                    Box::new(error),
                )
            })?;
            let primary_key_ordinal = u32::try_from(row.get::<_, i64>(5)?).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Integer,
                    Box::new(error),
                )
            })?;
            let hidden = u32::try_from(row.get::<_, i64>(6)?).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Integer,
                    Box::new(error),
                )
            })?;
            Ok(GeneratedColumn {
                database,
                table: table.to_owned(),
                ordinal,
                name: row.get(1)?,
                declared_type: row.get(2)?,
                not_null: row.get::<_, i64>(3)? != 0,
                default_value: row.get(4)?,
                primary_key_ordinal,
                hidden,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

fn read_indexes(
    conn: &Connection,
    database: StorageDatabaseKind,
) -> Result<Vec<GeneratedIndex>, String> {
    let mut statement = conn
        .prepare(
            "SELECT name, tbl_name, sql
               FROM sqlite_master
              WHERE type = 'index'
                AND name NOT LIKE 'sqlite_%'
              ORDER BY name",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut indexes = Vec::new();
    for row in rows {
        let (name, table, canonical_sql) = row.map_err(|error| error.to_string())?;
        let (unique, partial) = read_index_flags(conn, &table, &name)?;
        indexes.push(GeneratedIndex {
            database,
            table,
            name,
            unique,
            partial,
            canonical_sql,
        });
    }
    Ok(indexes)
}

fn read_index_flags(conn: &Connection, table: &str, index: &str) -> Result<(bool, bool), String> {
    let sql = format!("PRAGMA index_list({})", quoted_identifier(table));
    let mut statement = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let mut rows = statement.query([]).map_err(|error| error.to_string())?;
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        if row.get::<_, String>(1).map_err(|error| error.to_string())? == index {
            return Ok((
                row.get::<_, i64>(2).map_err(|error| error.to_string())? != 0,
                row.get::<_, i64>(4).map_err(|error| error.to_string())? != 0,
            ));
        }
    }
    Err(format!("index {index} is missing from PRAGMA index_list"))
}

fn quoted_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn sha256_canonical_json(value: &impl Serialize) -> Result<String, String> {
    let bytes = canonical_json_bytes(value).map_err(|error| error.to_string())?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{digest:x}"))
}

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
    validate_canonical_sql_source(database_kind, sql)
        .map_err(|detail| StoreError::schema_invariant(database_kind, detail))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    const CANONICAL_DDL_DIGEST: &str =
        "sha256:3786b445765cc41fdc643e7837a65b21ce361ceaa846feb347ecd8318f535150";
    const INTEGRITY_CONSTRAINTS_DIGEST: &str =
        "sha256:8b334c33827cb8158200cf5769da60f58dc528e35eb54057ed4c53aae0a8d085";

    #[test]
    fn canonical_schema_bytes_are_strict_and_crlf_changes_storage_identity() {
        validate_canonical_sql_source(REGISTRY_DATABASE_KIND, REGISTRY_SCHEMA_SQL)
            .expect("LF registry SQL source must be canonical");
        validate_canonical_sql_source(PROJECT_STATE_DATABASE_KIND, PROJECT_STATE_SCHEMA_SQL)
            .expect("LF project SQL source must be canonical");

        let canonical =
            extract_generated_schema_metadata(REGISTRY_SCHEMA_SQL, PROJECT_STATE_SCHEMA_SQL)
                .expect("extract canonical schema metadata");
        assert_eq!(canonical.canonical_ddl_digest, CANONICAL_DDL_DIGEST);
        assert_eq!(
            canonical.integrity_constraints_digest,
            INTEGRITY_CONSTRAINTS_DIGEST
        );

        assert_crlf_source_is_noncanonical_and_changes_identity(
            REGISTRY_DATABASE_KIND,
            REGISTRY_SCHEMA_SQL,
            PROJECT_STATE_SCHEMA_SQL,
            true,
            &canonical,
        );
        assert_crlf_source_is_noncanonical_and_changes_identity(
            PROJECT_STATE_DATABASE_KIND,
            REGISTRY_SCHEMA_SQL,
            PROJECT_STATE_SCHEMA_SQL,
            false,
            &canonical,
        );
    }

    fn assert_crlf_source_is_noncanonical_and_changes_identity(
        source: &str,
        registry_sql: &str,
        project_sql: &str,
        mutate_registry: bool,
        canonical: &GeneratedSchemaMetadata,
    ) {
        let mutated_registry = if mutate_registry {
            registry_sql.replace('\n', "\r\n")
        } else {
            registry_sql.to_owned()
        };
        let mutated_project = if mutate_registry {
            project_sql.to_owned()
        } else {
            project_sql.replace('\n', "\r\n")
        };
        let mutated_source = if mutate_registry {
            &mutated_registry
        } else {
            &mutated_project
        };

        let error = validate_canonical_sql_source(source, mutated_source)
            .expect_err("CRLF schema SQL must be rejected as non-canonical");
        assert!(error.contains("carriage-return byte"), "{error}");

        let mutated = extract_generated_schema_metadata(&mutated_registry, &mutated_project)
            .expect("extract CRLF-mutated schema metadata for identity comparison");
        assert_ne!(
            mutated.canonical_ddl_digest, canonical.canonical_ddl_digest,
            "{source} CRLF mutation must change the DDL identity"
        );
        assert_ne!(
            mutated.integrity_constraints_digest, canonical.integrity_constraints_digest,
            "{source} CRLF mutation must change the integrity identity"
        );
    }
}
