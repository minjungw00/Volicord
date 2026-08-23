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
mod understanding;

pub use candidate_inspection::{
    inspect_candidate, CandidateContentAccess, CandidateContentOmission, CandidateInspection,
    InspectionHealth, RetentionInspection,
};
pub use documents::{
    generate_documents, prepare_narrative_plan, realize_narrative, ClaimClass, DocumentBody,
    DocumentDecisionBasis, DocumentError, DocumentKind, DocumentMetadata, DocumentRequest,
    DocumentSection, DocumentSet, DocumentSourceBasis, FixedLocale, GeneratedDocument,
    GeneratedDocumentClaim, GeneratorIdentity, NarrativePlan, NarrativePlanClaim,
    NarrativePlanSection, NarrativeRealization, NarrativeRealizationState,
    NarrativeSourceTextOmission, OutputFormat, PublicationArtifact, RealizedNarrativeClaim,
    RealizedNarrativeSection, RequestedDestination, GENERATED_DOCUMENT_FORMAT_KIND,
    GENERATED_DOCUMENT_METADATA_VERSION, NARRATIVE_PLAN_PROTECTED_TERM_BYTE_LIMIT,
    NARRATIVE_PLAN_PROTECTED_TERM_LIMIT, NARRATIVE_PLAN_SOURCE_TEXT_BYTE_LIMIT,
    RENDERED_DOCUMENT_FIELD_BYTE_LIMIT, RENDERED_HTML_BYTE_LIMIT, RENDERED_MARKDOWN_BYTE_LIMIT,
};
pub use project::{
    build_project_projection, CandidateDependencyFailure, CandidateDependencyFailureKind,
    CandidateDependencyState, CandidateProjectionInput, CanonicalInspectionItem,
    CanonicalInspectionKind, CapabilityGap, CheckpointTimelineEntry, DecisionContextCodeLink,
    MapEntity, MapInterpretation, MapRelation, MapRelationClass, ProjectOverview,
    ProjectProjection, ProjectProjectionInputs, ProjectionBound, ProjectionHealth, ProjectionIssue,
    ProjectionIssueKind, RepositoryMap, SourceStatusSummary,
};
pub use recall::{
    build_resume_brief, BriefContextItem, BriefDecision, BriefDecisionState, BriefQuestion,
    BriefSnapshot, OmissionReason, RecallBound, RecallInputs, RecallOmission, RecallProposal,
    ResumeBrief,
};
pub use trigger::{RecallTriggerOutcome, SessionRecallTrigger};
pub use understanding::{
    build_project_understanding, ProjectUnderstanding, UnderstandingArchitecture,
    UnderstandingBound, UnderstandingDecision, UnderstandingEvidence, UnderstandingEvidenceClass,
    UnderstandingExplanation, UnderstandingExplanationKind, UnderstandingNextStep,
    UnderstandingOmission, UnderstandingWork,
};
