use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::{
    bootstrap::{validate_project_record_for_execution, ProjectRecord},
    migrations::{
        expected_project_state_migrations, expected_registry_migrations,
        PROJECT_STATE_DATABASE_KIND, PROJECT_STATE_SCHEMA_VERSION, REGISTRY_DATABASE_KIND,
        REGISTRY_SCHEMA_VERSION, STORAGE_PROFILE,
    },
    sqlite::{open_read_only_database, registry_db_path},
};

/// Read-only inspection result for a selected `Harness Runtime Home`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHomeInspection {
    pub runtime_home: PathBuf,
    pub registry_db_path: PathBuf,
    pub registry: RegistryDatabaseInspection,
}

/// Read-only inspection result for `registry.sqlite`.
pub type RegistryDatabaseInspection = DatabaseInspection<RegistryInspectionSnapshot>;

/// Read-only inspection result for project `state.sqlite`.
pub type ProjectStateDatabaseInspection = DatabaseInspection<ProjectStateInspectionSnapshot>;

/// Structured database inspection state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseInspection<T> {
    Missing {
        path: PathBuf,
    },
    Present(T),
    Unsupported {
        path: PathBuf,
        detected_version: i64,
        latest_supported_version: i64,
        detail: String,
    },
    Malformed {
        path: PathBuf,
        detail: String,
    },
    Unreadable {
        path: PathBuf,
        detail: String,
    },
}

/// Supported schema state for an inspectable database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectionSchemaState {
    Current {
        version: i64,
    },
    MigrationRequired {
        detected_version: i64,
        latest_supported_version: i64,
    },
}

impl InspectionSchemaState {
    pub const fn detected_version(&self) -> i64 {
        match self {
            Self::Current { version } => *version,
            Self::MigrationRequired {
                detected_version, ..
            } => *detected_version,
        }
    }
}

/// Current readable registry data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryInspectionSnapshot {
    pub path: PathBuf,
    pub schema: InspectionSchemaState,
    pub runtime_home: RuntimeHomeInspectionRecord,
    pub projects: Vec<ProjectInspectionRecord>,
}

/// Runtime Home singleton row read from `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHomeInspectionRecord {
    pub runtime_home_id: String,
    pub storage_profile: String,
    pub schema_version: i64,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Registered project row plus its project-state inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInspectionRecord {
    pub project_id: String,
    pub runtime_home_id: String,
    pub repo_root: PathBuf,
    pub project_home: PathBuf,
    pub state_db_path: PathBuf,
    pub status: String,
    pub metadata_json: String,
    pub project_state: ProjectStateDatabaseInspection,
}

/// Current or supported historical project-state data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStateInspectionSnapshot {
    pub path: PathBuf,
    pub schema: InspectionSchemaState,
    pub project_state: ProjectStateInspectionRecord,
    pub surfaces: Vec<SurfaceInspectionRecord>,
}

/// Project-state header row needed by setup planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStateInspectionRecord {
    pub project_id: String,
    pub storage_profile: String,
    pub schema_version: i64,
    pub state_version: i64,
    pub default_surface_id: Option<String>,
    pub default_surface_instance_id: Option<String>,
    pub metadata_json: String,
}

/// Surface row read from project `state.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceInspectionRecord {
    pub project_id: String,
    pub surface_id: String,
    pub surface_instance_id: String,
    pub surface_kind: String,
    pub interaction_role: String,
    pub interaction_role_source: SurfaceInteractionRoleSource,
    pub display_name: Option<String>,
    pub capability_profile_json: String,
    pub local_access_json: String,
    pub metadata_json: String,
}

/// Whether `surfaces.interaction_role` was stored or inferred from migration history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceInteractionRoleSource {
    Stored,
    HistoricalDefault,
}

#[derive(Debug)]
enum InspectionIssue {
    Malformed(String),
    Unsupported {
        detected_version: i64,
        detail: String,
    },
    Unreadable(String),
}

#[derive(Debug)]
struct MigrationRow {
    database_kind: String,
    version: i64,
    name: String,
    storage_profile: String,
}

#[derive(Debug)]
struct ProjectRegistryRow {
    project_id: String,
    runtime_home_id: String,
    repo_root: PathBuf,
    project_home: PathBuf,
    state_db_path: PathBuf,
    status: String,
    metadata_json: String,
}

/// Inspects a Runtime Home without creating files, opening writable databases, or migrating.
pub fn inspect_runtime_home(runtime_home: impl AsRef<Path>) -> RuntimeHomeInspection {
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_db_path = registry_db_path(&runtime_home);
    let registry = inspect_registry_database_at(&registry_db_path, &runtime_home);

    RuntimeHomeInspection {
        runtime_home,
        registry_db_path,
        registry,
    }
}

/// Inspects `registry.sqlite` under a Runtime Home.
pub fn inspect_registry_database(runtime_home: impl AsRef<Path>) -> RegistryDatabaseInspection {
    let runtime_home = runtime_home.as_ref();
    inspect_registry_database_at(&registry_db_path(runtime_home), runtime_home)
}

/// Inspects one project-state database for a registered project id.
pub fn inspect_project_state_database(
    path: impl AsRef<Path>,
    project_id: &str,
) -> ProjectStateDatabaseInspection {
    inspect_project_state_database_at(path.as_ref(), project_id)
}

