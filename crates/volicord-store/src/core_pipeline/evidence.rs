use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::ids::TaskId;

use super::{
    facade::CoreProjectStore, record_refs::StoredRecordRef, validation::nonnegative_i64_to_u64,
};
use crate::{StoreError, StoreResult};

const EVIDENCE_SUMMARY_COLUMNS: &str = "
    project_id, evidence_summary_id, task_id, change_unit_id,
    produced_at_state_version, status, coverage_json, supporting_refs_json,
    gap_refs_json, metadata_json";

const EVIDENCE_OBSERVATION_COLUMNS: &str = "
    project_id, evidence_observation_id, task_id, change_unit_id, run_id,
    acceptance_criterion_id, evidence_claim_id, source_kind, assurance_level,
    observed_by_actor_source, tool_name, tool_invocation_id, tool_metadata_json,
    input_refs_json, source_refs_json, output_artifact_refs_json,
    limitations_json, observed_at, recorded_at, metadata_json";

/// Stored evidence summary facts needed by close-readiness evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSummaryRecord {
    pub project_id: String,
    pub evidence_summary_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub produced_at_state_version: u64,
    pub status: String,
    pub coverage_json: String,
    pub supporting_refs_json: String,
    pub gap_refs_json: String,
    pub metadata_json: String,
}

/// Stored evidence observation facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceObservationRecord {
    pub project_id: String,
    pub evidence_observation_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub run_id: Option<String>,
    pub acceptance_criterion_id: Option<String>,
    pub evidence_claim_id: Option<String>,
    pub source_kind: String,
    pub assurance_level: String,
    pub observed_by_actor_source: Option<String>,
    pub tool_name: Option<String>,
    pub tool_invocation_id: Option<String>,
    pub tool_metadata_json: String,
    pub input_refs_json: String,
    pub source_refs_json: String,
    pub output_artifact_refs_json: String,
    pub limitations_json: String,
    pub observed_at: String,
    pub recorded_at: String,
    pub metadata_json: String,
}

