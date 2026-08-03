use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use volicord_types::{
    ids::{BaselineRef, TaskId},
    schema::{
        advisor_compatible_change_unit, CarryForwardDisposition, CurrentCloseBasis, JsonObject,
        SourceRef, StateRecordRef,
    },
    values::{
        AcceptancePolicy, ActorSource, EvidenceRequirement, PersistedCloseSummary,
        RequestedControlLevel, StateRecordKind, TaskControlLevel, TaskLifecyclePhase,
        TaskLineageRelation, TaskMode, TaskResult, UtcTimestamp, WorkPhase,
    },
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

/// Task and acceptance mutation applied inside one Core commit transaction.
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct TaskInsert {
    pub task_id: String,
    pub created_by_actor_source: ActorSource,
    pub mode: TaskMode,
    pub requested_control_level: RequestedControlLevel,
    pub effective_control_level: TaskControlLevel,
    pub control_level_reason: String,
    pub work_phase: WorkPhase,
    pub acceptance_policy: AcceptancePolicy,
    pub acceptance_policy_reason: String,
    pub predecessor_task_id: Option<String>,
    pub lineage_relation: Option<TaskLineageRelation>,
    pub lineage_reason: Option<String>,
    pub carry_forward: Vec<CarryForwardDisposition>,
    pub lifecycle_phase: TaskLifecyclePhase,
    pub result: Option<TaskResult>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub shaping: TaskShapingFacts,
    pub bounded_context: JsonObject,
    pub autonomy_boundary: TaskAutonomyBoundary,
    pub close_summary: PersistedCloseSummary,
    pub current_change_unit_id: Option<String>,
}

/// Storage input for updating Task scope-shaped current fields.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskScopeUpdate {
    pub task_id: String,
    pub work_phase: Option<WorkPhase>,
    pub lifecycle_phase: Option<TaskLifecyclePhase>,
    pub result: Option<TaskResult>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub shaping: Option<TaskShapingFacts>,
    pub bounded_context: Option<JsonObject>,
    pub autonomy_boundary: Option<TaskAutonomyBoundary>,
    pub close_summary: Option<PersistedCloseSummary>,
}

/// Storage input for an upward-only Task control transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskControlLevelUpdate {
    pub task_id: String,
    pub effective_control_level: TaskControlLevel,
    pub control_level_reason: String,
    pub acceptance_policy: Option<AcceptancePolicy>,
    pub acceptance_policy_reason: Option<String>,
}

/// One canonical acceptance criterion in a complete Task replacement set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceCriterionUpsert {
    pub acceptance_criterion_id: String,
    pub statement: String,
    pub evidence_requirement: EvidenceRequirement,
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
#[derive(Debug, Clone, PartialEq)]
pub struct TaskCloseBasisUpdate {
    pub task_id: String,
    pub close_basis_revision: u64,
    pub close_basis: Option<CurrentCloseBasis>,
}

/// Storage input for applying one terminal Task close transition.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskCloseUpdate {
    pub task_id: String,
    pub lifecycle_phase: TaskLifecyclePhase,
    pub result: TaskResult,
    pub close_summary: PersistedCloseSummary,
    pub closed_at: UtcTimestamp,
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
#[derive(Debug, Clone, PartialEq)]
pub struct TaskRecord {
    pub project_id: String,
    pub task_id: String,
    pub mode: TaskMode,
    pub requested_control_level: RequestedControlLevel,
    pub effective_control_level: TaskControlLevel,
    pub control_level_reason: String,
    pub work_phase: WorkPhase,
    pub acceptance_policy: AcceptancePolicy,
    pub acceptance_policy_reason: String,
    pub predecessor_task_id: Option<String>,
    pub lineage_relation: Option<TaskLineageRelation>,
    pub lineage_reason: Option<String>,
    pub carry_forward: Vec<CarryForwardDisposition>,
    pub lifecycle_phase: TaskLifecyclePhase,
    pub result: Option<TaskResult>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub shaping: TaskShapingFacts,
    pub bounded_context: JsonObject,
    pub autonomy_boundary: TaskAutonomyBoundary,
    pub scope_revision: u64,
    pub close_basis_revision: u64,
    pub close_basis: Option<CurrentCloseBasis>,
    pub close_summary: PersistedCloseSummary,
    pub current_change_unit_id: Option<String>,
    pub closed_at: Option<UtcTimestamp>,
    pub metadata: JsonObject,
}

