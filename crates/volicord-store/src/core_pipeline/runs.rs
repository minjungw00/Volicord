use std::collections::BTreeMap;

use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::{ids::TaskId, schema::ObservedChanges};

use super::{
    facade::CoreProjectStore,
    validation::{decode_owner_json_text, validate_identifier},
};
use crate::{StoreError, StoreResult};

const RUN_RECORD_COLUMNS: &str = "
    project_id, run_id, task_id, change_unit_id, scope_revision,
    observed_changes_json, status";

const RUN_OBSERVED_CHANGES_COLUMNS: &str = "
    rowid, project_id, run_id, task_id, change_unit_id,
    observed_changes_json, status";

/// Stored Run facts needed when resolving close-basis references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRecord {
    pub project_id: String,
    pub run_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub scope_revision: u64,
    pub baseline_ref: Option<String>,
    pub status: String,
}

/// Stored Run observed-change facts needed by reconciliation checks.
#[derive(Debug, Clone, PartialEq)]
pub struct RunObservedChangesRecord {
    pub project_id: String,
    pub run_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub observed_changes: ObservedChanges,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawRunRecord {
    project_id: String,
    run_id: String,
    task_id: String,
    change_unit_id: Option<String>,
    scope_revision: i64,
    observed_changes_json: String,
    status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawRunObservedChangesRecord {
    rowid: i64,
    project_id: String,
    run_id: String,
    task_id: String,
    change_unit_id: Option<String>,
    observed_changes_json: String,
    status: String,
}

impl CoreProjectStore<'_> {
    /// Returns whether a Run id already exists in this project.
    pub fn run_id_exists(&self, run_id: &str) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT COUNT(*)
                   FROM runs
                  WHERE project_id = ?1
                    AND run_id = ?2",
                params![self.project.project_id, run_id],
                |row| Ok(row.get::<_, i64>(0)? > 0),
            )
            .map_err(StoreError::from)
    }

    /// Returns whether a Run belongs to a Task in this project.
    pub fn run_belongs_to_task(&self, run_id: &str, task_id: &str) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT COUNT(*)
                   FROM runs
                  WHERE project_id = ?1
                    AND run_id = ?2
                    AND task_id = ?3",
                params![self.project.project_id, run_id, task_id],
                |row| Ok(row.get::<_, i64>(0)? > 0),
            )
            .map_err(StoreError::from)
    }

    /// Reads one committed Run row by exact project-local identity.
    pub fn run_record(&self, run_id: &str) -> StoreResult<Option<RunRecord>> {
        run_record(&self.conn, &self.project.project_id, run_id)
    }

    /// Lists committed Run rows for one Task with their observed changes.
    pub fn run_observed_changes_for_task(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Vec<RunObservedChangesRecord>> {
        run_observed_changes_for_task(&self.conn, &self.project.project_id, task_id.as_str())
    }
}

fn run_record(conn: &Connection, project_id: &str, run_id: &str) -> StoreResult<Option<RunRecord>> {
    let sql = format!(
        "SELECT {RUN_RECORD_COLUMNS}
           FROM runs
          WHERE project_id = ?1
            AND run_id = ?2"
    );
    conn.query_row(&sql, params![project_id, run_id], raw_run_record_from_row)
        .optional()?
        .map(decode_run_record)
        .transpose()
}

fn raw_run_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawRunRecord> {
    Ok(RawRunRecord {
        project_id: row.get(0)?,
        run_id: row.get(1)?,
        task_id: row.get(2)?,
        change_unit_id: row.get(3)?,
        scope_revision: row.get(4)?,
        observed_changes_json: row.get(5)?,
        status: row.get(6)?,
    })
}

