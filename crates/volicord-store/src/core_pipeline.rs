use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use volicord_types::{
    effective_user_action_status as derive_user_action_status, validate_channel_submission_id,
    ArtifactRef, ContinuityCursor, CurrentCloseBasis, MethodName, ObservedChanges,
    PersistedArtifactProducer, PersistedArtifactProvenance, PersistedArtifactProvenanceMetadata,
    PersistedCloseSummary, PersistedUserActionRequest, ProjectEnforcementProfile, RunId,
    StagedArtifactHandleId, TaskId, UserActionBasis, UserActionBasisStatus, UserActionChannelKind,
    UserActionKind, UserActionOptionAction, UserActionRequestBody, UserActionResolutionBody,
    UserActionStatus, UtcTimestamp, MAX_CONTINUITY_PAGE_SIZE,
};

use crate::{
    artifacts::{
        persistent_body_path_from_staging_tmp_path,
        verify_persistent_artifact_body as verify_persistent_artifact_body_in_store,
        PersistentArtifactBodySpec, PersistentArtifactVerification,
    },
    bootstrap::ProjectRecord,
    guards::{agent_session_from_conn, AgentSessionRecord},
    sqlite::ARTIFACTS_DIR,
    StoreError, StoreResult,
};

pub use crate::evidence_capture::{
    derive_evidence_capture_source_claims, EvidenceCaptureIntentInsert,
    EvidenceCaptureIntentRecord, EvidenceCaptureReceiptInsert, EvidenceCaptureReceiptRecord,
    EvidenceCaptureSourceClaimIdentity, EvidenceCaptureSourceClaimKind,
    EvidenceCaptureSourceClaimRecord, EvidenceProducerInsert, EvidenceProducerRecord,
};

pub use self::commit::commit_input;
use self::validation::*;

// These exact projection lists are deliberately shared by every query that
// uses the corresponding typed row decoder. SQL predicates and transaction
// boundaries remain visible at each call site.
const TASK_RECORD_COLUMNS: &str = "
    project_id, task_id, mode, requested_control_level,
    effective_control_level, control_level_reason, work_phase, acceptance_policy,
    acceptance_policy_reason, predecessor_task_id, lineage_relation,
    lineage_reason, carry_forward_json, lifecycle_phase, result, title,
    summary, shaping_summary_json, bounded_context_json,
    autonomy_boundary_json, scope_revision, close_basis_revision,
    close_basis_json, close_summary_json, current_change_unit_id, closed_at,
    metadata_json";

const CHANGE_UNIT_RECORD_COLUMNS: &str = "
    project_id, change_unit_id, task_id, status, is_current,
    basis_state_version, scope_summary_json, bounded_paths_json,
    write_basis_json, effect_contract_json, lifecycle_json";

const WRITE_TICKET_RECORD_COLUMNS: &str = "
    project_id, write_ticket_id, task_id, change_unit_id,
    basis_state_version, status, validity_basis_json,
    allowed_path_prefixes_json, denied_path_prefixes_json,
    attempt_scope_json, idle_expires_at, invalidation_reason, created_at,
    consumed_by_run_id, consumed_at";

/// Project-local store handle used by the Core request pipeline.
#[derive(Debug)]
pub struct CoreProjectStore {
    pub(crate) runtime_home: PathBuf,
    pub(crate) project: ProjectRecord,
    pub(crate) conn: Connection,
    pub(crate) writable: bool,
    pub(crate) last_clock_sample: RefCell<Option<UtcTimestamp>>,
}

/// Current project-state header values needed by request routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStateHeader {
    pub project_id: String,
    pub state_version: u64,
    pub active_task_id: Option<String>,
    pub updated_at: String,
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
    pub git_workspace_context_json: Option<String>,
    pub response_json: String,
}

/// Immutable replay response facts used by exact historical result retrieval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredOperationResult {
    pub project_id: String,
    pub source_method: String,
    pub source_idempotency_key: String,
    pub committed_state_version: u64,
    pub actor_source: String,
    pub operation_category: String,
    pub response_sha256: String,
    pub response_size_bytes: u64,
    pub response_json: String,
}

/// Verified replay identity derived from current invocation context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedReplayContext {
    pub actor_source: String,
    pub operation_category: String,
    pub verification_basis: Option<String>,
    pub git_workspace_context_json: Option<String>,
}

/// Pending event supplied by a method-specific commit branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTaskEvent {
    pub event_id: String,
    pub task_id: Option<String>,
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
    UpdateTaskControlLevel(TaskControlLevelUpdate),
    UpdateTaskScope(TaskScopeUpdate),
    UpdateTaskScopeRevision(TaskScopeRevisionUpdate),
    UpdateTaskCloseBasis(TaskCloseBasisUpdate),
    ReplaceAcceptanceCriteria(AcceptanceCriteriaReplace),
    EnsureEvidenceClaim(EvidenceClaimInsert),
    InsertCurrentChangeUnit(ChangeUnitInsert),
    ReplaceCurrentChangeUnit(ChangeUnitInsert),
    MarkActiveWriteTicketsStale { task_id: String },
    InvalidateActiveWriteTickets(WriteTicketInvalidation),
    InvalidateWriteTicket(WriteTicketByIdInvalidation),
    InsertWriteTicket(WriteTicketInsert),
    ConsumeWriteTicket(WriteTicketConsumption),
    InsertRun(RunInsert),
    InsertEvidenceCaptureIntent(EvidenceCaptureIntentInsert),
    PromoteStagedArtifact(ArtifactPromotion),
    LinkArtifact(ArtifactLinkInsert),
    UpsertEvidenceSummary(EvidenceSummaryUpsert),
    InsertEvidenceObservation(EvidenceObservationInsert),
    InsertEvidenceProducer(EvidenceProducerInsert),
    InsertUserActionRequest(UserActionRequestInsert),
    InsertUserActionResolution(UserActionResolutionInsert),
    ResolveUnrecordedChange(UnrecordedChangeResolutionUpdate),
    InsertProjectContinuityRecord(ProjectContinuityRecordInsert),
    UpdateUserActionBasis(UserActionBasisUpdate),
    MarkUserActionBasesStatus(UserActionBasisStatusMark),
    MarkUserActionsSupersededOrStale(UserActionInvalidation),
    ApplyProjectWorkflowPolicy(ProjectWorkflowPolicyMutation),
}

/// Storage input for one authority-bound project workflow-policy replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWorkflowPolicyMutation {
    pub policy_version: u64,
    pub policy_json: String,
    pub policy_fingerprint: String,
    pub source: String,
    pub expected_prior_fingerprint: Option<String>,
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

/// Storage input for inserting an immutable Task-scoped supplemental claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceClaimInsert {
    pub evidence_claim_id: String,
    pub task_id: String,
    pub statement: String,
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

/// Storage input for inserting a pending user-action request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionRequestInsert {
    pub user_action_request_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub action_kind: UserActionKind,
    pub request_json: String,
    pub basis_json: String,
    pub basis_status: UserActionBasisStatus,
    pub required_for_json: String,
    pub requested_by_actor_source: String,
    pub source_method: String,
    pub source_idempotency_key: String,
    pub requested_at: String,
    pub expires_at: Option<String>,
    pub metadata_json: String,
}

/// Storage input for inserting one immutable user-action resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionResolutionInsert {
    pub user_action_resolution_id: String,
    pub user_action_request_id: String,
    pub action_kind: UserActionKind,
    pub channel_kind: UserActionChannelKind,
    pub channel_submission_id: String,
    pub resolution_json: String,
    pub resolved_by_actor_source: String,
    pub resolved_verification_basis: String,
    pub resolved_assurance_level: String,
    pub resolved_at: String,
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

/// Storage input for replacing one user-action basis snapshot and compatibility status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionBasisUpdate {
    pub user_action_request_id: String,
    pub basis_json: String,
    pub basis_status: UserActionBasisStatus,
}

/// Storage input for marking selected user-action basis rows stale or superseded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionBasisStatusMark {
    pub user_action_request_ids: Vec<String>,
    pub basis_status: UserActionBasisStatus,
}

/// Storage input for invalidating current user-action authority after state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionInvalidation {
    pub task_id: String,
    pub action_kinds: Vec<UserActionKind>,
}

/// Storage input for inserting one open write ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketInsert {
    pub write_ticket_id: String,
    pub task_id: String,
    pub change_unit_id: String,
    pub validity_basis_json: String,
    pub allowed_path_prefixes_json: String,
    pub denied_path_prefixes_json: String,
    pub attempt_scope_json: String,
    pub created_by_actor_source: String,
    pub created_by_user_action_resolution_id: Option<String>,
    pub idle_expires_at: Option<String>,
    pub created_at: String,
    pub metadata_json: String,
}

/// Storage input for invalidating every active write ticket for one Task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketInvalidation {
    pub task_id: String,
    pub invalidation_reason: String,
}

/// Storage input for invalidating one specifically identified active write ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketByIdInvalidation {
    pub write_ticket_id: String,
    pub invalidation_reason: String,
}

/// Storage input for closing one open write ticket through a compatible Run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketConsumption {
    pub write_ticket_id: String,
    pub run_id: String,
    pub expected_basis_state_version: u64,
    pub expected_write_authority_fingerprint: String,
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

/// Non-authoritative observation-time candidate used only for bounded workflow metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductWriteObservationCandidate {
    pub source_table: String,
    pub source_id: String,
    pub observed_paths_json: String,
    pub observed_at: String,
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
    pub expected_created_at: String,
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
    pub produced_at_state_version: u64,
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
    pub clock_floor: Option<String>,
    /// Whether commit time must also sample SQLite's live UTC clock.
    pub include_live_storage_time: bool,
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
    pub acceptance_criteria: u64,
    pub evidence_claims: u64,
    pub change_units: u64,
    pub authority_events: u64,
    pub tool_invocations: u64,
    pub user_action_requests: u64,
    pub user_action_resolutions: u64,
    pub write_tickets: u64,
    pub runs: u64,
    pub evidence_capture_intents: u64,
    pub evidence_capture_receipts: u64,
    pub evidence_capture_source_claims: u64,
    pub artifact_staging: u64,
    pub artifacts: u64,
    pub artifact_links: u64,
    pub evidence_summaries: u64,
    pub evidence_observations: u64,
    pub evidence_producers: u64,
    pub blockers: u64,
    pub project_continuity_records: u64,
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

/// Current Change Unit row data needed by Core method implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeUnitRecord {
    pub project_id: String,
    pub change_unit_id: String,
    pub task_id: String,
    pub status: String,
    pub is_current: bool,
    pub basis_state_version: u64,
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
    pub change_unit_id: String,
    pub basis_state_version: u64,
    pub status: String,
    pub validity_basis_json: String,
    pub allowed_path_prefixes_json: String,
    pub denied_path_prefixes_json: String,
    pub attempt_scope_json: String,
    pub idle_expires_at: Option<String>,
    pub invalidation_reason: Option<String>,
    pub created_at: String,
    pub consumed_by_run_id: Option<String>,
    pub consumed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WriteTicketRecordRaw {
    project_id: String,
    write_ticket_id: String,
    task_id: String,
    change_unit_id: Option<String>,
    basis_state_version: u64,
    status: String,
    validity_basis_json: String,
    allowed_path_prefixes_json: String,
    denied_path_prefixes_json: String,
    attempt_scope_json: String,
    idle_expires_at: Option<String>,
    invalidation_reason: Option<String>,
    created_at: String,
    consumed_by_run_id: Option<String>,
    consumed_at: Option<String>,
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
    pub created_at: String,
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

/// Stored user-action request row data needed by Core method implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionRequestRecord {
    pub project_id: String,
    pub user_action_request_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub action_kind: UserActionKind,
    pub request_json: String,
    pub basis_json: String,
    pub basis_status: UserActionBasisStatus,
    pub required_for_json: String,
    pub requested_by_actor_source: String,
    pub source_method: String,
    pub source_idempotency_key: String,
    pub requested_at: String,
    pub expires_at: Option<String>,
    pub metadata_json: String,
}

/// Stored immutable user-action resolution row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionResolutionRecord {
    pub project_id: String,
    pub user_action_resolution_id: String,
    pub user_action_request_id: String,
    pub action_kind: UserActionKind,
    pub channel_kind: UserActionChannelKind,
    pub channel_submission_id: String,
    pub resolution_json: String,
    pub resolved_by_actor_source: String,
    pub resolved_verification_basis: String,
    pub resolved_assurance_level: String,
    pub resolved_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UserActionRequestRecordRaw {
    project_id: String,
    user_action_request_id: String,
    task_id: String,
    change_unit_id: Option<String>,
    action_kind: String,
    request_json: String,
    basis_json: String,
    basis_status: String,
    required_for_json: String,
    requested_by_actor_source: String,
    source_method: String,
    source_idempotency_key: String,
    requested_at: String,
    expires_at: Option<String>,
    metadata_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UserActionResolutionRecordRaw {
    project_id: String,
    user_action_resolution_id: String,
    user_action_request_id: String,
    action_kind: String,
    channel_kind: String,
    channel_submission_id: String,
    resolution_json: String,
    resolved_by_actor_source: String,
    resolved_verification_basis: String,
    resolved_assurance_level: String,
    resolved_at: String,
}

/// Stored request and optional resolution with its derived current lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveUserActionRecord {
    pub request: UserActionRequestRecord,
    pub resolution: Option<UserActionResolutionRecord>,
    pub status: UserActionStatus,
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

/// One strictly bounded active project-continuity page read from a single snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveProjectContinuityPage {
    pub records: Vec<ProjectContinuityRecordRecord>,
    pub total_count: u64,
    pub truncated: bool,
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
    committed_at: &'tx str,
    tx: &'tx Transaction<'tx>,
}

mod commit;
mod mutation_apply;
mod open;
mod replay;
pub(crate) mod validation;

impl CoreProjectStore {
    /// Runs related read-only lookups against one SQLite snapshot.
    ///
    /// The deferred transaction pins its snapshot at the closure's first read,
    /// so callers can attach one project-state version to a compound projection
    /// without mixing rows from a concurrent authority commit.
    pub fn with_read_snapshot<T>(
        &self,
        read: impl FnOnce(&Self) -> StoreResult<T>,
    ) -> StoreResult<T> {
        let transaction = self.conn.unchecked_transaction()?;
        let value = read(self)?;
        transaction.commit()?;
        Ok(value)
    }

    /// Reads the current project-state header.
    pub fn project_state(&self) -> StoreResult<ProjectStateHeader> {
        read_project_state(&self.conn, &self.project.project_id)
    }

