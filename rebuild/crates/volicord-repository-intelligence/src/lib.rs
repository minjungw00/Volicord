//! Parser-independent Production Repository Intelligence.
//!
//! Inventory is local and analyzer-independent. All output remains disposable
//! Derived State bound to typed Canonical Context references and an observed
//! repository source snapshot.

mod canonical;
mod grounding;
mod identity;
mod inventory;
mod model;
mod search;
mod semantic;
mod structural;

pub use canonical::{
    CanonicalGrounding, CanonicalGroundingError, CanonicalGroundingIssue,
    CanonicalGroundingIssueKind,
};
pub use grounding::grounded_explanation_basis;
pub use identity::{AnalysisSnapshotId, RepositorySnapshotId};
pub use inventory::{inventory_repository, InventoryError, InventoryRequest};
pub use model::{
    canonical_json, AdapterIdentity, AgentInterpretation, AnalysisDiagnostic, AnalysisProvenance,
    AnalysisSnapshot, AnalyzerIdentity, AreaId, AreaKind, CandidateKind, CanonicalCheckpointRef,
    CanonicalContextItemRef, CanonicalDecisionRef, CanonicalProjectRef, CanonicalReference,
    CanonicalSourceBasis, CanonicalSourceRef, Capability, CapabilityReport, CapabilityState,
    CodeEntity, CodeEntityKind, CoordinateConvention, Coverage, DiagnosticSeverity, Ecosystem,
    EcosystemObservation, EcosystemObservationKind, EntryKind, EvidenceCandidate, ExtensionValue,
    FileAnalysisBasis, FreshnessBasis, FreshnessState, GitObservation, GroundedExplanationBasis,
    GroundingEvidence, GroundingGap, GroundingStatementClass, InvalidationCategory,
    InvalidationRecord, InventoryClassification, InventoryEntry, InventorySnapshot, Language,
    LanguageExtension, ObservationBasis, ProvenanceClass, RangeMeaning, RefreshAction,
    RelationTarget, RepositorySnapshot, SearchHit, SearchResultKind, SemanticAnalysisResult,
    SemanticAnnotation, SemanticProvenance, SemanticRefresh, SemanticRelation,
    SemanticRelationKind, SourcePosition, SourceRange, StructuralFact, StructuralProvenance,
    StructuralRefresh, StructuralRelation, StructuralRelationKind, Uncertainty, UncertaintyLevel,
    UnresolvedTarget, ANALYSIS_SNAPSHOT_FORMAT_VERSION, ANALYSIS_SNAPSHOT_KIND,
};
pub use search::search_local;
pub use semantic::{
    analyze_repository_semantics, CanonicalLinkSelector, SemanticAnalysisError,
    SemanticAnalysisRequest,
};
pub use structural::{analyze_repository, StructuralAnalysisError, StructuralAnalysisRequest};
