use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use volicord_context::{
    CheckpointId, CheckpointKind, CommandTermination, ContextItemId, ContextItemRole, DecisionId,
    LocalBinding, OperationId, Project, SourceId, VerificationState, WorkState,
};
use volicord_inquiry::{
    CandidateFreshness, CandidateId, EngineeringChoice, LearningDeliberationState,
    LearningInitialResponse, LearningParticipation, LearningRecommendation,
    LearningValueRevisionRequest, MaterialityDimension,
};
use volicord_local_platform::{
    ProcessCompletion, ProcessStopTrigger, ProcessStreamArtifact, ProcessTermination,
    ProcessTreeCleanup,
};
use volicord_privacy::ProviderDeletionOutcome;
use volicord_repository_intelligence::{
    AnalysisSnapshot, CapabilityState, DiagnosticSeverity, RepositorySnapshot,
};
use volicord_repository_intelligence::{AnalysisSnapshotId, RepositorySnapshotId};

use crate::ForgettingState;

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
    pub partial_scopes: Vec<String>,
    pub failed_scopes: Vec<String>,
    pub omitted_scopes: Vec<String>,
}

pub fn bounded_repository_analysis_json(analysis: &AnalysisSnapshot) -> Value {
    const DIAGNOSTICS_PER_CAPABILITY: usize = 3;
    const DIAGNOSTIC_LIMIT: usize = 64;

    let diagnostics_by_id = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| (diagnostic.identity.as_str(), diagnostic))
        .collect::<BTreeMap<_, _>>();
    let mut selected_ids = BTreeSet::new();
    let capability_reports = analysis
        .capabilities
        .iter()
        .map(|report| {
            let mut representative_ids = Vec::new();
            let mut representative_causes = BTreeSet::new();
            for severity in [
                DiagnosticSeverity::Error,
                DiagnosticSeverity::Warning,
                DiagnosticSeverity::Information,
            ] {
                for identity in &report.diagnostics {
                    let Some(diagnostic) = diagnostics_by_id.get(identity.as_str()) else {
                        continue;
                    };
                    if diagnostic.severity != severity
                        || !representative_causes
                            .insert((diagnostic.code.clone(), diagnostic.message.clone()))
                        || representative_ids.len() >= DIAGNOSTICS_PER_CAPABILITY
                        || selected_ids.len() >= DIAGNOSTIC_LIMIT
                    {
                        continue;
                    }
                    representative_ids.push(identity.clone());
                    selected_ids.insert(identity.clone());
                }
            }
            let (recovery_owner, safe_next_action) = capability_recovery(report.state);
            json!({
                "capability":report.capability,
                "language":report.language,
                "area":report.area,
                "state":report.state,
                "reason":report.reason,
                "usable_remainder":report.usable_remainder,
                "user_visible_consequence":report.user_visible_consequence,
                "coverage":{
                    "included_count":report.coverage.included.len(),
                    "excluded_count":report.coverage.excluded.len(),
                    "unsupported_count":report.coverage.unsupported.len(),
                    "unavailable_count":report.coverage.unavailable.len(),
                    "failed_count":report.coverage.failed.len(),
                    "stale_count":report.coverage.stale.len(),
                    "covered_file_count":report.coverage.covered_file_count,
                    "covered_entity_count":report.coverage.covered_entity_count,
                    "covered_relation_count":report.coverage.covered_relation_count,
                },
                "diagnostic_count":report.diagnostics.len(),
                "diagnostic_ids":representative_ids,
                "adapter":report.adapter,
                "analyzer":report.analyzer,
                "provenance_class":report.provenance_class,
                "freshness":report.freshness,
                "uncertainty":report.uncertainty,
                "recovery_owner":recovery_owner,
                "safe_next_action":safe_next_action,
            })
        })
        .collect::<Vec<_>>();
    let diagnostics = selected_ids
        .iter()
        .filter_map(|identity| diagnostics_by_id.get(identity.as_str()))
        .map(|diagnostic| json!(diagnostic))
        .collect::<Vec<_>>();
    json!({
        "capability_reports":capability_reports,
        "diagnostics_omitted_count":analysis.diagnostics.len().saturating_sub(diagnostics.len()),
        "diagnostics":diagnostics,
    })
}