    /// Reads one Agent Session through this handle's current SQLite snapshot.
    pub fn agent_session(&self, session_id: &str) -> StoreResult<Option<AgentSessionRecord>> {
        validate_identifier("session_id", session_id)?;
        agent_session_from_conn(&self.conn, &self.project.project_id, session_id)
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

    /// Lists confirmed, Task-bound product-write observation candidates without assigning
    /// authority or interpreting their path payloads.
    pub fn product_write_observation_candidates_for_task(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Vec<ProductWriteObservationCandidate>> {
        validate_identifier("task_id", task_id.as_str())?;
        let mut candidates = Vec::new();
        let mut expected = self.conn.prepare(
            "SELECT expected_write_id, matched_paths_json, matched_at
               FROM expected_writes
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'matched'",
        )?;
        let rows =
            expected.query_map(params![self.project.project_id, task_id.as_str()], |row| {
                Ok(ProductWriteObservationCandidate {
                    source_table: "expected_writes".to_owned(),
                    source_id: row.get(0)?,
                    observed_paths_json: row.get(1)?,
                    observed_at: row.get(2)?,
                })
            })?;
        for row in rows {
            candidates.push(row?);
        }

        let mut unrecorded = self.conn.prepare(
            "SELECT unrecorded_change_id, observed_paths_json, detected_at
               FROM unrecorded_changes
              WHERE project_id = ?1
                AND task_id = ?2
                AND confidence = 'confirmed'",
        )?;
        let rows =
            unrecorded.query_map(params![self.project.project_id, task_id.as_str()], |row| {
                Ok(ProductWriteObservationCandidate {
                    source_table: "unrecorded_changes".to_owned(),
                    source_id: row.get(0)?,
                    observed_paths_json: row.get(1)?,
                    observed_at: row.get(2)?,
                })
            })?;
        for row in rows {
            candidates.push(row?);
        }
        Ok(candidates)
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
        row_exists(
            &self.conn,
            &self.project.project_id,
            "acceptance_criteria",
            "acceptance_criterion_id",
            acceptance_criterion_id,
        )
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
        has_prepared_artifact_input(&self.conn, &self.project.project_id, task_id.as_str(), now)
    }

    /// Returns whether a committed event id already exists in this project.
    pub fn event_id_exists(&self, event_id: &str) -> StoreResult<bool> {
        row_exists(
            &self.conn,
            &self.project.project_id,
            "authority_events",
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

    /// Returns whether a persistent artifact has one exact owner relation.
    pub fn artifact_has_owner_link(
        &self,
        artifact_id: &str,
        task_id: &str,
        owner_record_kind: &str,
        owner_record_id: &str,
    ) -> StoreResult<bool> {
        artifact_has_owner_link(
            &self.conn,
            &self.project.project_id,
            artifact_id,
            task_id,
            owner_record_kind,
            owner_record_id,
        )
    }

    /// Lists effective pending user-action refs for a Task at the supplied instant.
    pub fn pending_user_action_refs(
        &self,
        task_id: &TaskId,
        state_version: u64,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<StoredRecordRef>> {
        effective_user_action_refs(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            UserActionStatus::Pending,
            state_version,
            now,
        )
    }

    /// Lists effective pending user-action records for a Task.
    pub fn pending_user_action_records(
        &self,
        task_id: &TaskId,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<EffectiveUserActionRecord>> {
        effective_user_action_records_for_task(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            Some(UserActionStatus::Pending),
            now,
        )
    }

    /// Lists all user-action records for a Task in stable creation order.
    pub fn user_action_records_for_task(
        &self,
        task_id: &TaskId,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<EffectiveUserActionRecord>> {
        effective_user_action_records_for_task(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            None,
            now,
        )
    }

    /// Lists stale or superseded user-action refs for a Task and action kind.
    pub fn non_current_user_action_refs(
        &self,
        task_id: &TaskId,
        action_kind: UserActionKind,
        state_version: u64,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<StoredRecordRef>> {
        non_current_user_action_refs(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            action_kind,
            state_version,
            now,
        )
    }

    /// Reads one user-action request and optional resolution by request identity.
    pub fn user_action_record(
        &self,
        user_action_request_id: &str,
        now: &UtcTimestamp,
    ) -> StoreResult<Option<EffectiveUserActionRecord>> {
        effective_user_action_record(
            &self.conn,
            &self.project.project_id,
            user_action_request_id,
            now,
        )
    }

    /// Returns whether a user-action request id exists in this project.
    pub fn user_action_request_id_exists(&self, user_action_request_id: &str) -> StoreResult<bool> {
        row_exists(
            &self.conn,
            &self.project.project_id,
            "user_action_requests",
            "user_action_request_id",
            user_action_request_id,
        )
    }

    /// Reads one user-action resolution by its exact project-local identity.
    pub fn user_action_resolution_record(
        &self,
        user_action_resolution_id: &str,
    ) -> StoreResult<Option<UserActionResolutionRecord>> {
        user_action_resolution_record_by_id(
            &self.conn,
            &self.project.project_id,
            user_action_resolution_id,
        )
    }

    /// Reads one user-action resolution by its stable channel submission identity.
    pub fn user_action_resolution_for_channel_submission(
        &self,
        channel_kind: UserActionChannelKind,
        channel_submission_id: &str,
    ) -> StoreResult<Option<UserActionResolutionRecord>> {
        user_action_resolution_record_by_channel_submission(
            &self.conn,
            &self.project.project_id,
            channel_kind,
            channel_submission_id,
        )
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

    /// Reads one active project-continuity page in canonical status order.
    pub fn active_project_continuity_page(
        &self,
        page_size: u64,
        cursor: Option<&ContinuityCursor>,
    ) -> StoreResult<ActiveProjectContinuityPage> {
        active_project_continuity_page(&self.conn, &self.project.project_id, page_size, cursor)
    }

    /// Lists project-continuity rows that originated from one Task.
    pub fn project_continuity_records_for_task(
        &self,
        task_id: &str,
    ) -> StoreResult<Vec<ProjectContinuityRecordRecord>> {
        project_continuity_records_for_task(&self.conn, &self.project.project_id, task_id)
    }

    /// Lists effective resolved user-action records for a Task and action kind.
    pub fn resolved_user_action_records(
        &self,
        task_id: &TaskId,
        action_kind: UserActionKind,
        now: &UtcTimestamp,
    ) -> StoreResult<Vec<EffectiveUserActionRecord>> {
        effective_user_action_records_for_task_and_kind(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            action_kind,
            UserActionStatus::Resolved,
            now,
        )
    }

    /// Returns the monotonic Core current UTC clock for this Store handle.
    pub fn current_timestamp(&self) -> StoreResult<String> {
        let local_floor = self.last_clock_sample.borrow().clone();
        let timestamp = project_current_utc_timestamp_for_conn(
            &self.conn,
            &self.project.project_id,
            local_floor.as_ref(),
        )?;
        *self.last_clock_sample.borrow_mut() = Some(timestamp.clone());
        Ok(timestamp.to_string())
    }

    /// Returns the persisted project clock floor combined with samples already
    /// accepted on this Store handle, without sampling SQLite wall-clock time.
    pub fn current_clock_floor(&self) -> StoreResult<UtcTimestamp> {
        let persisted = self.project_state()?;
        let persisted = UtcTimestamp::parse(&persisted.updated_at).map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "project_state",
                &self.project.project_id,
                "updated_at",
            )
        })?;
        persisted
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| {
                StoreError::corrupt_owner_state_value(
                    "project_state",
                    &self.project.project_id,
                    "updated_at",
                )
            })?;
        let local = self.last_clock_sample.borrow().as_ref().cloned();
        if local
            .as_ref()
            .is_some_and(|timestamp| timestamp.ensure_canonical_rfc3339_representable().is_err())
        {
            return Err(StoreError::SchemaInvariant {
                database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
                detail: "Core Store handle clock sample is outside the canonical RFC 3339 range"
                    .to_owned(),
            });
        }
        Ok(local.map_or(persisted.clone(), |local| std::cmp::max(persisted, local)))
    }

    /// Carries an injected Core clock sample through this Store handle's next commit.
    pub fn remember_clock_sample(&self, sample: &UtcTimestamp) {
        let current = self.last_clock_sample.borrow().clone();
        if current.as_ref().is_none_or(|current| sample > current) {
            *self.last_clock_sample.borrow_mut() = Some(sample.clone());
        }
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
            acceptance_criteria: table_count(
                &self.conn,
                "acceptance_criteria",
                &self.project.project_id,
            )?,
            evidence_claims: table_count(&self.conn, "evidence_claims", &self.project.project_id)?,
            change_units: table_count(&self.conn, "change_units", &self.project.project_id)?,
            authority_events: table_count(
                &self.conn,
                "authority_events",
                &self.project.project_id,
            )?,
            tool_invocations: table_count(
                &self.conn,
                "tool_invocations",
                &self.project.project_id,
            )?,
            user_action_requests: table_count(
                &self.conn,
                "user_action_requests",
                &self.project.project_id,
            )?,
            user_action_resolutions: table_count(
                &self.conn,
                "user_action_resolutions",
                &self.project.project_id,
            )?,
            write_tickets: table_count(&self.conn, "write_tickets", &self.project.project_id)?,
            runs: table_count(&self.conn, "runs", &self.project.project_id)?,
            evidence_capture_intents: table_count(
                &self.conn,
                "evidence_capture_intents",
                &self.project.project_id,
            )?,
            evidence_capture_receipts: table_count(
                &self.conn,
                "evidence_capture_receipts",
                &self.project.project_id,
            )?,
            evidence_capture_source_claims: table_count(
                &self.conn,
                "evidence_capture_source_claims",
                &self.project.project_id,
            )?,
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
            evidence_producers: table_count(
                &self.conn,
                "evidence_producers",
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
    let state = conn
        .query_row(
            "SELECT
            project_id,
            state_version,
            active_task_id,
            updated_at
         FROM project_state
         WHERE project_id = ?1",
            params![project_id],
            project_state_from_row,
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project_state",
            id: project_id.to_owned(),
        })?;
    validate_project_state_updated_at(&state)?;
    Ok(state)
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
    let mut stmt = conn.prepare(
        "SELECT project_id, acceptance_criterion_id, task_id, statement, evidence_requirement, position, status
           FROM acceptance_criteria
          WHERE project_id = ?1
            AND task_id = ?2
            AND status = 'active'
          ORDER BY position, acceptance_criterion_id",
    )?;
    let rows = stmt.query_map(
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
    conn.query_row(
        "SELECT project_id, acceptance_criterion_id, task_id, statement, evidence_requirement, position, status
           FROM acceptance_criteria
          WHERE project_id = ?1
            AND acceptance_criterion_id = ?2",
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
    conn.query_row(
        "SELECT project_id, evidence_claim_id, task_id, statement
          FROM evidence_claims
          WHERE project_id = ?1
            AND task_id = ?2
            AND evidence_claim_id = ?3",
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
    let sql = format!(
        "SELECT {CHANGE_UNIT_RECORD_COLUMNS}
           FROM change_units
          WHERE project_id = ?1
            AND task_id = ?2
            AND status = 'active'
            AND is_current = 1"
    );
    conn.query_row(
        &sql,
        params![project_id, task_id],
        raw_change_unit_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)?
    .map(validate_decoded_change_unit_record)
    .transpose()
}

fn change_unit_record(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    change_unit_id: &str,
) -> StoreResult<Option<ChangeUnitRecord>> {
    let sql = format!(
        "SELECT {CHANGE_UNIT_RECORD_COLUMNS}
           FROM change_units
          WHERE project_id = ?1
            AND task_id = ?2
            AND change_unit_id = ?3"
    );
    conn.query_row(
        &sql,
        params![project_id, task_id, change_unit_id],
        raw_change_unit_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)?
    .map(validate_decoded_change_unit_record)
    .transpose()
}

struct RawChangeUnitRecord {
    project_id: String,
    change_unit_id: String,
    task_id: String,
    status: String,
    is_current: i64,
    basis_state_version: Option<i64>,
    scope_summary_json: String,
    bounded_paths_json: String,
    write_basis_json: String,
    effect_contract_json: String,
    lifecycle_json: String,
}

fn raw_change_unit_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RawChangeUnitRecord> {
    Ok(RawChangeUnitRecord {
        project_id: row.get(0)?,
        change_unit_id: row.get(1)?,
        task_id: row.get(2)?,
        status: row.get(3)?,
        is_current: row.get(4)?,
        basis_state_version: row.get(5)?,
        scope_summary_json: row.get(6)?,
        bounded_paths_json: row.get(7)?,
        write_basis_json: row.get(8)?,
        effect_contract_json: row.get(9)?,
        lifecycle_json: row.get(10)?,
    })
}

fn validate_decoded_change_unit_record(
    record: RawChangeUnitRecord,
) -> StoreResult<ChangeUnitRecord> {
    let corrupt_value = |logical_column| {
        StoreError::corrupt_owner_state_value(
            "change_units",
            record.change_unit_id.clone(),
            logical_column,
        )
    };
    let basis_state_version = record
        .basis_state_version
        .ok_or_else(|| corrupt_value("basis_state_version"))
        .and_then(|value| u64::try_from(value).map_err(|_| corrupt_value("basis_state_version")))?;
    let is_current = match record.is_current {
        0 => false,
        1 => true,
        _ => return Err(corrupt_value("is_current")),
    };
    if !matches!(
        record.status.as_str(),
        "proposed" | "active" | "replaced" | "closed"
    ) {
        return Err(corrupt_value("status"));
    }
    Ok(ChangeUnitRecord {
        project_id: record.project_id,
        change_unit_id: record.change_unit_id,
        task_id: record.task_id,
        status: record.status,
        is_current,
        basis_state_version,
        scope_summary_json: record.scope_summary_json,
        bounded_paths_json: record.bounded_paths_json,
        write_basis_json: record.write_basis_json,
        effect_contract_json: record.effect_contract_json,
        lifecycle_json: record.lifecycle_json,
    })
}

fn active_write_tickets(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<WriteTicketRecord>> {
    let sql = format!(
        "SELECT {WRITE_TICKET_RECORD_COLUMNS}
           FROM write_tickets
          WHERE project_id = ?1
            AND task_id = ?2
            AND status = 'active'
          ORDER BY write_ticket_id"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        params![project_id, task_id],
        write_ticket_record_raw_from_row,
    )?;
    let mut records = Vec::new();
    for row in rows {
        records.push(decode_write_ticket_record(row?)?);
    }
    Ok(records)
}

fn write_tickets_for_task(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<WriteTicketRecord>> {
    let sql = format!(
        "SELECT {WRITE_TICKET_RECORD_COLUMNS}
           FROM write_tickets
          WHERE project_id = ?1
            AND task_id = ?2
          ORDER BY basis_state_version DESC, write_ticket_id"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        params![project_id, task_id],
        write_ticket_record_raw_from_row,
    )?;
    let mut records = Vec::new();
    for row in rows {
        records.push(decode_write_ticket_record(row?)?);
    }
    Ok(records)
}

fn write_ticket_record(
    conn: &Connection,
    project_id: &str,
    write_ticket_id: &str,
) -> StoreResult<Option<WriteTicketRecord>> {
    let sql = format!(
        "SELECT {WRITE_TICKET_RECORD_COLUMNS}
           FROM write_tickets
          WHERE project_id = ?1
            AND write_ticket_id = ?2"
    );
    conn.query_row(
        &sql,
        params![project_id, write_ticket_id],
        write_ticket_record_raw_from_row,
    )
    .optional()
    .map_err(StoreError::from)?
    .map(decode_write_ticket_record)
    .transpose()
}

fn write_ticket_record_raw_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<WriteTicketRecordRaw> {
    let basis_state_version = row.get::<_, i64>(4)?;
    Ok(WriteTicketRecordRaw {
        project_id: row.get(0)?,
        write_ticket_id: row.get(1)?,
        task_id: row.get(2)?,
        change_unit_id: row.get(3)?,
        basis_state_version: nonnegative_i64_to_u64(
            "write_tickets.basis_state_version",
            basis_state_version,
        )?,
        status: row.get(5)?,
        validity_basis_json: row.get(6)?,
        allowed_path_prefixes_json: row.get(7)?,
        denied_path_prefixes_json: row.get(8)?,
        attempt_scope_json: row.get(9)?,
        idle_expires_at: row.get(10)?,
        invalidation_reason: row.get(11)?,
        created_at: row.get(12)?,
        consumed_by_run_id: row.get(13)?,
        consumed_at: row.get(14)?,
    })
}

fn decode_write_ticket_record(raw: WriteTicketRecordRaw) -> StoreResult<WriteTicketRecord> {
    let change_unit_id = raw
        .change_unit_id
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            StoreError::corrupt_owner_state_value(
                "write_tickets",
                raw.write_ticket_id.clone(),
                "change_unit_id",
            )
        })?;
    Ok(WriteTicketRecord {
        project_id: raw.project_id,
        write_ticket_id: raw.write_ticket_id,
        task_id: raw.task_id,
        change_unit_id,
        basis_state_version: raw.basis_state_version,
        status: raw.status,
        validity_basis_json: raw.validity_basis_json,
        allowed_path_prefixes_json: raw.allowed_path_prefixes_json,
        denied_path_prefixes_json: raw.denied_path_prefixes_json,
        attempt_scope_json: raw.attempt_scope_json,
        idle_expires_at: raw.idle_expires_at,
        invalidation_reason: raw.invalidation_reason,
        created_at: raw.created_at,
        consumed_by_run_id: raw.consumed_by_run_id,
        consumed_at: raw.consumed_at,
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
            rowid,
            project_id,
            run_id,
            task_id,
            change_unit_id,
            observed_changes_json,
            status
         FROM runs
         WHERE project_id = ?1
           AND task_id = ?2
         ORDER BY rowid DESC",
    )?;
    let rows = stmt.query_map(params![project_id, task_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
        ))
    })?;
    let mut records = Vec::new();
    for row in rows {
        let (rowid, project_id, run_id, task_id, change_unit_id, observed_changes_json, status) =
            row?;
        let observed_changes = decode_owner_json_text::<ObservedChanges>(
            "runs",
            run_id.clone(),
            "observed_changes_json",
            &observed_changes_json,
        )?;
        records.push((
            rowid,
            RunObservedChangesRecord {
                project_id,
                run_id,
                task_id,
                change_unit_id,
                observed_changes,
                status,
            },
        ));
    }
    let mut event_stmt = conn.prepare(
        "SELECT event_seq, payload_json
           FROM authority_events
          WHERE project_id = ?1
            AND task_id = ?2
            AND event_type = 'run_recorded'
          ORDER BY event_seq DESC",
    )?;
    let event_rows = event_stmt.query_map(params![project_id, task_id], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut event_order = BTreeMap::new();
    for row in event_rows {
        let (event_seq, payload_json) = row?;
        let payload = decode_owner_json_text::<serde_json::Value>(
            "authority_events",
            format!("event_seq:{event_seq}"),
            "payload_json",
            &payload_json,
        )?;
        let run_id = payload
            .get("run_id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                StoreError::corrupt_owner_state_value(
                    "authority_events",
                    format!("event_seq:{event_seq}"),
                    "payload_json.run_id",
                )
            })?;
        event_order.entry(run_id.to_owned()).or_insert(event_seq);
    }
    let mut ordered_records = records
        .into_iter()
        .map(|(rowid, record)| {
            let event_seq = event_order.get(&record.run_id).copied().ok_or_else(|| {
                StoreError::corrupt_owner_state_value(
                    "runs",
                    record.run_id.clone(),
                    "authority_events.run_recorded",
                )
            })?;
            Ok((event_seq, rowid, record))
        })
        .collect::<StoreResult<Vec<_>>>()?;
    ordered_records.sort_by(
        |(left_event_seq, left_rowid, _), (right_event_seq, right_rowid, _)| {
            right_event_seq
                .cmp(left_event_seq)
                .then_with(|| right_rowid.cmp(left_rowid))
        },
    );
    Ok(ordered_records
        .into_iter()
        .map(|(_, _, record)| record)
        .collect())
}

fn artifact_staging_record(
    conn: &Connection,
    project_id: &str,
    handle_id: &str,
) -> StoreResult<Option<StoredArtifactStagingRecord>> {
    let record = conn
        .query_row(
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
            created_at,
            expires_at
         FROM artifact_staging
         WHERE project_id = ?1
           AND handle_id = ?2",
            params![project_id, handle_id],
            artifact_staging_record_from_row,
        )
        .optional()?;
    record
        .map(validate_stored_artifact_staging_record)
        .transpose()
}

fn has_prepared_artifact_input(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    now: &UtcTimestamp,
) -> StoreResult<bool> {
    let mut stmt = conn.prepare(
        "SELECT handle_id, created_at, expires_at
           FROM artifact_staging
          WHERE project_id = ?1
            AND task_id = ?2
            AND status = 'staged'",
    )?;
    let rows = stmt.query_map(params![project_id, task_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut windows = Vec::new();
    for row in rows {
        let (handle_id, created_at, expires_at) = row?;
        windows.push(stored_artifact_staging_window(
            &handle_id,
            &created_at,
            &expires_at,
        )?);
    }
    Ok(windows
        .iter()
        .any(|(created_at, expires_at)| created_at <= now && now < expires_at))
}

fn artifact_staging_record_tx(
    tx: &Transaction<'_>,
    project_id: &str,
    handle_id: &str,
) -> StoreResult<Option<StoredArtifactStagingRecord>> {
    let record = tx
        .query_row(
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
            created_at,
            expires_at
         FROM artifact_staging
         WHERE project_id = ?1
           AND handle_id = ?2",
            params![project_id, handle_id],
            artifact_staging_record_from_row,
        )
        .optional()?;
    record
        .map(validate_stored_artifact_staging_record)
        .transpose()
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
        created_at: row.get(11)?,
        expires_at: row.get(12)?,
    })
}

fn validate_stored_artifact_staging_record(
    record: StoredArtifactStagingRecord,
) -> StoreResult<StoredArtifactStagingRecord> {
    stored_artifact_staging_window(&record.handle_id, &record.created_at, &record.expires_at)?;
    Ok(record)
}

fn stored_artifact_staging_window(
    handle_id: &str,
    created_at: &str,
    expires_at: &str,
) -> StoreResult<(UtcTimestamp, UtcTimestamp)> {
    let parse = |field, value: &str| {
        let timestamp = UtcTimestamp::parse(value).map_err(|_| {
            StoreError::corrupt_owner_state_value("artifact_staging", handle_id, field)
        })?;
        timestamp
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| {
                StoreError::corrupt_owner_state_value("artifact_staging", handle_id, field)
            })?;
        Ok::<_, StoreError>(timestamp)
    };
    let created_at = parse("created_at", created_at)?;
    let expires_at = parse("expires_at", expires_at)?;
    if expires_at <= created_at {
        return Err(StoreError::corrupt_owner_state_value(
            "artifact_staging",
            handle_id,
            "expires_at",
        ));
    }
    Ok((created_at, expires_at))
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

fn artifact_has_owner_link(
    conn: &Connection,
    project_id: &str,
    artifact_id: &str,
    task_id: &str,
    owner_record_kind: &str,
    owner_record_id: &str,
) -> StoreResult<bool> {
    conn.query_row(
        "SELECT COUNT(*)
           FROM artifact_links
          WHERE project_id = ?1
            AND artifact_id = ?2
            AND task_id = ?3
            AND owner_record_kind = ?4
            AND owner_record_id = ?5",
        params![
            project_id,
            artifact_id,
            task_id,
            owner_record_kind,
            owner_record_id
        ],
        |row| Ok(row.get::<_, i64>(0)? > 0),
    )
    .map_err(StoreError::from)
}

fn latest_evidence_summary(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Option<EvidenceSummaryRecord>> {
    let record = conn
        .query_row(
            "SELECT
            project_id,
            evidence_summary_id,
            task_id,
            change_unit_id,
            produced_at_state_version,
            status,
            coverage_json,
            supporting_refs_json,
            gap_refs_json,
            metadata_json
         FROM evidence_summaries
         WHERE project_id = ?1
           AND task_id = ?2
         ORDER BY produced_at_state_version DESC
         LIMIT 1",
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
    let record = conn
        .query_row(
            "SELECT
            project_id,
            evidence_summary_id,
            task_id,
            change_unit_id,
            produced_at_state_version,
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
    conn.query_row(
        "SELECT
            project_id,
            evidence_observation_id,
            task_id,
            change_unit_id,
            run_id,
            acceptance_criterion_id,
            evidence_claim_id,
            source_kind,
            assurance_level,
            observed_by_actor_source,
            tool_name,
            tool_invocation_id,
            tool_metadata_json,
            input_refs_json,
            source_refs_json,
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

pub(crate) fn user_action_request_record(
    conn: &Connection,
    project_id: &str,
    user_action_request_id: &str,
) -> StoreResult<Option<UserActionRequestRecord>> {
    let raw = conn
        .query_row(
            "SELECT
            project_id,
            user_action_request_id,
            task_id,
            change_unit_id,
            action_kind,
            request_json,
            basis_json,
            basis_status,
            required_for_json,
            requested_by_actor_source,
            source_method,
            source_idempotency_key,
            requested_at,
            expires_at,
            metadata_json
         FROM user_action_requests
         WHERE project_id = ?1
           AND user_action_request_id = ?2",
            params![project_id, user_action_request_id],
            user_action_request_record_raw_from_row,
        )
        .optional()?;
    raw.map(decode_user_action_request_record).transpose()
}

fn user_action_resolution_record_by_request(
    conn: &Connection,
    project_id: &str,
    user_action_request_id: &str,
) -> StoreResult<Option<UserActionResolutionRecord>> {
    let raw = conn
        .query_row(
            "SELECT
                project_id,
                user_action_resolution_id,
                user_action_request_id,
                action_kind,
                channel_kind,
                channel_submission_id,
                resolution_json,
                resolved_by_actor_source,
                resolved_verification_basis,
                resolved_assurance_level,
                resolved_at
             FROM user_action_resolutions
             WHERE project_id = ?1
               AND user_action_request_id = ?2",
            params![project_id, user_action_request_id],
            user_action_resolution_record_raw_from_row,
        )
        .optional()?;
    raw.map(decode_user_action_resolution_record).transpose()
}

fn user_action_resolution_record_by_id(
    conn: &Connection,
    project_id: &str,
    user_action_resolution_id: &str,
) -> StoreResult<Option<UserActionResolutionRecord>> {
    let raw = conn
        .query_row(
            "SELECT
                project_id,
                user_action_resolution_id,
                user_action_request_id,
                action_kind,
                channel_kind,
                channel_submission_id,
                resolution_json,
                resolved_by_actor_source,
                resolved_verification_basis,
                resolved_assurance_level,
                resolved_at
             FROM user_action_resolutions
             WHERE project_id = ?1
               AND user_action_resolution_id = ?2",
            params![project_id, user_action_resolution_id],
            user_action_resolution_record_raw_from_row,
        )
        .optional()?;
    let resolution = raw.map(decode_user_action_resolution_record).transpose()?;
    validate_resolution_with_stored_request(conn, project_id, resolution)
}

fn user_action_resolution_record_by_channel_submission(
    conn: &Connection,
    project_id: &str,
    channel_kind: UserActionChannelKind,
    channel_submission_id: &str,
) -> StoreResult<Option<UserActionResolutionRecord>> {
    let raw = conn
        .query_row(
            "SELECT
                project_id,
                user_action_resolution_id,
                user_action_request_id,
                action_kind,
                channel_kind,
                channel_submission_id,
                resolution_json,
                resolved_by_actor_source,
                resolved_verification_basis,
                resolved_assurance_level,
                resolved_at
             FROM user_action_resolutions
             WHERE project_id = ?1
               AND channel_kind = ?2
               AND channel_submission_id = ?3",
            params![
                project_id,
                user_action_channel_kind_as_str(channel_kind),
                channel_submission_id
            ],
            user_action_resolution_record_raw_from_row,
        )
        .optional()?;
    let resolution = raw.map(decode_user_action_resolution_record).transpose()?;
    validate_resolution_with_stored_request(conn, project_id, resolution)
}

fn user_action_request_record_raw_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<UserActionRequestRecordRaw> {
    Ok(UserActionRequestRecordRaw {
        project_id: row.get(0)?,
        user_action_request_id: row.get(1)?,
        task_id: row.get(2)?,
        change_unit_id: row.get(3)?,
        action_kind: row.get(4)?,
        request_json: row.get(5)?,
        basis_json: row.get(6)?,
        basis_status: row.get(7)?,
        required_for_json: row.get(8)?,
        requested_by_actor_source: row.get(9)?,
        source_method: row.get(10)?,
        source_idempotency_key: row.get(11)?,
        requested_at: row.get(12)?,
        expires_at: row.get(13)?,
        metadata_json: row.get(14)?,
    })
}

fn decode_user_action_request_record(
    raw: UserActionRequestRecordRaw,
) -> StoreResult<UserActionRequestRecord> {
    let record_id = raw.user_action_request_id.as_str();
    let action_kind = parse_user_action_kind(
        record_id,
        "user_action_requests.action_kind",
        &raw.action_kind,
    )?;
    let basis_status = parse_user_action_basis_status(
        record_id,
        "user_action_requests.basis_status",
        &raw.basis_status,
    )?;
    validate_json_text("user_action_requests.metadata_json", &raw.metadata_json).map_err(|_| {
        StoreError::corrupt_owner_state_json("user_action_requests", record_id, "metadata_json")
    })?;
    if raw.source_method != MethodName::RequestUserAction.as_str()
        && raw.source_method != MethodName::ReconcileChanges.as_str()
    {
        return Err(StoreError::corrupt_owner_state_value(
            "user_action_requests",
            record_id,
            "source_method",
        ));
    }
    validate_identifier(
        "user_action_requests.source_idempotency_key",
        &raw.source_idempotency_key,
    )
    .map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            record_id,
            "source_idempotency_key",
        )
    })?;
    validate_stored_timestamp("user_action_requests.requested_at", &raw.requested_at)?;
    if let Some(expires_at) = &raw.expires_at {
        validate_stored_timestamp("user_action_requests.expires_at", expires_at)?;
    }
    validate_stored_user_action_request_column_agreement(
        record_id,
        UserActionRequestColumnFacts {
            task_id: &raw.task_id,
            change_unit_id: raw.change_unit_id.as_deref(),
            request_json: &raw.request_json,
            basis_json: &raw.basis_json,
            required_for_json: &raw.required_for_json,
            requested_at: &raw.requested_at,
            expires_at: raw.expires_at.as_deref(),
            action_kind,
            basis_status,
        },
    )?;
    Ok(UserActionRequestRecord {
        project_id: raw.project_id,
        user_action_request_id: raw.user_action_request_id,
        task_id: raw.task_id,
        change_unit_id: raw.change_unit_id,
        action_kind,
        request_json: raw.request_json,
        basis_json: raw.basis_json,
        basis_status,
        required_for_json: raw.required_for_json,
        requested_by_actor_source: raw.requested_by_actor_source,
        source_method: raw.source_method,
        source_idempotency_key: raw.source_idempotency_key,
        requested_at: raw.requested_at,
        expires_at: raw.expires_at,
        metadata_json: raw.metadata_json,
    })
}

fn user_action_resolution_record_raw_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<UserActionResolutionRecordRaw> {
    Ok(UserActionResolutionRecordRaw {
        project_id: row.get(0)?,
        user_action_resolution_id: row.get(1)?,
        user_action_request_id: row.get(2)?,
        action_kind: row.get(3)?,
        channel_kind: row.get(4)?,
        channel_submission_id: row.get(5)?,
        resolution_json: row.get(6)?,
        resolved_by_actor_source: row.get(7)?,
        resolved_verification_basis: row.get(8)?,
        resolved_assurance_level: row.get(9)?,
        resolved_at: row.get(10)?,
    })
}

fn decode_user_action_resolution_record(
    raw: UserActionResolutionRecordRaw,
) -> StoreResult<UserActionResolutionRecord> {
    let record_id = raw.user_action_resolution_id.as_str();
    let action_kind = parse_user_action_kind(
        record_id,
        "user_action_resolutions.action_kind",
        &raw.action_kind,
    )?;
    let channel_kind = parse_user_action_channel_kind(
        record_id,
        "user_action_resolutions.channel_kind",
        &raw.channel_kind,
    )?;
    validate_user_action_resolution_column_agreement(
        &raw.resolution_json,
        action_kind,
        &raw.user_action_resolution_id,
    )
    .map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            record_id,
            "resolution_json",
        )
    })?;
    if validate_channel_submission_id(&raw.channel_submission_id).is_err()
        || validate_user_action_resolution_provenance(
            channel_kind,
            &raw.resolved_by_actor_source,
            &raw.resolved_verification_basis,
            &raw.resolved_assurance_level,
        )
        .is_err()
    {
        return Err(StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            record_id,
            "resolved_verification_basis",
        ));
    }
    validate_stored_timestamp("user_action_resolutions.resolved_at", &raw.resolved_at)?;
    Ok(UserActionResolutionRecord {
        project_id: raw.project_id,
        user_action_resolution_id: raw.user_action_resolution_id,
        user_action_request_id: raw.user_action_request_id,
        action_kind,
        channel_kind,
        channel_submission_id: raw.channel_submission_id,
        resolution_json: raw.resolution_json,
        resolved_by_actor_source: raw.resolved_by_actor_source,
        resolved_verification_basis: raw.resolved_verification_basis,
        resolved_assurance_level: raw.resolved_assurance_level,
        resolved_at: raw.resolved_at,
    })
}

fn active_project_continuity_page(
    conn: &Connection,
    project_id: &str,
    page_size: u64,
    cursor: Option<&ContinuityCursor>,
) -> StoreResult<ActiveProjectContinuityPage> {
    if !(1..=MAX_CONTINUITY_PAGE_SIZE).contains(&page_size) {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "project_continuity_records page_size must be between 1 and {MAX_CONTINUITY_PAGE_SIZE}"
            ),
        });
    }

    let (cursor_updated_at, cursor_record_id) = match cursor {
        Some(cursor) => {
            cursor
                .updated_at
                .ensure_canonical_rfc3339_representable()
                .map_err(|_| StoreError::InvalidInput {
                    detail: "project_continuity_records cursor.updated_at is not representable as canonical RFC 3339 UTC"
                        .to_owned(),
                })?;
            validate_identifier(
                "project_continuity_records cursor.continuity_record_id",
                cursor.continuity_record_id.as_str(),
            )?;
            (
                Some(cursor.updated_at.to_canonical_string()),
                Some(cursor.continuity_record_id.as_str()),
            )
        }
        None => (None, None),
    };
    let fetch_limit = i64::try_from(page_size + 1).map_err(|_| StoreError::InvalidInput {
        detail: "project_continuity_records page_size cannot be represented by SQLite".to_owned(),
    })?;
    let page_size = usize::try_from(page_size).map_err(|_| StoreError::InvalidInput {
        detail: "project_continuity_records page_size cannot be represented by this platform"
            .to_owned(),
    })?;

    let transaction = conn.unchecked_transaction()?;
    let total_count: i64 = transaction.query_row(
        "SELECT COUNT(*)
           FROM project_continuity_records
          WHERE project_id = ?1
            AND status = 'active'",
        [project_id],
        |row| row.get(0),
    )?;
    let total_count = u64::try_from(total_count).map_err(|_| StoreError::CorruptStoredValue {
        database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
        field: "project_continuity_records.total_count",
    })?;
    let mut records = {
        let mut stmt = transaction.prepare(
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
               AND (
                    ?2 IS NULL
                    OR volicord_utc_seconds(updated_at) < volicord_utc_seconds(?2)
                    OR (
                        volicord_utc_seconds(updated_at) = volicord_utc_seconds(?2)
                        AND volicord_utc_subsec_nanos(updated_at)
                            < volicord_utc_subsec_nanos(?2)
                    )
                    OR (
                        volicord_utc_seconds(updated_at) = volicord_utc_seconds(?2)
                        AND volicord_utc_subsec_nanos(updated_at)
                            = volicord_utc_subsec_nanos(?2)
                        AND continuity_record_id < ?3
                    )
               )
             ORDER BY volicord_utc_seconds(updated_at) DESC,
                      volicord_utc_subsec_nanos(updated_at) DESC,
                      continuity_record_id DESC
             LIMIT ?4",
        )?;
        let rows = stmt.query_map(
            params![
                project_id,
                cursor_updated_at.as_deref(),
                cursor_record_id,
                fetch_limit
            ],
            project_continuity_record_from_row,
        )?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        records
    };
    transaction.commit()?;

    let truncated = records.len() > page_size;
    if truncated {
        records.truncate(page_size);
    }
    Ok(ActiveProjectContinuityPage {
        records,
        total_count,
        truncated,
    })
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
         ORDER BY volicord_utc_seconds(created_at),
                  volicord_utc_subsec_nanos(created_at),
                  continuity_record_id",
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

pub(crate) fn effective_user_action_record(
    conn: &Connection,
    project_id: &str,
    user_action_request_id: &str,
    now: &UtcTimestamp,
) -> StoreResult<Option<EffectiveUserActionRecord>> {
    let Some(request) = user_action_request_record(conn, project_id, user_action_request_id)?
    else {
        return Ok(None);
    };
    let resolution =
        user_action_resolution_record_by_request(conn, project_id, user_action_request_id)?;
    if let Some(resolution) = &resolution {
        validate_user_action_request_resolution_pair(&request, resolution)?;
    }
    let status = effective_user_action_status(&request, resolution.as_ref(), now)?;
    Ok(Some(EffectiveUserActionRecord {
        request,
        resolution,
        status,
    }))
}

fn effective_user_action_records_for_task(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    status_filter: Option<UserActionStatus>,
    now: &UtcTimestamp,
) -> StoreResult<Vec<EffectiveUserActionRecord>> {
    let mut stmt = conn.prepare(
        "SELECT user_action_request_id
           FROM user_action_requests
          WHERE project_id = ?1
            AND task_id = ?2
          ORDER BY volicord_utc_seconds(requested_at),
                   volicord_utc_subsec_nanos(requested_at),
                   user_action_request_id",
    )?;
    let rows = stmt.query_map(params![project_id, task_id], |row| row.get::<_, String>(0))?;
    let mut records = Vec::new();
    for row in rows {
        let request_id = row?;
        let record =
            effective_user_action_record(conn, project_id, &request_id, now)?.ok_or_else(|| {
                StoreError::SchemaInvariant {
                    database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
                    detail: format!("user action request {request_id} disappeared during read"),
                }
            })?;
        if status_filter.is_none_or(|expected| record.status == expected) {
            records.push(record);
        }
    }
    Ok(records)
}

fn effective_user_action_records_for_task_and_kind(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    action_kind: UserActionKind,
    status_filter: UserActionStatus,
    now: &UtcTimestamp,
) -> StoreResult<Vec<EffectiveUserActionRecord>> {
    Ok(
        effective_user_action_records_for_task(conn, project_id, task_id, None, now)?
            .into_iter()
            .filter(|record| {
                record.request.action_kind == action_kind && record.status == status_filter
            })
            .collect(),
    )
}

/// Derives the current lifecycle status from immutable resolution presence, basis status, and time.
pub fn effective_user_action_status(
    request: &UserActionRequestRecord,
    resolution: Option<&UserActionResolutionRecord>,
    now: &UtcTimestamp,
) -> StoreResult<UserActionStatus> {
    let created_at = UtcTimestamp::parse(&request.requested_at).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            &request.user_action_request_id,
            "requested_at",
        )
    })?;
    let expires_at = request
        .expires_at
        .as_deref()
        .map(UtcTimestamp::parse)
        .transpose()
        .map_err(|_| StoreError::CorruptStoredValue {
            database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
            field: "user_action_requests.expires_at",
        })?;
    if let Some(resolution) = resolution {
        let resolved_at = UtcTimestamp::parse(&resolution.resolved_at).map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "user_action_resolutions",
                &resolution.user_action_resolution_id,
                "resolved_at",
            )
        })?;
        if &resolved_at > now {
            return Err(StoreError::corrupt_owner_state_value(
                "user_action_resolutions",
                &resolution.user_action_resolution_id,
                "resolved_at",
            ));
        }
    }
    derive_user_action_status(
        request.basis_status,
        &created_at,
        expires_at.as_ref(),
        resolution.is_some(),
        now,
    )
    .ok_or_else(|| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            &request.user_action_request_id,
            "requested_at",
        )
    })
}

