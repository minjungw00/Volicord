use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;
use volicord_types::{
    guard_manifest_from_json, project_agent_session_id, ConnectionIntegrationInstanceId,
    ConnectionVerificationReport, IntegrationRevision,
};

use crate::{
    agent_connections::{
        CONNECTION_INTENT_PERSONAL, CONNECTION_INTENT_SHARED, CONNECTION_MODE_READ_ONLY,
        CONNECTION_MODE_WORKFLOW, HOST_KIND_CODEX, HOST_SCOPE_PROJECT, HOST_SCOPE_USER,
    },
    bootstrap::{validate_project_record_for_execution, ProjectRecord},
    schema::{PROJECT_STATE_DATABASE_KIND, REGISTRY_DATABASE_KIND},
    sqlite::{
        open_read_only_database, registry_db_path, validate_persisted_manifest,
        validate_project_state_schema, validate_registry_schema,
    },
    StoreError,
};

/// Read-only inspection result for a selected `Volicord Runtime Home`.
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
    Missing { path: PathBuf },
    Present(T),
    Unsupported { path: PathBuf, detail: String },
    Malformed { path: PathBuf, detail: String },
    Unreadable { path: PathBuf, detail: String },
}

/// Supported schema state for an inspectable database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectionSchemaState {
    Current,
}

/// Current readable registry data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryInspectionSnapshot {
    pub path: PathBuf,
    pub schema: InspectionSchemaState,
    pub runtime_home: RuntimeHomeInspectionRecord,
    pub installation_profile: Option<InstallationProfileInspectionRecord>,
    pub projects: Vec<ProjectInspectionRecord>,
    pub agent_connections: Vec<AgentConnectionInspectionRecord>,
    pub connection_projects: Vec<ConnectionProjectInspectionRecord>,
    pub runtime_project_session_bindings: Vec<RuntimeProjectSessionBindingInspectionRecord>,
    pub guard_installations: Vec<GuardInstallationInspectionRecord>,
}

/// Runtime Home singleton row read from `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHomeInspectionRecord {
    pub runtime_home_id: String,
    pub runtime_home_path: PathBuf,
    pub registry_db_path: PathBuf,
    pub storage_profile: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Registered project row plus its project-state inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInspectionRecord {
    pub project_internal_id: String,
    pub project_id: String,
    pub project_name: String,
    pub project_alias: String,
    pub runtime_home_id: String,
    pub repo_root: PathBuf,
    pub project_home: PathBuf,
    pub state_db_path: PathBuf,
    pub status: String,
    pub metadata_json: String,
    pub project_state: ProjectStateDatabaseInspection,
}

/// Agent Connection row read from the current registry schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConnectionInspectionRecord {
    pub connection_internal_id: String,
    pub integration_instance_id: ConnectionIntegrationInstanceId,
    pub host_kind: String,
    pub intent: String,
    pub host_scope: String,
    pub project_internal_id: Option<String>,
    pub server_name: String,
    pub config_target: String,
    pub mode: String,
    pub enabled: bool,
    pub managed_fingerprint: String,
    pub integration_generation: i64,
    pub verification_report_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// Connection project membership row read from the current registry schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProjectInspectionRecord {
    pub connection_internal_id: String,
    pub project_internal_id: String,
    pub project_id: String,
    pub created_at: String,
}

/// Runtime/project Agent Session reservation read from the current registry schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProjectSessionBindingInspectionRecord {
    pub runtime_session_id: String,
    pub connection_internal_id: String,
    pub project_internal_id: String,
    pub project_id: String,
    pub session_id: String,
    pub project_integration_revision: String,
    pub host_session_id: String,
    pub bound_at: String,
}

/// Guard installation row read from the current registry schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardInstallationInspectionRecord {
    pub guard_installation_id: String,
    pub connection_internal_id: String,
    pub project_internal_id: String,
    pub project_id: String,
    pub manifest_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Installation profile row read from the current registry schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallationProfileInspectionRecord {
    pub installation_id: String,
    pub runtime_home_id: String,
    pub volicord_command: String,
    pub volicord_mcp_command: String,
    pub bin_dir: PathBuf,
    pub default_connection_mode: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Current project-state data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStateInspectionSnapshot {
    pub path: PathBuf,
    pub schema: InspectionSchemaState,
    pub project_state: ProjectStateInspectionRecord,
}

/// Project-state header row needed by setup planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStateInspectionRecord {
    pub project_id: String,
    pub storage_profile: String,
    pub state_version: i64,
    pub metadata_json: String,
}

#[derive(Debug)]
enum InspectionIssue {
    Malformed(String),
    Unsupported { detail: String },
    Unreadable(String),
}

#[derive(Debug)]
struct ProjectRegistryRow {
    project_internal_id: String,
    project_id: String,
    project_name: String,
    project_alias: String,
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

