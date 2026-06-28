use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};
use volicord_types::BASELINE_PROJECT_ENFORCEMENT_PROFILE_JSON;

use crate::{
    migrations::{PROJECT_STATE_SCHEMA_VERSION, REGISTRY_SCHEMA_VERSION, STORAGE_PROFILE},
    runtime_home::{
        normalize_lexical_path, validate_project_home_boundary,
        validate_runtime_home_product_repository, RuntimePathBoundaryError,
    },
    sqlite::{
        open_project_state_database, open_registry_database, project_home_path, registry_db_path,
        with_immediate_transaction, PROJECT_STATE_DB_FILE,
    },
    StoreError, StoreResult,
};

/// Baseline-valid project registration status.
pub const ACTIVE_PROJECT_STATUS: &str = "active";

/// Runtime Home metadata stored in `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHomeRecord {
    pub runtime_home: PathBuf,
    pub registry_db_path: PathBuf,
    pub runtime_home_id: String,
    pub storage_profile: String,
    pub schema_version: i64,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Installation profile registration input stored in the Runtime Home registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallationProfileRegistration {
    pub installation_id: String,
    pub volicord_command: String,
    pub volicord_mcp_command: String,
    pub bin_dir: PathBuf,
    pub default_connection_mode: String,
    pub metadata_json: String,
}

/// Installation profile record stored in `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallationProfileRecord {
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

/// Local project registration input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRegistration {
    pub project_id: String,
    pub repo_root: PathBuf,
    pub project_home: Option<PathBuf>,
    pub status: String,
    pub metadata_json: String,
}

/// Repository-root project ensure input that does not require caller-supplied IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoProjectRegistration {
    pub project_name: Option<String>,
    pub project_alias: Option<String>,
    pub repo_root: PathBuf,
    pub project_home: Option<PathBuf>,
    pub status: String,
    pub metadata_json: String,
}

/// Project registration record stored in `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRecord {
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
}

/// Creates or validates a Runtime Home registry and singleton metadata row.
pub fn initialize_runtime_home(
    runtime_home: impl AsRef<Path>,
    runtime_home_id: &str,
    metadata_json: &str,
) -> StoreResult<RuntimeHomeRecord> {
    validate_identifier("runtime_home_id", runtime_home_id)?;
    validate_json_object("runtime_home.metadata_json", metadata_json)?;

    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let runtime_home_text = path_to_text("runtime_home.runtime_home_path", &runtime_home)?;
    let registry_path_text = path_to_text("runtime_home.registry_db_path", &registry_path)?;
    let mut conn = open_registry_database(&registry_path)?;

    with_immediate_transaction(&mut conn, |tx| {
        tx.execute(
            "INSERT OR IGNORE INTO runtime_home (
                singleton_id,
                runtime_home_id,
                runtime_home_path,
                registry_db_path,
                storage_profile,
                schema_version,
                metadata_json,
                created_at,
                updated_at
            )
            VALUES (
                1,
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )",
            params![
                runtime_home_id,
                runtime_home_text,
                registry_path_text,
                STORAGE_PROFILE,
                REGISTRY_SCHEMA_VERSION,
                metadata_json
            ],
        )?;
        Ok(())
    })?;

    runtime_home_record_from_conn(&conn, runtime_home, registry_path)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "runtime_home",
            id: runtime_home_id.to_owned(),
        }
    })
}

/// Reads Runtime Home metadata when the registry database already exists.
pub fn runtime_home_record(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<Option<RuntimeHomeRecord>> {
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database(&registry_path)?;
    runtime_home_record_from_conn(&conn, runtime_home, registry_path)
}

/// Creates or updates the installation profile for the selected Runtime Home.
pub fn write_installation_profile(
    runtime_home: impl AsRef<Path>,
    registration: InstallationProfileRegistration,
) -> StoreResult<InstallationProfileRecord> {
    validate_identifier("installation_id", &registration.installation_id)?;
    validate_command_text("volicord_command", &registration.volicord_command)?;
    validate_command_text("volicord_mcp_command", &registration.volicord_mcp_command)?;
    validate_connection_mode(&registration.default_connection_mode)?;
    validate_json_object(
        "installation_profile.metadata_json",
        &registration.metadata_json,
    )?;

    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let runtime_home_row =
        runtime_home_record_from_conn(&conn, runtime_home.clone(), registry_path.clone())?
            .ok_or_else(|| StoreError::NotFound {
                entity: "runtime_home",
                id: registry_path.display().to_string(),
            })?;
    let bin_dir_text = path_to_text("installation_profile.bin_dir", &registration.bin_dir)?;

    with_immediate_transaction(&mut conn, |tx| {
        tx.execute(
            "INSERT INTO installation_profile (
                installation_id,
                runtime_home_id,
                volicord_command,
                volicord_mcp_command,
                bin_dir,
                default_connection_mode,
                metadata_json,
                created_at,
                updated_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6,
                ?7,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )
            ON CONFLICT(installation_id) DO UPDATE SET
                runtime_home_id = excluded.runtime_home_id,
                volicord_command = excluded.volicord_command,
                volicord_mcp_command = excluded.volicord_mcp_command,
                bin_dir = excluded.bin_dir,
                default_connection_mode = excluded.default_connection_mode,
                metadata_json = excluded.metadata_json,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![
                registration.installation_id,
                runtime_home_row.runtime_home_id,
                registration.volicord_command,
                registration.volicord_mcp_command,
                bin_dir_text,
                registration.default_connection_mode,
                registration.metadata_json,
            ],
        )?;
        Ok(())
    })?;

    installation_profile_from_conn(&conn)
}

