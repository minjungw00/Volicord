use std::path::Path;

use harness_types::{IdempotencyKey, MethodName, ProjectId, RequestHash, SurfaceId, TaskId};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::Value;

use crate::{
    bootstrap::{project_record, ProjectRecord, SurfaceRecord},
    sqlite::{begin_immediate_transaction, open_project_state_database},
    StoreError, StoreResult,
};

/// Project-local store handle used by the Core request pipeline.
#[derive(Debug)]
pub struct CoreProjectStore {
    pub(crate) project: ProjectRecord,
    pub(crate) conn: Connection,
}

/// Current project-state header values needed by request routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStateHeader {
    pub project_id: String,
    pub state_version: u64,
    pub active_task_id: Option<String>,
    pub default_surface_id: Option<String>,
    pub default_surface_instance_id: Option<String>,
}

/// Stored idempotency replay row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolInvocationRecord {
    pub project_id: String,
    pub tool_name: String,
    pub idempotency_key: String,
    pub request_hash: String,
    pub basis_state_version: u64,
    pub committed_state_version: u64,
    pub response_json: String,
}

/// Pending event supplied by a method-specific commit branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTaskEvent {
    pub event_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub event_kind: String,
    pub event_payload_json: String,
}

/// Storage-level mutation applied inside one Core commit transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreStorageMutation {
    InsertTask(TaskInsert),
    SetActiveTask { task_id: String },
    SupersedeTask { task_id: String },
    UpdateTaskScope(TaskScopeUpdate),
    InsertCurrentChangeUnit(ChangeUnitInsert),
    ReplaceCurrentChangeUnit(ChangeUnitInsert),
    MarkActiveWriteAuthorizationsStale { task_id: String },
    InsertWriteAuthorization(WriteAuthorizationInsert),
    ConsumeWriteAuthorization(WriteAuthorizationConsumption),
    InsertRun(RunInsert),
    PromoteStagedArtifact(ArtifactPromotion),
    LinkArtifact(ArtifactLinkInsert),
    UpsertEvidenceSummary(EvidenceSummaryUpsert),
    InsertUserJudgment(UserJudgmentInsert),
    ResolveUserJudgment(UserJudgmentResolutionUpdate),
}

/// Storage input for inserting a Task current row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInsert {
    pub task_id: String,
    pub created_by_surface_id: String,
    pub created_by_surface_instance_id: String,
    pub mode: String,
    pub lifecycle_phase: String,
    pub result: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub shaping_summary_json: String,
    pub bounded_context_json: String,
    pub autonomy_boundary_json: String,
    pub close_summary_json: String,
    pub completion_policy_json: String,
    pub current_change_unit_id: Option<String>,
}

/// Storage input for updating Task scope-shaped current fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskScopeUpdate {
    pub task_id: String,
    pub lifecycle_phase: Option<String>,
    pub result: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub shaping_summary_json: Option<String>,
    pub bounded_context_json: Option<String>,
    pub autonomy_boundary_json: Option<String>,
    pub close_summary_json: Option<String>,
    pub completion_policy_json: Option<String>,
}

/// Storage input for inserting a current Change Unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeUnitInsert {
    pub change_unit_id: String,
    pub task_id: String,
    pub scope_summary_json: String,
    pub bounded_paths_json: String,
    pub write_basis_json: String,
    pub close_basis_json: String,
    pub lifecycle_json: String,
}

/// Storage input for inserting a pending user-owned judgment request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserJudgmentInsert {
    pub judgment_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub judgment_kind: String,
    pub request_json: String,
    pub context_json: String,
    pub options_json: String,
    pub affected_refs_json: String,
    pub artifact_refs_json: String,
    pub sensitive_action_scope_json: String,
    pub requested_by_surface_id: String,
    pub requested_by_surface_instance_id: String,
    pub requested_at: String,
    pub metadata_json: String,
}

/// Storage input for resolving one pending user-owned judgment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserJudgmentResolutionUpdate {
    pub judgment_id: String,
    pub status: String,
    pub resolution_json: String,
    pub sensitive_action_scope_json: Option<String>,
    pub resolved_at: String,
}

/// Storage input for inserting one active Write Authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteAuthorizationInsert {
    pub write_authorization_id: String,
    pub task_id: String,
    pub change_unit_id: String,
    pub attempt_scope_json: String,
    pub created_by_surface_id: String,
    pub created_by_surface_instance_id: String,
    pub created_by_judgment_id: Option<String>,
    pub expires_at: String,
    pub metadata_json: String,
}

/// Storage input for consuming one active Write Authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteAuthorizationConsumption {
    pub write_authorization_id: String,
    pub run_id: String,
    pub expected_basis_state_version: u64,
}

/// Storage input for inserting one committed Run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunInsert {
    pub run_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub write_authorization_id: Option<String>,
    pub kind: String,
    pub status: String,
    pub summary_json: String,
    pub observed_changes_json: String,
    pub evidence_updates_json: String,
    pub authorization_effect_json: String,
    pub created_by_surface_id: String,
    pub created_by_surface_instance_id: String,
    pub metadata_json: String,
}

/// Storage input for promoting one staged artifact to a persistent artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactPromotion {
    pub handle_id: String,
    pub artifact_id: String,
    pub task_id: String,
    pub run_id: String,
    pub expected_created_by_surface_id: String,
    pub expected_created_by_surface_instance_id: String,
    pub expected_sha256: String,
    pub expected_size_bytes: u64,
    pub expected_redaction_state: String,
    pub uri: String,
    pub retention_json: String,
    pub producer_json: String,
    pub metadata_json: String,
}

/// Storage input for linking a persistent artifact to an owner relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactLinkInsert {
    pub artifact_id: String,
    pub task_id: String,
    pub owner_record_kind: String,
    pub owner_record_id: String,
    pub created_by_run_id: String,
    pub metadata_json: String,
}

/// Storage input for creating or replacing one evidence summary row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSummaryUpsert {
    pub evidence_summary_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub status: String,
    pub coverage_json: String,
    pub supporting_refs_json: String,
    pub gap_refs_json: String,
    pub metadata_json: String,
}

/// Event reference facts created by an atomic mutation commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedEventRef {
    pub event_id: String,
    pub event_kind: String,
}

/// Facts available to build the exact committed response before replay storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedMutationFacts {
    pub basis_state_version: u64,
    pub committed_state_version: u64,
    pub events: Vec<CommittedEventRef>,
}

/// Input for an atomic Core mutation commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitMutationInput {
    pub project_id: String,
    pub tool_name: String,
    pub idempotency_key: Option<String>,
    pub request_hash: String,
    pub expected_state_version: Option<u64>,
    pub events: Vec<PendingTaskEvent>,
}

/// Result of attempting a mutating commit through the replay/freshness gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationCommitOutcome {
    Replayed {
        response_json: String,
        basis_state_version: u64,
        committed_state_version: u64,
    },
    IdempotencyConflict {
        current_state_version: u64,
        idempotency_key: String,
        stored_request_hash: String,
        attempted_request_hash: String,
    },
    StaleExpectedState {
        current_state_version: u64,
        expected_state_version: u64,
    },
    Committed {
        response_json: String,
        basis_state_version: u64,
        committed_state_version: u64,
        events: Vec<CommittedEventRef>,
    },
}

/// Storage counters used to verify no-effect request branches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageEffectCounts {
    pub state_version: u64,
    pub tasks: u64,
    pub change_units: u64,
    pub task_events: u64,
    pub tool_invocations: u64,
    pub user_judgments: u64,
    pub write_authorizations: u64,
    pub runs: u64,
    pub artifact_staging: u64,
    pub artifacts: u64,
    pub artifact_links: u64,
    pub evidence_summaries: u64,
    pub blockers: u64,
}

/// Current Task row data needed by Core method implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    pub project_id: String,
    pub task_id: String,
    pub mode: String,
    pub lifecycle_phase: String,
    pub result: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub shaping_summary_json: String,
    pub bounded_context_json: String,
    pub autonomy_boundary_json: String,
    pub close_summary_json: String,
    pub completion_policy_json: String,
    pub current_change_unit_id: Option<String>,
    pub closed_at: Option<String>,
}

