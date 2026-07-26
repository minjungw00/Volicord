use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::{
    ids::TaskId,
    schema::CurrentCloseBasis,
    values::{PersistedCloseSummary, UtcTimestamp},
};

use super::{
    facade::CoreProjectStore,
    validation::{decode_current_close_basis_column, nonnegative_i64_to_u64, validate_identifier},
};
use crate::{StoreError, StoreResult};

const TASK_RECORD_COLUMNS: &str = "
    project_id, task_id, mode, requested_control_level,
    effective_control_level, control_level_reason, work_phase, acceptance_policy,
    acceptance_policy_reason, predecessor_task_id, lineage_relation,
    lineage_reason, carry_forward_json, lifecycle_phase, result, title,
    summary, shaping_summary_json, bounded_context_json,
    autonomy_boundary_json, scope_revision, close_basis_revision,
    close_basis_json, close_summary_json, current_change_unit_id, closed_at,
    metadata_json";

const ACCEPTANCE_CRITERION_COLUMNS: &str = "
    project_id, acceptance_criterion_id, task_id, statement,
    evidence_requirement, position, status";

const EVIDENCE_CLAIM_COLUMNS: &str = "
    project_id, evidence_claim_id, task_id, statement";

const TASK_REVISION_COLUMNS: &str = "
    project_id, task_id, scope_revision, close_basis_revision, close_basis_json";

/// Current Task row data needed by Core method implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    pub project_id: String,
    pub task_id: String,
    pub mode: String,
    pub requested_control_level: String,
    pub effective_control_level: String,
    pub control_level_reason: String,
    pub work_phase: String,
    pub acceptance_policy: String,
    pub acceptance_policy_reason: String,
    pub predecessor_task_id: Option<String>,
    pub lineage_relation: Option<String>,
    pub lineage_reason: Option<String>,
    pub carry_forward_json: String,
    pub lifecycle_phase: String,
    pub result: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub shaping_summary_json: String,
    pub bounded_context_json: String,
    pub autonomy_boundary_json: String,
    pub scope_revision: u64,
    pub close_basis_revision: u64,
    pub close_basis_json: Option<String>,
    pub close_summary_json: String,
    pub current_change_unit_id: Option<String>,
    pub closed_at: Option<String>,
    pub metadata_json: String,
}

/// Canonical acceptance criterion row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceCriterionRecord {
    pub project_id: String,
    pub acceptance_criterion_id: String,
    pub task_id: String,
    pub statement: String,
    pub evidence_requirement: String,
    pub position: u64,
    pub status: String,
}

/// Canonical Task-scoped supplemental evidence claim row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceClaimRecord {
    pub project_id: String,
    pub evidence_claim_id: String,
    pub task_id: String,
    pub statement: String,
}

/// Current Task revision coordinates and optional strict-decoded close basis.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskRevisionRecord {
    pub project_id: String,
    pub task_id: String,
    pub scope_revision: u64,
    pub close_basis_revision: u64,
    pub close_basis_json: Option<String>,
    pub current_close_basis: Option<CurrentCloseBasis>,
}

