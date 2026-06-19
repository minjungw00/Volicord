use std::path::{Path, PathBuf};

use harness_types::SurfaceInteractionRole;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::{
    migrations::{PROJECT_STATE_SCHEMA_VERSION, REGISTRY_SCHEMA_VERSION, STORAGE_PROFILE},
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

/// Project registration record stored in `registry.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRecord {
    pub project_id: String,
    pub runtime_home_id: String,
    pub repo_root: PathBuf,
    pub project_home: PathBuf,
    pub state_db_path: PathBuf,
    pub status: String,
    pub metadata_json: String,
}

/// Local surface registration input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceRegistration {
    pub project_id: String,
    pub surface_id: String,
    pub surface_instance_id: String,
    pub surface_kind: String,
    pub interaction_role: SurfaceInteractionRole,
    pub display_name: Option<String>,
    pub capability_profile_json: String,
    pub local_access_json: String,
    pub metadata_json: String,
}

/// Surface registration record stored in project `state.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceRecord {
    pub project_id: String,
    pub surface_id: String,
    pub surface_instance_id: String,
    pub surface_kind: String,
    pub interaction_role: String,
    pub display_name: Option<String>,
    pub capability_profile_json: String,
    pub local_access_json: String,
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
    let mut conn = open_registry_database(&registry_path)?;

    with_immediate_transaction(&mut conn, |tx| {
        tx.execute(
            "INSERT OR IGNORE INTO runtime_home (
                singleton_id,
                runtime_home_id,
                storage_profile,
                schema_version,
                created_at,
                updated_at,
                metadata_json
            )
            VALUES (
                1,
                ?1,
                ?2,
                ?3,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?4
            )",
            params![
                runtime_home_id,
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

/// Registers a Product Repository project and creates its project `state.sqlite`.
pub fn register_project(
    runtime_home: impl AsRef<Path>,
    registration: ProjectRegistration,
) -> StoreResult<ProjectRecord> {
    validate_identifier("project_id", &registration.project_id)?;
    if registration.project_home.is_none() {
        validate_path_component("project_id", &registration.project_id)?;
    }
    validate_project_status(&registration.status)?;
    validate_json_object("projects.metadata_json", &registration.metadata_json)?;

    let runtime_home = runtime_home.as_ref().to_path_buf();
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
        .unwrap_or_else(|| project_home_path(&runtime_home, &registration.project_id));
    let state_db_path = project_home.join(PROJECT_STATE_DB_FILE);
    let repo_root_text = path_to_text("repo_root", &registration.repo_root)?;
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
                metadata_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?4
            )
            ON CONFLICT(project_id) DO UPDATE SET
                storage_profile = excluded.storage_profile,
                schema_version = excluded.schema_version,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                metadata_json = excluded.metadata_json",
            params![
                registration.project_id,
                STORAGE_PROFILE,
                PROJECT_STATE_SCHEMA_VERSION,
                registration.metadata_json
            ],
        )?;
        Ok(())
    })?;

    with_immediate_transaction(&mut registry, |tx| {
        tx.execute(
            "INSERT INTO projects (
                project_id,
                runtime_home_id,
                repo_root,
                project_home,
                state_db_path,
                status,
                created_at,
                updated_at,
                metadata_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?7
            )
            ON CONFLICT(project_id) DO UPDATE SET
                runtime_home_id = excluded.runtime_home_id,
                repo_root = excluded.repo_root,
                project_home = excluded.project_home,
                state_db_path = excluded.state_db_path,
                status = excluded.status,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                metadata_json = excluded.metadata_json",
            params![
                registration.project_id,
                runtime_home_row.runtime_home_id,
                repo_root_text,
                project_home_text,
                state_db_path_text,
                registration.status,
                registration.metadata_json
            ],
        )?;
        Ok(())
    })?;

    project_record_from_conn(&registry, &registration.project_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "project",
            id: registration.project_id,
        }
    })
}

/// Lists registered projects in deterministic order.
pub fn list_projects(runtime_home: impl AsRef<Path>) -> StoreResult<Vec<ProjectRecord>> {
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(Vec::new());
    }

    let conn = open_registry_database(registry_path)?;
    let mut stmt = conn.prepare(
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
    )?;
    let rows = stmt.query_map([], project_record_from_row)?;
    let mut projects = Vec::new();
    for row in rows {
        projects.push(row?);
    }
    Ok(projects)
}

/// Reads one registered project from `registry.sqlite`.
pub fn project_record(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    validate_identifier("project_id", project_id)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database(registry_path)?;
    project_record_from_conn(&conn, project_id)
}

