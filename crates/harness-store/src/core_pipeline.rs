use std::{
    fs,
    path::{Component, Path},
};

use harness_types::{
    BaselineRef, ChangeUnitId, CurrentCloseBasis, EvidenceCoverageItem, IdempotencyKey,
    JudgmentBasis, JudgmentBasisCompatibilityStatus, JudgmentResolutionOutcome, MethodName,
    ObservedChanges, PersistedArtifactProducer, PersistedArtifactProvenance,
    PersistedArtifactProvenanceMetadata, PersistedEvidenceMetadata, PersistedJudgmentBasis,
    PersistedUserJudgmentOptions, PersistedUserJudgmentRequest, PersistedUserJudgmentResolution,
    ProjectEnforcementProfile, ProjectEnforcementProfileSource, ProjectEnforcementProfileStatus,
    ProjectId, RequestHash, RequiredNullable, ResidualRisk, RunId, StagedArtifactHandleId,
    StateRecordRef, SurfaceId, TaskId, UserJudgmentOptionAction, UtcTimestamp,
    BASELINE_COOPERATIVE_ENFORCEMENT_PROFILE_ID,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    artifacts::{
        verify_persistent_artifact_body as verify_persistent_artifact_body_in_store,
        PersistentArtifactBodySpec, PersistentArtifactVerification,
    },
    bootstrap::{project_record_for_execution, ProjectRecord, SurfaceRecord},
    sqlite::{begin_immediate_transaction, open_project_state_database, ARTIFACTS_DIR},
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

/// Strict-decoded project-owned enforcement profile row.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectEnforcementProfileRecord {
    pub project_id: String,
    pub enforcement_profile_json: String,
    pub profile: ProjectEnforcementProfile,
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
    pub surface_id: String,
    pub surface_instance_id: String,
    pub access_class: String,
    pub verification_basis: Option<String>,
    pub response_json: String,
}

/// Verified replay identity derived from the current surface context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedReplayContext {
    pub surface_id: String,
    pub surface_instance_id: String,
    pub access_class: String,
    pub verification_basis: Option<String>,
}

impl ToolInvocationRecord {
    /// Returns whether this replay row is eligible for the supplied verified context.
    pub fn matches_verified_replay_context(&self, context: &VerifiedReplayContext) -> bool {
        self.surface_id == context.surface_id.as_str()
            && self.surface_instance_id == context.surface_instance_id.as_str()
            && self.access_class == context.access_class.as_str()
    }
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
    CloseTask(TaskCloseUpdate),
    UpdateTaskScope(TaskScopeUpdate),
    UpdateTaskScopeRevision(TaskScopeRevisionUpdate),
    UpdateTaskCloseBasis(TaskCloseBasisUpdate),
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
    UpdateUserJudgmentBasis(UserJudgmentBasisUpdate),
    MarkUserJudgmentBasesStatus(UserJudgmentBasisStatusMark),
    MarkUserJudgmentsSupersededOrStale(UserJudgmentInvalidation),
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
    pub basis_json: String,
    pub basis_status: JudgmentBasisCompatibilityStatus,
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
    pub resolution_outcome: JudgmentResolutionOutcome,
    pub resolution_machine_action: UserJudgmentOptionAction,
    pub resolution_json: String,
    pub sensitive_action_scope_json: Option<String>,
    pub resolved_by_actor_kind: String,
    pub resolved_actor_role: String,
    pub resolved_by_surface_id: String,
    pub resolved_by_surface_instance_id: String,
    pub resolved_verification_basis: String,
    pub resolved_assurance_level: String,
    pub resolved_at: String,
}

/// Storage input for replacing one judgment basis snapshot and compatibility status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserJudgmentBasisUpdate {
    pub judgment_id: String,
    pub basis_json: String,
    pub basis_status: JudgmentBasisCompatibilityStatus,
}

/// Storage input for marking selected judgment basis rows stale or superseded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserJudgmentBasisStatusMark {
    pub judgment_ids: Vec<String>,
    pub basis_status: JudgmentBasisCompatibilityStatus,
}

/// Storage input for invalidating current judgment authority after state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserJudgmentInvalidation {
    pub task_id: String,
    pub judgment_kinds: Vec<String>,
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
    pub created_at: String,
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
    pub scope_revision: u64,
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
    pub expected_expires_at: String,
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

