use std::{
    fs,
    path::{Path, PathBuf},
    str,
};

use rusqlite::{types::ValueRef, Connection, Row};
use serde_json::{Map, Number, Value};

use crate::{
    bootstrap::{validate_current_project_registration, ProjectRecord},
    schema::PROJECT_STATE_DATABASE_KIND,
    sqlite::{
        open_read_only_database, registry_db_path, validate_project_state_schema,
        validate_registry_schema, ARTIFACTS_DIR,
    },
    StoreError, StoreResult,
};

const PROJECT_STATE_EXPORT_TABLES: &[&str] = &[
    "project_state",
    "agent_sessions",
    "guard_events",
    "prompt_captures",
    "expected_writes",
    "unrecorded_changes",
    "session_watch_baselines",
    "session_watch_observations",
    "tasks",
    "change_units",
    "evidence_capture_intents",
    "user_judgments",
    "local_web_consent_tokens",
    "project_continuity_records",
    "write_tickets",
    "runs",
    "artifact_staging",
    "evidence_capture_receipts",
    "evidence_capture_source_claims",
    "artifacts",
    "artifact_links",
    "evidence_summaries",
    "evidence_observations",
    "user_evidence_observations",
    "evidence_producers",
    "blockers",
    "authority_events",
    "tool_invocations",
];

/// Read-only export snapshot for a registered project's authority bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthorityBundleSnapshot {
    pub project: ProjectRecord,
    pub records: Vec<AuthorityBundleRecord>,
    pub table_counts: Vec<AuthorityBundleTableCount>,
    pub artifacts: Vec<AuthorityBundleArtifact>,
}

/// One exported storage row.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthorityBundleRecord {
    pub database: &'static str,
    pub table: String,
    pub row: Value,
}

/// Exported row count for one storage table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityBundleTableCount {
    pub database: &'static str,
    pub table: String,
    pub row_count: usize,
}

/// Persistent artifact metadata needed by an export bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityBundleArtifact {
    pub artifact_id: String,
    pub body_path: Option<String>,
    pub stored_sha256: Option<String>,
    pub size_bytes: Option<u64>,
    pub content_type: Option<String>,
    pub status: String,
    pub integrity_status: String,
    pub source_path: Option<PathBuf>,
}

/// Reads a registered project's exportable authority records without creating,
/// migrating, or repairing Runtime Home state.
pub fn read_authority_bundle_snapshot(
    runtime_home: impl AsRef<Path>,
    repo_root: impl AsRef<Path>,
) -> StoreResult<AuthorityBundleSnapshot> {
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let project = project_record_by_repo_root_read_only(&runtime_home, repo_root)?;
    let conn = open_read_only_database(&project.state_db_path)?;
    validate_project_state_schema(&conn)?;

    let (records, table_counts) = export_project_state_records(&conn)?;
    let artifacts = export_artifact_records(&conn, &project)?;

    Ok(AuthorityBundleSnapshot {
        project,
        records,
        table_counts,
        artifacts,
    })
}

fn project_record_by_repo_root_read_only(
    runtime_home: &Path,
    repo_root: impl AsRef<Path>,
) -> StoreResult<ProjectRecord> {
    let selected_repo = fs::canonicalize(repo_root.as_ref()).map_err(StoreError::Io)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Err(StoreError::NotFound {
            entity: "project",
            id: selected_repo.display().to_string(),
        });
    }

    let conn = open_read_only_database(registry_path)?;
    validate_registry_schema(&conn)?;
    let mut stmt = conn.prepare(
        "SELECT
            project_internal_id,
            project_name,
            project_alias,
            runtime_home_id,
            repo_root,
            project_home,
            state_db_path,
            status,
            metadata_json
         FROM projects
         ORDER BY project_name, project_internal_id",
    )?;
    let rows = stmt.query_map([], project_record_from_row)?;
    for row in rows {
        let project = validate_current_project_registration(runtime_home, &row?)?;
        if project.repo_root == selected_repo {
            return Ok(project);
        }
    }

    Err(StoreError::NotFound {
        entity: "project",
        id: selected_repo.display().to_string(),
    })
}

fn project_record_from_row(row: &Row<'_>) -> rusqlite::Result<ProjectRecord> {
    let project_internal_id = row.get::<_, String>(0)?;
    Ok(ProjectRecord {
        project_id: project_internal_id.clone(),
        project_internal_id,
        project_name: row.get(1)?,
        project_alias: row.get(2)?,
        runtime_home_id: row.get(3)?,
        repo_root: PathBuf::from(row.get::<_, String>(4)?),
        project_home: PathBuf::from(row.get::<_, String>(5)?),
        state_db_path: PathBuf::from(row.get::<_, String>(6)?),
        status: row.get(7)?,
        metadata_json: row.get(8)?,
    })
}