/// Reads the installation profile when one has been written.
pub fn installation_profile(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<Option<InstallationProfileRecord>> {
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database(registry_path)?;
    installation_profile_from_conn_optional(&conn)
}

/// Reads the installation profile and returns a storage error when setup is incomplete.
pub fn require_installation_profile(
    runtime_home: impl AsRef<Path>,
) -> StoreResult<InstallationProfileRecord> {
    installation_profile(runtime_home)?.ok_or_else(|| StoreError::NotFound {
        entity: "installation_profile",
        id: "singleton".to_owned(),
    })
}

/// Registers a Product Repository project and creates its project `state.sqlite`.
pub fn register_project(
    runtime_home: impl AsRef<Path>,
    registration: ProjectRegistration,
) -> StoreResult<ProjectRecord> {
    validate_project_id(&registration.project_id)?;
    write_project_registration(
        runtime_home,
        ProjectWriteRegistration {
            project_internal_id: registration.project_id.clone(),
            project_name: registration.project_id.clone(),
            project_alias: registration.project_id.clone(),
            repo_root: registration.repo_root,
            project_home: registration.project_home,
            status: registration.status,
            metadata_json: registration.metadata_json,
        },
    )
}

/// Ensures a project from its repository root and derives the internal ID.
pub fn ensure_project_for_repo(
    runtime_home: impl AsRef<Path>,
    registration: RepoProjectRegistration,
) -> StoreResult<ProjectRecord> {
    validate_project_status(&registration.status)?;
    validate_json_object("projects.metadata_json", &registration.metadata_json)?;

    let path_validation =
        validate_runtime_home_product_repository(runtime_home.as_ref(), &registration.repo_root)
            .map_err(path_boundary_input)?;
    if let Some(existing) =
        project_record_by_repo_root(&path_validation.runtime_home, &path_validation.repo_root)?
    {
        return Ok(existing);
    }

    let project_internal_id = project_internal_id_for_repo(&path_validation.repo_root)?;
    let project_name = registration
        .project_name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| default_project_name(&path_validation.repo_root));
    let project_alias = registration
        .project_alias
        .filter(|alias| !alias.trim().is_empty())
        .unwrap_or_else(|| default_project_alias(&project_name, &project_internal_id));
    write_project_registration_from_validated_paths(
        path_validation.runtime_home,
        path_validation.repo_root,
        ProjectWriteRegistration {
            project_internal_id,
            project_name,
            project_alias,
            repo_root: PathBuf::new(),
            project_home: registration.project_home,
            status: registration.status,
            metadata_json: registration.metadata_json,
        },
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectWriteRegistration {
    project_internal_id: String,
    project_name: String,
    project_alias: String,
    repo_root: PathBuf,
    project_home: Option<PathBuf>,
    status: String,
    metadata_json: String,
}

fn write_project_registration(
    runtime_home: impl AsRef<Path>,
    registration: ProjectWriteRegistration,
) -> StoreResult<ProjectRecord> {
    validate_project_id(&registration.project_internal_id)?;
    validate_project_name(&registration.project_name)?;
    validate_project_alias(&registration.project_alias)?;
    validate_project_status(&registration.status)?;
    validate_json_object("projects.metadata_json", &registration.metadata_json)?;

    let path_validation =
        validate_runtime_home_product_repository(runtime_home.as_ref(), &registration.repo_root)
            .map_err(path_boundary_input)?;
    write_project_registration_from_validated_paths(
        path_validation.runtime_home,
        path_validation.repo_root,
        registration,
    )
}

fn write_project_registration_from_validated_paths(
    runtime_home: PathBuf,
    repo_root: PathBuf,
    registration: ProjectWriteRegistration,
) -> StoreResult<ProjectRecord> {
    validate_project_id(&registration.project_internal_id)?;
    validate_project_name(&registration.project_name)?;
    validate_project_alias(&registration.project_alias)?;
    validate_project_status(&registration.status)?;
    validate_json_object("projects.metadata_json", &registration.metadata_json)?;

    let registry_path = registry_db_path(&runtime_home);
    let mut registry = open_registry_database(&registry_path)?;
    let runtime_home_row =
        runtime_home_record_from_conn(&registry, runtime_home.clone(), registry_path.clone())?
            .ok_or_else(|| StoreError::NotFound {
                entity: "runtime_home",
                id: registry_path.display().to_string(),
            })?;

    let project_home = registration
        .project_home
        .unwrap_or_else(|| project_home_path(&runtime_home, &registration.project_internal_id));
    let project_home = validate_project_home_boundary(&runtime_home, &repo_root, &project_home)
        .map_err(path_boundary_input)?;
    let state_db_path = project_home.join(PROJECT_STATE_DB_FILE);
    let repo_root_text = path_to_text("repo_root", &repo_root)?;
    let project_home_text = path_to_text("project_home", &project_home)?;
    let state_db_path_text = path_to_text("state_db_path", &state_db_path)?;

    let mut project_state = open_project_state_database(&state_db_path)?;
    with_immediate_transaction(&mut project_state, |tx| {
        tx.execute(
            "INSERT INTO project_state (
                project_id,
                storage_profile,
                schema_version,
                created_at,
                updated_at,
                metadata_json,
                enforcement_profile_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?4,
                ?5
            )
            ON CONFLICT(project_id) DO UPDATE SET
                storage_profile = excluded.storage_profile,
                schema_version = excluded.schema_version,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                metadata_json = excluded.metadata_json",
            params![
                registration.project_internal_id,
                STORAGE_PROFILE,
                PROJECT_STATE_SCHEMA_VERSION,
                registration.metadata_json,
                BASELINE_PROJECT_ENFORCEMENT_PROFILE_JSON
            ],
        )?;
        Ok(())
    })?;

    with_immediate_transaction(&mut registry, |tx| {
        tx.execute(
            "INSERT INTO projects (
                project_internal_id,
                project_name,
                project_alias,
                runtime_home_id,
                repo_root,
                project_home,
                state_db_path,
                status,
                metadata_json,
                created_at,
                updated_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6,
                ?7,
                ?8,
                ?9,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )
            ON CONFLICT(project_internal_id) DO UPDATE SET
                project_name = excluded.project_name,
                project_alias = excluded.project_alias,
                runtime_home_id = excluded.runtime_home_id,
                repo_root = excluded.repo_root,
                project_home = excluded.project_home,
                state_db_path = excluded.state_db_path,
                status = excluded.status,
                metadata_json = excluded.metadata_json,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![
                registration.project_internal_id,
                registration.project_name,
                registration.project_alias,
                runtime_home_row.runtime_home_id,
                repo_root_text,
                project_home_text,
                state_db_path_text,
                registration.status,
                registration.metadata_json
            ],
        )?;
        tx.execute(
            "INSERT INTO project_aliases (
                alias,
                project_internal_id,
                created_at
            )
            VALUES (
                ?1,
                ?2,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )
            ON CONFLICT(alias) DO UPDATE SET
                project_internal_id = excluded.project_internal_id",
            params![registration.project_alias, registration.project_internal_id],
        )?;
        Ok(())
    })?;

    project_record_from_conn(&registry, &runtime_home, &registration.project_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project",
            id: registration.project_internal_id,
        })
}

fn path_boundary_input(error: crate::runtime_home::RuntimePathBoundaryError) -> StoreError {
    StoreError::InvalidInput {
        detail: error.to_string(),
    }
}

