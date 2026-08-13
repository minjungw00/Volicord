//! Parser-independent Production Repository Intelligence.
//!
//! Inventory is local and analyzer-independent. All output remains disposable
//! Derived State bound to typed Canonical Context references and an observed
//! repository source snapshot.

mod identity;
mod inventory;
mod model;
mod search;
mod structural;

pub use identity::{AnalysisSnapshotId, RepositorySnapshotId};
pub use inventory::{inventory_repository, InventoryError, InventoryRequest};
pub use model::{
    canonical_json, AdapterIdentity, AnalysisDiagnostic, AnalysisProvenance, AnalysisSnapshot,
    AnalyzerIdentity, AreaId, AreaKind, CandidateKind, CanonicalProjectRef, CanonicalSourceRef,
    Capability, CapabilityReport, CapabilityState, CodeEntity, CodeEntityKind,
    CoordinateConvention, Coverage, DiagnosticSeverity, Ecosystem, EcosystemObservation,
    EcosystemObservationKind, EntryKind, EvidenceCandidate, ExtensionValue, FileAnalysisBasis,
    FreshnessBasis, FreshnessState, GitObservation, InvalidationCategory, InvalidationRecord,
    InventoryClassification, InventoryEntry, InventorySnapshot, Language, LanguageExtension,
    ObservationBasis, ProvenanceClass, RangeMeaning, RefreshAction, RelationTarget,
    RepositorySnapshot, SearchHit, SearchResultKind, SemanticAnalysisResult, SemanticAnnotation,
    SemanticProvenance, SemanticRelation, SemanticRelationKind, SourcePosition, SourceRange,
    StructuralFact, StructuralProvenance, StructuralRefresh, StructuralRelation,
    StructuralRelationKind, Uncertainty, UncertaintyLevel, UnresolvedTarget,
    ANALYSIS_SNAPSHOT_FORMAT_VERSION, ANALYSIS_SNAPSHOT_KIND,
};
pub use search::search_local;
pub use structural::{analyze_repository, StructuralAnalysisError, StructuralAnalysisRequest};
