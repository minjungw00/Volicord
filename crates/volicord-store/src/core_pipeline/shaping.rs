use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::canonical::canonical_json_string;
use volicord_types::ids::{BaselineRef, TaskId};
use volicord_types::schema::{ShapingCheckpointOperation, SourceRef, StateRecordRef};
use volicord_types::values::{
    ShapingCheckpointReadiness, ShapingDecisionApplicationOwner, ShapingGapKind, ShapingGapStatus,
    UserActionKind, UserActionRequiredFor, UtcTimestamp,
};

use super::{facade::CoreProjectStore, mutations::MutationContext, validation::*};
use crate::{StoreError, StoreResult};

const CHECKPOINT_COLUMNS: &str = "
    project_id, shaping_checkpoint_id, predecessor_shaping_checkpoint_id,
    task_id, scope_revision, baseline_ref,
    summary, implementation_boundary, readiness, source_refs_json,
    evidence_refs_json, created_at, superseded_at";

const GAP_COLUMNS: &str = "
    project_id, shaping_checkpoint_id, shaping_gap_id, task_id, gap_kind,
    summary, affected_refs_json, status, user_action_request_id,
    user_action_kind";

/// Shaping-checkpoint mutation applied inside one Core commit transaction.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapingCheckpointMutation {
    Record(ShapingCheckpointInsert),
    ResolveLinkedGap {
        user_action_request_id: String,
        user_action_resolution_id: String,
    },
    ApplyScopeAndRebaseCurrent {
        task_id: String,
        shaping_checkpoint_id: String,
        scope_revision: u64,
        baseline_ref: Option<BaselineRef>,
        applications: Vec<ShapingGapApplication>,
    },
    ApplyAdvanceAndTransition(ShapingAdvanceApplication),
    SupersedeCurrent {
        task_id: String,
    },
}

/// Exact shaping-gap and UserAction resolution pair selected for application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapingGapApplication {
    pub shaping_gap_id: String,
    pub user_action_resolution_id: String,
}

/// Exact aggregate basis for applying advance-owned decisions and entering implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapingAdvanceApplication {
    pub task_id: String,
    pub shaping_checkpoint_id: String,
    pub change_unit_id: String,
    pub scope_revision: u64,
    pub baseline_ref: BaselineRef,
    pub applications: Vec<ShapingGapApplication>,
}

/// Storage input for one checkpoint and its complete gap/link set.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapingCheckpointInsert {
    pub shaping_checkpoint_id: String,
    pub checkpoint_operation: ShapingCheckpointOperation,
    pub task_id: String,
    pub scope_revision: u64,
    pub baseline_ref: Option<BaselineRef>,
    pub summary: String,
    pub implementation_boundary: Option<String>,
    pub readiness: ShapingCheckpointReadiness,
    pub source_refs: Vec<SourceRef>,
    pub evidence_refs: Vec<StateRecordRef>,
    pub created_at: UtcTimestamp,
    pub gaps: Vec<ShapingCheckpointGapInsert>,
}

/// Storage input for one typed checkpoint gap.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapingCheckpointGapInsert {
    pub shaping_gap_id: String,
    pub gap_kind: ShapingGapKind,
    pub summary: String,
    pub affected_refs: Vec<StateRecordRef>,
    pub user_action: Option<ShapingCheckpointUserActionInsert>,
}

/// Storage input linking one user-owned gap to its exact request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapingCheckpointUserActionInsert {
    pub user_action_request_id: String,
    pub action_kind: UserActionKind,
}

/// Strictly decoded shaping checkpoint with its complete gap set.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapingCheckpointRecord {
    pub project_id: String,
    pub shaping_checkpoint_id: String,
    pub predecessor_shaping_checkpoint_id: Option<String>,
    pub task_id: String,
    pub scope_revision: u64,
    pub baseline_ref: Option<BaselineRef>,
    pub summary: String,
    pub implementation_boundary: Option<String>,
    pub readiness: ShapingCheckpointReadiness,
    pub source_refs: Vec<SourceRef>,
    pub evidence_refs: Vec<StateRecordRef>,
    pub created_at: UtcTimestamp,
    pub superseded_at: Option<UtcTimestamp>,
    pub gaps: Vec<ShapingCheckpointGapRecord>,
}

/// Strictly decoded shaping gap.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapingCheckpointGapRecord {
    pub shaping_gap_id: String,
    pub gap_kind: ShapingGapKind,
    pub summary: String,
    pub affected_refs: Vec<StateRecordRef>,
    pub status: ShapingGapStatus,
    pub user_action: Option<ShapingCheckpointUserActionRecord>,
}

/// Strictly decoded exact request/resolution link for a user-owned gap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapingCheckpointUserActionRecord {
    pub user_action_request_id: String,
    pub action_kind: UserActionKind,
    pub user_action_resolution_id: Option<String>,
    pub linked_at: UtcTimestamp,
    pub resolved_at: Option<UtcTimestamp>,
}

impl ShapingCheckpointMutation {
    pub(super) fn apply(&self, context: &mut MutationContext<'_>) -> StoreResult<()> {
        match self {
            Self::Record(input) => context.record_shaping_checkpoint(input),
            Self::ResolveLinkedGap {
                user_action_request_id,
                user_action_resolution_id,
            } => context.resolve_shaping_gap(user_action_request_id, user_action_resolution_id),
            Self::ApplyScopeAndRebaseCurrent {
                task_id,
                shaping_checkpoint_id,
                scope_revision,
                baseline_ref,
                applications,
            } => context.apply_scope_and_rebase_current_shaping_checkpoint(
                task_id,
                shaping_checkpoint_id,
                *scope_revision,
                baseline_ref.as_ref(),
                applications,
            ),
            Self::ApplyAdvanceAndTransition(input) => {
                context.apply_advance_shaping_and_transition(input)
            }
            Self::SupersedeCurrent { task_id } => {
                context.supersede_current_shaping_checkpoint(task_id)
            }
        }
    }
}

