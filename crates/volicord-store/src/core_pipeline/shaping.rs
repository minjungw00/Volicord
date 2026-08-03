use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::canonical::canonical_json_string;
use volicord_types::ids::{
    shaping_authority_reauthorization_id, shaping_decision_application_id, BaselineRef,
    ChangeUnitId, ShapingDecisionApplicationId, TaskId, UserActionResolutionId,
};
use volicord_types::schema::{
    PersistedUserActionRequestMetadata, ShapingCheckpointOperation, SourceRef,
    StaleShapingAuthorityAction, StateRecordRef, UserActionResolutionBody,
};
use volicord_types::values::{
    JudgmentKind, JudgmentResolutionOutcome, ShapingAuthorityReauthorizationOutcome,
    ShapingCheckpointReadiness, ShapingDecisionApplicationAuthorityStatus,
    ShapingDecisionApplicationOwner, ShapingGapKind, ShapingGapStatus, TaskLifecyclePhase,
    TaskMode, UserActionBasisStatus, UserActionKind, UserActionOptionAction, UserActionRequiredFor,
    UserActionStatus, UtcTimestamp,
};

use super::{
    facade::CoreProjectStore,
    mutations::MutationContext,
    user_actions::{effective_user_action_records_for_task, StoredUserActionRecordSet},
    validation::*,
};
use crate::{StoreError, StoreResult};

const CHECKPOINT_COLUMNS: &str = "
    project_id, shaping_checkpoint_id, predecessor_shaping_checkpoint_id,
    task_id, scope_revision, baseline_ref,
    summary, implementation_boundary, readiness, source_refs_json,
    evidence_refs_json, created_at, superseded_at";

const GAP_COLUMNS: &str = "
    project_id, shaping_checkpoint_id, shaping_gap_id, task_id, gap_kind,
    summary, affected_refs_json, status, reauthorizes_application_id, user_action_request_id,
    user_action_kind";

const APPLICATION_COLUMNS: &str = "
    application.project_id, application.shaping_decision_application_id, application.task_id,
    application.source_checkpoint_id, application.source_gap_id, application.user_action_request_id,
    application.user_action_resolution_id, application.judgment_kind, application.application_owner,
    application.applied_scope_revision, application.applied_baseline_ref, application.applied_change_unit_id,
    application.applied_at, application.authority_status, application.stale_at,
    application.superseded_at";

/// Shaping-checkpoint mutation applied inside one Core commit transaction.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapingCheckpointMutation {
    Record(Box<ShapingCheckpointInsert>),
    ResolveLinkedGap {
        user_action_request_id: String,
        user_action_resolution_id: String,
        disposition: ShapingGapStatus,
    },
    ApplyScopeAndRebaseCurrent {
        task_id: String,
        shaping_checkpoint_id: String,
        scope_revision: u64,
        baseline_ref: Option<BaselineRef>,
        change_unit_id: Option<String>,
        applications: Vec<ShapingGapApplication>,
    },
    ApplyAdvanceAndTransition(ShapingAdvanceApplication),
    ApplyAdvisorFinalization(ShapingAdvanceApplication),
    SupersedeCurrent {
        task_id: String,
    },
}

/// Exact shaping-gap and UserAction resolution pair selected for application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapingGapApplication {
    pub shaping_decision_application_id: String,
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

#[derive(Debug, Clone, Copy)]
struct ShapingApplicationBasis<'a> {
    scope_revision: u64,
    baseline_ref: Option<&'a BaselineRef>,
    change_unit_id: Option<&'a str>,
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
    pub retired_non_authorizing_request_ids: Vec<String>,
    pub carry_forward_application_ids: Vec<String>,
    pub stale_authority_dispositions: Vec<ShapingStaleAuthorityDisposition>,
    pub gaps: Vec<ShapingCheckpointGapInsert>,
}

/// Store materialization of one exact stale-authority action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapingStaleAuthorityDisposition {
    pub stale_application_id: String,
    pub stale_user_action_request_id: String,
    pub outcome: ShapingAuthorityReauthorizationOutcome,
    pub successor_gap_id: Option<String>,
    pub successor_user_action_request_id: Option<String>,
}

/// Storage input for one typed checkpoint gap.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapingCheckpointGapInsert {
    pub shaping_gap_id: String,
    pub gap_kind: ShapingGapKind,
    pub summary: String,
    pub affected_refs: Vec<StateRecordRef>,
    pub reauthorizes_application_id: Option<String>,
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
    pub applications: Vec<ShapingDecisionApplicationRecord>,
}

/// Strictly decoded durable shaping-decision application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapingDecisionApplicationRecord {
    pub project_id: String,
    pub shaping_decision_application_id: String,
    pub task_id: String,
    pub source_checkpoint_id: String,
    pub source_gap_id: String,
    pub user_action_request_id: String,
    pub user_action_resolution_id: String,
    pub judgment_kind: JudgmentKind,
    pub application_owner: ShapingDecisionApplicationOwner,
    pub applied_scope_revision: u64,
    pub applied_baseline_ref: BaselineRef,
    pub applied_change_unit_id: Option<ChangeUnitId>,
    pub applied_at: UtcTimestamp,
    pub authority_status: ShapingDecisionApplicationAuthorityStatus,
    pub stale_at: Option<UtcTimestamp>,
    pub superseded_at: Option<UtcTimestamp>,
    pub linked_checkpoint_id: Option<String>,
    pub carried_from_checkpoint_id: Option<String>,
}

/// Strictly decoded immutable stale-authority disposition lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapingAuthorityReauthorizationRecord {
    pub project_id: String,
    pub shaping_authority_reauthorization_id: String,
    pub task_id: String,
    pub stale_application_id: String,
    pub stale_user_action_request_id: String,
    pub successor_checkpoint_id: String,
    pub successor_gap_id: Option<String>,
    pub successor_user_action_request_id: Option<String>,
    pub outcome: ShapingAuthorityReauthorizationOutcome,
    pub created_at: UtcTimestamp,
}

/// One exact current-checkpoint gap and its Store-validated UserAction authority.
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentShapingGapDecision {
    pub checkpoint_id: String,
    pub gap: ShapingCheckpointGapRecord,
    pub user_action: StoredUserActionRecordSet,
}

/// One exact application, immutable source gap, and Store-validated UserAction authority.
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentShapingApplicationAuthority {
    pub application: ShapingDecisionApplicationRecord,
    pub source_gap: ShapingCheckpointGapRecord,
    pub user_action: StoredUserActionRecordSet,
}

/// Store-owned effective shaping authority used by current workflow progression.
///
/// Superseded requests and applications are intentionally absent. Complete
/// immutable history remains available from the explicitly historical reads.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CurrentShapingAuthorityGraph {
    pub task_id: String,
    pub current_checkpoint: Option<ShapingCheckpointRecord>,
    pub current_gap_decisions: Vec<CurrentShapingGapDecision>,
    pub current_applications: Vec<CurrentShapingApplicationAuthority>,
    pub stale_recovery_obligations: Vec<CurrentShapingApplicationAuthority>,
    pub current_resolution_ids: BTreeSet<String>,
}