impl CoreProjectStore<'_> {
    /// Returns whether an evidence summary id already exists in this project.
    pub fn evidence_summary_exists(&self, evidence_summary_id: &str) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT COUNT(*)
                   FROM evidence_summaries
                  WHERE project_id = ?1
                    AND evidence_summary_id = ?2",
                params![self.project.project_id, evidence_summary_id],
                |row| Ok(row.get::<_, i64>(0)? > 0),
            )
            .map_err(StoreError::from)
    }

    /// Returns whether an evidence observation id already exists in this project.
    pub fn evidence_observation_exists(&self, evidence_observation_id: &str) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT COUNT(*)
                   FROM evidence_observations
                  WHERE project_id = ?1
                    AND evidence_observation_id = ?2",
                params![self.project.project_id, evidence_observation_id],
                |row| Ok(row.get::<_, i64>(0)? > 0),
            )
            .map_err(StoreError::from)
    }

    /// Reads one evidence observation row by exact project-local observation identity.
    pub fn evidence_observation_record(
        &self,
        evidence_observation_id: &str,
    ) -> StoreResult<Option<EvidenceObservationRecord>> {
        evidence_observation_record(
            &self.conn,
            &self.project.project_id,
            evidence_observation_id,
        )
    }

    /// Lists evidence observation refs created by a committed Run.
    pub fn evidence_observation_refs_for_run(
        &self,
        task_id: &TaskId,
        run_id: &str,
        state_version: u64,
    ) -> StoreResult<Vec<StoredRecordRef>> {
        evidence_observation_refs_for_run(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            run_id,
            state_version,
        )
    }

    /// Reads the latest evidence summary row for a Task, when one exists.
    pub fn latest_evidence_summary(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Option<EvidenceSummaryRecord>> {
        latest_evidence_summary(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Reads one evidence summary row by exact project-local evidence identity.
    pub fn evidence_summary_record(
        &self,
        evidence_summary_id: &str,
    ) -> StoreResult<Option<EvidenceSummaryRecord>> {
        evidence_summary_record(&self.conn, &self.project.project_id, evidence_summary_id)
    }
}

fn evidence_observation_refs_for_run(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    run_id: &str,
    state_version: u64,
) -> StoreResult<Vec<StoredRecordRef>> {
    let mut statement = conn.prepare(
        "SELECT evidence_observation_id
           FROM evidence_observations
          WHERE project_id = ?1
            AND task_id = ?2
            AND run_id = ?3
          ORDER BY evidence_observation_id",
    )?;
    let rows = statement.query_map(params![project_id, task_id, run_id], |row| {
        Ok(StoredRecordRef {
            record_kind: "evidence_observation".to_owned(),
            record_id: row.get(0)?,
            project_id: project_id.to_owned(),
            task_id: Some(task_id.to_owned()),
            state_version: Some(state_version),
        })
    })?;
    let mut refs = Vec::new();
    for row in rows {
        refs.push(row?);
    }
    Ok(refs)
}

fn latest_evidence_summary(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Option<EvidenceSummaryRecord>> {
    let sql = format!(
        "SELECT {EVIDENCE_SUMMARY_COLUMNS}
           FROM evidence_summaries
          WHERE project_id = ?1
            AND task_id = ?2
          ORDER BY produced_at_state_version DESC
          LIMIT 1"
    );
    let record = conn
        .query_row(
            &sql,
            params![project_id, task_id],
            evidence_summary_record_from_row,
        )
        .optional()?;
    validate_evidence_summary_state_version(conn, project_id, record)
}

fn evidence_summary_record(
    conn: &Connection,
    project_id: &str,
    evidence_summary_id: &str,
) -> StoreResult<Option<EvidenceSummaryRecord>> {
    let sql = format!(
        "SELECT {EVIDENCE_SUMMARY_COLUMNS}
           FROM evidence_summaries
          WHERE project_id = ?1
            AND evidence_summary_id = ?2"
    );
    let record = conn
        .query_row(
            &sql,
            params![project_id, evidence_summary_id],
            evidence_summary_record_from_row,
        )
        .optional()?;
    validate_evidence_summary_state_version(conn, project_id, record)
}

fn validate_evidence_summary_state_version(
    conn: &Connection,
    project_id: &str,
    record: Option<EvidenceSummaryRecord>,
) -> StoreResult<Option<EvidenceSummaryRecord>> {
    let Some(record) = record else {
        return Ok(None);
    };
    let current_state_version = conn
        .query_row(
            "SELECT state_version FROM project_state WHERE project_id = ?1",
            [project_id],
            |row| nonnegative_i64_to_u64("project_state.state_version", row.get(0)?),
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project_state",
            id: project_id.to_owned(),
        })?;
    if record.produced_at_state_version > current_state_version {
        return Err(StoreError::corrupt_owner_state_value(
            "evidence_summaries",
            &record.evidence_summary_id,
            "produced_at_state_version",
        ));
    }
    Ok(Some(record))
}

fn evidence_summary_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<EvidenceSummaryRecord> {
    Ok(EvidenceSummaryRecord {
        project_id: row.get(0)?,
        evidence_summary_id: row.get(1)?,
        task_id: row.get(2)?,
        change_unit_id: row.get(3)?,
        produced_at_state_version: nonnegative_i64_to_u64(
            "evidence_summaries.produced_at_state_version",
            row.get(4)?,
        )?,
        status: row.get(5)?,
        coverage_json: row.get(6)?,
        supporting_refs_json: row.get(7)?,
        gap_refs_json: row.get(8)?,
        metadata_json: row.get(9)?,
    })
}

fn evidence_observation_record(
    conn: &Connection,
    project_id: &str,
    evidence_observation_id: &str,
) -> StoreResult<Option<EvidenceObservationRecord>> {
    let sql = format!(
        "SELECT {EVIDENCE_OBSERVATION_COLUMNS}
           FROM evidence_observations
          WHERE project_id = ?1
            AND evidence_observation_id = ?2"
    );
    conn.query_row(
        &sql,
        params![project_id, evidence_observation_id],
        evidence_observation_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn evidence_observation_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<EvidenceObservationRecord> {
    Ok(EvidenceObservationRecord {
        project_id: row.get(0)?,
        evidence_observation_id: row.get(1)?,
        task_id: row.get(2)?,
        change_unit_id: row.get(3)?,
        run_id: row.get(4)?,
        acceptance_criterion_id: row.get(5)?,
        evidence_claim_id: row.get(6)?,
        source_kind: row.get(7)?,
        assurance_level: row.get(8)?,
        observed_by_actor_source: row.get(9)?,
        tool_name: row.get(10)?,
        tool_invocation_id: row.get(11)?,
        tool_metadata_json: row.get(12)?,
        input_refs_json: row.get(13)?,
        source_refs_json: row.get(14)?,
        output_artifact_refs_json: row.get(15)?,
        limitations_json: row.get(16)?,
        observed_at: row.get(17)?,
        recorded_at: row.get(18)?,
        metadata_json: row.get(19)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_summary_decoder_rejects_a_negative_state_version() {
        let connection = Connection::open_in_memory().expect("in-memory database must open");
        let error = connection
            .query_row(
                "SELECT 'project', 'summary', 'task', NULL, -1, 'current',
                        '{}', '[]', '[]', '{}'",
                [],
                evidence_summary_record_from_row,
            )
            .expect_err("negative authority order must fail closed");

        assert!(matches!(
            error,
            rusqlite::Error::FromSqlConversionFailure(..)
        ));
    }
}