/// Stored evidence summary facts needed by close-readiness evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSummaryRecord {
    pub project_id: String,
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
    pub replay_context: Option<VerifiedReplayContext>,
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
    ReplayContextMismatch {
        current_state_version: u64,
        idempotency_key: String,
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
    pub scope_revision: u64,
    pub close_basis_revision: u64,
    pub close_basis_json: Option<String>,
    pub close_summary_json: String,
    pub completion_policy_json: String,
    pub current_change_unit_id: Option<String>,
    pub closed_at: Option<String>,
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

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyCategoryOnlyCloseBasis {
    close_basis_revision: u64,
    scope_revision: u64,
    task_id: TaskId,
    change_unit_id: ChangeUnitId,
    baseline_ref: RequiredNullable<BaselineRef>,
    result_summary: String,
    result_refs: Vec<StateRecordRef>,
    evidence_summary_ref: RequiredNullable<StateRecordRef>,
    residual_risks: Vec<ResidualRisk>,
    sensitive_categories: Vec<String>,
    recovery_constraints: Vec<String>,
    source_run_ref: StateRecordRef,
    updated_at: UtcTimestamp,
}

impl From<LegacyCategoryOnlyCloseBasis> for CurrentCloseBasis {
    fn from(value: LegacyCategoryOnlyCloseBasis) -> Self {
        Self {
            close_basis_revision: value.close_basis_revision,
            scope_revision: value.scope_revision,
            task_id: value.task_id,
            change_unit_id: value.change_unit_id,
            baseline_ref: value.baseline_ref,
            result_summary: value.result_summary,
            result_refs: value.result_refs,
            evidence_summary_ref: value.evidence_summary_ref,
            residual_risks: value.residual_risks,
            sensitive_categories: value.sensitive_categories,
            sensitive_action_requirements: Vec::new(),
            recovery_constraints: value.recovery_constraints,
            source_run_ref: value.source_run_ref,
            updated_at: value.updated_at,
        }
    }
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
    pub created_at: String,
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
    pub integrity_status: String,
    pub redaction_state: String,
    pub status: String,
    pub producer: PersistedArtifactProducer,
    pub provenance: PersistedArtifactProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredArtifactRecordRaw {
    project_id: String,
    artifact_id: String,
    task_id: String,
    producer_run_id: Option<String>,
    source_staging_handle_id: Option<String>,
    uri: String,
    body_path: Option<String>,
    sha256: Option<String>,
    size_bytes: Option<u64>,
    content_type: Option<String>,
    integrity_status: String,
    redaction_state: String,
    status: String,
    producer_json: String,
    metadata_json: String,
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
    pub basis_json: String,
    pub basis_status: String,
    pub resolution_outcome: Option<String>,
    pub resolution_machine_action: Option<String>,
    pub resolution_json: Option<String>,
    pub resolved_by_actor_kind: Option<String>,
    pub resolved_actor_role: Option<String>,
    pub resolved_by_surface_id: Option<String>,
    pub resolved_by_surface_instance_id: Option<String>,
    pub resolved_verification_basis: Option<String>,
    pub resolved_assurance_level: Option<String>,
    pub requested_by_surface_id: String,
    pub requested_by_surface_instance_id: String,
    pub requested_at: String,
    pub resolved_at: Option<String>,
    pub metadata_json: String,
}

/// Stored judgment-basis facts with strict-decoded typed JSON when present.
#[derive(Debug, Clone, PartialEq)]
pub struct UserJudgmentBasisRecord {
    pub project_id: String,
    pub judgment_id: String,
    pub basis_json: String,
    pub basis_status: JudgmentBasisCompatibilityStatus,
    pub basis: JudgmentBasis,
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
    project_home: &'tx Path,
    tx: &'tx Transaction<'tx>,
}

impl CoreProjectStore {
    /// Opens the registered project-local state store for Core pipeline work.
    pub fn open(runtime_home: impl AsRef<Path>, project_id: &ProjectId) -> StoreResult<Self> {
        let project =
            project_record_for_execution(runtime_home, project_id.as_str())?.ok_or_else(|| {
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

    /// Reads and strictly validates the active project enforcement profile.
    pub fn project_enforcement_profile(&self) -> StoreResult<ProjectEnforcementProfileRecord> {
        project_enforcement_profile(&self.conn, &self.project.project_id)
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
                interaction_role,
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

    /// Returns whether a Change Unit id already exists in this project.
    pub fn change_unit_id_exists(&self, change_unit_id: &str) -> StoreResult<bool> {
        row_exists(
            &self.conn,
            &self.project.project_id,
            "change_units",
            "change_unit_id",
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

    /// Lists Write Authorizations for a Task without mutating effective status.
    pub fn write_authorizations_for_task(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Vec<WriteAuthorizationRecord>> {
        write_authorizations_for_task(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Reads one Write Authorization row by exact project-local identity.
    pub fn write_authorization_record(
        &self,
        write_authorization_id: &str,
    ) -> StoreResult<Option<WriteAuthorizationRecord>> {
        write_authorization_record(&self.conn, &self.project.project_id, write_authorization_id)
    }

    /// Returns whether a Run id already exists in this project.
    pub fn run_id_exists(&self, run_id: &str) -> StoreResult<bool> {
        row_exists(
            &self.conn,
            &self.project.project_id,
            "runs",
            "run_id",
            run_id,
        )
    }

    /// Returns whether a Run belongs to a Task in this project.
    pub fn run_belongs_to_task(&self, run_id: &str, task_id: &str) -> StoreResult<bool> {
        row_exists_with_task(
            &self.conn,
            &self.project.project_id,
            "runs",
            "run_id",
            run_id,
            task_id,
        )
    }

    /// Reads one committed Run row by exact project-local identity.
    pub fn run_record(&self, run_id: &str) -> StoreResult<Option<RunRecord>> {
        run_record(&self.conn, &self.project.project_id, run_id)
    }

    /// Reads one staged artifact row by exact project-local handle identity.
    pub fn artifact_staging_record(
        &self,
        handle_id: &str,
    ) -> StoreResult<Option<StoredArtifactStagingRecord>> {
        artifact_staging_record(&self.conn, &self.project.project_id, handle_id)
    }

    /// Returns whether a committed event id already exists in this project.
    pub fn event_id_exists(&self, event_id: &str) -> StoreResult<bool> {
        row_exists(
            &self.conn,
            &self.project.project_id,
            "task_events",
            "event_id",
            event_id,
        )
    }

    /// Reads one persistent artifact row by exact project-local artifact identity.
    pub fn artifact_record(&self, artifact_id: &str) -> StoreResult<Option<StoredArtifactRecord>> {
        artifact_record(&self.conn, &self.project.project_id, artifact_id)
    }

    /// Verifies the current persistent body bytes for an artifact row.
    pub fn verify_persistent_artifact_body(
        &self,
        record: &StoredArtifactRecord,
    ) -> StoreResult<PersistentArtifactVerification> {
        let artifact_store_root = self.project.project_home.join(ARTIFACTS_DIR);
        verify_persistent_artifact_body_in_store(
            &artifact_store_root,
            &PersistentArtifactBodySpec {
                body_path: record.body_path.as_deref(),
                sha256: record.sha256.as_deref(),
                size_bytes: record.size_bytes,
                content_type: record.content_type.as_deref(),
                integrity_status: &record.integrity_status,
                availability_status: &record.status,
            },
        )
    }

    /// Returns whether an evidence summary id already exists in this project.
    pub fn evidence_summary_exists(&self, evidence_summary_id: &str) -> StoreResult<bool> {
        row_exists(
            &self.conn,
            &self.project.project_id,
            "evidence_summaries",
            "evidence_summary_id",
            evidence_summary_id,
        )
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

    /// Lists pending user-owned judgment records for a Task.
    pub fn pending_user_judgment_records(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Vec<UserJudgmentRecord>> {
        pending_user_judgment_records(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Lists stale or superseded user-judgment refs for a Task and judgment kind.
    pub fn non_current_user_judgment_refs(
        &self,
        task_id: &TaskId,
        judgment_kind: &str,
        state_version: u64,
    ) -> StoreResult<Vec<StoredRecordRef>> {
        non_current_user_judgment_refs(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            judgment_kind,
            state_version,
        )
    }

    /// Reads one user-owned judgment row by project-local judgment identity.
    pub fn user_judgment_record(
        &self,
        judgment_id: &str,
    ) -> StoreResult<Option<UserJudgmentRecord>> {
        user_judgment_record(&self.conn, &self.project.project_id, judgment_id)
    }

    /// Reads one user-owned judgment basis row with strict typed JSON decoding.
    pub fn user_judgment_basis_record(
        &self,
        judgment_id: &str,
    ) -> StoreResult<Option<UserJudgmentBasisRecord>> {
        user_judgment_basis_record(&self.conn, &self.project.project_id, judgment_id)
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
        if input.idempotency_key.is_some() {
            let replay_context =
                input
                    .replay_context
                    .as_ref()
                    .ok_or_else(|| StoreError::InvalidInput {
                        detail: "idempotent commits require verified replay context".to_owned(),
                    })?;
            validate_replay_context(replay_context)?;
        }
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
                let replay_context =
                    input
                        .replay_context
                        .as_ref()
                        .ok_or_else(|| StoreError::InvalidInput {
                            detail: "idempotent commits require verified replay context".to_owned(),
                        })?;
                if !record.matches_verified_replay_context(replay_context) {
                    return Ok(MutationCommitOutcome::ReplayContextMismatch {
                        current_state_version: current.state_version,
                        idempotency_key: idempotency_key.clone(),
                    });
                }
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
            project_home: &self.project.project_home,
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
            let replay_context = input
                .replay_context
                .as_ref()
                .expect("validated replay_context is present");
            tx.execute(
                "INSERT INTO tool_invocations (
                    project_id,
                    tool_name,
                    idempotency_key,
                    request_hash,
                    basis_state_version,
                    committed_state_version,
                    surface_id,
                    surface_instance_id,
                    access_class,
                    verification_basis,
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
                    ?8,
                    ?9,
                    ?10,
                    ?11,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                )",
                params![
                    self.project.project_id,
                    input.tool_name,
                    idempotency_key,
                    input.request_hash,
                    current_state_i64,
                    committed_state_i64,
                    replay_context.surface_id.as_str(),
                    replay_context.surface_instance_id.as_str(),
                    replay_context.access_class.as_str(),
                    replay_context.verification_basis.as_deref(),
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
    replay_context: Option<VerifiedReplayContext>,
    expected_state_version: Option<u64>,
    events: Vec<PendingTaskEvent>,
) -> CommitMutationInput {
    CommitMutationInput {
        project_id: project_id.as_str().to_owned(),
        tool_name: method_name.as_str().to_owned(),
        idempotency_key: idempotency_key.map(|key| key.as_str().to_owned()),
        request_hash: request_hash.as_str().to_owned(),
        replay_context,
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
            Self::CloseTask(input) => mutation.close_task(input),
            Self::UpdateTaskScope(input) => mutation.update_task_scope(input),
            Self::UpdateTaskScopeRevision(input) => mutation.update_task_scope_revision(input),
            Self::UpdateTaskCloseBasis(input) => mutation.update_task_close_basis(input),
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
            Self::UpdateUserJudgmentBasis(input) => mutation.update_user_judgment_basis(input),
            Self::MarkUserJudgmentBasesStatus(input) => {
                mutation.mark_user_judgment_bases_status(input)
            }
            Self::MarkUserJudgmentsSupersededOrStale(input) => {
                mutation.mark_user_judgments_superseded_or_stale(input)
            }
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

    fn close_task(&mut self, input: &TaskCloseUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("lifecycle_phase", &input.lifecycle_phase)?;
        validate_identifier("result", &input.result)?;
        validate_json_text("tasks.close_summary_json", &input.close_summary_json)?;
        validate_identifier("closed_at", &input.closed_at)?;

        let changed = self.tx.execute(
            "UPDATE tasks
                SET lifecycle_phase = ?3,
                    result = ?4,
                    close_summary_json = ?5,
                    closed_at = ?6,
                    updated_at = ?6
              WHERE project_id = ?1
                AND task_id = ?2",
            params![
                self.project_id,
                input.task_id,
                input.lifecycle_phase,
                input.result,
                input.close_summary_json,
                input.closed_at
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

    fn update_task_scope_revision(&mut self, input: &TaskScopeRevisionUpdate) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        let scope_revision = u64_to_i64("tasks.scope_revision", input.scope_revision)?;
        let changed = self.tx.execute(
            "UPDATE tasks
                SET scope_revision = ?3,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND task_id = ?2",
            params![self.project_id, input.task_id, scope_revision],
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
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
              WHERE project_id = ?1
                AND task_id = ?2",
            params![
                self.project_id,
                input.task_id,
                close_basis_revision,
                input.close_basis_json
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
        validate_identifier("created_at", &input.created_at)?;
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
                ?11,
                ?12
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
                input.created_at,
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
        let scope_revision = u64_to_i64("runs.scope_revision", input.scope_revision)?;
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
                scope_revision,
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
                ?14,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?15
            )",
            params![
                self.project_id,
                input.run_id,
                input.task_id,
                input.change_unit_id,
                scope_revision,
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
        validate_artifact_sha256("expected_sha256", &input.expected_sha256)?;
        validate_identifier("expected_redaction_state", &input.expected_redaction_state)?;
        validate_timestamp("expected_expires_at", &input.expected_expires_at)?;
        validate_identifier("artifacts.uri", &input.uri)?;
        validate_json_text("artifacts.retention_json", &input.retention_json)?;
        validate_artifact_producer_json("artifacts.producer_json", &input.producer_json)?;
        validate_artifact_provenance_metadata_json(
            "artifacts.metadata_json",
            &input.metadata_json,
        )?;

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
            || staging.expires_at != input.expected_expires_at
        {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "staged artifact changed before promotion".to_owned(),
            });
        }
        verify_staged_artifact_body(
            self.project_home,
            staging.tmp_path.as_deref(),
            &input.expected_sha256,
            input.expected_size_bytes,
        )?;

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
                integrity_status,
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
                'verified',
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
        validate_evidence_coverage_json("evidence_summaries.coverage_json", &input.coverage_json)?;
        validate_state_refs_json(
            "evidence_summaries.supporting_refs_json",
            &input.supporting_refs_json,
        )?;
        validate_state_refs_json("evidence_summaries.gap_refs_json", &input.gap_refs_json)?;
        validate_evidence_metadata_json("evidence_summaries.metadata_json", &input.metadata_json)?;

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
        validate_user_judgment_request_json("user_judgments.request_json", &input.request_json)?;
        validate_json_text("user_judgments.context_json", &input.context_json)?;
        validate_user_judgment_options_json("user_judgments.options_json", &input.options_json)?;
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
        validate_judgment_basis_json("user_judgments.basis_json", &input.basis_json)?;
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
                basis_json,
                basis_status,
                resolution_outcome,
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
                ?12,
                ?13,
                NULL,
                NULL,
                ?14,
                ?15,
                ?16,
                NULL,
                ?17
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
                input.basis_json,
                judgment_basis_status_as_str(input.basis_status),
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
        let resolution_outcome = judgment_resolution_outcome_as_str(input.resolution_outcome);
        let resolution_machine_action =
            judgment_machine_action_as_str(input.resolution_machine_action);
        if input.resolution_machine_action.resolution_outcome() != input.resolution_outcome {
            return Err(StoreError::InvalidInput {
                detail: "user_judgments.resolution_machine_action must match resolution_outcome"
                    .to_owned(),
            });
        }
        validate_user_judgment_resolution_json(
            "user_judgments.resolution_json",
            &input.resolution_json,
            input.resolution_machine_action,
            input.resolution_outcome,
        )?;
        if let Some(value) = &input.sensitive_action_scope_json {
            validate_json_text("user_judgments.sensitive_action_scope_json", value)?;
        }
        validate_actor_kind_value("resolved_by_actor_kind", &input.resolved_by_actor_kind)?;
        validate_interaction_role_value("resolved_actor_role", &input.resolved_actor_role)?;
        validate_identifier("resolved_by_surface_id", &input.resolved_by_surface_id)?;
        validate_identifier(
            "resolved_by_surface_instance_id",
            &input.resolved_by_surface_instance_id,
        )?;
        validate_identifier(
            "resolved_verification_basis",
            &input.resolved_verification_basis,
        )?;
        validate_identifier("resolved_assurance_level", &input.resolved_assurance_level)?;
        validate_identifier("resolved_at", &input.resolved_at)?;

        let changed = self.tx.execute(
            "UPDATE user_judgments
                SET status = ?3,
                    resolution_outcome = ?4,
                    resolution_machine_action = ?5,
                    resolution_json = ?6,
                    sensitive_action_scope_json = COALESCE(?7, sensitive_action_scope_json),
                    resolved_by_actor_kind = ?8,
                    resolved_actor_role = ?9,
                    resolved_by_surface_id = ?10,
                    resolved_by_surface_instance_id = ?11,
                    resolved_verification_basis = ?12,
                    resolved_assurance_level = ?13,
                    resolved_at = ?14
              WHERE project_id = ?1
                AND judgment_id = ?2
                AND status = 'pending'",
            params![
                self.project_id,
                input.judgment_id,
                input.status,
                resolution_outcome,
                resolution_machine_action,
                input.resolution_json,
                input.sensitive_action_scope_json,
                input.resolved_by_actor_kind,
                input.resolved_actor_role,
                input.resolved_by_surface_id,
                input.resolved_by_surface_instance_id,
                input.resolved_verification_basis,
                input.resolved_assurance_level,
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

    fn update_user_judgment_basis(&mut self, input: &UserJudgmentBasisUpdate) -> StoreResult<()> {
        validate_identifier("judgment_id", &input.judgment_id)?;
        validate_judgment_basis_json("user_judgments.basis_json", &input.basis_json)?;
        let changed = self.tx.execute(
            "UPDATE user_judgments
                SET basis_json = ?3,
                    basis_status = ?4
              WHERE project_id = ?1
                AND judgment_id = ?2",
            params![
                self.project_id,
                input.judgment_id,
                input.basis_json,
                judgment_basis_status_as_str(input.basis_status)
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "user judgment basis update changed no rows".to_owned(),
            })
        }
    }

    fn mark_user_judgment_bases_status(
        &mut self,
        input: &UserJudgmentBasisStatusMark,
    ) -> StoreResult<()> {
        let status = match input.basis_status {
            JudgmentBasisCompatibilityStatus::Stale
            | JudgmentBasisCompatibilityStatus::Superseded => {
                judgment_basis_status_as_str(input.basis_status)
            }
            _ => {
                return Err(StoreError::InvalidInput {
                    detail: "selected judgment bases may only be marked stale or superseded"
                        .to_owned(),
                })
            }
        };

        for judgment_id in &input.judgment_ids {
            validate_identifier("judgment_id", judgment_id)?;
            let changed = self.tx.execute(
                "UPDATE user_judgments
                    SET basis_status = ?3
                  WHERE project_id = ?1
                    AND judgment_id = ?2",
                params![self.project_id, judgment_id, status],
            )?;
            if changed != 1 {
                return Err(StoreError::SchemaInvariant {
                    database_kind: "project_state",
                    detail: format!(
                        "selected user judgment basis status update changed {changed} rows"
                    ),
                });
            }
        }

        Ok(())
    }

    fn mark_user_judgments_superseded_or_stale(
        &mut self,
        input: &UserJudgmentInvalidation,
    ) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        if input.judgment_kinds.is_empty() {
            self.mark_user_judgments_superseded_or_stale_for_kind(&input.task_id, None)?;
        } else {
            for judgment_kind in &input.judgment_kinds {
                validate_identifier("judgment_kind", judgment_kind)?;
                self.mark_user_judgments_superseded_or_stale_for_kind(
                    &input.task_id,
                    Some(judgment_kind),
                )?;
            }
        }
        Ok(())
    }

    fn mark_user_judgments_superseded_or_stale_for_kind(
        &mut self,
        task_id: &str,
        judgment_kind: Option<&str>,
    ) -> StoreResult<()> {
        match judgment_kind {
            Some(judgment_kind) => {
                self.tx.execute(
                    "UPDATE user_judgments
                        SET status = 'superseded',
                            basis_status = 'superseded'
                      WHERE project_id = ?1
                        AND task_id = ?2
                        AND judgment_kind = ?3
                        AND status = 'pending'
                        AND basis_status = 'current'",
                    params![self.project_id, task_id, judgment_kind],
                )?;
                self.tx.execute(
                    "UPDATE user_judgments
                        SET status = 'stale',
                            basis_status = 'stale'
                      WHERE project_id = ?1
                        AND task_id = ?2
                        AND judgment_kind = ?3
                        AND status = 'resolved'
                        AND basis_status = 'current'",
                    params![self.project_id, task_id, judgment_kind],
                )?;
            }
            None => {
                self.tx.execute(
                    "UPDATE user_judgments
                        SET status = 'superseded',
                            basis_status = 'superseded'
                      WHERE project_id = ?1
                        AND task_id = ?2
                        AND status = 'pending'
                        AND basis_status = 'current'",
                    params![self.project_id, task_id],
                )?;
                self.tx.execute(
                    "UPDATE user_judgments
                        SET status = 'stale',
                            basis_status = 'stale'
                      WHERE project_id = ?1
                        AND task_id = ?2
                        AND status = 'resolved'
                        AND basis_status = 'current'",
                    params![self.project_id, task_id],
                )?;
            }
        }
        Ok(())
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

fn project_enforcement_profile(
    conn: &Connection,
    project_id: &str,
) -> StoreResult<ProjectEnforcementProfileRecord> {
    let (row_project_id, enforcement_profile_json): (String, String) = conn
        .query_row(
            "SELECT project_id, enforcement_profile_json
               FROM project_state
              WHERE project_id = ?1",
            params![project_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project_state",
            id: project_id.to_owned(),
        })?;
    let profile = serde_json::from_str::<ProjectEnforcementProfile>(&enforcement_profile_json)
        .map_err(|_| {
            StoreError::corrupt_owner_state_json(
                "project_state",
                row_project_id.clone(),
                "enforcement_profile_json",
            )
        })?;
    validate_project_enforcement_profile(&profile, &row_project_id)?;
    Ok(ProjectEnforcementProfileRecord {
        project_id: row_project_id,
        enforcement_profile_json,
        profile,
    })
}

fn validate_project_enforcement_profile(
    profile: &ProjectEnforcementProfile,
    project_id: &str,
) -> StoreResult<()> {
    let unsupported = || {
        StoreError::corrupt_owner_state_value(
            "project_state",
            project_id.to_owned(),
            "enforcement_profile_json",
        )
    };
    if profile.profile_id.trim().is_empty() {
        return Err(unsupported());
    }
    if profile.profile_id != BASELINE_COOPERATIVE_ENFORCEMENT_PROFILE_ID {
        return Err(unsupported());
    }
    if profile.guarantee_level != harness_types::GuaranteeLevel::Cooperative {
        return Err(unsupported());
    }
    if !profile.enabled_mechanisms.is_empty() {
        return Err(unsupported());
    }
    if profile.source != ProjectEnforcementProfileSource::BaselineScope {
        return Err(unsupported());
    }
    if profile.status != ProjectEnforcementProfileStatus::Active {
        return Err(unsupported());
    }
    Ok(())
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
            scope_revision,
            close_basis_revision,
            close_basis_json,
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
        scope_revision: nonnegative_i64_to_u64("tasks.scope_revision", row.get(10)?)?,
        close_basis_revision: nonnegative_i64_to_u64("tasks.close_basis_revision", row.get(11)?)?,
        close_basis_json: row.get(12)?,
        close_summary_json: row.get(13)?,
        completion_policy_json: row.get(14)?,
        current_change_unit_id: row.get(15)?,
        closed_at: row.get(16)?,
    })
}

fn task_revision_record(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Option<TaskRevisionRecord>> {
    let row = conn
        .query_row(
            "SELECT
                project_id,
                task_id,
                scope_revision,
                close_basis_revision,
                close_basis_json
             FROM tasks
             WHERE project_id = ?1
               AND task_id = ?2",
            params![project_id, task_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
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
            expires_at,
            created_at
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

fn write_authorizations_for_task(
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
            expires_at,
            created_at
         FROM write_authorizations
         WHERE project_id = ?1
           AND task_id = ?2
         ORDER BY created_at DESC, write_authorization_id DESC",
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
            expires_at,
            created_at
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
        created_at: row.get(8)?,
    })
}

fn run_record(conn: &Connection, project_id: &str, run_id: &str) -> StoreResult<Option<RunRecord>> {
    let row = conn
        .query_row(
            "SELECT
            project_id,
            run_id,
            task_id,
            change_unit_id,
            scope_revision,
            observed_changes_json,
            status
         FROM runs
         WHERE project_id = ?1
           AND run_id = ?2",
            params![project_id, run_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .optional()?;

    row.map(
        |(
            project_id,
            run_id,
            task_id,
            change_unit_id,
            scope_revision,
            observed_changes_json,
            status,
        )| {
            let scope_revision = u64::try_from(scope_revision).map_err(|_| {
                StoreError::corrupt_owner_state_value("runs", run_id.clone(), "scope_revision")
            })?;
            let observed_changes = decode_owner_json_text::<ObservedChanges>(
                "runs",
                run_id.clone(),
                "observed_changes_json",
                &observed_changes_json,
            )?;
            Ok(RunRecord {
                project_id,
                run_id,
                task_id,
                change_unit_id,
                scope_revision,
                baseline_ref: observed_changes
                    .baseline_ref
                    .as_ref()
                    .map(|baseline_ref| baseline_ref.as_str().to_owned()),
                status,
            })
        },
    )
    .transpose()
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
    let row = conn
        .query_row(
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
            integrity_status,
            redaction_state,
            status,
            producer_json,
            metadata_json
         FROM artifacts
         WHERE project_id = ?1
           AND artifact_id = ?2",
            params![project_id, artifact_id],
            artifact_record_raw_from_row,
        )
        .optional()?;
    row.map(stored_artifact_record_from_raw).transpose()
}

fn stored_artifact_record_from_raw(
    raw: StoredArtifactRecordRaw,
) -> StoreResult<StoredArtifactRecord> {
    let producer = decode_owner_json_text::<PersistedArtifactProducer>(
        "artifacts",
        raw.artifact_id.clone(),
        "producer_json",
        &raw.producer_json,
    )?;
    let provenance_metadata = decode_owner_json_text::<PersistedArtifactProvenanceMetadata>(
        "artifacts",
        raw.artifact_id.clone(),
        "metadata_json",
        &raw.metadata_json,
    )?;
    let producer_run_id = raw.producer_run_id.as_ref().ok_or_else(|| {
        StoreError::corrupt_owner_state_value(
            "artifacts",
            raw.artifact_id.clone(),
            "producer_run_id",
        )
    })?;
    let source_staging_handle_id = raw.source_staging_handle_id.as_ref().ok_or_else(|| {
        StoreError::corrupt_owner_state_value(
            "artifacts",
            raw.artifact_id.clone(),
            "source_staging_handle_id",
        )
    })?;
    let provenance = PersistedArtifactProvenance {
        source_kind: provenance_metadata.source_kind,
        producer_run_id: RunId::new(producer_run_id.clone()),
        source_staging_handle_id: StagedArtifactHandleId::new(source_staging_handle_id.clone()),
    };
    Ok(StoredArtifactRecord {
        project_id: raw.project_id,
        artifact_id: raw.artifact_id,
        task_id: raw.task_id,
        producer_run_id: raw.producer_run_id,
        source_staging_handle_id: raw.source_staging_handle_id,
        uri: raw.uri,
        body_path: raw.body_path,
        sha256: raw.sha256,
        size_bytes: raw.size_bytes,
        content_type: raw.content_type,
        integrity_status: raw.integrity_status,
        redaction_state: raw.redaction_state,
        status: raw.status,
        producer,
        provenance,
    })
}

fn artifact_record_raw_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StoredArtifactRecordRaw> {
    let size_bytes = row
        .get::<_, Option<i64>>(8)?
        .map(|value| nonnegative_i64_to_u64("artifacts.size_bytes", value))
        .transpose()?;
    Ok(StoredArtifactRecordRaw {
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
        integrity_status: row.get(10)?,
        redaction_state: row.get(11)?,
        status: row.get(12)?,
        producer_json: row.get(13)?,
        metadata_json: row.get(14)?,
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

fn latest_evidence_summary(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Option<EvidenceSummaryRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            evidence_summary_id,
            task_id,
            change_unit_id,
            status,
            coverage_json,
            supporting_refs_json,
            gap_refs_json,
            metadata_json
         FROM evidence_summaries
         WHERE project_id = ?1
           AND task_id = ?2
         ORDER BY updated_at DESC, evidence_summary_id DESC
         LIMIT 1",
        params![project_id, task_id],
        evidence_summary_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn evidence_summary_record(
    conn: &Connection,
    project_id: &str,
    evidence_summary_id: &str,
) -> StoreResult<Option<EvidenceSummaryRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            evidence_summary_id,
            task_id,
            change_unit_id,
            status,
            coverage_json,
            supporting_refs_json,
            gap_refs_json,
            metadata_json
         FROM evidence_summaries
         WHERE project_id = ?1
           AND evidence_summary_id = ?2",
        params![project_id, evidence_summary_id],
        evidence_summary_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn evidence_summary_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<EvidenceSummaryRecord> {
    Ok(EvidenceSummaryRecord {
        project_id: row.get(0)?,
        evidence_summary_id: row.get(1)?,
        task_id: row.get(2)?,
        change_unit_id: row.get(3)?,
        status: row.get(4)?,
        coverage_json: row.get(5)?,
        supporting_refs_json: row.get(6)?,
        gap_refs_json: row.get(7)?,
        metadata_json: row.get(8)?,
    })
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
            basis_json,
            basis_status,
            resolution_outcome,
            resolution_machine_action,
            resolution_json,
            resolved_by_actor_kind,
            resolved_actor_role,
            resolved_by_surface_id,
            resolved_by_surface_instance_id,
            resolved_verification_basis,
            resolved_assurance_level,
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
            basis_json,
            basis_status,
            resolution_outcome,
            resolution_machine_action,
            resolution_json,
            resolved_by_actor_kind,
            resolved_actor_role,
            resolved_by_surface_id,
            resolved_by_surface_instance_id,
            resolved_verification_basis,
            resolved_assurance_level,
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

fn pending_user_judgment_records(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
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
            basis_json,
            basis_status,
            resolution_outcome,
            resolution_machine_action,
            resolution_json,
            resolved_by_actor_kind,
            resolved_actor_role,
            resolved_by_surface_id,
            resolved_by_surface_instance_id,
            resolved_verification_basis,
            resolved_assurance_level,
            requested_by_surface_id,
            requested_by_surface_instance_id,
            requested_at,
            resolved_at,
            metadata_json
         FROM user_judgments
         WHERE project_id = ?1
           AND task_id = ?2
           AND status = 'pending'
         ORDER BY judgment_id",
    )?;
    let rows = stmt.query_map(params![project_id, task_id], user_judgment_record_from_row)?;
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
        basis_json: row.get(12)?,
        basis_status: row.get(13)?,
        resolution_outcome: row.get(14)?,
        resolution_machine_action: row.get(15)?,
        resolution_json: row.get(16)?,
        resolved_by_actor_kind: row.get(17)?,
        resolved_actor_role: row.get(18)?,
        resolved_by_surface_id: row.get(19)?,
        resolved_by_surface_instance_id: row.get(20)?,
        resolved_verification_basis: row.get(21)?,
        resolved_assurance_level: row.get(22)?,
        requested_by_surface_id: row.get(23)?,
        requested_by_surface_instance_id: row.get(24)?,
        requested_at: row.get(25)?,
        resolved_at: row.get(26)?,
        metadata_json: row.get(27)?,
    })
}

fn user_judgment_basis_record(
    conn: &Connection,
    project_id: &str,
    judgment_id: &str,
) -> StoreResult<Option<UserJudgmentBasisRecord>> {
    let row = conn
        .query_row(
            "SELECT
                project_id,
                judgment_id,
                basis_json,
                basis_status
             FROM user_judgments
             WHERE project_id = ?1
               AND judgment_id = ?2",
            params![project_id, judgment_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;

    let Some((project_id, judgment_id, basis_json, basis_status)) = row else {
        return Ok(None);
    };
    let basis_status =
        parse_judgment_basis_status(&judgment_id, "user_judgments.basis_status", &basis_status)?;
    let basis = decode_judgment_basis_column(&judgment_id, &basis_json)?;

    Ok(Some(UserJudgmentBasisRecord {
        project_id,
        judgment_id,
        basis_json,
        basis_status,
        basis,
    }))
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

fn non_current_user_judgment_refs(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    judgment_kind: &str,
    state_version: u64,
) -> StoreResult<Vec<StoredRecordRef>> {
    let mut stmt = conn.prepare(
        "SELECT judgment_id
           FROM user_judgments
          WHERE project_id = ?1
            AND task_id = ?2
            AND judgment_kind = ?3
            AND status IN ('stale', 'superseded')
          ORDER BY judgment_id",
    )?;
    let rows = stmt.query_map(params![project_id, task_id, judgment_kind], |row| {
        Ok(StoredRecordRef {
            record_kind: "user_judgment".to_owned(),
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
            interaction_role,
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
        interaction_role: row.get(4)?,
        display_name: row.get(5)?,
        capability_profile_json: row.get(6)?,
        local_access_json: row.get(7)?,
        metadata_json: row.get(8)?,
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
            surface_id,
            surface_instance_id,
            access_class,
            verification_basis,
            response_json
         FROM tool_invocations
         WHERE project_id = ?1
           AND tool_name = ?2
           AND idempotency_key = ?3",
        params![project_id, tool_name, idempotency_key],
        tool_invocation_from_row,
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
            surface_id,
            surface_instance_id,
            access_class,
            verification_basis,
            response_json
         FROM tool_invocations
         WHERE project_id = ?1
           AND tool_name = ?2
           AND idempotency_key = ?3",
        params![project_id, tool_name, idempotency_key],
        tool_invocation_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn tool_invocation_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ToolInvocationRecord> {
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
        surface_id: row.get(6)?,
        surface_instance_id: row.get(7)?,
        access_class: row.get(8)?,
        verification_basis: row.get(9)?,
        response_json: row.get(10)?,
    })
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

fn row_exists(
    conn: &Connection,
    project_id: &str,
    table: &str,
    id_column: &str,
    id: &str,
) -> StoreResult<bool> {
    let sql = format!(
        "SELECT COUNT(*)
           FROM {}
          WHERE project_id = ?1
            AND {} = ?2",
        escape_sql_identifier(table),
        escape_sql_identifier(id_column),
    );
    conn.query_row(&sql, params![project_id, id], |row| {
        Ok(row.get::<_, i64>(0)? > 0)
    })
    .map_err(StoreError::from)
}

fn row_exists_with_task(
    conn: &Connection,
    project_id: &str,
    table: &str,
    id_column: &str,
    id: &str,
    task_id: &str,
) -> StoreResult<bool> {
    let sql = format!(
        "SELECT COUNT(*)
           FROM {}
          WHERE project_id = ?1
            AND {} = ?2
            AND task_id = ?3",
        escape_sql_identifier(table),
        escape_sql_identifier(id_column),
    );
    conn.query_row(&sql, params![project_id, id, task_id], |row| {
        Ok(row.get::<_, i64>(0)? > 0)
    })
    .map_err(StoreError::from)
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

fn validate_replay_context(context: &VerifiedReplayContext) -> StoreResult<()> {
    validate_identifier("surface_id", &context.surface_id)?;
    validate_identifier("surface_instance_id", &context.surface_instance_id)?;
    validate_identifier("access_class", &context.access_class)?;
    if let Some(verification_basis) = &context.verification_basis {
        validate_identifier("verification_basis", verification_basis)?;
    }
    Ok(())
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

fn validate_actor_kind_value(field: &'static str, value: &str) -> StoreResult<()> {
    match value {
        "agent" | "user" => Ok(()),
        _ => Err(StoreError::InvalidInput {
            detail: format!("{field} must be agent or user"),
        }),
    }
}

fn validate_interaction_role_value(field: &'static str, value: &str) -> StoreResult<()> {
    match value {
        "agent" | "user_interaction" => Ok(()),
        _ => Err(StoreError::InvalidInput {
            detail: format!("{field} must be agent or user_interaction"),
        }),
    }
}

fn validate_timestamp(field: &'static str, value: &str) -> StoreResult<()> {
    UtcTimestamp::parse(value)
        .map(|_| ())
        .map_err(|_| StoreError::InvalidInput {
            detail: format!("{field} must be a valid RFC 3339 timestamp"),
        })
}

fn validate_artifact_sha256(field: &'static str, value: &str) -> StoreResult<()> {
    if is_lowercase_sha256_hex(value) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be a lowercase 64-character SHA-256 hex string"),
        })
    }
}

fn is_lowercase_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn verify_staged_artifact_body(
    project_home: &Path,
    tmp_path: Option<&str>,
    expected_sha256: &str,
    expected_size_bytes: u64,
) -> StoreResult<()> {
    let tmp_path = tmp_path.ok_or_else(|| StoreError::SchemaInvariant {
        database_kind: "project_state",
        detail: "staged artifact body path is missing before promotion".to_owned(),
    })?;
    let relative = Path::new(tmp_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(StoreError::SchemaInvariant {
            database_kind: "project_state",
            detail: "staged artifact body path is not a safe relative path".to_owned(),
        });
    }

    let bytes = fs::read(project_home.join(relative))?;
    if u64::try_from(bytes.len()).map_err(|_| StoreError::InvalidInput {
        detail: "staged artifact body size does not fit in u64".to_owned(),
    })? != expected_size_bytes
    {
        return Err(StoreError::SchemaInvariant {
            database_kind: "project_state",
            detail: "staged artifact body size changed before promotion".to_owned(),
        });
    }
    let actual_sha256 = lowercase_sha256_hex(&bytes);
    if actual_sha256 != expected_sha256 {
        return Err(StoreError::SchemaInvariant {
            database_kind: "project_state",
            detail: "staged artifact body checksum changed before promotion".to_owned(),
        });
    }
    Ok(())
}

fn lowercase_sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn validate_json_text(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<Value>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be JSON text: {error}"),
    })?;
    Ok(())
}

fn validate_current_close_basis_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<CurrentCloseBasis>(text).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} must be CurrentCloseBasis JSON: {error}"),
    })?;
    Ok(())
}

fn validate_judgment_basis_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<PersistedJudgmentBasis>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be JudgmentBasis JSON: {error}"),
        }
    })?;
    Ok(())
}

fn validate_user_judgment_request_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<PersistedUserJudgmentRequest>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be persisted user judgment request JSON: {error}"),
        }
    })?;
    Ok(())
}

fn validate_user_judgment_options_json(field: &'static str, text: &str) -> StoreResult<()> {
    let persisted =
        serde_json::from_str::<PersistedUserJudgmentOptions>(text).map_err(|error| {
            StoreError::InvalidInput {
                detail: format!("{field} must be persisted user judgment option JSON: {error}"),
            }
        })?;
    for option in &persisted.options {
        if option.resolution_outcome != option.machine_action.resolution_outcome() {
            return Err(StoreError::InvalidInput {
                detail: format!(
                    "{field} current option resolution_outcome must match machine_action"
                ),
            });
        }
    }
    Ok(())
}

fn validate_user_judgment_resolution_json(
    field: &'static str,
    text: &str,
    expected_action: UserJudgmentOptionAction,
    expected_outcome: JudgmentResolutionOutcome,
) -> StoreResult<()> {
    let resolution =
        serde_json::from_str::<PersistedUserJudgmentResolution>(text).map_err(|error| {
            StoreError::InvalidInput {
                detail: format!("{field} must be persisted user judgment resolution JSON: {error}"),
            }
        })?;
    if resolution.machine_action != expected_action {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "{field} machine_action must match user_judgments.resolution_machine_action"
            ),
        });
    }
    if resolution.resolution_outcome != expected_outcome {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "{field} resolution_outcome must match user_judgments.resolution_outcome"
            ),
        });
    }
    if resolution.machine_action.resolution_outcome() != expected_outcome {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} machine_action must match resolution_outcome"),
        });
    }
    Ok(())
}

fn validate_artifact_producer_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<PersistedArtifactProducer>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be persisted artifact producer JSON: {error}"),
        }
    })?;
    Ok(())
}

fn validate_artifact_provenance_metadata_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<PersistedArtifactProvenanceMetadata>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be persisted artifact provenance metadata JSON: {error}"),
        }
    })?;
    Ok(())
}

fn validate_evidence_coverage_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<Vec<EvidenceCoverageItem>>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be persisted evidence coverage JSON: {error}"),
        }
    })?;
    Ok(())
}