impl CoreProjectStore<'_> {
    /// Reads the one non-superseded checkpoint for a Task.
    pub fn current_shaping_checkpoint(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Option<ShapingCheckpointRecord>> {
        shaping_checkpoint_where(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            None,
            true,
        )
    }

    /// Reads one checkpoint by exact Task and checkpoint identity.
    pub fn shaping_checkpoint_record(
        &self,
        task_id: &TaskId,
        shaping_checkpoint_id: &str,
    ) -> StoreResult<Option<ShapingCheckpointRecord>> {
        shaping_checkpoint_where(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            Some(shaping_checkpoint_id),
            false,
        )
    }

    /// Returns whether one checkpoint identity already exists in the project.
    pub fn shaping_checkpoint_id_exists(&self, shaping_checkpoint_id: &str) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT 1 FROM shaping_checkpoints WHERE project_id = ?1 AND shaping_checkpoint_id = ?2",
                params![self.project.project_id, shaping_checkpoint_id],
                |_| Ok(()),
            )
            .optional()
            .map(|value| value.is_some())
            .map_err(StoreError::from)
    }

    /// Returns whether one shaping-gap identity already exists in the project.
    pub fn shaping_gap_id_exists(&self, shaping_gap_id: &str) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT 1 FROM shaping_checkpoint_gaps WHERE project_id = ?1 AND shaping_gap_id = ?2",
                params![self.project.project_id, shaping_gap_id],
                |_| Ok(()),
            )
            .optional()
            .map(|value| value.is_some())
            .map_err(StoreError::from)
    }

    /// Reads the current checkpoint link for one exact UserAction request.
    pub fn shaping_user_action_for_request(
        &self,
        user_action_request_id: &str,
    ) -> StoreResult<Option<(String, String, String)>> {
        self.conn
            .query_row(
                "SELECT shaping_checkpoint_id, shaping_gap_id, task_id
                   FROM shaping_checkpoint_user_actions
                  WHERE project_id = ?1 AND user_action_request_id = ?2",
                params![self.project.project_id, user_action_request_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(StoreError::from)
    }
}

impl MutationContext<'_> {
    fn record_shaping_checkpoint(&mut self, input: &ShapingCheckpointInsert) -> StoreResult<()> {
        validate_checkpoint_insert(input)?;
        let scope_revision =
            i64::try_from(input.scope_revision).map_err(|_| StoreError::InvalidInput {
                detail: "shaping checkpoint scope_revision is too large".to_owned(),
            })?;
        self.require_task_scope_revision(&input.task_id, scope_revision)?;
        let predecessor_shaping_checkpoint_id = self.apply_checkpoint_succession(input)?;
        let source_refs_json =
            canonical_json_string(&input.source_refs).map_err(|_| StoreError::InvalidInput {
                detail: "shaping checkpoint source refs cannot be serialized".to_owned(),
            })?;
        let evidence_refs_json =
            canonical_json_string(&input.evidence_refs).map_err(|_| StoreError::InvalidInput {
                detail: "shaping checkpoint evidence refs cannot be serialized".to_owned(),
            })?;
        let readiness = encode_closed_value("readiness", &input.readiness)?;
        self.tx.execute(
            "INSERT INTO shaping_checkpoints (
               project_id, shaping_checkpoint_id, predecessor_shaping_checkpoint_id,
               task_id, scope_revision,
               baseline_ref, summary, implementation_boundary, readiness,
               source_refs_json, evidence_refs_json, created_at, superseded_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL)",
            params![
                self.project_id,
                input.shaping_checkpoint_id,
                predecessor_shaping_checkpoint_id,
                input.task_id,
                scope_revision,
                input.baseline_ref.as_ref().map(BaselineRef::as_str),
                input.summary,
                input.implementation_boundary,
                readiness,
                source_refs_json,
                evidence_refs_json,
                input.created_at.to_string(),
            ],
        )?;
        for gap in &input.gaps {
            self.insert_shaping_gap(input, gap)?;
        }
        self.reject_detached_live_shaping_authority(
            &input.task_id,
            Some(&input.shaping_checkpoint_id),
        )?;
        Ok(())
    }

    fn apply_checkpoint_succession(
        &mut self,
        input: &ShapingCheckpointInsert,
    ) -> StoreResult<Option<String>> {
        let current_ids = {
            let mut statement = self.tx.prepare(
                "SELECT shaping_checkpoint_id
                   FROM shaping_checkpoints
                  WHERE project_id = ?1
                    AND task_id = ?2
                    AND readiness <> 'superseded'",
            )?;
            let rows = statement
                .query_map(params![self.project_id, input.task_id], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        if current_ids.len() > 1 {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "a Task has more than one current shaping checkpoint".to_owned(),
            });
        }
        match &input.checkpoint_operation {
            ShapingCheckpointOperation::CreateInitial => {
                if !current_ids.is_empty() {
                    return Err(StoreError::InvalidInput {
                        detail: "create_initial requires no current shaping checkpoint".to_owned(),
                    });
                }
                Ok(None)
            }
            ShapingCheckpointOperation::ReplaceCurrent {
                expected_current_checkpoint_id,
            } => {
                let expected = expected_current_checkpoint_id.as_str();
                if current_ids.len() != 1 || current_ids[0] != expected {
                    return Err(StoreError::InvalidInput {
                        detail: "replace_current requires the exact current shaping checkpoint"
                            .to_owned(),
                    });
                }
                self.reject_live_checkpoint_user_actions(&input.task_id, expected)?;
                let changed = self.tx.execute(
                    "UPDATE shaping_checkpoints
                        SET readiness = 'superseded', superseded_at = ?4
                      WHERE project_id = ?1
                        AND task_id = ?2
                        AND shaping_checkpoint_id = ?3
                        AND readiness <> 'superseded'",
                    params![
                        self.project_id,
                        input.task_id,
                        expected,
                        input.created_at.to_string()
                    ],
                )?;
                if changed != 1 {
                    return Err(StoreError::SchemaInvariant {
                        database_kind: "project_state",
                        detail: "exact shaping-checkpoint compare-and-swap changed no row"
                            .to_owned(),
                    });
                }
                Ok(Some(expected.to_owned()))
            }
        }
    }

    fn reject_live_checkpoint_user_actions(
        &self,
        task_id: &str,
        checkpoint_id: &str,
    ) -> StoreResult<()> {
        let live_count: i64 = self.tx.query_row(
            "SELECT COUNT(*)
               FROM shaping_checkpoint_user_actions AS l
               JOIN shaping_checkpoint_gaps AS g
                 ON g.project_id = l.project_id
                AND g.shaping_checkpoint_id = l.shaping_checkpoint_id
                AND g.shaping_gap_id = l.shaping_gap_id
               JOIN user_action_requests AS r
                 ON r.project_id = l.project_id
                AND r.user_action_request_id = l.user_action_request_id
              WHERE l.project_id = ?1
                AND l.task_id = ?2
                AND l.shaping_checkpoint_id = ?3
                AND g.status <> 'applied'
                AND r.basis_status = 'current'",
            params![self.project_id, task_id, checkpoint_id],
            |row| row.get(0),
        )?;
        if live_count != 0 {
            return Err(StoreError::InvalidInput {
                detail: "current shaping checkpoint has live linked UserAction authority"
                    .to_owned(),
            });
        }
        Ok(())
    }

    fn reject_detached_live_shaping_authority(
        &self,
        task_id: &str,
        current_checkpoint_id: Option<&str>,
    ) -> StoreResult<()> {
        let detached_count: i64 = self.tx.query_row(
            "SELECT COUNT(*)
               FROM user_action_requests AS r
              WHERE r.project_id = ?1
                AND r.task_id = ?2
                AND r.source_method = 'volicord.record_shaping'
                AND r.basis_status = 'current'
                AND NOT EXISTS (
                  SELECT 1
                    FROM shaping_checkpoint_user_actions AS applied_link
                    JOIN shaping_checkpoint_gaps AS applied_gap
                      ON applied_gap.project_id = applied_link.project_id
                     AND applied_gap.shaping_checkpoint_id = applied_link.shaping_checkpoint_id
                     AND applied_gap.shaping_gap_id = applied_link.shaping_gap_id
                   WHERE applied_link.project_id = r.project_id
                     AND applied_link.user_action_request_id = r.user_action_request_id
                     AND applied_gap.status = 'applied'
                )
                AND NOT EXISTS (
                  SELECT 1
                    FROM shaping_checkpoint_user_actions AS l
                   WHERE l.project_id = r.project_id
                     AND l.user_action_request_id = r.user_action_request_id
                     AND (?3 IS NOT NULL AND l.shaping_checkpoint_id = ?3)
                )",
            params![self.project_id, task_id, current_checkpoint_id],
            |row| row.get(0),
        )?;
        if detached_count != 0 {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "live shaping UserAction authority is detached from the current checkpoint"
                    .to_owned(),
            });
        }
        Ok(())
    }

    fn insert_shaping_gap(
        &mut self,
        checkpoint: &ShapingCheckpointInsert,
        gap: &ShapingCheckpointGapInsert,
    ) -> StoreResult<()> {
        validate_identifier("shaping_gap_id", &gap.shaping_gap_id)?;
        validate_nonempty_text("shaping gap summary", &gap.summary)?;
        if gap.gap_kind.is_user_owned() != gap.user_action.is_some() {
            return Err(StoreError::InvalidInput {
                detail: "user-owned shaping gaps require one exact UserAction link".to_owned(),
            });
        }
        if gap
            .user_action
            .as_ref()
            .is_some_and(|link| gap.gap_kind.user_action_kind() != Some(link.action_kind))
        {
            return Err(StoreError::InvalidInput {
                detail: "shaping gap kind is incompatible with its UserAction kind".to_owned(),
            });
        }
        let affected_refs_json =
            canonical_json_string(&gap.affected_refs).map_err(|_| StoreError::InvalidInput {
                detail: "shaping gap affected refs cannot be serialized".to_owned(),
            })?;
        let gap_kind = encode_closed_value("gap_kind", &gap.gap_kind)?;
        let status = encode_closed_value("status", &ShapingGapStatus::Current)?;
        let action_kind = gap
            .user_action
            .as_ref()
            .map(|link| encode_closed_value("action_kind", &link.action_kind))
            .transpose()?;
        self.tx.execute(
            "INSERT INTO shaping_checkpoint_gaps (
               project_id, shaping_checkpoint_id, shaping_gap_id, task_id,
               gap_kind, summary, affected_refs_json, status,
               user_action_request_id, user_action_kind
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                self.project_id,
                checkpoint.shaping_checkpoint_id,
                gap.shaping_gap_id,
                checkpoint.task_id,
                gap_kind,
                gap.summary,
                affected_refs_json,
                status,
                gap.user_action
                    .as_ref()
                    .map(|link| link.user_action_request_id.as_str()),
                action_kind,
            ],
        )?;
        if let Some(link) = gap.user_action.as_ref() {
            self.tx.execute(
                "INSERT INTO shaping_checkpoint_user_actions (
                   project_id, shaping_checkpoint_id, shaping_gap_id, task_id,
                   user_action_request_id, action_kind, user_action_resolution_id,
                   linked_at, resolved_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, NULL)",
                params![
                    self.project_id,
                    checkpoint.shaping_checkpoint_id,
                    gap.shaping_gap_id,
                    checkpoint.task_id,
                    link.user_action_request_id,
                    action_kind,
                    self.committed_at,
                ],
            )?;
        }
        Ok(())
    }

    fn resolve_shaping_gap(
        &mut self,
        user_action_request_id: &str,
        user_action_resolution_id: &str,
    ) -> StoreResult<()> {
        validate_identifier("user_action_request_id", user_action_request_id)?;
        validate_identifier("user_action_resolution_id", user_action_resolution_id)?;
        let link = self
            .tx
            .query_row(
                "SELECT shaping_checkpoint_id, shaping_gap_id
                   FROM shaping_checkpoint_user_actions
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2
                    AND user_action_resolution_id IS NULL",
                params![self.project_id, user_action_request_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let Some((checkpoint_id, gap_id)) = link else {
            return Ok(());
        };
        self.tx.execute(
            "UPDATE shaping_checkpoint_user_actions
                SET user_action_resolution_id = ?3, resolved_at = ?4
              WHERE project_id = ?1 AND user_action_request_id = ?2",
            params![
                self.project_id,
                user_action_request_id,
                user_action_resolution_id,
                self.committed_at,
            ],
        )?;
        self.tx.execute(
            "UPDATE shaping_checkpoint_gaps
                SET status = 'resolved'
              WHERE project_id = ?1
                AND shaping_checkpoint_id = ?2
                AND shaping_gap_id = ?3",
            params![self.project_id, checkpoint_id, gap_id],
        )?;
        self.tx.execute(
            "UPDATE shaping_checkpoints
                SET readiness = 'ready'
              WHERE project_id = ?1
                AND shaping_checkpoint_id = ?2
                AND readiness = 'blocked'
                AND baseline_ref IS NOT NULL
                AND implementation_boundary IS NOT NULL
                AND NOT EXISTS (
                  SELECT 1 FROM shaping_checkpoint_gaps
                   WHERE project_id = ?1
                     AND shaping_checkpoint_id = ?2
                     AND status = 'current'
                )",
            params![self.project_id, checkpoint_id],
        )?;
        Ok(())
    }

    fn apply_scope_and_rebase_current_shaping_checkpoint(
        &mut self,
        task_id: &str,
        shaping_checkpoint_id: &str,
        scope_revision: u64,
        baseline_ref: Option<&BaselineRef>,
        applications: &[ShapingGapApplication],
    ) -> StoreResult<()> {
        validate_identifier("task_id", task_id)?;
        validate_identifier("shaping_checkpoint_id", shaping_checkpoint_id)?;
        let scope_revision =
            i64::try_from(scope_revision).map_err(|_| StoreError::InvalidInput {
                detail: "shaping checkpoint scope_revision is too large".to_owned(),
            })?;
        self.require_exact_current_checkpoint(task_id, shaping_checkpoint_id)?;
        self.require_task_scope_revision(task_id, scope_revision)?;
        self.apply_selected_gaps(
            task_id,
            shaping_checkpoint_id,
            ShapingDecisionApplicationOwner::UpdateScope,
            applications,
        )?;
        self.require_owner_gaps_applied(
            shaping_checkpoint_id,
            ShapingDecisionApplicationOwner::UpdateScope,
        )?;
        let changed = self.tx.execute(
            "UPDATE shaping_checkpoints
                SET scope_revision = ?3,
                    baseline_ref = ?4,
                    readiness = CASE
                      WHEN ?4 IS NOT NULL
                       AND implementation_boundary IS NOT NULL
                       AND NOT EXISTS (
                         SELECT 1 FROM shaping_checkpoint_gaps
                          WHERE project_id = ?1
                            AND shaping_checkpoint_id = shaping_checkpoints.shaping_checkpoint_id
                            AND status = 'current'
                       )
                      THEN 'ready'
                      ELSE readiness
                    END
              WHERE project_id = ?1
                AND task_id = ?2
                AND shaping_checkpoint_id = ?5
                AND readiness <> 'superseded'",
            params![
                self.project_id,
                task_id,
                scope_revision,
                baseline_ref.map(BaselineRef::as_str),
                shaping_checkpoint_id,
            ],
        )?;
        if changed != 1 {
            return Err(StoreError::Conflict {
                entity: "shaping_checkpoint",
                id: shaping_checkpoint_id.to_owned(),
                detail: "exact current shaping checkpoint could not be rebased".to_owned(),
            });
        }
        Ok(())
    }

    fn apply_advance_shaping_and_transition(
        &mut self,
        input: &ShapingAdvanceApplication,
    ) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("shaping_checkpoint_id", &input.shaping_checkpoint_id)?;
        validate_identifier("change_unit_id", &input.change_unit_id)?;
        self.require_exact_current_checkpoint(&input.task_id, &input.shaping_checkpoint_id)?;
        let expected_scope_revision =
            i64::try_from(input.scope_revision).map_err(|_| StoreError::InvalidInput {
                detail: "advance scope_revision is too large".to_owned(),
            })?;
        let checkpoint_basis_matches: bool = self.tx.query_row(
            "SELECT EXISTS (
               SELECT 1 FROM shaping_checkpoints
                WHERE project_id = ?1
                  AND task_id = ?2
                  AND shaping_checkpoint_id = ?3
                  AND readiness = 'ready'
                  AND scope_revision = ?4
                  AND baseline_ref = ?5
             )",
            params![
                self.project_id,
                input.task_id,
                input.shaping_checkpoint_id,
                expected_scope_revision,
                input.baseline_ref.as_str(),
            ],
            |row| row.get(0),
        )?;
        if !checkpoint_basis_matches {
            return Err(StoreError::InvalidInput {
                detail: "advance shaping checkpoint basis is not exact and ready".to_owned(),
            });
        }
        let current_change_unit_matches: bool = self.tx.query_row(
            "SELECT EXISTS (
               SELECT 1 FROM change_units
                WHERE project_id = ?1
                  AND task_id = ?2
                  AND change_unit_id = ?3
                  AND status = 'active'
                  AND is_current = 1
                  AND json_extract(write_basis_json, '$.baseline_ref') = ?4
                  AND COALESCE(
                        json_extract(lifecycle_json, '$.recovery_required'),
                        0
                      ) = 0
             )",
            params![
                self.project_id,
                input.task_id,
                input.change_unit_id,
                input.baseline_ref.as_str(),
            ],
            |row| row.get(0),
        )?;
        if !current_change_unit_matches {
            return Err(StoreError::InvalidInput {
                detail: "advance requires the exact current Change Unit".to_owned(),
            });
        }
        self.apply_selected_gaps(
            &input.task_id,
            &input.shaping_checkpoint_id,
            ShapingDecisionApplicationOwner::AdvanceTask,
            &input.applications,
        )?;
        let unapplied_count: i64 = self.tx.query_row(
            "SELECT COUNT(*) FROM shaping_checkpoint_gaps
              WHERE project_id = ?1
                AND shaping_checkpoint_id = ?2
                AND status <> 'applied'",
            params![self.project_id, input.shaping_checkpoint_id],
            |row| row.get(0),
        )?;
        if unapplied_count != 0 {
            return Err(StoreError::InvalidInput {
                detail: "advance cannot change phase while a shaping gap is unapplied".to_owned(),
            });
        }
        let changed = self.tx.execute(
            "UPDATE tasks
                SET work_phase = 'implementation',
                    lifecycle_phase = 'executing',
                    updated_at = ?6
              WHERE project_id = ?1
                AND task_id = ?2
                AND mode = 'work'
                AND work_phase = 'shaping'
                AND scope_revision = ?3
                AND current_change_unit_id = ?4
                AND json_extract(shaping_summary_json, '$.baseline_ref') = ?5
                AND lifecycle_phase NOT IN ('completed', 'cancelled', 'superseded')",
            params![
                self.project_id,
                input.task_id,
                expected_scope_revision,
                input.change_unit_id,
                input.baseline_ref.as_str(),
                self.committed_at,
            ],
        )?;
        if changed != 1 {
            return Err(StoreError::Conflict {
                entity: "task",
                id: input.task_id.clone(),
                detail: "Task is not eligible for the exact shaping-to-implementation transition"
                    .to_owned(),
            });
        }
        Ok(())
    }

    fn require_exact_current_checkpoint(
        &self,
        task_id: &str,
        shaping_checkpoint_id: &str,
    ) -> StoreResult<()> {
        let matches: bool = self.tx.query_row(
            "SELECT EXISTS (
               SELECT 1 FROM shaping_checkpoints
                WHERE project_id = ?1
                  AND task_id = ?2
                  AND shaping_checkpoint_id = ?3
                  AND readiness <> 'superseded'
             )",
            params![self.project_id, task_id, shaping_checkpoint_id],
            |row| row.get(0),
        )?;
        if matches {
            Ok(())
        } else {
            Err(StoreError::InvalidInput {
                detail: "selected shaping checkpoint is not the exact current checkpoint"
                    .to_owned(),
            })
        }
    }

    fn require_task_scope_revision(&self, task_id: &str, scope_revision: i64) -> StoreResult<()> {
        let matches: bool = self.tx.query_row(
            "SELECT EXISTS (
               SELECT 1 FROM tasks
                WHERE project_id = ?1
                  AND task_id = ?2
                  AND scope_revision = ?3
             )",
            params![self.project_id, task_id, scope_revision],
            |row| row.get(0),
        )?;
        if matches {
            Ok(())
        } else {
            Err(StoreError::InvalidInput {
                detail: "shaping checkpoint scope_revision does not match the current Task"
                    .to_owned(),
            })
        }
    }

    fn apply_selected_gaps(
        &mut self,
        task_id: &str,
        shaping_checkpoint_id: &str,
        owner: ShapingDecisionApplicationOwner,
        applications: &[ShapingGapApplication],
    ) -> StoreResult<()> {
        let mut selected_gap_ids = std::collections::BTreeSet::new();
        let mut selected_resolution_ids = std::collections::BTreeSet::new();
        for application in applications {
            validate_identifier("shaping_gap_id", &application.shaping_gap_id)?;
            validate_identifier(
                "user_action_resolution_id",
                &application.user_action_resolution_id,
            )?;
            if !selected_gap_ids.insert(application.shaping_gap_id.as_str())
                || !selected_resolution_ids.insert(application.user_action_resolution_id.as_str())
            {
                return Err(StoreError::InvalidInput {
                    detail: "shaping applications must contain unique gap and resolution ids"
                        .to_owned(),
                });
            }
            let row = self
                .tx
                .query_row(
                    "SELECT g.gap_kind, g.status, l.user_action_resolution_id,
                            r.basis_status, r.required_for_json
                       FROM shaping_checkpoint_gaps AS g
                       JOIN shaping_checkpoint_user_actions AS l
                         ON l.project_id = g.project_id
                        AND l.shaping_checkpoint_id = g.shaping_checkpoint_id
                        AND l.shaping_gap_id = g.shaping_gap_id
                       JOIN user_action_requests AS r
                         ON r.project_id = l.project_id
                        AND r.user_action_request_id = l.user_action_request_id
                      WHERE g.project_id = ?1
                        AND g.task_id = ?2
                        AND g.shaping_checkpoint_id = ?3
                        AND g.shaping_gap_id = ?4",
                    params![
                        self.project_id,
                        task_id,
                        shaping_checkpoint_id,
                        application.shaping_gap_id,
                    ],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                        ))
                    },
                )
                .optional()?;
            let Some((gap_kind, status, resolution_id, basis_status, required_for_json)) = row
            else {
                return Err(StoreError::InvalidInput {
                    detail: "selected shaping application does not identify one exact linked gap"
                        .to_owned(),
                });
            };
            let gap_kind: ShapingGapKind = decode_owner_closed_value(
                "shaping_checkpoint_gaps",
                &application.shaping_gap_id,
                "gap_kind",
                &gap_kind,
            )?;
            let Some(policy) = gap_kind.decision_policy() else {
                return Err(StoreError::InvalidInput {
                    detail: "a non-user shaping gap cannot be applied by a decision owner"
                        .to_owned(),
                });
            };
            let required_for: Vec<UserActionRequiredFor> = decode_owner_json_text(
                "shaping_checkpoint_gaps",
                &application.shaping_gap_id,
                "required_for_json",
                &required_for_json,
            )?;
            if policy.application_owner != owner
                || status != "resolved"
                || resolution_id.as_deref() != Some(application.user_action_resolution_id.as_str())
                || basis_status != "current"
                || required_for.as_slice() != policy.required_for
            {
                return Err(StoreError::InvalidInput {
                    detail: "selected shaping application does not match its exact semantic owner and resolution"
                        .to_owned(),
                });
            }
            let changed = self.tx.execute(
                "UPDATE shaping_checkpoint_gaps
                    SET status = 'applied'
                  WHERE project_id = ?1
                    AND shaping_checkpoint_id = ?2
                    AND shaping_gap_id = ?3
                    AND status = 'resolved'",
                params![
                    self.project_id,
                    shaping_checkpoint_id,
                    application.shaping_gap_id,
                ],
            )?;
            if changed != 1 {
                return Err(StoreError::Conflict {
                    entity: "shaping_gap",
                    id: application.shaping_gap_id.clone(),
                    detail: "selected shaping gap was already applied or changed".to_owned(),
                });
            }
        }
        Ok(())
    }

    fn require_owner_gaps_applied(
        &self,
        shaping_checkpoint_id: &str,
        owner: ShapingDecisionApplicationOwner,
    ) -> StoreResult<()> {
        let rows = {
            let mut statement = self.tx.prepare(
                "SELECT shaping_gap_id, gap_kind, status
                   FROM shaping_checkpoint_gaps
                  WHERE project_id = ?1
                    AND shaping_checkpoint_id = ?2
                  ORDER BY shaping_gap_id",
            )?;
            let rows = statement
                .query_map(params![self.project_id, shaping_checkpoint_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        for (gap_id, gap_kind, status) in rows {
            let gap_kind: ShapingGapKind = decode_owner_closed_value(
                "shaping_checkpoint_gaps",
                &gap_id,
                "gap_kind",
                &gap_kind,
            )?;
            if gap_kind
                .decision_policy()
                .is_some_and(|policy| policy.application_owner == owner)
                && status != "applied"
            {
                return Err(StoreError::InvalidInput {
                    detail: "every gap owned by the selected shaping application method must be applied exactly"
                        .to_owned(),
                });
            }
        }
        Ok(())
    }

    fn supersede_current_shaping_checkpoint(&mut self, task_id: &str) -> StoreResult<()> {
        self.tx.execute(
            "UPDATE shaping_checkpoints
                SET readiness = 'superseded', superseded_at = ?3
              WHERE project_id = ?1 AND task_id = ?2 AND readiness <> 'superseded'",
            params![self.project_id, task_id, self.committed_at],
        )?;
        Ok(())
    }
}

fn validate_checkpoint_insert(input: &ShapingCheckpointInsert) -> StoreResult<()> {
    validate_identifier("shaping_checkpoint_id", &input.shaping_checkpoint_id)?;
    validate_identifier("task_id", &input.task_id)?;
    validate_nonempty_text("shaping checkpoint summary", &input.summary)?;
    if input.readiness == ShapingCheckpointReadiness::Superseded {
        return Err(StoreError::InvalidInput {
            detail: "a newly recorded shaping checkpoint cannot be superseded".to_owned(),
        });
    }
    if input.readiness == ShapingCheckpointReadiness::Ready
        && (!input.gaps.is_empty()
            || input.baseline_ref.is_none()
            || input
                .implementation_boundary
                .as_ref()
                .is_none_or(|value| value.trim().is_empty()))
    {
        return Err(StoreError::InvalidInput {
            detail: "a ready shaping checkpoint requires a baseline, boundary, and no gaps"
                .to_owned(),
        });
    }
    if input.readiness == ShapingCheckpointReadiness::Blocked && input.gaps.is_empty() {
        return Err(StoreError::InvalidInput {
            detail: "a blocked shaping checkpoint requires at least one typed gap".to_owned(),
        });
    }
    Ok(())
}

fn shaping_checkpoint_where(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    checkpoint_id: Option<&str>,
    current_only: bool,
) -> StoreResult<Option<ShapingCheckpointRecord>> {
    let sql = if current_only {
        format!(
            "SELECT {CHECKPOINT_COLUMNS} FROM shaping_checkpoints
              WHERE project_id = ?1 AND task_id = ?2 AND readiness <> 'superseded'"
        )
    } else {
        format!(
            "SELECT {CHECKPOINT_COLUMNS} FROM shaping_checkpoints
              WHERE project_id = ?1 AND task_id = ?2 AND shaping_checkpoint_id = ?3"
        )
    };
    let raw = if current_only {
        let mut statement = conn.prepare(&sql)?;
        let rows = statement
            .query_map(params![project_id, task_id], raw_checkpoint)?
            .collect::<Result<Vec<_>, _>>()?;
        if rows.len() > 1 {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "a Task has more than one current shaping checkpoint".to_owned(),
            });
        }
        let raw = rows.into_iter().next();
        validate_current_shaping_authority(
            conn,
            project_id,
            task_id,
            raw.as_ref()
                .map(|value| value.shaping_checkpoint_id.as_str()),
        )?;
        raw
    } else {
        conn.query_row(
            &sql,
            params![project_id, task_id, checkpoint_id.unwrap_or_default()],
            raw_checkpoint,
        )
        .optional()?
    };
    raw.map(|raw| decode_checkpoint(conn, raw)).transpose()
}