fn validate_user_action_request_resolution_pair(
    request: &UserActionRequestRecord,
    resolution: &UserActionResolutionRecord,
) -> StoreResult<()> {
    if request.project_id != resolution.project_id
        || request.user_action_request_id != resolution.user_action_request_id
        || request.action_kind != resolution.action_kind
    {
        return Err(StoreError::SchemaInvariant {
            database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
            detail: "user-action resolution does not match its request identity and kind"
                .to_owned(),
        });
    }
    validate_stored_user_action_timestamp_order(request, resolution)?;
    let persisted_request = serde_json::from_str::<PersistedUserActionRequest>(
        &request.request_json,
    )
    .map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            &request.user_action_request_id,
            "request_json",
        )
    })?;
    let basis = serde_json::from_str::<UserActionBasis>(&request.basis_json).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            &request.user_action_request_id,
            "basis_json",
        )
    })?;
    let resolution_body = serde_json::from_str::<UserActionResolutionBody>(
        &resolution.resolution_json,
    )
    .map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            &resolution.user_action_resolution_id,
            "resolution_json",
        )
    })?;

    let agrees = match (&persisted_request.body, &basis, &resolution_body) {
        (
            UserActionRequestBody::Choice(choice),
            UserActionBasis::Choice(choice_basis),
            UserActionResolutionBody::Choice {
                selected_option_id,
                machine_action,
                resolution_outcome,
                accepted_risk_ids,
                ..
            },
        ) => choice
            .options
            .iter()
            .find(|option| option.option_id == *selected_option_id)
            .is_some_and(|option| {
                let expected_risk_ids = if request.action_kind
                    == UserActionKind::ResidualRiskAcceptance
                    && option.machine_action == UserActionOptionAction::Accept
                {
                    choice_basis.residual_risk_ids.as_slice()
                } else {
                    &[]
                };
                option.machine_action == *machine_action
                    && option.resolution_outcome == *resolution_outcome
                    && accepted_risk_ids == expected_risk_ids
            }),
        (
            UserActionRequestBody::EvidenceObservation(observation_request),
            UserActionBasis::EvidenceObservation(observation_basis),
            UserActionResolutionBody::EvidenceObservation { observation },
        ) => {
            let unique_artifact_ids = observation
                .output_artifact_refs
                .iter()
                .map(|artifact| &artifact.artifact_id)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == observation.output_artifact_refs.len();
            observation_request.target_candidates == observation_basis.target_candidates
                && observation_request.artifact_candidates == observation_basis.artifact_candidates
                && observation_request
                    .target_candidates
                    .contains(&observation.target)
                && observation.output_artifact_refs.iter().all(|selected| {
                    observation_request
                        .artifact_candidates
                        .iter()
                        .any(|candidate| user_action_artifact_ref_agrees(candidate, selected))
                })
                && unique_artifact_ids
                && matches!(
                    observation.relevance_status,
                    volicord_types::EvidenceRelevanceStatus::Supported
                        | volicord_types::EvidenceRelevanceStatus::Contradicted
                )
                && !observation.summary.trim().is_empty()
        }
        _ => false,
    };
    if !agrees {
        return Err(StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            &resolution.user_action_resolution_id,
            "resolution_json",
        ));
    }
    Ok(())
}

fn validate_stored_user_action_timestamp_order(
    request: &UserActionRequestRecord,
    resolution: &UserActionResolutionRecord,
) -> StoreResult<()> {
    let requested_at = UtcTimestamp::parse(&request.requested_at).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_requests",
            &request.user_action_request_id,
            "requested_at",
        )
    })?;
    let expires_at = request
        .expires_at
        .as_deref()
        .map(UtcTimestamp::parse)
        .transpose()
        .map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "user_action_requests",
                &request.user_action_request_id,
                "expires_at",
            )
        })?;
    let resolved_at = UtcTimestamp::parse(&resolution.resolved_at).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            &resolution.user_action_resolution_id,
            "resolved_at",
        )
    })?;
    match validate_user_action_timestamp_order(
        &requested_at,
        expires_at.as_ref(),
        Some(&resolved_at),
    ) {
        Ok(()) => Ok(()),
        Err(UserActionTimestampOrderFailure::ExpiryNotAfterRequest) => {
            Err(StoreError::corrupt_owner_state_value(
                "user_action_requests",
                &request.user_action_request_id,
                "expires_at",
            ))
        }
        Err(
            UserActionTimestampOrderFailure::ResolutionBeforeRequest
            | UserActionTimestampOrderFailure::ResolutionAtOrAfterExpiry,
        ) => Err(StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            &resolution.user_action_resolution_id,
            "resolved_at",
        )),
    }
}

fn user_action_artifact_ref_agrees(candidate: &ArtifactRef, selected: &ArtifactRef) -> bool {
    candidate == selected
}

fn validate_resolution_with_stored_request(
    conn: &Connection,
    project_id: &str,
    resolution: Option<UserActionResolutionRecord>,
) -> StoreResult<Option<UserActionResolutionRecord>> {
    let Some(resolution) = resolution else {
        return Ok(None);
    };
    let request = user_action_request_record(conn, project_id, &resolution.user_action_request_id)?
        .ok_or_else(|| StoreError::SchemaInvariant {
            database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
            detail: "user-action resolution has no matching request".to_owned(),
        })?;
    validate_user_action_request_resolution_pair(&request, &resolution)?;
    Ok(Some(resolution))
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

fn non_current_user_action_refs(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    action_kind: UserActionKind,
    state_version: u64,
    now: &UtcTimestamp,
) -> StoreResult<Vec<StoredRecordRef>> {
    Ok(
        effective_user_action_records_for_task(conn, project_id, task_id, None, now)?
            .into_iter()
            .filter(|record| {
                record.request.action_kind == action_kind
                    && matches!(
                        record.status,
                        UserActionStatus::Stale | UserActionStatus::Superseded
                    )
            })
            .map(|record| StoredRecordRef {
                record_kind: "user_action_request".to_owned(),
                record_id: record.request.user_action_request_id,
                project_id: project_id.to_owned(),
                task_id: Some(task_id.to_owned()),
                state_version: Some(state_version),
            })
            .collect(),
    )
}

fn effective_user_action_refs(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    status: UserActionStatus,
    state_version: u64,
    now: &UtcTimestamp,
) -> StoreResult<Vec<StoredRecordRef>> {
    Ok(
        effective_user_action_records_for_task(conn, project_id, task_id, Some(status), now)?
            .into_iter()
            .map(|record| StoredRecordRef {
                record_kind: "user_action_request".to_owned(),
                record_id: record.request.user_action_request_id,
                project_id: project_id.to_owned(),
                task_id: Some(task_id.to_owned()),
                state_version: Some(state_version),
            })
            .collect(),
    )
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
    let state = tx
        .query_row(
            "SELECT
            project_id,
            state_version,
            active_task_id,
            updated_at
         FROM project_state
         WHERE project_id = ?1",
            params![project_id],
            project_state_from_row,
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project_state",
            id: project_id.to_owned(),
        })?;
    validate_project_state_updated_at(&state)?;
    Ok(state)
}

