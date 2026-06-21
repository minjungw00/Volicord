use std::path::{Path, PathBuf};

use harness_types::{SurfaceInteractionRole, BASELINE_PROJECT_ENFORCEMENT_PROFILE_JSON};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::{
    migrations::{PROJECT_STATE_SCHEMA_VERSION, REGISTRY_SCHEMA_VERSION, STORAGE_PROFILE},
    runtime_home::{
        validate_project_home_boundary, validate_runtime_home_product_repository,
        RuntimePathBoundaryError,
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
    validate_project_id(&registration.project_id)?;
    validate_project_status(&registration.status)?;
    validate_json_object("projects.metadata_json", &registration.metadata_json)?;

    let path_validation =
        validate_runtime_home_product_repository(runtime_home.as_ref(), &registration.repo_root)
            .map_err(path_boundary_input)?;
    let runtime_home = path_validation.runtime_home;
    let repo_root = path_validation.repo_root;
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
                registration.project_id,
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

fn path_boundary_input(error: crate::runtime_home::RuntimePathBoundaryError) -> StoreError {
    StoreError::InvalidInput {
        detail: error.to_string(),
    }
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
    validate_project_id(project_id)?;
    let registry_path = registry_db_path(runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database(registry_path)?;
    project_record_from_conn(&conn, project_id)
}

/// Reads one registered project and validates it before execution use.
pub fn project_record_for_execution(
    runtime_home: impl AsRef<Path>,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    let runtime_home = runtime_home.as_ref();
    let project = project_record(runtime_home, project_id)?;
    if let Some(project) = project.as_ref() {
        validate_project_record_for_execution(runtime_home, project)?;
    }
    Ok(project)
}

/// Validates a stored project registration before execution use.
pub fn validate_project_record_for_execution(
    runtime_home: impl AsRef<Path>,
    project: &ProjectRecord,
) -> StoreResult<()> {
    validate_runtime_home_product_repository(runtime_home.as_ref(), &project.repo_root)
        .map_err(|error| registered_project_path_error(project, "repo_root", error))?;
    validate_project_home_boundary(
        runtime_home.as_ref(),
        &project.repo_root,
        &project.project_home,
    )
    .map_err(|error| registered_project_path_error(project, "project_home", error))?;
    Ok(())
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

/// Registers or updates a local surface for a project.
pub fn register_surface(
    runtime_home: impl AsRef<Path>,
    registration: SurfaceRegistration,
) -> StoreResult<SurfaceRecord> {
    validate_project_id(&registration.project_id)?;
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

    let project = project_record_for_execution(runtime_home, &registration.project_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project",
            id: registration.project_id.clone(),
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
    validate_project_id(project_id)?;
    let project = project_record_for_execution(runtime_home, project_id)?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "project",
            id: project_id.to_owned(),
        }
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

/// Validates a project id that may become one `projects/{project_id}` path component.
pub fn validate_project_id(project_id: &str) -> StoreResult<()> {
    validate_identifier("project_id", project_id)?;
    validate_path_component("project_id", project_id)
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

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fs,
        path::{Path, PathBuf},
    };

    use crate::{
        core_pipeline::CoreProjectStore,
        migrations::{
            test_support::create_project_state_fixture_version, PROJECT_STATE_DATABASE_KIND,
        },
        sqlite::open_read_only_database,
    };
    use harness_test_support::TempRuntimeHome;
    use harness_types::{ProjectId, SurfaceInteractionRole};
    use rusqlite::Connection;

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
            .contains("Product Repository must not be inside Harness Runtime Home"));
        assert!(!runtime_home
            .project_state_db_path("project_repo_inside")
            .exists());
        Ok(())
    }

    #[test]
    fn project_registration_rejects_runtime_home_inside_repository() -> Result<(), Box<dyn Error>> {
        let root = TempRuntimeHome::new("store-runtime-inside-repo")?;
        let repo_root = root.create_product_repo("repo")?;
        let runtime_home = repo_root.join(".harness");
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
            .contains("Harness Runtime Home must not be inside Product Repository"));
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
        let project_home = repo_root.join(".harness-project");
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
        assert_eq!(store.project_record().project_id, "project_valid");
        Ok(())
    }

    #[test]
    fn checked_project_record_rejects_legacy_same_path_registration_but_keeps_listing(
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

        let projects = list_projects(runtime_home.path())?;
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].project_id, "project_same_legacy");
        assert_eq!(projects[0].repo_root, runtime_home.path());
        assert!(project_record(runtime_home.path(), "project_same_legacy")?.is_some());
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
        assert_eq!(
            project_record(runtime_home.path(), "project_repo_inside_legacy")?
                .expect("record remains readable")
                .repo_root,
            repo_root
        );
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
        assert!(project_record(runtime_home.path(), "project_runtime_inside_legacy")?.is_some());
        Ok(())
    }

    #[test]
    fn surface_management_accepts_valid_separate_project_paths() -> Result<(), Box<dyn Error>> {
        let (runtime_home, _) = registered_project("store-surface-valid", "project_surface_valid")?;

        let registered = register_surface(
            runtime_home.path(),
            surface_registration("project_surface_valid"),
        )?;
        let surfaces = list_surfaces(runtime_home.path(), "project_surface_valid")?;

        assert_eq!(registered.project_id, "project_surface_valid");
        assert_eq!(registered.surface_id, "surface_main");
        assert_eq!(surfaces, vec![registered]);
        Ok(())
    }

    #[test]
    fn surface_management_rejects_invalid_legacy_records_without_surface_mutation(
    ) -> Result<(), Box<dyn Error>> {
        for relationship in InvalidProjectRelationship::ALL {
            let project_id = relationship.project_id("surface_existing");
            let (runtime_home, _) =
                registered_project(&relationship.prefix("surface-existing"), &project_id)?;
            register_surface(runtime_home.path(), surface_registration(&project_id))?;
            relationship.replace_repo_root(&runtime_home, &project_id)?;

            let project = project_record(runtime_home.path(), &project_id)?
                .expect("invalid legacy project record remains readable");
            let state_path = project.state_db_path.clone();
            let registry_before = project.clone();
            let migrations_before = migration_count(&state_path)?;
            let surfaces_before = surface_records_from_state(&state_path, &project_id)?;

            let update_error = register_surface(
                runtime_home.path(),
                updated_surface_registration(&project_id),
            )
            .expect_err("invalid legacy registration should reject surface update");
            assert_invalid_project_registration(update_error, relationship.expected_error());
            let add_error = register_surface(
                runtime_home.path(),
                additional_surface_registration(&project_id),
            )
            .expect_err("invalid legacy registration should reject surface insert");
            assert_invalid_project_registration(add_error, relationship.expected_error());
            let list_error = list_surfaces(runtime_home.path(), &project_id)
                .expect_err("invalid legacy registration should reject surface listing");
            assert_invalid_project_registration(list_error, relationship.expected_error());

            assert_eq!(migration_count(&state_path)?, migrations_before);
            assert_eq!(
                surface_records_from_state(&state_path, &project_id)?,
                surfaces_before
            );
            assert_registry_record_unchanged_and_visible(
                runtime_home.path(),
                &project_id,
                &registry_before,
            )?;
        }
        Ok(())
    }

    #[test]
    fn surface_management_rejects_invalid_legacy_records_before_missing_database_check(
    ) -> Result<(), Box<dyn Error>> {
        for relationship in InvalidProjectRelationship::ALL {
            let project_id = relationship.project_id("surface_missing_db");
            let (runtime_home, _) =
                registered_project(&relationship.prefix("surface-missing-db"), &project_id)?;
            relationship.replace_repo_root(&runtime_home, &project_id)?;

            let project = project_record(runtime_home.path(), &project_id)?
                .expect("invalid legacy project record remains readable");
            let state_path = project.state_db_path.clone();
            let registry_before = project.clone();
            fs::remove_file(&state_path)?;
            assert!(!state_path.exists());

            let register_error =
                register_surface(runtime_home.path(), surface_registration(&project_id))
                    .expect_err(
                        "invalid legacy registration should reject before missing state DB",
                    );
            assert_invalid_project_registration(register_error, relationship.expected_error());
            assert!(!state_path.exists());

            let list_error = list_surfaces(runtime_home.path(), &project_id)
                .expect_err("invalid legacy registration should reject before missing state DB");
            assert_invalid_project_registration(list_error, relationship.expected_error());
            assert!(!state_path.exists());

            assert_registry_record_unchanged_and_visible(
                runtime_home.path(),
                &project_id,
                &registry_before,
            )?;
        }
        Ok(())
    }

    #[test]
    fn surface_management_rejects_invalid_legacy_records_without_migrating_historical_state(
    ) -> Result<(), Box<dyn Error>> {
        for relationship in InvalidProjectRelationship::ALL {
            let project_id = relationship.project_id("surface_historical");
            let (runtime_home, _) =
                registered_project(&relationship.prefix("surface-historical"), &project_id)?;
            relationship.replace_repo_root(&runtime_home, &project_id)?;

            let project = project_record(runtime_home.path(), &project_id)?
                .expect("invalid legacy project record remains readable");
            let state_path = project.state_db_path.clone();
            let registry_before = project.clone();
            fs::remove_file(&state_path)?;
            let mut conn = Connection::open(&state_path)?;
            create_project_state_fixture_version(&mut conn, &project_id, 5)?;
            drop(conn);

            let migrations_before = migration_count(&state_path)?;
            let surface_count_before = surface_count(&state_path)?;
            assert_eq!(migrations_before, 5);
            assert!(!column_exists(
                &state_path,
                "project_state",
                "enforcement_profile_json"
            )?);
            assert!(!column_exists(&state_path, "surfaces", "interaction_role")?);

            let list_error = list_surfaces(runtime_home.path(), &project_id)
                .expect_err("invalid legacy registration should reject before state DB migration");
            assert_invalid_project_registration(list_error, relationship.expected_error());
            assert_historical_project_state_unchanged(
                &state_path,
                migrations_before,
                surface_count_before,
            )?;

            let register_error =
                register_surface(runtime_home.path(), surface_registration(&project_id))
                    .expect_err("invalid legacy registration should reject before surface insert");
            assert_invalid_project_registration(register_error, relationship.expected_error());
            assert_historical_project_state_unchanged(
                &state_path,
                migrations_before,
                surface_count_before,
            )?;

            assert_registry_record_unchanged_and_visible(
                runtime_home.path(),
                &project_id,
                &registry_before,
            )?;
        }
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
            "UPDATE projects SET repo_root = ?2 WHERE project_id = ?1",
            rusqlite::params![project_id, repo_root.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    #[derive(Clone, Copy)]
    enum InvalidProjectRelationship {
        SamePath,
        RepositoryInsideRuntimeHome,
        RuntimeHomeInsideRepository,
    }

    impl InvalidProjectRelationship {
        const ALL: [Self; 3] = [
            Self::SamePath,
            Self::RepositoryInsideRuntimeHome,
            Self::RuntimeHomeInsideRepository,
        ];

        fn name(self) -> &'static str {
            match self {
                Self::SamePath => "same",
                Self::RepositoryInsideRuntimeHome => "repo_inside_runtime",
                Self::RuntimeHomeInsideRepository => "runtime_inside_repo",
            }
        }

        fn expected_error(self) -> &'static str {
            match self {
                Self::SamePath => "same_path",
                Self::RepositoryInsideRuntimeHome => "runtime_home_contains_product_repository",
                Self::RuntimeHomeInsideRepository => "product_repository_contains_runtime_home",
            }
        }

        fn prefix(self, suffix: &str) -> String {
            format!("store-{suffix}-{}", self.name())
        }

        fn project_id(self, suffix: &str) -> String {
            format!("project_{suffix}_{}", self.name())
        }

        fn replace_repo_root(
            self,
            runtime_home: &TempRuntimeHome,
            project_id: &str,
        ) -> Result<PathBuf, Box<dyn Error>> {
            let repo_root = match self {
                Self::SamePath => runtime_home.path().to_path_buf(),
                Self::RepositoryInsideRuntimeHome => {
                    let repo_root = runtime_home.path().join("legacy-product-repo");
                    fs::create_dir_all(&repo_root)?;
                    repo_root
                }
                Self::RuntimeHomeInsideRepository => runtime_home
                    .path()
                    .parent()
                    .expect("runtime home has parent")
                    .to_path_buf(),
            };
            replace_project_repo_root(runtime_home.path(), project_id, &repo_root)?;
            Ok(repo_root)
        }
    }

    fn surface_registration(project_id: &str) -> SurfaceRegistration {
        SurfaceRegistration {
            project_id: project_id.to_owned(),
            surface_id: "surface_main".to_owned(),
            surface_instance_id: "surface_instance_main".to_owned(),
            surface_kind: "local_test".to_owned(),
            interaction_role: SurfaceInteractionRole::Agent,
            display_name: Some("Main Surface".to_owned()),
            capability_profile_json: "{}".to_owned(),
            local_access_json: "{}".to_owned(),
            metadata_json: "{}".to_owned(),
        }
    }

    fn updated_surface_registration(project_id: &str) -> SurfaceRegistration {
        SurfaceRegistration {
            display_name: Some("Updated Surface".to_owned()),
            metadata_json: "{\"changed\":true}".to_owned(),
            ..surface_registration(project_id)
        }
    }

    fn additional_surface_registration(project_id: &str) -> SurfaceRegistration {
        SurfaceRegistration {
            surface_id: "surface_extra".to_owned(),
            surface_instance_id: "surface_instance_extra".to_owned(),
            display_name: Some("Extra Surface".to_owned()),
            ..surface_registration(project_id)
        }
    }

    fn surface_records_from_state(
        state_path: &Path,
        project_id: &str,
    ) -> StoreResult<Vec<SurfaceRecord>> {
        let conn = open_read_only_database(state_path)?;
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

    fn surface_count(state_path: &Path) -> StoreResult<i64> {
        let conn = open_read_only_database(state_path)?;
        Ok(conn.query_row("SELECT COUNT(*) FROM surfaces", [], |row| row.get(0))?)
    }

    fn column_exists(state_path: &Path, table: &str, column: &str) -> StoreResult<bool> {
        let conn = open_read_only_database(state_path)?;
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

    fn assert_historical_project_state_unchanged(
        state_path: &Path,
        expected_migration_count: i64,
        expected_surface_count: i64,
    ) -> StoreResult<()> {
        assert_eq!(migration_count(state_path)?, expected_migration_count);
        assert_eq!(surface_count(state_path)?, expected_surface_count);
        assert!(!column_exists(
            state_path,
            "project_state",
            "enforcement_profile_json"
        )?);
        assert!(!column_exists(state_path, "surfaces", "interaction_role")?);
        Ok(())
    }

    fn assert_registry_record_unchanged_and_visible(
        runtime_home: &Path,
        project_id: &str,
        expected: &ProjectRecord,
    ) -> StoreResult<()> {
        let project = project_record(runtime_home, project_id)?.expect("project remains readable");
        assert_eq!(&project, expected);

        let projects = list_projects(runtime_home)?;
        assert!(
            projects.iter().any(|project| project == expected),
            "invalid registry record should remain visible through project listing"
        );
        Ok(())
    }

    fn assert_invalid_project_registration(error: StoreError, relationship: &str) {
        match error {
            StoreError::InvalidProjectRegistration {
                relationship: actual,
                detail,
                ..
            } => {
                assert_eq!(actual, relationship);
                assert!(detail.contains("Harness Runtime Home"));
                assert!(detail.contains("Product Repository"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
