pub use crate::evidence_capture::{
    derive_evidence_capture_source_claims, EvidenceCaptureIntentInsert,
    EvidenceCaptureIntentRecord, EvidenceCaptureReceiptInsert, EvidenceCaptureReceiptRecord,
    EvidenceCaptureSourceClaimIdentity, EvidenceCaptureSourceClaimKind,
    EvidenceCaptureSourceClaimRecord, EvidenceProducerInsert, EvidenceProducerRecord,
};

pub use self::commit::commit_input;

/// Pending event supplied by a method-specific commit branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTaskEvent {
    pub event_id: String,
    pub task_id: Option<String>,
    pub change_unit_id: Option<String>,
    pub event_kind: String,
    pub event_payload_json: String,
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
    pub clock_floor: Option<volicord_types::values::UtcTimestamp>,
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

mod agent_sessions;
mod artifacts;
mod blockers;
mod change_units;
pub(crate) mod clock;
mod commit;
mod continuity;
mod enforcement_profile;
mod events;
mod evidence;
mod facade;
mod inspection;
pub(crate) mod mutations;
mod open;
mod project_state;
mod reconciliation;
mod record_refs;
mod replay;
mod runs;
mod tasks;
mod user_action_reader;
mod user_actions;
pub(crate) mod validation;
mod write_tickets;

#[cfg(test)]
mod test_support;

pub use crate::workflow_records::{ProjectWorkflowPolicyMutation, WorkflowPolicyMutation};
pub use artifacts::{
    ArtifactLinkInsert, ArtifactMutation, ArtifactPromotion, ArtifactStagingStatus,
    StoredArtifactRecord, StoredArtifactStagingRecord,
};
pub use change_units::{
    ChangeUnitInsert, ChangeUnitMutation, ChangeUnitRecord, ChangeUnitStatus,
    StoredChangeUnitLifecycle, StoredChangeUnitScopeSummary, StoredChangeUnitWriteBasis,
    StoredGitWorkspaceContext,
};
pub use continuity::{
    ActiveProjectContinuityPage, ContinuityMutation, ProjectContinuityRecordInsert,
    ProjectContinuityRecordRecord, UnrecordedChangeResolutionUpdate,
};
pub use enforcement_profile::ProjectEnforcementProfileRecord;
pub use evidence::{
    EvidenceClaimInsert, EvidenceMutation, EvidenceObservationInsert, EvidenceObservationRecord,
    EvidenceSummaryRecord, EvidenceSummaryUpsert,
};
pub use facade::CoreProjectStore;
pub use inspection::StorageEffectCounts;
pub use mutations::CoreStorageMutation;
pub use project_state::ProjectStateHeader;
pub use reconciliation::{ProductWriteObservationCandidate, ProductWriteObservationSource};
pub use record_refs::StoredRecordRef;
pub use replay::{StoredOperationResult, ToolInvocationRecord, VerifiedReplayContext};
pub use runs::{
    RunInsert, RunMutation, RunObservedChangesRecord, RunRecord, RunStatus, StoredRunMetadata,
    StoredRunSummary, StoredRunWriteTicketEffect, StoredRunWriteTicketEffectKind,
};
pub use tasks::{
    AcceptanceCriteriaReplace, AcceptanceCriterionRecord, AcceptanceCriterionStatus,
    AcceptanceCriterionUpsert, EvidenceClaimRecord, TaskAutonomyBoundary, TaskCloseBasisUpdate,
    TaskCloseUpdate, TaskControlLevelUpdate, TaskInsert, TaskMutation, TaskRecord,
    TaskRevisionRecord, TaskScopeRevisionUpdate, TaskScopeUpdate, TaskShapingFacts,
};
pub use user_action_reader::UserActionStoreReader;
pub use user_actions::{
    effective_user_action_status, StoredUserActionRecordSet, StoredUserActionRequest,
    StoredUserActionResolution, UserActionBasisStatusMark, UserActionBasisUpdate,
    UserActionInvalidation, UserActionMutation, UserActionRequestInsert,
    UserActionResolutionInsert,
};
pub use write_tickets::{
    WriteTicketByIdInvalidation, WriteTicketConsumption, WriteTicketInsert,
    WriteTicketInvalidation, WriteTicketMutation, WriteTicketRecord,
};