/// Strictly decoded Task shaping facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskShapingFacts {
    #[serde(default)]
    pub goal_summary: Option<String>,
    #[serde(default)]
    pub scope_summary: Option<String>,
    #[serde(default)]
    pub non_goals: Vec<String>,
    #[serde(default)]
    pub baseline_ref: Option<BaselineRef>,
    #[serde(default)]
    pub autonomy_boundary: Option<String>,
    #[serde(default)]
    pub initial_context_refs: Vec<StateRecordRef>,
    #[serde(default)]
    pub initial_source_refs: Vec<SourceRef>,
}

/// Strictly decoded Task autonomy-boundary facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskAutonomyBoundary {
    #[serde(default)]
    pub autonomy_boundary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptanceCriterionStatus {
    Active,
    Retired,
}

/// Canonical acceptance criterion row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceCriterionRecord {
    pub project_id: String,
    pub acceptance_criterion_id: String,
    pub task_id: String,
    pub statement: String,
    pub evidence_requirement: EvidenceRequirement,
    pub position: u64,
    pub status: AcceptanceCriterionStatus,
}

/// Canonical Task-scoped supplemental evidence claim row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceClaimRecord {
    pub project_id: String,
    pub evidence_claim_id: String,
    pub task_id: String,
    pub statement: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AcceptanceCriterionRow {
    project_id: String,
    acceptance_criterion_id: String,
    task_id: String,
    statement: String,
    evidence_requirement: String,
    position: u64,
    status: String,
}

/// Current Task revision coordinates and optional strict-decoded close basis.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskRevisionRecord {
    pub project_id: String,
    pub task_id: String,
    pub scope_revision: u64,
    pub close_basis_revision: u64,
    pub current_close_basis: Option<CurrentCloseBasis>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskRecordRow {
    project_id: String,
    task_id: String,
    mode: String,
    requested_control_level: String,
    effective_control_level: String,
    control_level_reason: String,
    work_phase: String,
    acceptance_policy: String,
    acceptance_policy_reason: String,
    predecessor_task_id: Option<String>,
    lineage_relation: Option<String>,
    lineage_reason: Option<String>,
    carry_forward_json: String,
    lifecycle_phase: String,
    result: Option<String>,
    title: Option<String>,
    summary: Option<String>,
    shaping_summary_json: String,
    bounded_context_json: String,
    autonomy_boundary_json: String,
    scope_revision: u64,
    close_basis_revision: u64,
    close_basis_json: Option<String>,
    close_summary_json: String,
    current_change_unit_id: Option<String>,
    closed_at: Option<String>,
    metadata_json: String,
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
        self.task_record(task_id).map(|record| {
            record.map(|task| TaskRevisionRecord {
                project_id: task.project_id,
                task_id: task.task_id,
                scope_revision: task.scope_revision,
                close_basis_revision: task.close_basis_revision,
                current_close_basis: task.close_basis,
            })
        })
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
    let record = conn
        .query_row(&sql, params![project_id, task_id], task_record_from_row)
        .optional()
        .map_err(StoreError::from)?
        .map(validate_decoded_task_record)
        .transpose()?;
    record
        .map(|record| validate_task_aggregate(conn, record))
        .transpose()
}

