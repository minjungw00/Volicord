use std::path::PathBuf;
use volicord_context::{
    CheckpointId, CheckpointKind, CommandTermination, ContextItemId, ContextItemRole, DecisionId,
    LocalBinding, OperationId, Project, SourceId, VerificationState, WorkState,
};
use volicord_inquiry::CandidateFreshness;
use volicord_local_platform::{
    ProcessCompletion, ProcessStopTrigger, ProcessStreamArtifact, ProcessTermination,
    ProcessTreeCleanup,
};
use volicord_repository_intelligence::{AnalysisSnapshot, RepositorySnapshot};
use volicord_repository_intelligence::{AnalysisSnapshotId, RepositorySnapshotId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationState {
    Pending,
    Running,
    Partial,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Indeterminate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressState {
    pub phase: String,
    pub unit: Option<String>,
    pub completed: u64,
    pub total: Option<u64>,
    pub active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartialOutcome {
    pub completed_scopes: Vec<String>,
    pub failed_scopes: Vec<String>,
    pub omitted_scopes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LongOperationResult<T> {
    pub operation_id: OperationId,
    pub requested_scope: Vec<String>,
    pub state: OperationState,
    pub started_at_unix_micros: i64,
    pub ended_at_unix_micros: i64,
    pub duration_micros: u64,
    pub progress: ProgressState,
    pub partial: PartialOutcome,
    pub value: Option<T>,
    pub diagnostic: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildProcessOutcome {
    pub completion: ProcessCompletion,
    pub timeout_detected: bool,
    pub cancellation_requested: bool,
    pub termination: ProcessTermination,
    pub cleanup: ProcessTreeCleanup,
    pub stdout: ProcessStreamArtifact,
    pub stderr: ProcessStreamArtifact,
}

impl ChildProcessOutcome {
    pub(crate) fn from_observation(
        observation: &volicord_local_platform::ProcessObservation,
    ) -> Self {
        let termination = match observation.completion() {
            ProcessCompletion::Exited(value)
            | ProcessCompletion::ObservationFailed {
                termination: value, ..
            } => *value,
        };
        Self {
            completion: observation.completion().clone(),
            timeout_detected: observation.stop_trigger() == Some(ProcessStopTrigger::Timeout),
            cancellation_requested: observation.stop_trigger()
                == Some(ProcessStopTrigger::Cancellation),
            termination,
            cleanup: observation.cleanup().clone(),
            stdout: observation.stdout().clone(),
            stderr: observation.stderr().clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectInitialization {
    pub project: Project,
    pub binding: Option<BindingOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingOutcome {
    pub binding: LocalBinding,
    pub clone_identity: Option<String>,
    pub worktree_identity: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisOutcome {
    pub repository: RepositorySnapshot,
    pub analysis: AnalysisSnapshot,
    pub stored_at: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateRepositoryResearchDraft {
    pub capability: String,
    pub coverage: String,
    pub freshness: CandidateFreshness,
    pub source_basis: Vec<SourceId>,
    pub sufficient: bool,
    pub limits: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthState {
    Healthy,
    Degraded,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthIssueKind {
    Unavailable,
    Unsupported,
    Failed,
    Stale,
    Corrupt,
    RepairRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthIssue {
    pub kind: HealthIssueKind,
    pub scope: String,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthReport {
    pub state: HealthState,
    pub runtime_root: PathBuf,
    pub canonical_available: bool,
    pub candidate_available: bool,
    pub privacy_available: bool,
    pub guarded_available: bool,
    pub repository_available: Option<bool>,
    pub issues: Vec<HealthIssue>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepairKind {
    DerivedAnalysisRepair,
    DerivedRebuild,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RepairOutcome {
    pub kind: RepairKind,
    pub affected_scope: String,
    pub diagnosis: String,
    pub discarded_entries: u64,
    pub operation: LongOperationResult<AnalysisOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationOutcome {
    pub destination: PathBuf,
    pub bytes: u64,
    pub durability: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalMutationOutcome {
    pub record_kind: String,
    pub identity: String,
    pub revision: Option<u64>,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserContextRecordingOutcome {
    pub source_id: SourceId,
    pub context_item_id: ContextItemId,
    pub context_item_revision: u64,
    pub role: ContextItemRole,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandVerificationDraft {
    pub state: VerificationState,
    pub command_label: Option<String>,
    pub exit_code: Option<i32>,
    pub termination: Option<CommandTermination>,
    pub outcome: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroundedCheckpointDraft {
    pub project_id: volicord_context::ProjectId,
    pub goal_context_id: ContextItemId,
    pub baseline_analysis_snapshot_id: AnalysisSnapshotId,
    pub kind: CheckpointKind,
    pub work_state: WorkState,
    pub state_change: Option<String>,
    pub applied_decisions: Vec<DecisionId>,
    pub decision_components: Vec<String>,
    pub work_contexts: Vec<String>,
    pub met_revisit_triggers: Vec<String>,
    pub verification: Vec<CommandVerificationDraft>,
    pub known_limits: Vec<String>,
    pub non_goals: Vec<String>,
    pub next_step: String,
    pub handoff_to: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroundedCheckpointOutcome {
    pub checkpoint_id: CheckpointId,
    pub checkpoint_revision: u64,
    pub goal_context_id: ContextItemId,
    pub baseline_analysis_snapshot_id: AnalysisSnapshotId,
    pub current_analysis_snapshot_id: AnalysisSnapshotId,
    pub baseline_repository_snapshot_id: RepositorySnapshotId,
    pub current_repository_snapshot_id: RepositorySnapshotId,
    pub changed_paths: Vec<String>,
    pub applied_decisions: Vec<DecisionId>,
    pub verification_source_ids: Vec<SourceId>,
}
