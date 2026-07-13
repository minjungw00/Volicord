use std::{cell::RefCell, path::Path};

use volicord_types::ProjectId;

use super::CoreProjectStore;
use crate::{
    bootstrap::{
        project_record_for_execution, project_record_for_execution_read_only, ProjectRecord,
    },
    sqlite::{open_project_state_database, open_project_state_database_read_only},
    StoreError, StoreResult,
};

impl CoreProjectStore {
    /// Opens the registered project-local state store for Core pipeline work.
    pub fn open(runtime_home: impl AsRef<Path>, project_id: &ProjectId) -> StoreResult<Self> {
        let runtime_home = runtime_home.as_ref().to_path_buf();
        let project = project_record_for_execution(&runtime_home, project_id.as_str())?
            .ok_or_else(|| StoreError::NotFound {
                entity: "project",
                id: project_id.as_str().to_owned(),
            })?;

        if !project.state_db_path.exists() {
            return Err(StoreError::NotFound {
                entity: "project_state_database",
                id: project.state_db_path.display().to_string(),
            });
        }

        let conn = open_project_state_database(&project.state_db_path)?;
        Ok(Self {
            runtime_home,
            project,
            conn,
            writable: true,
            last_clock_sample: RefCell::new(None),
        })
    }

    /// Opens the registered project-local state store for read-only Core pipeline work.
    pub fn open_read_only(
        runtime_home: impl AsRef<Path>,
        project_id: &ProjectId,
    ) -> StoreResult<Self> {
        let runtime_home = runtime_home.as_ref().to_path_buf();
        let project = project_record_for_execution_read_only(&runtime_home, project_id.as_str())?
            .ok_or_else(|| StoreError::NotFound {
            entity: "project",
            id: project_id.as_str().to_owned(),
        })?;

        if !project.state_db_path.exists() {
            return Err(StoreError::NotFound {
                entity: "project_state_database",
                id: project.state_db_path.display().to_string(),
            });
        }

        let conn = open_project_state_database_read_only(&project.state_db_path)?;
        Ok(Self {
            runtime_home,
            project,
            conn,
            writable: false,
            last_clock_sample: RefCell::new(None),
        })
    }

    /// Returns the Runtime Home path that selected this project-local store.
    pub fn runtime_home(&self) -> &Path {
        &self.runtime_home
    }

    /// Returns the registry project row that selected this project-local store.
    pub const fn project_record(&self) -> &ProjectRecord {
        &self.project
    }

    /// Returns whether this store handle was opened for write-capable Core work.
    pub const fn is_writable(&self) -> bool {
        self.writable
    }
}