fn validate_task_aggregate(conn: &Connection, task: TaskRecord) -> StoreResult<TaskRecord> {
    let Some(basis) = task.close_basis.as_ref() else {
        return Ok(task);
    };
    let common_matches = basis.task_id.as_str() == task.task_id
        && basis.scope_revision == task.scope_revision
        && basis.close_basis_revision == task.close_basis_revision
        && basis.baseline_ref.as_ref() == task.shaping.baseline_ref.as_ref()
        && task.current_change_unit_id.as_deref() == Some(basis.change_unit_id.as_str());
    if !common_matches {
        return Err(StoreError::SchemaInvariant {
            database_kind: "project_state",
            detail: "current close basis does not match its Task aggregate coordinates".to_owned(),
        });
    }
    if task.mode == TaskMode::Advisor {
        if basis.source_run_ref.is_some()
            || basis
                .shaping_checkpoint_ref
                .as_ref()
                .is_none_or(|reference| {
                    reference.record_kind != StateRecordKind::ShapingCheckpoint
                        || reference.project_id.as_str() != task.project_id
                        || reference.task_id.as_ref().map(TaskId::as_str)
                            != Some(task.task_id.as_str())
                })
        {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "advisor close basis has invalid checkpoint-backed lineage".to_owned(),
            });
        }
        let checkpoint_id = basis
            .shaping_checkpoint_ref
            .as_ref()
            .expect("checked advisor checkpoint ref")
            .record_id
            .as_str();
        let checkpoint_matches: bool = conn.query_row(
            "SELECT EXISTS (
               SELECT 1 FROM shaping_checkpoints
                WHERE project_id = ?1 AND task_id = ?2
                  AND shaping_checkpoint_id = ?3 AND readiness <> 'superseded'
                  AND scope_revision = ?4 AND baseline_ref = ?5
             )",
            params![
                task.project_id,
                task.task_id,
                checkpoint_id,
                i64::try_from(task.scope_revision).map_err(|_| StoreError::SchemaInvariant {
                    database_kind: "project_state",
                    detail: "Task scope revision is too large".to_owned(),
                })?,
                task.shaping.baseline_ref.as_ref().map(BaselineRef::as_str),
            ],
            |row| row.get(0),
        )?;
        let change_unit =
            super::change_units::current_change_unit(conn, &task.project_id, &task.task_id)?;
        let change_unit_matches = change_unit.as_ref().is_some_and(|change_unit| {
            change_unit.change_unit_id == basis.change_unit_id.as_str()
                && advisor_compatible_change_unit(
                    &change_unit.bounded_paths,
                    change_unit.effect_contract.as_ref(),
                )
        });
        let applied_ids = {
            let mut statement = conn.prepare(
                "SELECT l.user_action_resolution_id
                   FROM shaping_checkpoint_gaps AS g
                   JOIN shaping_checkpoint_user_actions AS l
                     ON l.project_id = g.project_id
                    AND l.shaping_checkpoint_id = g.shaping_checkpoint_id
                    AND l.shaping_gap_id = g.shaping_gap_id
                  WHERE g.project_id = ?1 AND g.shaping_checkpoint_id = ?2
                    AND g.status = 'applied' AND l.user_action_resolution_id IS NOT NULL
                  ORDER BY l.user_action_resolution_id",
            )?;
            let rows = statement
                .query_map(params![task.project_id, checkpoint_id], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        let mut basis_ids = basis
            .applied_user_action_resolution_refs
            .iter()
            .map(|reference| reference.record_id.as_str().to_owned())
            .collect::<Vec<_>>();
        basis_ids.sort();
        if !checkpoint_matches || !change_unit_matches || applied_ids != basis_ids {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "advisor close basis does not match its current Change Unit, checkpoint, or applied resolutions"
                    .to_owned(),
            });
        }
    } else if basis.source_run_ref.is_none()
        || basis.shaping_checkpoint_ref.is_some()
        || !basis.applied_user_action_resolution_refs.is_empty()
    {
        return Err(StoreError::SchemaInvariant {
            database_kind: "project_state",
            detail: "direct/work close basis must remain Run-backed".to_owned(),
        });
    }
    Ok(task)
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