    if let Err(error) = validate_registry_schema(&conn) {
        return project_state_validation_issue(error).into_database_inspection(path);
    }

    let runtime_home_record = match read_runtime_home_record(&conn) {
        Ok(record) => record,
        Err(issue) => return issue.into_database_inspection(path),
    };

    let project_rows = match read_project_rows(&conn, &runtime_home_record.runtime_home_id) {
        Ok(rows) => rows,
        Err(issue) => return issue.into_database_inspection(path),
    };
    let installation_profile =
        match read_installation_profile_row(&conn, &runtime_home_record.runtime_home_id) {
            Ok(row) => row,
            Err(issue) => return issue.into_database_inspection(path),
        };

    let projects = project_rows
        .into_iter()
        .map(|row| {
            let project = ProjectRecord {
                project_internal_id: row.project_internal_id,
                project_id: row.project_id,
                project_name: row.project_name,
                project_alias: row.project_alias,
                runtime_home_id: row.runtime_home_id,
                repo_root: row.repo_root,
                project_home: row.project_home,
                state_db_path: row.state_db_path,
                status: row.status,
                metadata_json: row.metadata_json,
            };
            let project_state = inspect_registered_project_state(runtime_home, &project);
            ProjectInspectionRecord {
                project_internal_id: project.project_internal_id,
                project_id: project.project_id,
                project_name: project.project_name,
                project_alias: project.project_alias,
                runtime_home_id: project.runtime_home_id,
                repo_root: project.repo_root,
                project_home: project.project_home,
                state_db_path: project.state_db_path,
                status: project.status,
                metadata_json: project.metadata_json,
                project_state,
            }
        })
        .collect::<Vec<_>>();

    let agent_connections = match read_agent_connection_rows(&conn) {
        Ok(records) => records,
        Err(issue) => return issue.into_database_inspection(path),
    };
    let connection_projects =
        match read_connection_project_rows(&conn, &agent_connections, &projects) {
            Ok(records) => records,
            Err(issue) => return issue.into_database_inspection(path),
        };
    let runtime_project_session_bindings = match read_runtime_project_session_binding_rows(&conn) {
        Ok(records) => records,
        Err(issue) => return issue.into_database_inspection(path),
    };
    let guard_installations =
        match read_guard_installation_rows(&conn, &agent_connections, &projects) {
            Ok(records) => records,
            Err(issue) => return issue.into_database_inspection(path),
        };

