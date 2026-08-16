use crate::identity::decode_hex;
use crate::{AnalysisSnapshotId, RepositorySnapshotId};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use volicord_context::{CheckpointId, ContextItemId, DecisionId, ProjectId, SourceId};

pub const ANALYSIS_SNAPSHOT_KIND: &str = "volicord.repository_analysis";
pub const ANALYSIS_SNAPSHOT_FORMAT_VERSION: u32 = 5;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CanonicalProjectRef {
    identity: ProjectId,
}

impl Serialize for CanonicalProjectRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CanonicalProjectRef", 1)?;
        state.serialize_field("identity", &self.identity.to_string())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for CanonicalProjectRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            identity: String,
        }

        let wire = Wire::deserialize(deserializer)?;
        let identity = decode_hex::<16>(&wire.identity).map_err(serde::de::Error::custom)?;
        Ok(Self::new(ProjectId::from_bytes(identity)))
    }
}

impl CanonicalProjectRef {
    pub(crate) const fn new(identity: ProjectId) -> Self {
        Self { identity }
    }

    pub const fn identity(self) -> ProjectId {
        self.identity
    }
}

impl fmt::Display for CanonicalProjectRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.identity, formatter)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum CanonicalSourceBasis {
    Snapshot(String),
    NotApplicable,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CanonicalSourceRef {
    project: ProjectId,
    identity: SourceId,
    basis: CanonicalSourceBasis,
}

impl Serialize for CanonicalSourceRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CanonicalSourceRef", 3)?;
        state.serialize_field("project", &self.project.to_string())?;
        state.serialize_field("identity", &self.identity.to_string())?;
        state.serialize_field("basis", &self.basis)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for CanonicalSourceRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            project: String,
            identity: String,
            basis: CanonicalSourceBasis,
        }

        let wire = Wire::deserialize(deserializer)?;
        let project = decode_hex::<16>(&wire.project).map_err(serde::de::Error::custom)?;
        let identity = decode_hex::<16>(&wire.identity).map_err(serde::de::Error::custom)?;
        Ok(Self::new(
            ProjectId::from_bytes(project),
            SourceId::from_bytes(identity),
            wire.basis,
        ))
    }
}

impl CanonicalSourceRef {
    pub(crate) fn new(project: ProjectId, identity: SourceId, basis: CanonicalSourceBasis) -> Self {
        Self {
            project,
            identity,
            basis,
        }
    }

    pub const fn project(&self) -> ProjectId {
        self.project
    }

    pub const fn identity(&self) -> SourceId {
        self.identity
    }

    pub const fn basis(&self) -> &CanonicalSourceBasis {
        &self.basis
    }

    /// Returns whether two repository observations belong to the same bounded
    /// local repository coordinate. Observation Sources and snapshot identities
    /// remain distinct; only their explicit coordinate scope is compared.
    pub fn has_compatible_repository_observation_scope(&self, other: &Self) -> bool {
        if self.project != other.project {
            return false;
        }
        if self == other {
            return true;
        }
        match (&self.basis, &other.basis) {
            (CanonicalSourceBasis::Snapshot(left), CanonicalSourceBasis::Snapshot(right)) => {
                repository_observation_scope(left).is_some_and(|left_scope| {
                    repository_observation_scope(right)
                        .is_some_and(|right_scope| left_scope == right_scope)
                })
            }
            (CanonicalSourceBasis::NotApplicable, _) | (_, CanonicalSourceBasis::NotApplicable) => {
                false
            }
        }
    }
}

fn repository_observation_scope(value: &str) -> Option<&str> {
    let value = value.strip_prefix("local-observation:sha256:")?;
    let (scope, observed_at) = value.split_once(":at:")?;
    (scope.len() == 64
        && scope.bytes().all(|byte| byte.is_ascii_hexdigit())
        && observed_at.parse::<i64>().is_ok())
    .then_some(scope)
}

impl fmt::Display for CanonicalSourceRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.project, self.identity)
    }
}

macro_rules! revisioned_canonical_reference {
    ($name:ident, $inner:ty) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name {
            project: ProjectId,
            identity: $inner,
            revision: u64,
        }

        impl $name {
            pub(crate) const fn new(project: ProjectId, identity: $inner, revision: u64) -> Self {
                Self {
                    project,
                    identity,
                    revision,
                }
            }

            pub const fn project(self) -> ProjectId {
                self.project
            }

            pub const fn identity(self) -> $inner {
                self.identity
            }

            pub const fn revision(self) -> u64 {
                self.revision
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    formatter,
                    "{}:{}@{}",
                    self.project, self.identity, self.revision
                )
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let mut state = serializer.serialize_struct(stringify!($name), 3)?;
                state.serialize_field("project", &self.project.to_string())?;
                state.serialize_field("identity", &self.identity.to_string())?;
                state.serialize_field("revision", &self.revision)?;
                state.end()
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                #[derive(Deserialize)]
                struct Wire {
                    project: String,
                    identity: String,
                    revision: u64,
                }

                let wire = Wire::deserialize(deserializer)?;
                let project = decode_hex::<16>(&wire.project).map_err(serde::de::Error::custom)?;
                let identity =
                    decode_hex::<16>(&wire.identity).map_err(serde::de::Error::custom)?;
                Ok(Self::new(
                    ProjectId::from_bytes(project),
                    <$inner>::from_bytes(identity),
                    wire.revision,
                ))
            }
        }
    };
}