/// Current Change Unit row data needed by Core method implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeUnitRecord {
    pub project_id: String,
    pub change_unit_id: String,
    pub task_id: String,
    pub status: String,
    pub is_current: bool,
    pub basis_state_version: Option<u64>,
    pub scope_summary_json: String,
    pub bounded_paths_json: String,
    pub write_basis_json: String,
    pub close_basis_json: String,
    pub lifecycle_json: String,
}

/// Stored Write Authorization facts needed by status and stale-marking responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteAuthorizationRecord {
    pub project_id: String,
    pub write_authorization_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub basis_state_version: u64,
    pub status: String,
    pub attempt_scope_json: String,
    pub expires_at: String,
}

/// Stored staged artifact facts needed by `harness.record_run`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredArtifactStagingRecord {
    pub project_id: String,
    pub handle_id: String,
    pub task_id: String,
    pub created_by_surface_id: String,
    pub created_by_surface_instance_id: String,
    pub artifact_json: String,
    pub tmp_path: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
    pub content_type: Option<String>,
    pub redaction_state: String,
    pub status: String,
    pub expires_at: String,
}

/// Stored persistent artifact facts needed by `harness.record_run`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredArtifactRecord {
    pub project_id: String,
    pub artifact_id: String,
    pub task_id: String,
    pub producer_run_id: Option<String>,
    pub source_staging_handle_id: Option<String>,
    pub uri: String,
    pub body_path: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
    pub content_type: Option<String>,
    pub redaction_state: String,
    pub status: String,
    pub producer_json: String,
}

/// Stored user-owned judgment row data needed by Core method implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserJudgmentRecord {
    pub project_id: String,
    pub judgment_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub judgment_kind: String,
    pub status: String,
    pub request_json: String,
    pub context_json: String,
    pub options_json: String,
    pub affected_refs_json: String,
    pub artifact_refs_json: String,
    pub sensitive_action_scope_json: String,
    pub resolution_json: Option<String>,
    pub requested_by_surface_id: String,
    pub requested_by_surface_instance_id: String,
    pub requested_at: String,
    pub resolved_at: Option<String>,
    pub metadata_json: String,
}

/// Public record reference facts read from storage rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRecordRef {
    pub record_kind: String,
    pub record_id: String,
    pub project_id: String,
    pub task_id: Option<String>,
    pub state_version: Option<u64>,
}

/// Storage mutation handle scoped to a single committed transaction.
pub struct ProjectMutation<'tx> {
    project_id: &'tx str,
    tx: &'tx Transaction<'tx>,
}