fn project_state_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectStateHeader> {
    let state_version = row.get::<_, i64>(1)?;
    Ok(ProjectStateHeader {
        project_id: row.get(0)?,
        state_version: nonnegative_i64_to_u64("project_state.state_version", state_version)?,
        active_task_id: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

fn validate_project_state_updated_at(state: &ProjectStateHeader) -> StoreResult<()> {
    UtcTimestamp::parse(&state.updated_at)
        .and_then(|timestamp| {
            timestamp
                .ensure_canonical_rfc3339_representable()
                .map_err(|_| volicord_types::UtcTimestampParseError)
        })
        .map_err(|_| {
            StoreError::corrupt_owner_state_value("project_state", &state.project_id, "updated_at")
        })
}

pub(crate) fn project_current_utc_timestamp_for_conn(
    conn: &Connection,
    project_id: &str,
    local_floor: Option<&UtcTimestamp>,
) -> StoreResult<UtcTimestamp> {
    let (sqlite_now, persisted_floor): (String, String) = conn
        .query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), updated_at
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
    let sqlite_now = UtcTimestamp::parse(&sqlite_now).map_err(|_| StoreError::SchemaInvariant {
        database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
        detail: "SQLite returned an invalid Core current UTC timestamp".to_owned(),
    })?;
    sqlite_now
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::SchemaInvariant {
            database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
            detail: "SQLite returned an out-of-range Core current UTC timestamp".to_owned(),
        })?;
    let persisted_floor = UtcTimestamp::parse(&persisted_floor).map_err(|_| {
        StoreError::corrupt_owner_state_value("project_state", project_id, "updated_at")
    })?;
    persisted_floor
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| {
            StoreError::corrupt_owner_state_value("project_state", project_id, "updated_at")
        })?;
    if let Some(local_floor) = local_floor {
        local_floor
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| StoreError::SchemaInvariant {
                database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
                detail: "Core local UTC floor is outside the canonical timestamp range".to_owned(),
            })?;
    }
    Ok([
        Some(sqlite_now),
        Some(persisted_floor),
        local_floor.cloned(),
    ]
    .into_iter()
    .flatten()
    .max()
    .expect("Core current UTC clock always has SQLite and persisted samples"))
}