fn inspect_registry_database_at(path: &Path, runtime_home: &Path) -> RegistryDatabaseInspection {
    if let Some(missing) = missing_database(path) {
        return missing;
    }

    let conn = match open_read_only_database(path) {
        Ok(conn) => conn,
        Err(error) => return unreadable(path, error),
    };

    let schema = match inspect_migration_history(
        &conn,
        REGISTRY_DATABASE_KIND,
        REGISTRY_SCHEMA_VERSION,
        &expected_registry_migrations(),
    ) {
        Ok(schema) => schema,
        Err(issue) => return issue.into_database_inspection(path, REGISTRY_SCHEMA_VERSION),
    };

    if let Err(issue) = validate_registry_required_schema(&conn, schema.detected_version()) {
        return issue.into_database_inspection(path, REGISTRY_SCHEMA_VERSION);
    }

    let runtime_home_record = match read_runtime_home_record(&conn, schema.detected_version()) {
        Ok(record) => record,
        Err(issue) => return issue.into_database_inspection(path, REGISTRY_SCHEMA_VERSION),
    };

    let project_rows = match read_project_rows(&conn, &runtime_home_record.runtime_home_id) {
        Ok(rows) => rows,
        Err(issue) => return issue.into_database_inspection(path, REGISTRY_SCHEMA_VERSION),
    };

    let projects = project_rows
        .into_iter()
        .map(|row| {
            let project = ProjectRecord {
                project_id: row.project_id,
                runtime_home_id: row.runtime_home_id,
                repo_root: row.repo_root,
                project_home: row.project_home,
                state_db_path: row.state_db_path,
                status: row.status,
                metadata_json: row.metadata_json,
            };
            let project_state = inspect_registered_project_state(runtime_home, &project);
            ProjectInspectionRecord {
                project_id: project.project_id,
                runtime_home_id: project.runtime_home_id,
                repo_root: project.repo_root,
                project_home: project.project_home,
                state_db_path: project.state_db_path,
                status: project.status,
                metadata_json: project.metadata_json,
                project_state,
            }
        })
        .collect();

    DatabaseInspection::Present(RegistryInspectionSnapshot {
        path: path.to_path_buf(),
        schema,
        runtime_home: runtime_home_record,
        projects,
    })
}

fn inspect_registered_project_state(
    runtime_home: &Path,
    project: &ProjectRecord,
) -> ProjectStateDatabaseInspection {
    match validate_project_record_for_execution(runtime_home, project) {
        Ok(project) => {
            inspect_project_state_database_at(&project.state_db_path, &project.project_id)
        }
        Err(error) => malformed(&project.state_db_path, error.to_string()),
    }
}

fn inspect_project_state_database_at(
    path: &Path,
    project_id: &str,
) -> ProjectStateDatabaseInspection {
    if project_id.trim().is_empty() {
        return malformed(path, "project_id must not be empty");
    }
    if let Some(missing) = missing_database(path) {
        return missing;
    }

    let conn = match open_read_only_database(path) {
        Ok(conn) => conn,
        Err(error) => return unreadable(path, error),
    };

    let schema = match inspect_migration_history(
        &conn,
        PROJECT_STATE_DATABASE_KIND,
        PROJECT_STATE_SCHEMA_VERSION,
        &expected_project_state_migrations(),
    ) {
        Ok(schema) => schema,
        Err(issue) => return issue.into_database_inspection(path, PROJECT_STATE_SCHEMA_VERSION),
    };
    let detected_version = schema.detected_version();

    if let Err(issue) = validate_project_state_required_schema(&conn, detected_version) {
        return issue.into_database_inspection(path, PROJECT_STATE_SCHEMA_VERSION);
    }

    let project_state = match read_project_state_record(&conn, project_id, detected_version) {
        Ok(record) => record,
        Err(issue) => return issue.into_database_inspection(path, PROJECT_STATE_SCHEMA_VERSION),
    };
    let surfaces = match read_surface_rows(&conn, project_id, detected_version) {
        Ok(surfaces) => surfaces,
        Err(issue) => return issue.into_database_inspection(path, PROJECT_STATE_SCHEMA_VERSION),
    };

    DatabaseInspection::Present(ProjectStateInspectionSnapshot {
        path: path.to_path_buf(),
        schema,
        project_state,
        surfaces,
    })
}

impl InspectionIssue {
    fn into_database_inspection<T>(
        self,
        path: &Path,
        latest_supported_version: i64,
    ) -> DatabaseInspection<T> {
        match self {
            Self::Malformed(detail) => malformed(path, detail),
            Self::Unsupported {
                detected_version,
                detail,
            } => DatabaseInspection::Unsupported {
                path: path.to_path_buf(),
                detected_version,
                latest_supported_version,
                detail,
            },
            Self::Unreadable(detail) => DatabaseInspection::Unreadable {
                path: path.to_path_buf(),
                detail,
            },
        }
    }
}

fn missing_database<T>(path: &Path) -> Option<DatabaseInspection<T>> {
    match path.try_exists() {
        Ok(true) => None,
        Ok(false) => Some(DatabaseInspection::Missing {
            path: path.to_path_buf(),
        }),
        Err(error) => Some(DatabaseInspection::Unreadable {
            path: path.to_path_buf(),
            detail: error.to_string(),
        }),
    }
}

fn malformed<T>(path: &Path, detail: impl Into<String>) -> DatabaseInspection<T> {
    DatabaseInspection::Malformed {
        path: path.to_path_buf(),
        detail: detail.into(),
    }
}

fn unreadable<T>(path: &Path, error: impl ToString) -> DatabaseInspection<T> {
    DatabaseInspection::Unreadable {
        path: path.to_path_buf(),
        detail: error.to_string(),
    }
}