impl CoreProjectStore<'_> {
    /// Returns whether a Task exists in this project.
    pub fn task_exists(&self, task_id: &TaskId) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT COUNT(*)
                   FROM tasks
                  WHERE project_id = ?1
                    AND task_id = ?2",
                params![self.project.project_id, task_id.as_str()],
                |row| Ok(row.get::<_, i64>(0)? > 0),
            )
            .map_err(StoreError::from)
    }

    /// Reads one Task current row.
    pub fn task_record(&self, task_id: &TaskId) -> StoreResult<Option<TaskRecord>> {
        task_record(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Reads the immutable canonical creation time for one Task.
    pub fn task_created_at(&self, task_id: &TaskId) -> StoreResult<Option<UtcTimestamp>> {
        validate_identifier("task_id", task_id.as_str())?;
        let raw = self
            .conn
            .query_row(
                "SELECT created_at
                   FROM tasks
                  WHERE project_id = ?1
                    AND task_id = ?2",
                params![self.project.project_id, task_id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        raw.map(|value| {
            UtcTimestamp::parse(&value).map_err(|_| {
                StoreError::corrupt_owner_state_value("tasks", task_id.as_str(), "created_at")
            })
        })
        .transpose()
    }

    /// Lists every Task row for lineage-flow projection.
    pub fn task_records(&self) -> StoreResult<Vec<TaskRecord>> {
        task_records(&self.conn, &self.project.project_id)
    }

    /// Reads Task revision coordinates and the current close basis, when present.
    pub fn task_revision_record(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Option<TaskRevisionRecord>> {
        task_revision_record(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Reads the current active Task row, when `project_state.active_task_id` is set.
    pub fn active_task_record(&self) -> StoreResult<Option<TaskRecord>> {
        let state = self.project_state()?;
        match state.active_task_id {
            Some(task_id) => task_record(&self.conn, &self.project.project_id, &task_id),
            None => Ok(None),
        }
    }

    /// Lists the current canonical acceptance criteria for one Task.
    pub fn active_acceptance_criteria(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Vec<AcceptanceCriterionRecord>> {
        active_acceptance_criteria(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Reads an acceptance criterion by project-local identity, including retired rows.
    pub fn acceptance_criterion_record(
        &self,
        acceptance_criterion_id: &str,
    ) -> StoreResult<Option<AcceptanceCriterionRecord>> {
        acceptance_criterion_record(
            &self.conn,
            &self.project.project_id,
            acceptance_criterion_id,
        )
    }

    /// Returns whether an acceptance-criterion id exists in this project.
    pub fn acceptance_criterion_id_exists(
        &self,
        acceptance_criterion_id: &str,
    ) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT COUNT(*)
                   FROM acceptance_criteria
                  WHERE project_id = ?1
                    AND acceptance_criterion_id = ?2",
                params![self.project.project_id, acceptance_criterion_id],
                |row| Ok(row.get::<_, i64>(0)? > 0),
            )
            .map_err(StoreError::from)
    }

    /// Reads a Task-scoped supplemental evidence claim by project-local identity.
    pub fn evidence_claim_record(
        &self,
        task_id: &TaskId,
        evidence_claim_id: &str,
    ) -> StoreResult<Option<EvidenceClaimRecord>> {
        evidence_claim_record(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            evidence_claim_id,
        )
    }
}

fn task_record(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Option<TaskRecord>> {
    let sql = format!(
        "SELECT {TASK_RECORD_COLUMNS}
           FROM tasks
          WHERE project_id = ?1
            AND task_id = ?2"
    );
    conn.query_row(&sql, params![project_id, task_id], task_record_from_row)
        .optional()
        .map_err(StoreError::from)?
        .map(validate_decoded_task_record)
        .transpose()
}

fn task_records(conn: &Connection, project_id: &str) -> StoreResult<Vec<TaskRecord>> {
    let sql = format!(
        "SELECT {TASK_RECORD_COLUMNS}
           FROM tasks
          WHERE project_id = ?1
          ORDER BY volicord_utc_seconds(created_at),
                   volicord_utc_subsec_nanos(created_at),
                   task_id"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map([project_id], task_record_from_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StoreError::from)?
        .into_iter()
        .map(validate_decoded_task_record)
        .collect()
}

fn task_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRecord> {
    Ok(TaskRecord {
        project_id: row.get(0)?,
        task_id: row.get(1)?,
        mode: row.get(2)?,
        requested_control_level: row.get(3)?,
        effective_control_level: row.get(4)?,
        control_level_reason: row.get(5)?,
        work_phase: row.get(6)?,
        acceptance_policy: row.get(7)?,
        acceptance_policy_reason: row.get(8)?,
        predecessor_task_id: row.get(9)?,
        lineage_relation: row.get(10)?,
        lineage_reason: row.get(11)?,
        carry_forward_json: row.get(12)?,
        lifecycle_phase: row.get(13)?,
        result: row.get(14)?,
        title: row.get(15)?,
        summary: row.get(16)?,
        shaping_summary_json: row.get(17)?,
        bounded_context_json: row.get(18)?,
        autonomy_boundary_json: row.get(19)?,
        scope_revision: nonnegative_i64_to_u64("tasks.scope_revision", row.get(20)?)?,
        close_basis_revision: nonnegative_i64_to_u64("tasks.close_basis_revision", row.get(21)?)?,
        close_basis_json: row.get(22)?,
        close_summary_json: row.get(23)?,
        current_change_unit_id: row.get(24)?,
        closed_at: row.get(25)?,
        metadata_json: row.get(26)?,
    })
}

fn validate_decoded_task_record(record: TaskRecord) -> StoreResult<TaskRecord> {
    serde_json::from_str::<PersistedCloseSummary>(&record.close_summary_json).map_err(|_| {
        StoreError::corrupt_owner_state_json("tasks", record.task_id.clone(), "close_summary_json")
    })?;
    Ok(record)
}

fn active_acceptance_criteria(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<AcceptanceCriterionRecord>> {
    let sql = format!(
        "SELECT {ACCEPTANCE_CRITERION_COLUMNS}
           FROM acceptance_criteria
          WHERE project_id = ?1
            AND task_id = ?2
            AND status = 'active'
          ORDER BY position, acceptance_criterion_id"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(
        params![project_id, task_id],
        acceptance_criterion_record_from_row,
    )?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StoreError::from)
}

fn acceptance_criterion_record(
    conn: &Connection,
    project_id: &str,
    acceptance_criterion_id: &str,
) -> StoreResult<Option<AcceptanceCriterionRecord>> {
    let sql = format!(
        "SELECT {ACCEPTANCE_CRITERION_COLUMNS}
           FROM acceptance_criteria
          WHERE project_id = ?1
            AND acceptance_criterion_id = ?2"
    );
    conn.query_row(
        &sql,
        params![project_id, acceptance_criterion_id],
        acceptance_criterion_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn acceptance_criterion_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<AcceptanceCriterionRecord> {
    Ok(AcceptanceCriterionRecord {
        project_id: row.get(0)?,
        acceptance_criterion_id: row.get(1)?,
        task_id: row.get(2)?,
        statement: row.get(3)?,
        evidence_requirement: row.get(4)?,
        position: nonnegative_i64_to_u64("acceptance_criteria.position", row.get(5)?)?,
        status: row.get(6)?,
    })
}

fn evidence_claim_record(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    evidence_claim_id: &str,
) -> StoreResult<Option<EvidenceClaimRecord>> {
    let sql = format!(
        "SELECT {EVIDENCE_CLAIM_COLUMNS}
           FROM evidence_claims
          WHERE project_id = ?1
            AND task_id = ?2
            AND evidence_claim_id = ?3"
    );
    conn.query_row(
        &sql,
        params![project_id, task_id, evidence_claim_id],
        |row| {
            Ok(EvidenceClaimRecord {
                project_id: row.get(0)?,
                evidence_claim_id: row.get(1)?,
                task_id: row.get(2)?,
                statement: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

fn task_revision_record(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Option<TaskRevisionRecord>> {
    let sql = format!(
        "SELECT {TASK_REVISION_COLUMNS}
           FROM tasks
          WHERE project_id = ?1
            AND task_id = ?2"
    );
    let row = conn
        .query_row(&sql, params![project_id, task_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .optional()?;

    let Some((project_id, task_id, scope_revision, close_basis_revision, close_basis_json)) = row
    else {
        return Ok(None);
    };
    let current_close_basis =
        decode_current_close_basis_column(&task_id, close_basis_json.as_deref())?;

    Ok(Some(TaskRevisionRecord {
        project_id,
        task_id,
        scope_revision: nonnegative_i64_to_u64("tasks.scope_revision", scope_revision)
            .map_err(StoreError::from)?,
        close_basis_revision: nonnegative_i64_to_u64(
            "tasks.close_basis_revision",
            close_basis_revision,
        )
        .map_err(StoreError::from)?,
        close_basis_json,
        current_close_basis,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_decoder_rejects_a_close_summary_without_an_explicit_reason() {
        let record = TaskRecord {
            project_id: "project".to_owned(),
            task_id: "task".to_owned(),
            mode: "normal".to_owned(),
            requested_control_level: "light".to_owned(),
            effective_control_level: "light".to_owned(),
            control_level_reason: "reason".to_owned(),
            work_phase: "implementation".to_owned(),
            acceptance_policy: "not_required".to_owned(),
            acceptance_policy_reason: "reason".to_owned(),
            predecessor_task_id: None,
            lineage_relation: None,
            lineage_reason: None,
            carry_forward_json: "{}".to_owned(),
            lifecycle_phase: "active".to_owned(),
            result: None,
            title: None,
            summary: None,
            shaping_summary_json: "{}".to_owned(),
            bounded_context_json: "{}".to_owned(),
            autonomy_boundary_json: "{}".to_owned(),
            scope_revision: 0,
            close_basis_revision: 0,
            close_basis_json: None,
            close_summary_json: r#"{"result":"cancelled","closed_at":null}"#.to_owned(),
            current_change_unit_id: None,
            closed_at: None,
            metadata_json: "{}".to_owned(),
        };

        assert!(matches!(
            validate_decoded_task_record(record),
            Err(StoreError::CorruptOwnerStateJson { .. })
        ));
    }
}