fn validate_state_refs_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<Vec<StateRecordRef>>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be persisted StateRecordRef array JSON: {error}"),
        }
    })?;
    Ok(())
}

fn validate_evidence_metadata_json(field: &'static str, text: &str) -> StoreResult<()> {
    serde_json::from_str::<PersistedEvidenceMetadata>(text).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("{field} must be persisted evidence metadata JSON: {error}"),
        }
    })?;
    Ok(())
}

fn decode_owner_json_text<T>(
    table: &'static str,
    record_ref: impl Into<String>,
    logical_column: &'static str,
    text: &str,
) -> StoreResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let record_ref = record_ref.into();
    serde_json::from_str(text)
        .map_err(|_| StoreError::corrupt_owner_state_json(table, record_ref, logical_column))
}

fn decode_current_close_basis_column(
    record_ref: &str,
    text: Option<&str>,
) -> StoreResult<Option<CurrentCloseBasis>> {
    text.map(|value| decode_current_close_basis_text(record_ref, value))
        .transpose()
}

fn decode_current_close_basis_text(record_ref: &str, text: &str) -> StoreResult<CurrentCloseBasis> {
    match serde_json::from_str::<CurrentCloseBasis>(text) {
        Ok(current) => Ok(current),
        Err(_) => serde_json::from_str::<LegacyCategoryOnlyCloseBasis>(text)
            .map(CurrentCloseBasis::from)
            .map_err(|_| {
                StoreError::corrupt_owner_state_json("tasks", record_ref, "close_basis_json")
            }),
    }
}

