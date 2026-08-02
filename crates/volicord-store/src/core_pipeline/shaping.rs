use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::canonical::canonical_json_string;
use volicord_types::ids::{BaselineRef, TaskId};
use volicord_types::schema::{SourceRef, StateRecordRef};
use volicord_types::values::{
    ShapingCheckpointReadiness, ShapingGapKind, ShapingGapStatus, UserActionKind, UtcTimestamp,
};

use super::{facade::CoreProjectStore, mutations::MutationContext, validation::*};
use crate::{StoreError, StoreResult};

const CHECKPOINT_COLUMNS: &str = "
    project_id, shaping_checkpoint_id, task_id, scope_revision, baseline_ref,
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
    RebaseCurrent {
        task_id: String,
        scope_revision: u64,
        baseline_ref: Option<BaselineRef>,
    },
    SupersedeCurrent {
        task_id: String,
    },
}

/// Storage input for one checkpoint and its complete gap/link set.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapingCheckpointInsert {
    pub shaping_checkpoint_id: String,
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
            Self::RebaseCurrent {
                task_id,
                scope_revision,
                baseline_ref,
            } => context.rebase_current_shaping_checkpoint(
                task_id,
                *scope_revision,
                baseline_ref.as_ref(),
            ),
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
        self.supersede_current_shaping_checkpoint(&input.task_id)?;
        let scope_revision =
            i64::try_from(input.scope_revision).map_err(|_| StoreError::InvalidInput {
                detail: "shaping checkpoint scope_revision is too large".to_owned(),
            })?;
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
               project_id, shaping_checkpoint_id, task_id, scope_revision,
               baseline_ref, summary, implementation_boundary, readiness,
               source_refs_json, evidence_refs_json, created_at, superseded_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL)",
            params![
                self.project_id,
                input.shaping_checkpoint_id,
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

    fn rebase_current_shaping_checkpoint(
        &mut self,
        task_id: &str,
        scope_revision: u64,
        baseline_ref: Option<&BaselineRef>,
    ) -> StoreResult<()> {
        let scope_revision =
            i64::try_from(scope_revision).map_err(|_| StoreError::InvalidInput {
                detail: "shaping checkpoint scope_revision is too large".to_owned(),
            })?;
        self.tx.execute(
            "UPDATE shaping_checkpoint_gaps
                SET status = 'applied'
              WHERE project_id = ?1
                AND shaping_checkpoint_id IN (
                  SELECT shaping_checkpoint_id FROM shaping_checkpoints
                   WHERE project_id = ?1 AND task_id = ?2 AND readiness <> 'superseded'
                )
                AND status = 'resolved'",
            params![self.project_id, task_id],
        )?;
        self.tx.execute(
            "UPDATE shaping_checkpoints
                SET scope_revision = ?3, baseline_ref = ?4
              WHERE project_id = ?1 AND task_id = ?2 AND readiness <> 'superseded'",
            params![
                self.project_id,
                task_id,
                scope_revision,
                baseline_ref.map(BaselineRef::as_str),
            ],
        )?;
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
        conn.query_row(&sql, params![project_id, task_id], raw_checkpoint)
            .optional()?
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

#[derive(Debug)]
struct RawCheckpoint {
    project_id: String,
    shaping_checkpoint_id: String,
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
        task_id: row.get(2)?,
        scope_revision: row.get(3)?,
        baseline_ref: row.get(4)?,
        summary: row.get(5)?,
        implementation_boundary: row.get(6)?,
        readiness: row.get(7)?,
        source_refs_json: row.get(8)?,
        evidence_refs_json: row.get(9)?,
        created_at: row.get(10)?,
        superseded_at: row.get(11)?,
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
                    action_kind,
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
        if (decoded_status == ShapingGapStatus::Resolved)
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
    expected_action_kind: UserActionKind,
) -> StoreResult<ShapingCheckpointUserActionRecord> {
    let raw = conn
        .query_row(
            "SELECT task_id, action_kind, user_action_resolution_id, linked_at, resolved_at
               FROM shaping_checkpoint_user_actions
              WHERE project_id = ?1
                AND shaping_checkpoint_id = ?2
                AND user_action_request_id = ?3",
            params![project_id, checkpoint_id, request_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()?;
    let Some((linked_task_id, action_kind, resolution_id, linked_at, resolved_at)) = raw else {
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
    if linked_task_id != task_id || action_kind != expected_action_kind {
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
