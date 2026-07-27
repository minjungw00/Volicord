use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::{
    ids::TaskId,
    schema::CurrentCloseBasis,
    values::{PersistedCloseSummary, UtcTimestamp},
};

use super::{facade::CoreProjectStore, mutations::MutationContext, validation::*};
use crate::{workflow_records::clear_satisfied_task_policy_reevaluation, StoreError, StoreResult};

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

/// Task and acceptance mutation applied inside one Core commit transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskMutation {
    Insert(Box<TaskInsert>),
    SetActive { task_id: String },
    Supersede { task_id: String },
    Close(TaskCloseUpdate),
    UpdateControlLevel(TaskControlLevelUpdate),
    UpdateScope(TaskScopeUpdate),
    UpdateScopeRevision(TaskScopeRevisionUpdate),
    UpdateCloseBasis(TaskCloseBasisUpdate),
    ReplaceAcceptanceCriteria(AcceptanceCriteriaReplace),
}

/// Storage input for inserting a Task current row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInsert {
    pub task_id: String,
    pub created_by_actor_source: String,
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
    pub close_summary_json: String,
    pub current_change_unit_id: Option<String>,
}

/// Storage input for updating Task scope-shaped current fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskScopeUpdate {
    pub task_id: String,
    pub work_phase: Option<String>,
    pub lifecycle_phase: Option<String>,
    pub result: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub shaping_summary_json: Option<String>,
    pub bounded_context_json: Option<String>,
    pub autonomy_boundary_json: Option<String>,
    pub close_summary_json: Option<String>,
}

/// Storage input for an upward-only Task control transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskControlLevelUpdate {
    pub task_id: String,
    pub effective_control_level: String,
    pub control_level_reason: String,
    pub acceptance_policy: Option<String>,
    pub acceptance_policy_reason: Option<String>,
}

/// One canonical acceptance criterion in a complete Task replacement set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceCriterionUpsert {
    pub acceptance_criterion_id: String,
    pub statement: String,
    pub evidence_requirement: String,
    pub position: u64,
}

/// Storage input for atomically replacing the current Task criterion set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceCriteriaReplace {
    pub task_id: String,
    pub criteria: Vec<AcceptanceCriterionUpsert>,
}

/// Storage input for updating a Task scope revision coordinate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskScopeRevisionUpdate {
    pub task_id: String,
    pub scope_revision: u64,
}

/// Storage input for atomically replacing a Task close-basis coordinate and JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCloseBasisUpdate {
    pub task_id: String,
    pub close_basis_revision: u64,
    pub close_basis_json: Option<String>,
}

/// Storage input for applying one terminal Task close transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCloseUpdate {
    pub task_id: String,
    pub lifecycle_phase: String,
    pub result: String,
    pub close_summary_json: String,
    pub closed_at: String,
}

impl TaskMutation {
    /// Boxes the largest Task mutation payload while keeping planner construction typed.
    pub fn insert(input: TaskInsert) -> Self {
        Self::Insert(Box::new(input))
    }

    pub(super) fn apply(&self, context: &mut MutationContext<'_>) -> StoreResult<()> {
        match self {
            Self::Insert(input) => context.insert_task(input),
            Self::SetActive { task_id } => context.set_active_task(task_id),
            Self::Supersede { task_id } => context.supersede_task(task_id),
            Self::Close(input) => context.close_task(input),
            Self::UpdateControlLevel(input) => context.update_task_control_level(input),
            Self::UpdateScope(input) => context.update_task_scope(input),
            Self::UpdateScopeRevision(input) => context.update_task_scope_revision(input),
            Self::UpdateCloseBasis(input) => context.update_task_close_basis(input),
            Self::ReplaceAcceptanceCriteria(input) => context.replace_acceptance_criteria(input),
        }
    }
}

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

fn task_control_level_rank(value: &str) -> StoreResult<u8> {
    match value {
        "observe" => Ok(0),
        "light" => Ok(1),
        "tracked" => Ok(2),
        "sensitive" => Ok(3),
        _ => Err(StoreError::InvalidInput {
            detail: "effective_control_level is not supported".to_owned(),
        }),
    }
}

fn acceptance_policy_rank(value: &str) -> StoreResult<u8> {
    match value {
        "not_required" => Ok(0),
        "policy_dependent" => Ok(1),
        "required" => Ok(2),
        _ => Err(StoreError::InvalidInput {
            detail: "acceptance_policy is not supported".to_owned(),
        }),
    }
}

