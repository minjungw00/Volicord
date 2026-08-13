//! Staged Inquiry and local Session Candidate responsibility.
//!
//! Candidate persistence is physically separate from Canonical Context. This
//! crate may submit validated canonical intents to `volicord-context`, but it
//! never stores a second authoritative Question or Decision.

mod error;
mod frontier;
mod identity;
mod model;
mod store;

pub use error::{Error, ErrorKind};
pub use frontier::{compute_frontier, FrontierDiagnostic, FrontierDiagnosticKind, FrontierRead};
pub use identity::CandidateId;
pub use model::{
    CandidateCleanupKind, CandidateCollectionMode, CandidateCollectionScope, CandidateContent,
    CandidateDisposition, CandidateDraft, CandidateFreshness, CandidateKind,
    CandidateObservationBasis, CandidateOrigin, CandidateReadBasis, CandidateRecord,
    CandidateRetention, CollectionOptOut, CollectionOptOutScope, DuplicateAssessment, InquiryScope,
    MaterialityAssessment, MaterialityStatus, PromotionResult, QuestionCandidate,
    QuestionPresentation, RepositoryResearchBasis, SubmissionOutcome,
};
pub use store::{
    resolve_question_by_research, CandidateStore, CANDIDATE_SCHEMA_KIND, CANDIDATE_SCHEMA_VERSION,
};
