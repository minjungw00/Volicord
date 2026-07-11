use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use volicord_types::{
    CurrentCloseBasis, JudgmentBasis, JudgmentBasisCompatibilityStatus, JudgmentResolutionOutcome,
    ObservedChanges, PersistedArtifactProducer, PersistedArtifactProvenance,
    PersistedArtifactProvenanceMetadata, ProjectEnforcementProfile, RunId, StagedArtifactHandleId,
    TaskId, UserJudgmentOptionAction, UtcTimestamp,
};

use crate::{
    artifacts::{
        persistent_body_path_from_staging_tmp_path,
        verify_persistent_artifact_body as verify_persistent_artifact_body_in_store,
        PersistentArtifactBodySpec, PersistentArtifactVerification,
    },
    bootstrap::ProjectRecord,
    sqlite::ARTIFACTS_DIR,
    StoreError, StoreResult,
};

pub use self::commit::commit_input;
use self::validation::*;

/// Project-local store handle used by the Core request pipeline.
#[derive(Debug)]
pub struct CoreProjectStore {
    pub(crate) runtime_home: PathBuf,
    pub(crate) project: ProjectRecord,
    pub(crate) conn: Connection,
    pub(crate) writable: bool,
}

/// Current project-state header values needed by request routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStateHeader {
    pub project_id: String,
    pub state_version: u64,
    pub active_task_id: Option<String>,
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
    pub actor_source: String,
    pub operation_category: String,
    pub verification_basis: Option<String>,
    pub response_json: String,
}

/// Verified replay identity derived from current actor provenance and operation category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedReplayContext {
    pub actor_source: String,
    pub operation_category: String,
    pub verification_basis: Option<String>,
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
    MarkActiveWriteTicketsStale { task_id: String },
    InsertWriteTicket(WriteTicketInsert),
    ConsumeWriteTicket(WriteTicketConsumption),
    InsertRun(RunInsert),
    PromoteStagedArtifact(ArtifactPromotion),
    LinkArtifact(ArtifactLinkInsert),
    UpsertEvidenceSummary(EvidenceSummaryUpsert),
    InsertEvidenceObservation(EvidenceObservationInsert),
    InsertUserJudgment(UserJudgmentInsert),
    ResolveUserJudgment(UserJudgmentResolutionUpdate),
    ConsumeLocalWebConsentToken(LocalWebConsentTokenConsumption),
    ResolveUnrecordedChange(UnrecordedChangeResolutionUpdate),
    InsertProjectContinuityRecord(ProjectContinuityRecordInsert),
    UpdateUserJudgmentBasis(UserJudgmentBasisUpdate),
    MarkUserJudgmentBasesStatus(UserJudgmentBasisStatusMark),
    MarkUserJudgmentsSupersededOrStale(UserJudgmentInvalidation),
}

/// Storage input for inserting a Task current row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInsert {
    pub task_id: String,
    pub created_by_actor_source: String,
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
    pub effect_contract_json: String,
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
    pub requested_by_actor_source: String,
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
    pub resolution_rationale_json: String,
    pub sensitive_action_scope_json: Option<String>,
    pub resolved_by_actor_source: String,
    pub resolved_verification_basis: String,
    pub resolved_assurance_level: String,
    pub resolved_at: String,
}

/// Storage input for consuming a local web consent token with its judgment resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalWebConsentTokenConsumption {
    pub token_hash: String,
    pub connection_internal_id: String,
    pub judgment_id: String,
    pub consumed_at: String,
    pub completion_metadata_json: String,
}

/// Storage input for resolving one unrecorded Product Repository change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecordedChangeResolutionUpdate {
    pub unrecorded_change_id: String,
    pub resolution_json: String,
    pub resolved_at: String,
    pub resolved_by_actor_source: String,
}

/// Storage input for inserting one project-level continuity record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContinuityRecordInsert {
    pub continuity_record_id: String,
    pub source_task_id: String,
    pub source_change_unit_id: Option<String>,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub rationale: Option<String>,
    pub applies_to_paths_json: String,
    pub applies_to_refs_json: String,
    pub source_refs_json: String,
    pub artifact_refs_json: String,
    pub status: String,
    pub supersedes_refs_json: String,
    pub review_triggers_json: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
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

/// Storage input for inserting one open write ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketInsert {
    pub write_ticket_id: String,
    pub task_id: String,
    pub change_unit_id: String,
    pub attempt_scope_json: String,
    pub created_by_actor_source: String,
    pub created_by_judgment_id: Option<String>,
    pub expires_at: String,
    pub created_at: String,
    pub metadata_json: String,
}

/// Storage input for closing one open write ticket through a compatible Run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketConsumption {
    pub write_ticket_id: String,
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
    pub write_ticket_id: Option<String>,
    pub kind: String,
    pub status: String,
    pub summary_json: String,
    pub observed_changes_json: String,
    pub evidence_updates_json: String,
    pub write_ticket_effect_json: String,
    pub created_by_actor_source: String,
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

/// Storage input for promoting one staged artifact to a persistent artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactPromotion {
    pub handle_id: String,
    pub artifact_id: String,
    pub task_id: String,
    pub run_id: String,
    pub expected_created_by_actor_source: String,
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

