//! Staged Inquiry and local Session Candidate responsibility.
//!
//! Candidate persistence is physically separate from Canonical Context. This
//! crate may submit validated canonical intents to `volicord-context`, but it
//! never stores a second authoritative Question or Decision.

mod applicability;
mod checkpoint;
mod error;
mod frontier;
mod identity;
mod model;
mod response;
mod store;

pub use applicability::{
    evaluate_decision_applicability, mark_review_due, propose_requestioning, ApplicabilityIssue,
    ApplicabilityQuery, DecisionApplicability, DecisionApplicabilityState, DecisionBasisSummary,
    RequestioningProposal, ReviewDueIntent,
};
pub use checkpoint::{
    attribute_repository_changes, evaluate_checkpoint_candidate, record_checkpoint,
    ChangeAttribution, CheckpointCandidate, CheckpointEvaluation, CheckpointRejection,
    RepositoryWorkBasis,
};
pub use error::{Error, ErrorKind};
pub use frontier::{
    compute_frontier, recompute_frontier_for_resume, FrontierDiagnostic, FrontierDiagnosticKind,
    FrontierRead, ResumeFrontier,
};
pub use identity::CandidateId;
pub use model::{
    CandidateCleanupKind, CandidateCollectionMode, CandidateCollectionScope, CandidateContent,
    CandidateDisposition, CandidateDraft, CandidateFreshness, CandidateKind,
    CandidateObservationBasis, CandidateOrigin, CandidateReadBasis, CandidateRecord,
    CandidateRetention, CollectionOptOut, CollectionOptOutScope, DuplicateAssessment, InquiryScope,
    MaterialityAssessment, MaterialityStatus, PromotionResult, QuestionCandidate,
    QuestionPresentation, RepositoryResearchBasis, SubmissionOutcome,
};
pub use response::{
    interpret_current_host_response, record_response_batch, BatchResponseItem,
    BatchResponseOutcome, BatchResponseResult, CurrentHostResponse, DisplayedQuestion,
    ResponseInterpretation, ResponseMapping, ResponseRejection,
};
pub use store::{
    resolve_question_by_research, CandidateStore, CANDIDATE_SCHEMA_KIND, CANDIDATE_SCHEMA_VERSION,
};
