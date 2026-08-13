//! Read-only projections over canonical, Candidate, and Repository
//! Intelligence read bases.
//!
//! This crate has no mutation handle. Automatic Recall trigger state is local
//! to one in-memory agent session and never enters canonical storage.

mod candidate_inspection;
mod documents;
mod project;
mod recall;
mod trigger;

pub use candidate_inspection::{
    inspect_candidate, CandidateContentAccess, CandidateContentOmission, CandidateInspection,
    InspectionHealth, RetentionInspection,
};
pub use documents::{
    generate_documents, ClaimClass, DocumentBody, DocumentDecisionBasis, DocumentError,
    DocumentKind, DocumentMetadata, DocumentRequest, DocumentSection, DocumentSet,
    DocumentSourceBasis, FixedLocale, GeneratedDocument, GeneratedDocumentClaim, GeneratorIdentity,
    OutputFormat, PublicationArtifact, RequestedDestination, GENERATED_DOCUMENT_FORMAT_KIND,
    GENERATED_DOCUMENT_METADATA_VERSION,
};
pub use project::{
    build_project_projection, CanonicalInspectionItem, CanonicalInspectionKind, CapabilityGap,
    CheckpointTimelineEntry, DecisionContextCodeLink, MapEntity, MapInterpretation, MapRelation,
    MapRelationClass, ProjectOverview, ProjectProjection, ProjectProjectionInputs, ProjectionBound,
    ProjectionHealth, ProjectionIssue, ProjectionIssueKind, RepositoryMap, SourceStatusSummary,
};
pub use recall::{
    build_resume_brief, BriefContextItem, BriefDecision, BriefDecisionState, BriefQuestion,
    BriefSnapshot, OmissionReason, RecallBound, RecallInputs, RecallOmission, RecallProposal,
    ResumeBrief,
};
pub use trigger::{RecallTriggerOutcome, SessionRecallTrigger};