fn export_project_state_records(
    conn: &Connection,
) -> StoreResult<(Vec<AuthorityBundleRecord>, Vec<AuthorityBundleTableCount>)> {
    let mut records = Vec::new();
    let mut table_counts = Vec::new();
    for table in PROJECT_STATE_EXPORT_TABLES {
        let table_records = export_table_records(conn, table)?;
        table_counts.push(AuthorityBundleTableCount {
            database: PROJECT_STATE_DATABASE_KIND,
            table: (*table).to_owned(),
            row_count: table_records.len(),
        });
        records.extend(table_records);
    }
    Ok((records, table_counts))
}

fn export_table_records(
    conn: &Connection,
    table: &'static str,
) -> StoreResult<Vec<AuthorityBundleRecord>> {
    let columns = table_columns(conn, table)?;
    let sql = format!(
        "SELECT * FROM {table} ORDER BY {}",
        columns
            .iter()
            .map(|column| quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| row_json(row, table, &columns))?;

    let mut records = Vec::new();
    for row in rows {
        records.push(AuthorityBundleRecord {
            database: PROJECT_STATE_DATABASE_KIND,
            table: table.to_owned(),
            row: row.map_err(StoreError::from)?,
        });
    }
    Ok(records)
}

fn export_artifact_records(
    conn: &Connection,
    project: &ProjectRecord,
) -> StoreResult<Vec<AuthorityBundleArtifact>> {
    let mut stmt = conn.prepare(
        "SELECT
            artifact_id,
            body_path,
            sha256,
            size_bytes,
            content_type,
            status,
            integrity_status
         FROM artifacts
         ORDER BY artifact_id",
    )?;
    let rows = stmt.query_map([], |row| {
        let body_path = row.get::<_, Option<String>>(1)?;
        Ok(AuthorityBundleArtifact {
            artifact_id: row.get(0)?,
            source_path: body_path
                .as_deref()
                .and_then(|path| artifact_source_path(project, path)),
            body_path,
            stored_sha256: row.get(2)?,
            size_bytes: row.get::<_, Option<i64>>(3)?.and_then(nonnegative_u64),
            content_type: row.get(4)?,
            status: row.get(5)?,
            integrity_status: row.get(6)?,
        })
    })?;

    let mut artifacts = Vec::new();
    for row in rows {
        artifacts.push(row?);
    }
    Ok(artifacts)
}

fn artifact_source_path(project: &ProjectRecord, body_path: &str) -> Option<PathBuf> {
    let relative = safe_relative_path(body_path)?;
    Some(project.project_home.join(ARTIFACTS_DIR).join(relative))
}

fn safe_relative_path(path: &str) -> Option<PathBuf> {
    if path.trim().is_empty() {
        return None;
    }
    let mut output = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            std::path::Component::Normal(part) => output.push(part),
            _ => return None,
        }
    }
    if output.as_os_str().is_empty() {
        None
    } else {
        Some(output)
    }
}

fn nonnegative_u64(value: i64) -> Option<u64> {
    u64::try_from(value).ok()
}

fn row_json(row: &Row<'_>, table: &str, columns: &[String]) -> rusqlite::Result<Value> {
    let mut object = Map::new();
    for (index, column) in columns.iter().enumerate() {
        object.insert(
            column.clone(),
            json_value(row.get_ref(index)?, table, column)?,
        );
    }
    Ok(Value::Object(object))
}

fn json_value(value: ValueRef<'_>, table: &str, column: &str) -> rusqlite::Result<Value> {
    match value {
        ValueRef::Null => Ok(Value::Null),
        ValueRef::Integer(value) => Ok(Value::Number(Number::from(value))),
        ValueRef::Real(value) => Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| invalid_value_error(table, column, "non-finite REAL value")),
        ValueRef::Text(value) => str::from_utf8(value)
            .map(|text| Value::String(text.to_owned()))
            .map_err(|_| invalid_value_error(table, column, "non-UTF-8 TEXT value")),
        ValueRef::Blob(_) => Err(invalid_value_error(
            table,
            column,
            "BLOB values are outside the baseline export record model",
        )),
    }
}

fn invalid_value_error(table: &str, column: &str, detail: &str) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(format!("{table}.{column} contains {detail}").into())
}

fn table_columns(conn: &Connection, table: &str) -> StoreResult<Vec<String>> {
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
    let columns = columns
        .into_iter()
        .map(|(_, name)| name)
        .collect::<Vec<_>>();
    if columns.is_empty() {
        return Err(StoreError::schema_invariant(
            PROJECT_STATE_DATABASE_KIND,
            format!("export table {table} has no columns"),
        ));
    }
    Ok(columns)
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}
