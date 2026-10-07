//! Read-only projections over canonical, Candidate, and Repository
//! Intelligence read bases.
//!
//! This crate has no mutation handle. Automatic Recall trigger state is local
//! to one in-memory agent session and never enters canonical storage.

mod code_behavior;
pub use code_behavior::{CodeBehaviorClaim, CodeBehaviorReading, CodeExplanationState};
mod answers;
pub use answers::*;
mod analysis_status;
pub use analysis_status::*;
mod candidate_inspection;
mod documents;
mod explanation;
mod project;
mod reading;
mod recall;
mod selection;
mod trigger;
mod understanding;
pub use explanation::*;

pub use candidate_inspection::{
    build_learning_explanation_basis, inspect_candidate, learning_resume_projection,
    CandidateContentAccess, CandidateContentOmission, CandidateInspection, InspectionHealth,
    LearningExplanationAlternative, LearningExplanationAvailability, LearningExplanationBasis,
    LearningResumeItem, LearningResumeProjection, LearningSelectionOutcome, RetentionInspection,
};
pub use documents::{
    generate_documents, prepare_narrative_plan, realize_narrative, user_acceptance_label,
    user_review_label, verification_state_label, work_state_label,
    work_state_label_from_understanding, ClaimClass, DocumentBody, DocumentDecisionBasis,
    DocumentError, DocumentKind, DocumentMetadata, DocumentRequest, DocumentSection,
    DocumentSectionRole, DocumentSet, DocumentSourceBasis, FixedLocale, GeneratedDocument,
    GeneratedDocumentClaim, GeneratorIdentity, NarrativePlan, NarrativePlanClaim,
    NarrativePlanSection, NarrativeRealization, NarrativeRealizationState,
    NarrativeSourceTextOmission, OutputFormat, PublicationArtifact, RealizedNarrativeClaim,
    RealizedNarrativeSection, RequestedDestination, GENERATED_DOCUMENT_FORMAT_KIND,
    GENERATED_DOCUMENT_METADATA_VERSION, NARRATIVE_PLAN_PROTECTED_TERM_BYTE_LIMIT,
    NARRATIVE_PLAN_PROTECTED_TERM_LIMIT, NARRATIVE_PLAN_SOURCE_TEXT_BYTE_LIMIT,
    RENDERED_DOCUMENT_FIELD_BYTE_LIMIT, RENDERED_HTML_BYTE_LIMIT, RENDERED_MARKDOWN_BYTE_LIMIT,
};
pub use project::{
    build_canonical_inspection, build_memory_inspection, build_project_projection,
    CandidateDependencyFailure, CandidateDependencyFailureKind, CandidateDependencyState,
    CandidateProjectionInput, CanonicalInspectionItem, CanonicalInspectionKind, CapabilityGap,
    CheckpointTimelineEntry, CodeRelationshipRole, CurrentWorkCodeLink, CurrentWorkPathBasis,
    CurrentWorkTopology, DecisionContextCodeLink, MapEntity, MapInterpretation, MapRelation,
    MapRelationClass, MemoryInspectionProjection, ProjectOverview, ProjectProjection,
    ProjectProjectionInputs, ProjectReadSections, ProjectionBound, ProjectionDetail,
    ProjectionHealth, ProjectionIssue, ProjectionIssueKind, ProjectionReadRequirements,
    ReadSectionState, RepositoryMap, RepositoryScopeMetadata, SourceStatusSummary, WorkReadCost,
};
pub use recall::{
    build_resume_brief, build_resume_brief_from_metadata, BriefContextItem, BriefDecision,
    BriefDecisionState, BriefQuestion, BriefSnapshot, OmissionReason, RecallBound, RecallInputs,
    RecallMetadataInputs, RecallOmission, RecallProposal, ResumeBrief,
};
pub use trigger::{RecallTriggerOutcome, SessionRecallTrigger};
pub use understanding::{
    build_project_understanding, ArchitectureFlowEvidence, ArchitectureFlowState,
    ProjectUnderstanding, UnderstandingArchitecture, UnderstandingArchitectureSelection,
    UnderstandingArchitectureSelectionBasis, UnderstandingBound, UnderstandingDecision,
    UnderstandingEvidence, UnderstandingEvidenceClass, UnderstandingExplanation,
    UnderstandingExplanationKind, UnderstandingOmission, UnderstandingWork, UnderstandingWorkState,
    UnresolvedWorkGrouping, WorkOverview, WorkSection,
};

pub use reading::{
    DecisionReading, ReadingAvailability, ReadingBasis, ReadingRecord, ReadingRepresentation,
    ReadingSourceStatus, ReadingText, WorkAnswers, WorkCodeGap, WorkReading, WorkStateObservation,
    READING_TEXT_CHARACTER_LIMIT,
};
pub use selection::{WorkSelection, WorkSelectionBasis, WorkSelectionError, WorkSelector};
