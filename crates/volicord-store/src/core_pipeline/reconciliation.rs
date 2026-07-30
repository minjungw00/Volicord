use rusqlite::params;
use volicord_types::ids::TaskId;
use volicord_types::product_path::ProductRelativePath;
use volicord_types::values::UtcTimestamp;

use super::{facade::CoreProjectStore, validation::validate_identifier};
use crate::{
    guards::{unresolved_unrecorded_changes_from_conn, UnrecordedChangeRecord},
    StoreError, StoreResult,
};

const EXPECTED_WRITE_OBSERVATION_COLUMNS: &str = "
    expected_write_id, matched_paths_json, matched_at";

const UNRECORDED_CHANGE_OBSERVATION_COLUMNS: &str = "
    unrecorded_change_id, observed_paths_json, detected_at";

/// Non-authoritative observation-time candidate used only for bounded workflow metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductWriteObservationSource {
    ExpectedWrite,
    UnrecordedChange,
}

/// Non-authoritative observation-time candidate used only for bounded workflow metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductWriteObservationCandidate {
    pub source: ProductWriteObservationSource,
    pub source_id: String,
    pub observed_paths: Vec<ProductRelativePath>,
    pub observed_at: UtcTimestamp,
}

#[derive(Debug)]
struct ProductWriteObservationRaw {
    source: ProductWriteObservationSource,
    source_id: String,
    observed_paths_json: String,
    observed_at: String,
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

    /// Lists strictly decoded, Task-bound product-write metric candidates without
    /// assigning authority or reinterpreting their path payloads.
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
                Ok(ProductWriteObservationRaw {
                    source: ProductWriteObservationSource::ExpectedWrite,
                    source_id: row.get(0)?,
                    observed_paths_json: row.get(1)?,
                    observed_at: row.get(2)?,
                })
            })?;
        for row in rows {
            candidates.push(decode_product_write_observation(row?)?);
        }

        let unrecorded_sql = format!(
            "SELECT {UNRECORDED_CHANGE_OBSERVATION_COLUMNS}
               FROM unrecorded_changes
              WHERE project_id = ?1
                AND task_id = ?2"
        );
        let mut unrecorded = self.conn.prepare(&unrecorded_sql)?;
        let rows =
            unrecorded.query_map(params![self.project.project_id, task_id.as_str()], |row| {
                Ok(ProductWriteObservationRaw {
                    source: ProductWriteObservationSource::UnrecordedChange,
                    source_id: row.get(0)?,
                    observed_paths_json: row.get(1)?,
                    observed_at: row.get(2)?,
                })
            })?;
        for row in rows {
            candidates.push(decode_product_write_observation(row?)?);
        }
        Ok(candidates)
    }
}

fn decode_product_write_observation(
    raw: ProductWriteObservationRaw,
) -> StoreResult<ProductWriteObservationCandidate> {
    let table = match raw.source {
        ProductWriteObservationSource::ExpectedWrite => "expected_writes",
        ProductWriteObservationSource::UnrecordedChange => "unrecorded_changes",
    };
    let paths_column = match raw.source {
        ProductWriteObservationSource::ExpectedWrite => "matched_paths_json",
        ProductWriteObservationSource::UnrecordedChange => "observed_paths_json",
    };
    let time_column = match raw.source {
        ProductWriteObservationSource::ExpectedWrite => "matched_at",
        ProductWriteObservationSource::UnrecordedChange => "detected_at",
    };
    let observed_paths = serde_json::from_str::<Vec<ProductRelativePath>>(&raw.observed_paths_json)
        .map_err(|_| {
            StoreError::corrupt_owner_state_json(table, raw.source_id.clone(), paths_column)
        })?;
    if observed_paths.is_empty()
        || observed_paths
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != observed_paths.len()
    {
        return Err(StoreError::corrupt_owner_state_value(
            table,
            raw.source_id,
            paths_column,
        ));
    }
    let observed_at = UtcTimestamp::parse(&raw.observed_at).map_err(|_| {
        StoreError::corrupt_owner_state_value(table, raw.source_id.clone(), time_column)
    })?;
    observed_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| {
            StoreError::corrupt_owner_state_value(table, raw.source_id.clone(), time_column)
        })?;
    Ok(ProductWriteObservationCandidate {
        source: raw.source,
        source_id: raw.source_id,
        observed_paths,
        observed_at,
    })
}