fn capability_recovery(state: CapabilityState) -> (Option<&'static str>, Option<&'static str>) {
    match state {
        CapabilityState::Available => (None, None),
        CapabilityState::Partial => (
            Some("repository_intelligence"),
            Some("Use the reported usable remainder now; inspect linked diagnostics and rerun repository_analyze after correcting affected source or analyzer limits."),
        ),
        CapabilityState::Failed => (
            Some("repository_intelligence"),
            Some("Keep inventory and unaffected capability results; correct the reported source or analyzer failure, then rerun repository_analyze."),
        ),
        CapabilityState::Unavailable => (
            Some("repository_intelligence"),
            Some("Use available inventory or structural evidence; satisfy the reported prerequisite before retrying this capability."),
        ),
        CapabilityState::Unsupported => (
            Some("repository_intelligence"),
            Some("Use the supported capability results; do not retry this capability until Repository Intelligence support is added."),
        ),
        CapabilityState::Stale => (
            Some("repository_intelligence"),
            Some("Rerun repository_analyze against the current repository state."),
        ),
    }
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
pub enum ProjectResolution {
    Found {
        project: Project,
        binding: BindingOutcome,
    },
    NotFound {
        canonical_repository_path: PathBuf,
    },
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterialityReviewDraft {
    pub project_id: volicord_context::ProjectId,
    pub goal_context_id: ContextItemId,
    pub baseline_analysis_snapshot_id: AnalysisSnapshotId,
    pub session: String,
    pub source_operation: String,
    pub rationale: String,
    pub learning_participation: LearningParticipation,
    pub engineering_choice_discovery_candidate_id: CandidateId,
    pub dimensions: Vec<MaterialityDimension>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineeringChoiceDiscoveryDraft {
    pub project_id: volicord_context::ProjectId,
    pub goal_context_id: ContextItemId,
    pub baseline_analysis_snapshot_id: AnalysisSnapshotId,
    pub session: String,
    pub source_operation: String,
    pub summary: String,
    pub choices: Vec<EngineeringChoice>,
    pub material_boundary_review: Vec<volicord_inquiry::MaterialBoundaryReview>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineeringChoiceDiscoveryOutcome {
    pub discovery_candidate_id: CandidateId,
    pub goal_context_id: ContextItemId,
    pub baseline_analysis_snapshot_id: AnalysisSnapshotId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterialityReviewRevisionDraft {
    pub project_id: volicord_context::ProjectId,
    pub review_candidate_id: CandidateId,
    pub rationale: String,
    pub learning_participation: LearningParticipation,
    pub dimensions: Vec<MaterialityDimension>,
    pub learning_value_revision_bases: Vec<LearningValueRevisionRequest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearningDeliberationDraft {
    pub project_id: volicord_context::ProjectId,
    pub review_candidate_id: CandidateId,
    pub dimension_id: String,
    pub session: String,
    pub source_operation: String,
    pub problem: String,
    pub established_facts: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearningResponseDraft {
    pub project_id: volicord_context::ProjectId,
    pub deliberation_candidate_id: CandidateId,
    pub host: String,
    pub session: String,
    pub user_turn: String,
    pub response: LearningInitialResponse,
    pub user_rationale: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearningFeedbackDraft {
    pub project_id: volicord_context::ProjectId,
    pub deliberation_candidate_id: CandidateId,
    pub feedback: String,
    pub recommendation: LearningRecommendation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearningReconsiderationDraft {
    pub project_id: volicord_context::ProjectId,
    pub deliberation_candidate_id: CandidateId,
    pub host: String,
    pub session: String,
    pub user_turn: String,
    pub rationale: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearningDeliberationOutcome {
    pub deliberation_candidate_id: CandidateId,
    pub revision: u64,
    pub state: LearningDeliberationState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterialityReviewOutcome {
    pub review_candidate_id: CandidateId,
    pub review_revision: u64,
    pub goal_context_id: ContextItemId,
    pub baseline_analysis_snapshot_id: AnalysisSnapshotId,
    pub review_analysis_snapshot_id: AnalysisSnapshotId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkflowStage {
    ProjectResolution,
    ProjectInitialization,
    Recall,
    Goal,
    RepositoryBaseline,
    EngineeringChoiceDiscovery,
    MaterialityReview,
    LearningDeliberation,
    ResearchOrPrototype,
    QuestionCandidate,
    Inquiry,
    Decision,
    ReadyForWork,
    Checkpoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkflowDisposition {
    ProjectNotFound,
    RecallRequired,
    GoalRequired,
    BaselineRequired,
    EngineeringChoiceDiscoveryRequired,
    ReviewMissing,
    ReviewInvalid,
    ExecutableScopeRequired,
    LearningDeliberationPending,
    ResearchRequired,
    QuestionRequired,
    CandidateResearchRequired,
    CandidatePromotionRequired,
    UserResponseRequired,
    ReviewRevisionRequired,
    ReadyForWork,
    CheckpointRecorded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkflowAction {
    pub tool: String,
    pub action: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkflowBasisIdentity {
    pub kind: String,
    pub identity: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkflowRequirement {
    pub dimension_id: Option<String>,
    pub reason: String,
    pub basis_identities: Vec<WorkflowBasisIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkflowDirective {
    pub stage: WorkflowStage,
    pub disposition: WorkflowDisposition,
    pub required_next_action: Option<WorkflowAction>,
    pub blocks_ordinary_work: bool,
    pub reason: String,
    pub satisfied_basis_identities: Vec<WorkflowBasisIdentity>,
    pub unresolved_requirements: Vec<WorkflowRequirement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointScopeViolation {
    pub mismatch: volicord_inquiry::WorkScopeMismatch,
    pub review_candidate_id: Option<CandidateId>,
    pub review_revision: Option<u64>,
    pub workflow: WorkflowDirective,
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
    pub forgetting_available: bool,
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
pub struct ForgettingOutcome {
    pub operation_id: OperationId,
    pub record_kind: String,
    pub identity: String,
    pub state: ForgettingState,
    pub canonical_committed: bool,
    pub candidate_cleanup_completed: bool,
    pub managed_derived_cleanup_completed: bool,
    pub residue_verified: bool,
    pub replayed: bool,
    pub provider_deletion: ProviderDeletionOutcome,
    pub diagnostic: Option<String>,
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
    pub command_invocation: Option<String>,
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
    pub pre_existing_dirty_paths: Vec<String>,
    pub changed_paths: Vec<String>,
    pub applied_decisions: Vec<DecisionId>,
    pub verification_source_ids: Vec<SourceId>,
}