impl CoreProjectStore {
    /// Opens the registered project-local state store for Core pipeline work.
    pub fn open(runtime_home: impl AsRef<Path>, project_id: &ProjectId) -> StoreResult<Self> {
        let project = project_record(runtime_home, project_id.as_str())?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "project",
                id: project_id.as_str().to_owned(),
            }
        })?;

        if !project.state_db_path.exists() {
            return Err(StoreError::NotFound {
                entity: "project_state_database",
                id: project.state_db_path.display().to_string(),
            });
        }

        let conn = open_project_state_database(&project.state_db_path)?;
        Ok(Self { project, conn })
    }

    /// Returns the registry project row that selected this project-local store.
    pub const fn project_record(&self) -> &ProjectRecord {
        &self.project
    }

    /// Reads the current project-state header.
    pub fn project_state(&self) -> StoreResult<ProjectStateHeader> {
        read_project_state(&self.conn, &self.project.project_id)
    }

    /// Reads one surface instance by exact project/surface/instance identity.
    pub fn surface(
        &self,
        surface_id: &SurfaceId,
        surface_instance_id: &str,
    ) -> StoreResult<Option<SurfaceRecord>> {
        surface_by_instance(
            &self.conn,
            &self.project.project_id,
            surface_id.as_str(),
            surface_instance_id,
        )
    }

    /// Lists candidate surface instances for a request-level `surface_id`.
    pub fn surface_candidates(&self, surface_id: &SurfaceId) -> StoreResult<Vec<SurfaceRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                project_id,
                surface_id,
                surface_instance_id,
                surface_kind,
                display_name,
                capability_profile_json,
                local_access_json,
                metadata_json
             FROM surfaces
             WHERE project_id = ?1
               AND surface_id = ?2
             ORDER BY surface_instance_id",
        )?;
        let rows = stmt.query_map(
            params![self.project.project_id, surface_id.as_str()],
            surface_record_from_row,
        )?;

        let mut surfaces = Vec::new();
        for row in rows {
            surfaces.push(row?);
        }
        Ok(surfaces)
    }

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

    /// Reads the current active Task row, when `project_state.active_task_id` is set.
    pub fn active_task_record(&self) -> StoreResult<Option<TaskRecord>> {
        let state = self.project_state()?;
        match state.active_task_id {
            Some(task_id) => task_record(&self.conn, &self.project.project_id, &task_id),
            None => Ok(None),
        }
    }

    /// Reads the current active Change Unit row for a Task.
    pub fn current_change_unit(&self, task_id: &TaskId) -> StoreResult<Option<ChangeUnitRecord>> {
        current_change_unit(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Reads one Change Unit row by exact Task and Change Unit identity.
    pub fn change_unit_record(
        &self,
        task_id: &TaskId,
        change_unit_id: &str,
    ) -> StoreResult<Option<ChangeUnitRecord>> {
        change_unit_record(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            change_unit_id,
        )
    }

    /// Lists active Write Authorizations for a Task.
    pub fn active_write_authorizations(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Vec<WriteAuthorizationRecord>> {
        active_write_authorizations(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Reads one Write Authorization row by exact project-local identity.
    pub fn write_authorization_record(
        &self,
        write_authorization_id: &str,
    ) -> StoreResult<Option<WriteAuthorizationRecord>> {
        write_authorization_record(&self.conn, &self.project.project_id, write_authorization_id)
    }

    /// Reads one staged artifact row by exact project-local handle identity.
    pub fn artifact_staging_record(
        &self,
        handle_id: &str,
    ) -> StoreResult<Option<StoredArtifactStagingRecord>> {
        artifact_staging_record(&self.conn, &self.project.project_id, handle_id)
    }

    /// Reads one persistent artifact row by exact project-local artifact identity.
    pub fn artifact_record(&self, artifact_id: &str) -> StoreResult<Option<StoredArtifactRecord>> {
        artifact_record(&self.conn, &self.project.project_id, artifact_id)
    }

    /// Returns whether a persistent artifact already has an owner link for a Task.
    pub fn artifact_has_task_owner_link(
        &self,
        artifact_id: &str,
        task_id: &str,
    ) -> StoreResult<bool> {
        artifact_has_task_owner_link(&self.conn, &self.project.project_id, artifact_id, task_id)
    }

    /// Lists pending user-judgment refs for a Task.
    pub fn pending_user_judgment_refs(
        &self,
        task_id: &TaskId,
        state_version: u64,
    ) -> StoreResult<Vec<StoredRecordRef>> {
        task_scoped_refs(
            &self.conn,
            RefQuery {
                project_id: &self.project.project_id,
                table: "user_judgments",
                id_column: "judgment_id",
                record_kind: "user_judgment",
                task_id: task_id.as_str(),
                status_column: "status",
                status_value: "pending",
                state_version,
            },
        )
    }

    /// Reads one user-owned judgment row by project-local judgment identity.
    pub fn user_judgment_record(
        &self,
        judgment_id: &str,
    ) -> StoreResult<Option<UserJudgmentRecord>> {
        user_judgment_record(&self.conn, &self.project.project_id, judgment_id)
    }

    /// Lists resolved user-owned judgment records for a Task and judgment kind.
    pub fn resolved_user_judgment_records(
        &self,
        task_id: &TaskId,
        judgment_kind: &str,
    ) -> StoreResult<Vec<UserJudgmentRecord>> {
        resolved_user_judgment_records(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            judgment_kind,
        )
    }

    /// Returns the store clock in the public timestamp shape used by Core rows.
    pub fn current_timestamp(&self) -> StoreResult<String> {
        self.conn
            .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
                row.get(0)
            })
            .map_err(StoreError::from)
    }

    /// Lists active blocker refs for a Task.
    pub fn active_blocker_refs(
        &self,
        task_id: &TaskId,
        state_version: u64,
    ) -> StoreResult<Vec<StoredRecordRef>> {
        task_scoped_refs(
            &self.conn,
            RefQuery {
                project_id: &self.project.project_id,
                table: "blockers",
                id_column: "blocker_id",
                record_kind: "blocker",
                task_id: task_id.as_str(),
                status_column: "status",
                status_value: "active",
                state_version,
            },
        )
    }

    /// Reads a committed replay row without creating storage effects.
    pub fn tool_invocation(
        &self,
        method_name: MethodName,
        idempotency_key: &IdempotencyKey,
    ) -> StoreResult<Option<ToolInvocationRecord>> {
        tool_invocation(
            &self.conn,
            &self.project.project_id,
            method_name.as_str(),
            idempotency_key.as_str(),
        )
    }

    /// Reads the current storage-effect counters for this project.
    pub fn effect_counts(&self) -> StoreResult<StorageEffectCounts> {
        let state = self.project_state()?;
        Ok(StorageEffectCounts {
            state_version: state.state_version,
            tasks: table_count(&self.conn, "tasks", &self.project.project_id)?,
            change_units: table_count(&self.conn, "change_units", &self.project.project_id)?,
            task_events: table_count(&self.conn, "task_events", &self.project.project_id)?,
            tool_invocations: table_count(
                &self.conn,
                "tool_invocations",
                &self.project.project_id,
            )?,
            user_judgments: table_count(&self.conn, "user_judgments", &self.project.project_id)?,
            write_authorizations: table_count(
                &self.conn,
                "write_authorizations",
                &self.project.project_id,
            )?,
            runs: table_count(&self.conn, "runs", &self.project.project_id)?,
            artifact_staging: table_count(
                &self.conn,
                "artifact_staging",
                &self.project.project_id,
            )?,
            artifacts: table_count(&self.conn, "artifacts", &self.project.project_id)?,
            artifact_links: table_count(&self.conn, "artifact_links", &self.project.project_id)?,
            evidence_summaries: table_count(
                &self.conn,
                "evidence_summaries",
                &self.project.project_id,
            )?,
            blockers: table_count(&self.conn, "blockers", &self.project.project_id)?,
        })
    }

    /// Commits one state-changing Core mutation or returns replay/conflict outcomes.
    ///
    /// The helper performs replay lookup, stale-state checking, project clock
    /// increment, event append, response construction, and replay-row insertion
    /// in one immediate transaction. Any error rolls back the whole attempt.
    pub fn commit_mutation(
        &mut self,
        input: CommitMutationInput,
        apply_mutation: impl FnOnce(
            &mut ProjectMutation<'_>,
            &CommittedMutationFacts,
        ) -> StoreResult<()>,
        build_response_json: impl FnOnce(CommittedMutationFacts) -> StoreResult<String>,
    ) -> StoreResult<MutationCommitOutcome> {
        if input.project_id != self.project.project_id {
            return Err(StoreError::InvalidInput {
                detail: "commit project_id must match the opened project".to_owned(),
            });
        }
        if input.events.is_empty() {
            return Err(StoreError::InvalidInput {
                detail: "committed Core mutations must append at least one task_event".to_owned(),
            });
        }
        validate_identifier("tool_name", &input.tool_name)?;
        validate_identifier("request_hash", &input.request_hash)?;
        for event in &input.events {
            validate_pending_event(event)?;
        }

        let tx = begin_immediate_transaction(&mut self.conn)?;
        let current = read_project_state_tx(&tx, &self.project.project_id)?;

        if let Some(idempotency_key) = &input.idempotency_key {
            validate_identifier("idempotency_key", idempotency_key)?;
            if let Some(record) = tool_invocation_tx(
                &tx,
                &self.project.project_id,
                &input.tool_name,
                idempotency_key,
            )? {
                tx.rollback()?;
                if record.request_hash == input.request_hash {
                    return Ok(MutationCommitOutcome::Replayed {
                        response_json: record.response_json,
                        basis_state_version: record.basis_state_version,
                        committed_state_version: record.committed_state_version,
                    });
                }

                return Ok(MutationCommitOutcome::IdempotencyConflict {
                    current_state_version: current.state_version,
                    idempotency_key: idempotency_key.clone(),
                    stored_request_hash: record.request_hash,
                    attempted_request_hash: input.request_hash,
                });
            }
        }

        if let Some(expected_state_version) = input.expected_state_version {
            if expected_state_version != current.state_version {
                tx.rollback()?;
                return Ok(MutationCommitOutcome::StaleExpectedState {
                    current_state_version: current.state_version,
                    expected_state_version,
                });
            }
        }

        let committed_state_version =
            current
                .state_version
                .checked_add(1)
                .ok_or_else(|| StoreError::SchemaInvariant {
                    database_kind: "project_state",
                    detail: "project_state.state_version overflow".to_owned(),
                })?;
        let current_state_i64 = u64_to_i64("basis_state_version", current.state_version)?;
        let committed_state_i64 = u64_to_i64("committed_state_version", committed_state_version)?;

        let changed = tx.execute(
            "UPDATE project_state
                SET state_version = ?3,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND state_version = ?2",
            params![
                self.project.project_id,
                current_state_i64,
                committed_state_i64
            ],
        )?;
        if changed != 1 {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "project_state state_version update changed no rows".to_owned(),
            });
        }

        let committed_events = input
            .events
            .iter()
            .map(|event| CommittedEventRef {
                event_id: event.event_id.clone(),
                event_kind: event.event_kind.clone(),
            })
            .collect::<Vec<_>>();
        let facts = CommittedMutationFacts {
            basis_state_version: current.state_version,
            committed_state_version,
            events: committed_events.clone(),
        };
        let mut mutation = ProjectMutation {
            project_id: &self.project.project_id,
            tx: &tx,
        };
        apply_mutation(&mut mutation, &facts)?;

        let first_event_seq = next_event_seq(&tx, &self.project.project_id)?;
        for (index, event) in input.events.iter().enumerate() {
            let event_seq = first_event_seq
                + i64::try_from(index).map_err(|_| StoreError::InvalidInput {
                    detail: "event index does not fit in SQLite integer".to_owned(),
                })?;
            tx.execute(
                "INSERT INTO task_events (
                    project_id,
                    event_seq,
                    event_id,
                    task_id,
                    change_unit_id,
                    state_version,
                    event_kind,
                    event_payload_json,
                    created_at
                )
                VALUES (
                    ?1,
                    ?2,
                    ?3,
                    ?4,
                    ?5,
                    ?6,
                    ?7,
                    ?8,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                )",
                params![
                    self.project.project_id,
                    event_seq,
                    event.event_id,
                    event.task_id,
                    event.change_unit_id,
                    committed_state_i64,
                    event.event_kind,
                    event.event_payload_json
                ],
            )?;
        }

        let response_json = build_response_json(facts)?;
        validate_json_text("tool_invocations.response_json", &response_json)?;

        if let Some(idempotency_key) = &input.idempotency_key {
            tx.execute(
                "INSERT INTO tool_invocations (
                    project_id,
                    tool_name,
                    idempotency_key,
                    request_hash,
                    basis_state_version,
                    committed_state_version,
                    response_json,
                    created_at
                )
                VALUES (
                    ?1,
                    ?2,
                    ?3,
                    ?4,
                    ?5,
                    ?6,
                    ?7,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                )",
                params![
                    self.project.project_id,
                    input.tool_name,
                    idempotency_key,
                    input.request_hash,
                    current_state_i64,
                    committed_state_i64,
                    response_json
                ],
            )?;
        }

        tx.commit()?;
        Ok(MutationCommitOutcome::Committed {
            response_json,
            basis_state_version: current.state_version,
            committed_state_version,
            events: committed_events,
        })
    }
}

/// Builds a commit input from typed public identifiers.
pub fn commit_input(
    project_id: &ProjectId,
    method_name: MethodName,
    idempotency_key: Option<&IdempotencyKey>,
    request_hash: &RequestHash,
    expected_state_version: Option<u64>,
    events: Vec<PendingTaskEvent>,
) -> CommitMutationInput {
    CommitMutationInput {
        project_id: project_id.as_str().to_owned(),
        tool_name: method_name.as_str().to_owned(),
        idempotency_key: idempotency_key.map(|key| key.as_str().to_owned()),
        request_hash: request_hash.as_str().to_owned(),
        expected_state_version,
        events,
    }
}

