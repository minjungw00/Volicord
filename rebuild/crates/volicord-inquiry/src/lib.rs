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
mod work_authority;

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
    CandidateCleanup, CandidateCleanupKind, CandidateCollectionMode, CandidateCollectionScope,
    CandidateContent, CandidateDisposition, CandidateDraft, CandidateFreshness, CandidateKind,
    CandidateObservationBasis, CandidateOrigin, CandidateReadBasis, CandidateRecord,
    CandidateRetention, CollectionOptOut, CollectionOptOutScope, DuplicateAssessment,
    EngineeringAlternative, EngineeringChoice, EngineeringChoiceDiscovery,
    EngineeringChoiceEvidenceState, EngineeringChoiceRelationship, EngineeringEffectCategory,
    ExplicitDelegationEvidence, ExploratoryDisposition, InquiryScope, LearningAlternativeSelection,
    LearningDeliberation, LearningDeliberationRound, LearningDeliberationState,
    LearningInitialResponse, LearningParticipation, LearningRecommendation,
    LearningValueAssessment, MaterialOutcomeSignal, MaterialityAssessment, MaterialityDimension,
    MaterialityDisposition, MaterialityReview, MaterialityReviewRevision, MaterialityStatus,
    PromotionResult, QuestionCandidate, QuestionPresentation, RepositoryResearchBasis,
    SubmissionOutcome, WorkAuthorityBasis, WorkAuthorityBasisKind,
};
pub use response::{
    interpret_current_host_response, record_response_batch, BatchResponseItem,
    BatchResponseOutcome, BatchResponseResult, CurrentHostResponse, DisplayedQuestion,
    ResponseInterpretation, ResponseMapping, ResponseRejection,
};
pub use store::{
    resolve_question_by_research, CandidateStore, CANDIDATE_SCHEMA_KIND, CANDIDATE_SCHEMA_VERSION,
};
pub use work_authority::{
    bind_question_candidate_to_materiality, evaluate_work_authority, materiality_scope_token,
    WorkAuthorityAction, WorkAuthorityCandidateBasis, WorkAuthorityDisposition,
    WorkAuthorityRequirement, WorkAuthorityResult, WorkAuthorityStage,
};