fn decode_judgment_basis_column(record_ref: &str, text: &str) -> StoreResult<JudgmentBasis> {
    decode_owner_json_text::<PersistedJudgmentBasis>(
        "user_judgments",
        record_ref,
        "basis_json",
        text,
    )
}

fn judgment_basis_status_as_str(status: JudgmentBasisCompatibilityStatus) -> &'static str {
    match status {
        JudgmentBasisCompatibilityStatus::Current => "current",
        JudgmentBasisCompatibilityStatus::Stale => "stale",
        JudgmentBasisCompatibilityStatus::Superseded => "superseded",
    }
}

fn judgment_resolution_outcome_as_str(outcome: JudgmentResolutionOutcome) -> &'static str {
    match outcome {
        JudgmentResolutionOutcome::Accepted => "accepted",
        JudgmentResolutionOutcome::Rejected => "rejected",
        JudgmentResolutionOutcome::Deferred => "deferred",
        JudgmentResolutionOutcome::Blocked => "blocked",
    }
}

fn judgment_machine_action_as_str(action: UserJudgmentOptionAction) -> &'static str {
    match action {
        UserJudgmentOptionAction::Accept => "accept",
        UserJudgmentOptionAction::Reject => "reject",
        UserJudgmentOptionAction::Defer => "defer",
    }
}

