//! Reconstruction-owned local orchestration and user/operator CLI surface.
//!
//! This crate owns local runtime layout, operation observation, and sequencing.
//! It deliberately delegates canonical, analysis, privacy, inquiry, projection,
//! and portable-format meaning to their existing subsystem owners.

mod cli;
mod codex;
mod error;
mod forgetting;
mod guarded;
mod layout;
mod model;
mod operations;
mod provider;

pub use cli::{run_cli, run_cli_with_input, CliExit};
pub use error::Error;
pub use forgetting::ForgettingState;
pub use guarded::{
    BackgroundProviderDispatcher, BackgroundProviderOperationDraft, ConfirmationDecision,
    ConfirmationRejection, ConfirmationRequestId, ConfirmationResponse, ConfirmationResponseId,
    DispatchExpectation, DispatchObservation, GuardedEffectCandidate, GuardedEffectCategory,
    GuardedEffectDispatcher, GuardedEffectDraft, GuardedOperationId, GuardedOperationOutcome,
    GuardedOperationResult, GuardedProviderInspection, GuardedProviderPreparation,
    GuardedProviderPreparationOutcome, GuardedRisk, GuardedStore, RequestingProvenance,
};
pub use layout::RuntimeLayout;
pub use model::{
    bounded_repository_analysis_json, AnalysisOutcome, BindingOutcome,
    CandidateRepositoryResearchDraft, CanonicalMutationOutcome, CheckpointScopeViolation,
    ChildProcessOutcome, CommandVerificationDraft, EngineeringChoiceDiscoveryDraft,
    EngineeringChoiceDiscoveryOutcome, ForgettingOutcome, GroundedCheckpointDraft,
    GroundedCheckpointOutcome, HealthIssue, HealthIssueKind, HealthReport, HealthState,
    LearningDeliberationDraft, LearningDeliberationOutcome, LearningFeedbackDraft,
    LearningReconsiderationDraft, LearningResponseDraft, LongOperationResult,
    MaterialityReviewDraft, MaterialityReviewOutcome, MaterialityReviewRevisionDraft,
    OperationState, PartialOutcome, ProgressState, ProjectInitialization, ProjectResolution,
    PublicationOutcome, RepairKind, RepairOutcome, UserContextRecordingOutcome, WorkflowAction,
    WorkflowBasisIdentity, WorkflowDirective, WorkflowDisposition, WorkflowRequirement,
    WorkflowStage,
};
pub use operations::LocalOperations;
pub use provider::{
    CodexCliProviderConfig, CodexCliSemanticProvider, CODEX_CLI_PROVIDER, CODEX_EXECUTABLE_ENV,
};
pub use volicord_inquiry::{
    BehavioralContextBasis, CoupledArtifactAssessment, CoupledArtifactCategory,
    CoupledArtifactDisposition, CoupledArtifactReview, DiscoveredAlternativeAccounting,
    DiscoveredAlternativeResolution, EngineeringAlternative, EngineeringChoice,
    EngineeringChoiceEvidenceState, EngineeringChoiceRelationship, EngineeringEffectCategory,
    ExactAuthoritySufficiency, ExplicitDelegationEvidence, ExploratoryDisposition,
    LearningAlternativeSelection, LearningDeliberationState, LearningInitialResponse,
    LearningParticipation, LearningRecommendation, LearningValueAssessment, LearningValueRevision,
    LearningValueRevisionBasis, LearningValueRevisionRequest, MaterialBoundaryConclusion,
    MaterialBoundaryReview, MaterialOutcomeOwnershipAssessment, MaterialOutcomeSignal,
    MaterialityDimension, MaterialityDisposition, WorkAuthorityAction, WorkAuthorityBasis,
    WorkAuthorityBasisKind, WorkAuthorityCandidateBasis, WorkAuthorityDisposition,
    WorkAuthorityRequirement, WorkAuthorityResult, WorkAuthorityStage,
};
pub use volicord_privacy::{
    FilterOutcome, ProviderRequestId, ProviderRequestOutcome, ProviderRequestRecord, ScopeOutcome,
    SourceClass, TransmissionOutcome,
};
pub use volicord_repository_intelligence::AnalysisSnapshotId;