/// Registers or updates a local surface for a project.
pub fn register_surface(
    runtime_home: impl AsRef<Path>,
    registration: SurfaceRegistration,
) -> StoreResult<SurfaceRecord> {
    validate_identifier("project_id", &registration.project_id)?;
    validate_identifier("surface_id", &registration.surface_id)?;
    validate_identifier("surface_instance_id", &registration.surface_instance_id)?;
    validate_identifier("surface_kind", &registration.surface_kind)?;
    validate_surface_interaction_role(registration.interaction_role)?;
    validate_json_object(
        "surfaces.capability_profile_json",
        &registration.capability_profile_json,
    )?;
    validate_json_object(
        "surfaces.local_access_json",
        &registration.local_access_json,
    )?;
    validate_json_object("surfaces.metadata_json", &registration.metadata_json)?;

    let project = project_record(runtime_home, &registration.project_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "project",
            id: registration.project_id.clone(),
        }
    })?;
    require_existing_state_database(&project)?;
    let mut conn = open_project_state_database(&project.state_db_path)?;

    with_immediate_transaction(&mut conn, |tx| {
        tx.execute(
            "INSERT INTO surfaces (
                project_id,
                surface_id,
                surface_instance_id,
                surface_kind,
                interaction_role,
                display_name,
                capability_profile_json,
                local_access_json,
                registered_at,
                last_seen_at,
                metadata_json
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
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?9
            )
            ON CONFLICT(project_id, surface_id, surface_instance_id) DO UPDATE SET
                surface_kind = excluded.surface_kind,
                interaction_role = excluded.interaction_role,
                display_name = excluded.display_name,
                capability_profile_json = excluded.capability_profile_json,
                local_access_json = excluded.local_access_json,
                last_seen_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                metadata_json = excluded.metadata_json",
            params![
                registration.project_id,
                registration.surface_id,
                registration.surface_instance_id,
                registration.surface_kind,
                registration.interaction_role.as_str(),
                registration.display_name,
                registration.capability_profile_json,
                registration.local_access_json,
                registration.metadata_json
            ],
        )?;
        tx.execute(
            "UPDATE project_state
                SET default_surface_id = CASE
                        WHEN default_surface_id IS NULL THEN ?2
                        ELSE default_surface_id
                    END,
                    default_surface_instance_id = CASE
                        WHEN default_surface_instance_id IS NULL THEN ?3
                        ELSE default_surface_instance_id
                    END,
                    updated_at = CASE
                        WHEN default_surface_id IS NULL THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                        ELSE updated_at
                    END
              WHERE project_id = ?1",
            params![
                registration.project_id,
                registration.surface_id,
                registration.surface_instance_id
            ],
        )?;
        Ok(())
    })?;

    surface_record_from_conn(
        &conn,
        &registration.project_id,
        &registration.surface_id,
        &registration.surface_instance_id,
    )?
    .ok_or_else(|| StoreError::NotFound {
        entity: "surface",
        id: format!(
            "{}/{}/{}",
            registration.project_id, registration.surface_id, registration.surface_instance_id
        ),
    })
}

/// Lists registered surfaces for one project in deterministic order.
pub fn list_surfaces(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<Vec<SurfaceRecord>> {
    validate_identifier("project_id", project_id)?;
    let project =
        project_record(runtime_home, project_id)?.ok_or_else(|| StoreError::NotFound {
            entity: "project",
            id: project_id.to_owned(),
        })?;
    require_existing_state_database(&project)?;
    let conn = open_project_state_database(project.state_db_path)?;
    let mut stmt = conn.prepare(
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
         ORDER BY surface_id, surface_instance_id",
    )?;
    let rows = stmt.query_map(params![project_id], surface_record_from_row)?;
    let mut surfaces = Vec::new();
    for row in rows {
        surfaces.push(row?);
    }
    Ok(surfaces)
}

fn runtime_home_record_from_conn(
    conn: &Connection,
    runtime_home: PathBuf,
    registry_path: PathBuf,
) -> StoreResult<Option<RuntimeHomeRecord>> {
    conn.query_row(
        "SELECT runtime_home_id, storage_profile, schema_version
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
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

fn project_record_from_conn(
    conn: &Connection,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            runtime_home_id,
            repo_root,
            project_home,
            state_db_path,
            status,
            metadata_json
         FROM projects
         WHERE project_id = ?1",
        params![project_id],
        project_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn surface_record_from_conn(
    conn: &Connection,
    project_id: &str,
    surface_id: &str,
    surface_instance_id: &str,
) -> StoreResult<Option<SurfaceRecord>> {
    conn.query_row(
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
           AND surface_id = ?2
           AND surface_instance_id = ?3",
        params![project_id, surface_id, surface_instance_id],
        surface_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn project_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRecord> {
    Ok(ProjectRecord {
        project_id: row.get(0)?,
        runtime_home_id: row.get(1)?,
        repo_root: PathBuf::from(row.get::<_, String>(2)?),
        project_home: PathBuf::from(row.get::<_, String>(3)?),
        state_db_path: PathBuf::from(row.get::<_, String>(4)?),
        status: row.get(5)?,
        metadata_json: row.get(6)?,
    })
}

fn surface_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SurfaceRecord> {
    Ok(SurfaceRecord {
        project_id: row.get(0)?,
        surface_id: row.get(1)?,
        surface_instance_id: row.get(2)?,
        surface_kind: row.get(3)?,
        interaction_role: row.get(4)?,
        display_name: row.get(5)?,
        capability_profile_json: row.get(6)?,
        local_access_json: row.get(7)?,
        metadata_json: row.get(8)?,
    })
}

fn validate_surface_interaction_role(role: SurfaceInteractionRole) -> StoreResult<()> {
    match role {
        SurfaceInteractionRole::Agent | SurfaceInteractionRole::UserInteraction => Ok(()),
    }
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

fn validate_path_component(field: &'static str, value: &str) -> StoreResult<()> {
    if value == "." || value == ".." || value.contains('/') || value.contains('\\') {
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

fn require_existing_state_database(project: &ProjectRecord) -> StoreResult<()> {
    if project.state_db_path.exists() {
        Ok(())
    } else {
        Err(StoreError::NotFound {
            entity: "project_state_database",
            id: project.state_db_path.display().to_string(),
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