fn validate_current_shaping_authority(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    current_checkpoint_id: Option<&str>,
) -> StoreResult<()> {
    let detached_count: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM user_action_requests AS r
          WHERE r.project_id = ?1
            AND r.task_id = ?2
            AND r.source_method = 'volicord.record_shaping'
            AND r.basis_status = 'current'
            AND NOT EXISTS (
              SELECT 1
                FROM shaping_checkpoint_user_actions AS applied_link
                JOIN shaping_checkpoint_gaps AS applied_gap
                  ON applied_gap.project_id = applied_link.project_id
                 AND applied_gap.shaping_checkpoint_id = applied_link.shaping_checkpoint_id
                 AND applied_gap.shaping_gap_id = applied_link.shaping_gap_id
               WHERE applied_link.project_id = r.project_id
                 AND applied_link.user_action_request_id = r.user_action_request_id
                 AND applied_gap.status = 'applied'
            )
            AND NOT EXISTS (
              SELECT 1
                FROM shaping_checkpoint_user_actions AS l
               WHERE l.project_id = r.project_id
                 AND l.user_action_request_id = r.user_action_request_id
                 AND (?3 IS NOT NULL AND l.shaping_checkpoint_id = ?3)
            )",
        params![project_id, task_id, current_checkpoint_id],
        |row| row.get(0),
    )?;
    if detached_count != 0 {
        return Err(StoreError::corrupt_owner_state_value(
            "user_action_requests",
            task_id,
            "metadata_json",
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct RawCheckpoint {
    project_id: String,
    shaping_checkpoint_id: String,
    predecessor_shaping_checkpoint_id: Option<String>,
    task_id: String,
    scope_revision: i64,
    baseline_ref: Option<String>,
    summary: String,
    implementation_boundary: Option<String>,
    readiness: String,
    source_refs_json: String,
    evidence_refs_json: String,
    created_at: String,
    superseded_at: Option<String>,
}

fn raw_checkpoint(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawCheckpoint> {
    Ok(RawCheckpoint {
        project_id: row.get(0)?,
        shaping_checkpoint_id: row.get(1)?,
        predecessor_shaping_checkpoint_id: row.get(2)?,
        task_id: row.get(3)?,
        scope_revision: row.get(4)?,
        baseline_ref: row.get(5)?,
        summary: row.get(6)?,
        implementation_boundary: row.get(7)?,
        readiness: row.get(8)?,
        source_refs_json: row.get(9)?,
        evidence_refs_json: row.get(10)?,
        created_at: row.get(11)?,
        superseded_at: row.get(12)?,
    })
}

fn decode_checkpoint(
    conn: &Connection,
    raw: RawCheckpoint,
) -> StoreResult<ShapingCheckpointRecord> {
    let record_ref = raw.shaping_checkpoint_id.clone();
    let scope_revision = u64::try_from(raw.scope_revision).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "shaping_checkpoints",
            record_ref.clone(),
            "scope_revision",
        )
    })?;
    let readiness = decode_owner_closed_value(
        "shaping_checkpoints",
        record_ref.clone(),
        "readiness",
        &raw.readiness,
    )?;
    let created_at = UtcTimestamp::parse(&raw.created_at).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "shaping_checkpoints",
            record_ref.clone(),
            "created_at",
        )
    })?;
    let superseded_at = raw
        .superseded_at
        .as_deref()
        .map(UtcTimestamp::parse)
        .transpose()
        .map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "shaping_checkpoints",
                record_ref.clone(),
                "superseded_at",
            )
        })?;
    if (readiness == ShapingCheckpointReadiness::Superseded) != superseded_at.is_some() {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoints",
            record_ref.clone(),
            "readiness",
        ));
    }
    validate_predecessor_lineage(conn, &raw, &created_at)?;
    let task_scope_revision: i64 = conn.query_row(
        "SELECT scope_revision FROM tasks WHERE project_id = ?1 AND task_id = ?2",
        params![raw.project_id, raw.task_id],
        |row| row.get(0),
    )?;
    if readiness != ShapingCheckpointReadiness::Superseded
        && task_scope_revision != raw.scope_revision
    {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoints",
            record_ref.clone(),
            "scope_revision",
        ));
    }
    let gaps = shaping_gaps(conn, &raw.project_id, &raw.task_id, &record_ref)?;
    if readiness == ShapingCheckpointReadiness::Ready
        && (gaps
            .iter()
            .any(|gap| gap.status == ShapingGapStatus::Current)
            || raw.baseline_ref.is_none()
            || raw.implementation_boundary.is_none())
    {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoints",
            record_ref.clone(),
            "readiness",
        ));
    }
    Ok(ShapingCheckpointRecord {
        project_id: raw.project_id,
        shaping_checkpoint_id: raw.shaping_checkpoint_id,
        predecessor_shaping_checkpoint_id: raw.predecessor_shaping_checkpoint_id,
        task_id: raw.task_id,
        scope_revision,
        baseline_ref: raw.baseline_ref.map(BaselineRef::new),
        summary: raw.summary,
        implementation_boundary: raw.implementation_boundary,
        readiness,
        source_refs: decode_owner_json_text(
            "shaping_checkpoints",
            record_ref.clone(),
            "source_refs_json",
            &raw.source_refs_json,
        )?,
        evidence_refs: decode_owner_json_text(
            "shaping_checkpoints",
            record_ref,
            "evidence_refs_json",
            &raw.evidence_refs_json,
        )?,
        created_at,
        superseded_at,
        gaps,
    })
}