/// Storage input for inserting one durable evidence observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceObservationInsert {
    pub evidence_observation_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub run_id: Option<String>,
    pub claim: String,
    pub source_kind: String,
    pub assurance_level: String,
    pub observed_by_actor_source: Option<String>,
    pub tool_name: Option<String>,
    pub tool_invocation_id: Option<String>,
    pub tool_metadata_json: String,
    pub input_refs_json: String,
    pub output_artifact_refs_json: String,
    pub limitations_json: String,
    pub observed_at: String,
    pub recorded_at: String,
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
    pub claim: String,
    pub source_kind: String,
    pub assurance_level: String,
    pub observed_by_actor_source: Option<String>,
    pub tool_name: Option<String>,
    pub tool_invocation_id: Option<String>,
    pub tool_metadata_json: String,
    pub input_refs_json: String,
    pub output_artifact_refs_json: String,
    pub limitations_json: String,
    pub observed_at: String,
    pub recorded_at: String,
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
    pub write_tickets: u64,
    pub runs: u64,
    pub artifact_staging: u64,
    pub artifacts: u64,
    pub artifact_links: u64,
    pub evidence_summaries: u64,
    pub evidence_observations: u64,
    pub blockers: u64,
    pub project_continuity_records: u64,
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
    pub effect_contract_json: String,
    pub lifecycle_json: String,
}

/// Stored write ticket facts needed by status and stale-marking responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketRecord {
    pub project_id: String,
    pub write_ticket_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub basis_state_version: u64,
    pub status: String,
    pub attempt_scope_json: String,
    pub expires_at: String,
    pub created_at: String,
    pub consumed_by_run_id: Option<String>,
    pub consumed_at: Option<String>,
}

/// Stored staged artifact facts needed by `volicord.record_run`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredArtifactStagingRecord {
    pub project_id: String,
    pub handle_id: String,
    pub task_id: String,
    pub created_by_actor_source: String,
    pub artifact_json: String,
    pub tmp_path: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
    pub content_type: Option<String>,
    pub redaction_state: String,
    pub status: String,
    pub expires_at: String,
}

/// Stored persistent artifact facts needed by `volicord.record_run`.
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
    pub resolution_rationale_json: Option<String>,
    pub resolved_by_actor_source: Option<String>,
    pub resolved_verification_basis: Option<String>,
    pub resolved_assurance_level: Option<String>,
    pub requested_by_actor_source: String,
    pub requested_at: String,
    pub resolved_at: Option<String>,
    pub metadata_json: String,
}

/// Stored project-continuity row data needed by Core method implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContinuityRecordRecord {
    pub project_id: String,
    pub continuity_record_id: String,
    pub source_task_id: String,
    pub source_change_unit_id: Option<String>,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub rationale: Option<String>,
    pub applies_to_paths_json: String,
    pub applies_to_refs_json: String,
    pub source_refs_json: String,
    pub artifact_refs_json: String,
    pub status: String,
    pub supersedes_refs_json: String,
    pub review_triggers_json: String,
    pub created_at: String,
    pub updated_at: String,
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

mod commit;
mod mutation_apply;
mod open;
mod replay;
mod validation;

impl CoreProjectStore {
    /// Reads the current project-state header.
    pub fn project_state(&self) -> StoreResult<ProjectStateHeader> {
        read_project_state(&self.conn, &self.project.project_id)
    }