    DatabaseInspection::Present(RegistryInspectionSnapshot {
        path: path.to_path_buf(),
        schema: InspectionSchemaState::Current,
        runtime_home: runtime_home_record,
        installation_profile,
        projects,
        agent_connections,
        connection_projects,
        runtime_project_session_bindings,
        guard_installations,
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

    if let Err(error) = validate_project_state_schema(&conn) {
        return project_state_validation_issue(error).into_database_inspection(path);
    }

    let project_state = match read_project_state_record(&conn, project_id) {
        Ok(record) => record,
        Err(issue) => return issue.into_database_inspection(path),
    };
    DatabaseInspection::Present(ProjectStateInspectionSnapshot {
        path: path.to_path_buf(),
        schema: InspectionSchemaState::Current,
        project_state,
    })
}

impl InspectionIssue {
    fn into_database_inspection<T>(self, path: &Path) -> DatabaseInspection<T> {
        match self {
            Self::Malformed(detail) => malformed(path, detail),
            Self::Unsupported { detail } => DatabaseInspection::Unsupported {
                path: path.to_path_buf(),
                detail,
            },
            Self::Unreadable(detail) => DatabaseInspection::Unreadable {
                path: path.to_path_buf(),
                detail,
            },
        }
    }
}

fn project_state_validation_issue(error: StoreError) -> InspectionIssue {
    match error {
        StoreError::UnsupportedStorageProfile {
            database_kind,
            actual_storage_profile,
            expected_storage_profile,
        } => InspectionIssue::Unsupported {
            detail: unsupported_storage_profile_detail(
                database_kind,
                &actual_storage_profile,
                expected_storage_profile,
            ),
        },
        StoreError::Io(error) => InspectionIssue::Unreadable(error.to_string()),
        StoreError::Sqlite(error) => sqlite_unreadable(error),
        error => InspectionIssue::Malformed(error.to_string()),
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

fn read_runtime_home_record(
    conn: &Connection,
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
                runtime_home_path,
                registry_db_path,
                storage_profile,
                metadata_json,
                created_at,
                updated_at
             FROM runtime_home
             WHERE singleton_id = 1",
            [],
            |row| {
                Ok(RuntimeHomeInspectionRecord {
                    runtime_home_id: row.get(0)?,
                    runtime_home_path: PathBuf::from(row.get::<_, String>(1)?),
                    registry_db_path: PathBuf::from(row.get::<_, String>(2)?),
                    storage_profile: row.get(3)?,
                    metadata_json: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
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
    require_nonempty(
        "runtime_home.runtime_home_path",
        &record.runtime_home_path.display().to_string(),
    )?;
    require_nonempty(
        "runtime_home.registry_db_path",
        &record.registry_db_path.display().to_string(),
    )?;
    validate_storage_profile(REGISTRY_DATABASE_KIND, &record.storage_profile)?;
    validate_json_object("runtime_home.metadata_json", &record.metadata_json)?;
    Ok(record)
}

fn read_installation_profile_row(
    conn: &Connection,
    runtime_home_id: &str,
) -> Result<Option<InstallationProfileInspectionRecord>, InspectionIssue> {
    let record = conn
        .query_row(
            "SELECT
                installation_id,
                runtime_home_id,
                volicord_command,
                volicord_mcp_command,
                bin_dir,
                default_connection_mode,
                metadata_json,
                created_at,
                updated_at
             FROM installation_profile
             ORDER BY installation_id
             LIMIT 1",
            [],
            |row| {
                Ok(InstallationProfileInspectionRecord {
                    installation_id: row.get(0)?,
                    runtime_home_id: row.get(1)?,
                    volicord_command: row.get(2)?,
                    volicord_mcp_command: row.get(3)?,
                    bin_dir: PathBuf::from(row.get::<_, String>(4)?),
                    default_connection_mode: row.get(5)?,
                    metadata_json: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(registration_decode_error)?;
    if let Some(record) = &record {
        require_nonempty(
            "installation_profile.installation_id",
            &record.installation_id,
        )?;
        if record.runtime_home_id != runtime_home_id {
            return Err(InspectionIssue::Malformed(format!(
                "installation_profile references runtime_home_id {}, expected {runtime_home_id}",
                record.runtime_home_id
            )));
        }
        require_nonempty(
            "installation_profile.volicord_command",
            &record.volicord_command,
        )?;
        require_nonempty(
            "installation_profile.volicord_mcp_command",
            &record.volicord_mcp_command,
        )?;
        require_nonempty(
            "installation_profile.bin_dir",
            &record.bin_dir.display().to_string(),
        )?;
        validate_connection_mode(&record.default_connection_mode)?;
        validate_json_object("installation_profile.metadata_json", &record.metadata_json)?;
        require_nonempty("installation_profile.created_at", &record.created_at)?;
        require_nonempty("installation_profile.updated_at", &record.updated_at)?;
    }
    Ok(record)
}

fn read_project_rows(
    conn: &Connection,
    runtime_home_id: &str,
) -> Result<Vec<ProjectRegistryRow>, InspectionIssue> {
    let mut stmt = conn
        .prepare(
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
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })
        .map_err(sqlite_unreadable)?;

    let mut projects = Vec::new();
    for row in rows {
        let (
            project_internal_id,
            project_name,
            project_alias,
            row_runtime_home_id,
            repo_root,
            project_home,
            state_db_path,
            status,
            metadata_json,
        ) = row.map_err(registration_decode_error)?;
        let project_id = project_internal_id.clone();
        require_nonempty("projects.project_internal_id", &project_internal_id)?;
        require_nonempty("projects.project_name", &project_name)?;
        require_nonempty("projects.project_alias", &project_alias)?;
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
            project_internal_id,
            project_id,
            project_name,
            project_alias,
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

fn read_agent_connection_rows(
    conn: &Connection,
) -> Result<Vec<AgentConnectionInspectionRecord>, InspectionIssue> {
    let mut stmt = conn
        .prepare(
            "SELECT
                connection_internal_id,
                integration_instance_id,
                host_kind,
                intent,
                host_scope,
                project_internal_id,
                server_name,
                config_target,
                mode,
                enabled,
                managed_fingerprint,
                integration_generation,
                verification_report_json,
                created_at,
                updated_at,
                metadata_json
             FROM agent_connections
             ORDER BY host_kind, intent, host_scope, server_name, connection_internal_id",
        )
        .map_err(sqlite_unreadable)?;
    let rows = stmt
        .query_map([], |row| {
            let connection_internal_id = row.get::<_, String>(0)?;
            let integration_instance_id = ConnectionIntegrationInstanceId::parse(
                row.get::<_, String>(1)?,
            )
            .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(AgentConnectionInspectionRecord {
                connection_internal_id,
                integration_instance_id,
                host_kind: row.get(2)?,
                intent: row.get(3)?,
                host_scope: row.get(4)?,
                project_internal_id: row.get(5)?,
                server_name: row.get(6)?,
                config_target: row.get(7)?,
                mode: row.get(8)?,
                enabled: row.get::<_, i64>(9)? == 1,
                managed_fingerprint: row.get(10)?,
                integration_generation: row.get(11)?,
                verification_report_json: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
                metadata_json: row.get(15)?,
            })
        })
        .map_err(sqlite_unreadable)?;

    let mut connections = Vec::new();
    for row in rows {
        let connection = row.map_err(registration_decode_error)?;
        validate_agent_connection_row(&connection)?;
        connections.push(connection);
    }
    Ok(connections)
}

fn read_connection_project_rows(
    conn: &Connection,
    connections: &[AgentConnectionInspectionRecord],
    projects: &[ProjectInspectionRecord],
) -> Result<Vec<ConnectionProjectInspectionRecord>, InspectionIssue> {
    let connection_ids = connections
        .iter()
        .map(|record| record.connection_internal_id.as_str())
        .collect::<BTreeSet<_>>();
    let project_ids = projects
        .iter()
        .map(|record| record.project_internal_id.as_str())
        .collect::<BTreeSet<_>>();

    let mut stmt = conn
        .prepare(
            "SELECT connection_internal_id, project_internal_id, created_at
               FROM connection_projects
              ORDER BY connection_internal_id, project_internal_id",
        )
        .map_err(sqlite_unreadable)?;
    let rows = stmt
        .query_map([], |row| {
            let connection_internal_id = row.get::<_, String>(0)?;
            let project_internal_id = row.get::<_, String>(1)?;
            Ok(ConnectionProjectInspectionRecord {
                connection_internal_id,
                project_id: project_internal_id.clone(),
                project_internal_id,
                created_at: row.get(2)?,
            })
        })
        .map_err(sqlite_unreadable)?;

    let mut memberships = Vec::new();
    for row in rows {
        let membership = row.map_err(registration_decode_error)?;
        validate_connection_project_row(&membership, &connection_ids, &project_ids)?;
        memberships.push(membership);
    }
    Ok(memberships)
}

fn read_guard_installation_rows(
    conn: &Connection,
    connections: &[AgentConnectionInspectionRecord],
    projects: &[ProjectInspectionRecord],
) -> Result<Vec<GuardInstallationInspectionRecord>, InspectionIssue> {
    let connection_ids = connections
        .iter()
        .map(|record| record.connection_internal_id.as_str())
        .collect::<BTreeSet<_>>();
    let project_ids = projects
        .iter()
        .map(|record| record.project_internal_id.as_str())
        .collect::<BTreeSet<_>>();

    let mut stmt = conn
        .prepare(
            "SELECT
                gi.guard_installation_id,
                gi.connection_internal_id,
                gi.project_internal_id,
                json_extract(gi.manifest_json, '$.project_id'),
                gi.manifest_json,
                gi.created_at,
                gi.updated_at
             FROM guard_installations AS gi
             ORDER BY gi.connection_internal_id, gi.guard_installation_id",
        )
        .map_err(sqlite_unreadable)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(GuardInstallationInspectionRecord {
                guard_installation_id: row.get(0)?,
                connection_internal_id: row.get(1)?,
                project_internal_id: row.get(2)?,
                project_id: row.get(3)?,
                manifest_json: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(sqlite_unreadable)?;

    let mut installations = Vec::new();
    for row in rows {
        let installation = row.map_err(registration_decode_error)?;
        validate_guard_installation_row(&installation, &connection_ids, &project_ids)?;
        installations.push(installation);
    }
    Ok(installations)
}

fn read_runtime_project_session_binding_rows(
    conn: &Connection,
) -> Result<Vec<RuntimeProjectSessionBindingInspectionRecord>, InspectionIssue> {
    let mut stmt = conn
        .prepare(
            "SELECT
                b.runtime_session_id,
                b.connection_internal_id,
                b.project_internal_id,
                b.project_internal_id,
                b.session_id,
                b.project_integration_revision,
                b.host_session_id,
                b.bound_at
             FROM mcp_runtime_project_session_bindings AS b
             ORDER BY b.connection_internal_id, b.project_internal_id, b.session_id",
        )
        .map_err(sqlite_unreadable)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(RuntimeProjectSessionBindingInspectionRecord {
                runtime_session_id: row.get(0)?,
                connection_internal_id: row.get(1)?,
                project_internal_id: row.get(2)?,
                project_id: row.get(3)?,
                session_id: row.get(4)?,
                project_integration_revision: row.get(5)?,
                host_session_id: row.get(6)?,
                bound_at: row.get(7)?,
            })
        })
        .map_err(sqlite_unreadable)?;
    let mut bindings = Vec::new();
    for row in rows {
        let binding = row.map_err(registration_decode_error)?;
        for (field, value) in [
            (
                "runtime binding runtime_session_id",
                &binding.runtime_session_id,
            ),
            (
                "runtime binding connection_internal_id",
                &binding.connection_internal_id,
            ),
            (
                "runtime binding project_internal_id",
                &binding.project_internal_id,
            ),
            ("runtime binding project_id", &binding.project_id),
            ("runtime binding session_id", &binding.session_id),
            ("runtime binding host_session_id", &binding.host_session_id),
            ("runtime binding bound_at", &binding.bound_at),
        ] {
            require_nonempty(field, value)?;
        }
        IntegrationRevision::parse(binding.project_integration_revision.clone()).map_err(|_| {
            InspectionIssue::Malformed(
                "runtime binding project_integration_revision is invalid".to_owned(),
            )
        })?;
        let expected = project_agent_session_id(
            &binding.connection_internal_id,
            &binding.project_integration_revision,
            &binding.host_session_id,
        )
        .map_err(|_| {
            InspectionIssue::Malformed(
                "runtime binding Agent Session identity is invalid".to_owned(),
            )
        })?;
        if expected != binding.session_id {
            return Err(InspectionIssue::Malformed(
                "runtime binding Agent Session identity is noncanonical".to_owned(),
            ));
        }
        bindings.push(binding);
    }
    Ok(bindings)
}

fn read_project_state_record(
    conn: &Connection,
    project_id: &str,
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
                state_version,
                metadata_json
             FROM project_state
             WHERE project_id = ?1",
            [project_id],
            |row| {
                Ok(ProjectStateInspectionRecord {
                    project_id: row.get(0)?,
                    storage_profile: row.get(1)?,
                    state_version: row.get(2)?,
                    metadata_json: row.get(3)?,
                })
            },
        )
        .map_err(registration_decode_error)?;

    require_nonempty("project_state.project_id", &record.project_id)?;
    validate_storage_profile(PROJECT_STATE_DATABASE_KIND, &record.storage_profile)?;
    if record.state_version < 0 {
        return Err(InspectionIssue::Malformed(
            "project_state.state_version is negative".to_owned(),
        ));
    }
    validate_json_object("project_state.metadata_json", &record.metadata_json)?;
    Ok(record)
}

fn validate_agent_connection_row(
    connection: &AgentConnectionInspectionRecord,
) -> Result<(), InspectionIssue> {
    require_nonempty(
        "agent_connections.connection_internal_id",
        &connection.connection_internal_id,
    )?;
    validate_host_kind_scope(&connection.host_kind, &connection.host_scope)?;
    validate_connection_intent(&connection.intent)?;
    if let Some(project_internal_id) = &connection.project_internal_id {
        require_nonempty("agent_connections.project_internal_id", project_internal_id)?;
    }
    require_nonempty("agent_connections.server_name", &connection.server_name)?;
    require_nonempty("agent_connections.config_target", &connection.config_target)?;
    validate_connection_mode(&connection.mode)?;
    require_nonempty(
        "agent_connections.managed_fingerprint",
        &connection.managed_fingerprint,
    )?;
    if connection.integration_generation < 0 {
        return Err(InspectionIssue::Malformed(format!(
            "agent_connections.integration_generation for {} must be nonnegative",
            connection.connection_internal_id,
        )));
    }
    if let Some(text) = connection.verification_report_json.as_deref() {
        let report = serde_json::from_str::<ConnectionVerificationReport>(text).map_err(|error| {
            InspectionIssue::Malformed(format!(
                "agent_connections.verification_report_json for {} violates the canonical report contract: {error}",
                connection.connection_internal_id,
            ))
        })?;
        if serde_json::to_string(&report).ok().as_deref() != Some(text) {
            return Err(InspectionIssue::Malformed(format!(
                "agent_connections.verification_report_json for {} is not canonical JSON",
                connection.connection_internal_id,
            )));
        }
    }
    require_nonempty("agent_connections.created_at", &connection.created_at)?;
    require_nonempty("agent_connections.updated_at", &connection.updated_at)?;
    validate_json_object("agent_connections.metadata_json", &connection.metadata_json)?;
    Ok(())
}

fn validate_connection_project_row(
    membership: &ConnectionProjectInspectionRecord,
    connection_ids: &BTreeSet<&str>,
    project_ids: &BTreeSet<&str>,
) -> Result<(), InspectionIssue> {
    require_nonempty(
        "connection_projects.connection_internal_id",
        &membership.connection_internal_id,
    )?;
    require_nonempty(
        "connection_projects.project_internal_id",
        &membership.project_internal_id,
    )?;
    require_nonempty("connection_projects.created_at", &membership.created_at)?;
    if !connection_ids.contains(membership.connection_internal_id.as_str()) {
        return Err(InspectionIssue::Malformed(format!(
            "connection_projects references missing connection_internal_id {}",
            membership.connection_internal_id
        )));
    }
    if !project_ids.contains(membership.project_internal_id.as_str()) {
        return Err(InspectionIssue::Malformed(format!(
            "connection_projects references missing project_internal_id {}",
            membership.project_internal_id
        )));
    }
    Ok(())
}

fn validate_guard_installation_row(
    installation: &GuardInstallationInspectionRecord,
    connection_ids: &BTreeSet<&str>,
    project_ids: &BTreeSet<&str>,
) -> Result<(), InspectionIssue> {
    require_nonempty(
        "guard_installations.guard_installation_id",
        &installation.guard_installation_id,
    )?;
    require_nonempty(
        "guard_installations.connection_internal_id",
        &installation.connection_internal_id,
    )?;
    if !connection_ids.contains(installation.connection_internal_id.as_str()) {
        return Err(InspectionIssue::Malformed(format!(
            "guard_installations references missing connection_internal_id {}",
            installation.connection_internal_id
        )));
    }
    require_nonempty(
        "guard_installations.project_internal_id",
        &installation.project_internal_id,
    )?;
    if !project_ids.contains(installation.project_internal_id.as_str()) {
        return Err(InspectionIssue::Malformed(format!(
            "guard_installations references missing project_internal_id {}",
            installation.project_internal_id
        )));
    }
    require_nonempty("guard_installations.project_id", &installation.project_id)?;
    guard_manifest_from_json(&installation.manifest_json).map_err(|_| {
        InspectionIssue::Malformed(
            "guard_installations.manifest_json is not a canonical current Guard manifest"
                .to_owned(),
        )
    })?;
    require_nonempty("guard_installations.created_at", &installation.created_at)?;
    require_nonempty("guard_installations.updated_at", &installation.updated_at)?;
    Ok(())
}

fn validate_host_kind_scope(host_kind: &str, host_scope: &str) -> Result<(), InspectionIssue> {
    let valid = matches!(
        (host_kind, host_scope),
        (HOST_KIND_CODEX, HOST_SCOPE_USER) | (HOST_KIND_CODEX, HOST_SCOPE_PROJECT)
    );
    if valid {
        Ok(())
    } else {
        Err(InspectionIssue::Malformed(format!(
            "agent_connections host_kind={host_kind} host_scope={host_scope} is not supported"
        )))
    }
}

fn validate_connection_intent(intent: &str) -> Result<(), InspectionIssue> {
    if matches!(
        intent,
        CONNECTION_INTENT_PERSONAL | CONNECTION_INTENT_SHARED
    ) {
        Ok(())
    } else {
        Err(InspectionIssue::Malformed(format!(
            "agent_connections.intent is not supported: {intent}"
        )))
    }
}

fn validate_connection_mode(mode: &str) -> Result<(), InspectionIssue> {
    if matches!(mode, CONNECTION_MODE_READ_ONLY | CONNECTION_MODE_WORKFLOW) {
        Ok(())
    } else {
        Err(InspectionIssue::Malformed(format!(
            "agent_connections.mode is not supported: {mode}"
        )))
    }
}

fn validate_storage_profile(
    database_kind: &'static str,
    storage_profile: &str,
) -> Result<(), InspectionIssue> {
    validate_persisted_manifest(database_kind, storage_profile)
        .map_err(project_state_validation_issue)
}

fn unsupported_storage_profile_detail(
    database_kind: &'static str,
    storage_profile: &str,
    expected_storage_profile: &str,
) -> String {
    format!(
        "{database_kind} storage_profile {storage_profile} is not supported; expected {expected_storage_profile}; explicitly recreate the Runtime Home"
    )
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

    use rusqlite::{params, Connection};
    use sha2::{Digest, Sha256};
    use volicord_test_support::TempRuntimeHome;
    use volicord_types::{canonical_json_string, StorageManifest};

    use super::*;
    use crate::{
        agent_connections::{HOST_KIND_CODEX, HOST_SCOPE_USER},
        bootstrap::{
            initialize_runtime_home, register_project, ProjectRecord, ProjectRegistration,
            ACTIVE_PROJECT_STATUS,
        },
        schema::current_storage_manifest,
        sqlite::{open_read_only_database, project_state_db_path, registry_db_path},
    };

    const PROJECT_ID: &str = "project_inspect";
    const RUNTIME_HOME_ID: &str = "runtime_home_inspect";
    const UNSUPPORTED_STORAGE_PROFILE: &str = "unknown_storage_contract";

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

        assert_eq!(snapshot.schema, InspectionSchemaState::Current);
        assert_eq!(snapshot.runtime_home.runtime_home_id, RUNTIME_HOME_ID);
        assert_eq!(snapshot.projects.len(), 1);
        assert_eq!(snapshot.projects[0].project_id, PROJECT_ID);
        assert_eq!(snapshot.projects[0].status, ACTIVE_PROJECT_STATUS);
        assert!(snapshot.agent_connections.is_empty());
        assert!(snapshot.connection_projects.is_empty());
        Ok(())
    }

    #[test]
    fn unsupported_profile_registry_requires_recreation_without_mutation(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-old-profile-registry")?;
        let registry_path = fixture.runtime_home.registry_db_path();
        mark_registry_old_profile(&registry_path)?;
        let registry_hash_before = file_hash(&registry_path)?;
        let sidecars_before = existing_sidecars(std::slice::from_ref(&registry_path));

        let inspection = inspect_runtime_home(fixture.runtime_home.path());

        match inspection.registry {
            DatabaseInspection::Unsupported { detail, .. } => {
                assert!(detail.contains(UNSUPPORTED_STORAGE_PROFILE));
                assert!(detail.contains("explicitly recreate"));
            }
            other => panic!("expected unsupported old-profile registry, got {other:?}"),
        }
        assert_eq!(file_hash(&registry_path)?, registry_hash_before);
        assert_eq!(existing_sidecars(&[registry_path]), sidecars_before);
        Ok(())
    }

    #[test]
    fn malformed_registry_profile_is_corrupt_without_mutation() -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-malformed-profile-registry")?;
        let registry_path = fixture.runtime_home.registry_db_path();
        mark_registry_profile(&registry_path, "not-json")?;
        let before_hash = file_hash(&registry_path)?;

        let inspection = inspect_runtime_home(fixture.runtime_home.path());

        assert!(matches!(
            inspection.registry,
            DatabaseInspection::Malformed { .. }
        ));
        assert_eq!(file_hash(&registry_path)?, before_hash);
        Ok(())
    }

    #[test]
    fn current_registry_agent_connection_rows_are_inspected() -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-agent-connection-rows")?;
        let registry = Connection::open(fixture.runtime_home.registry_db_path())?;
        registry.execute(
            "INSERT INTO agent_connections (
                connection_internal_id,
                integration_instance_id,
                host_kind,
                intent,
                host_scope,
                project_internal_id,
                server_name,
                config_target,
                mode,
                enabled,
                managed_fingerprint,
                verification_report_json,
                metadata_json,
                created_at,
                updated_at
            )
            VALUES (
                'agent_inspected',
                'connection_instance_00112233-4455-4abb-8cdd-eeff10203040',
                ?1,
                ?2,
                ?3,
                NULL,
                'volicord-inspected',
                '/tmp/volicord-inspected-config.toml',
                'workflow',
                1,
                'fingerprint-inspected',
                NULL,
                '{}',
                't0',
                't0'
            )",
            params![HOST_KIND_CODEX, CONNECTION_INTENT_PERSONAL, HOST_SCOPE_USER],
        )?;
        registry.execute(
            "INSERT INTO connection_projects (connection_internal_id, project_internal_id, created_at)
             VALUES ('agent_inspected', ?1, 't0')",
            [PROJECT_ID],
        )?;
        let inspection = inspect_runtime_home(fixture.runtime_home.path());
        let snapshot = present_registry(&inspection.registry);

        assert_eq!(snapshot.agent_connections.len(), 1);
        assert_eq!(
            snapshot.agent_connections[0].connection_internal_id,
            "agent_inspected"
        );
        assert_eq!(
            snapshot.agent_connections[0]
                .integration_instance_id
                .as_str(),
            "connection_instance_00112233-4455-4abb-8cdd-eeff10203040"
        );
        assert_eq!(snapshot.agent_connections[0].host_kind, HOST_KIND_CODEX);
        assert_eq!(snapshot.agent_connections[0].host_scope, HOST_SCOPE_USER);
        assert_eq!(snapshot.agent_connections[0].mode, CONNECTION_MODE_WORKFLOW);
        assert_eq!(snapshot.connection_projects.len(), 1);
        assert_eq!(snapshot.connection_projects[0].project_id, PROJECT_ID);
        Ok(())
    }

    #[test]
    fn current_project_state_schema_is_inspected() -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-current-state")?;

        let state = inspect_project_state_database(&fixture.project.state_db_path, PROJECT_ID);
        let snapshot = present_project_state(&state);

        assert_eq!(snapshot.schema, InspectionSchemaState::Current);
        assert_eq!(snapshot.project_state.project_id, PROJECT_ID);
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
    fn unsupported_profile_project_state_requires_recreation_without_mutation(
    ) -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-old-profile-state")?;
        mark_project_state_old_profile(&fixture.project.state_db_path)?;
        let before_hash = file_hash(&fixture.project.state_db_path)?;

        let state = inspect_project_state_database(&fixture.project.state_db_path, PROJECT_ID);

        match state {
            DatabaseInspection::Unsupported { detail, .. } => {
                assert!(detail.contains(UNSUPPORTED_STORAGE_PROFILE));
                assert!(detail.contains("explicitly recreate"));
            }
            other => panic!("expected unsupported old-profile project state, got {other:?}"),
        }
        assert_eq!(file_hash(&fixture.project.state_db_path)?, before_hash);
        Ok(())
    }

    #[test]
    fn malformed_current_project_profile_is_corrupt_without_mutation() -> Result<(), Box<dyn Error>>
    {
        let fixture = current_fixture("inspect-malformed-current-profile-state")?;
        let malformed = format!(
            r#"{{"contract_id":"{}"}}"#,
            volicord_types::STORAGE_CONTRACT_ID
        );
        mark_project_state_profile(&fixture.project.state_db_path, &malformed)?;
        let before_hash = file_hash(&fixture.project.state_db_path)?;

        let state = inspect_project_state_database(&fixture.project.state_db_path, PROJECT_ID);

        assert!(matches!(state, DatabaseInspection::Malformed { .. }));
        assert_eq!(file_hash(&fixture.project.state_db_path)?, before_hash);
        Ok(())
    }

    #[test]
    fn forbidden_schema_migrations_table_requires_recreation() -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-forbidden-schema-ledger")?;
        Connection::open(&fixture.project.state_db_path)?.execute(
            "CREATE TABLE schema_migrations (database_kind TEXT NOT NULL)",
            [],
        )?;

        let state = inspect_project_state_database(&fixture.project.state_db_path, PROJECT_ID);

        match state {
            DatabaseInspection::Malformed { detail, .. } => {
                assert!(detail.contains("schema_migrations"));
                assert!(detail.contains("recreate"));
            }
            other => panic!("expected forbidden schema ledger diagnostic, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn missing_required_project_state_table_is_malformed() -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-missing-project-state-table")?;
        let conn = Connection::open(&fixture.project.state_db_path)?;
        conn.execute("DROP TABLE evidence_observations", [])?;

        let state = inspect_project_state_database(&fixture.project.state_db_path, PROJECT_ID);

        assert!(matches!(state, DatabaseInspection::Malformed { .. }));
        Ok(())
    }

    #[test]
    fn missing_user_action_origin_columns_are_malformed() -> Result<(), Box<dyn Error>> {
        for column in ["source_method", "source_idempotency_key"] {
            let fixture = current_fixture(&format!("inspect-missing-user-action-{column}"))?;
            let conn = Connection::open(&fixture.project.state_db_path)?;
            conn.execute(
                &format!(
                    "ALTER TABLE user_action_requests RENAME COLUMN {column} TO removed_{column}"
                ),
                [],
            )?;
            drop(conn);

            let state = inspect_project_state_database(&fixture.project.state_db_path, PROJECT_ID);

            assert!(
                matches!(state, DatabaseInspection::Malformed { .. }),
                "missing user_action_requests.{column} must not inspect as current: {state:?}"
            );
        }
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
    fn inspection_does_not_mutate_database_bytes_or_sidecars() -> Result<(), Box<dyn Error>> {
        let fixture = current_fixture("inspect-no-mutation")?;
        mark_registry_old_profile(&fixture.runtime_home.registry_db_path())?;
        mark_project_state_old_profile(&fixture.project.state_db_path)?;
        let registry_hash_before = file_hash(&fixture.runtime_home.registry_db_path())?;
        let state_hash_before = file_hash(&fixture.project.state_db_path)?;
        let sidecars_before = existing_sidecars(&[
            fixture.runtime_home.registry_db_path(),
            fixture.project.state_db_path.clone(),
        ]);

        let _inspection = inspect_runtime_home(fixture.runtime_home.path());

        assert!(matches!(
            inspect_runtime_home(fixture.runtime_home.path()).registry,
            DatabaseInspection::Unsupported { .. }
        ));
        assert_eq!(
            file_hash(&fixture.runtime_home.registry_db_path())?,
            registry_hash_before
        );
        assert_eq!(
            file_hash(&fixture.project.state_db_path)?,
            state_hash_before
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
        Ok(InspectionFixture {
            runtime_home,
            project,
        })
    }

    fn replace_project_state_db_path(
        runtime_home: &Path,
        project_id: &str,
        state_db_path: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let conn = Connection::open(registry_db_path(runtime_home))?;
        conn.execute(
            "UPDATE projects SET state_db_path = ?2 WHERE project_internal_id = ?1",
            params![project_id, state_db_path.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    fn unsupported_storage_profile() -> Result<String, Box<dyn Error>> {
        let current = current_storage_manifest()?;
        let manifest = StorageManifest::new(
            UNSUPPORTED_STORAGE_PROFILE,
            current.canonical_ddl_digest.clone(),
            current.integrity_constraints_digest.clone(),
            current.enabled_capabilities.clone(),
        )?;
        Ok(canonical_json_string(&manifest)?)
    }

    fn mark_registry_old_profile(path: &Path) -> Result<(), Box<dyn Error>> {
        mark_registry_profile(path, &unsupported_storage_profile()?)
    }

    fn mark_registry_profile(path: &Path, profile: &str) -> Result<(), Box<dyn Error>> {
        let conn = Connection::open(path)?;
        conn.execute("UPDATE runtime_home SET storage_profile = ?1", [profile])?;
        Ok(())
    }

    fn mark_project_state_old_profile(path: &Path) -> Result<(), Box<dyn Error>> {
        mark_project_state_profile(path, &unsupported_storage_profile()?)
    }

    fn mark_project_state_profile(path: &Path, profile: &str) -> Result<(), Box<dyn Error>> {
        let conn = Connection::open(path)?;
        conn.execute("UPDATE project_state SET storage_profile = ?1", [profile])?;
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
