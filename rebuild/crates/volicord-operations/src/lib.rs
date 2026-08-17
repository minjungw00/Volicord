//! Reconstruction-owned local orchestration and user/operator CLI surface.
//!
//! This crate owns local runtime layout, operation observation, and sequencing.
//! It deliberately delegates canonical, analysis, privacy, inquiry, projection,
//! and portable-format meaning to their existing subsystem owners.

mod cli;
mod codex;
mod error;
mod guarded;
mod layout;
mod model;
mod operations;

pub use cli::{run_cli, run_cli_with_input, CliExit};
pub use error::Error;
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
    AnalysisOutcome, BindingOutcome, CandidateRepositoryResearchDraft, CanonicalMutationOutcome,
    ChildProcessOutcome, CommandVerificationDraft, GroundedCheckpointDraft,
    GroundedCheckpointOutcome, HealthIssue, HealthIssueKind, HealthReport, HealthState,
    LongOperationResult, OperationState, PartialOutcome, ProgressState, ProjectInitialization,
    ProjectResolution, PublicationOutcome, RepairKind, RepairOutcome, UserContextRecordingOutcome,
};
pub use operations::LocalOperations;
pub use volicord_privacy::{
    FilterOutcome, ProviderRequestId, ProviderRequestOutcome, ProviderRequestRecord, ScopeOutcome,
    SourceClass, TransmissionOutcome,
};
pub use volicord_repository_intelligence::AnalysisSnapshotId;