fn task_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRecordRow> {
    Ok(TaskRecordRow {
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

fn validate_decoded_task_record(row: TaskRecordRow) -> StoreResult<TaskRecord> {
    let record_id = row.task_id.clone();
    let mode: TaskMode = decode_owner_closed_value("tasks", record_id.clone(), "mode", &row.mode)?;
    let requested_control_level: RequestedControlLevel = decode_owner_closed_value(
        "tasks",
        record_id.clone(),
        "requested_control_level",
        &row.requested_control_level,
    )?;
    let effective_control_level: TaskControlLevel = decode_owner_closed_value(
        "tasks",
        record_id.clone(),
        "effective_control_level",
        &row.effective_control_level,
    )?;
    let work_phase: WorkPhase =
        decode_owner_closed_value("tasks", record_id.clone(), "work_phase", &row.work_phase)?;
    let acceptance_policy: AcceptancePolicy = decode_owner_closed_value(
        "tasks",
        record_id.clone(),
        "acceptance_policy",
        &row.acceptance_policy,
    )?;
    let lineage_relation = row
        .lineage_relation
        .as_deref()
        .map(|value| {
            decode_owner_closed_value::<TaskLineageRelation>(
                "tasks",
                record_id.clone(),
                "lineage_relation",
                value,
            )
        })
        .transpose()?;
    let lifecycle_phase: TaskLifecyclePhase = decode_owner_closed_value(
        "tasks",
        record_id.clone(),
        "lifecycle_phase",
        &row.lifecycle_phase,
    )?;
    let result = row
        .result
        .as_deref()
        .map(|value| {
            decode_owner_closed_value::<TaskResult>("tasks", record_id.clone(), "result", value)
        })
        .transpose()?;
    let carry_forward = decode_owner_json_text(
        "tasks",
        record_id.clone(),
        "carry_forward_json",
        &row.carry_forward_json,
    )?;
    let shaping = decode_owner_json_text(
        "tasks",
        record_id.clone(),
        "shaping_summary_json",
        &row.shaping_summary_json,
    )?;
    let bounded_context = decode_owner_json_text(
        "tasks",
        record_id.clone(),
        "bounded_context_json",
        &row.bounded_context_json,
    )?;
    let autonomy_boundary = decode_owner_json_text(
        "tasks",
        record_id.clone(),
        "autonomy_boundary_json",
        &row.autonomy_boundary_json,
    )?;
    let close_basis =
        decode_current_close_basis_column(&record_id, row.close_basis_json.as_deref())?;
    let close_summary = decode_owner_json_text(
        "tasks",
        record_id.clone(),
        "close_summary_json",
        &row.close_summary_json,
    )?;
    let closed_at = row
        .closed_at
        .as_deref()
        .map(UtcTimestamp::parse)
        .transpose()
        .map_err(|_| {
            StoreError::corrupt_owner_state_value("tasks", record_id.clone(), "closed_at")
        })?;
    let metadata = decode_owner_json_text("tasks", record_id, "metadata_json", &row.metadata_json)?;
    Ok(TaskRecord {
        project_id: row.project_id,
        task_id: row.task_id,
        mode,
        requested_control_level,
        effective_control_level,
        control_level_reason: row.control_level_reason,
        work_phase,
        acceptance_policy,
        acceptance_policy_reason: row.acceptance_policy_reason,
        predecessor_task_id: row.predecessor_task_id,
        lineage_relation,
        lineage_reason: row.lineage_reason,
        carry_forward,
        lifecycle_phase,
        result,
        title: row.title,
        summary: row.summary,
        shaping,
        bounded_context,
        autonomy_boundary,
        scope_revision: row.scope_revision,
        close_basis_revision: row.close_basis_revision,
        close_basis,
        close_summary,
        current_change_unit_id: row.current_change_unit_id,
        closed_at,
        metadata,
    })
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
    let rows = statement.query_map(params![project_id, task_id], acceptance_criterion_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StoreError::from)
        .and_then(|rows| {
            rows.into_iter()
                .map(decode_acceptance_criterion_record)
                .collect()
        })
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
        acceptance_criterion_row,
    )
    .optional()
    .map_err(StoreError::from)?
    .map(decode_acceptance_criterion_record)
    .transpose()
}

fn acceptance_criterion_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AcceptanceCriterionRow> {
    Ok(AcceptanceCriterionRow {
        project_id: row.get(0)?,
        acceptance_criterion_id: row.get(1)?,
        task_id: row.get(2)?,
        statement: row.get(3)?,
        evidence_requirement: row.get(4)?,
        position: nonnegative_i64_to_u64("acceptance_criteria.position", row.get(5)?)?,
        status: row.get(6)?,
    })
}

fn decode_acceptance_criterion_record(
    row: AcceptanceCriterionRow,
) -> StoreResult<AcceptanceCriterionRecord> {
    let acceptance_criterion_id = row.acceptance_criterion_id;
    let evidence_requirement = decode_owner_closed_value(
        "acceptance_criteria",
        acceptance_criterion_id.clone(),
        "evidence_requirement",
        &row.evidence_requirement,
    )?;
    let status = match row.status.as_str() {
        "active" => AcceptanceCriterionStatus::Active,
        "retired" => AcceptanceCriterionStatus::Retired,
        _ => {
            return Err(StoreError::corrupt_owner_state_value(
                "acceptance_criteria",
                acceptance_criterion_id,
                "status",
            ))
        }
    };
    Ok(AcceptanceCriterionRecord {
        project_id: row.project_id,
        acceptance_criterion_id,
        task_id: row.task_id,
        statement: row.statement,
        evidence_requirement,
        position: row.position,
        status,
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

fn task_control_level_rank(value: TaskControlLevel) -> u8 {
    match value {
        TaskControlLevel::Observe => 0,
        TaskControlLevel::Light => 1,
        TaskControlLevel::Tracked => 2,
        TaskControlLevel::Sensitive => 3,
    }
}

fn acceptance_policy_rank(value: AcceptancePolicy) -> u8 {
    match value {
        AcceptancePolicy::NotRequired => 0,
        AcceptancePolicy::PolicyDependent => 1,
        AcceptancePolicy::Required => 2,
    }
}

impl MutationContext<'_> {
    fn insert_task(&mut self, input: &TaskInsert) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        if input.control_level_reason.trim().is_empty() {
            return Err(StoreError::InvalidInput {
                detail: "control_level_reason must not be empty".to_owned(),
            });
        }
        if input.acceptance_policy_reason.trim().is_empty() {
            return Err(StoreError::schema_invariant(
                "project_state",
                "Task acceptance policy reason must not be empty",
            ));
        }
        let created_by_actor_source = input.created_by_actor_source.to_canonical_string();
        let mode = encode_closed_value("tasks.mode", &input.mode)?;
        let requested_control_level = encode_closed_value(
            "tasks.requested_control_level",
            &input.requested_control_level,
        )?;
        let effective_control_level = encode_closed_value(
            "tasks.effective_control_level",
            &input.effective_control_level,
        )?;
        let work_phase = encode_closed_value("tasks.work_phase", &input.work_phase)?;
        let acceptance_policy =
            encode_closed_value("tasks.acceptance_policy", &input.acceptance_policy)?;
        let lineage_relation = input
            .lineage_relation
            .as_ref()
            .map(|value| encode_closed_value("tasks.lineage_relation", value))
            .transpose()?;
        let carry_forward_json =
            encode_json_column("tasks.carry_forward_json", &input.carry_forward)?;
        let lifecycle_phase = encode_closed_value("tasks.lifecycle_phase", &input.lifecycle_phase)?;
        let result = input
            .result
            .as_ref()
            .map(|value| encode_closed_value("tasks.result", value))
            .transpose()?;
        let shaping_summary_json =
            encode_json_column("tasks.shaping_summary_json", &input.shaping)?;
        let bounded_context_json =
            encode_json_column("tasks.bounded_context_json", &input.bounded_context)?;
        let autonomy_boundary_json =
            encode_json_column("tasks.autonomy_boundary_json", &input.autonomy_boundary)?;
        let close_summary_json =
            encode_json_column("tasks.close_summary_json", &input.close_summary)?;
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
                created_by_actor_source,
                mode,
                requested_control_level,
                effective_control_level,
                input.control_level_reason,
                work_phase,
                acceptance_policy,
                input.acceptance_policy_reason,
                input.predecessor_task_id,
                lineage_relation,
                input.lineage_reason,
                carry_forward_json,
                lifecycle_phase,
                result,
                input.title,
                input.summary,
                shaping_summary_json,
                bounded_context_json,
                autonomy_boundary_json,
                close_summary_json,
                input.current_change_unit_id,
                self.committed_at
            ],
        )?;
        Ok(())
    }

    fn update_task_control_level(&mut self, input: &TaskControlLevelUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        let requested_rank = task_control_level_rank(input.effective_control_level);
        if input.control_level_reason.trim().is_empty() {
            return Err(StoreError::InvalidInput {
                detail: "control_level_reason must not be empty".to_owned(),
            });
        }
        let acceptance_update = match (
            input.acceptance_policy,
            input.acceptance_policy_reason.as_deref(),
        ) {
            (None, None) => None,
            (Some(policy), Some(reason)) if !reason.trim().is_empty() => {
                Some((policy, reason, acceptance_policy_rank(policy)))
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
        let current_level: TaskControlLevel = decode_owner_closed_value(
            "tasks",
            input.task_id.clone(),
            "effective_control_level",
            &current_level,
        )?;
        let current_acceptance_policy: AcceptancePolicy = decode_owner_closed_value(
            "tasks",
            input.task_id.clone(),
            "acceptance_policy",
            &current_acceptance_policy,
        )?;
        if requested_rank < task_control_level_rank(current_level) {
            return Err(StoreError::Conflict {
                entity: "task",
                id: input.task_id.clone(),
                detail: "effective Task control level cannot decrease".to_owned(),
            });
        }
        if let Some((_, _, requested_acceptance_rank)) = &acceptance_update {
            if *requested_acceptance_rank < acceptance_policy_rank(current_acceptance_policy) {
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
        let effective_control_level = encode_closed_value(
            "tasks.effective_control_level",
            &input.effective_control_level,
        )?;
        let acceptance_policy_text = acceptance_policy
            .as_ref()
            .map(|value| encode_closed_value("tasks.acceptance_policy", value))
            .transpose()?;
        let current_acceptance_policy_text =
            encode_closed_value("tasks.acceptance_policy", &current_acceptance_policy)?;
        let metadata_json = clear_satisfied_task_policy_reevaluation(
            &metadata_json,
            &input.task_id,
            &effective_control_level,
            acceptance_policy_text
                .as_deref()
                .unwrap_or(&current_acceptance_policy_text),
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
                effective_control_level,
                input.control_level_reason,
                acceptance_policy_text,
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
        let lifecycle_phase = encode_closed_value("tasks.lifecycle_phase", &input.lifecycle_phase)?;
        let result = encode_closed_value("tasks.result", &input.result)?;
        let close_summary_json =
            encode_json_column("tasks.close_summary_json", &input.close_summary)?;
        let closed_at = input.closed_at.to_string();

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
                lifecycle_phase,
                result,
                close_summary_json,
                closed_at,
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
        if let Some(value) = &input.shaping {
            let value = encode_json_column("tasks.shaping_summary_json", value)?;
            self.update_task_text_column(&input.task_id, "shaping_summary_json", &value)?;
        }
        if let Some(value) = &input.bounded_context {
            let value = encode_json_column("tasks.bounded_context_json", value)?;
            self.update_task_text_column(&input.task_id, "bounded_context_json", &value)?;
        }
        if let Some(value) = &input.autonomy_boundary {
            let value = encode_json_column("tasks.autonomy_boundary_json", value)?;
            self.update_task_text_column(&input.task_id, "autonomy_boundary_json", &value)?;
        }
        if let Some(value) = &input.close_summary {
            let value = encode_json_column("tasks.close_summary_json", value)?;
            self.update_task_text_column(&input.task_id, "close_summary_json", &value)?;
        }
        if let Some(value) = &input.lifecycle_phase {
            let value = encode_closed_value("tasks.lifecycle_phase", value)?;
            self.update_task_text_column(&input.task_id, "lifecycle_phase", &value)?;
        }
        if let Some(value) = &input.work_phase {
            let value = encode_closed_value("tasks.work_phase", value)?;
            self.update_task_text_column(&input.task_id, "work_phase", &value)?;
        }
        if let Some(value) = &input.result {
            let value = encode_closed_value("tasks.result", value)?;
            self.update_task_text_column(&input.task_id, "result", &value)?;
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
            let evidence_requirement = encode_closed_value(
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
                    evidence_requirement,
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
        let close_basis_json = input
            .close_basis
            .as_ref()
            .map(|value| encode_json_column("tasks.close_basis_json", value))
            .transpose()?;
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
                close_basis_json,
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
        let record = TaskRecordRow {
            project_id: "project".to_owned(),
            task_id: "task".to_owned(),
            mode: "work".to_owned(),
            requested_control_level: "light".to_owned(),
            effective_control_level: "light".to_owned(),
            control_level_reason: "reason".to_owned(),
            work_phase: "implementation".to_owned(),
            acceptance_policy: "not_required".to_owned(),
            acceptance_policy_reason: "reason".to_owned(),
            predecessor_task_id: None,
            lineage_relation: None,
            lineage_reason: None,
            carry_forward_json: "[]".to_owned(),
            lifecycle_phase: "ready".to_owned(),
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
        let mut record = TaskRecordRow {
            project_id: "project".to_owned(),
            task_id: "task".to_owned(),
            mode: "work".to_owned(),
            requested_control_level: "light".to_owned(),
            effective_control_level: "light".to_owned(),
            control_level_reason: "reason".to_owned(),
            work_phase: "implementation".to_owned(),
            acceptance_policy: "not_required".to_owned(),
            acceptance_policy_reason: "reason".to_owned(),
            predecessor_task_id: None,
            lineage_relation: None,
            lineage_reason: None,
            carry_forward_json: "[]".to_owned(),
            lifecycle_phase: "ready".to_owned(),
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
        let valid_record = record.clone();

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

        let mut unknown_mode = valid_record;
        unknown_mode.mode = "legacy".to_owned();
        assert!(matches!(
            validate_decoded_task_record(unknown_mode),
            Err(StoreError::CorruptOwnerStateValue {
                table: "tasks",
                logical_column: "mode",
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