impl CoreStorageMutation {
    /// Applies this storage mutation inside the active Core commit transaction.
    pub fn apply(
        &self,
        mutation: &mut ProjectMutation<'_>,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        match self {
            Self::InsertTask(input) => mutation.insert_task(input),
            Self::SetActiveTask { task_id } => mutation.set_active_task(task_id),
            Self::SupersedeTask { task_id } => mutation.supersede_task(task_id),
            Self::UpdateTaskScope(input) => mutation.update_task_scope(input),
            Self::InsertCurrentChangeUnit(input) => {
                mutation.insert_current_change_unit(input, committed_state_version)
            }
            Self::ReplaceCurrentChangeUnit(input) => {
                mutation.replace_current_change_unit(input, committed_state_version)
            }
            Self::MarkActiveWriteAuthorizationsStale { task_id } => {
                mutation.mark_active_write_authorizations_stale(task_id)
            }
            Self::InsertWriteAuthorization(input) => {
                mutation.insert_write_authorization(input, committed_state_version)
            }
            Self::ConsumeWriteAuthorization(input) => mutation.consume_write_authorization(input),
            Self::InsertRun(input) => mutation.insert_run(input),
            Self::PromoteStagedArtifact(input) => mutation.promote_staged_artifact(input),
            Self::LinkArtifact(input) => mutation.link_artifact(input),
            Self::UpsertEvidenceSummary(input) => mutation.upsert_evidence_summary(input),
            Self::InsertUserJudgment(input) => mutation.insert_user_judgment(input),
            Self::ResolveUserJudgment(input) => mutation.resolve_user_judgment(input),
        }
    }
}

