use std::{cell::RefCell, path::Path};

use volicord_types::ids::ProjectId;

use super::CoreProjectStore;
use crate::{
    bootstrap::{project_record_for_execution_admitted, project_record_for_execution_read_only},
    sqlite::{open_project_state_database_for_mutation, open_project_state_database_read_only},
    RuntimeHomeMutationContext, StoreError, StoreResult,
};

impl<'mutation> CoreProjectStore<'mutation> {
    /// Opens the registered project-local state store for Core pipeline work.
    pub fn open_for_mutation(
        context: &RuntimeHomeMutationContext<'mutation>,
        project_id: &ProjectId,
    ) -> StoreResult<Self> {
        let runtime_home = context.runtime_home().as_path().to_path_buf();
        let project = project_record_for_execution_admitted(context, project_id.as_str())?
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

        let conn = open_project_state_database_for_mutation(context, &project)?;
        Ok(Self {
            runtime_home,
            canonical_runtime_home: Some(context.runtime_home().clone()),
            project,
            conn,
            writable: true,
            mutation_context: Some(context.reborrow()),
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
            canonical_runtime_home: None,
            project,
            conn,
            writable: false,
            mutation_context: None,
            last_clock_sample: RefCell::new(None),
        })
    }
}