impl MutationContext<'_> {
    fn insert_task(&mut self, input: &TaskInsert) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("created_by_actor_source", &input.created_by_actor_source)?;
        validate_identifier("mode", &input.mode)?;
        if !matches!(
            input.requested_control_level.as_str(),
            "auto" | "observe" | "light" | "tracked" | "sensitive"
        ) {
            return Err(StoreError::InvalidInput {
                detail: "requested_control_level is not supported".to_owned(),
            });
        }
        if !matches!(
            input.effective_control_level.as_str(),
            "observe" | "light" | "tracked" | "sensitive"
        ) {
            return Err(StoreError::InvalidInput {
                detail: "effective_control_level is not supported".to_owned(),
            });
        }
        if input.control_level_reason.trim().is_empty() {
            return Err(StoreError::InvalidInput {
                detail: "control_level_reason must not be empty".to_owned(),
            });
        }
        validate_identifier("work_phase", &input.work_phase)?;
        validate_identifier("acceptance_policy", &input.acceptance_policy)?;
        if input.acceptance_policy_reason.trim().is_empty() {
            return Err(StoreError::schema_invariant(
                "project_state",
                "Task acceptance policy reason must not be empty",
            ));
        }
        validate_json_text("tasks.carry_forward_json", &input.carry_forward_json)?;
        validate_identifier("lifecycle_phase", &input.lifecycle_phase)?;
        validate_json_text("tasks.shaping_summary_json", &input.shaping_summary_json)?;
        validate_json_text("tasks.bounded_context_json", &input.bounded_context_json)?;
        validate_json_text(
            "tasks.autonomy_boundary_json",
            &input.autonomy_boundary_json,
        )?;
        validate_persisted_close_summary_json(
            "tasks.close_summary_json",
            &input.close_summary_json,
        )?;
        self.tx.execute(
            "INSERT INTO tasks (
                project_id,
                task_id,
                created_by_actor_source,
                mode,
                requested_control_level,
                effective_control_level,
                control_level_reason,
                work_phase,
                acceptance_policy,
                acceptance_policy_reason,
                predecessor_task_id,
                lineage_relation,
                lineage_reason,
                carry_forward_json,
                lifecycle_phase,
                result,
                title,
                summary,
                shaping_summary_json,
                bounded_context_json,
                autonomy_boundary_json,
                close_summary_json,
                current_change_unit_id,
                created_at,
                updated_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23,
                ?24,
                ?24
            )",
            params![
                self.project_id,
                input.task_id,
                input.created_by_actor_source,
                input.mode,
                input.requested_control_level,
                input.effective_control_level,
                input.control_level_reason,
                input.work_phase,
                input.acceptance_policy,
                input.acceptance_policy_reason,
                input.predecessor_task_id,
                input.lineage_relation,
                input.lineage_reason,
                input.carry_forward_json,
                input.lifecycle_phase,
                input.result,
                input.title,
                input.summary,
                input.shaping_summary_json,
                input.bounded_context_json,
                input.autonomy_boundary_json,
                input.close_summary_json,
                input.current_change_unit_id,
                self.committed_at
            ],
        )?;
        Ok(())
    }

    fn update_task_control_level(&mut self, input: &TaskControlLevelUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        let requested_rank = task_control_level_rank(&input.effective_control_level)?;
        if input.control_level_reason.trim().is_empty() {
            return Err(StoreError::InvalidInput {
                detail: "control_level_reason must not be empty".to_owned(),
            });
        }
        let acceptance_update = match (
            input.acceptance_policy.as_deref(),
            input.acceptance_policy_reason.as_deref(),
        ) {
            (None, None) => None,
            (Some(policy), Some(reason)) if !reason.trim().is_empty() => {
                Some((policy, reason, acceptance_policy_rank(policy)?))
            }
            _ => {
                return Err(StoreError::InvalidInput {
                    detail: "acceptance_policy and acceptance_policy_reason must be supplied together with a non-empty reason".to_owned(),
                })
            }
        };
        let (current_level, current_acceptance_policy, metadata_json) = self
            .tx
            .query_row(
                "SELECT effective_control_level, acceptance_policy, metadata_json
                   FROM tasks
                  WHERE project_id = ?1
                    AND task_id = ?2",
                params![self.project_id, input.task_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| StoreError::NotFound {
                entity: "task",
                id: input.task_id.clone(),
            })?;
        if requested_rank < task_control_level_rank(&current_level)? {
            return Err(StoreError::Conflict {
                entity: "task",
                id: input.task_id.clone(),
                detail: "effective Task control level cannot decrease".to_owned(),
            });
        }
        if let Some((_, _, requested_acceptance_rank)) = &acceptance_update {
            if *requested_acceptance_rank < acceptance_policy_rank(&current_acceptance_policy)? {
                return Err(StoreError::Conflict {
                    entity: "task",
                    id: input.task_id.clone(),
                    detail: "Task acceptance policy cannot decrease".to_owned(),
                });
            }
        }
        let (acceptance_policy, acceptance_policy_reason) = acceptance_update
            .map(|(policy, reason, _)| (Some(policy), Some(reason)))
            .unwrap_or((None, None));
        let metadata_json = clear_satisfied_task_policy_reevaluation(
            &metadata_json,
            &input.task_id,
            &input.effective_control_level,
            acceptance_policy.unwrap_or(&current_acceptance_policy),
        )?;
        self.tx.execute(
            "UPDATE tasks
                SET effective_control_level = ?3,
                    control_level_reason = ?4,
                    acceptance_policy = COALESCE(?5, acceptance_policy),
                    acceptance_policy_reason = COALESCE(?6, acceptance_policy_reason),
                    metadata_json = ?7,
                    updated_at = ?8
              WHERE project_id = ?1
                AND task_id = ?2",
            params![
                self.project_id,
                input.task_id,
                input.effective_control_level,
                input.control_level_reason,
                acceptance_policy,
                acceptance_policy_reason,
                metadata_json,
                self.committed_at
            ],
        )?;
        Ok(())
    }

    fn set_active_task(&mut self, task_id: &str) -> StoreResult<()> {
        validate_identifier("task_id", task_id)?;
        let changed = self.tx.execute(
            "UPDATE project_state
                SET active_task_id = ?2
              WHERE project_id = ?1",
            params![self.project_id, task_id],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "active Task update changed no rows".to_owned(),
            })
        }
    }

    fn supersede_task(&mut self, task_id: &str) -> StoreResult<()> {
        validate_identifier("task_id", task_id)?;
        self.tx.execute(
            "UPDATE tasks
                SET lifecycle_phase = 'superseded',
                    result = 'superseded',
                    close_summary_json = '{\"close_reason\":\"superseded\"}',
                    closed_at = ?3,
                    updated_at = ?3
              WHERE project_id = ?1
                AND task_id = ?2",
            params![self.project_id, task_id, self.committed_at],
        )?;
        Ok(())
    }

    fn close_task(&mut self, input: &TaskCloseUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("lifecycle_phase", &input.lifecycle_phase)?;
        validate_identifier("result", &input.result)?;
        validate_persisted_close_summary_json(
            "tasks.close_summary_json",
            &input.close_summary_json,
        )?;
        validate_timestamp("tasks.closed_at", &input.closed_at)?;

        let changed = self.tx.execute(
            "UPDATE tasks
                SET lifecycle_phase = ?3,
                    result = ?4,
                    close_summary_json = ?5,
                    closed_at = ?6,
                    updated_at = ?7
              WHERE project_id = ?1
                AND task_id = ?2",
            params![
                self.project_id,
                input.task_id,
                input.lifecycle_phase,
                input.result,
                input.close_summary_json,
                input.closed_at,
                self.committed_at
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "Task close transition changed no rows".to_owned(),
            })
        }
    }

    fn update_task_scope(&mut self, input: &TaskScopeUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        if let Some(value) = &input.shaping_summary_json {
            validate_json_text("tasks.shaping_summary_json", value)?;
            self.update_task_text_column(&input.task_id, "shaping_summary_json", value)?;
        }
        if let Some(value) = &input.bounded_context_json {
            validate_json_text("tasks.bounded_context_json", value)?;
            self.update_task_text_column(&input.task_id, "bounded_context_json", value)?;
        }
        if let Some(value) = &input.autonomy_boundary_json {
            validate_json_text("tasks.autonomy_boundary_json", value)?;
            self.update_task_text_column(&input.task_id, "autonomy_boundary_json", value)?;
        }
        if let Some(value) = &input.close_summary_json {
            validate_persisted_close_summary_json("tasks.close_summary_json", value)?;
            self.update_task_text_column(&input.task_id, "close_summary_json", value)?;
        }
        if let Some(value) = &input.lifecycle_phase {
            validate_identifier("lifecycle_phase", value)?;
            self.update_task_text_column(&input.task_id, "lifecycle_phase", value)?;
        }
        if let Some(value) = &input.work_phase {
            validate_identifier("work_phase", value)?;
            self.update_task_text_column(&input.task_id, "work_phase", value)?;
        }
        if let Some(value) = &input.result {
            validate_identifier("result", value)?;
            self.update_task_text_column(&input.task_id, "result", value)?;
        }
        if let Some(value) = &input.title {
            self.update_task_nullable_text_column(&input.task_id, "title", Some(value))?;
        }
        if let Some(value) = &input.summary {
            self.update_task_nullable_text_column(&input.task_id, "summary", Some(value))?;
        }
        Ok(())
    }

    fn replace_acceptance_criteria(
        &mut self,
        input: &AcceptanceCriteriaReplace,
    ) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        let mut ids = Vec::with_capacity(input.criteria.len());
        for criterion in &input.criteria {
            validate_identifier(
                "acceptance_criterion_id",
                &criterion.acceptance_criterion_id,
            )?;
            validate_identifier(
                "acceptance_criteria.evidence_requirement",
                &criterion.evidence_requirement,
            )?;
            if criterion.statement.trim().is_empty() {
                return Err(StoreError::schema_invariant(
                    "project_state",
                    "acceptance criterion statement must not be empty",
                ));
            }
            ids.push(criterion.acceptance_criterion_id.clone());
            self.tx.execute(
                "INSERT INTO acceptance_criteria (
                    project_id,
                    acceptance_criterion_id,
                    task_id,
                    statement,
                    evidence_requirement,
                    position,
                    status,
                    created_at,
                    updated_at,
                    retired_at
                )
                VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, 'active',
                    ?7,
                    ?7,
                    NULL
                )
                ON CONFLICT(project_id, acceptance_criterion_id) DO UPDATE SET
                    statement = excluded.statement,
                    evidence_requirement = excluded.evidence_requirement,
                    position = excluded.position,
                    updated_at = excluded.updated_at
                WHERE acceptance_criteria.task_id = excluded.task_id
                  AND acceptance_criteria.status = 'active'",
                params![
                    self.project_id,
                    criterion.acceptance_criterion_id,
                    input.task_id,
                    criterion.statement,
                    criterion.evidence_requirement,
                    i64::try_from(criterion.position).map_err(|_| StoreError::schema_invariant(
                        "project_state",
                        "acceptance criterion position exceeds SQLite INTEGER range",
                    ))?,
                    self.committed_at,
                ],
            )?;
        }

        if ids.is_empty() {
            self.tx.execute(
                "UPDATE acceptance_criteria
                    SET status = 'retired',
                        retired_at = ?3,
                        updated_at = ?3
                  WHERE project_id = ?1
                    AND task_id = ?2
                    AND status = 'active'",
                params![self.project_id, input.task_id, self.committed_at],
            )?;
        } else {
            let placeholders = (0..ids.len()).map(|_| "?").collect::<Vec<_>>().join(", ");
            let sql = format!(
                "UPDATE acceptance_criteria
                    SET status = 'retired',
                        retired_at = ?,
                        updated_at = ?
                  WHERE project_id = ?
                    AND task_id = ?
                    AND status = 'active'
                    AND acceptance_criterion_id NOT IN ({placeholders})"
            );
            let mut values: Vec<&dyn rusqlite::ToSql> = vec![
                &self.committed_at,
                &self.committed_at,
                &self.project_id,
                &input.task_id,
            ];
            values.extend(ids.iter().map(|id| id as &dyn rusqlite::ToSql));
            self.tx.execute(&sql, values.as_slice())?;
        }
        Ok(())
    }

    fn update_task_scope_revision(&mut self, input: &TaskScopeRevisionUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        let scope_revision = u64_to_i64("tasks.scope_revision", input.scope_revision)?;
        let changed = self.tx.execute(
            "UPDATE tasks
                SET scope_revision = ?3,
                    updated_at = ?4
              WHERE project_id = ?1
                AND task_id = ?2",
            params![
                self.project_id,
                input.task_id,
                scope_revision,
                self.committed_at
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "Task scope revision update changed no rows".to_owned(),
            })
        }
    }

    fn update_task_close_basis(&mut self, input: &TaskCloseBasisUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        if let Some(value) = &input.close_basis_json {
            validate_current_close_basis_json("tasks.close_basis_json", value)?;
        }
        let close_basis_revision =
            u64_to_i64("tasks.close_basis_revision", input.close_basis_revision)?;
        let changed = self.tx.execute(
            "UPDATE tasks
                SET close_basis_revision = ?3,
                    close_basis_json = ?4,
                    updated_at = ?5
              WHERE project_id = ?1
                AND task_id = ?2",
            params![
                self.project_id,
                input.task_id,
                close_basis_revision,
                input.close_basis_json,
                self.committed_at
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "Task close-basis update changed no rows".to_owned(),
            })
        }
    }

    fn update_task_text_column(
        &mut self,
        task_id: &str,
        column: &'static str,
        value: &str,
    ) -> StoreResult<()> {
        let sql = match column {
            "shaping_summary_json" => {
                "UPDATE tasks SET shaping_summary_json = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            "bounded_context_json" => {
                "UPDATE tasks SET bounded_context_json = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            "autonomy_boundary_json" => {
                "UPDATE tasks SET autonomy_boundary_json = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            "close_summary_json" => {
                "UPDATE tasks SET close_summary_json = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            "lifecycle_phase" => {
                "UPDATE tasks SET lifecycle_phase = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            "work_phase" => {
                "UPDATE tasks SET work_phase = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            "result" => {
                "UPDATE tasks SET result = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            _ => {
                return Err(StoreError::InvalidInput {
                    detail: format!("unsupported Task text column {column}"),
                })
            }
        };
        let changed = self.tx.execute(
            sql,
            params![self.project_id, task_id, value, self.committed_at],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: format!("Task column {column} update changed no rows"),
            })
        }
    }

    fn update_task_nullable_text_column(
        &mut self,
        task_id: &str,
        column: &'static str,
        value: Option<&str>,
    ) -> StoreResult<()> {
        let sql = match column {
            "title" => {
                "UPDATE tasks SET title = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            "summary" => {
                "UPDATE tasks SET summary = ?3, updated_at = ?4 WHERE project_id = ?1 AND task_id = ?2"
            }
            _ => {
                return Err(StoreError::InvalidInput {
                    detail: format!("unsupported nullable Task column {column}"),
                })
            }
        };
        let changed = self.tx.execute(
            sql,
            params![self.project_id, task_id, value, self.committed_at],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: format!("Task column {column} update changed no rows"),
            })
        }
    }
}