/// Lists registered projects in deterministic order.
pub fn list_projects(runtime_home: impl AsRef<Path>) -> StoreResult<Vec<ProjectRecord>> {
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Ok(Vec::new());
    }

    let conn = open_registry_database(registry_path)?;
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
    let mut projects = Vec::new();
    for row in rows {
        let project = row?;
        projects.push(validate_current_project_registration(
            &runtime_home,
            &project,
        )?);
    }
    Ok(projects)
}

/// Reads one registered project from `registry.sqlite`.
pub fn project_record(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    validate_project_reference(project_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database(registry_path)?;
    project_record_from_conn(&conn, &runtime_home, project_id)
}

/// Reads one registered project by internal id.
pub fn project_record_by_internal_id(
    runtime_home: impl AsRef<Path>,
    project_internal_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    project_record(runtime_home, project_internal_id)
}

/// Reads one registered project by repository root.
pub fn project_record_by_repo_root(
    runtime_home: impl AsRef<Path>,
    repo_root: impl AsRef<Path>,
) -> StoreResult<Option<ProjectRecord>> {
    let path_validation = validate_runtime_home_product_repository(runtime_home, repo_root)
        .map_err(path_boundary_input)?;
    let registry_path = registry_db_path(&path_validation.runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }
    let repo_root_text = path_to_text("repo_root", &path_validation.repo_root)?;
    let conn = open_registry_database(registry_path)?;
    let project = conn
        .query_row(
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
             WHERE repo_root = ?1",
            [repo_root_text],
            project_record_from_row,
        )
        .optional()
        .map_err(StoreError::from)?;
    project
        .map(|project| {
            validate_current_project_registration(path_validation.runtime_home, &project)
        })
        .transpose()
}

/// Updates a project's display name and, optionally, its primary alias.
pub fn rename_project(
    runtime_home: impl AsRef<Path>,
    project_ref: &str,
    project_name: &str,
    project_alias: Option<&str>,
) -> StoreResult<ProjectRecord> {
    validate_project_reference(project_ref)?;
    validate_project_name(project_name)?;
    if let Some(alias) = project_alias {
        validate_project_alias(alias)?;
    }

    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    let mut conn = open_registry_database(&registry_path)?;
    let current =
        raw_project_record_from_conn(&conn, project_ref)?.ok_or_else(|| StoreError::NotFound {
            entity: "project",
            id: project_ref.to_owned(),
        })?;
    let next_alias = project_alias.unwrap_or(&current.project_alias);
    let tx = crate::sqlite::begin_immediate_transaction(&mut conn)?;
    tx.execute(
        "UPDATE projects
            SET project_name = ?2,
                project_alias = ?3,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
          WHERE project_internal_id = ?1",
        params![current.project_internal_id, project_name, next_alias],
    )?;
    tx.execute(
        "INSERT INTO project_aliases (
            alias,
            project_internal_id,
            created_at
        )
        VALUES (
            ?1,
            ?2,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        )
        ON CONFLICT(alias) DO UPDATE SET
            project_internal_id = excluded.project_internal_id",
        params![next_alias, current.project_internal_id],
    )?;
    tx.commit()?;

    project_record_from_conn(&conn, &runtime_home, &current.project_internal_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "project",
            id: current.project_internal_id,
        }
    })
}

/// Removes a project registry row without deleting project-state files.
pub fn forget_project(runtime_home: impl AsRef<Path>, project_ref: &str) -> StoreResult<bool> {
    validate_project_reference(project_ref)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(false);
    }
    let mut conn = open_registry_database(&registry_path)?;
    let Some(current) = raw_project_record_from_conn(&conn, project_ref)? else {
        return Ok(false);
    };
    let tx = crate::sqlite::begin_immediate_transaction(&mut conn)?;
    tx.execute(
        "DELETE FROM project_aliases WHERE project_internal_id = ?1",
        [current.project_internal_id.as_str()],
    )?;
    let changed = tx.execute(
        "DELETE FROM projects WHERE project_internal_id = ?1",
        [current.project_internal_id.as_str()],
    )?;
    tx.commit()?;
    Ok(changed > 0)
}

/// Reads one registered project and validates it before execution use.
pub fn project_record_for_execution(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    project_record(runtime_home, project_id)
}

/// Validates a stored project registration for current operational use.
pub fn validate_current_project_registration(
    runtime_home: impl AsRef<Path>,
    project: &ProjectRecord,
) -> StoreResult<ProjectRecord> {
    validate_project_id(&project.project_id).map_err(|error| {
        StoreError::InvalidProjectRegistration {
            project_id: project.project_id.clone(),
            field: "project_id",
            relationship: "invalid_project_id",
            detail: error.to_string(),
        }
    })?;
    let path_validation =
        validate_runtime_home_product_repository(runtime_home.as_ref(), &project.repo_root)
            .map_err(|error| registered_project_path_error(project, "repo_root", error))?;
    let project_home = validate_project_home_boundary(
        &path_validation.runtime_home,
        &path_validation.repo_root,
        &project.project_home,
    )
    .map_err(|error| registered_project_path_error(project, "project_home", error))?;
    let expected_state_db_path = project_home.join(PROJECT_STATE_DB_FILE);
    let stored_state_db_path = normalize_lexical_path("state_db_path", &project.state_db_path)
        .map_err(|error| registered_project_path_error(project, "state_db_path", error))?;
    if stored_state_db_path != expected_state_db_path {
        return Err(state_db_path_mismatch_error(
            project,
            &stored_state_db_path,
            &expected_state_db_path,
        ));
    }

    Ok(ProjectRecord {
        repo_root: path_validation.repo_root,
        project_home,
        state_db_path: expected_state_db_path,
        ..project.clone()
    })
}

/// Validates a stored project registration before execution use.
pub fn validate_project_record_for_execution(
    runtime_home: impl AsRef<Path>,
    project: &ProjectRecord,
) -> StoreResult<ProjectRecord> {
    validate_current_project_registration(runtime_home, project)
}

fn registered_project_path_error(
    project: &ProjectRecord,
    field: &'static str,
    error: RuntimePathBoundaryError,
) -> StoreError {
    let relationship = error
        .violation()
        .map(|violation| violation.as_str())
        .unwrap_or("invalid_path");
    StoreError::InvalidProjectRegistration {
        project_id: project.project_id.clone(),
        field,
        relationship,
        detail: error.to_string(),
    }
}

fn state_db_path_mismatch_error(
    project: &ProjectRecord,
    stored: &Path,
    expected: &Path,
) -> StoreError {
    StoreError::InvalidProjectRegistration {
        project_id: project.project_id.clone(),
        field: "state_db_path",
        relationship: "state_db_path_mismatch",
        detail: format!(
            "state_db_path must match project_home/{PROJECT_STATE_DB_FILE}: stored {}, expected {}",
            stored.display(),
            expected.display()
        ),
    }
}