fn validate_predecessor_lineage(
    conn: &Connection,
    raw: &RawCheckpoint,
    created_at: &UtcTimestamp,
) -> StoreResult<()> {
    let Some(predecessor_id) = raw.predecessor_shaping_checkpoint_id.as_deref() else {
        return Ok(());
    };
    if predecessor_id == raw.shaping_checkpoint_id {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoints",
            &raw.shaping_checkpoint_id,
            "predecessor_shaping_checkpoint_id",
        ));
    }
    let predecessor = conn
        .query_row(
            "SELECT task_id, readiness, superseded_at
               FROM shaping_checkpoints
              WHERE project_id = ?1
                AND shaping_checkpoint_id = ?2",
            params![raw.project_id, predecessor_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((predecessor_task_id, predecessor_readiness, predecessor_superseded_at)) = predecessor
    else {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoints",
            &raw.shaping_checkpoint_id,
            "predecessor_shaping_checkpoint_id",
        ));
    };
    let expected_superseded_at = created_at.to_canonical_string();
    if predecessor_task_id != raw.task_id
        || predecessor_readiness != "superseded"
        || predecessor_superseded_at.as_deref() != Some(expected_superseded_at.as_str())
    {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoints",
            &raw.shaping_checkpoint_id,
            "predecessor_shaping_checkpoint_id",
        ));
    }
    Ok(())
}