pub(crate) fn advance_project_utc_floor_tx(
    tx: &Transaction<'_>,
    project_id: &str,
    sample: &UtcTimestamp,
) -> StoreResult<UtcTimestamp> {
    sample
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::SchemaInvariant {
            database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
            detail: "Core UTC floor sample is outside the canonical timestamp range".to_owned(),
        })?;
    let persisted_floor = tx
        .query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            params![project_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project_state",
            id: project_id.to_owned(),
        })?;
    let persisted_floor = UtcTimestamp::parse(&persisted_floor).map_err(|_| {
        StoreError::corrupt_owner_state_value("project_state", project_id, "updated_at")
    })?;
    persisted_floor
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| {
            StoreError::corrupt_owner_state_value("project_state", project_id, "updated_at")
        })?;
    let floor = std::cmp::max(persisted_floor, sample.clone());
    let changed = tx.execute(
        "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
        params![project_id, floor.to_string()],
    )?;
    if changed != 1 {
        return Err(StoreError::SchemaInvariant {
            database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
            detail: "Core current UTC floor update changed no rows".to_owned(),
        });
    }
    Ok(floor)
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
    use sha2::{Digest, Sha256};
    use volicord_test_support::TempRuntimeHome;
    use volicord_types::{
        IdempotencyKey, JudgmentResolutionOutcome, MethodName, ProjectContinuityRecordId,
        ProjectId, RecordId, RequestHash, RequiredNullable, StateRecordKind, StateRecordRef,
        TaskId, UserActionBasis, UserActionOptionAction,
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
    fn decoded_change_unit_requires_a_basis_state_version() {
        let error = validate_decoded_change_unit_record(RawChangeUnitRecord {
            project_id: PROJECT_ID.to_owned(),
            change_unit_id: "cu_missing_basis".to_owned(),
            task_id: "task_missing_basis".to_owned(),
            status: "active".to_owned(),
            is_current: 1,
            basis_state_version: None,
            scope_summary_json: "{}".to_owned(),
            bounded_paths_json: "[]".to_owned(),
            write_basis_json: "{}".to_owned(),
            effect_contract_json: "null".to_owned(),
            lifecycle_json: "{}".to_owned(),
        })
        .expect_err("a persisted Change Unit without its basis must be corrupt");

        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "change_units",
                logical_column: "basis_state_version",
                ..
            }
        ));
    }

    #[test]
    fn task_close_summary_requires_an_explicit_close_reason_on_write_and_read(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let before = store.effect_counts()?;
        let mut invalid = task_insert("task_missing_close_reason_write");
        invalid.close_summary_json = "{}".to_owned();
        let write = store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::Intake,
                Some(&IdempotencyKey::new("idem_missing_close_reason_write")),
                &RequestHash::new("sha256:missing-close-reason-write"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task(
                    "missing_close_reason_write",
                    "task_missing_close_reason_write",
                )],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(invalid)
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        );
        assert!(matches!(write, Err(StoreError::InvalidInput { .. })));
        assert_eq!(store.effect_counts()?, before);

        let task_id = "task_missing_close_reason_read";
        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::Intake,
                Some(&IdempotencyKey::new("idem_missing_close_reason_read")),
                &RequestHash::new("sha256:missing-close-reason-read"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("missing_close_reason_read", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        store.conn.execute(
            "UPDATE tasks SET close_summary_json = '{}' WHERE project_id = ?1 AND task_id = ?2",
            params![PROJECT_ID, task_id],
        )?;
        let read = store.task_record(&TaskId::new(task_id));
        assert!(matches!(
            read,
            Err(StoreError::CorruptOwnerStateJson {
                table: "tasks",
                logical_column: "close_summary_json",
                ..
            })
        ));
        Ok(())
    }

    #[test]
    fn default_commit_clock_includes_transaction_live_storage_time() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let configured_floor = "2000-01-01T00:00:00Z";
        store.conn.execute(
            "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
            params![PROJECT_ID, configured_floor],
        )?;
        let sqlite_before: String =
            store
                .conn
                .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
                    row.get(0)
                })?;
        let task_id = "task_live_commit_clock";
        let mut input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new("idem_live_commit_clock")),
            &RequestHash::new("sha256:live-commit-clock"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("live_commit_clock", task_id)],
        );
        input.clock_floor = Some(configured_floor.to_owned());

        let outcome = store.commit_mutation(
            input,
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        assert!(matches!(outcome, MutationCommitOutcome::Committed { .. }));
        let committed_at = UtcTimestamp::parse(&store.project_state()?.updated_at)?;
        assert!(committed_at >= UtcTimestamp::parse(&sqlite_before)?);
        assert!(committed_at > UtcTimestamp::parse(configured_floor)?);
        Ok(())
    }

    #[test]
    fn canonical_clock_helpers_reject_corrupt_floor_and_extreme_sample_without_effect(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let store = harness.store()?;
        let before = store.effect_counts()?;
        let original_floor = store.project_state()?.updated_at;
        let out_of_range = "9999-12-31T23:59:59-23:59";
        store.conn.execute(
            "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
            params![PROJECT_ID, out_of_range],
        )?;

        assert!(matches!(
            store.current_timestamp(),
            Err(StoreError::CorruptOwnerStateValue { .. })
        ));
        let persisted: String = store.conn.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(persisted, out_of_range);

        store.conn.execute(
            "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
            params![PROJECT_ID, original_floor],
        )?;
        assert_eq!(store.effect_counts()?, before);
        drop(store);
        let mut conn = open_project_state_database(
            harness
                .runtime_home_path
                .join("projects")
                .join(PROJECT_ID)
                .join("state.sqlite"),
        )?;
        let tx = conn.transaction()?;
        let extreme = UtcTimestamp::from_datetime(chrono::DateTime::<chrono::Utc>::MAX_UTC);
        assert!(matches!(
            advance_project_utc_floor_tx(&tx, PROJECT_ID, &extreme),
            Err(StoreError::SchemaInvariant { .. })
        ));
        drop(tx);
        let after_floor: String = conn.query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(after_floor, original_floor);
        drop(conn);
        assert_eq!(harness.store()?.effect_counts()?, before);
        Ok(())
    }

    #[test]
    fn latest_evidence_summary_uses_state_version_when_time_and_ids_disagree(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_evidence_summary_authority_order";
        let fixed_time = "2999-07-13T12:34:56.789123456Z";

        let mut first_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RecordRun,
            Some(&IdempotencyKey::new("idem_summary_authority_old")),
            &RequestHash::new("sha256:summary-authority-old"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("summary_authority_old", task_id)],
        );
        first_input.clock_floor = Some(fixed_time.to_owned());
        first_input.include_live_storage_time = false;
        store.commit_mutation(
            first_input,
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::UpsertEvidenceSummary(evidence_summary_upsert(
                    "summary_z_old",
                    task_id,
                    "run_summary_old",
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        let mut second_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RecordRun,
            Some(&IdempotencyKey::new("idem_summary_authority_new")),
            &RequestHash::new("sha256:summary-authority-new"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(1),
            vec![pending_event_for_task("summary_authority_new", task_id)],
        );
        second_input.clock_floor = Some(fixed_time.to_owned());
        second_input.include_live_storage_time = false;
        store.commit_mutation(
            second_input,
            |mutation, facts| {
                CoreStorageMutation::UpsertEvidenceSummary(evidence_summary_upsert(
                    "summary_a_new",
                    task_id,
                    "run_summary_new",
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        let latest = store
            .latest_evidence_summary(&TaskId::new(task_id))?
            .expect("latest evidence summary should exist");
        assert_eq!(latest.evidence_summary_id, "summary_a_new");
        assert_eq!(latest.produced_at_state_version, 2);
        let timestamps = store
            .conn
            .prepare(
                "SELECT created_at
                   FROM evidence_summaries
                  WHERE project_id = ?1 AND task_id = ?2
                  ORDER BY evidence_summary_id",
            )?
            .query_map(params![PROJECT_ID, task_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(
            timestamps,
            vec![fixed_time.to_owned(), fixed_time.to_owned()]
        );
        assert_eq!(store.project_state()?.updated_at, fixed_time);

        let before_counts = store.effect_counts()?;
        let before_state = store.project_state()?;
        let mut duplicate_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RecordRun,
            Some(&IdempotencyKey::new("idem_summary_authority_duplicate")),
            &RequestHash::new("sha256:summary-authority-duplicate"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(2),
            vec![pending_event_for_task(
                "summary_authority_duplicate",
                task_id,
            )],
        );
        duplicate_input.clock_floor = Some(fixed_time.to_owned());
        duplicate_input.include_live_storage_time = false;
        let error = store
            .commit_mutation(
                duplicate_input,
                |mutation, facts| {
                    for summary_id in ["summary_duplicate_first", "summary_duplicate_second"] {
                        CoreStorageMutation::UpsertEvidenceSummary(evidence_summary_upsert(
                            summary_id,
                            task_id,
                            "run_summary_duplicate",
                        ))
                        .apply(mutation, facts.committed_state_version)?;
                    }
                    Ok(())
                },
                response_json,
            )
            .expect_err("one Task cannot have two summaries produced by one commit");
        assert!(matches!(error, StoreError::Sqlite(_)));
        assert_eq!(store.effect_counts()?, before_counts);
        assert_eq!(store.project_state()?, before_state);
        assert_eq!(
            store
                .latest_evidence_summary(&TaskId::new(task_id))?
                .expect("rolled-back duplicate must preserve current summary")
                .evidence_summary_id,
            "summary_a_new"
        );
        Ok(())
    }

    #[test]
    fn prepared_artifact_eligibility_uses_exact_submillisecond_expiry() -> Result<(), Box<dyn Error>>
    {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_staged_exact_expiry";
        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::Intake,
                Some(&IdempotencyKey::new("idem_staged_exact_expiry")),
                &RequestHash::new("sha256:staged-exact-expiry"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("staged_exact_expiry", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        store.conn.execute(
            "INSERT INTO artifact_staging (
                project_id, handle_id, task_id, created_by_actor_source,
                redaction_state, status, expires_at, created_at
             ) VALUES (
                ?1, 'stage_exact_expiry', ?2, ?3,
                'none', 'staged', '2026-07-13T00:10:00.000000501Z',
                '2026-07-13T00:00:00Z'
             )",
            params![PROJECT_ID, task_id, ACTOR_SOURCE],
        )?;
        let now = UtcTimestamp::parse("2026-07-13T00:10:00.000000500Z")?;
        let before_state = store.project_state()?;

        assert!(store.has_prepared_artifact_input(&TaskId::new(task_id), &now)?);
        store.conn.execute(
            "UPDATE artifact_staging
                SET expires_at = '2026-07-13T00:10:00.000000500Z'
              WHERE project_id = ?1 AND handle_id = 'stage_exact_expiry'",
            [PROJECT_ID],
        )?;
        assert!(!store.has_prepared_artifact_input(&TaskId::new(task_id), &now)?);
        store.conn.execute(
            "UPDATE artifact_staging
                SET expires_at = '2026-07-13T00:10:00.000000499Z'
              WHERE project_id = ?1 AND handle_id = 'stage_exact_expiry'",
            [PROJECT_ID],
        )?;
        assert!(!store.has_prepared_artifact_input(&TaskId::new(task_id), &now)?);
        assert_eq!(store.project_state()?, before_state);
        Ok(())
    }

    #[test]
    fn explicit_future_clock_floor_survives_active_task_commit_and_reopen(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_clock_floor";
        let first = store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::Intake,
                Some(&IdempotencyKey::new("idem_clock_floor_task")),
                &RequestHash::new("sha256:clock-floor-task"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("clock_floor_task", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert!(matches!(first, MutationCommitOutcome::Committed { .. }));

        let future_floor = UtcTimestamp::parse("2999-07-13T12:34:56.789Z")?;
        let future_task_id = "task_clock_floor_future";
        let mut clock_floor_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new("idem_clock_floor_activate")),
            &RequestHash::new("sha256:clock-floor-activate"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(1),
            vec![
                pending_event_for_task("clock_floor_activate", future_task_id),
                pending_event_for_task("clock_floor_activate_second", future_task_id),
            ],
        );
        clock_floor_input.clock_floor = Some(future_floor.to_string());
        let second = store.commit_mutation(
            clock_floor_input,
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(future_task_id))
                    .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::EnsureEvidenceClaim(EvidenceClaimInsert {
                    evidence_claim_id: "claim_clock_floor".to_owned(),
                    task_id: future_task_id.to_owned(),
                    statement: "The canonical commit clock is shared.".to_owned(),
                })
                .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::CloseTask(TaskCloseUpdate {
                    task_id: task_id.to_owned(),
                    lifecycle_phase: "completed".to_owned(),
                    result: "completed".to_owned(),
                    close_summary_json: "{\"close_reason\":\"completed_self_checked\"}".to_owned(),
                    closed_at: "2999-07-13T12:00:00Z".to_owned(),
                })
                .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::SetActiveTask {
                    task_id: future_task_id.to_owned(),
                }
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert!(matches!(second, MutationCommitOutcome::Committed { .. }));

        let expected = future_floor.to_string();
        let state = store.project_state()?;
        assert_eq!(state.active_task_id.as_deref(), Some(future_task_id));
        assert_eq!(state.updated_at, expected);
        let (task_created_at, task_updated_at) = store.conn.query_row(
            "SELECT created_at, updated_at
               FROM tasks
              WHERE project_id = ?1 AND task_id = ?2",
            params![PROJECT_ID, future_task_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?;
        assert_eq!(task_created_at, expected);
        assert_eq!(task_updated_at, expected);
        let (closed_at, closed_task_updated_at) = store.conn.query_row(
            "SELECT closed_at, updated_at
               FROM tasks
              WHERE project_id = ?1 AND task_id = ?2",
            params![PROJECT_ID, task_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?;
        assert_eq!(closed_at, "2999-07-13T12:00:00Z");
        assert_eq!(closed_task_updated_at, expected);
        let claim_created_at = store.conn.query_row(
            "SELECT created_at
               FROM evidence_claims
              WHERE project_id = ?1 AND evidence_claim_id = 'claim_clock_floor'",
            [PROJECT_ID],
            |row| row.get::<_, String>(0),
        )?;
        assert_eq!(claim_created_at, expected);
        let event_created_at = store.conn.query_row(
            "SELECT created_at FROM authority_events
              WHERE project_id = ?1 AND event_id = 'evt_clock_floor_activate'",
            [PROJECT_ID],
            |row| row.get::<_, String>(0),
        )?;
        let invocation_created_at = store.conn.query_row(
            "SELECT created_at FROM tool_invocations
              WHERE project_id = ?1 AND idempotency_key = 'idem_clock_floor_activate'",
            [PROJECT_ID],
            |row| row.get::<_, String>(0),
        )?;
        assert_eq!(event_created_at, expected);
        assert_eq!(invocation_created_at, expected);
        let (event_count, distinct_event_timestamps) = store.conn.query_row(
            "SELECT COUNT(*), COUNT(DISTINCT created_at)
               FROM authority_events
              WHERE project_id = ?1
                AND event_id IN ('evt_clock_floor_activate', 'evt_clock_floor_activate_second')",
            [PROJECT_ID],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;
        assert_eq!(event_count, 2);
        assert_eq!(distinct_event_timestamps, 1);

        let before_noncommitting = store.effect_counts()?;
        let future_attempt_floor = "4000-01-01T00:00:00Z";
        let mut replay_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new("idem_clock_floor_activate")),
            &RequestHash::new("sha256:clock-floor-activate"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(1),
            vec![
                pending_event_for_task("clock_floor_activate", future_task_id),
                pending_event_for_task("clock_floor_activate_second", future_task_id),
            ],
        );
        replay_input.clock_floor = Some(future_attempt_floor.to_owned());
        let replay = store.commit_mutation(
            replay_input,
            |_, _| panic!("replay must not invoke the mutation closure"),
            response_json,
        )?;
        assert!(matches!(replay, MutationCommitOutcome::Replayed { .. }));
        assert_eq!(store.project_state()?.updated_at, expected);
        assert_eq!(store.effect_counts()?, before_noncommitting);

        let mut stale_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new("idem_clock_floor_stale")),
            &RequestHash::new("sha256:clock-floor-stale"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task("clock_floor_stale", future_task_id)],
        );
        stale_input.clock_floor = Some(future_attempt_floor.to_owned());
        let stale = store.commit_mutation(
            stale_input,
            |_, _| panic!("stale expected state must not invoke the mutation closure"),
            response_json,
        )?;
        assert!(matches!(
            stale,
            MutationCommitOutcome::StaleExpectedState { .. }
        ));
        assert_eq!(store.project_state()?.updated_at, expected);
        assert_eq!(store.effect_counts()?, before_noncommitting);

        let before_invalid = store.effect_counts()?;
        let mut invalid_floor = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new("idem_invalid_clock_floor")),
            &RequestHash::new("sha256:invalid-clock-floor"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(2),
            vec![pending_event_for_task("invalid_clock_floor", task_id)],
        );
        invalid_floor.clock_floor = Some("not-a-timestamp".to_owned());
        let error = store
            .commit_mutation(invalid_floor, |_, _| Ok(()), response_json)
            .expect_err("invalid explicit clock floor must fail before effects");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert_eq!(store.effect_counts()?, before_invalid);

        let remembered_floor = UtcTimestamp::parse("3000-01-01T00:00:00Z")?;
        store.remember_clock_sample(&remembered_floor);
        assert!(UtcTimestamp::parse(&store.current_timestamp()?)? >= remembered_floor);
        drop(store);
        let reopened = harness.store()?;
        assert_eq!(reopened.current_timestamp()?, expected);
        Ok(())
    }

    #[test]
    fn unrepresentable_remembered_clock_sample_rejects_commit_without_effect(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let before_state = store.project_state()?;
        let before_effects = store.effect_counts()?;
        let unrepresentable = UtcTimestamp::parse("9999-12-31T23:59:59-23:59")?;
        assert!(unrepresentable
            .ensure_canonical_rfc3339_representable()
            .is_err());
        store.remember_clock_sample(&unrepresentable);

        let mut input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::Intake,
            Some(&IdempotencyKey::new(
                "idem_unrepresentable_remembered_clock",
            )),
            &RequestHash::new("sha256:unrepresentable-remembered-clock"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event_for_task(
                "unrepresentable_remembered_clock",
                "task_unrepresentable_remembered_clock",
            )],
        );
        input.include_live_storage_time = false;

        let error = store
            .commit_mutation(
                input,
                |_, _| panic!("invalid remembered sample must fail before mutation"),
                response_json,
            )
            .expect_err("unrepresentable remembered sample must fail closed");
        assert!(matches!(error, StoreError::SchemaInvariant { .. }));
        assert_eq!(store.project_state()?, before_state);
        assert_eq!(store.effect_counts()?, before_effects);
        Ok(())
    }

    #[test]
    fn semantic_timestamp_inputs_reject_atomically_before_durable_rows(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_invalid_semantic_timestamp";
        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::Intake,
                Some(&IdempotencyKey::new("idem_invalid_timestamp_setup")),
                &RequestHash::new("sha256:invalid-timestamp-setup"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("invalid_timestamp_setup", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        let before = store.effect_counts()?;

        let close = store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::CloseTask,
                Some(&IdempotencyKey::new("idem_invalid_closed_at")),
                &RequestHash::new("sha256:invalid-closed-at"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(1),
                vec![pending_event_for_task("invalid_closed_at", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::CloseTask(TaskCloseUpdate {
                    task_id: task_id.to_owned(),
                    lifecycle_phase: "completed".to_owned(),
                    result: "completed".to_owned(),
                    close_summary_json: "{\"close_reason\":\"completed_self_checked\"}".to_owned(),
                    closed_at: "tomorrow".to_owned(),
                })
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        );
        assert!(matches!(close, Err(StoreError::InvalidInput { .. })));
        assert_eq!(store.effect_counts()?, before);

        let write_ticket = store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::PrepareWrite,
                Some(&IdempotencyKey::new("idem_invalid_write_ticket_expiry")),
                &RequestHash::new("sha256:invalid-write-ticket-expiry"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(1),
                vec![pending_event_for_task(
                    "invalid_write_ticket_expiry",
                    task_id,
                )],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertWriteTicket(WriteTicketInsert {
                    write_ticket_id: "write_ticket_invalid_expiry".to_owned(),
                    task_id: task_id.to_owned(),
                    change_unit_id: "change_unit_missing".to_owned(),
                    validity_basis_json: "{}".to_owned(),
                    allowed_path_prefixes_json: "[]".to_owned(),
                    denied_path_prefixes_json: "[]".to_owned(),
                    attempt_scope_json: "{}".to_owned(),
                    created_by_actor_source: ACTOR_SOURCE.to_owned(),
                    created_by_user_action_resolution_id: None,
                    idle_expires_at: Some("tomorrow".to_owned()),
                    created_at: "2026-07-13T00:00:00Z".to_owned(),
                    metadata_json: "{}".to_owned(),
                })
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        );
        assert!(matches!(write_ticket, Err(StoreError::InvalidInput { .. })));
        assert_eq!(store.effect_counts()?, before);
        Ok(())
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
    fn transaction_replay_rejects_changed_git_workspace_context() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let mut first_context = replay_context(CONNECTION_ID, "agent_workflow");
        first_context.git_workspace_context_json =
            Some(volicord_types::canonical_json_string(&json!({
                "git_common_dir": "/tmp/repo/.git",
                "worktree_id": format!("sha256:{}", "1".repeat(64)),
                "branch_ref": "refs/heads/original",
                "head_sha": "1111111111111111111111111111111111111111",
                "workspace_fingerprint": format!("sha256:{}", "2".repeat(64))
            }))?);
        let first_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_workspace_context")),
            &RequestHash::new("sha256:same-request"),
            Some(first_context.clone()),
            Some(0),
            vec![pending_event("workspace_first")],
        );
        let first = store.commit_mutation(
            first_input,
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert("task_workspace_first"))
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert!(matches!(first, MutationCommitOutcome::Committed { .. }));
        let before = store.effect_counts()?;

        let mut changed_basis = first_context.clone();
        changed_basis.verification_basis = Some("different_verified_channel".to_owned());
        let basis_replay_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_workspace_context")),
            &RequestHash::new("sha256:same-request"),
            Some(changed_basis),
            Some(1),
            vec![pending_event("basis_second")],
        );
        let basis_replay =
            store.commit_mutation(basis_replay_input, |_, _| Ok(()), response_json)?;
        assert!(matches!(
            basis_replay,
            MutationCommitOutcome::ReplayContextMismatch { .. }
        ));
        assert_eq!(store.effect_counts()?, before);

        let mut changed_context = first_context;
        changed_context.git_workspace_context_json =
            Some(volicord_types::canonical_json_string(&json!({
                "git_common_dir": "/tmp/repo/.git",
                "worktree_id": format!("sha256:{}", "3".repeat(64)),
                "branch_ref": "refs/heads/other",
                "head_sha": "2222222222222222222222222222222222222222",
                "workspace_fingerprint": format!("sha256:{}", "4".repeat(64))
            }))?);
        let replay_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&IdempotencyKey::new("idem_store_workspace_context")),
            &RequestHash::new("sha256:same-request"),
            Some(changed_context),
            Some(1),
            vec![pending_event("workspace_second")],
        );
        let replay = store.commit_mutation(replay_input, |_, _| Ok(()), response_json)?;

        assert!(matches!(
            replay,
            MutationCommitOutcome::ReplayContextMismatch { .. }
        ));
        assert_eq!(store.effect_counts()?, before);
        Ok(())
    }

    #[test]
    fn malformed_stored_git_workspace_replay_context_is_corruption() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let mut context = replay_context(CONNECTION_ID, "agent_workflow");
        context.git_workspace_context_json = Some(volicord_types::canonical_json_string(&json!({
            "git_common_dir": "/tmp/repo/.git",
            "worktree_id": format!("sha256:{}", "1".repeat(64)),
            "branch_ref": "refs/heads/original",
            "head_sha": "1111111111111111111111111111111111111111",
            "workspace_fingerprint": format!("sha256:{}", "2".repeat(64))
        }))?);
        let idempotency_key = IdempotencyKey::new("idem_store_workspace_corrupt");
        let first = store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::UpdateScope,
                Some(&idempotency_key),
                &RequestHash::new("sha256:workspace-corrupt"),
                Some(context),
                Some(0),
                vec![pending_event("workspace_corrupt")],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert("task_workspace_corrupt"))
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert!(matches!(first, MutationCommitOutcome::Committed { .. }));
        drop(store);

        let conn = open_project_state_database(
            harness
                .runtime_home_path
                .join("projects")
                .join(PROJECT_ID)
                .join("state.sqlite"),
        )?;
        conn.execute(
            "UPDATE tool_invocations
                SET git_workspace_context_json = '{\"unexpected\":true}'
              WHERE project_id = ?1
                AND tool_name = ?2
                AND idempotency_key = ?3",
            params![
                PROJECT_ID,
                MethodName::UpdateScope.as_str(),
                idempotency_key.as_str()
            ],
        )?;
        drop(conn);

        let store = harness.store()?;
        let error = store
            .tool_invocation(MethodName::UpdateScope, &idempotency_key)
            .expect_err("malformed replay workspace context must be corrupt owner state");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateJson {
                table: "tool_invocations",
                logical_column: "git_workspace_context_json",
                ..
            }
        ));
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
    fn operation_result_reuses_exact_replay_bytes_and_metadata() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let idempotency_key = IdempotencyKey::new("idem_store_operation_result");
        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&idempotency_key),
            &RequestHash::new("sha256:operation-result"),
            Some(replay_context(CONNECTION_ID, "agent_workflow")),
            Some(0),
            vec![pending_event("operation_result")],
        );
        let committed = store.commit_mutation(
            input,
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert("task_operation_result"))
                    .apply(mutation, facts.committed_state_version)
            },
            |facts| {
                Ok(format!(
                    "{{\"base\":{{\"state_version\":{}}},\"unicode\":\"결과🙂\"}}",
                    facts.committed_state_version
                ))
            },
        )?;
        let MutationCommitOutcome::Committed { response_json, .. } = committed else {
            panic!("operation-result fixture should commit");
        };

        let stored = store
            .operation_result(MethodName::UpdateScope, &idempotency_key)?
            .expect("committed replay response should be retrievable");
        assert_eq!(stored.project_id, PROJECT_ID);
        assert_eq!(stored.source_method, MethodName::UpdateScope.as_str());
        assert_eq!(stored.source_idempotency_key, idempotency_key.as_str());
        assert_eq!(stored.committed_state_version, 1);
        assert_eq!(stored.actor_source, ACTOR_SOURCE);
        assert_eq!(stored.operation_category, "agent_workflow");
        assert_eq!(stored.response_json, response_json);
        assert_eq!(stored.response_size_bytes, response_json.len() as u64);
        assert_eq!(
            stored.response_sha256,
            format!("sha256:{:x}", Sha256::digest(response_json.as_bytes()))
        );
        Ok(())
    }

    #[test]
    fn invalid_replay_identity_is_rejected_before_transaction_and_effects(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let before_state = store.project_state()?;
        let before_effects = store.effect_counts()?;

        let mut invalid_actor = replay_context(CONNECTION_ID, "agent_workflow");
        invalid_actor.actor_source = "agent_connection:".to_owned();
        let mut invalid_category = replay_context(CONNECTION_ID, "agent_workflow");
        invalid_category.operation_category = "agent-workflow".to_owned();
        let mut blank_basis = replay_context(CONNECTION_ID, "agent_workflow");
        blank_basis.verification_basis = Some(" \t ".to_owned());
        let mut invalid_git_context = replay_context(CONNECTION_ID, "agent_workflow");
        invalid_git_context.git_workspace_context_json = Some("{}".to_owned());

        for (case, context, expected_field) in [
            ("actor", invalid_actor, "actor_source"),
            ("category", invalid_category, "operation_category"),
            ("basis", blank_basis, "verification_basis"),
            (
                "git_context",
                invalid_git_context,
                "tool_invocations.git_workspace_context_json",
            ),
        ] {
            let idempotency_key =
                IdempotencyKey::new(format!("idem_invalid_replay_identity_{case}"));
            let input = commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::UpdateScope,
                Some(&idempotency_key),
                &RequestHash::new(format!("sha256:invalid-replay-identity-{case}")),
                Some(context),
                Some(before_state.state_version),
                vec![pending_event(&format!("invalid_replay_identity_{case}"))],
            );
            let error = store
                .commit_mutation(
                    input,
                    |_, _| panic!("invalid replay identity must not apply a mutation"),
                    |_| panic!("invalid replay identity must not build a response"),
                )
                .expect_err("invalid replay identity must fail before commit");
            let StoreError::InvalidInput { detail } = error else {
                panic!("unexpected invalid replay identity error: {error}");
            };
            assert!(
                detail.starts_with(expected_field),
                "{case} reported unexpected detail: {detail}"
            );
            assert!(store.conn.is_autocommit());
            assert_eq!(store.project_state()?, before_state);
            let after_effects = store.effect_counts()?;
            assert_eq!(after_effects.state_version, before_effects.state_version);
            assert_eq!(
                after_effects.authority_events,
                before_effects.authority_events
            );
            assert_eq!(
                after_effects.tool_invocations,
                before_effects.tool_invocations
            );
            assert_eq!(after_effects, before_effects);
            assert!(store
                .tool_invocation(MethodName::UpdateScope, &idempotency_key)?
                .is_none());
        }
        Ok(())
    }

    #[test]
    fn loaded_replay_context_rejects_corrupt_typed_identity_without_effect(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let idempotency_key = IdempotencyKey::new("idem_store_loaded_replay_identity");
        let context = replay_context(CONNECTION_ID, "agent_workflow");
        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&idempotency_key),
            &RequestHash::new("sha256:loaded-replay-identity"),
            Some(context.clone()),
            Some(0),
            vec![pending_event("loaded_replay_identity")],
        );
        let committed = store.commit_mutation(
            input,
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert("task_loaded_replay_identity"))
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert!(matches!(committed, MutationCommitOutcome::Committed { .. }));
        let before = store.effect_counts()?;
        let expected_record_ref = format!(
            "{PROJECT_ID}/{}/{}",
            MethodName::UpdateScope.as_str(),
            idempotency_key.as_str()
        );
        let assert_corrupt_value = |error: StoreError, expected_column: &str| match error {
            StoreError::CorruptOwnerStateValue {
                database_kind,
                table,
                record_ref,
                logical_column,
            } => {
                assert_eq!(database_kind, "project_state");
                assert_eq!(table, "tool_invocations");
                assert_eq!(record_ref, expected_record_ref);
                assert_eq!(logical_column, expected_column);
            }
            other => panic!("unexpected replay identity error: {other}"),
        };

        store.conn.execute(
            "UPDATE tool_invocations
                SET actor_source = 'not-an-actor'
              WHERE project_id = ?1
                AND tool_name = ?2
                AND idempotency_key = ?3",
            params![
                PROJECT_ID,
                MethodName::UpdateScope.as_str(),
                idempotency_key.as_str()
            ],
        )?;
        let actor_error = store
            .operation_result(MethodName::UpdateScope, &idempotency_key)
            .expect_err("malformed stored actor source must fail closed");
        assert_corrupt_value(actor_error, "actor_source");
        store.conn.execute(
            "UPDATE tool_invocations
                SET actor_source = ?4
              WHERE project_id = ?1
                AND tool_name = ?2
                AND idempotency_key = ?3",
            params![
                PROJECT_ID,
                MethodName::UpdateScope.as_str(),
                idempotency_key.as_str(),
                ACTOR_SOURCE
            ],
        )?;

        store
            .conn
            .execute_batch("PRAGMA ignore_check_constraints = ON")?;
        store.conn.execute(
            "UPDATE tool_invocations
                SET operation_category = 'unsupported'
              WHERE project_id = ?1
                AND tool_name = ?2
                AND idempotency_key = ?3",
            params![
                PROJECT_ID,
                MethodName::UpdateScope.as_str(),
                idempotency_key.as_str()
            ],
        )?;
        store
            .conn
            .execute_batch("PRAGMA ignore_check_constraints = OFF")?;
        let category_error = store
            .tool_invocation(MethodName::UpdateScope, &idempotency_key)
            .expect_err("unsupported stored operation category must fail closed");
        assert_corrupt_value(category_error, "operation_category");
        store.conn.execute(
            "UPDATE tool_invocations
                SET operation_category = 'agent_workflow'
              WHERE project_id = ?1
                AND tool_name = ?2
                AND idempotency_key = ?3",
            params![
                PROJECT_ID,
                MethodName::UpdateScope.as_str(),
                idempotency_key.as_str()
            ],
        )?;

        store.conn.execute(
            "UPDATE tool_invocations
                SET verification_basis = ''
              WHERE project_id = ?1
                AND tool_name = ?2
                AND idempotency_key = ?3",
            params![
                PROJECT_ID,
                MethodName::UpdateScope.as_str(),
                idempotency_key.as_str()
            ],
        )?;
        let replay_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::UpdateScope,
            Some(&idempotency_key),
            &RequestHash::new("sha256:loaded-replay-identity"),
            Some(context),
            Some(0),
            vec![pending_event("loaded_replay_identity")],
        );
        let basis_error = store
            .commit_mutation(
                replay_input,
                |_, _| panic!("corrupt replay identity must not apply a mutation"),
                |_| panic!("corrupt replay identity must not rebuild a response"),
            )
            .expect_err("empty stored verification basis must fail closed");
        assert_corrupt_value(basis_error, "verification_basis");
        assert_eq!(store.effect_counts()?, before);
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
    fn write_ticket_consumption_revalidates_policy_authority_inside_transaction(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_ticket_policy_transaction";
        let write_ticket_id = "ticket_policy_transaction";
        let run_id = "run_policy_transaction";
        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::Intake,
                Some(&IdempotencyKey::new("idem_ticket_policy_transaction_setup")),
                &RequestHash::new("sha256:ticket-policy-transaction-setup"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task(
                    "ticket_policy_transaction_setup",
                    task_id,
                )],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        let change_unit_id = "change_unit_ticket_policy_transaction";
        store.conn.execute(
            "INSERT INTO change_units (
                project_id, change_unit_id, task_id, status, is_current,
                basis_state_version, created_at, updated_at
             ) VALUES (?1, ?2, ?3, 'active', 1, 1,
                       '2026-07-17T00:00:00Z', '2026-07-17T00:00:00Z')",
            params![PROJECT_ID, change_unit_id, task_id],
        )?;
        store.conn.execute(
            "UPDATE tasks
                SET current_change_unit_id = ?3
              WHERE project_id = ?1
                AND task_id = ?2",
            params![PROJECT_ID, task_id, change_unit_id],
        )?;

        let issued_fingerprint =
            crate::workflow_records::project_write_authority_fingerprint(None)?;
        let validity_basis_json = volicord_types::canonical_json_string(&json!({
            "task_id": task_id,
            "change_unit_id": change_unit_id,
            "scope_revision": 0,
            "baseline_ref": null,
            "workspace_context_sha256": null,
            "write_authority_fingerprint": issued_fingerprint,
            "approval_basis_refs": []
        }))?;
        store.conn.execute(
            "INSERT INTO write_tickets (
                project_id, write_ticket_id, task_id, change_unit_id,
                basis_state_version, status, validity_basis_json,
                allowed_path_prefixes_json, denied_path_prefixes_json,
                attempt_scope_json, created_by_actor_source, created_at,
                metadata_json
             ) VALUES (?1, ?2, ?3, ?4, 1, 'active', ?5,
                       '[\"src/export.rs\"]', '[]', '{}', ?6,
                       '2026-07-17T00:00:00Z', '{}')",
            params![
                PROJECT_ID,
                write_ticket_id,
                task_id,
                change_unit_id,
                validity_basis_json,
                ACTOR_SOURCE
            ],
        )?;
        let tightened_policy = json!({
            "schema": volicord_types::WORKFLOW_POLICY_CONTRACT_ID,
            "workflow": {
                "default_direct_control": "tracked",
                "default_work_control": "tracked",
                "light": {
                    "enabled": false,
                    "max_intended_paths": 3,
                    "allowed_path_patterns": [],
                    "denied_path_patterns": ["src/**"],
                    "final_acceptance": "policy_dependent"
                },
                "write_ticket": {
                    "idle_timeout_minutes": null
                }
            }
        });
        let policy_json = volicord_types::canonical_json_string(&tightened_policy)?;
        let policy_fingerprint =
            volicord_types::canonical_json_sha256(&tightened_policy)?.into_inner();
        let current_fingerprint =
            crate::workflow_records::project_write_authority_fingerprint(Some(&policy_json))?;
        assert_ne!(issued_fingerprint, current_fingerprint);
        store.conn.execute(
            "INSERT INTO project_workflow_policies (
                project_id, policy_schema, policy_version, policy_json,
                policy_fingerprint, source, applied_at, created_at
             ) VALUES (?1, ?2, 1, ?3, ?4, 'store_test',
                       '2026-07-17T00:00:00Z', '2026-07-17T00:00:00Z')",
            params![
                PROJECT_ID,
                volicord_types::WORKFLOW_POLICY_CONTRACT_ID,
                policy_json,
                policy_fingerprint
            ],
        )?;
        let before_state = store.project_state()?;
        let before_effects = store.effect_counts()?;

        let error = store
            .commit_mutation(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::RecordRun,
                    Some(&IdempotencyKey::new(
                        "idem_ticket_policy_transaction_consume",
                    )),
                    &RequestHash::new("sha256:ticket-policy-transaction-consume"),
                    Some(replay_context(CONNECTION_ID, "agent_workflow")),
                    Some(1),
                    vec![pending_event_for_task(
                        "ticket_policy_transaction_consume",
                        task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::InsertRun(RunInsert {
                        run_id: run_id.to_owned(),
                        task_id: task_id.to_owned(),
                        change_unit_id: None,
                        scope_revision: 0,
                        write_ticket_id: Some(write_ticket_id.to_owned()),
                        kind: "implementation".to_owned(),
                        status: "recorded".to_owned(),
                        summary_json: "{}".to_owned(),
                        observed_changes_json: "{}".to_owned(),
                        evidence_updates_json: "[]".to_owned(),
                        write_ticket_effect_json: "{}".to_owned(),
                        created_by_actor_source: ACTOR_SOURCE.to_owned(),
                        metadata_json: "{}".to_owned(),
                    })
                    .apply(mutation, facts.committed_state_version)?;
                    CoreStorageMutation::ConsumeWriteTicket(WriteTicketConsumption {
                        write_ticket_id: write_ticket_id.to_owned(),
                        run_id: run_id.to_owned(),
                        expected_basis_state_version: 1,
                        expected_write_authority_fingerprint: issued_fingerprint.clone(),
                    })
                    .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )
            .expect_err("changed policy authority must reject ticket consumption");

        assert!(matches!(
            error,
            StoreError::Conflict {
                entity: "write_ticket",
                ..
            }
        ));
        let (status, consumed_by_run_id): (String, Option<String>) = store.conn.query_row(
            "SELECT status, consumed_by_run_id
               FROM write_tickets
              WHERE project_id = ?1
                AND write_ticket_id = ?2",
            params![PROJECT_ID, write_ticket_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(status, "active");
        assert_eq!(consumed_by_run_id, None);
        let run_count: i64 = store.conn.query_row(
            "SELECT COUNT(*)
               FROM runs
              WHERE project_id = ?1
                AND run_id = ?2",
            params![PROJECT_ID, run_id],
            |row| row.get(0),
        )?;
        assert_eq!(run_count, 0);
        assert_eq!(store.project_state()?, before_state);
        assert_eq!(store.effect_counts()?, before_effects);
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

        let user_context = user_replay_context();
        let second = store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new("idem_authority_event_second")),
                &RequestHash::new("sha256:authority-second"),
                Some(user_context),
                Some(1),
                vec![pending_event_for_task("authority_second", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::UpdateTaskScope(TaskScopeUpdate {
                    task_id: task_id.to_owned(),
                    work_phase: None,
                    lifecycle_phase: None,
                    result: None,
                    title: Some("Authority event projection".to_owned()),
                    summary: None,
                    shaping_summary_json: None,
                    bounded_context_json: None,
                    autonomy_boundary_json: None,
                    close_summary_json: None,
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
        assert_eq!(rows[1].4, "local_user");
        assert_eq!(rows[1].5, "user_only");
        assert_eq!(rows[1].7, "sha256:authority-second");
        assert_eq!(rows[1].8.as_deref(), Some(rows[0].9.as_str()));
        assert!(rows[1].9.starts_with("sha256:"));
        assert_eq!(rows[1].9.len(), 71);
        assert_ne!(rows[0].9, rows[1].9);

        let task_scoped_event_count: i64 = store.conn.query_row(
            "SELECT COUNT(*)
               FROM authority_events
              WHERE project_id = ?1
                AND task_id IS NOT NULL
                AND event_type = 'store_test_event'",
            [PROJECT_ID],
            |row| row.get(0),
        )?;
        assert_eq!(task_scoped_event_count, 2);
        Ok(())
    }

    #[test]
    fn user_action_request_and_basis_store_apis_round_trip() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_basis_round_trip";
        let request_id = "action_basis_round_trip";
        let now = UtcTimestamp::parse("2026-01-01T00:10:00Z")?;

        let first_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
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
                    CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                        request_id, task_id, None,
                    )),
                ] {
                    storage_mutation.apply(mutation, facts.committed_state_version)?;
                }
                Ok(())
            },
            response_json,
        )?;
        assert!(matches!(first, MutationCommitOutcome::Committed { .. }));

        let current = store
            .user_action_record(request_id, &now)?
            .expect("user-action request should be readable");
        assert_eq!(current.status, UserActionStatus::Pending);
        assert_eq!(current.request.user_action_request_id, request_id);
        assert_eq!(current.request.task_id, task_id);
        assert_eq!(current.request.action_kind, UserActionKind::ProductDecision);
        assert_eq!(current.request.basis_status, UserActionBasisStatus::Current);
        assert_eq!(current.request.required_for_json, r#"["informational"]"#);
        assert_eq!(current.request.requested_by_actor_source, ACTOR_SOURCE);
        assert!(current.resolution.is_none());
        let basis: UserActionBasis = serde_json::from_str(&current.request.basis_json)?;
        assert_eq!(basis.compatibility_status(), UserActionBasisStatus::Current);
        assert_eq!(basis.coordinates().task_id.as_str(), task_id);

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
                CoreStorageMutation::MarkUserActionBasesStatus(UserActionBasisStatusMark {
                    user_action_request_ids: vec![request_id.to_owned()],
                    basis_status: UserActionBasisStatus::Stale,
                })
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert!(matches!(stale, MutationCommitOutcome::Committed { .. }));
        let stale = store
            .user_action_record(request_id, &now)?
            .expect("stale request should remain readable");
        assert_eq!(stale.status, UserActionStatus::Stale);
        assert_eq!(stale.request.basis_status, UserActionBasisStatus::Stale);
        let stale_basis: UserActionBasis = serde_json::from_str(&stale.request.basis_json)?;
        assert_eq!(
            stale_basis.compatibility_status(),
            UserActionBasisStatus::Stale
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
                CoreStorageMutation::MarkUserActionBasesStatus(UserActionBasisStatusMark {
                    user_action_request_ids: vec![request_id.to_owned()],
                    basis_status: UserActionBasisStatus::Superseded,
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
                .user_action_record(request_id, &now)?
                .expect("superseded request should remain readable")
                .status,
            UserActionStatus::Superseded
        );
        Ok(())
    }

    #[test]
    fn user_action_request_store_rejects_empty_duplicate_and_mismatched_owner_facts(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_invalid_user_action_owner_facts";

        for (marker, mut action) in [
            (
                "empty_required_for",
                user_action_request_insert("action_empty_required_for", task_id, None),
            ),
            (
                "duplicate_required_for",
                user_action_request_insert("action_duplicate_required_for", task_id, None),
            ),
            (
                "mismatched_sensitive_scope",
                user_action_request_insert("action_mismatched_sensitive_scope", task_id, None),
            ),
            (
                "incompatible_required_for",
                user_action_request_insert("action_incompatible_required_for", task_id, None),
            ),
        ] {
            match marker {
                "empty_required_for" => {
                    let mut request = serde_json::from_str::<Value>(&action.request_json)?;
                    request["required_for"] = json!([]);
                    action.request_json = request.to_string();
                    action.required_for_json = "[]".to_owned();
                }
                "duplicate_required_for" => {
                    let mut request = serde_json::from_str::<Value>(&action.request_json)?;
                    request["required_for"] = json!(["informational", "informational"]);
                    action.request_json = request.to_string();
                    action.required_for_json = r#"["informational","informational"]"#.to_owned();
                }
                "mismatched_sensitive_scope" => {
                    let mut basis = serde_json::from_str::<Value>(&action.basis_json)?;
                    basis["sensitive_action_scope"] = json!({
                        "action_kind": "write_files",
                        "description": "Bounded write.",
                        "intended_paths": ["src/lib.rs"],
                        "sensitive_categories": ["product_file_write"],
                        "command_or_tool_summary": null,
                        "network_or_host_summary": null,
                        "secret_or_credential_summary": null,
                        "capability_claim": "Local file write only.",
                        "expires_at": null
                    });
                    action.basis_json = basis.to_string();
                }
                "incompatible_required_for" => {
                    let mut request = serde_json::from_str::<Value>(&action.request_json)?;
                    request["required_for"] = json!(["close_cancel"]);
                    action.request_json = request.to_string();
                    action.required_for_json = r#"["close_cancel"]"#.to_owned();
                }
                _ => unreachable!("test table contains only declared invalid cases"),
            }
            let error = store
                .commit_mutation(
                    commit_input(
                        &ProjectId::new(PROJECT_ID),
                        MethodName::RequestUserAction,
                        Some(&IdempotencyKey::new(format!("idem_store_{marker}"))),
                        &RequestHash::new(format!("sha256:{marker}")),
                        Some(replay_context(CONNECTION_ID, "agent_workflow")),
                        Some(0),
                        vec![pending_event_for_task(marker, task_id)],
                    ),
                    |mutation, facts| {
                        CoreStorageMutation::InsertTask(task_insert(task_id))
                            .apply(mutation, facts.committed_state_version)?;
                        CoreStorageMutation::InsertUserActionRequest(action)
                            .apply(mutation, facts.committed_state_version)
                    },
                    response_json,
                )
                .expect_err("invalid user-action owner facts must fail closed");
            assert!(matches!(&error, StoreError::InvalidInput { .. }));
            if marker == "incompatible_required_for" {
                assert!(matches!(
                    error,
                    StoreError::InvalidInput { detail }
                        if detail == "user_action_requests.request_json required_for contains an operation incompatible with its action kind"
                ));
            }
            assert_eq!(store.effect_counts()?.tasks, 0);
        }
        Ok(())
    }

    #[test]
    fn user_action_request_timestamp_order_is_strict_at_insert_boundaries(
    ) -> Result<(), Box<dyn Error>> {
        for (suffix, expires_at, should_commit) in [
            ("before", "2025-12-31T23:59:59.999Z", false),
            ("equal", "2026-01-01T00:00:00Z", false),
            ("after", "2026-01-01T00:00:00.001Z", true),
        ] {
            let harness = StoreHarness::new()?;
            let mut store = harness.store()?;
            let task_id = format!("task_request_timestamp_{suffix}");
            let request_id = format!("action_request_timestamp_{suffix}");
            let mut action = user_action_request_insert(&request_id, &task_id, None);
            set_user_action_request_expiry(&mut action, expires_at);
            let outcome = store.commit_mutation(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::RequestUserAction,
                    Some(&IdempotencyKey::new(format!(
                        "idem_request_timestamp_{suffix}"
                    ))),
                    &RequestHash::new(format!("sha256:request-timestamp-{suffix}")),
                    Some(replay_context(CONNECTION_ID, "agent_workflow")),
                    Some(0),
                    vec![pending_event_for_task(
                        &format!("{suffix}_request"),
                        &task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::InsertTask(task_insert(&task_id))
                        .apply(mutation, facts.committed_state_version)?;
                    CoreStorageMutation::InsertUserActionRequest(action)
                        .apply(mutation, facts.committed_state_version)
                },
                response_json,
            );

            if should_commit {
                assert!(matches!(outcome?, MutationCommitOutcome::Committed { .. }));
                assert_eq!(
                    store
                        .user_action_record(
                            &request_id,
                            &UtcTimestamp::parse("2026-01-01T00:00:00Z")?,
                        )?
                        .expect("strictly later expiry should remain readable")
                        .status,
                    UserActionStatus::Pending
                );
            } else {
                let error = outcome.expect_err("non-later expiry must reject atomically");
                assert!(matches!(
                    error,
                    StoreError::InvalidInput { detail }
                        if detail == "user_action_requests.expires_at must be later than user_action_requests.requested_at"
                ));
                assert_eq!(store.effect_counts()?.tasks, 0);
            }
        }
        Ok(())
    }

    #[test]
    fn evidence_observation_request_insert_rejects_extended_ttl_without_effect(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_evidence_action_extended_ttl";
        let request_id = "action_evidence_action_extended_ttl";
        let mut action = evidence_user_action_request_insert(request_id, task_id, 1);
        set_user_action_request_expiry(&mut action, "2026-01-01T00:16:00Z");
        let before_state = store.project_state()?;
        let before_effects = store.effect_counts()?;

        let error = store
            .commit_mutation(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::RequestUserAction,
                    Some(&IdempotencyKey::new("idem_evidence_action_extended_ttl")),
                    &RequestHash::new("sha256:evidence-action-extended-ttl"),
                    Some(replay_context(CONNECTION_ID, "agent_workflow")),
                    Some(0),
                    vec![pending_event_for_task(
                        "evidence_action_extended_ttl",
                        task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::InsertTask(task_insert(task_id))
                        .apply(mutation, facts.committed_state_version)?;
                    CoreStorageMutation::InsertUserActionRequest(action)
                        .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )
            .expect_err("a 16-minute evidence-observation request TTL must reject atomically");

        assert!(matches!(
            error,
            StoreError::InvalidInput { detail }
                if detail == "evidence-observation user_action_requests.expires_at must be exactly 15 minutes after user_action_requests.requested_at"
        ));
        assert_eq!(store.project_state()?, before_state);
        assert_eq!(store.effect_counts()?, before_effects);
        Ok(())
    }

    #[test]
    fn evidence_capture_intent_insert_rejects_extended_ttl_without_effect(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_capture_intent_extended_ttl";
        let change_unit_id = "cu_capture_intent_extended_ttl";
        let before_state = store.project_state()?;
        let before_effects = store.effect_counts()?;
        let capture_intent = EvidenceCaptureIntentInsert {
            evidence_capture_intent_id: "capture_intent_extended_ttl".to_owned(),
            task_id: task_id.to_owned(),
            change_unit_id: change_unit_id.to_owned(),
            scope_revision: 0,
            baseline_ref: "baseline_capture_intent_extended_ttl".to_owned(),
            target_json: json!({
                "target_kind": "supplemental_claim",
                "evidence_claim_id": "claim_capture_intent_extended_ttl",
                "statement": "A fixed capture-intent TTL is required."
            })
            .to_string(),
            capture_kind: "verified_command_execution".to_owned(),
            capture_spec_json: json!({
                "capture_type": "verified_command_execution",
                "command_summary": "Run a bounded local verification."
            })
            .to_string(),
            input_sha256: "a".repeat(64),
            expected_outcome_json: "{}".to_owned(),
            requested_by_actor_source: ACTOR_SOURCE.to_owned(),
            requesting_connection_internal_id: CONNECTION_ID.to_owned(),
            session_context_json: "{}".to_owned(),
            workspace_context_json: "{}".to_owned(),
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            expires_at: "2026-01-01T00:16:00Z".to_owned(),
            metadata_json: "{}".to_owned(),
        };

        let error = store
            .commit_mutation(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::PrepareEvidenceCapture,
                    Some(&IdempotencyKey::new("idem_capture_intent_extended_ttl")),
                    &RequestHash::new("sha256:capture-intent-extended-ttl"),
                    Some(replay_context(CONNECTION_ID, "agent_workflow")),
                    Some(0),
                    vec![pending_event_for_task(
                        "capture_intent_extended_ttl",
                        task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::InsertTask(task_insert(task_id))
                        .apply(mutation, facts.committed_state_version)?;
                    CoreStorageMutation::InsertCurrentChangeUnit(change_unit_insert(
                        change_unit_id,
                        task_id,
                        "null".to_owned(),
                    ))
                    .apply(mutation, facts.committed_state_version)?;
                    CoreStorageMutation::InsertEvidenceCaptureIntent(capture_intent)
                        .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )
            .expect_err("a 16-minute evidence-capture intent TTL must reject atomically");

        assert!(matches!(error, StoreError::SchemaInvariant { .. }));
        assert_eq!(store.project_state()?, before_state);
        assert_eq!(store.effect_counts()?, before_effects);
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
                    CoreStorageMutation::EnsureEvidenceClaim(EvidenceClaimInsert {
                        task_id: task_id.to_owned(),
                        evidence_claim_id: "claim_search_result_count".to_owned(),
                        statement: "Search result count was verified.".to_owned(),
                    }),
                    CoreStorageMutation::InsertEvidenceObservation(EvidenceObservationInsert {
                        evidence_observation_id: observation_id.to_owned(),
                        task_id: task_id.to_owned(),
                        change_unit_id: None,
                        run_id: Some(run_id.to_owned()),
                        acceptance_criterion_id: None,
                        evidence_claim_id: Some("claim_search_result_count".to_owned()),
                        source_kind: "external_tool".to_owned(),
                        assurance_level: "external_tool_result".to_owned(),
                        observed_by_actor_source: Some(ACTOR_SOURCE.to_owned()),
                        tool_name: Some("local-test-runner".to_owned()),
                        tool_invocation_id: Some("tool_invocation_001".to_owned()),
                        tool_metadata_json: json!({"exit_code": 0}).to_string(),
                        input_refs_json: "[]".to_owned(),
                        source_refs_json: json!([{
                            "source_kind": "user_context",
                            "source": {"context_id": "message_store_evidence"}
                        }])
                        .to_string(),
                        output_artifact_refs_json: "[]".to_owned(),
                        limitations_json: json!(["External tool result is not a proof."])
                            .to_string(),
                        observed_at: "2026-06-18T00:00:00Z".to_owned(),
                        recorded_at: "2026-06-18T00:00:01Z".to_owned(),
                        metadata_json: json!({
                            "recorded_by_run_id": run_id,
                            "invocation_verification_basis": "store_test_boundary",
                            "producer_anchor": {
                                "producer_kind": "unverified_caller",
                                "producer_ref": null,
                                "output_artifact_refs": [],
                                "verification_basis": null
                            },
                            "relevance_assessment": {
                                "status": "unassessed",
                                "assessment_ref": null,
                                "assessed_by_actor_source": null
                            }
                        })
                        .to_string(),
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
            serde_json::from_str::<Value>(&record.source_refs_json)?,
            json!([{
                "source_kind": "user_context",
                "source": {"context_id": "message_store_evidence"}
            }])
        );
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
    fn user_action_store_derives_expiry_resolution_and_stale_status() -> Result<(), Box<dyn Error>>
    {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_user_action_status";

        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_store_action_expiring")),
                &RequestHash::new("sha256:action-expiring"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("action_expiring", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                    "action_expiring",
                    task_id,
                    Some("2026-01-01T00:15:00Z"),
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        let before_expiry = UtcTimestamp::parse("2026-01-01T00:14:59Z")?;
        let at_expiry = UtcTimestamp::parse("2026-01-01T00:15:00Z")?;
        assert_eq!(
            store
                .user_action_record("action_expiring", &before_expiry)?
                .expect("expiring action should be readable")
                .status,
            UserActionStatus::Pending
        );
        assert_eq!(
            store
                .user_action_record("action_expiring", &at_expiry)?
                .expect("expired action should remain readable")
                .status,
            UserActionStatus::Expired
        );

        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_store_action_current")),
                &RequestHash::new("sha256:action-current"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(1),
                vec![pending_event_for_task("action_current", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                    "action_current",
                    task_id,
                    None,
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new("idem_store_action_resolve")),
                &RequestHash::new("sha256:action-resolve"),
                Some(VerifiedReplayContext {
                    actor_source: "local_user".to_owned(),
                    operation_category: "user_only".to_owned(),
                    verification_basis: Some("store_test_user_channel".to_owned()),
                    git_workspace_context_json: None,
                }),
                Some(2),
                vec![pending_event_for_task("action_resolve", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertUserActionResolution(user_action_resolution_insert(
                    "resolution_current",
                    "action_current",
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert_eq!(
            store
                .user_action_record("action_current", &at_expiry)?
                .expect("resolved action should be readable")
                .status,
            UserActionStatus::Resolved
        );

        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::UpdateScope,
                Some(&IdempotencyKey::new("idem_store_action_stale")),
                &RequestHash::new("sha256:action-stale"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(3),
                vec![pending_event_for_task("action_stale", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::MarkUserActionBasesStatus(UserActionBasisStatusMark {
                    user_action_request_ids: vec!["action_current".to_owned()],
                    basis_status: UserActionBasisStatus::Stale,
                })
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        let stale = store
            .user_action_record("action_current", &at_expiry)?
            .expect("stale action should be readable");
        assert_eq!(stale.status, UserActionStatus::Stale);
        assert_eq!(
            serde_json::from_str::<Value>(&stale.request.basis_json)?["coordinates"]
                ["compatibility_status"],
            "stale"
        );
        Ok(())
    }

    #[test]
    fn user_action_resolution_round_trips_choice_and_channel_provenance(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_deferred_action";
        let request_id = "action_deferred_pair";
        let resolution_id = "resolution_deferred_pair";
        let mut deferred_request = user_action_request_insert(request_id, task_id, None);
        let mut deferred_request_json =
            serde_json::from_str::<Value>(&deferred_request.request_json)?;
        deferred_request_json["body"]["options"]
            .as_array_mut()
            .expect("choice options should be an array")
            .push(json!({
                "option_id": "defer",
                "label": "Defer",
                "description": "Defer this bounded decision.",
                "consequence": "The request remains resolved as deferred.",
                "machine_action": "defer",
                "resolution_outcome": "deferred",
                "is_default": false
            }));
        deferred_request.request_json = deferred_request_json.to_string();

        let insert_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
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
                    CoreStorageMutation::InsertUserActionRequest(deferred_request),
                ] {
                    storage_mutation.apply(mutation, facts.committed_state_version)?;
                }
                Ok(())
            },
            response_json,
        )?;
        assert!(matches!(inserted, MutationCommitOutcome::Committed { .. }));

        let mut resolution = user_action_resolution_insert(resolution_id, request_id);
        resolution.channel_submission_id = "submission_deferred_pair".to_owned();
        resolution.resolution_json = choice_resolution_json(
            "defer",
            UserActionOptionAction::Defer,
            JudgmentResolutionOutcome::Deferred,
        );
        resolution.resolved_assurance_level = "verified_local_user_channel".to_owned();
        let resolve_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_defer_resolve")),
            &RequestHash::new("sha256:defer-resolve"),
            Some(user_replay_context()),
            Some(1),
            vec![pending_event_for_task("defer_resolve", task_id)],
        );
        let resolved = store.commit_mutation(
            resolve_input,
            |mutation, facts| {
                CoreStorageMutation::InsertUserActionResolution(resolution)
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        assert!(matches!(resolved, MutationCommitOutcome::Committed { .. }));

        let record = store
            .user_action_resolution_record(resolution_id)?
            .expect("resolved user action should be readable");
        assert_eq!(record.user_action_request_id, request_id);
        assert_eq!(record.channel_kind, UserActionChannelKind::Cli);
        assert_eq!(record.channel_submission_id, "submission_deferred_pair");
        assert_eq!(record.resolved_by_actor_source, "local_user");
        assert_eq!(
            serde_json::from_str::<Value>(&record.resolution_json)?["machine_action"],
            "defer"
        );
        assert_eq!(
            store
                .user_action_resolution_for_channel_submission(
                    UserActionChannelKind::Cli,
                    "submission_deferred_pair",
                )?
                .expect("channel submission lookup should return the immutable resolution"),
            record
        );
        assert_eq!(
            store
                .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:11:00Z")?,)?
                .expect("resolved request should remain readable")
                .status,
            UserActionStatus::Resolved
        );
        let before_tamper = store.effect_counts()?;
        store
            .conn
            .execute_batch("PRAGMA ignore_check_constraints = ON")?;
        store.conn.execute(
            "UPDATE user_action_resolutions
                SET channel_submission_id = ?3
              WHERE project_id = ?1
                AND user_action_resolution_id = ?2",
            params![PROJECT_ID, resolution_id, "x".repeat(257)],
        )?;
        store
            .conn
            .execute_batch("PRAGMA ignore_check_constraints = OFF")?;
        assert!(matches!(
            store.user_action_resolution_record(resolution_id),
            Err(StoreError::CorruptOwnerStateValue { .. })
        ));
        assert_eq!(store.effect_counts()?, before_tamper);
        Ok(())
    }

    #[test]
    fn user_action_resolution_timestamp_order_enforces_half_open_boundaries(
    ) -> Result<(), Box<dyn Error>> {
        for (suffix, resolved_at, expected_error) in [
            (
                "before_request",
                "2025-12-31T23:59:59.999Z",
                Some(
                    "user_action_resolutions.resolved_at must be at or after user_action_requests.requested_at",
                ),
            ),
            ("at_request", "2026-01-01T00:00:00Z", None),
            ("before_expiry", "2026-01-01T00:00:09.999Z", None),
            (
                "at_expiry",
                "2026-01-01T00:00:10Z",
                Some(
                    "user_action_resolutions.resolved_at must be before user_action_requests.expires_at",
                ),
            ),
            (
                "after_expiry",
                "2026-01-01T00:00:10.001Z",
                Some(
                    "user_action_resolutions.resolved_at must be before user_action_requests.expires_at",
                ),
            ),
        ] {
            let harness = StoreHarness::new()?;
            let mut store = harness.store()?;
            let task_id = format!("task_resolution_timestamp_{suffix}");
            let request_id = format!("action_resolution_timestamp_{suffix}");
            let resolution_id = format!("resolution_timestamp_{suffix}");
            let mut action = user_action_request_insert(&request_id, &task_id, None);
            set_user_action_request_expiry(&mut action, "2026-01-01T00:00:10Z");
            store.commit_mutation(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::RequestUserAction,
                    Some(&IdempotencyKey::new(format!(
                        "idem_resolution_timestamp_request_{suffix}"
                    ))),
                    &RequestHash::new(format!(
                        "sha256:resolution-timestamp-request-{suffix}"
                    )),
                    Some(replay_context(CONNECTION_ID, "agent_workflow")),
                    Some(0),
                    vec![pending_event_for_task(
                        &format!("{suffix}_request"),
                        &task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::InsertTask(task_insert(&task_id))
                        .apply(mutation, facts.committed_state_version)?;
                    CoreStorageMutation::InsertUserActionRequest(action)
                        .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )?;

            let mut resolution = user_action_resolution_insert(&resolution_id, &request_id);
            resolution.resolved_at = resolved_at.to_owned();
            let outcome = store.commit_mutation(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::ResolveUserAction,
                    Some(&IdempotencyKey::new(format!(
                        "idem_resolution_timestamp_resolve_{suffix}"
                    ))),
                    &RequestHash::new(format!(
                        "sha256:resolution-timestamp-resolve-{suffix}"
                    )),
                    Some(user_replay_context()),
                    Some(1),
                    vec![pending_event_for_task(
                        &format!("{suffix}_resolve"),
                        &task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::InsertUserActionResolution(resolution)
                        .apply(mutation, facts.committed_state_version)
                },
                response_json,
            );

            if let Some(expected_error) = expected_error {
                let error = outcome.expect_err("out-of-window resolution must reject atomically");
                assert!(matches!(
                    error,
                    StoreError::InvalidInput { detail } if detail == expected_error
                ));
                assert_eq!(store.effect_counts()?.user_action_resolutions, 0);
                assert_eq!(store.project_state()?.state_version, 1);
            } else {
                assert!(matches!(outcome?, MutationCommitOutcome::Committed { .. }));
                assert_eq!(
                    store
                        .user_action_resolution_record(&resolution_id)?
                        .expect("in-window resolution should remain readable")
                        .resolved_at,
                    resolved_at
                );
            }
        }
        Ok(())
    }

    #[test]
    fn evidence_observation_resolution_preserves_exact_candidate_after_projection_advances(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_observation_resolution_reread";
        let request_id = "action_observation_resolution_reread";
        let resolution_id = "resolution_observation_reread";

        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_store_observation_request")),
                &RequestHash::new("sha256:observation-request"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("observation_request", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::InsertUserActionRequest(evidence_user_action_request_insert(
                    request_id, task_id, 3,
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        let before_mismatch = store.effect_counts()?;
        let mismatch = store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new("idem_store_observation_resolution")),
                &RequestHash::new("sha256:observation-resolution"),
                Some(user_replay_context()),
                Some(1),
                vec![pending_event_for_task("observation_resolution", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertUserActionResolution(
                    evidence_user_action_resolution_insert(resolution_id, request_id, task_id, 4),
                )
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        );
        assert!(matches!(mismatch, Err(StoreError::InvalidInput { .. })));
        assert_eq!(store.effect_counts()?, before_mismatch);
        assert!(store
            .user_action_resolution_record(resolution_id)?
            .is_none());

        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new("idem_store_observation_resolution")),
                &RequestHash::new("sha256:observation-resolution"),
                Some(user_replay_context()),
                Some(1),
                vec![pending_event_for_task("observation_resolution", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertUserActionResolution(
                    evidence_user_action_resolution_insert(resolution_id, request_id, task_id, 3),
                )
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        let resolved = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)?
            .expect("resolved evidence-observation action should remain readable");
        assert_eq!(resolved.status, UserActionStatus::Resolved);
        let resolution = store
            .user_action_resolution_record(resolution_id)?
            .expect("the immutable resolution should be readable by id");
        assert_eq!(
            serde_json::from_str::<Value>(&resolution.resolution_json)?["observation"]
                ["output_artifact_refs"][0]["created_by_run_ref"]["produced_at_state_version"],
            3
        );

        let mut tampered: Value = serde_json::from_str(&resolution.resolution_json)?;
        tampered["observation"]["output_artifact_refs"][0]["sha256"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        store.conn.execute(
            "UPDATE user_action_resolutions
                SET resolution_json = ?2
              WHERE project_id = ?1
                AND user_action_resolution_id = ?3",
            params![PROJECT_ID, tampered.to_string(), resolution_id],
        )?;
        assert!(matches!(
            store.user_action_resolution_record(resolution_id),
            Err(StoreError::CorruptOwnerStateValue { .. })
        ));
        Ok(())
    }

    #[test]
    fn user_action_resolution_is_one_to_one_and_channel_submission_is_unique(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_resolution_uniqueness";
        let first_request_id = "action_resolution_unique_first";
        let second_request_id = "action_resolution_unique_second";

        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_store_resolution_unique_insert")),
                &RequestHash::new("sha256:resolution-unique-insert"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("resolution_unique_insert", task_id)],
            ),
            |mutation, facts| {
                for storage_mutation in [
                    CoreStorageMutation::InsertTask(task_insert(task_id)),
                    CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                        first_request_id,
                        task_id,
                        None,
                    )),
                    CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                        second_request_id,
                        task_id,
                        None,
                    )),
                ] {
                    storage_mutation.apply(mutation, facts.committed_state_version)?;
                }
                Ok(())
            },
            response_json,
        )?;

        let mut first_resolution =
            user_action_resolution_insert("resolution_unique_first", first_request_id);
        first_resolution.channel_submission_id = "submission_unique".to_owned();
        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new("idem_store_resolution_unique_first")),
                &RequestHash::new("sha256:resolution-unique-first"),
                Some(user_replay_context()),
                Some(1),
                vec![pending_event_for_task("resolution_unique_first", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertUserActionResolution(first_resolution)
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        let before_conflicts = store.effect_counts()?;

        let second_for_same_request = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_resolution_same_request")),
            &RequestHash::new("sha256:resolution-same-request"),
            Some(user_replay_context()),
            Some(2),
            vec![pending_event_for_task("resolution_same_request", task_id)],
        );
        let error = store
            .commit_mutation(
                second_for_same_request,
                |mutation, facts| {
                    CoreStorageMutation::InsertUserActionResolution(user_action_resolution_insert(
                        "resolution_unique_duplicate_request",
                        first_request_id,
                    ))
                    .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )
            .expect_err("one request must not accept a second immutable resolution");
        assert!(matches!(error, StoreError::Sqlite(_)));
        assert_eq!(store.effect_counts()?, before_conflicts);

        let mut reused_submission =
            user_action_resolution_insert("resolution_unique_submission", second_request_id);
        reused_submission.channel_submission_id = "submission_unique".to_owned();
        let error = store
            .commit_mutation(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::ResolveUserAction,
                    Some(&IdempotencyKey::new(
                        "idem_store_resolution_same_submission",
                    )),
                    &RequestHash::new("sha256:resolution-same-submission"),
                    Some(user_replay_context()),
                    Some(2),
                    vec![pending_event_for_task(
                        "resolution_same_submission",
                        task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::InsertUserActionResolution(reused_submission)
                        .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )
            .expect_err("one channel submission must not resolve two requests");
        assert!(matches!(error, StoreError::Sqlite(_)));
        assert_eq!(store.effect_counts()?, before_conflicts);
        assert_eq!(
            store
                .user_action_resolution_for_channel_submission(
                    UserActionChannelKind::Cli,
                    "submission_unique",
                )?
                .expect("the first resolution must remain canonical")
                .user_action_request_id,
            first_request_id
        );
        Ok(())
    }

    #[test]
    fn user_action_resolution_rejects_request_action_kind_mismatch() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_resolution_kind_mismatch";
        let request_id = "action_resolution_kind_mismatch";

        let insert_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
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
                    CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                        request_id, task_id, None,
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
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_missing_action_resolve")),
            &RequestHash::new("sha256:missing-action-resolve"),
            Some(user_replay_context()),
            Some(1),
            vec![pending_event_for_task("missing_action_resolve", task_id)],
        );
        let mut resolution = user_action_resolution_insert("resolution_kind_mismatch", request_id);
        resolution.action_kind = UserActionKind::TechnicalDecision;

        let error = store
            .commit_mutation(
                resolve_input,
                |mutation, facts| {
                    CoreStorageMutation::InsertUserActionResolution(resolution)
                        .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )
            .expect_err("resolution action kind must match its request");
        assert!(matches!(error, StoreError::InvalidInput { .. }));
        assert_eq!(store.effect_counts()?, before);
        let record = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)?
            .expect("pending user action should remain readable");
        assert_eq!(record.status, UserActionStatus::Pending);
        assert!(record.resolution.is_none());
        Ok(())
    }

    #[test]
    fn user_action_resolution_read_fails_closed_on_tampered_choice_authority(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_tampered_choice_authority";
        let request_id = "action_tampered_choice_authority";
        let resolution_id = "resolution_tampered_choice_authority";
        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_store_tampered_choice_insert")),
                &RequestHash::new("sha256:tampered-choice-insert"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("tampered_choice_insert", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                    request_id, task_id, None,
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new("idem_store_tampered_choice_resolve")),
                &RequestHash::new("sha256:tampered-choice-resolve"),
                Some(user_replay_context()),
                Some(1),
                vec![pending_event_for_task("tampered_choice_resolve", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertUserActionResolution(user_action_resolution_insert(
                    resolution_id,
                    request_id,
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        for tampered_resolution in [
            choice_resolution_json(
                "not_a_request_option",
                UserActionOptionAction::Accept,
                JudgmentResolutionOutcome::Accepted,
            ),
            choice_resolution_json(
                "accept",
                UserActionOptionAction::Reject,
                JudgmentResolutionOutcome::Rejected,
            ),
        ] {
            store.conn.execute(
                "UPDATE user_action_resolutions
                    SET resolution_json = ?2
                  WHERE project_id = ?1
                    AND user_action_resolution_id = ?3",
                params![PROJECT_ID, tampered_resolution, resolution_id],
            )?;
            assert!(matches!(
                store.user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?),
                Err(StoreError::CorruptOwnerStateValue { .. })
            ));
            assert!(matches!(
                store.user_action_resolution_record(resolution_id),
                Err(StoreError::CorruptOwnerStateValue { .. })
            ));
        }
        Ok(())
    }

    #[test]
    fn user_action_resolution_requires_local_user_and_verified_provenance(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_resolution_provenance";
        let request_id = "action_resolution_provenance";

        let insert_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
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
                    CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                        request_id, task_id, None,
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

        let mut invalid_resolutions = Vec::new();
        let mut wrong_actor = user_action_resolution_insert("resolution_wrong_actor", request_id);
        wrong_actor.resolved_by_actor_source = ACTOR_SOURCE.to_owned();
        invalid_resolutions.push(("wrong_actor", wrong_actor));
        let mut missing_basis =
            user_action_resolution_insert("resolution_missing_basis", request_id);
        missing_basis.resolved_verification_basis.clear();
        invalid_resolutions.push(("missing_basis", missing_basis));
        let mut missing_assurance =
            user_action_resolution_insert("resolution_missing_assurance", request_id);
        missing_assurance.resolved_assurance_level.clear();
        invalid_resolutions.push(("missing_assurance", missing_assurance));
        let mut mismatched_channel_basis =
            user_action_resolution_insert("resolution_mismatched_channel_basis", request_id);
        mismatched_channel_basis.resolved_verification_basis =
            "unsupported_user_action_channel".to_owned();
        invalid_resolutions.push(("mismatched_channel_basis", mismatched_channel_basis));

        for (marker, resolution) in invalid_resolutions {
            let error = store
                .commit_mutation(
                    commit_input(
                        &ProjectId::new(PROJECT_ID),
                        MethodName::ResolveUserAction,
                        Some(&IdempotencyKey::new(format!(
                            "idem_store_resolution_{marker}"
                        ))),
                        &RequestHash::new(format!("sha256:resolution-{marker}")),
                        Some(user_replay_context()),
                        Some(1),
                        vec![pending_event_for_task(marker, task_id)],
                    ),
                    |mutation, facts| {
                        CoreStorageMutation::InsertUserActionResolution(resolution)
                            .apply(mutation, facts.committed_state_version)
                    },
                    response_json,
                )
                .expect_err("invalid user actor or provenance must reject");
            assert!(matches!(error, StoreError::InvalidInput { .. }));
            assert_eq!(store.effect_counts()?, before);
        }
        let record = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)?
            .expect("pending request should remain readable");
        assert_eq!(record.status, UserActionStatus::Pending);
        assert!(record.resolution.is_none());
        Ok(())
    }

    #[test]
    fn user_action_resolution_rejects_unknown_fields_and_invalid_outcomes(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_invalid_resolution_json";
        let request_id = "action_invalid_resolution_json";

        let insert_input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
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
                    CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                        request_id, task_id, None,
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

        let mut unknown_field =
            user_action_resolution_insert("resolution_unknown_field", request_id);
        let mut unknown_value: Value = serde_json::from_str(&unknown_field.resolution_json)?;
        unknown_value["unknown_resolution_field"] = json!(true);
        unknown_field.resolution_json = unknown_value.to_string();
        let mut invalid_outcome =
            user_action_resolution_insert("resolution_invalid_outcome", request_id);
        let mut invalid_outcome_value: Value =
            serde_json::from_str(&invalid_outcome.resolution_json)?;
        invalid_outcome_value["resolution_outcome"] = json!("blocked");
        invalid_outcome.resolution_json = invalid_outcome_value.to_string();

        for (marker, resolution) in [
            ("unknown_field", unknown_field),
            ("invalid_outcome", invalid_outcome),
        ] {
            let error = store
                .commit_mutation(
                    commit_input(
                        &ProjectId::new(PROJECT_ID),
                        MethodName::ResolveUserAction,
                        Some(&IdempotencyKey::new(format!(
                            "idem_store_resolution_{marker}"
                        ))),
                        &RequestHash::new(format!("sha256:resolution-{marker}")),
                        Some(user_replay_context()),
                        Some(1),
                        vec![pending_event_for_task(marker, task_id)],
                    ),
                    |mutation, facts| {
                        CoreStorageMutation::InsertUserActionResolution(resolution)
                            .apply(mutation, facts.committed_state_version)
                    },
                    response_json,
                )
                .expect_err("unsupported closed resolution shapes must reject");
            assert!(matches!(error, StoreError::InvalidInput { .. }));
            assert_eq!(store.effect_counts()?, before);
        }
        let record = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)?
            .expect("pending request should remain readable");
        assert_eq!(record.status, UserActionStatus::Pending);
        assert!(record.resolution.is_none());
        Ok(())
    }

    #[test]
    fn malformed_stored_user_action_basis_json_is_store_data_error() -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_malformed_basis";
        let request_id = "action_malformed_basis";

        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::RequestUserAction,
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
                CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                    request_id, task_id, None,
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
            "UPDATE user_action_requests
                SET basis_json = 'not-json'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![PROJECT_ID, request_id],
        )?;
        drop(conn);

        let store = harness.store()?;
        let error = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)
            .expect_err("malformed persisted basis JSON should be corruption");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_requests",
                logical_column: "basis_json",
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn stored_user_action_request_errors_preserve_request_and_required_for_columns(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_request_owner_columns";
        let malformed_request_id = "action_malformed_request_column";
        let mismatched_required_for_id = "action_mismatched_required_for_column";

        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_store_request_owner_columns")),
                &RequestHash::new("sha256:request-owner-columns"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("request_owner_columns", task_id)],
            ),
            |mutation, facts| {
                for storage_mutation in [
                    CoreStorageMutation::InsertTask(task_insert(task_id)),
                    CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                        malformed_request_id,
                        task_id,
                        None,
                    )),
                    CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                        mismatched_required_for_id,
                        task_id,
                        None,
                    )),
                ] {
                    storage_mutation.apply(mutation, facts.committed_state_version)?;
                }
                Ok(())
            },
            response_json,
        )?;
        store.conn.execute(
            "UPDATE user_action_requests
                SET request_json = 'not-json'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![PROJECT_ID, malformed_request_id],
        )?;
        store.conn.execute(
            "UPDATE user_action_requests
                SET required_for_json = '[\"close_complete\"]'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![PROJECT_ID, mismatched_required_for_id],
        )?;

        for (request_id, expected_column) in [
            (malformed_request_id, "request_json"),
            (mismatched_required_for_id, "required_for_json"),
        ] {
            let error = store
                .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)
                .expect_err("invalid owner JSON should fail closed on its canonical column");
            assert!(matches!(
                error,
                StoreError::CorruptOwnerStateValue {
                    table: "user_action_requests",
                    logical_column,
                    ..
                } if logical_column == expected_column
            ));
        }
        Ok(())
    }

    #[test]
    fn stored_user_action_request_fails_closed_on_incompatible_required_for(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_incompatible_required_for_reread";
        let request_id = "action_incompatible_required_for_reread";

        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new(
                    "idem_store_incompatible_required_for_reread",
                )),
                &RequestHash::new("sha256:incompatible-required-for-reread"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task(
                    "incompatible_required_for_reread",
                    task_id,
                )],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                    request_id, task_id, None,
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        let stored_request_json: String = store.conn.query_row(
            "SELECT request_json
               FROM user_action_requests
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![PROJECT_ID, request_id],
            |row| row.get(0),
        )?;
        let mut request_json = serde_json::from_str::<Value>(&stored_request_json)?;
        request_json["required_for"] = json!(["close_cancel"]);
        store.conn.execute(
            "UPDATE user_action_requests
                SET request_json = ?3,
                    required_for_json = '[\"close_cancel\"]'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![PROJECT_ID, request_id, request_json.to_string()],
        )?;

        let error = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:10:00Z")?)
            .expect_err("incompatible persisted required_for must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_requests",
                logical_column: "request_json",
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn stored_user_action_request_fails_closed_on_invalid_timestamp_order(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_request_timestamp_reread";
        let request_id = "action_request_timestamp_reread";
        let mut action = user_action_request_insert(request_id, task_id, None);
        set_user_action_request_expiry(&mut action, "2026-01-01T00:00:10Z");
        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_request_timestamp_reread")),
                &RequestHash::new("sha256:request-timestamp-reread"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("request_timestamp_reread", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::InsertUserActionRequest(action)
                    .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        let stored_request_json: String = store.conn.query_row(
            "SELECT request_json
               FROM user_action_requests
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![PROJECT_ID, request_id],
            |row| row.get(0),
        )?;
        let mut request_json = serde_json::from_str::<Value>(&stored_request_json)?;
        request_json["expires_at"] = json!("2026-01-01T00:00:00Z");
        store.conn.execute(
            "UPDATE user_action_requests
                SET request_json = ?3,
                    expires_at = '2026-01-01T00:00:00Z'
              WHERE project_id = ?1
                AND user_action_request_id = ?2",
            params![PROJECT_ID, request_id, request_json.to_string()],
        )?;

        let error = store
            .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:00:00Z")?)
            .expect_err("invalid stored request timestamp order must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_requests",
                logical_column: "expires_at",
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn stored_user_action_resolution_fails_closed_on_invalid_timestamp_order(
    ) -> Result<(), Box<dyn Error>> {
        for (suffix, corrupted_resolved_at) in [
            ("before_request", "2025-12-31T23:59:59.999Z"),
            ("at_expiry", "2026-01-01T00:00:10Z"),
        ] {
            let harness = StoreHarness::new()?;
            let mut store = harness.store()?;
            let task_id = format!("task_resolution_timestamp_reread_{suffix}");
            let request_id = format!("action_resolution_timestamp_reread_{suffix}");
            let resolution_id = format!("resolution_timestamp_reread_{suffix}");
            let mut action = user_action_request_insert(&request_id, &task_id, None);
            set_user_action_request_expiry(&mut action, "2026-01-01T00:00:10Z");
            store.commit_mutation(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::RequestUserAction,
                    Some(&IdempotencyKey::new(format!(
                        "idem_resolution_timestamp_reread_request_{suffix}"
                    ))),
                    &RequestHash::new(format!(
                        "sha256:resolution-timestamp-reread-request-{suffix}"
                    )),
                    Some(replay_context(CONNECTION_ID, "agent_workflow")),
                    Some(0),
                    vec![pending_event_for_task(
                        &format!("{suffix}_request"),
                        &task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::InsertTask(task_insert(&task_id))
                        .apply(mutation, facts.committed_state_version)?;
                    CoreStorageMutation::InsertUserActionRequest(action)
                        .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )?;
            let mut resolution = user_action_resolution_insert(&resolution_id, &request_id);
            resolution.resolved_at = "2026-01-01T00:00:05Z".to_owned();
            store.commit_mutation(
                commit_input(
                    &ProjectId::new(PROJECT_ID),
                    MethodName::ResolveUserAction,
                    Some(&IdempotencyKey::new(format!(
                        "idem_resolution_timestamp_reread_resolve_{suffix}"
                    ))),
                    &RequestHash::new(format!(
                        "sha256:resolution-timestamp-reread-resolve-{suffix}"
                    )),
                    Some(user_replay_context()),
                    Some(1),
                    vec![pending_event_for_task(
                        &format!("{suffix}_resolve"),
                        &task_id,
                    )],
                ),
                |mutation, facts| {
                    CoreStorageMutation::InsertUserActionResolution(resolution)
                        .apply(mutation, facts.committed_state_version)
                },
                response_json,
            )?;

            store.conn.execute(
                "UPDATE user_action_resolutions
                    SET resolved_at = ?3
                  WHERE project_id = ?1
                    AND user_action_resolution_id = ?2",
                params![PROJECT_ID, resolution_id, corrupted_resolved_at],
            )?;
            let error = store
                .user_action_resolution_record(&resolution_id)
                .expect_err("invalid stored resolution timestamp order must fail closed");
            assert!(matches!(
                error,
                StoreError::CorruptOwnerStateValue {
                    table: "user_action_resolutions",
                    logical_column: "resolved_at",
                    ..
                }
            ));
        }
        Ok(())
    }

    #[test]
    fn effective_user_action_rejects_resolution_from_future_without_effect(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_resolution_future_reread";
        let request_id = "action_resolution_future_reread";
        let resolution_id = "resolution_future_reread";
        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_resolution_future_request")),
                &RequestHash::new("sha256:resolution-future-request"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("resolution_future_request", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                    request_id, task_id, None,
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::ResolveUserAction,
                Some(&IdempotencyKey::new("idem_resolution_future_resolve")),
                &RequestHash::new("sha256:resolution-future-resolve"),
                Some(user_replay_context()),
                Some(1),
                vec![pending_event_for_task("resolution_future_resolve", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertUserActionResolution(user_action_resolution_insert(
                    resolution_id,
                    request_id,
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;
        store.conn.execute(
            "UPDATE user_action_resolutions
                SET resolved_at = '2999-07-13T00:00:00Z'
              WHERE project_id = ?1 AND user_action_resolution_id = ?2",
            params![PROJECT_ID, resolution_id],
        )?;
        let before = (store.effect_counts()?, store.project_state()?);
        let now = UtcTimestamp::parse(&store.current_timestamp()?)?;

        let error = store
            .user_action_record(request_id, &now)
            .expect_err("a future stored resolution cannot be current authority");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_resolutions",
                logical_column: "resolved_at",
                ..
            }
        ));
        assert_eq!((store.effect_counts()?, store.project_state()?), before);
        Ok(())
    }

    #[test]
    fn effective_user_action_read_enforces_requested_at_lower_bound() -> Result<(), Box<dyn Error>>
    {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_requested_at_lower_bound";
        let request_id = "action_requested_at_lower_bound";

        store.commit_mutation(
            commit_input(
                &ProjectId::new(PROJECT_ID),
                MethodName::RequestUserAction,
                Some(&IdempotencyKey::new("idem_store_requested_at_lower_bound")),
                &RequestHash::new("sha256:requested-at-lower-bound"),
                Some(replay_context(CONNECTION_ID, "agent_workflow")),
                Some(0),
                vec![pending_event_for_task("requested_at_lower_bound", task_id)],
            ),
            |mutation, facts| {
                CoreStorageMutation::InsertTask(task_insert(task_id))
                    .apply(mutation, facts.committed_state_version)?;
                CoreStorageMutation::InsertUserActionRequest(user_action_request_insert(
                    request_id, task_id, None,
                ))
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        let error = store
            .user_action_record(
                request_id,
                &UtcTimestamp::parse("2025-12-31T23:59:59.999Z")?,
            )
            .expect_err("time before requested_at must fail closed");
        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "user_action_requests",
                logical_column: "requested_at",
                ..
            }
        ));

        assert_eq!(
            store
                .user_action_record(request_id, &UtcTimestamp::parse("2026-01-01T00:00:00Z")?,)?
                .expect("requested_at boundary is inclusive")
                .status,
            UserActionStatus::Pending
        );
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
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_continuity")),
            &RequestHash::new("sha256:store-continuity"),
            Some(user_replay_context()),
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
                    project_continuity_record_insert(
                        "continuity_store_001",
                        task_id,
                        change_unit_id,
                        "2026-01-01T00:00:00Z",
                    ),
                )
                .apply(mutation, facts.committed_state_version)
            },
            response_json,
        )?;

        let active = store.active_project_continuity_page(10, None)?;
        assert_eq!(store.effect_counts()?.project_continuity_records, 1);
        assert_eq!(active.total_count, 1);
        assert!(!active.truncated);
        assert_eq!(active.records.len(), 1);
        assert_eq!(
            active.records[0].continuity_record_id,
            "continuity_store_001"
        );
        assert_eq!(active.records[0].kind, "decision");
        assert_eq!(active.records[0].status, "active");
        assert_eq!(active.records[0].source_task_id, task_id);
        assert_eq!(
            active.records[0].source_change_unit_id.as_deref(),
            Some(change_unit_id)
        );

        let task_records = store.project_continuity_records_for_task(task_id)?;
        assert_eq!(task_records.len(), 1);
        assert!(store.project_continuity_record_exists("continuity_store_001")?);
        Ok(())
    }

    #[test]
    fn project_continuity_pages_are_exclusive_totalled_and_tie_broken_by_id(
    ) -> Result<(), Box<dyn Error>> {
        let harness = StoreHarness::new()?;
        let mut store = harness.store()?;
        let task_id = "task_continuity_page";
        let change_unit_id = "cu_continuity_page";
        let input = commit_input(
            &ProjectId::new(PROJECT_ID),
            MethodName::ResolveUserAction,
            Some(&IdempotencyKey::new("idem_store_continuity_page")),
            &RequestHash::new("sha256:store-continuity-page"),
            Some(user_replay_context()),
            Some(0),
            vec![pending_event_for_task("continuity_page", task_id)],
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
                for (record_id, updated_at) in [
                    ("continuity_a", "2026-01-02T00:00:00Z"),
                    ("continuity_c", "2026-01-02T00:00:00Z"),
                    ("continuity_b", "2026-01-02T00:00:00Z"),
                    ("continuity_d", "2026-01-01T23:59:59Z"),
                ] {
                    CoreStorageMutation::InsertProjectContinuityRecord(
                        project_continuity_record_insert(
                            record_id,
                            task_id,
                            change_unit_id,
                            updated_at,
                        ),
                    )
                    .apply(mutation, facts.committed_state_version)?;
                }
                Ok(())
            },
            response_json,
        )?;

        let first = store.active_project_continuity_page(2, None)?;
        assert_eq!(first.total_count, 4);
        assert!(first.truncated);
        assert_eq!(
            first
                .records
                .iter()
                .map(|record| record.continuity_record_id.as_str())
                .collect::<Vec<_>>(),
            vec!["continuity_c", "continuity_b"]
        );
        let last = first.records.last().expect("first page cursor source");
        let cursor = ContinuityCursor {
            updated_at: UtcTimestamp::parse(&last.updated_at)?,
            continuity_record_id: ProjectContinuityRecordId::new(last.continuity_record_id.clone()),
        };
        let second = store.active_project_continuity_page(2, Some(&cursor))?;
        assert_eq!(second.total_count, 4);
        assert!(!second.truncated);
        assert_eq!(
            second
                .records
                .iter()
                .map(|record| record.continuity_record_id.as_str())
                .collect::<Vec<_>>(),
            vec!["continuity_a", "continuity_d"]
        );

        for invalid_page_size in [0, MAX_CONTINUITY_PAGE_SIZE + 1] {
            assert!(matches!(
                store.active_project_continuity_page(invalid_page_size, None),
                Err(StoreError::InvalidInput { .. })
            ));
        }
        let malformed_cursor = ContinuityCursor {
            updated_at: UtcTimestamp::parse("2026-01-02T00:00:00Z")?,
            continuity_record_id: ProjectContinuityRecordId::new("   "),
        };
        assert!(matches!(
            store.active_project_continuity_page(2, Some(&malformed_cursor)),
            Err(StoreError::InvalidInput { .. })
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
            git_workspace_context_json: None,
        }
    }

    fn user_replay_context() -> VerifiedReplayContext {
        VerifiedReplayContext {
            actor_source: "local_user".to_owned(),
            operation_category: "user_only".to_owned(),
            verification_basis: Some("store_test_user_channel".to_owned()),
            git_workspace_context_json: None,
        }
    }

    fn pending_event(marker: &str) -> PendingTaskEvent {
        pending_event_for_task(marker, &format!("task_{marker}"))
    }

    fn pending_event_for_task(marker: &str, task_id: &str) -> PendingTaskEvent {
        PendingTaskEvent {
            event_id: format!("evt_{marker}"),
            task_id: Some(task_id.to_owned()),
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
            requested_control_level: "tracked".to_owned(),
            effective_control_level: "tracked".to_owned(),
            control_level_reason: "Store test control.".to_owned(),
            work_phase: "shaping".to_owned(),
            acceptance_policy: "required".to_owned(),
            acceptance_policy_reason: "Store test policy.".to_owned(),
            predecessor_task_id: None,
            lineage_relation: None,
            lineage_reason: None,
            carry_forward_json: "[]".to_owned(),
            lifecycle_phase: "shaping".to_owned(),
            result: None,
            title: None,
            summary: None,
            shaping_summary_json: "{}".to_owned(),
            bounded_context_json: "[]".to_owned(),
            autonomy_boundary_json: "{}".to_owned(),
            close_summary_json: "{\"close_reason\":\"none\"}".to_owned(),
            current_change_unit_id: None,
        }
    }

    fn evidence_summary_upsert(
        evidence_summary_id: &str,
        task_id: &str,
        updated_by_run_id: &str,
    ) -> EvidenceSummaryUpsert {
        EvidenceSummaryUpsert {
            evidence_summary_id: evidence_summary_id.to_owned(),
            task_id: task_id.to_owned(),
            change_unit_id: None,
            status: "unknown".to_owned(),
            coverage_json: "[]".to_owned(),
            supporting_refs_json: "[]".to_owned(),
            gap_refs_json: "[]".to_owned(),
            metadata_json: json!({ "updated_by_run_id": updated_by_run_id }).to_string(),
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

    fn user_action_request_insert(
        request_id: &str,
        task_id: &str,
        expires_at: Option<&str>,
    ) -> UserActionRequestInsert {
        let request_json = json!({
            "body": {
                "action_type": "choice",
                "judgment_kind": "product_decision",
                "presentation": "short",
                "question": "Choose the current product direction.",
                "options": [{
                    "option_id": "accept",
                    "label": "Accept",
                    "description": "Accept the current direction.",
                    "consequence": "The work may continue.",
                    "machine_action": "accept",
                    "resolution_outcome": "accepted",
                    "is_default": true
                }],
                "context": {
                    "summary": "A bounded choice is required.",
                    "related_refs": [],
                    "artifact_refs": [],
                    "visible_risks": [],
                    "constraints": []
                },
                "affected_refs": [],
                "sensitive_action_scope": null
            },
            "required_for": ["informational"],
            "expires_at": expires_at
        })
        .to_string();
        let basis_json = json!({
            "action_type": "choice",
            "coordinates": {
                "task_id": task_id,
                "change_unit_id": null,
                "scope_revision": 0,
                "baseline_ref": null,
                "created_at_state_version": 0,
                "compatibility_status": "current"
            },
            "close_basis_revision": null,
            "result_refs": [],
            "residual_risk_ids": [],
            "sensitive_action_scope": null
        })
        .to_string();
        UserActionRequestInsert {
            user_action_request_id: request_id.to_owned(),
            task_id: task_id.to_owned(),
            change_unit_id: None,
            action_kind: UserActionKind::ProductDecision,
            request_json,
            basis_json,
            basis_status: UserActionBasisStatus::Current,
            required_for_json: r#"["informational"]"#.to_owned(),
            requested_by_actor_source: ACTOR_SOURCE.to_owned(),
            source_method: MethodName::RequestUserAction.as_str().to_owned(),
            source_idempotency_key: format!("idem_{request_id}"),
            requested_at: "2026-01-01T00:00:00Z".to_owned(),
            expires_at: expires_at.map(str::to_owned),
            metadata_json: "{}".to_owned(),
        }
    }

    fn set_user_action_request_expiry(input: &mut UserActionRequestInsert, expires_at: &str) {
        let mut request_json = serde_json::from_str::<Value>(&input.request_json)
            .expect("test user-action request JSON should decode");
        request_json["expires_at"] = json!(expires_at);
        input.request_json = request_json.to_string();
        input.expires_at = Some(expires_at.to_owned());
    }

    fn evidence_user_action_request_insert(
        request_id: &str,
        task_id: &str,
        produced_at_state_version: u64,
    ) -> UserActionRequestInsert {
        let target = json!({
            "target_kind": "acceptance_criterion",
            "acceptance_criterion_id": "criterion_observation_reread"
        });
        let artifact = user_action_artifact_ref_json(task_id, produced_at_state_version);
        UserActionRequestInsert {
            user_action_request_id: request_id.to_owned(),
            task_id: task_id.to_owned(),
            change_unit_id: None,
            action_kind: UserActionKind::EvidenceObservation,
            request_json: json!({
                "body": {
                    "action_type": "evidence_observation",
                    "question": "Does this artifact support the criterion?",
                    "context_summary": "Review the exact stored artifact bytes.",
                    "target_candidates": [target.clone()],
                    "artifact_candidates": [artifact.clone()]
                },
                "required_for": ["record_run"],
                "expires_at": "2026-01-01T00:15:00Z"
            })
            .to_string(),
            basis_json: json!({
                "action_type": "evidence_observation",
                "coordinates": {
                    "task_id": task_id,
                    "change_unit_id": null,
                    "scope_revision": 0,
                    "baseline_ref": null,
                    "created_at_state_version": 0,
                    "compatibility_status": "current"
                },
                "target_candidates": [target],
                "artifact_candidates": [artifact]
            })
            .to_string(),
            basis_status: UserActionBasisStatus::Current,
            required_for_json: r#"["record_run"]"#.to_owned(),
            requested_by_actor_source: ACTOR_SOURCE.to_owned(),
            source_method: MethodName::RequestUserAction.as_str().to_owned(),
            source_idempotency_key: format!("idem_{request_id}"),
            requested_at: "2026-01-01T00:00:00Z".to_owned(),
            expires_at: Some("2026-01-01T00:15:00Z".to_owned()),
            metadata_json: "{}".to_owned(),
        }
    }

    fn evidence_user_action_resolution_insert(
        resolution_id: &str,
        request_id: &str,
        task_id: &str,
        produced_at_state_version: u64,
    ) -> UserActionResolutionInsert {
        UserActionResolutionInsert {
            user_action_resolution_id: resolution_id.to_owned(),
            user_action_request_id: request_id.to_owned(),
            action_kind: UserActionKind::EvidenceObservation,
            channel_kind: UserActionChannelKind::Cli,
            channel_submission_id: format!("submission_{resolution_id}"),
            resolution_json: json!({
                "resolution_type": "evidence_observation",
                "observation": {
                    "target": {
                        "target_kind": "acceptance_criterion",
                        "acceptance_criterion_id": "criterion_observation_reread"
                    },
                    "relevance_status": "supported",
                    "output_artifact_refs": [user_action_artifact_ref_json(
                        task_id,
                        produced_at_state_version
                    )],
                    "summary": "The exact artifact bytes support the criterion."
                }
            })
            .to_string(),
            resolved_by_actor_source: "local_user".to_owned(),
            resolved_verification_basis: "cli_direct_user_channel".to_owned(),
            resolved_assurance_level: "local_user_channel".to_owned(),
            resolved_at: "2026-01-01T00:10:00Z".to_owned(),
        }
    }

    fn user_action_artifact_ref_json(task_id: &str, produced_at_state_version: u64) -> Value {
        json!({
            "artifact_id": "artifact_observation_reread",
            "project_id": PROJECT_ID,
            "task_id": task_id,
            "display_name": "observation.json",
            "content_type": "application/json",
            "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "size_bytes": 64,
            "integrity_status": "verified",
            "redaction_state": "none",
            "availability": "available",
            "created_by_run_ref": {
                "record_kind": "run",
                "record_id": "run_observation_reread",
                "project_id": PROJECT_ID,
                "task_id": task_id,
                "produced_at_state_version": produced_at_state_version
            },
            "created_by_actor_source": ACTOR_SOURCE,
            "storage_ref": "artifact-storage://observation-reread"
        })
    }

    fn user_action_resolution_insert(
        resolution_id: &str,
        request_id: &str,
    ) -> UserActionResolutionInsert {
        UserActionResolutionInsert {
            user_action_resolution_id: resolution_id.to_owned(),
            user_action_request_id: request_id.to_owned(),
            action_kind: UserActionKind::ProductDecision,
            channel_kind: UserActionChannelKind::Cli,
            channel_submission_id: format!("submission_{resolution_id}"),
            resolution_json: json!({
                "resolution_type": "choice",
                "selected_option_id": "accept",
                "machine_action": "accept",
                "resolution_outcome": "accepted",
                "note": null,
                "accepted_risk_ids": []
            })
            .to_string(),
            resolved_by_actor_source: "local_user".to_owned(),
            resolved_verification_basis: "cli_direct_user_channel".to_owned(),
            resolved_assurance_level: "local_user_channel".to_owned(),
            resolved_at: "2026-01-01T00:10:00Z".to_owned(),
        }
    }

    fn choice_resolution_json(
        selected_option_id: &str,
        machine_action: UserActionOptionAction,
        resolution_outcome: JudgmentResolutionOutcome,
    ) -> String {
        json!({
            "resolution_type": "choice",
            "selected_option_id": selected_option_id,
            "machine_action": machine_action,
            "resolution_outcome": resolution_outcome,
            "note": null,
            "accepted_risk_ids": []
        })
        .to_string()
    }

    fn project_continuity_record_insert(
        continuity_record_id: &str,
        task_id: &str,
        change_unit_id: &str,
        updated_at: &str,
    ) -> ProjectContinuityRecordInsert {
        ProjectContinuityRecordInsert {
            continuity_record_id: continuity_record_id.to_owned(),
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
            created_at: updated_at.to_owned(),
            updated_at: updated_at.to_owned(),
            metadata_json: json!({"source": "store_test"}).to_string(),
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