impl ProjectMutation<'_> {
    fn insert_task(&mut self, input: &TaskInsert) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("created_by_surface_id", &input.created_by_surface_id)?;
        validate_identifier(
            "created_by_surface_instance_id",
            &input.created_by_surface_instance_id,
        )?;
        validate_identifier("mode", &input.mode)?;
        validate_identifier("lifecycle_phase", &input.lifecycle_phase)?;
        validate_json_text("tasks.shaping_summary_json", &input.shaping_summary_json)?;
        validate_json_text("tasks.bounded_context_json", &input.bounded_context_json)?;
        validate_json_text(
            "tasks.autonomy_boundary_json",
            &input.autonomy_boundary_json,
        )?;
        validate_json_text("tasks.close_summary_json", &input.close_summary_json)?;
        validate_json_text(
            "tasks.completion_policy_json",
            &input.completion_policy_json,
        )?;

        self.tx.execute(
            "INSERT INTO tasks (
                project_id,
                task_id,
                created_by_surface_id,
                created_by_surface_instance_id,
                mode,
                lifecycle_phase,
                result,
                title,
                summary,
                shaping_summary_json,
                bounded_context_json,
                autonomy_boundary_json,
                close_summary_json,
                completion_policy_json,
                current_change_unit_id,
                created_at,
                updated_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6,
                ?7,
                ?8,
                ?9,
                ?10,
                ?11,
                ?12,
                ?13,
                ?14,
                ?15,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )",
            params![
                self.project_id,
                input.task_id,
                input.created_by_surface_id,
                input.created_by_surface_instance_id,
                input.mode,
                input.lifecycle_phase,
                input.result,
                input.title,
                input.summary,
                input.shaping_summary_json,
                input.bounded_context_json,
                input.autonomy_boundary_json,
                input.close_summary_json,
                input.completion_policy_json,
                input.current_change_unit_id
            ],
        )?;
        Ok(())
    }

    fn set_active_task(&mut self, task_id: &str) -> StoreResult<()> {
        validate_identifier("task_id", task_id)?;
        let changed = self.tx.execute(
            "UPDATE project_state
                SET active_task_id = ?2,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
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
                    closed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND task_id = ?2",
            params![self.project_id, task_id],
        )?;
        Ok(())
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
            validate_json_text("tasks.close_summary_json", value)?;
            self.update_task_text_column(&input.task_id, "close_summary_json", value)?;
        }
        if let Some(value) = &input.completion_policy_json {
            validate_json_text("tasks.completion_policy_json", value)?;
            self.update_task_text_column(&input.task_id, "completion_policy_json", value)?;
        }
        if let Some(value) = &input.lifecycle_phase {
            validate_identifier("lifecycle_phase", value)?;
            self.update_task_text_column(&input.task_id, "lifecycle_phase", value)?;
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

    fn insert_current_change_unit(
        &mut self,
        input: &ChangeUnitInsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        self.insert_change_unit(input, committed_state_version)?;
        self.set_task_current_change_unit(&input.task_id, Some(&input.change_unit_id))
    }

    fn replace_current_change_unit(
        &mut self,
        input: &ChangeUnitInsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        self.tx.execute(
            "UPDATE change_units
                SET status = 'replaced',
                    is_current = 0,
                    closed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'
                AND is_current = 1",
            params![self.project_id, input.task_id],
        )?;
        self.insert_current_change_unit(input, committed_state_version)
    }

    fn insert_change_unit(
        &mut self,
        input: &ChangeUnitInsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        validate_identifier("change_unit_id", &input.change_unit_id)?;
        validate_identifier("task_id", &input.task_id)?;
        validate_json_text("change_units.scope_summary_json", &input.scope_summary_json)?;
        validate_json_text("change_units.bounded_paths_json", &input.bounded_paths_json)?;
        validate_json_text("change_units.write_basis_json", &input.write_basis_json)?;
        validate_json_text("change_units.close_basis_json", &input.close_basis_json)?;
        validate_json_text("change_units.lifecycle_json", &input.lifecycle_json)?;
        let basis_state_version = u64_to_i64("basis_state_version", committed_state_version)?;

        self.tx.execute(
            "INSERT INTO change_units (
                project_id,
                change_unit_id,
                task_id,
                status,
                is_current,
                basis_state_version,
                scope_summary_json,
                bounded_paths_json,
                write_basis_json,
                close_basis_json,
                lifecycle_json,
                created_at,
                updated_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                'active',
                1,
                ?4,
                ?5,
                ?6,
                ?7,
                ?8,
                ?9,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )",
            params![
                self.project_id,
                input.change_unit_id,
                input.task_id,
                basis_state_version,
                input.scope_summary_json,
                input.bounded_paths_json,
                input.write_basis_json,
                input.close_basis_json,
                input.lifecycle_json
            ],
        )?;
        Ok(())
    }

    fn set_task_current_change_unit(
        &mut self,
        task_id: &str,
        change_unit_id: Option<&str>,
    ) -> StoreResult<()> {
        validate_identifier("task_id", task_id)?;
        let changed = self.tx.execute(
            "UPDATE tasks
                SET current_change_unit_id = ?3,
                    lifecycle_phase = CASE
                        WHEN ?3 IS NULL THEN lifecycle_phase
                        ELSE 'ready'
                    END,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND task_id = ?2",
            params![self.project_id, task_id, change_unit_id],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "Task current Change Unit update changed no rows".to_owned(),
            })
        }
    }

    fn mark_active_write_authorizations_stale(&mut self, task_id: &str) -> StoreResult<()> {
        validate_identifier("task_id", task_id)?;
        self.tx.execute(
            "UPDATE write_authorizations
                SET status = 'stale'
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'",
            params![self.project_id, task_id],
        )?;
        Ok(())
    }

    fn insert_write_authorization(
        &mut self,
        input: &WriteAuthorizationInsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        validate_identifier("write_authorization_id", &input.write_authorization_id)?;
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("change_unit_id", &input.change_unit_id)?;
        validate_json_text(
            "write_authorizations.attempt_scope_json",
            &input.attempt_scope_json,
        )?;
        validate_identifier("created_by_surface_id", &input.created_by_surface_id)?;
        validate_identifier(
            "created_by_surface_instance_id",
            &input.created_by_surface_instance_id,
        )?;
        if let Some(created_by_judgment_id) = &input.created_by_judgment_id {
            validate_identifier("created_by_judgment_id", created_by_judgment_id)?;
        }
        validate_identifier("expires_at", &input.expires_at)?;
        validate_json_text("write_authorizations.metadata_json", &input.metadata_json)?;
        let basis_state_version = u64_to_i64("basis_state_version", committed_state_version)?;

        self.tx.execute(
            "INSERT INTO write_authorizations (
                project_id,
                write_authorization_id,
                task_id,
                change_unit_id,
                basis_state_version,
                status,
                attempt_scope_json,
                created_by_surface_id,
                created_by_surface_instance_id,
                created_by_judgment_id,
                expires_at,
                consumed_by_run_id,
                consumed_at,
                revoked_at,
                created_at,
                metadata_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                'active',
                ?6,
                ?7,
                ?8,
                ?9,
                ?10,
                NULL,
                NULL,
                NULL,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?11
            )",
            params![
                self.project_id,
                input.write_authorization_id,
                input.task_id,
                input.change_unit_id,
                basis_state_version,
                input.attempt_scope_json,
                input.created_by_surface_id,
                input.created_by_surface_instance_id,
                input.created_by_judgment_id,
                input.expires_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn consume_write_authorization(
        &mut self,
        input: &WriteAuthorizationConsumption,
    ) -> StoreResult<()> {
        validate_identifier("write_authorization_id", &input.write_authorization_id)?;
        validate_identifier("run_id", &input.run_id)?;
        let expected_basis = u64_to_i64(
            "write_authorizations.basis_state_version",
            input.expected_basis_state_version,
        )?;
        let changed = self.tx.execute(
            "UPDATE write_authorizations
                SET status = 'consumed',
                    consumed_by_run_id = ?3,
                    consumed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND write_authorization_id = ?2
                AND status = 'active'
                AND basis_state_version = ?4",
            params![
                self.project_id,
                input.write_authorization_id,
                input.run_id,
                expected_basis
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "active Write Authorization consumption changed no rows".to_owned(),
            })
        }
    }

    fn insert_run(&mut self, input: &RunInsert) -> StoreResult<()> {
        validate_identifier("run_id", &input.run_id)?;
        validate_identifier("task_id", &input.task_id)?;
        if let Some(change_unit_id) = &input.change_unit_id {
            validate_identifier("change_unit_id", change_unit_id)?;
        }
        if let Some(write_authorization_id) = &input.write_authorization_id {
            validate_identifier("write_authorization_id", write_authorization_id)?;
        }
        validate_identifier("runs.kind", &input.kind)?;
        validate_identifier("runs.status", &input.status)?;
        validate_json_text("runs.summary_json", &input.summary_json)?;
        validate_json_text("runs.observed_changes_json", &input.observed_changes_json)?;
        validate_json_text("runs.evidence_updates_json", &input.evidence_updates_json)?;
        validate_json_text(
            "runs.authorization_effect_json",
            &input.authorization_effect_json,
        )?;
        validate_identifier("created_by_surface_id", &input.created_by_surface_id)?;
        validate_identifier(
            "created_by_surface_instance_id",
            &input.created_by_surface_instance_id,
        )?;
        validate_json_text("runs.metadata_json", &input.metadata_json)?;

        self.tx.execute(
            "INSERT INTO runs (
                project_id,
                run_id,
                task_id,
                change_unit_id,
                write_authorization_id,
                kind,
                status,
                summary_json,
                observed_changes_json,
                evidence_updates_json,
                authorization_effect_json,
                created_by_surface_id,
                created_by_surface_instance_id,
                started_at,
                completed_at,
                created_at,
                metadata_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6,
                ?7,
                ?8,
                ?9,
                ?10,
                ?11,
                ?12,
                ?13,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?14
            )",
            params![
                self.project_id,
                input.run_id,
                input.task_id,
                input.change_unit_id,
                input.write_authorization_id,
                input.kind,
                input.status,
                input.summary_json,
                input.observed_changes_json,
                input.evidence_updates_json,
                input.authorization_effect_json,
                input.created_by_surface_id,
                input.created_by_surface_instance_id,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn promote_staged_artifact(&mut self, input: &ArtifactPromotion) -> StoreResult<()> {
        validate_identifier("artifact_staging.handle_id", &input.handle_id)?;
        validate_identifier("artifact_id", &input.artifact_id)?;
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("run_id", &input.run_id)?;
        validate_identifier(
            "expected_created_by_surface_id",
            &input.expected_created_by_surface_id,
        )?;
        validate_identifier(
            "expected_created_by_surface_instance_id",
            &input.expected_created_by_surface_instance_id,
        )?;
        validate_identifier("expected_sha256", &input.expected_sha256)?;
        validate_identifier("expected_redaction_state", &input.expected_redaction_state)?;
        validate_identifier("artifacts.uri", &input.uri)?;
        validate_json_text("artifacts.retention_json", &input.retention_json)?;
        validate_json_text("artifacts.producer_json", &input.producer_json)?;
        validate_json_text("artifacts.metadata_json", &input.metadata_json)?;

        let staging = artifact_staging_record_tx(self.tx, self.project_id, &input.handle_id)?
            .ok_or_else(|| StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "staged artifact disappeared before promotion".to_owned(),
            })?;
        if staging.task_id != input.task_id
            || staging.created_by_surface_id != input.expected_created_by_surface_id
            || staging.created_by_surface_instance_id
                != input.expected_created_by_surface_instance_id
            || staging.status != "staged"
            || staging.sha256.as_deref() != Some(input.expected_sha256.as_str())
            || staging.size_bytes != Some(input.expected_size_bytes)
            || staging.redaction_state != input.expected_redaction_state
        {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "staged artifact changed before promotion".to_owned(),
            });
        }
        let expired: bool = self.tx.query_row(
            "SELECT ?1 <= strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![staging.expires_at],
            |row| row.get::<_, i64>(0).map(|value| value != 0),
        )?;
        if expired {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "staged artifact expired before promotion".to_owned(),
            });
        }

        let size_bytes = u64_to_i64("artifacts.size_bytes", input.expected_size_bytes)?;
        self.tx.execute(
            "INSERT INTO artifacts (
                project_id,
                artifact_id,
                task_id,
                producer_run_id,
                source_staging_handle_id,
                uri,
                body_path,
                sha256,
                size_bytes,
                content_type,
                redaction_state,
                status,
                retention_json,
                producer_json,
                created_at,
                updated_at,
                metadata_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6,
                ?7,
                ?8,
                ?9,
                ?10,
                ?11,
                'available',
                ?12,
                ?13,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?14
            )",
            params![
                self.project_id,
                input.artifact_id,
                input.task_id,
                input.run_id,
                input.handle_id,
                input.uri,
                staging.tmp_path,
                input.expected_sha256,
                size_bytes,
                staging.content_type,
                input.expected_redaction_state,
                input.retention_json,
                input.producer_json,
                input.metadata_json
            ],
        )?;

        let changed = self.tx.execute(
            "UPDATE artifact_staging
                SET status = 'consumed',
                    consumed_by_run_id = ?3,
                    promoted_artifact_id = ?4,
                    consumed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND handle_id = ?2
                AND status = 'staged'",
            params![
                self.project_id,
                input.handle_id,
                input.run_id,
                input.artifact_id
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "staged artifact consumption changed no rows".to_owned(),
            })
        }
    }

    fn link_artifact(&mut self, input: &ArtifactLinkInsert) -> StoreResult<()> {
        validate_identifier("artifact_id", &input.artifact_id)?;
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("owner_record_kind", &input.owner_record_kind)?;
        validate_identifier("owner_record_id", &input.owner_record_id)?;
        validate_identifier("created_by_run_id", &input.created_by_run_id)?;
        validate_json_text("artifact_links.metadata_json", &input.metadata_json)?;

        self.tx.execute(
            "INSERT OR IGNORE INTO artifact_links (
                project_id,
                artifact_id,
                task_id,
                owner_record_kind,
                owner_record_id,
                created_by_run_id,
                created_at,
                metadata_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?7
            )",
            params![
                self.project_id,
                input.artifact_id,
                input.task_id,
                input.owner_record_kind,
                input.owner_record_id,
                input.created_by_run_id,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn upsert_evidence_summary(&mut self, input: &EvidenceSummaryUpsert) -> StoreResult<()> {
        validate_identifier("evidence_summary_id", &input.evidence_summary_id)?;
        validate_identifier("task_id", &input.task_id)?;
        if let Some(change_unit_id) = &input.change_unit_id {
            validate_identifier("change_unit_id", change_unit_id)?;
        }
        validate_identifier("evidence_summaries.status", &input.status)?;
        validate_json_text("evidence_summaries.coverage_json", &input.coverage_json)?;
        validate_json_text(
            "evidence_summaries.supporting_refs_json",
            &input.supporting_refs_json,
        )?;
        validate_json_text("evidence_summaries.gap_refs_json", &input.gap_refs_json)?;
        validate_json_text("evidence_summaries.metadata_json", &input.metadata_json)?;

        self.tx.execute(
            "INSERT INTO evidence_summaries (
                project_id,
                evidence_summary_id,
                task_id,
                change_unit_id,
                status,
                coverage_json,
                supporting_refs_json,
                gap_refs_json,
                created_at,
                updated_at,
                metadata_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6,
                ?7,
                ?8,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?9
            )
            ON CONFLICT(project_id, evidence_summary_id) DO UPDATE SET
                task_id = excluded.task_id,
                change_unit_id = excluded.change_unit_id,
                status = excluded.status,
                coverage_json = excluded.coverage_json,
                supporting_refs_json = excluded.supporting_refs_json,
                gap_refs_json = excluded.gap_refs_json,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                metadata_json = excluded.metadata_json",
            params![
                self.project_id,
                input.evidence_summary_id,
                input.task_id,
                input.change_unit_id,
                input.status,
                input.coverage_json,
                input.supporting_refs_json,
                input.gap_refs_json,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn insert_user_judgment(&mut self, input: &UserJudgmentInsert) -> StoreResult<()> {
        validate_identifier("judgment_id", &input.judgment_id)?;
        validate_identifier("task_id", &input.task_id)?;
        if let Some(change_unit_id) = &input.change_unit_id {
            validate_identifier("change_unit_id", change_unit_id)?;
        }
        validate_identifier("judgment_kind", &input.judgment_kind)?;
        validate_json_text("user_judgments.request_json", &input.request_json)?;
        validate_json_text("user_judgments.context_json", &input.context_json)?;
        validate_json_text("user_judgments.options_json", &input.options_json)?;
        validate_json_text(
            "user_judgments.affected_refs_json",
            &input.affected_refs_json,
        )?;
        validate_json_text(
            "user_judgments.artifact_refs_json",
            &input.artifact_refs_json,
        )?;
        validate_json_text(
            "user_judgments.sensitive_action_scope_json",
            &input.sensitive_action_scope_json,
        )?;
        validate_identifier("requested_by_surface_id", &input.requested_by_surface_id)?;
        validate_identifier(
            "requested_by_surface_instance_id",
            &input.requested_by_surface_instance_id,
        )?;
        validate_identifier("requested_at", &input.requested_at)?;
        validate_json_text("user_judgments.metadata_json", &input.metadata_json)?;

        self.tx.execute(
            "INSERT INTO user_judgments (
                project_id,
                judgment_id,
                task_id,
                change_unit_id,
                judgment_kind,
                status,
                request_json,
                context_json,
                options_json,
                affected_refs_json,
                artifact_refs_json,
                sensitive_action_scope_json,
                resolution_json,
                requested_by_surface_id,
                requested_by_surface_instance_id,
                requested_at,
                resolved_at,
                metadata_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                'pending',
                ?6,
                ?7,
                ?8,
                ?9,
                ?10,
                ?11,
                NULL,
                ?12,
                ?13,
                ?14,
                NULL,
                ?15
            )",
            params![
                self.project_id,
                input.judgment_id,
                input.task_id,
                input.change_unit_id,
                input.judgment_kind,
                input.request_json,
                input.context_json,
                input.options_json,
                input.affected_refs_json,
                input.artifact_refs_json,
                input.sensitive_action_scope_json,
                input.requested_by_surface_id,
                input.requested_by_surface_instance_id,
                input.requested_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn resolve_user_judgment(&mut self, input: &UserJudgmentResolutionUpdate) -> StoreResult<()> {
        validate_identifier("judgment_id", &input.judgment_id)?;
        validate_identifier("status", &input.status)?;
        validate_json_text("user_judgments.resolution_json", &input.resolution_json)?;
        if let Some(value) = &input.sensitive_action_scope_json {
            validate_json_text("user_judgments.sensitive_action_scope_json", value)?;
        }
        validate_identifier("resolved_at", &input.resolved_at)?;

        let changed = self.tx.execute(
            "UPDATE user_judgments
                SET status = ?3,
                    resolution_json = ?4,
                    sensitive_action_scope_json = COALESCE(?5, sensitive_action_scope_json),
                    resolved_at = ?6
              WHERE project_id = ?1
                AND judgment_id = ?2
                AND status = 'pending'",
            params![
                self.project_id,
                input.judgment_id,
                input.status,
                input.resolution_json,
                input.sensitive_action_scope_json,
                input.resolved_at
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "pending user judgment resolution changed no rows".to_owned(),
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
                "UPDATE tasks SET shaping_summary_json = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            "bounded_context_json" => {
                "UPDATE tasks SET bounded_context_json = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            "autonomy_boundary_json" => {
                "UPDATE tasks SET autonomy_boundary_json = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            "close_summary_json" => {
                "UPDATE tasks SET close_summary_json = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            "completion_policy_json" => {
                "UPDATE tasks SET completion_policy_json = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            "lifecycle_phase" => {
                "UPDATE tasks SET lifecycle_phase = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            "result" => {
                "UPDATE tasks SET result = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            _ => {
                return Err(StoreError::InvalidInput {
                    detail: format!("unsupported Task text column {column}"),
                })
            }
        };
        let changed = self
            .tx
            .execute(sql, params![self.project_id, task_id, value])?;
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
                "UPDATE tasks SET title = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            "summary" => {
                "UPDATE tasks SET summary = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE project_id = ?1 AND task_id = ?2"
            }
            _ => {
                return Err(StoreError::InvalidInput {
                    detail: format!("unsupported nullable Task column {column}"),
                })
            }
        };
        let changed = self
            .tx
            .execute(sql, params![self.project_id, task_id, value])?;
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

fn read_project_state(conn: &Connection, project_id: &str) -> StoreResult<ProjectStateHeader> {
    conn.query_row(
        "SELECT
            project_id,
            state_version,
            active_task_id,
            default_surface_id,
            default_surface_instance_id
         FROM project_state
         WHERE project_id = ?1",
        params![project_id],
        project_state_from_row,
    )
    .optional()?
    .ok_or_else(|| StoreError::NotFound {
        entity: "project_state",
        id: project_id.to_owned(),
    })
}

fn task_record(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Option<TaskRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            task_id,
            mode,
            lifecycle_phase,
            result,
            title,
            summary,
            shaping_summary_json,
            bounded_context_json,
            autonomy_boundary_json,
            close_summary_json,
            completion_policy_json,
            current_change_unit_id,
            closed_at
         FROM tasks
         WHERE project_id = ?1
           AND task_id = ?2",
        params![project_id, task_id],
        task_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn task_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRecord> {
    Ok(TaskRecord {
        project_id: row.get(0)?,
        task_id: row.get(1)?,
        mode: row.get(2)?,
        lifecycle_phase: row.get(3)?,
        result: row.get(4)?,
        title: row.get(5)?,
        summary: row.get(6)?,
        shaping_summary_json: row.get(7)?,
        bounded_context_json: row.get(8)?,
        autonomy_boundary_json: row.get(9)?,
        close_summary_json: row.get(10)?,
        completion_policy_json: row.get(11)?,
        current_change_unit_id: row.get(12)?,
        closed_at: row.get(13)?,
    })
}

fn current_change_unit(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Option<ChangeUnitRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            change_unit_id,
            task_id,
            status,
            is_current,
            basis_state_version,
            scope_summary_json,
            bounded_paths_json,
            write_basis_json,
            close_basis_json,
            lifecycle_json
         FROM change_units
         WHERE project_id = ?1
           AND task_id = ?2
           AND status = 'active'
           AND is_current = 1",
        params![project_id, task_id],
        change_unit_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn change_unit_record(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    change_unit_id: &str,
) -> StoreResult<Option<ChangeUnitRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            change_unit_id,
            task_id,
            status,
            is_current,
            basis_state_version,
            scope_summary_json,
            bounded_paths_json,
            write_basis_json,
            close_basis_json,
            lifecycle_json
         FROM change_units
         WHERE project_id = ?1
           AND task_id = ?2
           AND change_unit_id = ?3",
        params![project_id, task_id, change_unit_id],
        change_unit_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn change_unit_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChangeUnitRecord> {
    let is_current = row.get::<_, i64>(4)? == 1;
    let basis_state_version = match row.get::<_, Option<i64>>(5)? {
        Some(value) => Some(nonnegative_i64_to_u64(
            "change_units.basis_state_version",
            value,
        )?),
        None => None,
    };
    Ok(ChangeUnitRecord {
        project_id: row.get(0)?,
        change_unit_id: row.get(1)?,
        task_id: row.get(2)?,
        status: row.get(3)?,
        is_current,
        basis_state_version,
        scope_summary_json: row.get(6)?,
        bounded_paths_json: row.get(7)?,
        write_basis_json: row.get(8)?,
        close_basis_json: row.get(9)?,
        lifecycle_json: row.get(10)?,
    })
}

fn active_write_authorizations(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<WriteAuthorizationRecord>> {
    let mut stmt = conn.prepare(
        "SELECT
            project_id,
            write_authorization_id,
            task_id,
            change_unit_id,
            basis_state_version,
            status,
            attempt_scope_json,
            expires_at
         FROM write_authorizations
         WHERE project_id = ?1
           AND task_id = ?2
           AND status = 'active'
         ORDER BY write_authorization_id",
    )?;
    let rows = stmt.query_map(
        params![project_id, task_id],
        write_authorization_record_from_row,
    )?;
    let mut records = Vec::new();
    for row in rows {
        records.push(row?);
    }
    Ok(records)
}

fn write_authorization_record(
    conn: &Connection,
    project_id: &str,
    write_authorization_id: &str,
) -> StoreResult<Option<WriteAuthorizationRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            write_authorization_id,
            task_id,
            change_unit_id,
            basis_state_version,
            status,
            attempt_scope_json,
            expires_at
         FROM write_authorizations
         WHERE project_id = ?1
           AND write_authorization_id = ?2",
        params![project_id, write_authorization_id],
        write_authorization_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn write_authorization_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<WriteAuthorizationRecord> {
    let basis_state_version = row.get::<_, i64>(4)?;
    Ok(WriteAuthorizationRecord {
        project_id: row.get(0)?,
        write_authorization_id: row.get(1)?,
        task_id: row.get(2)?,
        change_unit_id: row.get(3)?,
        basis_state_version: nonnegative_i64_to_u64(
            "write_authorizations.basis_state_version",
            basis_state_version,
        )?,
        status: row.get(5)?,
        attempt_scope_json: row.get(6)?,
        expires_at: row.get(7)?,
    })
}

fn artifact_staging_record(
    conn: &Connection,
    project_id: &str,
    handle_id: &str,
) -> StoreResult<Option<StoredArtifactStagingRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            handle_id,
            task_id,
            created_by_surface_id,
            created_by_surface_instance_id,
            artifact_json,
            tmp_path,
            sha256,
            size_bytes,
            content_type,
            redaction_state,
            status,
            expires_at
         FROM artifact_staging
         WHERE project_id = ?1
           AND handle_id = ?2",
        params![project_id, handle_id],
        artifact_staging_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn artifact_staging_record_tx(
    tx: &Transaction<'_>,
    project_id: &str,
    handle_id: &str,
) -> StoreResult<Option<StoredArtifactStagingRecord>> {
    tx.query_row(
        "SELECT
            project_id,
            handle_id,
            task_id,
            created_by_surface_id,
            created_by_surface_instance_id,
            artifact_json,
            tmp_path,
            sha256,
            size_bytes,
            content_type,
            redaction_state,
            status,
            expires_at
         FROM artifact_staging
         WHERE project_id = ?1
           AND handle_id = ?2",
        params![project_id, handle_id],
        artifact_staging_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn artifact_staging_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StoredArtifactStagingRecord> {
    let size_bytes = row
        .get::<_, Option<i64>>(8)?
        .map(|value| nonnegative_i64_to_u64("artifact_staging.size_bytes", value))
        .transpose()?;
    Ok(StoredArtifactStagingRecord {
        project_id: row.get(0)?,
        handle_id: row.get(1)?,
        task_id: row.get(2)?,
        created_by_surface_id: row.get(3)?,
        created_by_surface_instance_id: row.get(4)?,
        artifact_json: row.get(5)?,
        tmp_path: row.get(6)?,
        sha256: row.get(7)?,
        size_bytes,
        content_type: row.get(9)?,
        redaction_state: row.get(10)?,
        status: row.get(11)?,
        expires_at: row.get(12)?,
    })
}

fn artifact_record(
    conn: &Connection,
    project_id: &str,
    artifact_id: &str,
) -> StoreResult<Option<StoredArtifactRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            artifact_id,
            task_id,
            producer_run_id,
            source_staging_handle_id,
            uri,
            body_path,
            sha256,
            size_bytes,
            content_type,
            redaction_state,
            status,
            producer_json
         FROM artifacts
         WHERE project_id = ?1
           AND artifact_id = ?2",
        params![project_id, artifact_id],
        artifact_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn artifact_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredArtifactRecord> {
    let size_bytes = row
        .get::<_, Option<i64>>(8)?
        .map(|value| nonnegative_i64_to_u64("artifacts.size_bytes", value))
        .transpose()?;
    Ok(StoredArtifactRecord {
        project_id: row.get(0)?,
        artifact_id: row.get(1)?,
        task_id: row.get(2)?,
        producer_run_id: row.get(3)?,
        source_staging_handle_id: row.get(4)?,
        uri: row.get(5)?,
        body_path: row.get(6)?,
        sha256: row.get(7)?,
        size_bytes,
        content_type: row.get(9)?,
        redaction_state: row.get(10)?,
        status: row.get(11)?,
        producer_json: row.get(12)?,
    })
}

fn artifact_has_task_owner_link(
    conn: &Connection,
    project_id: &str,
    artifact_id: &str,
    task_id: &str,
) -> StoreResult<bool> {
    conn.query_row(
        "SELECT COUNT(*)
           FROM artifact_links
          WHERE project_id = ?1
            AND artifact_id = ?2
            AND task_id = ?3",
        params![project_id, artifact_id, task_id],
        |row| Ok(row.get::<_, i64>(0)? > 0),
    )
    .map_err(StoreError::from)
}

fn user_judgment_record(
    conn: &Connection,
    project_id: &str,
    judgment_id: &str,
) -> StoreResult<Option<UserJudgmentRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            judgment_id,
            task_id,
            change_unit_id,
            judgment_kind,
            status,
            request_json,
            context_json,
            options_json,
            affected_refs_json,
            artifact_refs_json,
            sensitive_action_scope_json,
            resolution_json,
            requested_by_surface_id,
            requested_by_surface_instance_id,
            requested_at,
            resolved_at,
            metadata_json
         FROM user_judgments
         WHERE project_id = ?1
           AND judgment_id = ?2",
        params![project_id, judgment_id],
        user_judgment_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn resolved_user_judgment_records(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    judgment_kind: &str,
) -> StoreResult<Vec<UserJudgmentRecord>> {
    let mut stmt = conn.prepare(
        "SELECT
            project_id,
            judgment_id,
            task_id,
            change_unit_id,
            judgment_kind,
            status,
            request_json,
            context_json,
            options_json,
            affected_refs_json,
            artifact_refs_json,
            sensitive_action_scope_json,
            resolution_json,
            requested_by_surface_id,
            requested_by_surface_instance_id,
            requested_at,
            resolved_at,
            metadata_json
         FROM user_judgments
         WHERE project_id = ?1
           AND task_id = ?2
           AND judgment_kind = ?3
           AND status = 'resolved'
         ORDER BY judgment_id",
    )?;
    let rows = stmt.query_map(
        params![project_id, task_id, judgment_kind],
        user_judgment_record_from_row,
    )?;
    let mut records = Vec::new();
    for row in rows {
        records.push(row?);
    }
    Ok(records)
}

fn user_judgment_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserJudgmentRecord> {
    Ok(UserJudgmentRecord {
        project_id: row.get(0)?,
        judgment_id: row.get(1)?,
        task_id: row.get(2)?,
        change_unit_id: row.get(3)?,
        judgment_kind: row.get(4)?,
        status: row.get(5)?,
        request_json: row.get(6)?,
        context_json: row.get(7)?,
        options_json: row.get(8)?,
        affected_refs_json: row.get(9)?,
        artifact_refs_json: row.get(10)?,
        sensitive_action_scope_json: row.get(11)?,
        resolution_json: row.get(12)?,
        requested_by_surface_id: row.get(13)?,
        requested_by_surface_instance_id: row.get(14)?,
        requested_at: row.get(15)?,
        resolved_at: row.get(16)?,
        metadata_json: row.get(17)?,
    })
}

struct RefQuery<'a> {
    project_id: &'a str,
    table: &'static str,
    id_column: &'static str,
    record_kind: &'static str,
    task_id: &'a str,
    status_column: &'static str,
    status_value: &'static str,
    state_version: u64,
}

fn task_scoped_refs(conn: &Connection, query: RefQuery<'_>) -> StoreResult<Vec<StoredRecordRef>> {
    let table = escape_sql_identifier(query.table);
    let id_column = escape_sql_identifier(query.id_column);
    let status_column = escape_sql_identifier(query.status_column);
    let sql = format!(
        "SELECT {id_column}
           FROM {table}
          WHERE project_id = ?1
            AND task_id = ?2
            AND {status_column} = ?3
          ORDER BY {id_column}"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        params![query.project_id, query.task_id, query.status_value],
        |row| row.get::<_, String>(0),
    )?;
    let mut refs = Vec::new();
    for row in rows {
        refs.push(StoredRecordRef {
            record_kind: query.record_kind.to_owned(),
            record_id: row?,
            project_id: query.project_id.to_owned(),
            task_id: Some(query.task_id.to_owned()),
            state_version: Some(query.state_version),
        });
    }
    Ok(refs)
}

fn read_project_state_tx(
    tx: &Transaction<'_>,
    project_id: &str,
) -> StoreResult<ProjectStateHeader> {
    tx.query_row(
        "SELECT
            project_id,
            state_version,
            active_task_id,
            default_surface_id,
            default_surface_instance_id
         FROM project_state
         WHERE project_id = ?1",
        params![project_id],
        project_state_from_row,
    )
    .optional()?
    .ok_or_else(|| StoreError::NotFound {
        entity: "project_state",
        id: project_id.to_owned(),
    })
}

fn project_state_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectStateHeader> {
    let state_version = row.get::<_, i64>(1)?;
    Ok(ProjectStateHeader {
        project_id: row.get(0)?,
        state_version: nonnegative_i64_to_u64("project_state.state_version", state_version)?,
        active_task_id: row.get(2)?,
        default_surface_id: row.get(3)?,
        default_surface_instance_id: row.get(4)?,
    })
}

fn surface_by_instance(
    conn: &Connection,
    project_id: &str,
    surface_id: &str,
    surface_instance_id: &str,
) -> StoreResult<Option<SurfaceRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            surface_id,
            surface_instance_id,
            surface_kind,
            display_name,
            capability_profile_json,
            local_access_json,
            metadata_json
         FROM surfaces
         WHERE project_id = ?1
           AND surface_id = ?2
           AND surface_instance_id = ?3",
        params![project_id, surface_id, surface_instance_id],
        surface_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn surface_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SurfaceRecord> {
    Ok(SurfaceRecord {
        project_id: row.get(0)?,
        surface_id: row.get(1)?,
        surface_instance_id: row.get(2)?,
        surface_kind: row.get(3)?,
        display_name: row.get(4)?,
        capability_profile_json: row.get(5)?,
        local_access_json: row.get(6)?,
        metadata_json: row.get(7)?,
    })
}

fn tool_invocation_tx(
    tx: &Transaction<'_>,
    project_id: &str,
    tool_name: &str,
    idempotency_key: &str,
) -> StoreResult<Option<ToolInvocationRecord>> {
    tx.query_row(
        "SELECT
            project_id,
            tool_name,
            idempotency_key,
            request_hash,
            basis_state_version,
            committed_state_version,
            response_json
         FROM tool_invocations
         WHERE project_id = ?1
           AND tool_name = ?2
           AND idempotency_key = ?3",
        params![project_id, tool_name, idempotency_key],
        |row| {
            let basis_state_version = row.get::<_, i64>(4)?;
            let committed_state_version = row.get::<_, i64>(5)?;
            Ok(ToolInvocationRecord {
                project_id: row.get(0)?,
                tool_name: row.get(1)?,
                idempotency_key: row.get(2)?,
                request_hash: row.get(3)?,
                basis_state_version: nonnegative_i64_to_u64(
                    "tool_invocations.basis_state_version",
                    basis_state_version,
                )?,
                committed_state_version: nonnegative_i64_to_u64(
                    "tool_invocations.committed_state_version",
                    committed_state_version,
                )?,
                response_json: row.get(6)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

fn tool_invocation(
    conn: &Connection,
    project_id: &str,
    tool_name: &str,
    idempotency_key: &str,
) -> StoreResult<Option<ToolInvocationRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            tool_name,
            idempotency_key,
            request_hash,
            basis_state_version,
            committed_state_version,
            response_json
         FROM tool_invocations
         WHERE project_id = ?1
           AND tool_name = ?2
           AND idempotency_key = ?3",
        params![project_id, tool_name, idempotency_key],
        |row| {
            let basis_state_version = row.get::<_, i64>(4)?;
            let committed_state_version = row.get::<_, i64>(5)?;
            Ok(ToolInvocationRecord {
                project_id: row.get(0)?,
                tool_name: row.get(1)?,
                idempotency_key: row.get(2)?,
                request_hash: row.get(3)?,
                basis_state_version: nonnegative_i64_to_u64(
                    "tool_invocations.basis_state_version",
                    basis_state_version,
                )?,
                committed_state_version: nonnegative_i64_to_u64(
                    "tool_invocations.committed_state_version",
                    committed_state_version,
                )?,
                response_json: row.get(6)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

fn next_event_seq(tx: &Transaction<'_>, project_id: &str) -> StoreResult<i64> {
    let last_seq: i64 = tx.query_row(
        "SELECT COALESCE(MAX(event_seq), 0)
           FROM task_events
          WHERE project_id = ?1",
        params![project_id],
        |row| row.get(0),
    )?;
    last_seq
        .checked_add(1)
        .ok_or_else(|| StoreError::SchemaInvariant {
            database_kind: "project_state",
            detail: "task_events.event_seq overflow".to_owned(),
        })
}

fn table_count(conn: &Connection, table: &str, project_id: &str) -> StoreResult<u64> {
    let escaped_table = table.replace('"', "\"\"");
    let sql = format!("SELECT COUNT(*) FROM \"{escaped_table}\" WHERE project_id = ?1");
    let count: i64 = conn.query_row(&sql, params![project_id], |row| row.get(0))?;
    nonnegative_i64_to_u64("table count", count).map_err(StoreError::from)
}

fn escape_sql_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn validate_pending_event(event: &PendingTaskEvent) -> StoreResult<()> {
    validate_identifier("event_id", &event.event_id)?;
    validate_identifier("task_id", &event.task_id)?;
    validate_identifier("event_kind", &event.event_kind)?;
    validate_json_text("task_events.event_payload_json", &event.event_payload_json)
}

fn validate_identifier(field: &'static str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}

fn validate_json_text(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<Value>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be JSON text: {error}"),
    })?;
    Ok(())
}

fn nonnegative_i64_to_u64(field: &'static str, value: i64) -> Result<u64, rusqlite::Error> {
    u64::try_from(value).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            format!("{field} must be nonnegative").into(),
        )
    })
}

fn u64_to_i64(field: &'static str, value: u64) -> StoreResult<i64> {
    i64::try_from(value).map_err(|_| StoreError::InvalidInput {
        detail: format!("{field} does not fit in SQLite integer"),
    })
}
