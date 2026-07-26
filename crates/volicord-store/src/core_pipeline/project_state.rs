use rusqlite::{params, Connection, OptionalExtension, Transaction};
use volicord_types::values::UtcTimestamp;

use super::{facade::CoreProjectStore, validation::nonnegative_i64_to_u64};
use crate::{StoreError, StoreResult};

const PROJECT_STATE_COLUMNS: &str = "
    project_id, state_version, active_task_id, updated_at";

/// Current project-state header values needed by request routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStateHeader {
    pub project_id: String,
    pub state_version: u64,
    pub active_task_id: Option<String>,
    pub updated_at: String,
}

impl CoreProjectStore<'_> {
    /// Reads the current project-state header.
    pub fn project_state(&self) -> StoreResult<ProjectStateHeader> {
        read_project_state(&self.conn, &self.project.project_id)
    }
}

fn read_project_state(conn: &Connection, project_id: &str) -> StoreResult<ProjectStateHeader> {
    let sql = format!(
        "SELECT {PROJECT_STATE_COLUMNS}
           FROM project_state
          WHERE project_id = ?1"
    );
    let state = conn
        .query_row(&sql, params![project_id], project_state_from_row)
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project_state",
            id: project_id.to_owned(),
        })?;
    validate_project_state_updated_at(&state)?;
    Ok(state)
}

pub(super) fn read_project_state_tx(
    tx: &Transaction<'_>,
    project_id: &str,
) -> StoreResult<ProjectStateHeader> {
    let sql = format!(
        "SELECT {PROJECT_STATE_COLUMNS}
           FROM project_state
          WHERE project_id = ?1"
    );
    let state = tx
        .query_row(&sql, params![project_id], project_state_from_row)
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project_state",
            id: project_id.to_owned(),
        })?;
    validate_project_state_updated_at(&state)?;
    Ok(state)
}

fn project_state_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectStateHeader> {
    let state_version = row.get::<_, i64>(1)?;
    Ok(ProjectStateHeader {
        project_id: row.get(0)?,
        state_version: nonnegative_i64_to_u64("project_state.state_version", state_version)?,
        active_task_id: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

fn validate_project_state_updated_at(state: &ProjectStateHeader) -> StoreResult<()> {
    UtcTimestamp::parse(&state.updated_at)
        .and_then(|timestamp| {
            timestamp
                .ensure_canonical_rfc3339_representable()
                .map_err(|_| volicord_types::values::UtcTimestampParseError)
        })
        .map_err(|_| {
            StoreError::corrupt_owner_state_value("project_state", &state.project_id, "updated_at")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_state_decoder_rejects_a_negative_state_version() {
        let connection = Connection::open_in_memory().expect("in-memory database must open");
        let error = connection
            .query_row(
                "SELECT 'project', -1, NULL, '2026-01-01T00:00:00Z'",
                [],
                project_state_from_row,
            )
            .expect_err("negative state version must fail strict row decoding");

        assert!(matches!(
            error,
            rusqlite::Error::FromSqlConversionFailure(..)
        ));
    }
}
