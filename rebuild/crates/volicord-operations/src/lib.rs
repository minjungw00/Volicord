//! Reconstruction-owned local orchestration and user/operator CLI surface.
//!
//! This crate owns local runtime layout, operation observation, and sequencing.
//! It deliberately delegates canonical, analysis, privacy, inquiry, projection,
//! and portable-format meaning to their existing subsystem owners.

mod cli;
mod error;
mod guarded;
mod layout;
mod model;
mod operations;

pub use cli::{run_cli, CliExit};
pub use error::Error;
pub use guarded::{
    BackgroundProviderDispatcher, ConfirmationDecision, ConfirmationRejection,
    ConfirmationRequestId, ConfirmationResponse, ConfirmationResponseId, DispatchExpectation,
    DispatchObservation, GuardedEffectCandidate, GuardedEffectCategory, GuardedEffectDispatcher,
    GuardedEffectDraft, GuardedOperationId, GuardedOperationOutcome, GuardedOperationResult,
    GuardedRisk, GuardedStore, RequestingProvenance,
};
pub use layout::RuntimeLayout;
pub use model::{
    AnalysisOutcome, BindingOutcome, CanonicalMutationOutcome, ChildProcessOutcome, HealthIssue,
    HealthIssueKind, HealthReport, HealthState, LongOperationResult, OperationState,
    PartialOutcome, ProgressState, ProjectInitialization, PublicationOutcome, RepairKind,
    RepairOutcome,
};
pub use operations::LocalOperations;
