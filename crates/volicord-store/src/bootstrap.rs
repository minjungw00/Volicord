use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
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

    project_record_from_conn(&registry, &runtime_home, &registration.project_id)?.ok_or_else(|| {
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
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
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
    validate_project_id(project_id)?;
    let runtime_home = runtime_home.as_ref().to_path_buf();
    let registry_path = registry_db_path(&runtime_home);
    if !registry_path.exists() {
        return Ok(None);
    }

    let conn = open_registry_database(registry_path)?;
    project_record_from_conn(&conn, &runtime_home, project_id)
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
    runtime_home: &Path,
    project_id: &str,
) -> StoreResult<Option<ProjectRecord>> {
    let project = conn
        .query_row(
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
        .map_err(StoreError::from)?;
    project
        .map(|project| validate_current_project_registration(runtime_home, &project))
        .transpose()
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
            "UPDATE projects SET repo_root = ?2 WHERE project_id = ?1",
            rusqlite::params![project_id, repo_root.to_string_lossy().as_ref()],
        )?;
        Ok(())
    }

    fn replace_project_id(
        runtime_home: &Path,
        old_project_id: &str,
        new_project_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let conn = open_registry_database(registry_db_path(runtime_home))?;
        conn.execute(
            "UPDATE projects SET project_id = ?2 WHERE project_id = ?1",
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
            "UPDATE projects SET state_db_path = ?2 WHERE project_id = ?1",
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
            "UPDATE projects SET project_home = ?2 WHERE project_id = ?1",
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
                project_id,
                runtime_home_id,
                repo_root,
                project_home,
                state_db_path,
                status,
                metadata_json
             FROM projects
             WHERE project_id = ?1",
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
