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
    pub updated_at: UtcTimestamp,
}

#[derive(Debug)]
struct ProjectStateRaw {
    project_id: String,
    state_version: u64,
    active_task_id: Option<String>,
    updated_at: String,
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
    let raw = conn
        .query_row(&sql, params![project_id], project_state_raw_from_row)
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project_state",
            id: project_id.to_owned(),
        })?;
    decode_project_state(raw)
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
    let raw = tx
        .query_row(&sql, params![project_id], project_state_raw_from_row)
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project_state",
            id: project_id.to_owned(),
        })?;
    decode_project_state(raw)
}

fn project_state_raw_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectStateRaw> {
    let state_version = row.get::<_, i64>(1)?;
    Ok(ProjectStateRaw {
        project_id: row.get(0)?,
        state_version: nonnegative_i64_to_u64("project_state.state_version", state_version)?,
        active_task_id: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

fn decode_project_state(raw: ProjectStateRaw) -> StoreResult<ProjectStateHeader> {
    let updated_at = UtcTimestamp::parse(&raw.updated_at).map_err(|_| {
        StoreError::corrupt_owner_state_value("project_state", &raw.project_id, "updated_at")
    })?;
    updated_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| {
            StoreError::corrupt_owner_state_value("project_state", &raw.project_id, "updated_at")
        })?;
    Ok(ProjectStateHeader {
        project_id: raw.project_id,
        state_version: raw.state_version,
        active_task_id: raw.active_task_id,
        updated_at,
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
                project_state_raw_from_row,
            )
            .expect_err("negative state version must fail strict row decoding");

        assert!(matches!(
            error,
            rusqlite::Error::FromSqlConversionFailure(..)
        ));
    }
}