fn runtime_home_record_from_conn(
    conn: &Connection,
    runtime_home: PathBuf,
    registry_path: PathBuf,
) -> StoreResult<Option<RuntimeHomeRecord>> {
    conn.query_row(
        "SELECT
            runtime_home_id,
            storage_profile,
            schema_version,
            metadata_json,
            created_at,
            updated_at
           FROM runtime_home
          WHERE singleton_id = 1",
        [],
        |row| {
            Ok(RuntimeHomeRecord {
                runtime_home: runtime_home.clone(),
                registry_db_path: registry_path.clone(),
                runtime_home_id: row.get(0)?,
                storage_profile: row.get(1)?,
                schema_version: row.get(2)?,
                metadata_json: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

fn project_record_from_conn(
    conn: &Connection,
    runtime_home: &Path,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    let project = raw_project_record_from_conn(conn, project_id)?;
    project
        .map(|project| validate_current_project_registration(runtime_home, &project))
        .transpose()
}

pub(crate) fn raw_project_record_from_conn(
    conn: &Connection,
    project_ref: &str,
) -> StoreResult<Option<ProjectRecord>> {
    conn.query_row(
        "SELECT
            p.project_internal_id,
            p.project_name,
            p.project_alias,
            p.runtime_home_id,
            p.repo_root,
            p.project_home,
            p.state_db_path,
            p.status,
            p.metadata_json
         FROM projects AS p
         LEFT JOIN project_aliases AS pa
           ON pa.project_internal_id = p.project_internal_id
          AND pa.alias = ?1
         WHERE p.project_internal_id = ?1
            OR p.project_alias = ?1
            OR pa.alias = ?1
         ORDER BY p.project_internal_id
         LIMIT 1",
        [project_ref],
        project_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn project_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRecord> {
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

/// Validates a project id that may become one `projects/{project_id}` path component.
pub fn validate_project_id(project_id: &str) -> StoreResult<()> {
    validate_identifier("project_id", project_id)?;
    validate_path_component("project_id", project_id)
}

fn validate_project_reference(project_ref: &str) -> StoreResult<()> {
    validate_identifier("project_ref", project_ref)?;
    validate_path_component("project_ref", project_ref)
}

fn validate_project_name(name: &str) -> StoreResult<()> {
    validate_identifier("project_name", name)?;
    if name.contains('\0') {
        Err(StoreError::InvalidInput {
            detail: "project_name must not contain NUL".to_owned(),
        })
    } else {
        Ok(())
    }
}

fn validate_project_alias(alias: &str) -> StoreResult<()> {
    validate_identifier("project_alias", alias)?;
    validate_path_component("project_alias", alias)
}

fn validate_identifier(field: &'static str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}

fn validate_command_text(field: &'static str, value: &str) -> StoreResult<()> {
    validate_identifier(field, value)?;
    if value.contains('\0') {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not contain NUL"),
        })
    } else {
        Ok(())
    }
}

fn validate_path_component(field: &'static str, value: &str) -> StoreResult<()> {
    if value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.contains('\0')
    {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be a single path component"),
        })
    } else {
        Ok(())
    }
}

fn validate_project_status(status: &str) -> StoreResult<()> {
    if status == ACTIVE_PROJECT_STATUS {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("project status must be {ACTIVE_PROJECT_STATUS}"),
        })
    }
}

fn validate_connection_mode(mode: &str) -> StoreResult<()> {
    if matches!(mode, "read_only" | "workflow") {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "default_connection_mode must be read_only or workflow".to_owned(),
        })
    }
}

fn validate_json_object(field: &'static str, text: &str) -> StoreResult<()> {
    let value = serde_json::from_str::<Value>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be JSON object text: {error}"),
    })?;

    if value.is_object() {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be a JSON object"),
        })
    }
}

fn path_to_text(field: &'static str, path: &Path) -> StoreResult<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| StoreError::InvalidInput {
            detail: format!("{field} must be valid UTF-8"),
        })
}

fn installation_profile_from_conn(conn: &Connection) -> StoreResult<InstallationProfileRecord> {
    installation_profile_from_conn_optional(conn)?.ok_or_else(|| StoreError::NotFound {
        entity: "installation_profile",
        id: "singleton".to_owned(),
    })
}