fn parse_judgment_basis_status(
    record_ref: &str,
    logical_column: &'static str,
    value: &str,
) -> StoreResult<JudgmentBasisCompatibilityStatus> {
    match value {
        "current" => Ok(JudgmentBasisCompatibilityStatus::Current),
        "stale" => Ok(JudgmentBasisCompatibilityStatus::Stale),
        "superseded" => Ok(JudgmentBasisCompatibilityStatus::Superseded),
        _ => Err(StoreError::corrupt_owner_state_value(
            "user_judgments",
            record_ref,
            logical_column,
        )),
    }
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

#[cfg(test)]
mod tests {
    use std::{error::Error, path::PathBuf};

    use harness_test_support::TempRuntimeHome;
    use harness_types::{
        BaselineRef, ChangeUnitId, IdempotencyKey, JudgmentBasisCompatibilityStatus, MethodName,
        ProjectId, RecordId, RequestHash, RequiredNullable, RiskId, StateRecordKind,
        StateRecordRef, SurfaceInteractionRole, TaskId,
    };
    use serde_json::{json, Value};

    use super::*;
    use crate::bootstrap::{
        initialize_runtime_home, register_project, register_surface, ProjectRegistration,
        SurfaceRegistration, ACTIVE_PROJECT_STATUS,
    };

    const PROJECT_ID: &str = "project_store";
    const SURFACE_ID: &str = "surface_store";
    const SURFACE_INSTANCE_ID: &str = "surface_instance_store";

    struct StoreHarness {
        _runtime_home: TempRuntimeHome,
        runtime_home_path: PathBuf,
    }

    impl StoreHarness {
        fn new() -> Result<Self, Box<dyn Error>> {
            let runtime_home = TempRuntimeHome::new("store-replay-context")?;
            initialize_runtime_home(runtime_home.path(), "runtime_home_store", "{}")?;
            register_project(
                runtime_home.path(),
                ProjectRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    repo_root: runtime_home.create_product_repo("repo")?,
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            register_surface(
                runtime_home.path(),
                SurfaceRegistration {
                    project_id: PROJECT_ID.to_owned(),
                    surface_id: SURFACE_ID.to_owned(),
                    surface_instance_id: SURFACE_INSTANCE_ID.to_owned(),
                    surface_kind: "local_test".to_owned(),
                    interaction_role: SurfaceInteractionRole::Agent,
                    display_name: None,
                    capability_profile_json: "{}".to_owned(),
                    local_access_json: json!({
                        "authorized_access_classes": ["core_mutation"],
                        "verification_basis": "store_test_registration"
                    })
                    .to_string(),
                    metadata_json: "{}".to_owned(),
                },
            )?;

            Ok(Self {
                runtime_home_path: runtime_home.path().to_path_buf(),
                _runtime_home: runtime_home,
            })
        }

        fn store(&self) -> StoreResult<CoreProjectStore> {
            CoreProjectStore::open(&self.runtime_home_path, &ProjectId::new(PROJECT_ID))
        }
    }

    #[test]
    fn transaction_replay_context_mismatch_precedes_request_hash_conflict(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let first_context = replay_context(SURFACE_INSTANCE_ID, "core_mutation");
        let first_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_context")),
            &RequestHash::new("sha256:first"),
            Some(first_context),
            Some(0),
            vec![pending_event("first")],
        );
        let first = store.commit_mutation(
            first_input,
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert("task_first"))
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert!(matches!(first, MutationCommitOutcome::Committed { .. }));
        let before = store.effect_counts()?;

        let mismatch_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_context")),
            &RequestHash::new("sha256:second"),
            Some(replay_context("surface_instance_other", "core_mutation")),
            Some(1),
            vec![pending_event("second")],
        );
        let mismatch = store.commit_mutation(mismatch_input, |_, _| Ok(()), response_json)?;

        assert!(matches!(
            mismatch,
            MutationCommitOutcome::ReplayContextMismatch { .. }
        ));
        assert_eq!(store.effect_counts()?, before);
        Ok(())
    }

    #[test]
    fn transaction_replay_returns_stored_response_before_stale_expected_state(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let context = replay_context(SURFACE_INSTANCE_ID, "core_mutation");
        let first_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_replay_stale")),
            &RequestHash::new("sha256:replay"),
            Some(context.clone()),
            Some(0),
            vec![pending_event("replay_stale_first")],
        );
        let first = store.commit_mutation(
            first_input,
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert("task_replay_stale_first"))
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        let MutationCommitOutcome::Committed {
            response_json: stored_response,
            ..
        } = first
        else {
            panic!("first transaction should commit");
        };
        let before_replay = store.effect_counts()?;

        let replay_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_replay_stale")),
            &RequestHash::new("sha256:replay"),
            Some(context),
            Some(0),
            vec![pending_event("replay_stale_second")],
        );
        let replay = store.commit_mutation(
            replay_input,
            |_, _| panic!("eligible replay must not apply a second mutation"),
            |_| panic!("eligible replay must not build a fresh response"),
        )?;

        assert!(matches!(
            replay,
            MutationCommitOutcome::Replayed {
                response_json,
                ..
            } if response_json == stored_response
        ));
        assert_eq!(store.effect_counts()?, before_replay);
        Ok(())
    }

    #[test]
    fn transaction_replay_hash_conflict_rejects_without_effect() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let context = replay_context(SURFACE_INSTANCE_ID, "core_mutation");
        let first_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_hash_conflict")),
            &RequestHash::new("sha256:first"),
            Some(context.clone()),
            Some(0),
            vec![pending_event("hash_conflict_first")],
        );
        let first = store.commit_mutation(
            first_input,
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert("task_hash_conflict_first"))
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert!(matches!(first, MutationCommitOutcome::Committed { .. }));
        let before_conflict = store.effect_counts()?;

        let conflict_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_hash_conflict")),
            &RequestHash::new("sha256:second"),
            Some(context),
            Some(1),
            vec![pending_event("hash_conflict_second")],
        );
        let conflict = store.commit_mutation(
            conflict_input,
            |_, _| panic!("hash conflict must not apply a second mutation"),
            |_| panic!("hash conflict must not build a fresh response"),
        )?;

        assert!(matches!(
            conflict,
            MutationCommitOutcome::IdempotencyConflict {
                stored_request_hash,
                attempted_request_hash,
                ..
            } if stored_request_hash == "sha256:first"
                && attempted_request_hash == "sha256:second"
        ));
        assert_eq!(store.effect_counts()?, before_conflict);
        Ok(())
    }

    #[test]
    fn task_and_judgment_basis_store_apis_round_trip() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_basis_round_trip";
        let close_basis = current_close_basis(task_id, 2, 3);
        let close_basis_json = serde_json::to_string(&close_basis)?;
        let judgment_basis = judgment_basis(task_id, 2, Some(3));
        let judgment_basis_json = serde_json::to_string(&judgment_basis)?;

        let first_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_basis_initial")),
            &RequestHash::new("sha256:basis-initial"),
            Some(replay_context(SURFACE_INSTANCE_ID, "core_mutation")),
            Some(0),
            vec![pending_event_for_task("basis_initial", task_id)],
        );
        let first = store.commit_mutation(
            first_input,
            |mutation, facts| {
                for storage_mutation in [
                    CoreStorageMutation::InsertTask(task_insert(task_id)),
                    CoreStorageMutation::UpdateTaskScopeRevision(TaskScopeRevisionUpdate {
                        task_id: task_id.to_owned(),
                        scope_revision: 2,
                    }),
                    CoreStorageMutation::UpdateTaskCloseBasis(TaskCloseBasisUpdate {
                        task_id: task_id.to_owned(),
                        close_basis_revision: 3,
                        close_basis_json: Some(close_basis_json.clone()),
                    }),
                    CoreStorageMutation::InsertUserJudgment(user_judgment_insert(
                        "judgment_basis_round_trip",
                        task_id,
                        Some(judgment_basis_json.clone()),
                        JudgmentBasisCompatibilityStatus::Current,
                    )),
                ] {
                    storage_mutation.apply(mutation, facts.committed_state_version)?;
                }
                Ok(())
            },
            response_json,
        )?;
        assert!(matches!(first, MutationCommitOutcome::Committed { .. }));

        let task_revisions = store
            .task_revision_record(&TaskId::new(task_id))?
            .expect("task revisions should be readable");
        assert_eq!(task_revisions.scope_revision, 2);
        assert_eq!(task_revisions.close_basis_revision, 3);
        assert_eq!(task_revisions.current_close_basis, Some(close_basis));

        let basis_record = store
            .user_judgment_basis_record("judgment_basis_round_trip")?
            .expect("judgment basis should be readable");
        assert_eq!(
            basis_record.basis_status,
            JudgmentBasisCompatibilityStatus::Current
        );
        assert_eq!(basis_record.basis, judgment_basis);

        let stale_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_basis_stale")),
            &RequestHash::new("sha256:basis-stale"),
            Some(replay_context(SURFACE_INSTANCE_ID, "core_mutation")),
            Some(1),
            vec![pending_event_for_task("basis_stale", task_id)],
        );
        let stale = store.commit_mutation(
            stale_input,
            |mutation, facts| {
                CoreStorageMutation::MarkUserJudgmentBasesStatus(UserJudgmentBasisStatusMark {
                    judgment_ids: vec!["judgment_basis_round_trip".to_owned()],
                    basis_status: JudgmentBasisCompatibilityStatus::Stale,
                })
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert!(matches!(stale, MutationCommitOutcome::Committed { .. }));
        assert_eq!(
            store
                .user_judgment_basis_record("judgment_basis_round_trip")?
                .expect("judgment basis should remain readable")
                .basis_status,
            JudgmentBasisCompatibilityStatus::Stale
        );

        let superseded_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_basis_superseded")),
            &RequestHash::new("sha256:basis-superseded"),
            Some(replay_context(SURFACE_INSTANCE_ID, "core_mutation")),
            Some(2),
            vec![pending_event_for_task("basis_superseded", task_id)],
        );
        let superseded = store.commit_mutation(
            superseded_input,
            |mutation, facts| {
                CoreStorageMutation::MarkUserJudgmentBasesStatus(UserJudgmentBasisStatusMark {
                    judgment_ids: vec!["judgment_basis_round_trip".to_owned()],
                    basis_status: JudgmentBasisCompatibilityStatus::Superseded,
                })
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert!(matches!(
            superseded,
            MutationCommitOutcome::Committed { .. }
        ));
        assert_eq!(
            store
                .user_judgment_basis_record("judgment_basis_round_trip")?
                .expect("judgment basis should remain readable")
                .basis_status,
            JudgmentBasisCompatibilityStatus::Superseded
        );
        Ok(())
    }

    #[test]
    fn resolve_user_judgment_writes_deferred_action_outcome_pair() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_deferred_judgment";
        let judgment_id = "judgment_deferred_pair";

        let insert_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserJudgment,
            Some(&IdempotencyKey::new("idem_store_defer_insert")),
            &RequestHash::new("sha256:defer-insert"),
            Some(replay_context(SURFACE_INSTANCE_ID, "core_mutation")),
            Some(0),
            vec![pending_event_for_task("defer_insert", task_id)],
        );
        let inserted = store.commit_mutation(
            insert_input,
            |mutation, facts| {
                for storage_mutation in [
                    CoreStorageMutation::InsertTask(task_insert(task_id)),
                    CoreStorageMutation::InsertUserJudgment(user_judgment_insert(
                        judgment_id,
                        task_id,
                        None,
                        JudgmentBasisCompatibilityStatus::Current,
                    )),
                ] {
                    storage_mutation.apply(mutation, facts.committed_state_version)?;
                }
                Ok(())
            },
            response_json,
        )?;
        assert!(matches!(inserted, MutationCommitOutcome::Committed { .. }));

        let resolve_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RecordUserJudgment,
            Some(&IdempotencyKey::new("idem_store_defer_resolve")),
            &RequestHash::new("sha256:defer-resolve"),
            Some(replay_context(SURFACE_INSTANCE_ID, "core_mutation")),
            Some(1),
            vec![pending_event_for_task("defer_resolve", task_id)],
        );
        let resolved = store.commit_mutation(
            resolve_input,
            |mutation, facts| {
                CoreStorageMutation::ResolveUserJudgment(user_judgment_resolution_update(
                    judgment_id,
                    UserJudgmentOptionAction::Defer,
                    JudgmentResolutionOutcome::Deferred,
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert!(matches!(resolved, MutationCommitOutcome::Committed { .. }));

        let record = store
            .user_judgment_record(judgment_id)?
            .expect("resolved judgment should be readable");
        assert_eq!(record.resolution_machine_action, Some("defer".to_owned()));
        assert_eq!(record.resolution_outcome, Some("deferred".to_owned()));
        assert_eq!(
            serde_json::from_str::<Value>(
                record
                    .resolution_json
                    .as_deref()
                    .expect("resolution JSON should be stored"),
            )?["machine_action"],
            "defer"
        );
        Ok(())
    }

    #[test]
    fn resolve_user_judgment_requires_resolution_json_action() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_missing_json_action";
        let judgment_id = "judgment_missing_json_action";

        let insert_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserJudgment,
            Some(&IdempotencyKey::new("idem_store_missing_action_insert")),
            &RequestHash::new("sha256:missing-action-insert"),
            Some(replay_context(SURFACE_INSTANCE_ID, "core_mutation")),
            Some(0),
            vec![pending_event_for_task("missing_action_insert", task_id)],
        );
        let inserted = store.commit_mutation(
            insert_input,
            |mutation, facts| {
                for storage_mutation in [
                    CoreStorageMutation::InsertTask(task_insert(task_id)),
                    CoreStorageMutation::InsertUserJudgment(user_judgment_insert(
                        judgment_id,
                        task_id,
                        None,
                        JudgmentBasisCompatibilityStatus::Current,
                    )),
                ] {
                    storage_mutation.apply(mutation, facts.committed_state_version)?;
                }
                Ok(())
            },
            response_json,
        )?;
        assert!(matches!(inserted, MutationCommitOutcome::Committed { .. }));
        let before = store.effect_counts()?;

        let resolve_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RecordUserJudgment,
            Some(&IdempotencyKey::new("idem_store_missing_action_resolve")),
            &RequestHash::new("sha256:missing-action-resolve"),
            Some(replay_context(SURFACE_INSTANCE_ID, "core_mutation")),
            Some(1),
            vec![pending_event_for_task("missing_action_resolve", task_id)],
        );
        let mut update = user_judgment_resolution_update(
            judgment_id,
            UserJudgmentOptionAction::Accept,
            JudgmentResolutionOutcome::Accepted,
        );
        update.resolution_json = json!({
            "selected_option_id": "accept",
            "resolution_outcome": "accepted",
            "answer": {
                "product_decision": null,
                "technical_decision": null,
                "scope_decision": null,
                "sensitive_action_scope": null,
                "final_acceptance": { "judgment": { "decision": "accepted" } },
                "residual_risk_acceptance": null,
                "cancellation": null
            },
            "note": null,
            "accepted_risks": [],
            "resolved_by_actor_kind": "user"
        })
        .to_string();

        let error = store
            .commit_mutation(
                resolve_input,
                |mutation, facts| {
                    CoreStorageMutation::ResolveUserJudgment(update)
                        .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )
            .expect_err("resolution JSON without machine_action should reject");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert_eq!(store.effect_counts()?, before);
        let record = store
            .user_judgment_record(judgment_id)?
            .expect("pending judgment should remain readable");
        assert_eq!(record.status, "pending");
        assert_eq!(record.resolution_machine_action, None);
        Ok(())
    }

    #[test]
    fn malformed_stored_judgment_basis_json_is_store_data_error() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_malformed_basis";
        let basis_json = serde_json::to_string(&judgment_basis(task_id, 0, None))?;

        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_basis_malformed")),
            &RequestHash::new("sha256:basis-malformed"),
            Some(replay_context(SURFACE_INSTANCE_ID, "core_mutation")),
            Some(0),
            vec![pending_event_for_task("basis_malformed", task_id)],
        );
        store.commit_mutation(
            input,
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::InsertUserJudgment(user_judgment_insert(
                    "judgment_malformed_basis",
                    task_id,
                    Some(basis_json),
                    JudgmentBasisCompatibilityStatus::Current,
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        let conn = open_project_state_database(
            harness
                .runtime_home_path
                .join("projects")
                .join(PROJECT_ID)
                .join("state.sqlite"),
        )?;
        conn.execute(
            "UPDATE user_judgments
                SET basis_json = 'not-json'
              WHERE project_id = ?1
                AND judgment_id = 'judgment_malformed_basis'",
            [PROJECT_ID],
        )?;
        drop(conn);

        let store = harness.store()?;
        let error = store
            .user_judgment_basis_record("judgment_malformed_basis")
            .expect_err("malformed persisted basis JSON should be corruption");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateJson {
                table: "user_judgments",
                logical_column: "basis_json",
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn foreign_key_constraint_failure_is_classified() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RecordRun,
            Some(&IdempotencyKey::new("idem_store_foreign_key")),
            &RequestHash::new("sha256:foreign-key"),
            Some(replay_context(SURFACE_INSTANCE_ID, "core_mutation")),
            Some(0),
            vec![pending_event("foreign_key")],
        );

        let error = store
            .commit_mutation(
                input,
                |mutation, facts| {
                    CoreStorageMutation::InsertRun(run_insert_with_missing_task())
                        .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )
            .expect_err("missing run task should fail a foreign-key constraint");
        let classification = error.classification();

        assert_eq!(classification.category, "constraint_foreign_key");
        assert!(matches!(
            classification.route,
            crate::StoreFailureRoute::OperationalUnavailable
        ));
        Ok(())
    }

    fn replay_context(surface_instance_id: &str, access_class: &str) -> VerifiedReplayContext {
        VerifiedReplayContext {
            surface_id: SURFACE_ID.to_owned(),
            surface_instance_id: surface_instance_id.to_owned(),
            access_class: access_class.to_owned(),
            verification_basis: Some("store_test_registration".to_owned()),
        }
    }

    fn pending_event(marker: &str) -> PendingTaskEvent {
        pending_event_for_task(marker, &format!("task_{marker}"))
    }

    fn pending_event_for_task(marker: &str, task_id: &str) -> PendingTaskEvent {
        PendingTaskEvent {
            event_id: format!("evt_{marker}"),
            task_id: task_id.to_owned(),
            change_unit_id: None,
            event_kind: "store_test_event".to_owned(),
            event_payload_json: "{}".to_owned(),
        }
    }

    fn task_insert(task_id: &str) -> TaskInsert {
        TaskInsert {
            task_id: task_id.to_owned(),
            created_by_surface_id: SURFACE_ID.to_owned(),
            created_by_surface_instance_id: SURFACE_INSTANCE_ID.to_owned(),
            mode: "work".to_owned(),
            lifecycle_phase: "shaping".to_owned(),
            result: None,
            title: None,
            summary: None,
            shaping_summary_json: "{}".to_owned(),
            bounded_context_json: "[]".to_owned(),
            autonomy_boundary_json: "{}".to_owned(),
            close_summary_json: "{}".to_owned(),
            completion_policy_json: "{}".to_owned(),
            current_change_unit_id: None,
        }
    }

    fn user_judgment_insert(
        judgment_id: &str,
        task_id: &str,
        basis_json: Option<String>,
        basis_status: JudgmentBasisCompatibilityStatus,
    ) -> UserJudgmentInsert {
        let basis_json = basis_json.unwrap_or_else(|| {
            serde_json::to_string(&judgment_basis(task_id, 0, None))
                .expect("test judgment basis should serialize")
        });
        UserJudgmentInsert {
            judgment_id: judgment_id.to_owned(),
            task_id: task_id.to_owned(),
            change_unit_id: None,
            judgment_kind: "final_acceptance".to_owned(),
            request_json: json!({
                "presentation": "short",
                "question": "Accept the current close basis?",
                "required_for": ["close_complete"],
                "expires_at": Value::Null
            })
            .to_string(),
            context_json: "{}".to_owned(),
            options_json: json!({
                "schema_version": 1,
                "options": [{
                "option_id": "accept",
                "label": "Accept",
                "description": "Accept the current close basis.",
                "consequence": "The judgment can be resolved.",
                "machine_action": "accept",
                "resolution_outcome": "accepted",
                "is_default": true
                }]
            })
            .to_string(),
            affected_refs_json: "[]".to_owned(),
            artifact_refs_json: "[]".to_owned(),
            sensitive_action_scope_json: "{}".to_owned(),
            basis_json,
            basis_status,
            requested_by_surface_id: SURFACE_ID.to_owned(),
            requested_by_surface_instance_id: SURFACE_INSTANCE_ID.to_owned(),
            requested_at: "t0".to_owned(),
            metadata_json: "{}".to_owned(),
        }
    }

    fn user_judgment_resolution_update(
        judgment_id: &str,
        action: UserJudgmentOptionAction,
        outcome: JudgmentResolutionOutcome,
    ) -> UserJudgmentResolutionUpdate {
        UserJudgmentResolutionUpdate {
            judgment_id: judgment_id.to_owned(),
            status: "resolved".to_owned(),
            resolution_outcome: outcome,
            resolution_machine_action: action,
            resolution_json: json!({
                "selected_option_id": match action {
                    UserJudgmentOptionAction::Accept => "accept",
                    UserJudgmentOptionAction::Reject => "reject",
                    UserJudgmentOptionAction::Defer => "defer",
                },
                "machine_action": action,
                "resolution_outcome": outcome,
                "answer": {
                    "product_decision": null,
                    "technical_decision": null,
                    "scope_decision": null,
                    "sensitive_action_scope": null,
                    "final_acceptance": { "judgment": { "decision": outcome } },
                    "residual_risk_acceptance": null,
                    "cancellation": null
                },
                "note": null,
                "accepted_risks": [],
                "resolved_by_actor_kind": "user"
            })
            .to_string(),
            sensitive_action_scope_json: None,
            resolved_by_actor_kind: "user".to_owned(),
            resolved_actor_role: "user_interaction".to_owned(),
            resolved_by_surface_id: SURFACE_ID.to_owned(),
            resolved_by_surface_instance_id: SURFACE_INSTANCE_ID.to_owned(),
            resolved_verification_basis: "store_test_registration".to_owned(),
            resolved_assurance_level: "registered_surface_cooperative".to_owned(),
            resolved_at: "t1".to_owned(),
        }
    }

    fn current_close_basis(
        task_id: &str,
        scope_revision: u64,
        close_basis_revision: u64,
    ) -> CurrentCloseBasis {
        CurrentCloseBasis {
            close_basis_revision,
            scope_revision,
            task_id: TaskId::new(task_id),
            change_unit_id: ChangeUnitId::new("cu_basis"),
            baseline_ref: RequiredNullable::some(BaselineRef::new("baseline_store")),
            result_summary: "Store basis result summary.".to_owned(),
            result_refs: vec![state_ref(StateRecordKind::Run, "run_basis", task_id, 1)],
            evidence_summary_ref: RequiredNullable::null(),
            residual_risks: vec![harness_types::ResidualRisk {
                risk_id: RiskId::new("risk_store_001"),
                summary: "Known visible risk.".to_owned(),
                consequence: "The user may accept this named risk.".to_owned(),
                acceptance_required: true,
                source_refs: vec![state_ref(StateRecordKind::Run, "run_basis", task_id, 1)],
            }],
            sensitive_categories: vec!["network".to_owned()],
            sensitive_action_requirements: vec![harness_types::SensitiveActionRequirement {
                action_kind: "local_sensitive_step".to_owned(),
                normalized_paths: vec!["src/export.rs".to_owned()],
                sensitive_categories: vec!["network".to_owned()],
                baseline_ref: RequiredNullable::some(BaselineRef::new("baseline_store")),
                change_unit_id: ChangeUnitId::new("cu_basis"),
                source_run_ref: state_ref(StateRecordKind::Run, "run_basis", task_id, 1),
                source_write_authorization_ref: state_ref(
                    StateRecordKind::WriteAuthorization,
                    "wa_basis",
                    task_id,
                    1,
                ),
            }],
            recovery_constraints: vec!["Rollback requires operator action.".to_owned()],
            source_run_ref: state_ref(StateRecordKind::Run, "run_basis", task_id, 1),
            updated_at: UtcTimestamp::parse("2026-06-18T00:00:00Z")
                .expect("test timestamp should parse"),
        }
    }

    fn judgment_basis(
        task_id: &str,
        scope_revision: u64,
        close_basis_revision: Option<u64>,
    ) -> JudgmentBasis {
        JudgmentBasis {
            task_id: TaskId::new(task_id),
            change_unit_id: RequiredNullable::some(ChangeUnitId::new("cu_basis")),
            scope_revision,
            close_basis_revision: RequiredNullable::new(close_basis_revision),
            baseline_ref: RequiredNullable::some(BaselineRef::new("baseline_store")),
            result_refs: vec![state_ref(StateRecordKind::Run, "run_basis", task_id, 1)],
            residual_risk_ids: vec![RiskId::new("risk_store_001")],
            sensitive_action_scope: RequiredNullable::null(),
            created_at_state_version: 1,
            compatibility_status: JudgmentBasisCompatibilityStatus::Current,
        }
    }

    fn state_ref(
        record_kind: StateRecordKind,
        record_id: &str,
        task_id: &str,
        state_version: u64,
    ) -> StateRecordRef {
        StateRecordRef {
            record_kind,
            record_id: RecordId::new(record_id),
            project_id: ProjectId::new(PROJECT_ID),
            task_id: RequiredNullable::some(TaskId::new(task_id)),
            state_version: RequiredNullable::some(state_version),
        }
    }

    fn run_insert_with_missing_task() -> RunInsert {
        RunInsert {
            run_id: "run_missing_task".to_owned(),
            task_id: "missing_task".to_owned(),
            change_unit_id: None,
            scope_revision: 0,
            write_authorization_id: None,
            kind: "implementation".to_owned(),
            status: "completed".to_owned(),
            summary_json: "{}".to_owned(),
            observed_changes_json: "{}".to_owned(),
            evidence_updates_json: "[]".to_owned(),
            authorization_effect_json: "{}".to_owned(),
            created_by_surface_id: SURFACE_ID.to_owned(),
            created_by_surface_instance_id: SURFACE_INSTANCE_ID.to_owned(),
            metadata_json: "{}".to_owned(),
        }
    }

    fn response_json(facts: CommittedMutationFacts) -> StoreResult<String> {
        Ok(json!({
            "base": {
                "state_version": facts.committed_state_version
            },
            "stored_response": "must_not_leak_on_mismatch"
        })
        .to_string())
    }
}