fn inspect_migration_history(
    conn: &Connection,
    database_kind: &'static str,
    latest_supported_version: i64,
    expected: &[crate::migrations::ExpectedMigration],
) -> Result<InspectionSchemaState, InspectionIssue> {
    require_table(conn, database_kind, "schema_migrations")?;
    require_columns(
        conn,
        database_kind,
        "schema_migrations",
        &[
            "database_kind",
            "version",
            "name",
            "storage_profile",
            "applied_at",
            "checksum_sha256",
            "metadata_json",
        ],
    )?;

    let mut stmt = conn
        .prepare(
            "SELECT database_kind, version, name, storage_profile
               FROM schema_migrations
              ORDER BY version, database_kind",
        )
        .map_err(sqlite_unreadable)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(MigrationRow {
                database_kind: row.get(0)?,
                version: row.get(1)?,
                name: row.get(2)?,
                storage_profile: row.get(3)?,
            })
        })
        .map_err(sqlite_unreadable)?;

    let mut actual = Vec::new();
    for row in rows {
        actual.push(row.map_err(|error| {
            InspectionIssue::Malformed(format!("could not decode schema_migrations row: {error}"))
        })?);
    }

    if actual.is_empty() {
        return Err(InspectionIssue::Malformed(
            "schema_migrations has no rows".to_owned(),
        ));
    }
    if actual.iter().any(|row| row.database_kind != database_kind) {
        return Err(InspectionIssue::Malformed(format!(
            "schema_migrations contains rows outside {database_kind}"
        )));
    }

    let mut seen_versions = BTreeSet::new();
    for row in &actual {
        if !seen_versions.insert(row.version) {
            return Err(InspectionIssue::Malformed(format!(
                "schema_migrations has duplicate version {}",
                row.version
            )));
        }
        if row.version > latest_supported_version {
            return Err(InspectionIssue::Unsupported {
                detected_version: row.version,
                detail: format!(
                    "{database_kind} migration version {} is newer than supported version {latest_supported_version}",
                    row.version
                ),
            });
        }
    }

    if actual.len() > expected.len() {
        return Err(InspectionIssue::Unsupported {
            detected_version: actual
                .last()
                .map(|row| row.version)
                .unwrap_or(latest_supported_version),
            detail: format!(
                "{database_kind} migration history has more rows than the compiled catalog"
            ),
        });
    }

    for (index, row) in actual.iter().enumerate() {
        let expected_row = expected[index];
        if row.version != expected_row.version
            || row.name != expected_row.name
            || row.storage_profile != STORAGE_PROFILE
        {
            return Err(InspectionIssue::Malformed(format!(
                "schema_migrations row {index} is version={} name={} profile={}, expected version={} name={} profile={}",
                row.version,
                row.name,
                row.storage_profile,
                expected_row.version,
                expected_row.name,
                STORAGE_PROFILE
            )));
        }
    }

    let detected_version = actual
        .last()
        .map(|row| row.version)
        .ok_or_else(|| InspectionIssue::Malformed("schema_migrations has no rows".to_owned()))?;
    if detected_version == latest_supported_version && actual.len() == expected.len() {
        Ok(InspectionSchemaState::Current {
            version: detected_version,
        })
    } else {
        Ok(InspectionSchemaState::MigrationRequired {
            detected_version,
            latest_supported_version,
        })
    }
}

fn validate_registry_required_schema(
    conn: &Connection,
    detected_version: i64,
) -> Result<(), InspectionIssue> {
    require_tables(conn, REGISTRY_DATABASE_KIND, &["runtime_home", "projects"])?;
    require_columns(
        conn,
        REGISTRY_DATABASE_KIND,
        "runtime_home",
        &[
            "singleton_id",
            "runtime_home_id",
            "storage_profile",
            "schema_version",
            "created_at",
            "updated_at",
            "metadata_json",
        ],
    )?;
    require_columns(
        conn,
        REGISTRY_DATABASE_KIND,
        "projects",
        &[
            "project_id",
            "runtime_home_id",
            "repo_root",
            "project_home",
            "state_db_path",
            "status",
            "metadata_json",
        ],
    )?;
    if detected_version >= 2 {
        require_tables(
            conn,
            REGISTRY_DATABASE_KIND,
            &[
                "agent_integrations",
                "integration_projects",
                "host_installations",
            ],
        )?;
        require_columns(
            conn,
            REGISTRY_DATABASE_KIND,
            "agent_integrations",
            &[
                "integration_id",
                "interaction_role",
                "surface_id",
                "surface_instance_id",
                "default_project_id",
                "enabled",
                "created_at",
                "updated_at",
                "metadata_json",
            ],
        )?;
        require_columns(
            conn,
            REGISTRY_DATABASE_KIND,
            "integration_projects",
            &["integration_id", "project_id", "created_at"],
        )?;
        require_columns(
            conn,
            REGISTRY_DATABASE_KIND,
            "host_installations",
            &[
                "installation_id",
                "integration_id",
                "host_kind",
                "host_scope",
                "server_name",
                "config_target",
                "managed_fingerprint",
                "last_verified_status",
                "created_at",
                "updated_at",
                "metadata_json",
            ],
        )?;
    }
    Ok(())
}