/// Strictly decoded shaping gap.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapingCheckpointGapRecord {
    pub shaping_gap_id: String,
    pub gap_kind: ShapingGapKind,
    pub summary: String,
    pub affected_refs: Vec<StateRecordRef>,
    pub status: ShapingGapStatus,
    pub reauthorizes_application_id: Option<String>,
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
                disposition,
            } => context.resolve_shaping_gap(
                user_action_request_id,
                user_action_resolution_id,
                *disposition,
            ),
            Self::ApplyScopeAndRebaseCurrent {
                task_id,
                shaping_checkpoint_id,
                scope_revision,
                baseline_ref,
                change_unit_id,
                applications,
            } => context.apply_scope_and_rebase_current_shaping_checkpoint(
                task_id,
                shaping_checkpoint_id,
                *scope_revision,
                baseline_ref.as_ref(),
                change_unit_id.as_deref(),
                applications,
            ),
            Self::ApplyAdvanceAndTransition(input) => {
                context.apply_advance_shaping_and_transition(input)
            }
            Self::ApplyAdvisorFinalization(input) => {
                context.apply_advisor_shaping_finalization(input)
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

    /// Reads immutable decision-application history for one Task.
    ///
    /// This audit-oriented read includes current, stale, and superseded records.
    /// Workflow progression must use `current_shaping_authority_graph` instead.
    pub fn shaping_decision_application_history_for_task(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Vec<ShapingDecisionApplicationRecord>> {
        shaping_applications_for_task(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Reads immutable stale-authority disposition history for one Task.
    pub fn shaping_authority_reauthorization_history_for_task(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Vec<ShapingAuthorityReauthorizationRecord>> {
        shaping_reauthorization_history_for_task(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
        )
    }

    /// Derives the exact effective shaping authority graph for current workflow progression.
    pub fn current_shaping_authority_graph(
        &self,
        task_id: &TaskId,
        now: &UtcTimestamp,
    ) -> StoreResult<CurrentShapingAuthorityGraph> {
        let checkpoint = shaping_checkpoint_where(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            None,
            true,
        )?;
        let user_actions = effective_user_action_records_for_task(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            None,
            now,
        )?;
        let applications =
            shaping_applications_for_task(&self.conn, &self.project.project_id, task_id.as_str())?;
        build_current_shaping_authority_graph(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            checkpoint,
            user_actions,
            applications,
        )
    }

    /// Reads one durable decision application by exact identity.
    pub fn shaping_decision_application_record(
        &self,
        task_id: &TaskId,
        application_id: &str,
    ) -> StoreResult<Option<ShapingDecisionApplicationRecord>> {
        shaping_application_by_id(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            application_id,
        )
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
        self.insert_stale_authority_lineage(input)?;
        if let Some(predecessor_id) = predecessor_shaping_checkpoint_id.as_deref() {
            self.insert_carried_application_links(input, predecessor_id)?;
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
                if !current_ids.is_empty() || !input.carry_forward_application_ids.is_empty() {
                    return Err(StoreError::InvalidInput {
                        detail: "create_initial requires no current shaping checkpoint".to_owned(),
                    });
                }
                Ok(None)
            }
            ShapingCheckpointOperation::ReplaceCurrent {
                expected_current_checkpoint_id,
                ..
            } => {
                let expected = expected_current_checkpoint_id.as_str();
                if current_ids.len() != 1 || current_ids[0] != expected {
                    return Err(StoreError::InvalidInput {
                        detail: "replace_current requires the exact current shaping checkpoint"
                            .to_owned(),
                    });
                }
                self.validate_carry_forward_applications(input, expected)?;
                self.retire_checkpoint_user_actions(input, expected)?;
                self.consume_stale_authority(input)?;
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

    fn retire_checkpoint_user_actions(
        &self,
        input: &ShapingCheckpointInsert,
        checkpoint_id: &str,
    ) -> StoreResult<()> {
        let ShapingCheckpointOperation::ReplaceCurrent {
            retired_non_authorizing_request_refs,
            ..
        } = &input.checkpoint_operation
        else {
            return Err(StoreError::InvalidInput {
                detail: "shaping retirement requires replace_current".to_owned(),
            });
        };
        let operation_ids = retired_non_authorizing_request_refs
            .iter()
            .map(|reference| {
                if reference.record_kind
                    != volicord_types::values::StateRecordKind::UserActionRequest
                    || reference.project_id.as_str() != self.project_id
                    || reference.task_id.as_ref().map(TaskId::as_str)
                        != Some(input.task_id.as_str())
                {
                    return Err(StoreError::InvalidInput {
                        detail: "retired request refs must belong to the exact predecessor Task"
                            .to_owned(),
                    });
                }
                Ok(reference.record_id.as_str().to_owned())
            })
            .collect::<StoreResult<std::collections::BTreeSet<_>>>()?;
        let input_ids = input
            .retired_non_authorizing_request_ids
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        if operation_ids.len() != retired_non_authorizing_request_refs.len()
            || input_ids.len() != input.retired_non_authorizing_request_ids.len()
            || operation_ids != input_ids
        {
            return Err(StoreError::InvalidInput {
                detail: "typed retirement refs and aggregate retirement ids must match exactly"
                    .to_owned(),
            });
        }
        let rows = {
            let mut statement = self.tx.prepare(
                "SELECT l.user_action_request_id, g.status, r.basis_status,
                        r.expires_at, resolution.resolution_json
               FROM shaping_checkpoint_user_actions AS l
               JOIN shaping_checkpoint_gaps AS g
                 ON g.project_id = l.project_id
                AND g.shaping_checkpoint_id = l.shaping_checkpoint_id
                AND g.shaping_gap_id = l.shaping_gap_id
               JOIN user_action_requests AS r
                ON r.project_id = l.project_id
                AND r.user_action_request_id = l.user_action_request_id
               LEFT JOIN user_action_resolutions AS resolution
                 ON resolution.project_id = l.project_id
                AND resolution.user_action_request_id = l.user_action_request_id
              WHERE l.project_id = ?1
                AND l.task_id = ?2
                AND l.shaping_checkpoint_id = ?3
                AND g.status <> 'applied'
                AND r.basis_status = 'current'
               ORDER BY l.user_action_request_id",
            )?;
            let rows = statement
                .query_map(
                    params![self.project_id, input.task_id, checkpoint_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?,
                        ))
                    },
                )?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        let mut required_retirements = std::collections::BTreeSet::new();
        for (request_id, status, basis_status, expires_at, resolution_json) in rows {
            if basis_status != "current" {
                return Err(StoreError::SchemaInvariant {
                    database_kind: "project_state",
                    detail: "shaping retirement selected a non-current request basis".to_owned(),
                });
            }
            let permitted = match status.as_str() {
                "rejected" => resolution_json.as_deref().is_some_and(|json| {
                    serde_json::from_str::<UserActionResolutionBody>(json).is_ok_and(|body| {
                        matches!(
                            body,
                            UserActionResolutionBody::Choice {
                                machine_action: UserActionOptionAction::Reject,
                                resolution_outcome: JudgmentResolutionOutcome::Rejected,
                                ..
                            }
                        )
                    })
                }),
                "deferred" => resolution_json.as_deref().is_some_and(|json| {
                    serde_json::from_str::<UserActionResolutionBody>(json).is_ok_and(|body| {
                        matches!(
                            body,
                            UserActionResolutionBody::Choice {
                                machine_action: UserActionOptionAction::Defer,
                                resolution_outcome: JudgmentResolutionOutcome::Deferred,
                                ..
                            }
                        )
                    })
                }),
                "current" => {
                    resolution_json.is_none()
                        && expires_at.as_deref().is_some_and(|expires_at| {
                            UtcTimestamp::parse(expires_at)
                                .is_ok_and(|expires_at| input.created_at >= expires_at)
                        })
                }
                _ => false,
            };
            if !permitted {
                return Err(StoreError::InvalidInput {
                    detail: "only rejected, deferred, or expired shaping requests may be retired"
                        .to_owned(),
                });
            }
            required_retirements.insert(request_id);
        }
        let supplied = input
            .retired_non_authorizing_request_ids
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        if supplied.len() != input.retired_non_authorizing_request_ids.len()
            || supplied != required_retirements
        {
            return Err(StoreError::InvalidInput {
                detail: "retired UserAction requests must exactly match the predecessor's recoverable decisions"
                    .to_owned(),
            });
        }
        for request_id in supplied {
            let changed = self.tx.execute(
                "UPDATE user_action_requests
                    SET basis_status = 'superseded',
                        basis_json = json_set(basis_json, '$.coordinates.compatibility_status', 'superseded')
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2
                    AND basis_status = 'current'",
                params![self.project_id, request_id],
            )?;
            if changed != 1 {
                return Err(StoreError::Conflict {
                    entity: "user_action_request",
                    id: request_id,
                    detail: "exact shaping decision retirement compare-and-swap failed".to_owned(),
                });
            }
        }
        Ok(())
    }

    fn consume_stale_authority(&self, input: &ShapingCheckpointInsert) -> StoreResult<()> {
        let ShapingCheckpointOperation::ReplaceCurrent {
            stale_authority_actions,
            ..
        } = &input.checkpoint_operation
        else {
            return Err(StoreError::InvalidInput {
                detail: "stale authority disposition requires replace_current".to_owned(),
            });
        };
        let mut action_outcomes = BTreeMap::new();
        for action in stale_authority_actions {
            let (reference, outcome) = match action {
                StaleShapingAuthorityAction::Retire {
                    stale_application_ref,
                } => (
                    stale_application_ref,
                    ShapingAuthorityReauthorizationOutcome::Retired,
                ),
                StaleShapingAuthorityAction::Reauthorize {
                    stale_application_ref,
                    ..
                } => (
                    stale_application_ref,
                    ShapingAuthorityReauthorizationOutcome::Reissued,
                ),
            };
            if reference.record_kind
                != volicord_types::values::StateRecordKind::ShapingDecisionApplication
                || reference.project_id.as_str() != self.project_id
                || reference.task_id.as_ref().map(TaskId::as_str) != Some(input.task_id.as_str())
                || action_outcomes
                    .insert(reference.record_id.as_str().to_owned(), outcome)
                    .is_some()
            {
                return Err(StoreError::InvalidInput {
                    detail: "stale authority actions must use unique exact Task application refs"
                        .to_owned(),
                });
            }
        }
        let dispositions = input
            .stale_authority_dispositions
            .iter()
            .map(|disposition| (disposition.stale_application_id.clone(), disposition))
            .collect::<BTreeMap<_, _>>();
        if dispositions.len() != input.stale_authority_dispositions.len()
            || action_outcomes.len() != stale_authority_actions.len()
            || dispositions.keys().collect::<BTreeSet<_>>()
                != action_outcomes.keys().collect::<BTreeSet<_>>()
        {
            return Err(StoreError::InvalidInput {
                detail: "typed stale actions and materialized dispositions must match exactly"
                    .to_owned(),
            });
        }
        let stored_stale = {
            let mut statement = self.tx.prepare(
                "SELECT application.shaping_decision_application_id,
                        application.user_action_request_id,
                        application.judgment_kind,
                        application.application_owner
                   FROM shaping_decision_applications AS application
                   JOIN user_action_requests AS request
                     ON request.project_id = application.project_id
                    AND request.user_action_request_id = application.user_action_request_id
                  WHERE application.project_id = ?1
                    AND application.task_id = ?2
                    AND application.authority_status = 'stale'
                    AND application.stale_at IS NOT NULL
                    AND application.superseded_at IS NULL
                    AND request.basis_status = 'stale'
                    AND json_extract(request.basis_json, '$.coordinates.compatibility_status') = 'stale'",
            )?;
            let rows = statement
                .query_map(params![self.project_id, input.task_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        let stored_ids = stored_stale
            .iter()
            .map(|row| row.0.as_str())
            .collect::<BTreeSet<_>>();
        if stored_ids != action_outcomes.keys().map(String::as_str).collect() {
            return Err(StoreError::InvalidInput {
                detail: "stale authority actions must exactly consume every Task stale application"
                    .to_owned(),
            });
        }
        for (application_id, request_id, judgment_kind_raw, owner_raw) in stored_stale {
            let disposition = dispositions[&application_id];
            if disposition.stale_user_action_request_id != request_id
                || action_outcomes[&application_id] != disposition.outcome
            {
                return Err(StoreError::InvalidInput {
                    detail: "stale authority disposition does not match its exact application"
                        .to_owned(),
                });
            }
            let judgment_kind: JudgmentKind = decode_owner_closed_value(
                "shaping_decision_applications",
                &application_id,
                "judgment_kind",
                &judgment_kind_raw,
            )?;
            let owner: ShapingDecisionApplicationOwner = decode_owner_closed_value(
                "shaping_decision_applications",
                &application_id,
                "application_owner",
                &owner_raw,
            )?;
            match disposition.outcome {
                ShapingAuthorityReauthorizationOutcome::Retired => {
                    if disposition.successor_gap_id.is_some()
                        || disposition.successor_user_action_request_id.is_some()
                    {
                        return Err(StoreError::InvalidInput {
                            detail: "retired stale authority cannot identify a successor request"
                                .to_owned(),
                        });
                    }
                }
                ShapingAuthorityReauthorizationOutcome::Reissued => {
                    let (Some(gap_id), Some(successor_request_id)) = (
                        disposition.successor_gap_id.as_deref(),
                        disposition.successor_user_action_request_id.as_deref(),
                    ) else {
                        return Err(StoreError::InvalidInput {
                            detail: "reissued stale authority requires a successor gap and request"
                                .to_owned(),
                        });
                    };
                    let gap = input
                        .gaps
                        .iter()
                        .find(|gap| gap.shaping_gap_id == gap_id)
                        .ok_or_else(|| StoreError::InvalidInput {
                            detail: "reauthorization successor gap is absent from the checkpoint"
                                .to_owned(),
                        })?;
                    let policy = gap
                        .gap_kind
                        .decision_policy_for_mode(self.task_mode(&input.task_id)?)
                        .ok_or_else(|| StoreError::InvalidInput {
                            detail: "reauthorization successor must be a user-owned gap".to_owned(),
                        })?;
                    if gap.reauthorizes_application_id.as_deref() != Some(application_id.as_str())
                        || gap.gap_kind.judgment_kind() != Some(judgment_kind)
                        || policy.application_owner != owner
                        || gap
                            .user_action
                            .as_ref()
                            .map(|link| link.user_action_request_id.as_str())
                            != Some(successor_request_id)
                        || successor_request_id == request_id
                    {
                        return Err(StoreError::InvalidInput {
                            detail:
                                "reauthorization successor conflicts with stale authority policy"
                                    .to_owned(),
                        });
                    }
                }
            }
            let changed = self.tx.execute(
                "UPDATE shaping_decision_applications
                    SET authority_status = 'superseded', superseded_at = ?3
                  WHERE project_id = ?1
                    AND shaping_decision_application_id = ?2
                    AND authority_status = 'stale'
                    AND stale_at IS NOT NULL
                    AND superseded_at IS NULL",
                params![self.project_id, application_id, self.committed_at],
            )?;
            if changed != 1 {
                return Err(StoreError::Conflict {
                    entity: "shaping_decision_application",
                    id: application_id,
                    detail: "stale authority compare-and-swap failed".to_owned(),
                });
            }
            let changed = self.tx.execute(
                "UPDATE user_action_requests
                    SET basis_status = 'superseded',
                        basis_json = json_set(
                          basis_json,
                          '$.coordinates.compatibility_status',
                          'superseded'
                        )
                  WHERE project_id = ?1
                    AND user_action_request_id = ?2
                    AND basis_status = 'stale'",
                params![self.project_id, request_id],
            )?;
            if changed != 1 {
                return Err(StoreError::Conflict {
                    entity: "user_action_request",
                    id: request_id,
                    detail: "stale UserAction basis compare-and-swap failed".to_owned(),
                });
            }
        }
        Ok(())
    }

    fn insert_stale_authority_lineage(&self, input: &ShapingCheckpointInsert) -> StoreResult<()> {
        for disposition in &input.stale_authority_dispositions {
            let application_id =
                ShapingDecisionApplicationId::new(disposition.stale_application_id.clone());
            let lineage_id =
                shaping_authority_reauthorization_id(&application_id).map_err(|_| {
                    StoreError::InvalidInput {
                        detail: "stale authority lineage identity could not be derived".to_owned(),
                    }
                })?;
            self.tx.execute(
                "INSERT INTO shaping_authority_reauthorizations (
                   project_id, shaping_authority_reauthorization_id, task_id,
                   stale_application_id, stale_user_action_request_id,
                   successor_checkpoint_id, successor_gap_id,
                   successor_user_action_request_id, outcome, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    self.project_id,
                    lineage_id.as_str(),
                    input.task_id,
                    disposition.stale_application_id,
                    disposition.stale_user_action_request_id,
                    input.shaping_checkpoint_id,
                    disposition.successor_gap_id,
                    disposition.successor_user_action_request_id,
                    disposition.outcome.as_str(),
                    self.committed_at,
                ],
            )?;
        }
        Ok(())
    }

    fn validate_carry_forward_applications(
        &self,
        input: &ShapingCheckpointInsert,
        predecessor_checkpoint_id: &str,
    ) -> StoreResult<()> {
        let ShapingCheckpointOperation::ReplaceCurrent {
            carry_forward_application_refs,
            ..
        } = &input.checkpoint_operation
        else {
            return Err(StoreError::InvalidInput {
                detail: "application carry-forward requires replace_current".to_owned(),
            });
        };
        let supplied_ref_ids = carry_forward_application_refs
            .iter()
            .map(|reference| {
                if reference.record_kind
                    != volicord_types::values::StateRecordKind::ShapingDecisionApplication
                    || reference.project_id.as_str() != self.project_id
                    || reference.task_id.as_ref().map(TaskId::as_str)
                        != Some(input.task_id.as_str())
                {
                    return Err(StoreError::InvalidInput {
                        detail:
                            "carried application refs must belong to the exact predecessor Task"
                                .to_owned(),
                    });
                }
                Ok(reference.record_id.as_str().to_owned())
            })
            .collect::<StoreResult<std::collections::BTreeSet<_>>>()?;
        let supplied_ids = input
            .carry_forward_application_ids
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        if supplied_ref_ids.len() != carry_forward_application_refs.len()
            || supplied_ids.len() != input.carry_forward_application_ids.len()
            || supplied_ref_ids != supplied_ids
        {
            return Err(StoreError::InvalidInput {
                detail: "typed carry-forward refs and aggregate application ids must match exactly"
                    .to_owned(),
            });
        }
        let expected_scope_revision =
            i64::try_from(input.scope_revision).map_err(|_| StoreError::InvalidInput {
                detail: "shaping checkpoint scope_revision is too large".to_owned(),
            })?;
        let current_change_unit_id: Option<String> = self.tx.query_row(
            "SELECT current_change_unit_id FROM tasks WHERE project_id = ?1 AND task_id = ?2",
            params![self.project_id, input.task_id],
            |row| row.get(0),
        )?;
        let rows = {
            let mut statement = self.tx.prepare(
                "SELECT application.shaping_decision_application_id,
                        application.judgment_kind,
                        application.application_owner,
                        application.applied_scope_revision,
                        application.applied_baseline_ref,
                        application.applied_change_unit_id,
                        EXISTS (
                          SELECT 1 FROM shaping_checkpoint_applications AS link
                           WHERE link.project_id = application.project_id
                             AND link.task_id = application.task_id
                             AND link.shaping_checkpoint_id = ?3
                             AND link.shaping_decision_application_id = application.shaping_decision_application_id
                        )
                   FROM shaping_decision_applications AS application
                  WHERE application.project_id = ?1
                    AND application.task_id = ?2
                    AND application.authority_status = 'current'
                  ORDER BY application.shaping_decision_application_id",
            )?;
            let rows = statement
                .query_map(
                    params![self.project_id, input.task_id, predecessor_checkpoint_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, Option<String>>(5)?,
                            row.get::<_, bool>(6)?,
                        ))
                    },
                )?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        let task_mode = self.task_mode(&input.task_id)?;
        let successor_judgments = input
            .gaps
            .iter()
            .filter_map(|gap| gap.gap_kind.judgment_kind())
            .collect::<Vec<_>>();
        let mut required = std::collections::BTreeSet::new();
        for (
            application_id,
            judgment_kind,
            owner,
            scope_revision,
            baseline_ref,
            change_unit_id,
            predecessor_linked,
        ) in rows
        {
            let judgment_kind: JudgmentKind = decode_owner_closed_value(
                "shaping_decision_applications",
                &application_id,
                "judgment_kind",
                &judgment_kind,
            )?;
            let owner: ShapingDecisionApplicationOwner = decode_owner_closed_value(
                "shaping_decision_applications",
                &application_id,
                "application_owner",
                &owner,
            )?;
            let policy_owner = match judgment_kind {
                JudgmentKind::ScopeDecision => ShapingDecisionApplicationOwner::UpdateScope,
                JudgmentKind::ProductDecision
                | JudgmentKind::TechnicalDecision
                | JudgmentKind::SensitiveApproval => {
                    if task_mode == TaskMode::Advisor {
                        ShapingDecisionApplicationOwner::RecordShaping
                    } else {
                        ShapingDecisionApplicationOwner::AdvanceTask
                    }
                }
                JudgmentKind::FinalAcceptance
                | JudgmentKind::ResidualRiskAcceptance
                | JudgmentKind::Cancellation => {
                    return Err(StoreError::SchemaInvariant {
                        database_kind: "project_state",
                        detail: "a shaping application has a non-shaping judgment kind".to_owned(),
                    });
                }
            };
            if !predecessor_linked
                || scope_revision != expected_scope_revision
                || Some(baseline_ref.as_str())
                    != input.baseline_ref.as_ref().map(BaselineRef::as_str)
                || change_unit_id != current_change_unit_id
                || owner != policy_owner
            {
                return Err(StoreError::SchemaInvariant {
                    database_kind: "project_state",
                    detail: "a current shaping application is detached or incompatible with successor coordinates"
                        .to_owned(),
                });
            }
            if successor_judgments.contains(&judgment_kind) {
                return Err(StoreError::InvalidInput {
                    detail: "a successor gap conflicts with carried shaping application authority"
                        .to_owned(),
                });
            }
            required.insert(application_id);
        }
        if supplied_ids != required {
            return Err(StoreError::InvalidInput {
                detail:
                    "carry-forward application refs must exactly match current compatible authority"
                        .to_owned(),
            });
        }
        Ok(())
    }

    fn insert_carried_application_links(
        &self,
        input: &ShapingCheckpointInsert,
        predecessor_checkpoint_id: &str,
    ) -> StoreResult<()> {
        for application_id in &input.carry_forward_application_ids {
            self.tx.execute(
                "INSERT INTO shaping_checkpoint_applications (
                   project_id, task_id, shaping_checkpoint_id,
                   shaping_decision_application_id, carried_from_checkpoint_id, linked_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    self.project_id,
                    input.task_id,
                    input.shaping_checkpoint_id,
                    application_id,
                    predecessor_checkpoint_id,
                    self.committed_at,
                ],
            )?;
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
                    FROM shaping_decision_applications AS application
                    JOIN shaping_checkpoint_applications AS application_link
                      ON application_link.project_id = application.project_id
                     AND application_link.shaping_decision_application_id = application.shaping_decision_application_id
                   WHERE application.project_id = r.project_id
                     AND application.user_action_request_id = r.user_action_request_id
                     AND application.authority_status = 'current'
                     AND (?3 IS NOT NULL AND application_link.shaping_checkpoint_id = ?3)
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
        if gap.reauthorizes_application_id.is_some() && !gap.gap_kind.is_user_owned() {
            return Err(StoreError::InvalidInput {
                detail: "only a user-owned shaping gap may reauthorize stale authority".to_owned(),
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
               reauthorizes_application_id, user_action_request_id, user_action_kind
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                self.project_id,
                checkpoint.shaping_checkpoint_id,
                gap.shaping_gap_id,
                checkpoint.task_id,
                gap_kind,
                gap.summary,
                affected_refs_json,
                status,
                gap.reauthorizes_application_id,
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
        disposition: ShapingGapStatus,
    ) -> StoreResult<()> {
        validate_identifier("user_action_request_id", user_action_request_id)?;
        validate_identifier("user_action_resolution_id", user_action_resolution_id)?;
        if !matches!(
            disposition,
            ShapingGapStatus::Accepted | ShapingGapStatus::Rejected | ShapingGapStatus::Deferred
        ) {
            return Err(StoreError::InvalidInput {
                detail: "a shaping resolution requires an exact terminal decision disposition"
                    .to_owned(),
            });
        }
        let resolution_json: String = self.tx.query_row(
            "SELECT resolution_json FROM user_action_resolutions
              WHERE project_id = ?1
                AND user_action_request_id = ?2
                AND user_action_resolution_id = ?3",
            params![
                self.project_id,
                user_action_request_id,
                user_action_resolution_id
            ],
            |row| row.get(0),
        )?;
        let resolution: UserActionResolutionBody =
            serde_json::from_str(&resolution_json).map_err(|_| StoreError::InvalidInput {
                detail: "shaping resolution body is not canonical".to_owned(),
            })?;
        let expected_disposition = match resolution {
            UserActionResolutionBody::Choice {
                machine_action: UserActionOptionAction::Accept,
                resolution_outcome: JudgmentResolutionOutcome::Accepted,
                ..
            } => ShapingGapStatus::Accepted,
            UserActionResolutionBody::Choice {
                machine_action: UserActionOptionAction::Reject,
                resolution_outcome: JudgmentResolutionOutcome::Rejected,
                ..
            } => ShapingGapStatus::Rejected,
            UserActionResolutionBody::Choice {
                machine_action: UserActionOptionAction::Defer,
                resolution_outcome: JudgmentResolutionOutcome::Deferred,
                ..
            } => ShapingGapStatus::Deferred,
            _ => {
                return Err(StoreError::InvalidInput {
                    detail: "shaping resolution action and outcome are not compatible".to_owned(),
                });
            }
        };
        if disposition != expected_disposition {
            return Err(StoreError::InvalidInput {
                detail: "shaping disposition does not match the immutable resolution outcome"
                    .to_owned(),
            });
        }
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
                SET status = ?4
              WHERE project_id = ?1
                AND shaping_checkpoint_id = ?2
                AND shaping_gap_id = ?3",
            params![self.project_id, checkpoint_id, gap_id, disposition.as_str()],
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
        change_unit_id: Option<&str>,
        applications: &[ShapingGapApplication],
    ) -> StoreResult<()> {
        validate_identifier("task_id", task_id)?;
        validate_identifier("shaping_checkpoint_id", shaping_checkpoint_id)?;
        let stored_scope_revision =
            i64::try_from(scope_revision).map_err(|_| StoreError::InvalidInput {
                detail: "shaping checkpoint scope_revision is too large".to_owned(),
            })?;
        self.require_exact_current_checkpoint(task_id, shaping_checkpoint_id)?;
        self.require_task_scope_revision(task_id, stored_scope_revision)?;
        self.invalidate_incompatible_current_applications(
            task_id,
            scope_revision,
            baseline_ref,
            change_unit_id,
        )?;
        self.apply_selected_gaps(
            task_id,
            shaping_checkpoint_id,
            ShapingDecisionApplicationOwner::UpdateScope,
            ShapingApplicationBasis {
                scope_revision,
                baseline_ref,
                change_unit_id,
            },
            applications,
        )?;
        self.require_owner_gaps_applied(
            task_id,
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
            ShapingApplicationBasis {
                scope_revision: input.scope_revision,
                baseline_ref: Some(&input.baseline_ref),
                change_unit_id: Some(&input.change_unit_id),
            },
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

    fn apply_advisor_shaping_finalization(
        &mut self,
        input: &ShapingAdvanceApplication,
    ) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("shaping_checkpoint_id", &input.shaping_checkpoint_id)?;
        validate_identifier("change_unit_id", &input.change_unit_id)?;
        self.require_exact_current_checkpoint(&input.task_id, &input.shaping_checkpoint_id)?;
        let expected_scope_revision =
            i64::try_from(input.scope_revision).map_err(|_| StoreError::InvalidInput {
                detail: "advisor finalization scope_revision is too large".to_owned(),
            })?;
        let basis_matches: bool = self.tx.query_row(
            "SELECT EXISTS (
               SELECT 1
                 FROM tasks AS t
                 JOIN shaping_checkpoints AS s
                   ON s.project_id = t.project_id AND s.task_id = t.task_id
                 JOIN change_units AS c
                   ON c.project_id = t.project_id AND c.task_id = t.task_id
                WHERE t.project_id = ?1
                  AND t.task_id = ?2
                  AND t.mode = 'advisor'
                  AND t.work_phase = 'shaping'
                  AND t.scope_revision = ?4
                  AND t.current_change_unit_id = ?5
                  AND json_extract(t.shaping_summary_json, '$.baseline_ref') = ?6
                  AND s.shaping_checkpoint_id = ?3
                  AND s.readiness = 'ready'
                  AND s.scope_revision = ?4
                  AND s.baseline_ref = ?6
                  AND c.change_unit_id = ?5
                  AND c.status = 'active'
                  AND c.is_current = 1
                  AND json_extract(c.write_basis_json, '$.baseline_ref') = ?6
                  AND COALESCE(json_extract(c.lifecycle_json, '$.recovery_required'), 0) = 0
             )",
            params![
                self.project_id,
                input.task_id,
                input.shaping_checkpoint_id,
                expected_scope_revision,
                input.change_unit_id,
                input.baseline_ref.as_str(),
            ],
            |row| row.get(0),
        )?;
        if !basis_matches {
            return Err(StoreError::InvalidInput {
                detail: "advisor finalization basis is not exact, current, and ready".to_owned(),
            });
        }
        self.apply_selected_gaps(
            &input.task_id,
            &input.shaping_checkpoint_id,
            ShapingDecisionApplicationOwner::RecordShaping,
            ShapingApplicationBasis {
                scope_revision: input.scope_revision,
                baseline_ref: Some(&input.baseline_ref),
                change_unit_id: Some(&input.change_unit_id),
            },
            &input.applications,
        )?;
        self.require_owner_gaps_applied(
            &input.task_id,
            &input.shaping_checkpoint_id,
            ShapingDecisionApplicationOwner::RecordShaping,
        )?;
        let current_count: i64 = self.tx.query_row(
            "SELECT COUNT(*) FROM shaping_checkpoint_gaps
              WHERE project_id = ?1 AND shaping_checkpoint_id = ?2 AND status = 'current'",
            params![self.project_id, input.shaping_checkpoint_id],
            |row| row.get(0),
        )?;
        if current_count != 0 {
            return Err(StoreError::InvalidInput {
                detail: "advisor finalization cannot retain an unresolved shaping gap".to_owned(),
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
        basis: ShapingApplicationBasis<'_>,
        applications: &[ShapingGapApplication],
    ) -> StoreResult<()> {
        let task_mode = self.task_mode(task_id)?;
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
                    "SELECT g.gap_kind, g.status, l.user_action_request_id,
                            l.user_action_resolution_id, r.basis_status, r.required_for_json
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
                            row.get::<_, String>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, String>(5)?,
                        ))
                    },
                )
                .optional()?;
            let Some((
                gap_kind,
                status,
                request_id,
                resolution_id,
                basis_status,
                required_for_json,
            )) = row
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
            let Some(policy) = gap_kind.decision_policy_for_mode(task_mode) else {
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
                || status != "accepted"
                || resolution_id.as_deref() != Some(application.user_action_resolution_id.as_str())
                || basis_status != "current"
                || required_for.as_slice() != policy.required_for
            {
                return Err(StoreError::InvalidInput {
                    detail: "selected shaping application does not match its exact semantic owner and resolution"
                        .to_owned(),
                });
            }
            let judgment_kind =
                gap_kind
                    .judgment_kind()
                    .ok_or_else(|| StoreError::InvalidInput {
                        detail: "a shaping application requires a judgment kind".to_owned(),
                    })?;
            let baseline_ref = basis.baseline_ref.ok_or_else(|| StoreError::InvalidInput {
                detail: "a shaping application requires a current baseline".to_owned(),
            })?;
            let expected_application_id = shaping_decision_application_id(
                &UserActionResolutionId::new(&application.user_action_resolution_id),
                owner,
            )
            .map_err(|_| StoreError::InvalidInput {
                detail: "shaping application identity could not be derived".to_owned(),
            })?;
            if application.shaping_decision_application_id != expected_application_id.as_str() {
                return Err(StoreError::InvalidInput {
                    detail: "shaping application identity does not match its resolution and owner"
                        .to_owned(),
                });
            }
            self.tx.execute(
                "INSERT INTO shaping_decision_applications (
                   project_id, shaping_decision_application_id, task_id,
                   source_checkpoint_id, source_gap_id, user_action_request_id,
                   user_action_resolution_id, judgment_kind, application_owner,
                   applied_scope_revision, applied_baseline_ref, applied_change_unit_id,
                   applied_at, authority_status, stale_at, superseded_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'current', NULL, NULL)",
                params![
                    self.project_id,
                    application.shaping_decision_application_id,
                    task_id,
                    shaping_checkpoint_id,
                    application.shaping_gap_id,
                    request_id,
                    application.user_action_resolution_id,
                    encode_closed_value("judgment_kind", &judgment_kind)?,
                    encode_closed_value("application_owner", &owner)?,
                    i64::try_from(basis.scope_revision).map_err(|_| StoreError::InvalidInput {
                        detail: "shaping application scope revision is too large".to_owned(),
                    })?,
                    baseline_ref.as_str(),
                    basis.change_unit_id,
                    self.committed_at,
                ],
            )?;
            self.tx.execute(
                "INSERT INTO shaping_checkpoint_applications (
                   project_id, task_id, shaping_checkpoint_id,
                   shaping_decision_application_id, carried_from_checkpoint_id, linked_at
                 ) VALUES (?1, ?2, ?3, ?4, NULL, ?5)",
                params![
                    self.project_id,
                    task_id,
                    shaping_checkpoint_id,
                    application.shaping_decision_application_id,
                    self.committed_at,
                ],
            )?;
            let changed = self.tx.execute(
                "UPDATE shaping_checkpoint_gaps
                    SET status = 'applied'
                  WHERE project_id = ?1
                    AND shaping_checkpoint_id = ?2
                    AND shaping_gap_id = ?3
                    AND status = 'accepted'",
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
        task_id: &str,
        shaping_checkpoint_id: &str,
        owner: ShapingDecisionApplicationOwner,
    ) -> StoreResult<()> {
        let task_mode = self.task_mode(task_id)?;
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
                .decision_policy_for_mode(task_mode)
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

    fn task_mode(&self, task_id: &str) -> StoreResult<TaskMode> {
        let raw: String = self.tx.query_row(
            "SELECT mode FROM tasks WHERE project_id = ?1 AND task_id = ?2",
            params![self.project_id, task_id],
            |row| row.get(0),
        )?;
        decode_owner_closed_value("tasks", task_id, "mode", &raw)
    }

    fn invalidate_incompatible_current_applications(
        &mut self,
        task_id: &str,
        scope_revision: u64,
        baseline_ref: Option<&BaselineRef>,
        change_unit_id: Option<&str>,
    ) -> StoreResult<()> {
        let scope_revision =
            i64::try_from(scope_revision).map_err(|_| StoreError::InvalidInput {
                detail: "shaping application scope revision is too large".to_owned(),
            })?;
        let rows = {
            let mut statement = self.tx.prepare(
                "SELECT shaping_decision_application_id, user_action_request_id
                   FROM shaping_decision_applications
                  WHERE project_id = ?1
                    AND task_id = ?2
                    AND authority_status = 'current'
                    AND (
                      applied_scope_revision <> ?3
                      OR applied_baseline_ref IS NOT ?4
                      OR applied_change_unit_id IS NOT ?5
                    )",
            )?;
            let rows = statement
                .query_map(
                    params![
                        self.project_id,
                        task_id,
                        scope_revision,
                        baseline_ref.map(BaselineRef::as_str),
                        change_unit_id,
                    ],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        self.invalidate_application_rows(rows, ShapingDecisionApplicationAuthorityStatus::Stale)
    }

    fn invalidate_all_current_applications(
        &mut self,
        task_id: &str,
        status: ShapingDecisionApplicationAuthorityStatus,
    ) -> StoreResult<()> {
        let rows = {
            let mut statement = self.tx.prepare(
                "SELECT shaping_decision_application_id, user_action_request_id
                   FROM shaping_decision_applications
                  WHERE project_id = ?1
                    AND task_id = ?2
                    AND authority_status = 'current'",
            )?;
            let rows = statement
                .query_map(params![self.project_id, task_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        self.invalidate_application_rows(rows, status)
    }

    fn invalidate_application_rows(
        &mut self,
        rows: Vec<(String, String)>,
        status: ShapingDecisionApplicationAuthorityStatus,
    ) -> StoreResult<()> {
        if status == ShapingDecisionApplicationAuthorityStatus::Current {
            return Err(StoreError::InvalidInput {
                detail: "application invalidation requires a non-current status".to_owned(),
            });
        }
        for (application_id, request_id) in rows {
            let (stale_at, superseded_at) = match status {
                ShapingDecisionApplicationAuthorityStatus::Stale => (Some(self.committed_at), None),
                ShapingDecisionApplicationAuthorityStatus::Superseded => {
                    (None, Some(self.committed_at))
                }
                ShapingDecisionApplicationAuthorityStatus::Current => {
                    return Err(StoreError::InvalidInput {
                        detail: "application invalidation requires a non-current status".to_owned(),
                    })
                }
            };
            let changed = self.tx.execute(
                "UPDATE shaping_decision_applications
                    SET authority_status = ?3, stale_at = ?4, superseded_at = ?5
                  WHERE project_id = ?1
                    AND shaping_decision_application_id = ?2
                    AND authority_status = 'current'",
                params![
                    self.project_id,
                    application_id,
                    status.as_str(),
                    stale_at,
                    superseded_at,
                ],
            )?;
            if changed != 1 {
                return Err(StoreError::Conflict {
                    entity: "shaping_decision_application",
                    id: application_id,
                    detail: "application authority invalidation compare-and-swap failed".to_owned(),
                });
            }
            let request_changed = if status == ShapingDecisionApplicationAuthorityStatus::Superseded
            {
                self.tx.execute(
                    "UPDATE user_action_requests
                        SET basis_status = 'superseded',
                            basis_json = json_set(
                              basis_json,
                              '$.coordinates.compatibility_status',
                              'superseded'
                            )
                      WHERE project_id = ?1
                        AND user_action_request_id = ?2
                        AND basis_status IN ('current', 'stale')",
                    params![self.project_id, request_id],
                )?
            } else {
                self.tx.execute(
                    "UPDATE user_action_requests
                        SET basis_status = 'stale',
                            basis_json = json_set(
                              basis_json,
                              '$.coordinates.compatibility_status',
                              'stale'
                            )
                      WHERE project_id = ?1
                        AND user_action_request_id = ?2
                        AND basis_status = 'current'",
                    params![self.project_id, request_id],
                )?
            };
            if request_changed != 1 {
                let expected_status = status.as_str();
                let already_invalidated: bool = self.tx.query_row(
                    "SELECT EXISTS (
                       SELECT 1 FROM user_action_requests
                        WHERE project_id = ?1
                          AND user_action_request_id = ?2
                          AND basis_status = ?3
                          AND json_extract(
                            basis_json,
                            '$.coordinates.compatibility_status'
                          ) = ?3
                     )",
                    params![self.project_id, request_id, expected_status],
                    |row| row.get(0),
                )?;
                if !already_invalidated {
                    return Err(StoreError::Conflict {
                        entity: "user_action_request",
                        id: request_id,
                        detail: "application request-basis invalidation compare-and-swap failed"
                            .to_owned(),
                    });
                }
            }
        }
        Ok(())
    }

    pub(super) fn supersede_task_shaping_applications(&mut self, task_id: &str) -> StoreResult<()> {
        self.invalidate_all_current_applications(
            task_id,
            ShapingDecisionApplicationAuthorityStatus::Superseded,
        )
    }

    fn supersede_current_shaping_checkpoint(&mut self, task_id: &str) -> StoreResult<()> {
        self.invalidate_all_current_applications(
            task_id,
            ShapingDecisionApplicationAuthorityStatus::Superseded,
        )?;
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
    if matches!(
        input.checkpoint_operation,
        ShapingCheckpointOperation::CreateInitial
    ) && (!input.retired_non_authorizing_request_ids.is_empty()
        || !input.carry_forward_application_ids.is_empty()
        || !input.stale_authority_dispositions.is_empty())
    {
        return Err(StoreError::InvalidInput {
            detail: "create_initial cannot retire requests or carry applications".to_owned(),
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
    let detached_application_count: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM shaping_decision_applications AS application
          WHERE application.project_id = ?1
            AND application.task_id = ?2
            AND application.authority_status = 'current'
            AND NOT EXISTS (
              SELECT 1 FROM shaping_checkpoint_applications AS link
               WHERE link.project_id = application.project_id
                 AND link.task_id = application.task_id
                 AND link.shaping_decision_application_id = application.shaping_decision_application_id
                 AND (?3 IS NOT NULL AND link.shaping_checkpoint_id = ?3)
            )",
        params![project_id, task_id, current_checkpoint_id],
        |row| row.get(0),
    )?;
    if detached_application_count != 0 {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_decision_applications",
            task_id,
            "authority_status",
        ));
    }
    let detached_count: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM user_action_requests AS r
          WHERE r.project_id = ?1
            AND r.task_id = ?2
            AND r.source_method = 'volicord.record_shaping'
            AND r.basis_status = 'current'
            AND NOT EXISTS (
              SELECT 1
                FROM shaping_decision_applications AS application
                JOIN shaping_checkpoint_applications AS application_link
                  ON application_link.project_id = application.project_id
                 AND application_link.shaping_decision_application_id = application.shaping_decision_application_id
               WHERE application.project_id = r.project_id
                 AND application.user_action_request_id = r.user_action_request_id
                 AND application.authority_status = 'current'
                 AND (?3 IS NOT NULL AND application_link.shaping_checkpoint_id = ?3)
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
    let (task_scope_revision, task_mode_raw): (i64, String) = conn.query_row(
        "SELECT scope_revision, mode FROM tasks WHERE project_id = ?1 AND task_id = ?2",
        params![raw.project_id, raw.task_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let task_mode: TaskMode =
        decode_owner_closed_value("tasks", &raw.task_id, "mode", &task_mode_raw)?;
    if readiness != ShapingCheckpointReadiness::Superseded
        && task_scope_revision != raw.scope_revision
    {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoints",
            record_ref.clone(),
            "scope_revision",
        ));
    }
    let gaps = shaping_gaps(conn, &raw.project_id, &raw.task_id, &record_ref, task_mode)?;
    let applications =
        shaping_applications_for_checkpoint(conn, &raw.project_id, &raw.task_id, &record_ref)?;
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
        applications,
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

#[derive(Debug)]
struct RawShapingApplication {
    project_id: String,
    application_id: String,
    task_id: String,
    source_checkpoint_id: String,
    source_gap_id: String,
    request_id: String,
    resolution_id: String,
    judgment_kind: String,
    application_owner: String,
    applied_scope_revision: i64,
    applied_baseline_ref: String,
    applied_change_unit_id: Option<String>,
    applied_at: String,
    authority_status: String,
    stale_at: Option<String>,
    superseded_at: Option<String>,
    linked_checkpoint_id: Option<String>,
    carried_from_checkpoint_id: Option<String>,
}

fn raw_shaping_application(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawShapingApplication> {
    Ok(RawShapingApplication {
        project_id: row.get(0)?,
        application_id: row.get(1)?,
        task_id: row.get(2)?,
        source_checkpoint_id: row.get(3)?,
        source_gap_id: row.get(4)?,
        request_id: row.get(5)?,
        resolution_id: row.get(6)?,
        judgment_kind: row.get(7)?,
        application_owner: row.get(8)?,
        applied_scope_revision: row.get(9)?,
        applied_baseline_ref: row.get(10)?,
        applied_change_unit_id: row.get(11)?,
        applied_at: row.get(12)?,
        authority_status: row.get(13)?,
        stale_at: row.get(14)?,
        superseded_at: row.get(15)?,
        linked_checkpoint_id: row.get(16)?,
        carried_from_checkpoint_id: row.get(17)?,
    })
}

fn shaping_applications_for_task(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<ShapingDecisionApplicationRecord>> {
    let sql = format!(
        "SELECT {APPLICATION_COLUMNS}, current_link.shaping_checkpoint_id,
                current_link.carried_from_checkpoint_id
           FROM shaping_decision_applications AS application
           LEFT JOIN shaping_checkpoint_applications AS current_link
             ON current_link.project_id = application.project_id
            AND current_link.task_id = application.task_id
            AND current_link.shaping_decision_application_id = application.shaping_decision_application_id
            AND EXISTS (
              SELECT 1 FROM shaping_checkpoints AS current_checkpoint
               WHERE current_checkpoint.project_id = current_link.project_id
                 AND current_checkpoint.task_id = current_link.task_id
                 AND current_checkpoint.shaping_checkpoint_id = current_link.shaping_checkpoint_id
                 AND current_checkpoint.readiness <> 'superseded'
            )
          WHERE application.project_id = ?1 AND application.task_id = ?2
          ORDER BY application.shaping_decision_application_id"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(params![project_id, task_id], raw_shaping_application)?;
    rows.collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|raw| decode_shaping_application(conn, raw))
        .collect()
}

fn build_current_shaping_authority_graph(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    checkpoint: Option<ShapingCheckpointRecord>,
    user_actions: Vec<StoredUserActionRecordSet>,
    applications: Vec<ShapingDecisionApplicationRecord>,
) -> StoreResult<CurrentShapingAuthorityGraph> {
    let (
        task_scope_revision,
        task_baseline_ref,
        task_change_unit_id,
        task_mode_raw,
        task_lifecycle_raw,
        task_closed_at,
    ): (
        i64,
        Option<String>,
        Option<String>,
        String,
        String,
        Option<String>,
    ) = conn.query_row(
        "SELECT scope_revision,
                json_extract(shaping_summary_json, '$.baseline_ref'),
                current_change_unit_id,
                mode,
                lifecycle_phase,
                closed_at
           FROM tasks
          WHERE project_id = ?1 AND task_id = ?2",
        params![project_id, task_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        },
    )?;
    let task_scope_revision = u64::try_from(task_scope_revision)
        .map_err(|_| StoreError::corrupt_owner_state_value("tasks", task_id, "scope_revision"))?;
    let task_mode = decode_owner_closed_value("tasks", task_id, "mode", &task_mode_raw)?;
    let task_lifecycle: TaskLifecyclePhase =
        decode_owner_closed_value("tasks", task_id, "lifecycle_phase", &task_lifecycle_raw)?;
    let task_closed_at = task_closed_at
        .as_deref()
        .map(UtcTimestamp::parse)
        .transpose()
        .map_err(|_| StoreError::corrupt_owner_state_value("tasks", task_id, "closed_at"))?;
    let task_is_terminal = matches!(
        task_lifecycle,
        TaskLifecyclePhase::Completed
            | TaskLifecyclePhase::Cancelled
            | TaskLifecyclePhase::Superseded
    );
    if task_is_terminal != task_closed_at.is_some() {
        return Err(StoreError::corrupt_owner_state_value(
            "tasks",
            task_id,
            "closed_at",
        ));
    }
    if checkpoint.as_ref().is_some_and(|checkpoint| {
        checkpoint.project_id != project_id || checkpoint.task_id != task_id
    }) {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoints",
            task_id,
            "task_id",
        ));
    }

    let user_actions = user_actions
        .into_iter()
        .map(|record| (record.request().user_action_request_id().to_owned(), record))
        .collect::<BTreeMap<_, _>>();
    let mut graph = CurrentShapingAuthorityGraph {
        task_id: task_id.to_owned(),
        current_checkpoint: if !task_is_terminal {
            checkpoint.clone()
        } else {
            None
        },
        ..Default::default()
    };
    let mut represented_current_requests = BTreeSet::new();

    if !task_is_terminal {
        if let Some(checkpoint) = checkpoint.as_ref() {
            for gap in checkpoint
                .gaps
                .iter()
                .filter(|gap| gap.user_action.is_some())
            {
                let link = gap.user_action.as_ref().ok_or_else(|| {
                    StoreError::corrupt_owner_state_value(
                        "shaping_checkpoint_gaps",
                        &gap.shaping_gap_id,
                        "user_action_request_id",
                    )
                })?;
                let record = user_actions
                    .get(&link.user_action_request_id)
                    .ok_or_else(|| {
                        StoreError::corrupt_owner_state_value(
                            "shaping_checkpoint_user_actions",
                            &link.user_action_request_id,
                            "user_action_request_id",
                        )
                    })?;
                let request = record.request();
                let coordinates = request.basis().coordinates();
                let metadata_matches = matches!(
                    request.metadata(),
                    PersistedUserActionRequestMetadata::Shaping(metadata)
                        if metadata.shaping_checkpoint_id.as_str()
                            == checkpoint.shaping_checkpoint_id
                            && metadata.shaping_gap_id.as_str() == gap.shaping_gap_id
                            && metadata.reauthorizes_application_id.as_ref().map(|id| id.as_str())
                                == gap.reauthorizes_application_id.as_deref()
                );
                if request.project_id() != project_id
                    || request.task_id() != task_id
                    || coordinates.task_id.as_str() != task_id
                    || !metadata_matches
                {
                    return Err(StoreError::corrupt_owner_state_value(
                        "user_action_requests",
                        request.user_action_request_id(),
                        "metadata_json",
                    ));
                }
                match request.basis_status() {
                    UserActionBasisStatus::Current => {
                        if request.basis().compatibility_status() != UserActionBasisStatus::Current
                            || coordinates.scope_revision != task_scope_revision
                            || coordinates.baseline_ref.as_ref().map(BaselineRef::as_str)
                                != task_baseline_ref.as_deref()
                            || coordinates
                                .change_unit_id
                                .as_ref()
                                .map(ChangeUnitId::as_str)
                                != task_change_unit_id.as_deref()
                        {
                            return Err(StoreError::corrupt_owner_state_value(
                                "user_action_requests",
                                request.user_action_request_id(),
                                "basis_json",
                            ));
                        }
                        represented_current_requests
                            .insert(request.user_action_request_id().to_owned());
                        graph.current_gap_decisions.push(CurrentShapingGapDecision {
                            checkpoint_id: checkpoint.shaping_checkpoint_id.clone(),
                            gap: gap.clone(),
                            user_action: record.clone(),
                        });
                    }
                    UserActionBasisStatus::Stale => {}
                    UserActionBasisStatus::Superseded => {
                        return Err(StoreError::corrupt_owner_state_value(
                            "shaping_checkpoint_user_actions",
                            request.user_action_request_id(),
                            "user_action_request_id",
                        ));
                    }
                }
            }
        }
    }

    let mut current_application_keys = BTreeSet::new();
    let mut stale_request_ids = BTreeSet::new();
    for application in applications {
        let record = user_actions
            .get(&application.user_action_request_id)
            .ok_or_else(|| {
                StoreError::corrupt_owner_state_value(
                    "shaping_decision_applications",
                    &application.shaping_decision_application_id,
                    "user_action_request_id",
                )
            })?;
        let request = record.request();
        let resolution_matches = record.resolution().is_some_and(|resolution| {
            resolution.user_action_resolution_id() == application.user_action_resolution_id
        });
        if request.project_id() != project_id || request.task_id() != task_id || !resolution_matches
        {
            return Err(StoreError::corrupt_owner_state_value(
                "shaping_decision_applications",
                &application.shaping_decision_application_id,
                "user_action_request_id",
            ));
        }
        let source_checkpoint = shaping_checkpoint_where(
            conn,
            project_id,
            task_id,
            Some(&application.source_checkpoint_id),
            false,
        )?
        .ok_or_else(|| {
            StoreError::corrupt_owner_state_value(
                "shaping_decision_applications",
                &application.shaping_decision_application_id,
                "source_checkpoint_id",
            )
        })?;
        let source_gap = source_checkpoint
            .gaps
            .into_iter()
            .find(|gap| gap.shaping_gap_id == application.source_gap_id)
            .ok_or_else(|| {
                StoreError::corrupt_owner_state_value(
                    "shaping_decision_applications",
                    &application.shaping_decision_application_id,
                    "source_gap_id",
                )
            })?;
        let policy_matches = source_gap
            .gap_kind
            .decision_policy_for_mode(task_mode)
            .is_some_and(|policy| {
                policy.application_owner == application.application_owner
                    && policy.user_action_kind == request.action_kind()
                    && policy.required_for == request.required_for()
                    && source_gap.gap_kind.judgment_kind() == Some(application.judgment_kind)
            });
        if !policy_matches {
            return Err(StoreError::corrupt_owner_state_value(
                "shaping_decision_applications",
                &application.shaping_decision_application_id,
                "application_owner",
            ));
        }
        let authority = CurrentShapingApplicationAuthority {
            application: application.clone(),
            source_gap,
            user_action: record.clone(),
        };
        match application.authority_status {
            ShapingDecisionApplicationAuthorityStatus::Current => {
                if task_is_terminal {
                    return Err(StoreError::corrupt_owner_state_value(
                        "shaping_decision_applications",
                        &application.shaping_decision_application_id,
                        "authority_status",
                    ));
                }
                let current_checkpoint_id = checkpoint
                    .as_ref()
                    .map(|checkpoint| checkpoint.shaping_checkpoint_id.as_str());
                if application.linked_checkpoint_id.as_deref() != current_checkpoint_id
                    || request.basis_status() != UserActionBasisStatus::Current
                    || request.basis().compatibility_status() != UserActionBasisStatus::Current
                    || record.status() != UserActionStatus::Resolved
                {
                    return Err(StoreError::corrupt_owner_state_value(
                        "shaping_decision_applications",
                        &application.shaping_decision_application_id,
                        "authority_status",
                    ));
                }
                let key = (
                    application.user_action_request_id.clone(),
                    application.user_action_resolution_id.clone(),
                );
                if !current_application_keys.insert(key)
                    || !graph
                        .current_resolution_ids
                        .insert(application.user_action_resolution_id.clone())
                {
                    return Err(StoreError::corrupt_owner_state_value(
                        "shaping_decision_applications",
                        &application.shaping_decision_application_id,
                        "authority_status",
                    ));
                }
                represented_current_requests.insert(application.user_action_request_id.clone());
                graph.current_applications.push(authority);
            }
            ShapingDecisionApplicationAuthorityStatus::Stale => {
                if request.basis_status() != UserActionBasisStatus::Stale
                    || request.basis().compatibility_status() != UserActionBasisStatus::Stale
                {
                    return Err(StoreError::corrupt_owner_state_value(
                        "shaping_decision_applications",
                        &application.shaping_decision_application_id,
                        "authority_status",
                    ));
                }
                if !task_is_terminal {
                    stale_request_ids.insert(application.user_action_request_id.clone());
                    graph.stale_recovery_obligations.push(authority);
                }
            }
            ShapingDecisionApplicationAuthorityStatus::Superseded => {
                if request.basis_status() != UserActionBasisStatus::Superseded
                    || request.basis().compatibility_status() != UserActionBasisStatus::Superseded
                {
                    return Err(StoreError::corrupt_owner_state_value(
                        "shaping_decision_applications",
                        &application.shaping_decision_application_id,
                        "authority_status",
                    ));
                }
                if !task_is_terminal && application.linked_checkpoint_id.is_some() {
                    return Err(StoreError::corrupt_owner_state_value(
                        "shaping_checkpoint_applications",
                        &application.shaping_decision_application_id,
                        "shaping_decision_application_id",
                    ));
                }
            }
        }
    }

    for record in user_actions.values().filter(|_| !task_is_terminal) {
        let request = record.request();
        if !matches!(
            request.metadata(),
            PersistedUserActionRequestMetadata::Shaping(_)
        ) {
            continue;
        }
        match request.basis_status() {
            UserActionBasisStatus::Current
                if !represented_current_requests.contains(request.user_action_request_id()) =>
            {
                return Err(StoreError::corrupt_owner_state_value(
                    "user_action_requests",
                    request.user_action_request_id(),
                    "basis_status",
                ));
            }
            UserActionBasisStatus::Stale
                if checkpoint.as_ref().is_some_and(|checkpoint| {
                    checkpoint.gaps.iter().any(|gap| {
                        gap.user_action.as_ref().is_some_and(|link| {
                            link.user_action_request_id == request.user_action_request_id()
                        })
                    })
                }) && !stale_request_ids.contains(request.user_action_request_id()) =>
            {
                return Err(StoreError::corrupt_owner_state_value(
                    "user_action_requests",
                    request.user_action_request_id(),
                    "basis_status",
                ));
            }
            UserActionBasisStatus::Current
            | UserActionBasisStatus::Stale
            | UserActionBasisStatus::Superseded => {}
        }
    }

    Ok(graph)
}

fn shaping_application_by_id(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    application_id: &str,
) -> StoreResult<Option<ShapingDecisionApplicationRecord>> {
    let sql = format!(
        "SELECT {APPLICATION_COLUMNS}, current_link.shaping_checkpoint_id,
                current_link.carried_from_checkpoint_id
           FROM shaping_decision_applications AS application
           LEFT JOIN shaping_checkpoint_applications AS current_link
             ON current_link.project_id = application.project_id
            AND current_link.task_id = application.task_id
            AND current_link.shaping_decision_application_id = application.shaping_decision_application_id
            AND EXISTS (
              SELECT 1 FROM shaping_checkpoints AS current_checkpoint
               WHERE current_checkpoint.project_id = current_link.project_id
                 AND current_checkpoint.task_id = current_link.task_id
                 AND current_checkpoint.shaping_checkpoint_id = current_link.shaping_checkpoint_id
                 AND current_checkpoint.readiness <> 'superseded'
            )
          WHERE application.project_id = ?1 AND application.task_id = ?2
            AND application.shaping_decision_application_id = ?3"
    );
    conn.query_row(
        &sql,
        params![project_id, task_id, application_id],
        raw_shaping_application,
    )
    .optional()?
    .map(|raw| decode_shaping_application(conn, raw))
    .transpose()
}

fn shaping_reauthorization_history_for_task(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<ShapingAuthorityReauthorizationRecord>> {
    let mut statement = conn.prepare(
        "SELECT project_id, shaping_authority_reauthorization_id, task_id,
                stale_application_id, stale_user_action_request_id,
                successor_checkpoint_id, successor_gap_id,
                successor_user_action_request_id, outcome, created_at
           FROM shaping_authority_reauthorizations
          WHERE project_id = ?1 AND task_id = ?2
          ORDER BY shaping_authority_reauthorization_id",
    )?;
    let rows = statement
        .query_map(params![project_id, task_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(
            |(
                row_project_id,
                lineage_id,
                row_task_id,
                stale_application_id,
                stale_request_id,
                successor_checkpoint_id,
                successor_gap_id,
                successor_request_id,
                outcome_raw,
                created_at_raw,
            )| {
                let corrupt = |column| {
                    StoreError::corrupt_owner_state_value(
                        "shaping_authority_reauthorizations",
                        lineage_id.clone(),
                        column,
                    )
                };
                if row_project_id != project_id || row_task_id != task_id {
                    return Err(corrupt("task_id"));
                }
                let outcome: ShapingAuthorityReauthorizationOutcome = decode_owner_closed_value(
                    "shaping_authority_reauthorizations",
                    &lineage_id,
                    "outcome",
                    &outcome_raw,
                )?;
                let expected_id = shaping_authority_reauthorization_id(
                    &ShapingDecisionApplicationId::new(stale_application_id.clone()),
                )
                .map_err(|_| corrupt("shaping_authority_reauthorization_id"))?;
                if expected_id.as_str() != lineage_id
                    || matches!(outcome, ShapingAuthorityReauthorizationOutcome::Retired)
                        != (successor_gap_id.is_none() && successor_request_id.is_none())
                    || matches!(outcome, ShapingAuthorityReauthorizationOutcome::Reissued)
                        != (successor_gap_id.is_some() && successor_request_id.is_some())
                {
                    return Err(corrupt("outcome"));
                }
                let exact_matches: bool = conn.query_row(
                    "SELECT EXISTS (
                       SELECT 1
                         FROM shaping_decision_applications AS application
                         JOIN user_action_requests AS old_request
                           ON old_request.project_id = application.project_id
                          AND old_request.user_action_request_id = application.user_action_request_id
                         JOIN shaping_checkpoints AS successor
                           ON successor.project_id = application.project_id
                          AND successor.task_id = application.task_id
                          AND successor.shaping_checkpoint_id = ?6
                        WHERE application.project_id = ?1
                          AND application.task_id = ?2
                          AND application.shaping_decision_application_id = ?3
                          AND application.user_action_request_id = ?4
                          AND application.authority_status = 'superseded'
                          AND application.stale_at IS NOT NULL
                          AND application.superseded_at = ?5
                          AND old_request.basis_status = 'superseded'
                     )",
                    params![
                        project_id,
                        task_id,
                        stale_application_id,
                        stale_request_id,
                        created_at_raw,
                        successor_checkpoint_id,
                    ],
                    |row| row.get(0),
                )?;
                if !exact_matches {
                    return Err(corrupt("stale_application_id"));
                }
                if matches!(outcome, ShapingAuthorityReauthorizationOutcome::Reissued) {
                    let successor_gap_id = successor_gap_id
                        .as_deref()
                        .ok_or_else(|| corrupt("successor_gap_id"))?;
                    let successor_request_id = successor_request_id
                        .as_deref()
                        .ok_or_else(|| corrupt("successor_user_action_request_id"))?;
                    let exact_successor: bool = conn.query_row(
                        "SELECT EXISTS (
                           SELECT 1
                             FROM shaping_checkpoint_gaps AS gap
                             JOIN shaping_checkpoint_user_actions AS link
                               ON link.project_id = gap.project_id
                              AND link.shaping_checkpoint_id = gap.shaping_checkpoint_id
                              AND link.shaping_gap_id = gap.shaping_gap_id
                             JOIN user_action_requests AS request
                               ON request.project_id = link.project_id
                              AND request.user_action_request_id = link.user_action_request_id
                             JOIN shaping_decision_applications AS application
                               ON application.project_id = gap.project_id
                              AND application.task_id = gap.task_id
                              AND application.shaping_decision_application_id = gap.reauthorizes_application_id
                            WHERE gap.project_id = ?1
                              AND gap.task_id = ?2
                              AND gap.shaping_checkpoint_id = ?3
                              AND gap.shaping_gap_id = ?4
                              AND gap.reauthorizes_application_id = ?5
                              AND gap.user_action_request_id = ?6
                              AND gap.gap_kind = CASE application.judgment_kind
                                WHEN 'product_decision' THEN 'user_product_decision_required'
                                WHEN 'technical_decision' THEN 'user_technical_decision_required'
                                WHEN 'scope_decision' THEN 'user_scope_decision_required'
                                WHEN 'sensitive_approval' THEN 'sensitive_approval_required'
                              END
                              AND request.task_id = ?2
                              AND link.linked_at = ?8
                              AND request.requested_at = ?8
                              AND request.source_method = 'volicord.record_shaping'
                              AND json_extract(request.metadata_json, '$.reauthorizes_application_id') = ?5
                              AND request.user_action_request_id <> ?7
                         )",
                        params![
                            project_id,
                            task_id,
                            successor_checkpoint_id,
                            successor_gap_id,
                            stale_application_id,
                            successor_request_id,
                            stale_request_id,
                            created_at_raw,
                        ],
                        |row| row.get(0),
                    )?;
                    if !exact_successor {
                        return Err(corrupt("successor_gap_id"));
                    }
                }
                let created_at =
                    UtcTimestamp::parse(&created_at_raw).map_err(|_| corrupt("created_at"))?;
                Ok(ShapingAuthorityReauthorizationRecord {
                    project_id: row_project_id,
                    shaping_authority_reauthorization_id: lineage_id,
                    task_id: row_task_id,
                    stale_application_id,
                    stale_user_action_request_id: stale_request_id,
                    successor_checkpoint_id,
                    successor_gap_id,
                    successor_user_action_request_id: successor_request_id,
                    outcome,
                    created_at,
                })
            },
        )
        .collect()
}

fn shaping_applications_for_checkpoint(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    checkpoint_id: &str,
) -> StoreResult<Vec<ShapingDecisionApplicationRecord>> {
    let sql = format!(
        "SELECT {APPLICATION_COLUMNS}, link.shaping_checkpoint_id,
                link.carried_from_checkpoint_id
           FROM shaping_checkpoint_applications AS link
           JOIN shaping_decision_applications AS application
             ON application.project_id = link.project_id
            AND application.task_id = link.task_id
            AND application.shaping_decision_application_id = link.shaping_decision_application_id
          WHERE link.project_id = ?1 AND link.task_id = ?2
            AND link.shaping_checkpoint_id = ?3
          ORDER BY application.shaping_decision_application_id"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(
        params![project_id, task_id, checkpoint_id],
        raw_shaping_application,
    )?;
    rows.collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|raw| decode_shaping_application(conn, raw))
        .collect()
}

fn decode_shaping_application(
    conn: &Connection,
    raw: RawShapingApplication,
) -> StoreResult<ShapingDecisionApplicationRecord> {
    let corrupt = |column| {
        StoreError::corrupt_owner_state_value(
            "shaping_decision_applications",
            raw.application_id.clone(),
            column,
        )
    };
    let judgment_kind: JudgmentKind = decode_owner_closed_value(
        "shaping_decision_applications",
        &raw.application_id,
        "judgment_kind",
        &raw.judgment_kind,
    )?;
    let application_owner: ShapingDecisionApplicationOwner = decode_owner_closed_value(
        "shaping_decision_applications",
        &raw.application_id,
        "application_owner",
        &raw.application_owner,
    )?;
    let authority_status: ShapingDecisionApplicationAuthorityStatus = decode_owner_closed_value(
        "shaping_decision_applications",
        &raw.application_id,
        "authority_status",
        &raw.authority_status,
    )?;
    let applied_scope_revision =
        u64::try_from(raw.applied_scope_revision).map_err(|_| corrupt("applied_scope_revision"))?;
    let applied_at = UtcTimestamp::parse(&raw.applied_at).map_err(|_| corrupt("applied_at"))?;
    let stale_at = raw
        .stale_at
        .as_deref()
        .map(UtcTimestamp::parse)
        .transpose()
        .map_err(|_| corrupt("stale_at"))?;
    let superseded_at = raw
        .superseded_at
        .as_deref()
        .map(UtcTimestamp::parse)
        .transpose()
        .map_err(|_| corrupt("superseded_at"))?;
    let timestamps_match = match authority_status {
        ShapingDecisionApplicationAuthorityStatus::Current => {
            stale_at.is_none() && superseded_at.is_none()
        }
        ShapingDecisionApplicationAuthorityStatus::Stale => {
            stale_at.is_some() && superseded_at.is_none()
        }
        ShapingDecisionApplicationAuthorityStatus::Superseded => superseded_at.is_some(),
    };
    if !timestamps_match || raw.applied_baseline_ref.trim().is_empty() {
        return Err(corrupt("authority_status"));
    }
    let has_reauthorization_lineage: bool = conn.query_row(
        "SELECT EXISTS (
           SELECT 1 FROM shaping_authority_reauthorizations
            WHERE project_id = ?1 AND task_id = ?2
              AND stale_application_id = ?3
         )",
        params![raw.project_id, raw.task_id, raw.application_id],
        |row| row.get(0),
    )?;
    let lifecycle_matches_lineage = match authority_status {
        ShapingDecisionApplicationAuthorityStatus::Current
        | ShapingDecisionApplicationAuthorityStatus::Stale => !has_reauthorization_lineage,
        ShapingDecisionApplicationAuthorityStatus::Superseded => {
            stale_at.is_some() == has_reauthorization_lineage
        }
    };
    if !lifecycle_matches_lineage {
        return Err(corrupt("stale_at"));
    }
    let expected_id = shaping_decision_application_id(
        &UserActionResolutionId::new(&raw.resolution_id),
        application_owner,
    )
    .map_err(|_| corrupt("shaping_decision_application_id"))?;
    if expected_id.as_str() != raw.application_id {
        return Err(corrupt("shaping_decision_application_id"));
    }
    let source_matches: bool = conn.query_row(
        "SELECT EXISTS (
           SELECT 1
             FROM shaping_checkpoint_gaps AS gap
             JOIN shaping_checkpoint_user_actions AS link
               ON link.project_id = gap.project_id
              AND link.shaping_checkpoint_id = gap.shaping_checkpoint_id
              AND link.shaping_gap_id = gap.shaping_gap_id
             JOIN user_action_resolutions AS resolution
               ON resolution.project_id = link.project_id
              AND resolution.user_action_request_id = link.user_action_request_id
              AND resolution.user_action_resolution_id = link.user_action_resolution_id
            WHERE gap.project_id = ?1 AND gap.task_id = ?2
              AND gap.shaping_checkpoint_id = ?3 AND gap.shaping_gap_id = ?4
              AND gap.status = 'applied'
              AND link.user_action_request_id = ?5
              AND link.user_action_resolution_id = ?6
              AND link.action_kind = ?7
              AND json_extract(resolution.resolution_json, '$.resolution_type') = 'choice'
              AND json_extract(resolution.resolution_json, '$.machine_action') = 'accept'
              AND json_extract(resolution.resolution_json, '$.resolution_outcome') = 'accepted'
         )",
        params![
            raw.project_id,
            raw.task_id,
            raw.source_checkpoint_id,
            raw.source_gap_id,
            raw.request_id,
            raw.resolution_id,
            encode_closed_value("judgment_kind", &judgment_kind)?,
        ],
        |row| row.get(0),
    )?;
    if !source_matches {
        return Err(corrupt("source_gap_id"));
    }
    if let Some(linked_checkpoint_id) = raw.linked_checkpoint_id.as_deref() {
        let lineage_matches: bool = conn.query_row(
            "SELECT EXISTS (
               SELECT 1 FROM shaping_checkpoint_applications AS link
                WHERE link.project_id = ?1 AND link.task_id = ?2
                  AND link.shaping_checkpoint_id = ?3
                  AND link.shaping_decision_application_id = ?4
                  AND link.carried_from_checkpoint_id IS ?5
             )",
            params![
                raw.project_id,
                raw.task_id,
                linked_checkpoint_id,
                raw.application_id,
                raw.carried_from_checkpoint_id,
            ],
            |row| row.get(0),
        )?;
        if !lineage_matches {
            return Err(corrupt("shaping_decision_application_id"));
        }
    }
    if authority_status == ShapingDecisionApplicationAuthorityStatus::Current {
        let current_matches: bool = conn.query_row(
            "SELECT EXISTS (
               SELECT 1 FROM tasks AS task
                WHERE task.project_id = ?1 AND task.task_id = ?2
                  AND task.scope_revision = ?3
                  AND json_extract(task.shaping_summary_json, '$.baseline_ref') = ?4
                  AND task.current_change_unit_id IS ?5
                  AND ?6 IS NOT NULL
             )",
            params![
                raw.project_id,
                raw.task_id,
                raw.applied_scope_revision,
                raw.applied_baseline_ref,
                raw.applied_change_unit_id,
                raw.linked_checkpoint_id,
            ],
            |row| row.get(0),
        )?;
        if !current_matches {
            return Err(corrupt("authority_status"));
        }
    }
    Ok(ShapingDecisionApplicationRecord {
        project_id: raw.project_id,
        shaping_decision_application_id: raw.application_id,
        task_id: raw.task_id,
        source_checkpoint_id: raw.source_checkpoint_id,
        source_gap_id: raw.source_gap_id,
        user_action_request_id: raw.request_id,
        user_action_resolution_id: raw.resolution_id,
        judgment_kind,
        application_owner,
        applied_scope_revision,
        applied_baseline_ref: BaselineRef::new(raw.applied_baseline_ref),
        applied_change_unit_id: raw.applied_change_unit_id.map(ChangeUnitId::new),
        applied_at,
        authority_status,
        stale_at,
        superseded_at,
        linked_checkpoint_id: raw.linked_checkpoint_id,
        carried_from_checkpoint_id: raw.carried_from_checkpoint_id,
    })
}

fn shaping_gaps(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    checkpoint_id: &str,
    task_mode: TaskMode,
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
            row.get::<_, Option<String>>(10)?,
        ))
    })?;
    let mut gaps = Vec::new();
    for row in rows {
        let (
            gap_id,
            gap_kind,
            summary,
            affected_refs_json,
            status,
            reauthorizes_application_id,
            request_id,
            action_kind,
        ) = row?;
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
                    ShapingLinkExpectation {
                        checkpoint_id,
                        gap_id: &gap_id,
                        request_id: &request_id,
                        reauthorizes_application_id: reauthorizes_application_id.as_deref(),
                        gap_kind: decoded_kind,
                        task_mode,
                        status: decoded_status,
                    },
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
        if decoded_status == ShapingGapStatus::Applied {
            let application_exists: bool = conn.query_row(
                "SELECT EXISTS (
                   SELECT 1 FROM shaping_decision_applications
                    WHERE project_id = ?1
                      AND source_checkpoint_id = ?2
                      AND source_gap_id = ?3
                 )",
                params![project_id, checkpoint_id, gap_id],
                |row| row.get(0),
            )?;
            if !application_exists {
                return Err(StoreError::corrupt_owner_state_value(
                    "shaping_checkpoint_gaps",
                    &gap_id,
                    "status",
                ));
            }
        }
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
            reauthorizes_application_id,
            user_action,
        });
    }
    Ok(gaps)
}

struct ShapingLinkExpectation<'a> {
    checkpoint_id: &'a str,
    gap_id: &'a str,
    request_id: &'a str,
    reauthorizes_application_id: Option<&'a str>,
    gap_kind: ShapingGapKind,
    task_mode: TaskMode,
    status: ShapingGapStatus,
}

fn shaping_link(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    expected: ShapingLinkExpectation<'_>,
) -> StoreResult<ShapingCheckpointUserActionRecord> {
    let raw = conn
        .query_row(
            "SELECT l.task_id, l.action_kind, l.user_action_resolution_id,
                    l.linked_at, l.resolved_at, r.required_for_json,
                    r.metadata_json, resolution.resolution_json
               FROM shaping_checkpoint_user_actions AS l
               JOIN user_action_requests AS r
                 ON r.project_id = l.project_id
                AND r.user_action_request_id = l.user_action_request_id
               LEFT JOIN user_action_resolutions AS resolution
                 ON resolution.project_id = l.project_id
                AND resolution.user_action_request_id = l.user_action_request_id
                AND resolution.user_action_resolution_id = l.user_action_resolution_id
              WHERE l.project_id = ?1
                AND l.shaping_checkpoint_id = ?2
                AND l.user_action_request_id = ?3",
            params![project_id, expected.checkpoint_id, expected.request_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
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
        metadata_json,
        resolution_json,
    )) = raw
    else {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoint_user_actions",
            expected.request_id.to_owned(),
            "user_action_request_id",
        ));
    };
    let action_kind: UserActionKind = decode_owner_closed_value(
        "shaping_checkpoint_user_actions",
        expected.request_id.to_owned(),
        "action_kind",
        &action_kind,
    )?;
    let Some(policy) = expected
        .gap_kind
        .decision_policy_for_mode(expected.task_mode)
    else {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoint_gaps",
            expected.request_id.to_owned(),
            "gap_kind",
        ));
    };
    let required_for: Vec<UserActionRequiredFor> = decode_owner_json_text(
        "shaping_checkpoint_user_actions",
        expected.request_id,
        "required_for_json",
        &required_for_json,
    )?;
    let metadata: PersistedUserActionRequestMetadata = decode_owner_json_text(
        "shaping_checkpoint_user_actions",
        expected.request_id,
        "metadata_json",
        &metadata_json,
    )?;
    let origin_matches = matches!(
        metadata,
        PersistedUserActionRequestMetadata::Shaping(metadata)
            if metadata.shaping_checkpoint_id.as_str() == expected.checkpoint_id
                && metadata.shaping_gap_id.as_str() == expected.gap_id
                && metadata.reauthorizes_application_id.as_ref().map(|id| id.as_str())
                    == expected.reauthorizes_application_id
    );
    if linked_task_id != task_id
        || action_kind != policy.user_action_kind
        || required_for.as_slice() != policy.required_for
        || !origin_matches
    {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoint_user_actions",
            expected.request_id.to_owned(),
            "task_id",
        ));
    }
    let linked_at = UtcTimestamp::parse(&linked_at).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "shaping_checkpoint_user_actions",
            expected.request_id.to_owned(),
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
                expected.request_id.to_owned(),
                "resolved_at",
            )
        })?;
    if resolution_id.is_some() != resolved_at.is_some() {
        return Err(StoreError::corrupt_owner_state_value(
            "shaping_checkpoint_user_actions",
            expected.request_id.to_owned(),
            "user_action_resolution_id",
        ));
    }
    match (expected.status, resolution_json.as_deref()) {
        (ShapingGapStatus::Current, None) => {}
        (ShapingGapStatus::Accepted | ShapingGapStatus::Applied, Some(json)) => {
            let resolution: UserActionResolutionBody = decode_owner_json_text(
                "shaping_checkpoint_user_actions",
                expected.request_id,
                "resolution_json",
                json,
            )?;
            if !matches!(
                resolution,
                UserActionResolutionBody::Choice {
                    machine_action: UserActionOptionAction::Accept,
                    resolution_outcome: JudgmentResolutionOutcome::Accepted,
                    ..
                }
            ) {
                return Err(StoreError::corrupt_owner_state_value(
                    "shaping_checkpoint_gaps",
                    expected.gap_id,
                    "status",
                ));
            }
        }
        (ShapingGapStatus::Rejected, Some(json)) => {
            let resolution: UserActionResolutionBody = decode_owner_json_text(
                "shaping_checkpoint_user_actions",
                expected.request_id,
                "resolution_json",
                json,
            )?;
            if !matches!(
                resolution,
                UserActionResolutionBody::Choice {
                    machine_action: UserActionOptionAction::Reject,
                    resolution_outcome: JudgmentResolutionOutcome::Rejected,
                    ..
                }
            ) {
                return Err(StoreError::corrupt_owner_state_value(
                    "shaping_checkpoint_gaps",
                    expected.gap_id,
                    "status",
                ));
            }
        }
        (ShapingGapStatus::Deferred, Some(json)) => {
            let resolution: UserActionResolutionBody = decode_owner_json_text(
                "shaping_checkpoint_user_actions",
                expected.request_id,
                "resolution_json",
                json,
            )?;
            if !matches!(
                resolution,
                UserActionResolutionBody::Choice {
                    machine_action: UserActionOptionAction::Defer,
                    resolution_outcome: JudgmentResolutionOutcome::Deferred,
                    ..
                }
            ) {
                return Err(StoreError::corrupt_owner_state_value(
                    "shaping_checkpoint_gaps",
                    expected.gap_id,
                    "status",
                ));
            }
        }
        _ => {
            return Err(StoreError::corrupt_owner_state_value(
                "shaping_checkpoint_gaps",
                expected.gap_id,
                "status",
            ));
        }
    }
    Ok(ShapingCheckpointUserActionRecord {
        user_action_request_id: expected.request_id.to_owned(),
        action_kind,
        user_action_resolution_id: resolution_id,
        linked_at,
        resolved_at,
    })
}

#[cfg(test)]
#[path = "shaping_behavior_tests.rs"]
mod behavior_tests;