fn installation_profile_from_conn_optional(
    conn: &Connection,
) -> StoreResult<Option<InstallationProfileRecord>> {
    conn.query_row(
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
            Ok(InstallationProfileRecord {
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
    .map_err(StoreError::from)
}

fn project_internal_id_for_repo(repo_root: &Path) -> StoreResult<String> {
    let repo_root_text = path_to_text("repo_root", repo_root)?;
    Ok(stable_internal_id("prj", &repo_root_text))
}

fn default_project_name(repo_root: &Path) -> String {
    repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("project")
        .to_owned()
}

fn default_project_alias(project_name: &str, project_internal_id: &str) -> String {
    let mut alias = String::new();
    let mut previous_separator = false;
    for character in project_name.chars() {
        if character.is_ascii_alphanumeric() {
            alias.push(character.to_ascii_lowercase());
            previous_separator = false;
        } else if matches!(character, '-' | '_') {
            if !alias.is_empty() && !previous_separator {
                alias.push(character);
                previous_separator = true;
            }
        } else if !alias.is_empty() && !previous_separator {
            alias.push('-');
            previous_separator = true;
        }
    }
    while alias.ends_with('-') || alias.ends_with('_') {
        alias.pop();
    }
    if alias.is_empty() {
        alias.push_str("project");
    }

    let suffix = project_internal_id
        .strip_prefix("prj_")
        .unwrap_or(project_internal_id)
        .chars()
        .take(8)
        .collect::<String>();
    format!("{alias}-{suffix}")
}

fn stable_internal_id(prefix: &str, input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let mut suffix = String::with_capacity(24);
    for byte in digest.iter().take(12) {
        suffix.push_str(&format!("{byte:02x}"));
    }
    format!("{prefix}_{suffix}")
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fs,
        path::{Path, PathBuf},
    };

    use crate::{
        core_pipeline::CoreProjectStore,
        inspection::{inspect_registry_database, DatabaseInspection},
        migrations::PROJECT_STATE_DATABASE_KIND,
        sqlite::{open_project_state_database, open_read_only_database},
    };
    use volicord_test_support::TempRuntimeHome;
    use volicord_types::ProjectId;

    use super::*;

    #[test]
    fn project_id_validator_rejects_unsafe_path_components() {
        for invalid in ["", "   ", ".", "..", "a/b", "a\\b", "a\0b"] {
            let error = validate_project_id(invalid).expect_err("project_id should be rejected");
            assert!(
                matches!(error, StoreError::InvalidInput { .. }),
                "unexpected error for {invalid:?}: {error}"
            );
        }
    }

    #[test]
    fn project_id_validator_accepts_normal_ascii_and_utf8() -> Result<(), Box<dyn Error>> {
        validate_project_id("project_alpha")?;
        validate_project_id("프로젝트")?;
        Ok(())
    }

    #[test]
    fn project_registration_uses_project_id_validator_even_with_custom_home(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-project-id-validation")?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        initialize_runtime_home(runtime_home.path(), "runtime_home_validation", "{}")?;

        let error = register_project(
            runtime_home.path(),
            ProjectRegistration {
                project_id: "a/b".to_owned(),
                repo_root,
                project_home: Some(runtime_home.path().join("custom-project-home")),
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("invalid project_id should be rejected before registration");

        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert!(!runtime_home.path().join("custom-project-home").exists());
        Ok(())
    }

    #[test]
    fn project_registration_rejects_same_runtime_home_and_repository() -> Result<(), Box<dyn Error>>
    {
        let runtime_home = TempRuntimeHome::new("store-same-runtime-repo")?;
        initialize_runtime_home(runtime_home.path(), "runtime_home_same_repo", "{}")?;

        let error = register_project(
            runtime_home.path(),
            ProjectRegistration {
                project_id: "project_same".to_owned(),
                repo_root: runtime_home.path().to_path_buf(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("same Runtime Home and Product Repository should be rejected");

        assert!(error.to_string().contains("same path"));
        assert!(!runtime_home.project_state_db_path("project_same").exists());
        Ok(())
    }

    #[test]
    fn installation_profile_accepts_executable_paths() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-installation-profile")?;
        initialize_runtime_home(runtime_home.path(), "runtime_home_installation", "{}")?;

        let profile = write_installation_profile(
            runtime_home.path(),
            InstallationProfileRegistration {
                installation_id: "default".to_owned(),
                volicord_command: "/opt/volicord/bin/volicord".to_owned(),
                volicord_mcp_command: "/opt/volicord/bin/volicord-mcp".to_owned(),
                bin_dir: PathBuf::from("/opt/volicord/bin"),
                default_connection_mode: "workflow".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;

        assert_eq!(profile.volicord_command, "/opt/volicord/bin/volicord");
        assert_eq!(profile.default_connection_mode, "workflow");
        assert_eq!(
            require_installation_profile(runtime_home.path())?.volicord_mcp_command,
            "/opt/volicord/bin/volicord-mcp"
        );
        Ok(())
    }

    #[test]
    fn installation_profile_rejects_impossible_command_text() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-installation-profile-invalid")?;
        initialize_runtime_home(
            runtime_home.path(),
            "runtime_home_installation_invalid",
            "{}",
        )?;

        let error = write_installation_profile(
            runtime_home.path(),
            InstallationProfileRegistration {
                installation_id: "default".to_owned(),
                volicord_command: "volicord\0bad".to_owned(),
                volicord_mcp_command: "volicord-mcp".to_owned(),
                bin_dir: PathBuf::from("/opt/volicord/bin"),
                default_connection_mode: "workflow".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("NUL command should be rejected");

        assert!(error
            .to_string()
            .contains("volicord_command must not contain NUL"));
        Ok(())
    }

    #[test]
    fn project_registration_rejects_repository_inside_runtime_home() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-repo-inside-runtime")?;
        let repo_root = runtime_home.path().join("repo");
        fs::create_dir_all(&repo_root)?;
        initialize_runtime_home(runtime_home.path(), "runtime_home_contains_repo", "{}")?;

        let error = register_project(
            runtime_home.path(),
            ProjectRegistration {
                project_id: "project_repo_inside".to_owned(),
                repo_root,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("repository under Runtime Home should be rejected");

        assert!(error
            .to_string()
            .contains("Product Repository must not be inside Volicord Runtime Home"));
        assert!(!runtime_home
            .project_state_db_path("project_repo_inside")
            .exists());
        Ok(())
    }

    #[test]
    fn project_registration_rejects_runtime_home_inside_repository() -> Result<(), Box<dyn Error>> {
        let root = TempRuntimeHome::new("store-runtime-inside-repo")?;
        let repo_root = root.create_product_repo("repo")?;
        let runtime_home = repo_root.join(".volicord");
        initialize_runtime_home(&runtime_home, "runtime_home_inside_repo", "{}")?;

        let error = register_project(
            &runtime_home,
            ProjectRegistration {
                project_id: "project_runtime_inside".to_owned(),
                repo_root,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("Runtime Home under repository should be rejected");

        assert!(error
            .to_string()
            .contains("Volicord Runtime Home must not be inside Product Repository"));
        assert!(!project_home_path(&runtime_home, "project_runtime_inside").exists());
        Ok(())
    }

    #[test]
    fn project_registration_accepts_separate_sibling_paths() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-sibling-paths")?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        initialize_runtime_home(runtime_home.path(), "runtime_home_sibling", "{}")?;

        let record = register_project(
            runtime_home.path(),
            ProjectRegistration {
                project_id: "project_sibling".to_owned(),
                repo_root: repo_root.clone(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;

        assert_eq!(record.repo_root, fs::canonicalize(repo_root)?);
        assert!(record.project_home.starts_with(runtime_home.path()));
        assert!(record.state_db_path.exists());
        Ok(())
    }

    #[test]
    fn repo_project_registration_uses_basename_with_unique_safe_aliases(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-repo-project-basename")?;
        let repo_a = runtime_home.create_product_repo("left/repo")?;
        let repo_b = runtime_home.create_product_repo("right/repo")?;
        initialize_runtime_home(runtime_home.path(), "runtime_home_repo_project", "{}")?;

        let first = ensure_project_for_repo(
            runtime_home.path(),
            RepoProjectRegistration {
                project_name: None,
                project_alias: None,
                repo_root: repo_a,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        let second = ensure_project_for_repo(
            runtime_home.path(),
            RepoProjectRegistration {
                project_name: None,
                project_alias: None,
                repo_root: repo_b,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;

        assert_eq!(first.project_name, "repo");
        assert_eq!(second.project_name, "repo");
        assert_ne!(first.project_internal_id, second.project_internal_id);
        assert_ne!(first.project_alias, second.project_alias);
        assert!(first.project_alias.starts_with("repo-"));
        assert!(second.project_alias.starts_with("repo-"));
        Ok(())
    }

    #[test]
    fn repo_project_registration_reuses_existing_project_without_renaming(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-repo-project-reuse")?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        initialize_runtime_home(runtime_home.path(), "runtime_home_repo_reuse", "{}")?;

        let original = ensure_project_for_repo(
            runtime_home.path(),
            RepoProjectRegistration {
                project_name: None,
                project_alias: None,
                repo_root: repo_root.clone(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        rename_project(
            runtime_home.path(),
            &original.project_internal_id,
            "Renamed Project",
            None,
        )?;

        let reused = ensure_project_for_repo(
            runtime_home.path(),
            RepoProjectRegistration {
                project_name: None,
                project_alias: None,
                repo_root,
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{\"ignored\":true}".to_owned(),
            },
        )?;

        assert_eq!(reused.project_internal_id, original.project_internal_id);
        assert_eq!(reused.project_name, "Renamed Project");
        assert_eq!(reused.metadata_json, "{}");
        Ok(())
    }

    #[test]
    fn project_registration_accepts_valid_custom_project_home() -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-custom-project-home")?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        let project_home = runtime_home.path().join("custom-projects/project_custom");
        initialize_runtime_home(runtime_home.path(), "runtime_home_custom_project", "{}")?;

        let record = register_project(
            runtime_home.path(),
            ProjectRegistration {
                project_id: "project_custom".to_owned(),
                repo_root,
                project_home: Some(project_home.clone()),
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        let project = project_record_for_execution(runtime_home.path(), "project_custom")?
            .expect("project should be registered");
        let store = CoreProjectStore::open(runtime_home.path(), &ProjectId::new("project_custom"))?;

        assert_eq!(record.project_home, project_home);
        assert_eq!(project.project_home, project_home);
        assert_eq!(
            project.state_db_path,
            project_home.join(PROJECT_STATE_DB_FILE)
        );
        assert_eq!(store.project_record().state_db_path, project.state_db_path);
        assert!(project.state_db_path.exists());
        Ok(())
    }

    #[test]
    fn project_registration_rejects_custom_home_outside_runtime_home() -> Result<(), Box<dyn Error>>
    {
        let runtime_home = TempRuntimeHome::new("store-project-home-outside")?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        let project_home = runtime_home
            .path()
            .parent()
            .expect("runtime home has parent")
            .join("outside-project-home");
        initialize_runtime_home(
            runtime_home.path(),
            "runtime_home_project_home_outside",
            "{}",
        )?;

        let error = register_project(
            runtime_home.path(),
            ProjectRegistration {
                project_id: "project_home_outside".to_owned(),
                repo_root,
                project_home: Some(project_home.clone()),
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("project_home outside Runtime Home should be rejected");

        assert!(error.to_string().contains("project_home must be inside"));
        assert!(!project_home.exists());
        Ok(())
    }

    #[test]
    fn project_registration_rejects_custom_home_overlapping_repository(
    ) -> Result<(), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new("store-project-home-repo-overlap")?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        let project_home = repo_root.join(".volicord-project");
        initialize_runtime_home(
            runtime_home.path(),
            "runtime_home_project_home_overlap",
            "{}",
        )?;

        let error = register_project(
            runtime_home.path(),
            ProjectRegistration {
                project_id: "project_home_overlap".to_owned(),
                repo_root,
                project_home: Some(project_home.clone()),
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
        .expect_err("project_home overlapping Product Repository should be rejected");

        assert!(error
            .to_string()
            .contains("project_home must not overlap Product Repository"));
        assert!(!project_home.exists());
        Ok(())
    }

    #[test]
    fn checked_project_record_accepts_valid_existing_registration() -> Result<(), Box<dyn Error>> {
        let (runtime_home, repo_root) = registered_project("store-checked-valid", "project_valid")?;

        let project = project_record_for_execution(runtime_home.path(), "project_valid")?
            .expect("project should be registered");
        let store = CoreProjectStore::open(runtime_home.path(), &ProjectId::new("project_valid"))?;

        assert_eq!(project.repo_root, fs::canonicalize(repo_root)?);
        assert_eq!(
            project.state_db_path,
            project.project_home.join(PROJECT_STATE_DB_FILE)
        );
        assert_eq!(store.project_record().project_id, "project_valid");
        assert_eq!(store.project_record().state_db_path, project.state_db_path);
        Ok(())
    }

    #[test]
    fn checked_project_list_rejects_unsafe_stored_project_id() -> Result<(), Box<dyn Error>> {
        let original_project_id = "project_unsafe_id_original";
        let damaged_project_id = "project/unsafe";
        let (runtime_home, _) = registered_project("store-checked-unsafe-id", original_project_id)?;
        let original = project_record(runtime_home.path(), original_project_id)?
            .expect("project should be registered");

        replace_project_id(runtime_home.path(), original_project_id, damaged_project_id)?;

        let list_error = list_projects(runtime_home.path())
            .expect_err("project listing should reject unsafe stored project_id");
        assert_invalid_project_registration(list_error, "invalid_project_id");
        let damaged = raw_project_record(runtime_home.path(), damaged_project_id)?;
        assert_eq!(damaged.project_id, damaged_project_id);
        assert_eq!(damaged.project_home, original.project_home);
        assert_eq!(damaged.state_db_path, original.state_db_path);
        assert_registry_record_unchanged_and_visible(
            runtime_home.path(),
            damaged_project_id,
            &damaged,
        )?;
        Ok(())
    }

    #[test]
    fn checked_project_record_rejects_state_db_path_mismatch_before_alternate_creation(
    ) -> Result<(), Box<dyn Error>> {
        let project_id = "project_state_db_mismatch_missing";
        let (runtime_home, _) = registered_project("store-state-db-missing-alt", project_id)?;
        let original =
            project_record(runtime_home.path(), project_id)?.expect("project should be registered");
        let expected_state_path = original.project_home.join(PROJECT_STATE_DB_FILE);
        let alternate_state_path = runtime_home.path().join("alternate/missing-state.sqlite");

        replace_project_state_db_path(runtime_home.path(), project_id, &alternate_state_path)?;
        assert!(!alternate_state_path.exists());

        let lookup_error = project_record(runtime_home.path(), project_id)
            .expect_err("mismatched state_db_path should be rejected by project lookup");
        assert_state_db_path_mismatch(lookup_error, &alternate_state_path, &expected_state_path);
        let list_error = list_projects(runtime_home.path())
            .expect_err("mismatched state_db_path should be rejected by project listing");
        assert_state_db_path_mismatch(list_error, &alternate_state_path, &expected_state_path);
        let error = project_record_for_execution(runtime_home.path(), project_id)
            .expect_err("mismatched state_db_path should be rejected for execution");
        assert_state_db_path_mismatch(error, &alternate_state_path, &expected_state_path);
        let open_error = CoreProjectStore::open(runtime_home.path(), &ProjectId::new(project_id))
            .expect_err("Core store open should reject mismatched state_db_path");
        assert_state_db_path_mismatch(open_error, &alternate_state_path, &expected_state_path);
        assert!(!alternate_state_path.exists());

        let damaged = raw_project_record(runtime_home.path(), project_id)?;
        assert_eq!(damaged.state_db_path, alternate_state_path);
        assert_registry_record_unchanged_and_visible(runtime_home.path(), project_id, &damaged)?;
        Ok(())
    }

    #[test]
    fn checked_project_record_rejects_existing_alternate_without_mutating_alternate(
    ) -> Result<(), Box<dyn Error>> {
        let project_id = "project_state_db_mismatch_existing";
        let (runtime_home, _) = registered_project("store-state-db-existing-alt", project_id)?;
        let original =
            project_record(runtime_home.path(), project_id)?.expect("project should be registered");
        let expected_state_path = original.project_home.join(PROJECT_STATE_DB_FILE);
        let alternate_state_path = runtime_home.path().join("alternate/existing-state.sqlite");
        fs::create_dir_all(
            alternate_state_path
                .parent()
                .expect("alternate state path has parent"),
        )?;
        let conn = open_project_state_database(&alternate_state_path)?;
        drop(conn);
        let migrations_before = migration_count(&alternate_state_path)?;

        replace_project_state_db_path(runtime_home.path(), project_id, &alternate_state_path)?;

        let open_error = CoreProjectStore::open(runtime_home.path(), &ProjectId::new(project_id))
            .expect_err("Core store open should reject mismatched state_db_path");
        assert_state_db_path_mismatch(open_error, &alternate_state_path, &expected_state_path);
        assert_eq!(migration_count(&alternate_state_path)?, migrations_before);
        let lookup_error = project_record(runtime_home.path(), project_id)
            .expect_err("mismatched state_db_path should be rejected by project lookup");
        assert_state_db_path_mismatch(lookup_error, &alternate_state_path, &expected_state_path);
        let list_error = list_projects(runtime_home.path())
            .expect_err("mismatched state_db_path should be rejected by project listing");
        assert_state_db_path_mismatch(list_error, &alternate_state_path, &expected_state_path);
        let damaged = raw_project_record(runtime_home.path(), project_id)?;
        assert_eq!(damaged.state_db_path, alternate_state_path);
        assert_registry_record_unchanged_and_visible(runtime_home.path(), project_id, &damaged)?;
        Ok(())
    }

    #[test]
    fn checked_project_record_rejects_same_path_registration_for_operational_reads(
    ) -> Result<(), Box<dyn Error>> {
        let (runtime_home, _) = registered_project("store-checked-same", "project_same_legacy")?;
        replace_project_repo_root(
            runtime_home.path(),
            "project_same_legacy",
            runtime_home.path(),
        )?;

        let error = project_record_for_execution(runtime_home.path(), "project_same_legacy")
            .expect_err("same-path legacy registration should be rejected for execution");
        assert_invalid_project_registration(error, "same_path");
        let open_error =
            CoreProjectStore::open(runtime_home.path(), &ProjectId::new("project_same_legacy"))
                .expect_err("Core store open should reject same-path legacy registration");
        assert_invalid_project_registration(open_error, "same_path");

        let lookup_error = project_record(runtime_home.path(), "project_same_legacy")
            .expect_err("project lookup should reject same-path registration");
        assert_invalid_project_registration(lookup_error, "same_path");
        let list_error = list_projects(runtime_home.path())
            .expect_err("project listing should reject same-path registration");
        assert_invalid_project_registration(list_error, "same_path");
        let damaged = raw_project_record(runtime_home.path(), "project_same_legacy")?;
        assert_eq!(damaged.repo_root, runtime_home.path());
        assert_registry_record_unchanged_and_visible(
            runtime_home.path(),
            "project_same_legacy",
            &damaged,
        )?;
        Ok(())
    }

    #[test]
    fn checked_project_record_rejects_legacy_repository_inside_runtime_home(
    ) -> Result<(), Box<dyn Error>> {
        let (runtime_home, _) =
            registered_project("store-checked-repo-inside", "project_repo_inside_legacy")?;
        let repo_root = runtime_home.path().join("legacy-product-repo");
        fs::create_dir_all(&repo_root)?;
        replace_project_repo_root(
            runtime_home.path(),
            "project_repo_inside_legacy",
            &repo_root,
        )?;

        let error = CoreProjectStore::open(
            runtime_home.path(),
            &ProjectId::new("project_repo_inside_legacy"),
        )
        .expect_err("repository under Runtime Home should be rejected for execution");

        assert_invalid_project_registration(error, "runtime_home_contains_product_repository");
        let lookup_error = project_record(runtime_home.path(), "project_repo_inside_legacy")
            .expect_err("project lookup should reject repository under Runtime Home");
        assert_invalid_project_registration(
            lookup_error,
            "runtime_home_contains_product_repository",
        );
        let list_error = list_projects(runtime_home.path())
            .expect_err("project listing should reject repository under Runtime Home");
        assert_invalid_project_registration(list_error, "runtime_home_contains_product_repository");
        let damaged = raw_project_record(runtime_home.path(), "project_repo_inside_legacy")?;
        assert_eq!(damaged.repo_root, repo_root);
        assert_registry_record_unchanged_and_visible(
            runtime_home.path(),
            "project_repo_inside_legacy",
            &damaged,
        )?;
        Ok(())
    }

    #[test]
    fn checked_project_record_rejects_legacy_runtime_home_inside_repository(
    ) -> Result<(), Box<dyn Error>> {
        let (runtime_home, _) = registered_project(
            "store-checked-runtime-inside",
            "project_runtime_inside_legacy",
        )?;
        let repo_root = runtime_home
            .path()
            .parent()
            .expect("runtime home has parent")
            .to_path_buf();
        replace_project_repo_root(
            runtime_home.path(),
            "project_runtime_inside_legacy",
            &repo_root,
        )?;

        let error = CoreProjectStore::open(
            runtime_home.path(),
            &ProjectId::new("project_runtime_inside_legacy"),
        )
        .expect_err("Runtime Home under repository should be rejected for execution");

        assert_invalid_project_registration(error, "product_repository_contains_runtime_home");
        let lookup_error = project_record(runtime_home.path(), "project_runtime_inside_legacy")
            .expect_err("project lookup should reject Runtime Home under repository");
        assert_invalid_project_registration(
            lookup_error,
            "product_repository_contains_runtime_home",
        );
        let list_error = list_projects(runtime_home.path())
            .expect_err("project listing should reject Runtime Home under repository");
        assert_invalid_project_registration(list_error, "product_repository_contains_runtime_home");
        let damaged = raw_project_record(runtime_home.path(), "project_runtime_inside_legacy")?;
        assert_registry_record_unchanged_and_visible(
            runtime_home.path(),
            "project_runtime_inside_legacy",
            &damaged,
        )?;
        Ok(())
    }

    #[test]
    fn checked_project_record_rejects_project_home_outside_runtime_home(
    ) -> Result<(), Box<dyn Error>> {
        let project_id = "project_home_outside_damaged";
        let (runtime_home, _) = registered_project("store-checked-project-home", project_id)?;
        let original =
            project_record(runtime_home.path(), project_id)?.expect("project should be registered");
        let outside_project_home = runtime_home
            .path()
            .parent()
            .expect("runtime home has parent")
            .join("outside-project-home-damaged");

        replace_project_home(runtime_home.path(), project_id, &outside_project_home)?;

        let lookup_error = project_record(runtime_home.path(), project_id)
            .expect_err("project lookup should reject project_home outside Runtime Home");
        assert_invalid_project_registration(lookup_error, "project_home_outside_runtime_home");
        let list_error = list_projects(runtime_home.path())
            .expect_err("project listing should reject project_home outside Runtime Home");
        assert_invalid_project_registration(list_error, "project_home_outside_runtime_home");
        let open_error = CoreProjectStore::open(runtime_home.path(), &ProjectId::new(project_id))
            .expect_err("Core store open should reject project_home outside Runtime Home");
        assert_invalid_project_registration(open_error, "project_home_outside_runtime_home");
        let damaged = raw_project_record(runtime_home.path(), project_id)?;
        assert_eq!(damaged.project_home, outside_project_home);
        assert_eq!(damaged.state_db_path, original.state_db_path);
        assert_registry_record_unchanged_and_visible(runtime_home.path(), project_id, &damaged)?;
        Ok(())
    }

    fn registered_project(
        prefix: &str,
        project_id: &str,
    ) -> Result<(TempRuntimeHome, PathBuf), Box<dyn Error>> {
        let runtime_home = TempRuntimeHome::new(prefix)?;
        let repo_root = runtime_home.create_product_repo("repo")?;
        initialize_runtime_home(
            runtime_home.path(),
            &format!("runtime_home_{project_id}"),
            "{}",
        )?;
        register_project(
            runtime_home.path(),
            ProjectRegistration {
                project_id: project_id.to_owned(),
                repo_root: repo_root.clone(),
                project_home: None,
                status: ACTIVE_PROJECT_STATUS.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        Ok((runtime_home, repo_root))
    }

    fn replace_project_repo_root(
        runtime_home: &Path,
        project_id: &str,
        repo_root: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let conn = open_registry_database(registry_db_path(runtime_home))?;
        conn.execute(
            "UPDATE projects SET repo_root = ?2 WHERE project_internal_id = ?1",
            rusqlite::params![project_id, repo_root.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    fn replace_project_id(
        runtime_home: &Path,
        old_project_id: &str,
        new_project_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let conn = rusqlite::Connection::open(registry_db_path(runtime_home))?;
        conn.pragma_update(None, "foreign_keys", "OFF")?;
        conn.execute(
            "UPDATE projects SET project_internal_id = ?2 WHERE project_internal_id = ?1",
            rusqlite::params![old_project_id, new_project_id],
        )?;
        conn.execute(
            "UPDATE project_aliases SET project_internal_id = ?2 WHERE project_internal_id = ?1",
            rusqlite::params![old_project_id, new_project_id],
        )?;
        Ok(())
    }

    fn replace_project_state_db_path(
        runtime_home: &Path,
        project_id: &str,
        state_db_path: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let conn = open_registry_database(registry_db_path(runtime_home))?;
        conn.execute(
            "UPDATE projects SET state_db_path = ?2 WHERE project_internal_id = ?1",
            rusqlite::params![project_id, state_db_path.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    fn replace_project_home(
        runtime_home: &Path,
        project_id: &str,
        project_home: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let conn = open_registry_database(registry_db_path(runtime_home))?;
        conn.execute(
            "UPDATE projects SET project_home = ?2 WHERE project_internal_id = ?1",
            rusqlite::params![project_id, project_home.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    fn migration_count(state_path: &Path) -> StoreResult<i64> {
        let conn = open_read_only_database(state_path)?;
        Ok(conn.query_row(
            "SELECT COUNT(*)
               FROM schema_migrations
              WHERE database_kind = ?1",
            [PROJECT_STATE_DATABASE_KIND],
            |row| row.get(0),
        )?)
    }

    fn assert_registry_record_unchanged_and_visible(
        runtime_home: &Path,
        project_id: &str,
        expected: &ProjectRecord,
    ) -> StoreResult<()> {
        let project = raw_project_record(runtime_home, project_id)?;
        assert_eq!(&project, expected);

        let inspection = inspect_registry_database(runtime_home);
        let DatabaseInspection::Present(snapshot) = inspection else {
            panic!("expected present registry inspection, got {inspection:?}");
        };
        assert!(
            snapshot.projects.iter().any(|project| {
                project.project_id == expected.project_id
                    && project.repo_root == expected.repo_root
                    && project.project_home == expected.project_home
                    && project.state_db_path == expected.state_db_path
            }),
            "invalid registry record should remain inspectable"
        );
        Ok(())
    }

    fn raw_project_record(runtime_home: &Path, project_id: &str) -> StoreResult<ProjectRecord> {
        let conn = open_read_only_database(registry_db_path(runtime_home))?;
        Ok(conn.query_row(
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
             WHERE project_internal_id = ?1",
            rusqlite::params![project_id],
            project_record_from_row,
        )?)
    }

    fn assert_invalid_project_registration(error: StoreError, relationship: &str) {
        match error {
            StoreError::InvalidProjectRegistration {
                relationship: actual,
                detail,
                ..
            } => {
                assert_eq!(actual, relationship);
                assert!(!detail.is_empty());
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    fn assert_state_db_path_mismatch(error: StoreError, stored: &Path, expected: &Path) {
        match error {
            StoreError::InvalidProjectRegistration {
                field,
                relationship,
                detail,
                ..
            } => {
                assert_eq!(field, "state_db_path");
                assert_eq!(relationship, "state_db_path_mismatch");
                assert!(detail.contains(&stored.display().to_string()));
                assert!(detail.contains(&expected.display().to_string()));
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