fn shaping_gaps(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    checkpoint_id: &str,
) -> StoreResult<Vec<ShapingCheckpointGapRecord>> {
    let sql = format!(
        "SELECT {GAP_COLUMNS} FROM shaping_checkpoint_gaps
          WHERE project_id = ?1 AND shaping_checkpoint_id = ?2
          ORDER BY shaping_gap_id"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(params![project_id, checkpoint_id], |row| {
        Ok((
            row.get::<_, String>(2)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, Option<String>>(8)?,
            row.get::<_, Option<String>>(9)?,
        ))
    })?;
    let mut gaps = Vec::new();
    for row in rows {
        let (gap_id, gap_kind, summary, affected_refs_json, status, request_id, action_kind) = row?;
        let decoded_kind: ShapingGapKind = decode_owner_closed_value(
            "shaping_checkpoint_gaps",
            gap_id.clone(),
            "gap_kind",
            &gap_kind,
        )?;
        let decoded_status: ShapingGapStatus = decode_owner_closed_value(
            "shaping_checkpoint_gaps",
            gap_id.clone(),
            "status",
            &status,
        )?;
        let user_action = match (request_id, action_kind) {
            (Some(request_id), Some(action_kind)) => {
                let action_kind: UserActionKind = decode_owner_closed_value(
                    "shaping_checkpoint_gaps",
                    gap_id.clone(),
                    "user_action_kind",
                    &action_kind,
                )?;
                if decoded_kind.user_action_kind() != Some(action_kind) {
                    return Err(StoreError::corrupt_owner_state_value(
                        "shaping_checkpoint_gaps",
                        gap_id,
                        "user_action_kind",
                    ));
                }
                Some(shaping_link(
                    conn,
                    project_id,
                    task_id,
                    checkpoint_id,
                    &request_id,
                    decoded_kind,
                )?)
            }
            (None, None) if !decoded_kind.is_user_owned() => None,
            _ => {
                return Err(StoreError::corrupt_owner_state_value(
                    "shaping_checkpoint_gaps",
                    gap_id,
                    "user_action_request_id",
                ))
            }
        };
        if (decoded_status != ShapingGapStatus::Current)
            != user_action
                .as_ref()
                .is_some_and(|link| link.user_action_resolution_id.is_some())
        {
            return Err(StoreError::corrupt_owner_state_value(
                "shaping_checkpoint_gaps",
                gap_id,
                "status",
            ));
        }
        gaps.push(ShapingCheckpointGapRecord {
            shaping_gap_id: gap_id.clone(),
            gap_kind: decoded_kind,
            summary,
            affected_refs: decode_owner_json_text(
                "shaping_checkpoint_gaps",
                gap_id,
                "affected_refs_json",
                &affected_refs_json,
            )?,
            status: decoded_status,
            user_action,
        });
    }
    Ok(gaps)
}