fn validate_project_state_required_schema(
    conn: &Connection,
    detected_version: i64,
) -> Result<(), InspectionIssue> {
    require_tables(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        &["project_state", "surfaces"],
    )?;
    require_columns(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "project_state",
        &[
            "project_id",
            "storage_profile",
            "schema_version",
            "state_version",
            "default_surface_id",
            "default_surface_instance_id",
            "metadata_json",
        ],
    )?;
    require_columns(
        conn,
        PROJECT_STATE_DATABASE_KIND,
        "surfaces",
        &[
            "project_id",
            "surface_id",
            "surface_instance_id",
            "surface_kind",
            "display_name",
            "capability_profile_json",
            "local_access_json",
            "metadata_json",
        ],
    )?;

    let has_role = column_exists(conn, "surfaces", "interaction_role")?;
    if detected_version >= 7 && !has_role {
        return Err(InspectionIssue::Malformed(
            "missing column surfaces.interaction_role".to_owned(),
        ));
    }
    if detected_version < 7 && has_role {
        return Err(InspectionIssue::Malformed(
            "surfaces.interaction_role exists before migration version 7".to_owned(),
        ));
    }

    Ok(())
}

fn require_tables(
    conn: &Connection,
    database_kind: &'static str,
    tables: &[&str],
) -> Result<(), InspectionIssue> {
    for table in tables {
        require_table(conn, database_kind, table)?;
    }
    Ok(())
}

fn require_table(
    conn: &Connection,
    database_kind: &'static str,
    table: &str,
) -> Result<(), InspectionIssue> {
    if sqlite_object_exists(conn, "table", table)? {
        Ok(())
    } else {
        Err(InspectionIssue::Malformed(format!(
            "{database_kind} missing table {table}"
        )))
    }
}

fn require_columns(
    conn: &Connection,
    database_kind: &'static str,
    table: &str,
    columns: &[&str],
) -> Result<(), InspectionIssue> {
    for column in columns {
        if !column_exists(conn, table, column)? {
            return Err(InspectionIssue::Malformed(format!(
                "{database_kind} missing column {table}.{column}"
            )));
        }
    }
    Ok(())
}