#[cfg(test)]
mod behavior_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_pipeline::mutations::with_empty_mutation_context;

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

    #[test]
    fn task_decoders_own_close_summary_and_current_basis_corruption() {
        let mut record = TaskRecord {
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
            scope_revision: 1,
            close_basis_revision: 0,
            close_basis_json: None,
            close_summary_json: r#"{"close_reason":"none"}"#.to_owned(),
            current_change_unit_id: None,
            closed_at: None,
            metadata_json: "{}".to_owned(),
        };

        for malformed in [
            "{",
            r#"{"close_reason":"unknown"}"#,
            r#"{"close_reason":"none","residual_risks":"wrong-type"}"#,
        ] {
            record.close_summary_json = malformed.to_owned();
            assert!(matches!(
                validate_decoded_task_record(record.clone()),
                Err(StoreError::CorruptOwnerStateJson {
                    table: "tasks",
                    logical_column: "close_summary_json",
                    ..
                })
            ));
        }

        assert!(matches!(
            decode_current_close_basis_column("task", Some("{")),
            Err(StoreError::CorruptOwnerStateJson {
                table: "tasks",
                logical_column: "close_basis_json",
                ..
            })
        ));
    }

    #[test]
    fn task_mutation_validates_its_storage_identity_before_sql() {
        let error = with_empty_mutation_context(|context| {
            TaskMutation::SetActive {
                task_id: " ".to_owned(),
            }
            .apply(context)
            .expect_err("blank task id must fail before SQL")
        });

        assert!(matches!(error, StoreError::InvalidInput { .. }));
    }
}
