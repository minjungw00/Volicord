use rusqlite::params;
use volicord_types::ids::TaskId;

use super::{facade::CoreProjectStore, validation::validate_identifier};
use crate::{
    guards::{unresolved_unrecorded_changes_from_conn, UnrecordedChangeRecord},
    StoreResult,
};

const EXPECTED_WRITE_OBSERVATION_COLUMNS: &str = "
    expected_write_id, matched_paths_json, matched_at";

const UNRECORDED_CHANGE_OBSERVATION_COLUMNS: &str = "
    unrecorded_change_id, observed_paths_json, detected_at";

/// Non-authoritative observation-time candidate used only for bounded workflow metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductWriteObservationCandidate {
    pub source_table: String,
    pub source_id: String,
    pub observed_paths_json: String,
    pub observed_at: String,
}

impl CoreProjectStore<'_> {
    /// Lists unresolved Unrecorded Changes from this handle's current project
    /// snapshot without reopening the project database.
    pub fn unresolved_unrecorded_changes(
        &self,
        connection_internal_id: Option<&str>,
    ) -> StoreResult<Vec<UnrecordedChangeRecord>> {
        unresolved_unrecorded_changes_from_conn(
            &self.conn,
            &self.project.project_id,
            connection_internal_id,
        )
    }

    /// Lists confirmed, Task-bound product-write observation candidates without assigning
    /// authority or interpreting their path payloads.
    pub fn product_write_observation_candidates_for_task(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Vec<ProductWriteObservationCandidate>> {
        validate_identifier("task_id", task_id.as_str())?;
        let expected_sql = format!(
            "SELECT {EXPECTED_WRITE_OBSERVATION_COLUMNS}
               FROM expected_writes
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'matched'"
        );
        let mut candidates = Vec::new();
        let mut expected = self.conn.prepare(&expected_sql)?;
        let rows =
            expected.query_map(params![self.project.project_id, task_id.as_str()], |row| {
                Ok(ProductWriteObservationCandidate {
                    source_table: "expected_writes".to_owned(),
                    source_id: row.get(0)?,
                    observed_paths_json: row.get(1)?,
                    observed_at: row.get(2)?,
                })
            })?;
        for row in rows {
            candidates.push(row?);
        }

        let unrecorded_sql = format!(
            "SELECT {UNRECORDED_CHANGE_OBSERVATION_COLUMNS}
               FROM unrecorded_changes
              WHERE project_id = ?1
                AND task_id = ?2
                AND confidence = 'confirmed'"
        );
        let mut unrecorded = self.conn.prepare(&unrecorded_sql)?;
        let rows =
            unrecorded.query_map(params![self.project.project_id, task_id.as_str()], |row| {
                Ok(ProductWriteObservationCandidate {
                    source_table: "unrecorded_changes".to_owned(),
                    source_id: row.get(0)?,
                    observed_paths_json: row.get(1)?,
                    observed_at: row.get(2)?,
                })
            })?;
        for row in rows {
            candidates.push(row?);
        }
        Ok(candidates)
    }
}