fn sqlite_object_exists(
    conn: &Connection,
    object_type: &str,
    name: &str,
) -> Result<bool, InspectionIssue> {
    conn.query_row(
        "SELECT COUNT(*)
           FROM sqlite_master
          WHERE type = ?1 AND name = ?2",
        params![object_type, name],
        |row| Ok(row.get::<_, i64>(0)? > 0),
    )
    .map_err(sqlite_unreadable)
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, InspectionIssue> {
    let escaped_table = table.replace('"', "\"\"");
    let sql = format!("PRAGMA table_info(\"{escaped_table}\")");
    let mut stmt = conn.prepare(&sql).map_err(sqlite_unreadable)?;
    let mut rows = stmt.query([]).map_err(sqlite_unreadable)?;
    while let Some(row) = rows.next().map_err(sqlite_unreadable)? {
        let name = row.get::<_, String>(1).map_err(|error| {
            InspectionIssue::Malformed(format!("could not decode {table} column info: {error}"))
        })?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn read_runtime_home_record(
    conn: &Connection,
    detected_version: i64,
) -> Result<RuntimeHomeInspectionRecord, InspectionIssue> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM runtime_home", [], |row| row.get(0))
        .map_err(sqlite_unreadable)?;
    if count != 1 {
        return Err(InspectionIssue::Malformed(format!(
            "runtime_home has {count} rows, expected 1"
        )));
    }

    let record = conn
        .query_row(
            "SELECT
                runtime_home_id,
                storage_profile,
                schema_version,
                created_at,
                updated_at,
                metadata_json
             FROM runtime_home
             WHERE singleton_id = 1",
            [],
            |row| {
                Ok(RuntimeHomeInspectionRecord {
                    runtime_home_id: row.get(0)?,
                    storage_profile: row.get(1)?,
                    schema_version: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    metadata_json: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(registration_decode_error)?
        .ok_or_else(|| {
            InspectionIssue::Malformed(
                "runtime_home singleton row with singleton_id=1 is missing".to_owned(),
            )
        })?;

    require_nonempty("runtime_home.runtime_home_id", &record.runtime_home_id)?;
    validate_storage_profile(
        REGISTRY_DATABASE_KIND,
        detected_version,
        &record.storage_profile,
    )?;
    if record.schema_version != detected_version {
        return Err(InspectionIssue::Malformed(format!(
            "runtime_home.schema_version is {}, expected detected migration version {detected_version}",
            record.schema_version
        )));
    }
    validate_json_object("runtime_home.metadata_json", &record.metadata_json)?;
    Ok(record)
}

fn read_project_rows(
    conn: &Connection,
    runtime_home_id: &str,
) -> Result<Vec<ProjectRegistryRow>, InspectionIssue> {
    let mut stmt = conn
        .prepare(
            "SELECT
                project_id,
                runtime_home_id,
                repo_root,
                project_home,
                state_db_path,
                status,
                metadata_json
             FROM projects
             ORDER BY project_id",
        )
        .map_err(sqlite_unreadable)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(sqlite_unreadable)?;

    let mut projects = Vec::new();
    for row in rows {
        let (
            project_id,
            row_runtime_home_id,
            repo_root,
            project_home,
            state_db_path,
            status,
            metadata_json,
        ) = row.map_err(registration_decode_error)?;
        require_nonempty("projects.project_id", &project_id)?;
        require_nonempty("projects.runtime_home_id", &row_runtime_home_id)?;
        require_nonempty("projects.repo_root", &repo_root)?;
        require_nonempty("projects.project_home", &project_home)?;
        require_nonempty("projects.state_db_path", &state_db_path)?;
        if row_runtime_home_id != runtime_home_id {
            return Err(InspectionIssue::Malformed(format!(
                "project {project_id} references runtime_home_id {}, expected {runtime_home_id}",
                row_runtime_home_id
            )));
        }
        require_nonempty("projects.status", &status)?;
        validate_json_object("projects.metadata_json", &metadata_json)?;

        projects.push(ProjectRegistryRow {
            project_id,
            runtime_home_id: row_runtime_home_id,
            repo_root: PathBuf::from(repo_root),
            project_home: PathBuf::from(project_home),
            state_db_path: PathBuf::from(state_db_path),
            status,
            metadata_json,
        });
    }

    Ok(projects)
}

fn read_project_state_record(
    conn: &Connection,
    project_id: &str,
    detected_version: i64,
) -> Result<ProjectStateInspectionRecord, InspectionIssue> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
               FROM project_state
              WHERE project_id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .map_err(sqlite_unreadable)?;
    if count != 1 {
        return Err(InspectionIssue::Malformed(format!(
            "project_state row count for {project_id} is {count}, expected 1"
        )));
    }

    let record = conn
        .query_row(
            "SELECT
                project_id,
                storage_profile,
                schema_version,
                state_version,
                default_surface_id,
                default_surface_instance_id,
                metadata_json
             FROM project_state
             WHERE project_id = ?1",
            [project_id],
            |row| {
                Ok(ProjectStateInspectionRecord {
                    project_id: row.get(0)?,
                    storage_profile: row.get(1)?,
                    schema_version: row.get(2)?,
                    state_version: row.get(3)?,
                    default_surface_id: row.get(4)?,
                    default_surface_instance_id: row.get(5)?,
                    metadata_json: row.get(6)?,
                })
            },
        )
        .map_err(registration_decode_error)?;

    require_nonempty("project_state.project_id", &record.project_id)?;
    validate_storage_profile(
        PROJECT_STATE_DATABASE_KIND,
        detected_version,
        &record.storage_profile,
    )?;
    if record.schema_version != detected_version {
        return Err(InspectionIssue::Malformed(format!(
            "project_state.schema_version is {}, expected detected migration version {detected_version}",
            record.schema_version
        )));
    }
    if record.state_version < 0 {
        return Err(InspectionIssue::Malformed(
            "project_state.state_version is negative".to_owned(),
        ));
    }
    if record.default_surface_id.is_some() != record.default_surface_instance_id.is_some() {
        return Err(InspectionIssue::Malformed(
            "project_state default surface columns must both be present or both be absent"
                .to_owned(),
        ));
    }
    validate_json_object("project_state.metadata_json", &record.metadata_json)?;
    Ok(record)
}

fn read_surface_rows(
    conn: &Connection,
    project_id: &str,
    detected_version: i64,
) -> Result<Vec<SurfaceInspectionRecord>, InspectionIssue> {
    let has_stored_role = detected_version >= 7;
    let sql = if has_stored_role {
        "SELECT
            project_id,
            surface_id,
            surface_instance_id,
            surface_kind,
            interaction_role,
            display_name,
            capability_profile_json,
            local_access_json,
            metadata_json
         FROM surfaces
         WHERE project_id = ?1
         ORDER BY surface_id, surface_instance_id"
    } else {
        "SELECT
            project_id,
            surface_id,
            surface_instance_id,
            surface_kind,
            'agent' AS interaction_role,
            display_name,
            capability_profile_json,
            local_access_json,
            metadata_json
         FROM surfaces
         WHERE project_id = ?1
         ORDER BY surface_id, surface_instance_id"
    };

    let mut stmt = conn.prepare(sql).map_err(sqlite_unreadable)?;
    let rows = stmt
        .query_map([project_id], |row| {
            Ok(SurfaceInspectionRecord {
                project_id: row.get(0)?,
                surface_id: row.get(1)?,
                surface_instance_id: row.get(2)?,
                surface_kind: row.get(3)?,
                interaction_role: row.get(4)?,
                interaction_role_source: if has_stored_role {
                    SurfaceInteractionRoleSource::Stored
                } else {
                    SurfaceInteractionRoleSource::HistoricalDefault
                },
                display_name: row.get(5)?,
                capability_profile_json: row.get(6)?,
                local_access_json: row.get(7)?,
                metadata_json: row.get(8)?,
            })
        })
        .map_err(sqlite_unreadable)?;

    let mut surfaces = Vec::new();
    for row in rows {
        let surface = row.map_err(registration_decode_error)?;
        validate_surface_row(&surface)?;
        surfaces.push(surface);
    }
    Ok(surfaces)
}

fn validate_surface_row(surface: &SurfaceInspectionRecord) -> Result<(), InspectionIssue> {
    require_nonempty("surfaces.project_id", &surface.project_id)?;
    require_nonempty("surfaces.surface_id", &surface.surface_id)?;
    require_nonempty("surfaces.surface_instance_id", &surface.surface_instance_id)?;
    require_nonempty("surfaces.surface_kind", &surface.surface_kind)?;
    require_nonempty("surfaces.interaction_role", &surface.interaction_role)?;
    Ok(())
}

fn validate_storage_profile(
    database_kind: &'static str,
    detected_version: i64,
    storage_profile: &str,
) -> Result<(), InspectionIssue> {
    if storage_profile == STORAGE_PROFILE {
        Ok(())
    } else {
        Err(InspectionIssue::Unsupported {
            detected_version,
            detail: format!("{database_kind} storage_profile {storage_profile} is not supported"),
        })
    }
}

fn require_nonempty(field: &'static str, value: &str) -> Result<(), InspectionIssue> {
    if value.trim().is_empty() {
        Err(InspectionIssue::Malformed(format!(
            "{field} must not be empty"
        )))
    } else {
        Ok(())
    }
}

fn validate_json_object(field: &'static str, text: &str) -> Result<(), InspectionIssue> {
    let value = serde_json::from_str::<Value>(text).map_err(|error| {
        InspectionIssue::Malformed(format!("{field} must be JSON object text: {error}"))
    })?;
    if value.is_object() {
        Ok(())
    } else {
        Err(InspectionIssue::Malformed(format!(
            "{field} must be a JSON object"
        )))
    }
}

fn sqlite_unreadable(error: rusqlite::Error) -> InspectionIssue {
    InspectionIssue::Unreadable(error.to_string())
}

fn registration_decode_error(error: rusqlite::Error) -> InspectionIssue {
    InspectionIssue::Malformed(format!("could not decode registration row: {error}"))
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        ffi::OsString,
        fs,
        path::{Path, PathBuf},
    };

    use harness_test_support::TempRuntimeHome;
    use harness_types::{SurfaceInteractionRole, VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION};
    use rusqlite::{params, Connection};
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::{
        bootstrap::{
            initialize_runtime_home, register_project, register_surface, ProjectRecord,
            ProjectRegistration, SurfaceRegistration, ACTIVE_PROJECT_STATUS,
        },
        migrations::test_support::create_project_state_fixture_version,
        sqlite::{open_read_only_database, project_state_db_path, registry_db_path},
        StoreResult,
    };

    const PROJECT_ID: &str = "project_inspect";
    const RUNTIME_HOME_ID: &str = "runtime_home_inspect";
    const SURFACE_ID: &str = "agent_mcp";
    const SURFACE_INSTANCE_ID: &str = "agent_mcp_local";

    struct InspectionFixture {
        runtime_home: TempRuntimeHome,
        project: ProjectRecord,
    }

    #[test]
    fn missing_runtime_home_directory_is_reported_without_creation() -> Result<(), Box<dyn Error>> {
        let root = TempRuntimeHome::new("inspect-missing-runtime-root")?;
        let missing_runtime_home = root.path().join("missing-runtime-home");

        let inspection = inspect_runtime_home(&missing_runtime_home);

        assert!(matches!(
            inspection.registry,
            DatabaseInspection::Missing { .. }
        ));
        assert!(!missing_runtime_home.exists());
        Ok(())
    }

    #[test]
    fn missing_registry_database_is_reported_without_creation() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("inspect-missing-registry")?;
        let registry_path = runtime_home.registry_db_path();

        let inspection = inspect_runtime_home(runtime_home.path());

        assert!(matches!(
            inspection.registry,
            DatabaseInspection::Missing { .. }
        ));
        assert!(!registry_path.exists());
        Ok(())
    }

    #[test]
    fn current_registry_schema_is_inspected() -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-current-registry")?;

        let inspection = inspect_runtime_home(fixture.runtime_home.path());
        let snapshot = present_registry(&inspection.registry);

        assert_eq!(
            snapshot.schema,
            InspectionSchemaState::Current {
                version: REGISTRY_SCHEMA_VERSION
            }
        );
        assert_eq!(snapshot.runtime_home.runtime_home_id, RUNTIME_HOME_ID);
        assert_eq!(snapshot.projects.len(), 1);
        assert_eq!(snapshot.projects[0].project_id, PROJECT_ID);
        assert_eq!(snapshot.projects[0].status, ACTIVE_PROJECT_STATUS);
        Ok(())
    }

    #[test]
    fn current_project_state_schema_and_surfaces_are_inspected() -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-current-state")?;

        let state = inspect_project_state_database(&fixture.project.state_db_path, PROJECT_ID);
        let snapshot = present_project_state(&state);

        assert_eq!(
            snapshot.schema,
            InspectionSchemaState::Current {
                version: PROJECT_STATE_SCHEMA_VERSION
            }
        );
        assert_eq!(snapshot.project_state.project_id, PROJECT_ID);
        assert_eq!(snapshot.surfaces.len(), 1);
        assert_eq!(snapshot.surfaces[0].surface_kind, "mcp");
        assert_eq!(snapshot.surfaces[0].interaction_role, "agent");
        assert_eq!(
            snapshot.surfaces[0].interaction_role_source,
            SurfaceInteractionRoleSource::Stored
        );
        Ok(())
    }

    #[test]
    fn registry_reports_missing_project_state_database() -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-missing-project-state")?;
        fs::remove_file(&fixture.project.state_db_path)?;

        let inspection = inspect_runtime_home(fixture.runtime_home.path());
        let snapshot = present_registry(&inspection.registry);

        assert!(matches!(
            snapshot.projects[0].project_state,
            DatabaseInspection::Missing { .. }
        ));
        Ok(())
    }

    #[test]
    fn registry_reports_state_db_path_mismatch_without_inspecting_alternate(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-state-db-mismatch")?;
        let alternate_state_path = fixture
            .runtime_home
            .path()
            .join("alternate/corrupt-state.sqlite");
        fs::create_dir_all(
            alternate_state_path
                .parent()
                .expect("alternate state path has parent"),
        )?;
        fs::write(&alternate_state_path, b"not a sqlite database")?;
        replace_project_state_db_path(
            fixture.runtime_home.path(),
            PROJECT_ID,
            &alternate_state_path,
        )?;

        let inspection = inspect_runtime_home(fixture.runtime_home.path());
        let snapshot = present_registry(&inspection.registry);

        match &snapshot.projects[0].project_state {
            DatabaseInspection::Malformed { path, detail } => {
                assert_eq!(path, &alternate_state_path);
                assert!(detail.contains("state_db_path_mismatch"));
                assert!(detail.contains("state_db_path"));
            }
            other => panic!("expected malformed project-state diagnostic, got {other:?}"),
        }
        assert_eq!(fs::read(&alternate_state_path)?, b"not a sqlite database");
        Ok(())
    }

    #[test]
    fn supported_historical_project_state_schemas_are_read_without_migration(
    ) -> Result<(), Box<dyn Error>> {
        for version in 1..PROJECT_STATE_SCHEMA_VERSION {
            let fixture = historical_fixture(&format!("inspect-historical-v{version}"), version)?;
            let before_migrations =
                migration_count(&fixture.project.state_db_path, PROJECT_STATE_DATABASE_KIND)?;

            let state = inspect_project_state_database(&fixture.project.state_db_path, PROJECT_ID);

            let snapshot = present_project_state(&state);
            assert_eq!(
                snapshot.schema,
                InspectionSchemaState::MigrationRequired {
                    detected_version: version,
                    latest_supported_version: PROJECT_STATE_SCHEMA_VERSION,
                }
            );
            assert_eq!(snapshot.surfaces.len(), 1);
            assert_eq!(snapshot.surfaces[0].interaction_role, "agent");
            assert_eq!(
                snapshot.surfaces[0].interaction_role_source,
                if version < 7 {
                    SurfaceInteractionRoleSource::HistoricalDefault
                } else {
                    SurfaceInteractionRoleSource::Stored
                }
            );
            assert_eq!(
                migration_count(&fixture.project.state_db_path, PROJECT_STATE_DATABASE_KIND)?,
                before_migrations
            );
        }
        Ok(())
    }

    #[test]
    fn unsupported_migration_version_is_structured() -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-unsupported-migration")?;
        Connection::open(&fixture.project.state_db_path)?.execute(
            "INSERT INTO schema_migrations (
                database_kind,
                version,
                name,
                storage_profile,
                applied_at
            )
            VALUES (?1, 999, 'project_state_future_v999', ?2, 't_future')",
            params![PROJECT_STATE_DATABASE_KIND, STORAGE_PROFILE],
        )?;

        let state = inspect_project_state_database(&fixture.project.state_db_path, PROJECT_ID);

        assert!(matches!(
            state,
            DatabaseInspection::Unsupported {
                detected_version: 999,
                latest_supported_version: PROJECT_STATE_SCHEMA_VERSION,
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn inconsistent_migration_records_are_malformed() -> Result<(), Box<dyn Error>> {
        let fixture = historical_fixture("inspect-inconsistent-migrations", 3)?;
        Connection::open(&fixture.project.state_db_path)?
            .execute("DELETE FROM schema_migrations WHERE version = 2", [])?;

        let state = inspect_project_state_database(&fixture.project.state_db_path, PROJECT_ID);

        assert!(matches!(state, DatabaseInspection::Malformed { .. }));
        Ok(())
    }

    #[test]
    fn missing_required_surface_table_is_malformed() -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-missing-surface-table")?;
        let conn = Connection::open(&fixture.project.state_db_path)?;
        conn.execute(
            "UPDATE project_state
                SET default_surface_id = NULL,
                    default_surface_instance_id = NULL
              WHERE project_id = ?1",
            [PROJECT_ID],
        )?;
        conn.execute("DROP TABLE surfaces", [])?;

        let state = inspect_project_state_database(&fixture.project.state_db_path, PROJECT_ID);

        assert!(matches!(state, DatabaseInspection::Malformed { .. }));
        Ok(())
    }

    #[test]
    fn setup_relevant_surface_registration_values_are_returned_raw() -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-malformed-surface-row")?;
        Connection::open(&fixture.project.state_db_path)?.execute(
            "UPDATE surfaces
                SET metadata_json = '[]'
              WHERE project_id = ?1",
            [PROJECT_ID],
        )?;

        let state = inspect_project_state_database(&fixture.project.state_db_path, PROJECT_ID);
        let snapshot = present_project_state(&state);

        assert_eq!(snapshot.surfaces[0].metadata_json, "[]");
        Ok(())
    }

    #[test]
    fn corrupt_database_is_unreadable() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("inspect-corrupt-db")?;
        let path = project_state_db_path(runtime_home.path(), PROJECT_ID);
        fs::create_dir_all(path.parent().expect("state path has parent"))?;
        fs::write(&path, b"this is not sqlite")?;

        let state = inspect_project_state_database(&path, PROJECT_ID);

        assert!(matches!(state, DatabaseInspection::Unreadable { .. }));
        Ok(())
    }

    #[test]
    fn inspection_does_not_create_parent_directory_or_database() -> Result<(), Box<dyn Error>> {
        let root = TempRuntimeHome::new("inspect-no-create-root")?;
        let missing_state = root
            .path()
            .join("missing-parent")
            .join("project")
            .join("state.sqlite");

        let state = inspect_project_state_database(&missing_state, PROJECT_ID);

        assert!(matches!(state, DatabaseInspection::Missing { .. }));
        assert!(!missing_state.exists());
        assert!(!missing_state
            .parent()
            .expect("state path has parent")
            .exists());
        Ok(())
    }

    #[test]
    fn inspection_does_not_mutate_database_bytes_migrations_or_sidecars(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = historical_fixture("inspect-no-mutation", 6)?;
        let registry_hash_before = file_hash(&fixture.runtime_home.registry_db_path())?;
        let state_hash_before = file_hash(&fixture.project.state_db_path)?;
        let migration_count_before =
            migration_count(&fixture.project.state_db_path, PROJECT_STATE_DATABASE_KIND)?;
        let sidecars_before = existing_sidecars(&[
            fixture.runtime_home.registry_db_path(),
            fixture.project.state_db_path.clone(),
        ]);

        let _inspection = inspect_runtime_home(fixture.runtime_home.path());

        assert_eq!(
            file_hash(&fixture.runtime_home.registry_db_path())?,
            registry_hash_before
        );
        assert_eq!(
            file_hash(&fixture.project.state_db_path)?,
            state_hash_before
        );
        assert_eq!(
            migration_count(&fixture.project.state_db_path, PROJECT_STATE_DATABASE_KIND)?,
            migration_count_before
        );
        assert_eq!(
            existing_sidecars(&[
                fixture.runtime_home.registry_db_path(),
                fixture.project.state_db_path.clone(),
            ]),
            sidecars_before
        );
        assert!(
            sidecars_before.is_empty(),
            "fixture should be closed without SQLite sidecars"
        );
        Ok(())
    }

    #[test]
    fn read_only_database_connection_rejects_writes() -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-read-only-connection")?;
        let conn = open_read_only_database(&fixture.project.state_db_path)?;

        let error = conn
            .execute("CREATE TABLE inspection_write_probe (id INTEGER)", [])
            .expect_err("DDL must fail on the inspection connection");

        assert!(error.to_string().contains("readonly"));
        Ok(())
    }

    #[test]
    fn repeated_inspection_is_deterministic() -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-deterministic")?;

        let first = inspect_runtime_home(fixture.runtime_home.path());
        let second = inspect_runtime_home(fixture.runtime_home.path());

        assert_eq!(first, second);
        Ok(())
    }

    fn current_fixture(prefix: &str) -> Result<InspectionFixture, Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new(prefix)?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        initialize_runtime_home(runtime_home.path(), RUNTIME_HOME_ID, "{}")?;
        let project = register_project(
            runtime_home.path(),
            ProjectRegistration {
                project_id: PROJECT_ID.to_owned(),
                repo_root,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        register_surface(runtime_home.path(), surface_registration())?;
        Ok(InspectionFixture {
            runtime_home,
            project,
        })
    }

    fn historical_fixture(prefix: &str, version: i64) -> Result<InspectionFixture, Box<dyn Error>> {
        let fixture = current_fixture(prefix)?;
        fs::remove_file(&fixture.project.state_db_path)?;
        let mut conn = Connection::open(&fixture.project.state_db_path)?;
        create_project_state_fixture_version(&mut conn, PROJECT_ID, version)?;
        insert_historical_surface(&conn)?;
        drop(conn);
        Ok(fixture)
    }

    fn replace_project_state_db_path(
        runtime_home: &Path,
        project_id: &str,
        state_db_path: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let conn = Connection::open(registry_db_path(runtime_home))?;
        conn.execute(
            "UPDATE projects SET state_db_path = ?2 WHERE project_id = ?1",
            params![project_id, state_db_path.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    fn surface_registration() -> SurfaceRegistration {
        SurfaceRegistration {
            project_id: PROJECT_ID.to_owned(),
            surface_id: SURFACE_ID.to_owned(),
            surface_instance_id: SURFACE_INSTANCE_ID.to_owned(),
            surface_kind: "mcp".to_owned(),
            interaction_role: SurfaceInteractionRole::Agent,
            display_name: Some("Agent MCP".to_owned()),
            capability_profile_json: "{}".to_owned(),
            local_access_json: format!(
                "{{\"access_class\":\"core_mutation\",\"authorized_access_classes\":[\"read_status\",\"core_mutation\"],\"verification_basis\":\"{}\"}}",
                VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
            ),
            metadata_json: "{}".to_owned(),
        }
    }

    fn insert_historical_surface(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO surfaces (
                project_id,
                surface_id,
                surface_instance_id,
                surface_kind,
                display_name,
                capability_profile_json,
                local_access_json,
                registered_at,
                metadata_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                'mcp',
                'Agent MCP',
                '{}',
                '{\"access_class\":\"core_mutation\",\"authorized_access_classes\":[\"read_status\",\"core_mutation\"],\"verification_basis\":\"local_admin_registration\"}',
                't0',
                '{}'
            )",
            params![PROJECT_ID, SURFACE_ID, SURFACE_INSTANCE_ID],
        )?;
        Ok(())
    }

    fn present_registry(inspection: &RegistryDatabaseInspection) -> &RegistryInspectionSnapshot {
        match inspection {
            DatabaseInspection::Present(snapshot) => snapshot,
            other => panic!("expected present registry inspection, got {other:?}"),
        }
    }

    fn present_project_state(
        inspection: &ProjectStateDatabaseInspection,
    ) -> &ProjectStateInspectionSnapshot {
        match inspection {
            DatabaseInspection::Present(snapshot) => snapshot,
            other => panic!("expected present project-state inspection, got {other:?}"),
        }
    }

    fn migration_count(path: &Path, database_kind: &str) -> StoreResult<i64> {
        let conn = open_read_only_database(path)?;
        Ok(conn.query_row(
            "SELECT COUNT(*)
               FROM schema_migrations
              WHERE database_kind = ?1",
            [database_kind],
            |row| row.get(0),
        )?)
    }

    fn file_hash(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(Sha256::digest(fs::read(path)?).to_vec())
    }

    fn existing_sidecars(paths: &[PathBuf]) -> Vec<PathBuf> {
        let mut sidecars = Vec::new();
        for path in paths {
            for sidecar in sqlite_sidecar_paths(path) {
                if sidecar.exists() {
                    sidecars.push(sidecar);
                }
            }
        }
        sidecars.sort();
        sidecars
    }

    fn sqlite_sidecar_paths(path: &Path) -> Vec<PathBuf> {
        ["-wal", "-shm", "-journal"]
            .iter()
            .map(|suffix| {
                let mut raw = OsString::from(path.as_os_str());
                raw.push(suffix);
                PathBuf::from(raw)
            })
            .collect()
    }
}