    /// Reads and strictly validates the active project enforcement profile.
    pub fn project_enforcement_profile(&self) -> StoreResult<ProjectEnforcementProfileRecord> {
        project_enforcement_profile(&self.conn, &self.project.project_id)
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

    /// Lists active Write Tickets for a Task.
    pub fn active_write_tickets(&self, task_id: &TaskId) -> StoreResult<Vec<WriteTicketRecord>> {
        active_write_tickets(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Lists Write Tickets for a Task without mutating effective status.
    pub fn write_tickets_for_task(&self, task_id: &TaskId) -> StoreResult<Vec<WriteTicketRecord>> {
        write_tickets_for_task(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Reads one Write Ticket row by exact project-local identity.
    pub fn write_ticket_record(
        &self,
        write_ticket_id: &str,
    ) -> StoreResult<Option<WriteTicketRecord>> {
        write_ticket_record(&self.conn, &self.project.project_id, write_ticket_id)
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

    /// Lists committed Run rows for one Task with their observed changes.
    pub fn run_observed_changes_for_task(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Vec<RunObservedChangesRecord>> {
        run_observed_changes_for_task(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Reads one staged artifact row by exact project-local handle identity.
    pub fn artifact_staging_record(
        &self,
        handle_id: &str,
    ) -> StoreResult<Option<StoredArtifactStagingRecord>> {
        artifact_staging_record(&self.conn, &self.project.project_id, handle_id)
    }

    /// Returns whether a Task has prepared artifact input that has not been consumed.
    pub fn has_prepared_artifact_input(
        &self,
        task_id: &TaskId,
        now: &UtcTimestamp,
    ) -> StoreResult<bool> {
        has_prepared_artifact_input(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            &now.to_string(),
        )
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

    /// Returns whether an evidence observation id already exists in this project.
    pub fn evidence_observation_exists(&self, evidence_observation_id: &str) -> StoreResult<bool> {
        row_exists(
            &self.conn,
            &self.project.project_id,
            "evidence_observations",
            "evidence_observation_id",
            evidence_observation_id,
        )
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

    /// Lists all user-owned judgment records for a Task in stable creation order.
    pub fn user_judgment_records_for_task(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Vec<UserJudgmentRecord>> {
        user_judgment_records_for_task(&self.conn, &self.project.project_id, task_id.as_str())
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

    /// Returns whether a project-continuity record id already exists in this project.
    pub fn project_continuity_record_exists(
        &self,
        continuity_record_id: &str,
    ) -> StoreResult<bool> {
        row_exists(
            &self.conn,
            &self.project.project_id,
            "project_continuity_records",
            "continuity_record_id",
            continuity_record_id,
        )
    }

    /// Lists active project-continuity rows for compact status projection.
    pub fn active_project_continuity_records(
        &self,
        limit: usize,
    ) -> StoreResult<Vec<ProjectContinuityRecordRecord>> {
        active_project_continuity_records(&self.conn, &self.project.project_id, limit)
    }

    /// Lists project-continuity rows that originated from one Task.
    pub fn project_continuity_records_for_task(
        &self,
        task_id: &str,
    ) -> StoreResult<Vec<ProjectContinuityRecordRecord>> {
        project_continuity_records_for_task(&self.conn, &self.project.project_id, task_id)
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
            write_tickets: table_count(&self.conn, "write_tickets", &self.project.project_id)?,
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
            evidence_observations: table_count(
                &self.conn,
                "evidence_observations",
                &self.project.project_id,
            )?,
            blockers: table_count(&self.conn, "blockers", &self.project.project_id)?,
            project_continuity_records: table_count(
                &self.conn,
                "project_continuity_records",
                &self.project.project_id,
            )?,
        })
    }
}

fn read_project_state(conn: &Connection, project_id: &str) -> StoreResult<ProjectStateHeader> {
    conn.query_row(
        "SELECT
            project_id,
            state_version,
            active_task_id
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
            effect_contract_json,
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
            effect_contract_json,
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
        effect_contract_json: row.get(9)?,
        lifecycle_json: row.get(10)?,
    })
}

fn active_write_tickets(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<WriteTicketRecord>> {
    let mut stmt = conn.prepare(
        "SELECT
            project_id,
            write_ticket_id,
            task_id,
            change_unit_id,
            basis_state_version,
            status,
            attempt_scope_json,
            expires_at,
            created_at,
            consumed_by_run_id,
            consumed_at
         FROM write_tickets
         WHERE project_id = ?1
           AND task_id = ?2
           AND status = 'active'
         ORDER BY write_ticket_id",
    )?;
    let rows = stmt.query_map(params![project_id, task_id], write_ticket_record_from_row)?;
    let mut records = Vec::new();
    for row in rows {
        records.push(row?);
    }
    Ok(records)
}

fn write_tickets_for_task(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<WriteTicketRecord>> {
    let mut stmt = conn.prepare(
        "SELECT
            project_id,
            write_ticket_id,
            task_id,
            change_unit_id,
            basis_state_version,
            status,
            attempt_scope_json,
            expires_at,
            created_at,
            consumed_by_run_id,
            consumed_at
         FROM write_tickets
         WHERE project_id = ?1
           AND task_id = ?2
         ORDER BY created_at DESC, write_ticket_id DESC",
    )?;
    let rows = stmt.query_map(params![project_id, task_id], write_ticket_record_from_row)?;
    let mut records = Vec::new();
    for row in rows {
        records.push(row?);
    }
    Ok(records)
}

fn write_ticket_record(
    conn: &Connection,
    project_id: &str,
    write_ticket_id: &str,
) -> StoreResult<Option<WriteTicketRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            write_ticket_id,
            task_id,
            change_unit_id,
            basis_state_version,
            status,
            attempt_scope_json,
            expires_at,
            created_at,
            consumed_by_run_id,
            consumed_at
         FROM write_tickets
         WHERE project_id = ?1
           AND write_ticket_id = ?2",
        params![project_id, write_ticket_id],
        write_ticket_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn write_ticket_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WriteTicketRecord> {
    let basis_state_version = row.get::<_, i64>(4)?;
    Ok(WriteTicketRecord {
        project_id: row.get(0)?,
        write_ticket_id: row.get(1)?,
        task_id: row.get(2)?,
        change_unit_id: row.get(3)?,
        basis_state_version: nonnegative_i64_to_u64(
            "write_tickets.basis_state_version",
            basis_state_version,
        )?,
        status: row.get(5)?,
        attempt_scope_json: row.get(6)?,
        expires_at: row.get(7)?,
        created_at: row.get(8)?,
        consumed_by_run_id: row.get(9)?,
        consumed_at: row.get(10)?,
    })
}

fn evidence_observation_refs_for_run(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    run_id: &str,
    state_version: u64,
) -> StoreResult<Vec<StoredRecordRef>> {
    let mut stmt = conn.prepare(
        "SELECT evidence_observation_id
           FROM evidence_observations
          WHERE project_id = ?1
            AND task_id = ?2
            AND run_id = ?3
          ORDER BY evidence_observation_id",
    )?;
    let rows = stmt.query_map(params![project_id, task_id, run_id], |row| {
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

fn run_observed_changes_for_task(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<RunObservedChangesRecord>> {
    validate_identifier("task_id", task_id)?;
    let mut stmt = conn.prepare(
        "SELECT
            project_id,
            run_id,
            task_id,
            change_unit_id,
            observed_changes_json,
            status
         FROM runs
         WHERE project_id = ?1
           AND task_id = ?2
         ORDER BY created_at DESC, run_id DESC",
    )?;
    let rows = stmt.query_map(params![project_id, task_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
        ))
    })?;
    let mut records = Vec::new();
    for row in rows {
        let (project_id, run_id, task_id, change_unit_id, observed_changes_json, status) = row?;
        let observed_changes = decode_owner_json_text::<ObservedChanges>(
            "runs",
            run_id.clone(),
            "observed_changes_json",
            &observed_changes_json,
        )?;
        records.push(RunObservedChangesRecord {
            project_id,
            run_id,
            task_id,
            change_unit_id,
            observed_changes,
            status,
        });
    }
    Ok(records)
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
            created_by_actor_source,
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

fn has_prepared_artifact_input(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    now: &str,
) -> StoreResult<bool> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM artifact_staging
            WHERE project_id = ?1
              AND task_id = ?2
              AND status = 'staged'
              AND expires_at > ?3
        )",
        params![project_id, task_id, now],
        |row| row.get::<_, i64>(0),
    )
    .map(|value| value != 0)
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
            created_by_actor_source,
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
        .get::<_, Option<i64>>(7)?
        .map(|value| nonnegative_i64_to_u64("artifact_staging.size_bytes", value))
        .transpose()?;
    Ok(StoredArtifactStagingRecord {
        project_id: row.get(0)?,
        handle_id: row.get(1)?,
        task_id: row.get(2)?,
        created_by_actor_source: row.get(3)?,
        artifact_json: row.get(4)?,
        tmp_path: row.get(5)?,
        sha256: row.get(6)?,
        size_bytes,
        content_type: row.get(8)?,
        redaction_state: row.get(9)?,
        status: row.get(10)?,
        expires_at: row.get(11)?,
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

fn evidence_observation_record(
    conn: &Connection,
    project_id: &str,
    evidence_observation_id: &str,
) -> StoreResult<Option<EvidenceObservationRecord>> {
    conn.query_row(
        "SELECT
            project_id,
            evidence_observation_id,
            task_id,
            change_unit_id,
            run_id,
            claim,
            source_kind,
            assurance_level,
            observed_by_actor_source,
            tool_name,
            tool_invocation_id,
            tool_metadata_json,
            input_refs_json,
            output_artifact_refs_json,
            limitations_json,
            observed_at,
            recorded_at,
            metadata_json
         FROM evidence_observations
         WHERE project_id = ?1
           AND evidence_observation_id = ?2",
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
        claim: row.get(5)?,
        source_kind: row.get(6)?,
        assurance_level: row.get(7)?,
        observed_by_actor_source: row.get(8)?,
        tool_name: row.get(9)?,
        tool_invocation_id: row.get(10)?,
        tool_metadata_json: row.get(11)?,
        input_refs_json: row.get(12)?,
        output_artifact_refs_json: row.get(13)?,
        limitations_json: row.get(14)?,
        observed_at: row.get(15)?,
        recorded_at: row.get(16)?,
        metadata_json: row.get(17)?,
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
            resolution_rationale_json,
            resolved_by_actor_source,
            resolved_verification_basis,
            resolved_assurance_level,
            requested_by_actor_source,
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

fn active_project_continuity_records(
    conn: &Connection,
    project_id: &str,
    limit: usize,
) -> StoreResult<Vec<ProjectContinuityRecordRecord>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let limit = i64::try_from(limit).map_err(|_| StoreError::InvalidInput {
        detail: "project_continuity_records limit is too large".to_owned(),
    })?;
    let mut stmt = conn.prepare(
        "SELECT
            project_id,
            continuity_record_id,
            source_task_id,
            source_change_unit_id,
            kind,
            title,
            summary,
            rationale,
            applies_to_paths_json,
            applies_to_refs_json,
            source_refs_json,
            artifact_refs_json,
            status,
            supersedes_refs_json,
            review_triggers_json,
            created_at,
            updated_at,
            metadata_json
         FROM project_continuity_records
         WHERE project_id = ?1
           AND status = 'active'
         ORDER BY updated_at DESC, continuity_record_id DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(
        params![project_id, limit],
        project_continuity_record_from_row,
    )?;
    let mut records = Vec::new();
    for row in rows {
        records.push(row?);
    }
    Ok(records)
}

fn project_continuity_records_for_task(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<ProjectContinuityRecordRecord>> {
    let mut stmt = conn.prepare(
        "SELECT
            project_id,
            continuity_record_id,
            source_task_id,
            source_change_unit_id,
            kind,
            title,
            summary,
            rationale,
            applies_to_paths_json,
            applies_to_refs_json,
            source_refs_json,
            artifact_refs_json,
            status,
            supersedes_refs_json,
            review_triggers_json,
            created_at,
            updated_at,
            metadata_json
         FROM project_continuity_records
         WHERE project_id = ?1
           AND source_task_id = ?2
         ORDER BY created_at, continuity_record_id",
    )?;
    let rows = stmt.query_map(
        params![project_id, task_id],
        project_continuity_record_from_row,
    )?;
    let mut records = Vec::new();
    for row in rows {
        records.push(row?);
    }
    Ok(records)
}

fn project_continuity_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProjectContinuityRecordRecord> {
    Ok(ProjectContinuityRecordRecord {
        project_id: row.get(0)?,
        continuity_record_id: row.get(1)?,
        source_task_id: row.get(2)?,
        source_change_unit_id: row.get(3)?,
        kind: row.get(4)?,
        title: row.get(5)?,
        summary: row.get(6)?,
        rationale: row.get(7)?,
        applies_to_paths_json: row.get(8)?,
        applies_to_refs_json: row.get(9)?,
        source_refs_json: row.get(10)?,
        artifact_refs_json: row.get(11)?,
        status: row.get(12)?,
        supersedes_refs_json: row.get(13)?,
        review_triggers_json: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
        metadata_json: row.get(17)?,
    })
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
            resolution_rationale_json,
            resolved_by_actor_source,
            resolved_verification_basis,
            resolved_assurance_level,
            requested_by_actor_source,
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
            resolution_rationale_json,
            resolved_by_actor_source,
            resolved_verification_basis,
            resolved_assurance_level,
            requested_by_actor_source,
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

fn user_judgment_records_for_task(
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
            resolution_rationale_json,
            resolved_by_actor_source,
            resolved_verification_basis,
            resolved_assurance_level,
            requested_by_actor_source,
            requested_at,
            resolved_at,
            metadata_json
         FROM user_judgments
         WHERE project_id = ?1
           AND task_id = ?2
         ORDER BY requested_at, judgment_id",
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
        resolution_rationale_json: row.get(17)?,
        resolved_by_actor_source: row.get(18)?,
        resolved_verification_basis: row.get(19)?,
        resolved_assurance_level: row.get(20)?,
        requested_by_actor_source: row.get(21)?,
        requested_at: row.get(22)?,
        resolved_at: row.get(23)?,
        metadata_json: row.get(24)?,
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
            active_task_id
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

#[cfg(test)]
mod tests {
    use std::{error::Error, path::PathBuf};

    use serde_json::{json, Value};
    use volicord_test_support::TempRuntimeHome;
    use volicord_types::{
        BaselineRef, ChangeUnitId, IdempotencyKey, JudgmentBasisCompatibilityStatus, MethodName,
        ProjectId, RecordId, RequestHash, RequiredNullable, RiskId, StateRecordKind,
        StateRecordRef, TaskId,
    };

    use super::*;
    use crate::bootstrap::{
        initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
    };
    use crate::sqlite::open_project_state_database;

    const PROJECT_ID: &str = "project_store";
    const CONNECTION_ID: &str = "conn_store";
    const ACTOR_SOURCE: &str = "agent_connection:conn_store";

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
        let first_context = replay_context(CONNECTION_ID, "agent_workflow");
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
            Some(replay_context("conn_other", "agent_workflow")),
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
        let context = replay_context(CONNECTION_ID, "agent_workflow");
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
        let context = replay_context(CONNECTION_ID, "agent_workflow");
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
    fn committed_mutations_append_authority_events_with_context_and_hash_chain(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_authority_events";

        let first = store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::Intake,
                Some(&IdempotencyKey::new("idem_authority_event_first")),
                &RequestHash::new("sha256:authority-first"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("authority_first", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert!(matches!(first, MutationCommitOutcome::Committed { .. }));

        let user_context = VerifiedReplayContext {
            actor_source: "user_channel:local_user".to_owned(),
            operation_category: "user_only".to_owned(),
            verification_basis: Some("store_test_user_channel".to_owned()),
        };
        let second = store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RecordUserJudgment,
                Some(&IdempotencyKey::new("idem_authority_event_second")),
                &RequestHash::new("sha256:authority-second"),
                Some(user_context),
                Some(1),
                vec![pending_event_for_task("authority_second", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::UpdateTaskScope(TaskScopeUpdate {
                    task_id: task_id.to_owned(),
                    lifecycle_phase: None,
                    result: None,
                    title: Some("Authority event projection".to_owned()),
                    summary: None,
                    shaping_summary_json: None,
                    bounded_context_json: None,
                    autonomy_boundary_json: None,
                    close_summary_json: None,
                    completion_policy_json: None,
                })
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert!(matches!(second, MutationCommitOutcome::Committed { .. }));

        let mut stmt = store.conn.prepare(
            "SELECT
                event_seq,
                event_id,
                state_version,
                event_type,
                actor_source,
                operation_category,
                payload_json,
                request_hash,
                previous_event_hash,
                event_hash
             FROM authority_events
             WHERE project_id = ?1
             ORDER BY event_seq",
        )?;
        let rows = stmt
            .query_map([PROJECT_ID], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, String>(9)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, 1);
        assert_eq!(rows[0].2, 1);
        assert_eq!(rows[0].3, "store_test_event");
        assert_eq!(rows[0].4, ACTOR_SOURCE);
        assert_eq!(rows[0].5, "agent_workflow");
        assert_eq!(rows[0].6, "{}");
        assert_eq!(rows[0].7, "sha256:authority-first");
        assert!(rows[0].8.is_none());
        assert!(rows[0].9.starts_with("sha256:"));
        assert_eq!(rows[0].9.len(), 71);

        assert_eq!(rows[1].0, 2);
        assert_eq!(rows[1].2, 2);
        assert_eq!(rows[1].4, "user_channel:local_user");
        assert_eq!(rows[1].5, "user_only");
        assert_eq!(rows[1].7, "sha256:authority-second");
        assert_eq!(rows[1].8.as_deref(), Some(rows[0].9.as_str()));
        assert!(rows[1].9.starts_with("sha256:"));
        assert_eq!(rows[1].9.len(), 71);
        assert_ne!(rows[0].9, rows[1].9);

        let view_count: i64 = store.conn.query_row(
            "SELECT COUNT(*)
               FROM task_events
              WHERE project_id = ?1
                AND event_kind = 'store_test_event'",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(view_count, 2);
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
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
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
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
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
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
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
    fn evidence_observation_store_api_round_trips() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_evidence_observation";
        let run_id = "run_evidence_observation";
        let observation_id = "evidence_observation_store";

        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RecordRun,
            Some(&IdempotencyKey::new("idem_store_evidence_observation")),
            &RequestHash::new("sha256:evidence-observation"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("evidence_observation", task_id)],
        );
        let committed = store.commit_mutation(
            input,
            |mutation, facts| {
                for storage_mutation in [
                    CoreStorageMutation::InsertTask(task_insert(task_id)),
                    CoreStorageMutation::InsertRun(run_insert(run_id, task_id)),
                    CoreStorageMutation::InsertEvidenceObservation(EvidenceObservationInsert {
                        evidence_observation_id: observation_id.to_owned(),
                        task_id: task_id.to_owned(),
                        change_unit_id: None,
                        run_id: Some(run_id.to_owned()),
                        claim: "Search result count was verified.".to_owned(),
                        source_kind: "external_tool".to_owned(),
                        assurance_level: "external_tool_result".to_owned(),
                        observed_by_actor_source: Some(ACTOR_SOURCE.to_owned()),
                        tool_name: Some("local-test-runner".to_owned()),
                        tool_invocation_id: Some("tool_invocation_001".to_owned()),
                        tool_metadata_json: json!({"exit_code": 0}).to_string(),
                        input_refs_json: "[]".to_owned(),
                        output_artifact_refs_json: "[]".to_owned(),
                        limitations_json: json!(["External tool result is not a proof."])
                            .to_string(),
                        observed_at: "2026-06-18T00:00:00Z".to_owned(),
                        recorded_at: "2026-06-18T00:00:01Z".to_owned(),
                        metadata_json: json!({"recorded_by_run_id": run_id}).to_string(),
                    }),
                ] {
                    storage_mutation.apply(mutation, facts.committed_state_version)?;
                }
                Ok(())
            },
            response_json,
        )?;
        assert!(matches!(committed, MutationCommitOutcome::Committed { .. }));

        let record = store
            .evidence_observation_record(observation_id)?
            .expect("evidence observation should be readable");
        assert_eq!(record.run_id.as_deref(), Some(run_id));
        assert_eq!(record.source_kind, "external_tool");
        assert_eq!(record.assurance_level, "external_tool_result");
        assert_eq!(
            serde_json::from_str::<Vec<String>>(&record.limitations_json)?,
            vec!["External tool result is not a proof."]
        );
        assert_eq!(store.effect_counts()?.evidence_observations, 1);
        Ok(())
    }

    #[test]
    fn change_unit_effect_contract_json_round_trips() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_effect_contract";
        let contract = json!({
            "allowed_effects": ["product_file_write"],
            "forbidden_effects": ["external_network"],
            "allowed_paths": ["src/export.rs"],
            "expected_outputs": ["Updated export behavior."],
            "invariants": ["Keep unrelated behavior unchanged."],
            "evidence_expectations": ["Record a focused test run."],
            "sensitive_action_expectations": ["No secret access is expected."]
        });

        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_effect_contract")),
            &RequestHash::new("sha256:effect-contract"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("effect_contract", task_id)],
        );
        store.commit_mutation(
            input,
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::InsertCurrentChangeUnit(change_unit_insert(
                    "cu_effect_contract",
                    task_id,
                    contract.to_string(),
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        let record = store
            .current_change_unit(&TaskId::new(task_id))?
            .expect("current Change Unit should be readable");
        assert_eq!(
            serde_json::from_str::<Value>(&record.effect_contract_json)?,
            contract
        );
        Ok(())
    }

    #[test]
    fn malformed_effect_contract_json_rejects_commit_without_effect() -> Result<(), Box<dyn Error>>
    {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_bad_effect_contract";
        let before = store.effect_counts()?;

        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_bad_effect_contract")),
            &RequestHash::new("sha256:bad-effect-contract"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("bad_effect_contract", task_id)],
        );
        let error = store
            .commit_mutation(
                input,
                |mutation, facts| {
                    CoreStorageMutation::InsertTask(task_insert(task_id))
                        .apply(mutation, facts.committed_state_version)?;
                    CoreStorageMutation::InsertCurrentChangeUnit(change_unit_insert(
                        "cu_bad_effect_contract",
                        task_id,
                        r#"{"allowed_effects":["not_an_effect"]}"#.to_owned(),
                    ))
                    .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )
            .expect_err("unsupported effect contract values should reject");

        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert_eq!(store.effect_counts()?, before);
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
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
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
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
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
        assert_eq!(
            serde_json::from_str::<Value>(
                record
                    .resolution_rationale_json
                    .as_deref()
                    .expect("resolution rationale JSON should be stored"),
            )?["summary"],
            "The user selected the focused judgment option."
        );
        Ok(())
    }

    #[test]
    fn local_web_consent_token_consumption_rolls_back_with_judgment_record_failure(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_local_web_atomic_rollback";
        let judgment_id = "judgment_local_web_atomic_rollback";
        let token_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

        let insert_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserJudgment,
            Some(&IdempotencyKey::new("idem_store_local_web_insert")),
            &RequestHash::new("sha256:local-web-insert"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("local_web_insert", task_id)],
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

        store.conn.execute(
            "INSERT INTO local_web_consent_tokens (
                project_id, token_hash, connection_internal_id, judgment_id, capture_basis,
                status, created_at, expires_at, created_metadata_json, completion_metadata_json
             ) VALUES (?1, ?2, ?3, ?4, 'local_user_local_web', 'pending', 't0', 't9', '{}', '{}')",
            params![PROJECT_ID, token_hash, CONNECTION_ID, judgment_id],
        )?;
        let before = store.effect_counts()?;

        let resolve_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RecordUserJudgment,
            Some(&IdempotencyKey::new("idem_store_local_web_rollback")),
            &RequestHash::new("sha256:local-web-rollback"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(1),
            vec![pending_event_for_task("local_web_rollback", task_id)],
        );
        let error = store
            .commit_mutation(
                resolve_input,
                |mutation, facts| {
                    CoreStorageMutation::ConsumeLocalWebConsentToken(
                        LocalWebConsentTokenConsumption {
                            token_hash: token_hash.to_owned(),
                            connection_internal_id: CONNECTION_ID.to_owned(),
                            judgment_id: judgment_id.to_owned(),
                            consumed_at: "t1".to_owned(),
                            completion_metadata_json: "{}".to_owned(),
                        },
                    )
                    .apply(mutation, facts.committed_state_version)?;
                    CoreStorageMutation::ResolveUserJudgment(user_judgment_resolution_update(
                        judgment_id,
                        UserJudgmentOptionAction::Accept,
                        JudgmentResolutionOutcome::Accepted,
                    ))
                    .apply(mutation, facts.committed_state_version)?;
                    CoreStorageMutation::InsertRun(run_insert_with_missing_task())
                        .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )
            .expect_err("later write failure should roll back the whole commit");
        assert!(matches!(error, StoreError::Sqlite(_)));

        assert_eq!(store.effect_counts()?, before);
        let record = store
            .user_judgment_record(judgment_id)?
            .expect("pending judgment should remain readable");
        assert_eq!(record.status, "pending");
        assert_eq!(record.resolution_outcome, None);
        let (status, consumed_at, completed_at) = local_web_token_state(&store, token_hash)?;
        assert_eq!(status, "pending");
        assert_eq!(consumed_at, None);
        assert_eq!(completed_at, None);
        Ok(())
    }

    #[test]
    fn insert_user_judgment_rejects_blocked_option_outcome() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_blocked_option_outcome";
        let judgment_id = "judgment_blocked_option_outcome";
        let before = store.effect_counts()?;

        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserJudgment,
            Some(&IdempotencyKey::new("idem_store_blocked_option")),
            &RequestHash::new("sha256:blocked-option"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("blocked_option", task_id)],
        );
        let error = store
            .commit_mutation(
                input,
                |mutation, facts| {
                    CoreStorageMutation::InsertTask(task_insert(task_id))
                        .apply(mutation, facts.committed_state_version)?;
                    let mut insert = user_judgment_insert(
                        judgment_id,
                        task_id,
                        None,
                        JudgmentBasisCompatibilityStatus::Current,
                    );
                    insert.options_json = json!({
                        "options": [{
                            "option_id": "accept",
                            "label": "Accept",
                            "description": "Accept the current close basis.",
                            "consequence": "The judgment can be resolved.",
                            "machine_action": "accept",
                            "resolution_outcome": "blocked",
                            "is_default": true
                        }]
                    })
                    .to_string();
                    CoreStorageMutation::InsertUserJudgment(insert)
                        .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )
            .expect_err("blocked persisted option outcome should reject");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert_eq!(store.effect_counts()?, before);
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
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
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
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
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
            "resolved_by_actor_source": "local_user"
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
    fn resolve_user_judgment_rejects_blocked_resolution_json() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_blocked_resolution_json";
        let judgment_id = "judgment_blocked_resolution_json";

        let insert_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserJudgment,
            Some(&IdempotencyKey::new("idem_store_blocked_resolution_insert")),
            &RequestHash::new("sha256:blocked-resolution-insert"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("blocked_resolution_insert", task_id)],
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
            Some(&IdempotencyKey::new("idem_store_blocked_resolution")),
            &RequestHash::new("sha256:blocked-resolution"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(1),
            vec![pending_event_for_task("blocked_resolution", task_id)],
        );
        let mut update = user_judgment_resolution_update(
            judgment_id,
            UserJudgmentOptionAction::Accept,
            JudgmentResolutionOutcome::Accepted,
        );
        update.resolution_json = json!({
            "selected_option_id": "accept",
            "machine_action": "accept",
            "resolution_outcome": "blocked",
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
            "resolved_by_actor_source": "local_user"
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
            .expect_err("resolution JSON with blocked outcome should reject");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert_eq!(store.effect_counts()?, before);
        let record = store
            .user_judgment_record(judgment_id)?
            .expect("pending judgment should remain readable");
        assert_eq!(record.status, "pending");
        assert_eq!(record.resolution_outcome, None);
        Ok(())
    }

    #[test]
    fn resolve_user_judgment_rejects_unknown_rationale_field() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_unknown_rationale_json";
        let judgment_id = "judgment_unknown_rationale_json";

        let insert_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserJudgment,
            Some(&IdempotencyKey::new("idem_store_unknown_rationale_insert")),
            &RequestHash::new("sha256:unknown-rationale-insert"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("unknown_rationale_insert", task_id)],
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
            Some(&IdempotencyKey::new("idem_store_unknown_rationale")),
            &RequestHash::new("sha256:unknown-rationale"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(1),
            vec![pending_event_for_task("unknown_rationale", task_id)],
        );
        let mut update = user_judgment_resolution_update(
            judgment_id,
            UserJudgmentOptionAction::Accept,
            JudgmentResolutionOutcome::Accepted,
        );
        let mut rationale: Value = serde_json::from_str(&update.resolution_rationale_json)?;
        rationale["unknown_rationale_field"] = json!(true);
        update.resolution_rationale_json = rationale.to_string();

        let error = store
            .commit_mutation(
                resolve_input,
                |mutation, facts| {
                    CoreStorageMutation::ResolveUserJudgment(update)
                        .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )
            .expect_err("rationale JSON with unknown field should reject");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert_eq!(store.effect_counts()?, before);
        let record = store
            .user_judgment_record(judgment_id)?
            .expect("pending judgment should remain readable");
        assert_eq!(record.status, "pending");
        assert_eq!(record.resolution_rationale_json, None);
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
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
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
    fn project_continuity_record_mutation_persists_and_reads_active_rows(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_continuity_store";
        let change_unit_id = "cu_continuity_store";
        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RecordUserJudgment,
            Some(&IdempotencyKey::new("idem_store_continuity")),
            &RequestHash::new("sha256:store-continuity"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("continuity", task_id)],
        );

        store.commit_mutation(
            input,
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::InsertCurrentChangeUnit(change_unit_insert(
                    change_unit_id,
                    task_id,
                    "null".to_owned(),
                ))
                .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::InsertProjectContinuityRecord(
                    project_continuity_record_insert(task_id, change_unit_id),
                )
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        let active = store.active_project_continuity_records(10)?;
        assert_eq!(store.effect_counts()?.project_continuity_records, 1);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].continuity_record_id, "continuity_store_001");
        assert_eq!(active[0].kind, "decision");
        assert_eq!(active[0].status, "active");
        assert_eq!(active[0].source_task_id, task_id);
        assert_eq!(
            active[0].source_change_unit_id.as_deref(),
            Some(change_unit_id)
        );

        let task_records = store.project_continuity_records_for_task(task_id)?;
        assert_eq!(task_records.len(), 1);
        assert!(store.project_continuity_record_exists("continuity_store_001")?);
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
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
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

    fn replay_context(connection_id: &str, operation_category: &str) -> VerifiedReplayContext {
        VerifiedReplayContext {
            actor_source: format!("agent_connection:{connection_id}"),
            operation_category: operation_category.to_owned(),
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
            created_by_actor_source: ACTOR_SOURCE.to_owned(),
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

    fn change_unit_insert(
        change_unit_id: &str,
        task_id: &str,
        effect_contract_json: String,
    ) -> ChangeUnitInsert {
        ChangeUnitInsert {
            change_unit_id: change_unit_id.to_owned(),
            task_id: task_id.to_owned(),
            scope_summary_json: json!({
                "scope_summary": "Store effect contract scope."
            })
            .to_string(),
            bounded_paths_json: json!(["src/export.rs"]).to_string(),
            write_basis_json: json!({
                "baseline_ref": "baseline_store"
            })
            .to_string(),
            effect_contract_json,
            lifecycle_json: "{}".to_owned(),
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
            requested_by_actor_source: ACTOR_SOURCE.to_owned(),
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
                "resolved_by_actor_source": "local_user"
            })
            .to_string(),
            resolution_rationale_json: json!({
                "summary": "The user selected the focused judgment option.",
                "selected_reason": "The selected option matches the visible judgment prompt.",
                "considered_alternatives": ["Use a different judgment option."],
                "rejected_alternatives": [],
                "assumptions": [],
                "tradeoffs": ["The recorded judgment remains limited to its stored option and basis."],
                "uncertainties": [],
                "review_triggers": ["Review if the judgment basis becomes stale."],
                "related_refs": [],
                "artifact_refs": []
            })
            .to_string(),
            sensitive_action_scope_json: None,
            resolved_by_actor_source: "local_user".to_owned(),
            resolved_verification_basis: "store_test_registration".to_owned(),
            resolved_assurance_level: "local_user_channel".to_owned(),
            resolved_at: "t1".to_owned(),
        }
    }

    fn project_continuity_record_insert(
        task_id: &str,
        change_unit_id: &str,
    ) -> ProjectContinuityRecordInsert {
        ProjectContinuityRecordInsert {
            continuity_record_id: "continuity_store_001".to_owned(),
            source_task_id: task_id.to_owned(),
            source_change_unit_id: Some(change_unit_id.to_owned()),
            kind: "decision".to_owned(),
            title: "Store continuity decision".to_owned(),
            summary: "A durable store-level continuity decision.".to_owned(),
            rationale: Some("The test records a traceable decision.".to_owned()),
            applies_to_paths_json: json!(["src/export.rs"]).to_string(),
            applies_to_refs_json: serde_json::to_string(&vec![state_ref(
                StateRecordKind::ChangeUnit,
                change_unit_id,
                task_id,
                1,
            )])
            .expect("state ref JSON should serialize"),
            source_refs_json: serde_json::to_string(&vec![state_ref(
                StateRecordKind::Task,
                task_id,
                task_id,
                1,
            )])
            .expect("state ref JSON should serialize"),
            artifact_refs_json: "[]".to_owned(),
            status: "active".to_owned(),
            supersedes_refs_json: "[]".to_owned(),
            review_triggers_json: json!(["Review if the source Task changes."]).to_string(),
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            updated_at: "2026-01-01T00:00:00Z".to_owned(),
            metadata_json: json!({"source": "store_test"}).to_string(),
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
            residual_risks: vec![volicord_types::ResidualRisk {
                risk_id: RiskId::new("risk_store_001"),
                summary: "Known visible risk.".to_owned(),
                consequence: "The user may accept this named risk.".to_owned(),
                acceptance_required: true,
                source_refs: vec![state_ref(StateRecordKind::Run, "run_basis", task_id, 1)],
            }],
            sensitive_categories: vec!["network".to_owned()],
            sensitive_action_requirements: vec![volicord_types::SensitiveActionRequirement {
                action_kind: "local_sensitive_step".to_owned(),
                normalized_paths: vec!["src/export.rs".to_owned()],
                sensitive_categories: vec!["network".to_owned()],
                baseline_ref: RequiredNullable::some(BaselineRef::new("baseline_store")),
                change_unit_id: ChangeUnitId::new("cu_basis"),
                source_run_ref: state_ref(StateRecordKind::Run, "run_basis", task_id, 1),
                source_write_ticket_ref: state_ref(
                    StateRecordKind::WriteTicket,
                    "wt_basis",
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
            produced_at_state_version: RequiredNullable::some(state_version),
        }
    }

    fn run_insert_with_missing_task() -> RunInsert {
        RunInsert {
            run_id: "run_missing_task".to_owned(),
            task_id: "missing_task".to_owned(),
            change_unit_id: None,
            scope_revision: 0,
            write_ticket_id: None,
            kind: "implementation".to_owned(),
            status: "completed".to_owned(),
            summary_json: "{}".to_owned(),
            observed_changes_json: "{}".to_owned(),
            evidence_updates_json: "[]".to_owned(),
            write_ticket_effect_json: "{}".to_owned(),
            created_by_actor_source: ACTOR_SOURCE.to_owned(),
            metadata_json: "{}".to_owned(),
        }
    }

    fn run_insert(run_id: &str, task_id: &str) -> RunInsert {
        RunInsert {
            run_id: run_id.to_owned(),
            task_id: task_id.to_owned(),
            change_unit_id: None,
            scope_revision: 0,
            write_ticket_id: None,
            kind: "implementation".to_owned(),
            status: "recorded".to_owned(),
            summary_json: "{}".to_owned(),
            observed_changes_json: "{}".to_owned(),
            evidence_updates_json: "[]".to_owned(),
            write_ticket_effect_json: "{}".to_owned(),
            created_by_actor_source: ACTOR_SOURCE.to_owned(),
            metadata_json: "{}".to_owned(),
        }
    }

    fn local_web_token_state(
        store: &CoreProjectStore,
        token_hash: &str,
    ) -> StoreResult<(String, Option<String>, Option<String>)> {
        store
            .conn
            .query_row(
                "SELECT status, consumed_at, completed_at
               FROM local_web_consent_tokens
              WHERE project_id = ?1
                AND token_hash = ?2",
                params![PROJECT_ID, token_hash],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(StoreError::Sqlite)
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
