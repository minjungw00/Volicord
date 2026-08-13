//! Read-only projections over canonical, Candidate, and Repository
//! Intelligence read bases.
//!
//! This crate has no mutation handle. Automatic Recall trigger state is local
//! to one in-memory agent session and never enters canonical storage.

mod candidate_inspection;
mod recall;
mod trigger;

pub use candidate_inspection::{
    inspect_candidate, CandidateContentAccess, CandidateContentOmission, CandidateInspection,
    InspectionHealth, RetentionInspection,
};
pub use recall::{
    build_resume_brief, BriefContextItem, BriefDecision, BriefDecisionState, BriefQuestion,
    BriefSnapshot, OmissionReason, RecallBound, RecallInputs, RecallOmission, RecallProposal,
    ResumeBrief,
};
pub use trigger::{RecallTriggerOutcome, SessionRecallTrigger};
