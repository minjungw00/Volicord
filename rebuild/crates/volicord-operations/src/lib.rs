//! Reconstruction-owned local orchestration and user/operator CLI surface.
//!
//! This crate owns local runtime layout, operation observation, and sequencing.
//! It deliberately delegates canonical, analysis, privacy, inquiry, projection,
//! and portable-format meaning to their existing subsystem owners.

mod analysis_io;
mod analysis_storage;
mod cli;
mod codex;
mod error;
mod explanation;
mod forgetting;
mod guarded;
mod layout;
mod model;
mod operations;
mod payload;
mod provider;
mod recall;

pub use analysis_storage::{
    AnalysisFileFootprint, AnalysisReachabilityReference, AnalysisSectionFootprint,
    AnalysisStorageFootprint,
};
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
    ChildProcessOutcome, CommandVerificationDraft, EngineeringChoiceAuthoringBasis,
    EngineeringChoiceDiscoveryDraft, EngineeringChoiceDiscoveryOutcome, ForgettingOutcome,
    GroundedCheckpointDraft, GroundedCheckpointOutcome, HealthIssue, HealthIssueKind, HealthReport,
    HealthState, LearningDeliberationDraft, LearningDeliberationOutcome, LearningFeedbackDraft,
    LearningReconsiderationDraft, LearningResponseDraft, LongOperationResult,
    MaterialityReviewDraft, MaterialityReviewOutcome, MaterialityReviewRevisionDraft,
    OperationState, PartialOutcome, ProgressState, ProjectInitialization, ProjectResolution,
    PublicationOutcome, RepairKind, RepairOutcome, UserContextRecordingOutcome, WorkTransition,
    WorkflowAction, WorkflowBasisIdentity, WorkflowDirective, WorkflowDisposition,
    WorkflowRequirement, WorkflowStage,
};
pub use operations::{HealthCheckProfile, LocalOperations, ProjectProjectionProfile};
pub use payload::{
    bounded_read_section, HOST_READ_RESULT_BYTE_BUDGET, HOST_READ_STRUCTURED_BYTE_BUDGET,
};
pub use provider::{
    CodexCliProviderConfig, CodexCliSemanticProvider, CODEX_CLI_PROVIDER, CODEX_EXECUTABLE_ENV,
};
pub use recall::{decision_reading_json, resume_brief_json, work_reading_json};
pub use volicord_inquiry::{
    AuthoritySourceEvidence, AuthoritySourceRole, BehavioralContextBasis,
    CoupledArtifactAssessment, CoupledArtifactCategory, CoupledArtifactDisposition,
    CoupledArtifactReview, DiscoveredAlternativeAccounting, DiscoveredAlternativeResolution,
    EngineeringAlternative, EngineeringChoice, EngineeringChoiceEvidenceState,
    EngineeringChoiceRelationship, EngineeringEffectCategory, ExactAuthoritySufficiency,
    ExplicitDelegationEvidence, ExploratoryDisposition, ImplementationDiscretionCounterfactual,
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