revisioned_canonical_reference!(CanonicalDecisionRef, DecisionId);
revisioned_canonical_reference!(CanonicalContextItemRef, ContextItemId);
revisioned_canonical_reference!(CanonicalCheckpointRef, CheckpointId);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "target")]
pub enum CanonicalReference {
    Source(CanonicalSourceRef),
    Decision(CanonicalDecisionRef),
    ContextItem(CanonicalContextItemRef),
    Checkpoint(CanonicalCheckpointRef),
}

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
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RepositoryWorktreeObservation {
    Git {
        status_fingerprint: String,
        dirty_paths: Vec<String>,
    },
    NonGit,
}

impl RepositoryWorktreeObservation {
    pub fn dirty_paths(&self) -> &[String] {
        match self {
            Self::Git { dirty_paths, .. } => dirty_paths,
            Self::NonGit => &[],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AnalysisSnapshot {
    pub format_kind: String,
    pub format_version: u32,
    pub identity: AnalysisSnapshotId,
    /// Every analysis result is bound to exactly this one Repository Snapshot.
    pub repository_snapshot: RepositorySnapshotId,
    pub project: CanonicalProjectRef,
    pub repository_source: CanonicalSourceRef,
    /// Repository-owned baseline evidence captured with this exact analysis.
    pub repository_worktree: RepositoryWorktreeObservation,
    pub inventory: InventorySnapshot,
    pub capabilities: Vec<CapabilityReport>,
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub structural_facts: Vec<StructuralFact>,
    pub semantic_results: Vec<SemanticAnalysisResult>,
    pub semantic_annotations: Vec<SemanticAnnotation>,
    pub agent_interpretations: Vec<AgentInterpretation>,
    pub structural_bases: Vec<FileAnalysisBasis>,
    pub semantic_bases: Vec<FileAnalysisBasis>,
    pub invalidations: Vec<InvalidationRecord>,
    pub refresh: StructuralRefresh,
    pub semantic_refresh: SemanticRefresh,
    pub generated_at_unix_micros: i64,
    pub freshness: FreshnessBasis,
}

#[derive(Deserialize)]
struct AnalysisSnapshotWire {
    format_kind: String,
    format_version: u32,
    identity: AnalysisSnapshotId,
    repository_snapshot: RepositorySnapshotId,
    project: CanonicalProjectRef,
    repository_source: CanonicalSourceRef,
    repository_worktree: RepositoryWorktreeObservation,
    inventory: InventorySnapshot,
    capabilities: Vec<CapabilityReport>,
    diagnostics: Vec<AnalysisDiagnostic>,
    structural_facts: Vec<StructuralFact>,
    semantic_results: Vec<SemanticAnalysisResult>,
    semantic_annotations: Vec<SemanticAnnotation>,
    agent_interpretations: Vec<AgentInterpretation>,
    structural_bases: Vec<FileAnalysisBasis>,
    semantic_bases: Vec<FileAnalysisBasis>,
    invalidations: Vec<InvalidationRecord>,
    refresh: StructuralRefresh,
    semantic_refresh: SemanticRefresh,
    generated_at_unix_micros: i64,
    freshness: FreshnessBasis,
}

impl<'de> Deserialize<'de> for AnalysisSnapshot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AnalysisSnapshotWire::deserialize(deserializer)?;
        if wire.format_kind != ANALYSIS_SNAPSHOT_KIND {
            return Err(serde::de::Error::custom(format!(
                "unsupported Analysis Snapshot format kind: {}",
                wire.format_kind
            )));
        }
        if wire.format_version != ANALYSIS_SNAPSHOT_FORMAT_VERSION {
            return Err(serde::de::Error::custom(format!(
                "unsupported Analysis Snapshot format version: {}; current version is {}",
                wire.format_version, ANALYSIS_SNAPSHOT_FORMAT_VERSION
            )));
        }
        validate_repository_worktree(&wire.repository_worktree)
            .map_err(serde::de::Error::custom)?;
        Ok(Self {
            format_kind: wire.format_kind,
            format_version: wire.format_version,
            identity: wire.identity,
            repository_snapshot: wire.repository_snapshot,
            project: wire.project,
            repository_source: wire.repository_source,
            repository_worktree: wire.repository_worktree,
            inventory: wire.inventory,
            capabilities: wire.capabilities,
            diagnostics: wire.diagnostics,
            structural_facts: wire.structural_facts,
            semantic_results: wire.semantic_results,
            semantic_annotations: wire.semantic_annotations,
            agent_interpretations: wire.agent_interpretations,
            structural_bases: wire.structural_bases,
            semantic_bases: wire.semantic_bases,
            invalidations: wire.invalidations,
            refresh: wire.refresh,
            semantic_refresh: wire.semantic_refresh,
            generated_at_unix_micros: wire.generated_at_unix_micros,
            freshness: wire.freshness,
        })
    }
}

fn validate_repository_worktree(value: &RepositoryWorktreeObservation) -> Result<(), &'static str> {
    let RepositoryWorktreeObservation::Git {
        status_fingerprint,
        dirty_paths,
    } = value
    else {
        return Ok(());
    };
    if !status_fingerprint.starts_with("sha256:")
        || status_fingerprint.len() != "sha256:".len() + 64
        || !status_fingerprint["sha256:".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("Git worktree observation has an invalid status fingerprint");
    }
    if dirty_paths.windows(2).any(|pair| pair[0] >= pair[1])
        || dirty_paths.iter().any(|path| {
            path.is_empty()
                || path.starts_with('/')
                || path
                    .split('/')
                    .any(|part| part.is_empty() || part == "." || part == "..")
        })
    {
        return Err("Git worktree observation has non-canonical dirty paths");
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FileAnalysisBasis {
    pub area: AreaId,
    pub language: Language,
    pub content_sha256: String,
    pub adapter: AdapterIdentity,
    pub analyzer: AnalyzerIdentity,
    pub dependency_locators: Vec<String>,
    pub build_context_sha256: Option<String>,
    pub state: CapabilityState,
    pub diagnostic_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidationCategory {
    Added,
    FileContent,
    Dependency,
    BuildContext,
    AdapterContract,
    PriorFailure,
    Removed,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefreshAction {
    Parsed,
    Reused,
    Failed,
    Removed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InvalidationRecord {
    pub area: AreaId,
    pub language: Language,
    pub category: InvalidationCategory,
    pub action: RefreshAction,
    pub basis: String,
    pub dependency_area: Option<AreaId>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct StructuralRefresh {
    pub parsed_file_count: u64,
    pub reused_file_count: u64,
    pub failed_file_count: u64,
    pub removed_file_count: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SemanticRefresh {
    pub analyzed_file_count: u64,
    pub reused_file_count: u64,
    pub failed_file_count: u64,
    pub unavailable_file_count: u64,
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
    pub canonical_links: Vec<CanonicalReference>,
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    pub analysis_snapshot: AnalysisSnapshotId,
    pub repository_snapshot: RepositorySnapshotId,
    pub source: CanonicalSourceRef,
    pub source_range: Option<SourceRange>,
    pub result_kind: SearchResultKind,
    pub matched_text: String,
    pub capability: Capability,
    pub coverage: Coverage,
    pub freshness: FreshnessBasis,
    pub diagnostics: Vec<String>,
    pub provenance_class: ProvenanceClass,
    pub navigation_is_current: bool,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchResultKind {
    Inventory,
    Entity,
    Relation,
    SemanticRelation,
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroundingStatementClass {
    RepositoryObservation,
    StructuralFact,
    SemanticResult,
    AgentInterpretation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GroundingEvidence {
    pub identity: String,
    pub statement_class: GroundingStatementClass,
    pub source: CanonicalSourceRef,
    pub source_range: Option<SourceRange>,
    pub capability: Capability,
    pub provenance_class: ProvenanceClass,
    pub freshness: FreshnessBasis,
    pub diagnostics: Vec<String>,
    pub uncertainty: Uncertainty,
    pub canonical_links: Vec<CanonicalReference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GroundingGap {
    pub capability: Capability,
    pub language: Option<Language>,
    pub state: CapabilityState,
    pub reason: String,
    pub affected_areas: Vec<AreaId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GroundedExplanationBasis {
    pub analysis_snapshot: AnalysisSnapshotId,
    pub repository_snapshot: RepositorySnapshotId,
    pub evidence: Vec<GroundingEvidence>,
    pub gaps: Vec<GroundingGap>,
    pub coverage: Vec<CapabilityReport>,
    pub freshness: FreshnessBasis,
    /// This basis is local evidence only and never proves model/provider transmission.
    pub background_source_transmitted: bool,
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
    use super::{CanonicalProjectRef, CanonicalSourceBasis, CanonicalSourceRef};
    use volicord_context::{ProjectId, SourceId};

    #[test]
    fn canonical_references_round_trip_as_typed_hex() -> Result<(), serde_json::Error> {
        let project = CanonicalProjectRef::new(ProjectId::from_bytes([0x12; 16]));
        let source = CanonicalSourceRef::new(
            project.identity(),
            SourceId::from_bytes([0x34; 16]),
            CanonicalSourceBasis::Snapshot("snapshot-1".to_owned()),
        );
        let encoded = serde_json::to_string(&(project, source.clone()))?;
        let decoded: (CanonicalProjectRef, CanonicalSourceRef) = serde_json::from_str(&encoded)?;
        assert_eq!(decoded, (project, source));
        Ok(())
    }
}
