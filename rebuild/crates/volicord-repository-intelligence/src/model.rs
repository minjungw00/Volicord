use crate::identity::decode_hex;
use crate::{AnalysisSnapshotId, RepositorySnapshotId};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use volicord_context::{ProjectId, SourceId};

pub const ANALYSIS_SNAPSHOT_KIND: &str = "volicord.repository_analysis";
pub const ANALYSIS_SNAPSHOT_FORMAT_VERSION: u32 = 1;

macro_rules! canonical_reference {
    ($name:ident, $inner:ty) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub $inner);

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, formatter)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                let bytes = decode_hex::<16>(&value).map_err(serde::de::Error::custom)?;
                Ok(Self(<$inner>::from_bytes(bytes)))
            }
        }
    };
}

canonical_reference!(CanonicalProjectRef, ProjectId);
canonical_reference!(CanonicalSourceRef, SourceId);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepositorySnapshot {
    pub format_version: u32,
    pub identity: RepositorySnapshotId,
    pub project: CanonicalProjectRef,
    pub repository_source: CanonicalSourceRef,
    /// A portable locator inside the repository Source, never a local absolute path.
    pub source_boundary: String,
    pub observation_basis: ObservationBasis,
    pub included_areas: Vec<AreaId>,
    pub excluded_areas: Vec<AreaId>,
    pub unavailable_areas: Vec<AreaId>,
    pub observed_at_unix_micros: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservationBasis {
    pub content_fingerprint_sha256: String,
    pub git: Option<GitObservation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GitObservation {
    pub head: String,
    pub reference: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AnalysisSnapshot {
    pub format_kind: String,
    pub format_version: u32,
    pub identity: AnalysisSnapshotId,
    /// Every analysis result is bound to exactly this one Repository Snapshot.
    pub repository_snapshot: RepositorySnapshotId,
    pub project: CanonicalProjectRef,
    pub repository_source: CanonicalSourceRef,
    pub inventory: InventorySnapshot,
    pub capabilities: Vec<CapabilityReport>,
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub structural_facts: Vec<StructuralFact>,
    pub semantic_results: Vec<SemanticAnalysisResult>,
    pub semantic_annotations: Vec<SemanticAnnotation>,
    pub agent_interpretations: Vec<AgentInterpretation>,
    pub generated_at_unix_micros: i64,
    pub freshness: FreshnessBasis,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InventorySnapshot {
    pub entries: Vec<InventoryEntry>,
    pub languages: BTreeSet<Language>,
    pub ecosystem_observations: Vec<EcosystemObservation>,
    pub evidence_candidates: Vec<EvidenceCandidate>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InventoryEntry {
    pub area: AreaId,
    pub entry_kind: EntryKind,
    pub language: Option<Language>,
    pub classifications: BTreeSet<InventoryClassification>,
    pub size_bytes: Option<u64>,
    pub content_sha256: Option<String>,
    pub diagnostic_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryClassification {
    Included,
    Excluded,
    Ignored,
    Vendor,
    Generated,
    Binary,
    Unavailable,
    Source,
    Manifest,
    WorkspaceManifest,
    Configuration,
    Document,
    Test,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "name")]
pub enum Language {
    Java,
    Python,
    JavaScript,
    TypeScript,
    C,
    Cpp,
    Rust,
    Markdown,
    Json,
    Yaml,
    Toml,
    Xml,
    Shell,
    Go,
    OtherText(String),
    UnknownText,
}

impl Language {
    pub const fn is_structural_gate_language(&self) -> bool {
        matches!(
            self,
            Self::Java
                | Self::Python
                | Self::JavaScript
                | Self::TypeScript
                | Self::C
                | Self::Cpp
                | Self::Rust
        )
    }

    pub const fn is_auxiliary_text_format(&self) -> bool {
        matches!(
            self,
            Self::Markdown | Self::Json | Self::Yaml | Self::Toml | Self::Xml | Self::Shell
        )
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AreaId {
    pub kind: AreaKind,
    /// Slash-separated locator relative to the Repository Snapshot boundary.
    pub path: String,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AreaKind {
    Repository,
    Directory,
    File,
    Package,
    Component,
    Workspace,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Inventory,
    AgentAssisted,
    Structural,
    Semantic,
    Ecosystem,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Available,
    Unavailable,
    Unsupported,
    Partial,
    Failed,
    Stale,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CapabilityReport {
    pub repository_snapshot: RepositorySnapshotId,
    pub language: Option<Language>,
    pub area: AreaId,
    pub capability: Capability,
    pub state: CapabilityState,
    pub reason: Option<String>,
    pub usable_remainder: Option<String>,
    pub user_visible_consequence: Option<String>,
    pub coverage: Coverage,
    pub diagnostics: Vec<String>,
    pub adapter: Option<AdapterIdentity>,
    pub analyzer: Option<AnalyzerIdentity>,
    pub provenance_class: ProvenanceClass,
    pub observed_at_unix_micros: i64,
    pub freshness: FreshnessBasis,
    pub uncertainty: Uncertainty,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Coverage {
    pub included: Vec<AreaId>,
    pub excluded: Vec<AreaId>,
    pub unsupported: Vec<AreaId>,
    pub unavailable: Vec<AreaId>,
    pub failed: Vec<AreaId>,
    pub stale: Vec<AreaId>,
    pub covered_file_count: u64,
    pub covered_entity_count: u64,
    pub covered_relation_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AnalysisDiagnostic {
    pub identity: String,
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub affected_area: AreaId,
    pub capability: Capability,
    pub adapter: Option<AdapterIdentity>,
    pub analyzer: Option<AnalyzerIdentity>,
    pub usable_remainder: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Information,
    Warning,
    Error,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AdapterIdentity {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AnalyzerIdentity {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceClass {
    RepositoryObservation,
    StructuralFact,
    SemanticResult,
    SemanticAnnotation,
    AgentInterpretation,
    UserCorrection,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AnalysisProvenance {
    pub class: ProvenanceClass,
    pub repository_snapshot: RepositorySnapshotId,
    pub analysis_snapshot: AnalysisSnapshotId,
    pub adapter: Option<AdapterIdentity>,
    pub analyzer: Option<AnalyzerIdentity>,
    pub source_basis: Vec<CanonicalSourceRef>,
    pub observed_or_generated_at_unix_micros: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FreshnessBasis {
    pub state: FreshnessState,
    pub repository_snapshot: RepositorySnapshotId,
    pub compared_repository_snapshot: Option<RepositorySnapshotId>,
    pub reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessState {
    Current,
    Stale,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Uncertainty {
    pub level: UncertaintyLevel,
    pub reasons: Vec<String>,
}

impl Uncertainty {
    pub const fn none() -> Self {
        Self {
            level: UncertaintyLevel::None,
            reasons: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UncertaintyLevel {
    None,
    Low,
    Medium,
    High,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EcosystemObservation {
    pub ecosystem: Ecosystem,
    pub kind: EcosystemObservationKind,
    pub area: AreaId,
    pub evidence: Vec<AreaId>,
    pub provenance_class: ProvenanceClass,
    pub uncertainty: Uncertainty,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ecosystem {
    Maven,
    Gradle,
    PythonPackage,
    Node,
    TypeScript,
    Cmake,
    CompilationDatabase,
    Cargo,
    GoModules,
    Other(String),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EcosystemObservationKind {
    PackageManifest,
    WorkspaceManifest,
    BuildConfiguration,
    ToolchainConfiguration,
    DependencyManifest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCandidate {
    pub kind: CandidateKind,
    pub name: Option<String>,
    pub area: AreaId,
    pub evidence: Vec<AreaId>,
    pub provenance_class: ProvenanceClass,
    pub uncertainty: Uncertainty,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateKind {
    Package,
    Component,
    EntryPoint,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CodeEntity {
    pub identity: String,
    pub repository_snapshot: RepositorySnapshotId,
    pub analysis_snapshot: AnalysisSnapshotId,
    pub language: Language,
    pub area: AreaId,
    pub kind: CodeEntityKind,
    pub source: CanonicalSourceRef,
    pub source_range: Option<SourceRange>,
    pub display_name: Option<String>,
    pub qualified_name: Option<String>,
    pub diagnostics: Vec<String>,
    pub uncertainty: Uncertainty,
    pub freshness: FreshnessBasis,
    pub extensions: Vec<LanguageExtension>,
    pub canonical_references: Vec<CanonicalSourceRef>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "extension")]
pub enum CodeEntityKind {
    Repository,
    Package,
    Module,
    Namespace,
    File,
    Class,
    Interface,
    Trait,
    Struct,
    Enum,
    Type,
    Function,
    Method,
    Field,
    Test,
    Configuration,
    Document,
    LanguageSpecific(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceRange {
    pub source: CanonicalSourceRef,
    pub repository_snapshot: RepositorySnapshotId,
    pub locator: String,
    pub start: SourcePosition,
    pub end: SourcePosition,
    pub coordinate_convention: CoordinateConvention,
    pub meaning: RangeMeaning,
    pub adapter: AdapterIdentity,
    pub precision_limit: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourcePosition {
    pub line: u64,
    pub column: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateConvention {
    OneBasedUnicodeScalar,
    ZeroBasedUtf8Byte,
    ZeroBasedUtf16CodeUnit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RangeMeaning {
    WholeFile,
    Entity,
    Symbol,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UnresolvedTarget {
    pub display: String,
    pub language: Option<Language>,
    pub locator_hint: Option<String>,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StructuralRelation {
    pub identity: String,
    pub repository_snapshot: RepositorySnapshotId,
    pub analysis_snapshot: AnalysisSnapshotId,
    pub source_entity: String,
    pub target: RelationTarget,
    pub kind: StructuralRelationKind,
    pub supporting_range: Option<SourceRange>,
    pub diagnostics: Vec<String>,
    pub uncertainty: Uncertainty,
    pub freshness: FreshnessBasis,
    pub extensions: Vec<LanguageExtension>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "resolution", content = "target")]
pub enum RelationTarget {
    ResolvedEntity(String),
    Unresolved(UnresolvedTarget),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "extension")]
pub enum StructuralRelationKind {
    Contains,
    Declares,
    Imports,
    Includes,
    Exports,
    Inherits,
    Implements,
    CallsSyntactically,
    Tests,
    Configures,
    LanguageSpecific(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SemanticRelation {
    pub identity: String,
    pub repository_snapshot: RepositorySnapshotId,
    pub analysis_snapshot: AnalysisSnapshotId,
    pub source_entity: String,
    pub target: RelationTarget,
    pub kind: SemanticRelationKind,
    pub supporting_range: Option<SourceRange>,
    pub diagnostics: Vec<String>,
    pub uncertainty: Uncertainty,
    pub freshness: FreshnessBasis,
    pub extensions: Vec<LanguageExtension>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "extension")]
pub enum SemanticRelationKind {
    Defines,
    References,
    ResolvesTo,
    TypeOf,
    Implements,
    Overrides,
    InstantiatedBy,
    LanguageSpecific(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StructuralProvenance {
    pub adapter: AdapterIdentity,
    pub analyzer: AnalyzerIdentity,
    pub supported_construct: String,
    pub analysis: AnalysisProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SemanticProvenance {
    pub adapter: AdapterIdentity,
    pub analyzer: AnalyzerIdentity,
    pub build_context: Option<String>,
    pub resolution_basis: String,
    pub analysis: AnalysisProvenance,
}

/// A parser/analyzer-confirmed structural fact. Inventory cannot construct this
/// without naming a structural adapter and analyzer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StructuralFact {
    pub entity: CodeEntity,
    pub relations: Vec<StructuralRelation>,
    pub provenance: StructuralProvenance,
}

/// A resolved semantic result, distinct from syntax and generated annotation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SemanticAnalysisResult {
    pub relation: SemanticRelation,
    pub provenance: SemanticProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SemanticAnnotation {
    pub identity: String,
    pub analysis_snapshot: AnalysisSnapshotId,
    pub provider: String,
    pub model: String,
    pub purpose: String,
    pub included_sources: Vec<CanonicalSourceRef>,
    pub text: String,
    pub generated_at_unix_micros: i64,
    pub uncertainty: Uncertainty,
    pub retention_state: String,
    pub provenance_class: ProvenanceClass,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentInterpretation {
    pub identity: String,
    pub analysis_snapshot: AnalysisSnapshotId,
    pub agent: String,
    pub host: String,
    pub session: String,
    pub source_basis: Vec<CanonicalSourceRef>,
    pub analysis_basis: Vec<String>,
    pub text: String,
    pub generated_at_unix_micros: i64,
    pub known_gaps: Vec<String>,
    pub uncertainty: Uncertainty,
    pub provenance_class: ProvenanceClass,
}

pub type ExtensionValue = serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LanguageExtension {
    pub language: Language,
    pub owning_adapter: AdapterIdentity,
    pub namespace: String,
    pub values: BTreeMap<String, ExtensionValue>,
    pub source_range: Option<SourceRange>,
    pub diagnostics: Vec<String>,
}

pub fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(value)
}

#[cfg(test)]
mod tests {
    use super::{CanonicalProjectRef, CanonicalSourceRef};
    use volicord_context::{ProjectId, SourceId};

    #[test]
    fn canonical_references_round_trip_as_typed_hex() -> Result<(), serde_json::Error> {
        let project = CanonicalProjectRef(ProjectId::from_bytes([0x12; 16]));
        let source = CanonicalSourceRef(SourceId::from_bytes([0x34; 16]));
        let encoded = serde_json::to_string(&(project, source))?;
        let decoded: (CanonicalProjectRef, CanonicalSourceRef) = serde_json::from_str(&encoded)?;
        assert_eq!(decoded, (project, source));
        Ok(())
    }
}