fn decode_run_record(raw: RawRunRecord) -> StoreResult<RunRecord> {
    let scope_revision = u64::try_from(raw.scope_revision).map_err(|_| {
        StoreError::corrupt_owner_state_value("runs", raw.run_id.clone(), "scope_revision")
    })?;
    let observed_changes = decode_owner_json_text::<ObservedChanges>(
        "runs",
        raw.run_id.clone(),
        "observed_changes_json",
        &raw.observed_changes_json,
    )?;
    Ok(RunRecord {
        project_id: raw.project_id,
        run_id: raw.run_id,
        task_id: raw.task_id,
        change_unit_id: raw.change_unit_id,
        scope_revision,
        baseline_ref: observed_changes
            .baseline_ref
            .as_ref()
            .map(|baseline_ref| baseline_ref.as_str().to_owned()),
        status: raw.status,
    })
}

fn run_observed_changes_for_task(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<RunObservedChangesRecord>> {
    validate_identifier("task_id", task_id)?;
    let sql = format!(
        "SELECT {RUN_OBSERVED_CHANGES_COLUMNS}
           FROM runs
          WHERE project_id = ?1
            AND task_id = ?2
          ORDER BY rowid DESC"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(
        params![project_id, task_id],
        raw_run_observed_changes_record_from_row,
    )?;
    let mut records = Vec::new();
    for row in rows {
        let raw = row?;
        let observed_changes = decode_owner_json_text::<ObservedChanges>(
            "runs",
            raw.run_id.clone(),
            "observed_changes_json",
            &raw.observed_changes_json,
        )?;
        records.push((
            raw.rowid,
            RunObservedChangesRecord {
                project_id: raw.project_id,
                run_id: raw.run_id,
                task_id: raw.task_id,
                change_unit_id: raw.change_unit_id,
                observed_changes,
                status: raw.status,
            },
        ));
    }

    let mut event_statement = conn.prepare(
        "SELECT event_seq, payload_json
           FROM authority_events
          WHERE project_id = ?1
            AND task_id = ?2
            AND event_type = 'run_recorded'
          ORDER BY event_seq DESC",
    )?;
    let event_rows = event_statement.query_map(params![project_id, task_id], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut event_order = BTreeMap::new();
    for row in event_rows {
        let (event_seq, payload_json) = row?;
        let payload = decode_owner_json_text::<serde_json::Value>(
            "authority_events",
            format!("event_seq:{event_seq}"),
            "payload_json",
            &payload_json,
        )?;
        let run_id = payload
            .get("run_id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                StoreError::corrupt_owner_state_value(
                    "authority_events",
                    format!("event_seq:{event_seq}"),
                    "payload_json.run_id",
                )
            })?;
        event_order.entry(run_id.to_owned()).or_insert(event_seq);
    }

    let mut ordered_records = records
        .into_iter()
        .map(|(rowid, record)| {
            let event_seq = event_order.get(&record.run_id).copied().ok_or_else(|| {
                StoreError::corrupt_owner_state_value(
                    "runs",
                    record.run_id.clone(),
                    "authority_events.run_recorded",
                )
            })?;
            Ok((event_seq, rowid, record))
        })
        .collect::<StoreResult<Vec<_>>>()?;
    ordered_records.sort_by(
        |(left_event_seq, left_rowid, _), (right_event_seq, right_rowid, _)| {
            right_event_seq
                .cmp(left_event_seq)
                .then_with(|| right_rowid.cmp(left_rowid))
        },
    );
    Ok(ordered_records
        .into_iter()
        .map(|(_, _, record)| record)
        .collect())
}

fn raw_run_observed_changes_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RawRunObservedChangesRecord> {
    Ok(RawRunObservedChangesRecord {
        rowid: row.get(0)?,
        project_id: row.get(1)?,
        run_id: row.get(2)?,
        task_id: row.get(3)?,
        change_unit_id: row.get(4)?,
        observed_changes_json: row.get(5)?,
        status: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_decoder_rejects_malformed_observed_changes_json() {
        let error = decode_run_record(RawRunRecord {
            project_id: "project".to_owned(),
            run_id: "run".to_owned(),
            task_id: "task".to_owned(),
            change_unit_id: None,
            scope_revision: 0,
            observed_changes_json: "{".to_owned(),
            status: "passed".to_owned(),
        })
        .expect_err("malformed observed changes must fail closed");

        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateJson {
                table: "runs",
                logical_column: "observed_changes_json",
                ..
            }
        ));
    }
}