fn shaping_link(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    checkpoint_id: &str,
    request_id: &str,
    expected_gap_kind: ShapingGapKind,
) -> StoreResult<ShapingCheckpointUserActionRecord> {
    let raw = conn
        .query_row(
            "SELECT l.task_id, l.action_kind, l.user_action_resolution_id,
                    l.linked_at, l.resolved_at, r.required_for_json
               FROM shaping_checkpoint_user_actions AS l
               JOIN user_action_requests AS r
                 ON r.project_id = l.project_id
                AND r.user_action_request_id = l.user_action_request_id
              WHERE l.project_id = ?1
                AND l.shaping_checkpoint_id = ?2
                AND l.user_action_request_id = ?3",
            params![project_id, checkpoint_id, request_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?;
    let Some((
        linked_task_id,
        action_kind,
        resolution_id,
        linked_at,
        resolved_at,
        required_for_json,
    )) = raw
    else {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoint_user_actions",
            request_id.to_owned(),
            "user_action_request_id",
        ));
    };
    let action_kind: UserActionKind = decode_owner_closed_value(
        "shaping_checkpoint_user_actions",
        request_id.to_owned(),
        "action_kind",
        &action_kind,
    )?;
    let Some(policy) = expected_gap_kind.decision_policy() else {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoint_gaps",
            request_id.to_owned(),
            "gap_kind",
        ));
    };
    let required_for: Vec<UserActionRequiredFor> = decode_owner_json_text(
        "shaping_checkpoint_user_actions",
        request_id,
        "required_for_json",
        &required_for_json,
    )?;
    if linked_task_id != task_id
        || action_kind != policy.user_action_kind
        || required_for.as_slice() != policy.required_for
    {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoint_user_actions",
            request_id.to_owned(),
            "task_id",
        ));
    }
    let linked_at = UtcTimestamp::parse(&linked_at).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "shaping_checkpoint_user_actions",
            request_id.to_owned(),
            "linked_at",
        )
    })?;
    let resolved_at = resolved_at
        .as_deref()
        .map(UtcTimestamp::parse)
        .transpose()
        .map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "shaping_checkpoint_user_actions",
                request_id.to_owned(),
                "resolved_at",
            )
        })?;
    if resolution_id.is_some() != resolved_at.is_some() {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoint_user_actions",
            request_id.to_owned(),
            "user_action_resolution_id",
        ));
    }
    Ok(ShapingCheckpointUserActionRecord {
        user_action_request_id: request_id.to_owned(),
        action_kind,
        user_action_resolution_id: resolution_id,
        linked_at,
        resolved_at,
    })
}

#[cfg(test)]
#[path = "shaping_behavior_tests.rs"]
mod behavior_tests;
