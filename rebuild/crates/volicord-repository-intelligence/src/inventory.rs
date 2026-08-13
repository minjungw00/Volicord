use crate::model::{
    AdapterIdentity, AnalysisDiagnostic, AnalysisSnapshot, AreaId, AreaKind, CandidateKind,
    CanonicalProjectRef, CanonicalSourceRef, Capability, CapabilityReport, CapabilityState,
    Coverage, DiagnosticSeverity, Ecosystem, EcosystemObservation, EcosystemObservationKind,
    EntryKind, EvidenceCandidate, FreshnessBasis, FreshnessState, GitObservation,
    InventoryClassification, InventoryEntry, InventorySnapshot, Language, ObservationBasis,
    ProvenanceClass, RepositorySnapshot, Uncertainty, UncertaintyLevel,
    ANALYSIS_SNAPSHOT_FORMAT_VERSION, ANALYSIS_SNAPSHOT_KIND,
};
use crate::{AnalysisSnapshotId, RepositorySnapshotId};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error as StdError;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use volicord_context::{ProjectId, SourceId};

const INVENTORY_ADAPTER_NAME: &str = "volicord-filesystem-inventory";
const INVENTORY_ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug)]
pub struct InventoryRequest<'a> {
    pub root: &'a Path,
    pub project_id: ProjectId,
    pub repository_source_id: SourceId,
    pub observed_at_unix_micros: i64,
    /// Slash-separated repository-relative paths. A directory excludes its subtree.
    pub excluded_paths: Vec<String>,
}

impl<'a> InventoryRequest<'a> {
    pub fn new(
        root: &'a Path,
        project_id: ProjectId,
        repository_source_id: SourceId,
        observed_at_unix_micros: i64,
    ) -> Self {
        Self {
            root,
            project_id,
            repository_source_id,
            observed_at_unix_micros,
            excluded_paths: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct InventoryError {
    message: String,
    source: Option<io::Error>,
}

impl InventoryError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    fn io(message: impl Into<String>, source: io::Error) -> Self {
        Self {
            message: message.into(),
            source: Some(source),
        }
    }
}

impl fmt::Display for InventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl StdError for InventoryError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_ref()
            .map(|source| source as &(dyn StdError + 'static))
    }
}

pub fn inventory_repository(
    request: InventoryRequest<'_>,
) -> Result<(RepositorySnapshot, AnalysisSnapshot), InventoryError> {
    let root_metadata = fs::metadata(request.root)
        .map_err(|error| InventoryError::io("repository source boundary is unavailable", error))?;
    if !root_metadata.is_dir() {
        return Err(InventoryError::new(
            "repository source boundary must be a directory",
        ));
    }

    let excluded_paths = normalize_excluded_paths(&request.excluded_paths)?;
    let ignore_patterns = load_ignore_patterns(request.root);
    let mut state = ScanState::new(excluded_paths, ignore_patterns);
    state.entries.push(InventoryEntry {
        area: repository_area(),
        entry_kind: EntryKind::Directory,
        language: None,
        classifications: BTreeSet::from([InventoryClassification::Included]),
        size_bytes: None,
        content_sha256: None,
        diagnostic_ids: Vec::new(),
    });
    scan_directory(request.root, Path::new(""), &mut state);
    state
        .entries
        .sort_by(|left, right| left.area.cmp(&right.area));
    state.diagnostics.sort_by(|left, right| {
        (&left.affected_area, &left.code, &left.identity).cmp(&(
            &right.affected_area,
            &right.code,
            &right.identity,
        ))
    });

    let git = observe_git(request.root);
    let content_fingerprint = inventory_fingerprint(&state.entries);
    let repository_identity = repository_snapshot_identity(
        request.project_id,
        request.repository_source_id,
        &content_fingerprint,
        git.as_ref(),
    );
    let observation_basis = ObservationBasis {
        content_fingerprint_sha256: content_fingerprint,
        git,
    };
    let (included_areas, excluded_areas, unavailable_areas) = partition_areas(&state.entries);
    let repository_snapshot = RepositorySnapshot {
        format_version: ANALYSIS_SNAPSHOT_FORMAT_VERSION,
        identity: repository_identity,
        project: CanonicalProjectRef(request.project_id),
        repository_source: CanonicalSourceRef(request.repository_source_id),
        source_boundary: ".".to_owned(),
        observation_basis,
        included_areas,
        excluded_areas,
        unavailable_areas,
        observed_at_unix_micros: request.observed_at_unix_micros,
    };

    let languages = state
        .entries
        .iter()
        .filter(|entry| {
            entry
                .classifications
                .contains(&InventoryClassification::Included)
        })
        .filter_map(|entry| entry.language.clone())
        .collect::<BTreeSet<_>>();
    let (ecosystem_observations, evidence_candidates) = inventory_evidence(&state.entries);
    let inventory = InventorySnapshot {
        entries: state.entries,
        languages: languages.clone(),
        ecosystem_observations,
        evidence_candidates,
    };
    let freshness = FreshnessBasis {
        state: FreshnessState::Current,
        repository_snapshot: repository_identity,
        compared_repository_snapshot: None,
        reason: None,
    };
    let capabilities = capability_reports(
        repository_identity,
        &inventory,
        &state.diagnostics,
        request.observed_at_unix_micros,
        &freshness,
    );
    let analysis_identity = analysis_snapshot_identity(
        repository_identity,
        &inventory,
        &capabilities,
        &state.diagnostics,
    )?;
    let analysis = AnalysisSnapshot {
        format_kind: ANALYSIS_SNAPSHOT_KIND.to_owned(),
        format_version: ANALYSIS_SNAPSHOT_FORMAT_VERSION,
        identity: analysis_identity,
        repository_snapshot: repository_identity,
        project: CanonicalProjectRef(request.project_id),
        repository_source: CanonicalSourceRef(request.repository_source_id),
        inventory,
        capabilities,
        diagnostics: state.diagnostics,
        structural_facts: Vec::new(),
        semantic_results: Vec::new(),
        semantic_annotations: Vec::new(),
        agent_interpretations: Vec::new(),
        generated_at_unix_micros: request.observed_at_unix_micros,
        freshness,
    };
    Ok((repository_snapshot, analysis))
}

struct ScanState {
    excluded_paths: Vec<String>,
    ignore_patterns: Vec<IgnorePattern>,
    entries: Vec<InventoryEntry>,
    diagnostics: Vec<AnalysisDiagnostic>,
}

impl ScanState {
    fn new(excluded_paths: Vec<String>, ignore_patterns: Vec<IgnorePattern>) -> Self {
        Self {
            excluded_paths,
            ignore_patterns,
            entries: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn diagnostic(
        &mut self,
        code: &str,
        message: String,
        area: AreaId,
        severity: DiagnosticSeverity,
    ) -> String {
        let identity = diagnostic_identity(code, &area.path);
        self.diagnostics.push(AnalysisDiagnostic {
            identity: identity.clone(),
            severity,
            code: code.to_owned(),
            message,
            affected_area: area,
            capability: Capability::Inventory,
            adapter: Some(inventory_adapter()),
            analyzer: None,
            usable_remainder: Some("unaffected inventory entries remain usable".to_owned()),
        });
        identity
    }
}

fn scan_directory(absolute: &Path, relative: &Path, state: &mut ScanState) {
    let read_directory = match fs::read_dir(absolute) {
        Ok(entries) => entries,
        Err(error) => {
            let area = directory_area(relative);
            state.diagnostic(
                "directory_unavailable",
                format!("directory could not be read: {error}"),
                area.clone(),
                DiagnosticSeverity::Error,
            );
            if let Some(entry) = state.entries.iter_mut().find(|entry| entry.area == area) {
                entry
                    .classifications
                    .remove(&InventoryClassification::Included);
                entry
                    .classifications
                    .insert(InventoryClassification::Unavailable);
            }
            return;
        }
    };

    let mut children = read_directory.collect::<Vec<_>>();
    children.sort_by(|left, right| match (left, right) {
        (Ok(left), Ok(right)) => left.file_name().cmp(&right.file_name()),
        (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
        (Ok(_), Err(_)) => std::cmp::Ordering::Less,
        (Err(left), Err(right)) => left.to_string().cmp(&right.to_string()),
    });

    for child in children {
        match child {
            Ok(child) => scan_entry(child.path(), relative.join(child.file_name()), state),
            Err(error) => {
                let area = directory_area(relative);
                state.diagnostic(
                    "directory_entry_unavailable",
                    format!("a directory entry could not be read: {error}"),
                    area,
                    DiagnosticSeverity::Error,
                );
            }
        }
    }
}

fn scan_entry(absolute: PathBuf, relative: PathBuf, state: &mut ScanState) {
    let locator = portable_locator(&relative);
    let metadata = match fs::symlink_metadata(&absolute) {
        Ok(metadata) => metadata,
        Err(error) => {
            let area = file_area(locator);
            let diagnostic = state.diagnostic(
                "metadata_unavailable",
                format!("entry metadata could not be read: {error}"),
                area.clone(),
                DiagnosticSeverity::Error,
            );
            state.entries.push(InventoryEntry {
                area,
                entry_kind: EntryKind::Other,
                language: None,
                classifications: BTreeSet::from([InventoryClassification::Unavailable]),
                size_bytes: None,
                content_sha256: None,
                diagnostic_ids: vec![diagnostic],
            });
            return;
        }
    };

    let is_directory = metadata.is_dir();
    let classifications = classify_path(&locator, is_directory, state);
    let excluded = classifications.contains(&InventoryClassification::Excluded)
        || classifications.contains(&InventoryClassification::Ignored)
        || classifications.contains(&InventoryClassification::Vendor)
        || classifications.contains(&InventoryClassification::Generated);

    if metadata.file_type().is_symlink() {
        let area = file_area(locator);
        let diagnostic = state.diagnostic(
            "symlink_not_followed",
            "symbolic-link content is outside this inventory observation".to_owned(),
            area.clone(),
            DiagnosticSeverity::Information,
        );
        let mut classifications = classifications;
        classifications.remove(&InventoryClassification::Included);
        classifications.insert(InventoryClassification::Unavailable);
        state.entries.push(InventoryEntry {
            area,
            entry_kind: EntryKind::Symlink,
            language: None,
            classifications,
            size_bytes: None,
            content_sha256: None,
            diagnostic_ids: vec![diagnostic],
        });
        return;
    }

    if is_directory {
        state.entries.push(InventoryEntry {
            area: directory_area(&relative),
            entry_kind: EntryKind::Directory,
            language: None,
            classifications,
            size_bytes: None,
            content_sha256: None,
            diagnostic_ids: Vec::new(),
        });
        if !excluded {
            scan_directory(&absolute, &relative, state);
        }
        return;
    }

    if !metadata.is_file() {
        let area = file_area(locator);
        let diagnostic = state.diagnostic(
            "entry_type_unavailable",
            "entry is neither a regular file, directory, nor symbolic link".to_owned(),
            area.clone(),
            DiagnosticSeverity::Warning,
        );
        let mut classifications = classifications;
        classifications.insert(InventoryClassification::Unavailable);
        state.entries.push(InventoryEntry {
            area,
            entry_kind: EntryKind::Other,
            language: None,
            classifications,
            size_bytes: None,
            content_sha256: None,
            diagnostic_ids: vec![diagnostic],
        });
        return;
    }

    let mut classifications = classifications;
    let area = file_area(locator.clone());
    if excluded {
        state.entries.push(InventoryEntry {
            area,
            entry_kind: EntryKind::File,
            language: detect_language_from_path(&relative),
            classifications,
            size_bytes: Some(metadata.len()),
            content_sha256: None,
            diagnostic_ids: Vec::new(),
        });
        return;
    }

    match fs::read(&absolute) {
        Ok(bytes) => {
            let binary = bytes.iter().take(8192).any(|byte| *byte == 0);
            if binary {
                classifications.insert(InventoryClassification::Binary);
            } else {
                classify_text_role(&relative, &bytes, &mut classifications);
            }
            let language = if binary {
                None
            } else {
                detect_language(&relative, &bytes)
            };
            state.entries.push(InventoryEntry {
                area,
                entry_kind: EntryKind::File,
                language,
                classifications,
                size_bytes: Some(metadata.len()),
                content_sha256: Some(sha256_hex(&bytes)),
                diagnostic_ids: Vec::new(),
            });
        }
        Err(error) => {
            let diagnostic = state.diagnostic(
                "file_unavailable",
                format!("file content could not be read: {error}"),
                area.clone(),
                DiagnosticSeverity::Error,
            );
            classifications.remove(&InventoryClassification::Included);
            classifications.insert(InventoryClassification::Unavailable);
            state.entries.push(InventoryEntry {
                area,
                entry_kind: EntryKind::File,
                language: detect_language_from_path(&relative),
                classifications,
                size_bytes: Some(metadata.len()),
                content_sha256: None,
                diagnostic_ids: vec![diagnostic],
            });
        }
    }
}

fn classify_path(
    locator: &str,
    is_directory: bool,
    state: &ScanState,
) -> BTreeSet<InventoryClassification> {
    let mut values = BTreeSet::from([InventoryClassification::Included]);
    let basename = locator.rsplit('/').next().unwrap_or(locator);
    if locator == ".git"
        || locator.starts_with(".git/")
        || path_is_excluded(locator, &state.excluded_paths)
    {
        values.remove(&InventoryClassification::Included);
        values.insert(InventoryClassification::Excluded);
    } else if is_ignored(locator, is_directory, &state.ignore_patterns) {
        values.remove(&InventoryClassification::Included);
        values.insert(InventoryClassification::Ignored);
    } else if matches!(basename, "node_modules" | "vendor" | "third_party") {
        values.remove(&InventoryClassification::Included);
        values.insert(InventoryClassification::Vendor);
    } else if matches!(
        basename,
        "target" | "dist" | "build" | "coverage" | ".cache" | "__pycache__"
    ) {
        values.remove(&InventoryClassification::Included);
        values.insert(InventoryClassification::Generated);
    }
    values
}

fn classify_text_role(path: &Path, bytes: &[u8], values: &mut BTreeSet<InventoryClassification>) {
    let locator = portable_locator(path);
    let basename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let lower = basename.to_ascii_lowercase();
    if manifest_observation(&locator).is_some() {
        values.insert(InventoryClassification::Manifest);
    }
    if is_workspace_manifest(path, bytes) {
        values.insert(InventoryClassification::WorkspaceManifest);
    }
    if is_configuration_file(&lower) {
        values.insert(InventoryClassification::Configuration);
    }
    if is_document_file(&lower) {
        values.insert(InventoryClassification::Document);
    }
    if is_source_file(path) {
        values.insert(InventoryClassification::Source);
    }
    if locator
        .split('/')
        .any(|part| matches!(part, "test" | "tests"))
        || lower.starts_with("test_")
        || lower.contains(".test.")
        || lower.contains("_test.")
    {
        values.insert(InventoryClassification::Test);
    }
}

fn detect_language(path: &Path, bytes: &[u8]) -> Option<Language> {
    let detected = detect_language_from_path(path);
    if detected.is_some() {
        return detected;
    }
    if std::str::from_utf8(bytes).is_ok() {
        Some(Language::UnknownText)
    } else {
        None
    }
}

fn detect_language_from_path(path: &Path) -> Option<Language> {
    let basename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let language = match extension.as_str() {
        "java" => Language::Java,
        "py" | "pyi" => Language::Python,
        "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
        "ts" | "tsx" | "mts" | "cts" => Language::TypeScript,
        "c" => Language::C,
        "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => Language::Cpp,
        "h" => Language::C,
        "rs" => Language::Rust,
        "md" | "mdx" => Language::Markdown,
        "json" | "jsonc" => Language::Json,
        "yaml" | "yml" => Language::Yaml,
        "toml" => Language::Toml,
        "xml" => Language::Xml,
        "sh" | "bash" | "zsh" => Language::Shell,
        "go" => Language::Go,
        "rb" => Language::OtherText("ruby".to_owned()),
        "kt" | "kts" => Language::OtherText("kotlin".to_owned()),
        "swift" => Language::OtherText("swift".to_owned()),
        "php" => Language::OtherText("php".to_owned()),
        "cs" => Language::OtherText("csharp".to_owned()),
        _ if matches!(basename, "Makefile" | "Dockerfile") => Language::UnknownText,
        _ => return None,
    };
    Some(language)
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        detect_language_from_path(path),
        Some(
            Language::Java
                | Language::Python
                | Language::JavaScript
                | Language::TypeScript
                | Language::C
                | Language::Cpp
                | Language::Rust
                | Language::Go
                | Language::OtherText(_)
        )
    )
}

fn is_configuration_file(lower_basename: &str) -> bool {
    lower_basename.starts_with('.')
        || matches!(
            lower_basename,
            "cmakelists.txt"
                | "compile_commands.json"
                | "tsconfig.json"
                | "makefile"
                | "dockerfile"
        )
        || lower_basename.ends_with(".config")
        || lower_basename.ends_with(".conf")
        || lower_basename.ends_with(".json")
        || lower_basename.ends_with(".jsonc")
        || lower_basename.ends_with(".yaml")
        || lower_basename.ends_with(".yml")
        || lower_basename.ends_with(".toml")
        || lower_basename.ends_with(".xml")
}

fn is_workspace_manifest(path: &Path, bytes: &[u8]) -> bool {
    let Some(basename) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    if matches!(
        basename,
        "settings.gradle" | "settings.gradle.kts" | "pnpm-workspace.yaml"
    ) {
        return true;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    match basename {
        "Cargo.toml" => text.lines().any(|line| line.trim() == "[workspace]"),
        "package.json" => text.contains("\"workspaces\""),
        "pom.xml" => text.contains("<modules") || text.contains("<module>"),
        _ => false,
    }
}

fn is_document_file(lower_basename: &str) -> bool {
    lower_basename.starts_with("readme")
        || lower_basename.starts_with("license")
        || lower_basename.starts_with("changelog")
        || lower_basename.ends_with(".md")
        || lower_basename.ends_with(".mdx")
}

fn inventory_evidence(
    entries: &[InventoryEntry],
) -> (Vec<EcosystemObservation>, Vec<EvidenceCandidate>) {
    let mut observations = Vec::new();
    let mut candidates = Vec::new();
    let mut component_evidence = BTreeMap::<String, Vec<AreaId>>::new();

    for entry in entries {
        if entry.entry_kind != EntryKind::File
            || !entry
                .classifications
                .contains(&InventoryClassification::Included)
        {
            continue;
        }
        if let Some((ecosystem, detected_kind)) = manifest_observation(&entry.area.path) {
            let kind = if entry
                .classifications
                .contains(&InventoryClassification::WorkspaceManifest)
            {
                EcosystemObservationKind::WorkspaceManifest
            } else {
                detected_kind
            };
            observations.push(EcosystemObservation {
                ecosystem,
                kind,
                area: entry.area.clone(),
                evidence: vec![entry.area.clone()],
                provenance_class: ProvenanceClass::RepositoryObservation,
                uncertainty: Uncertainty::none(),
            });
            if kind == EcosystemObservationKind::PackageManifest {
                candidates.push(EvidenceCandidate {
                    kind: CandidateKind::Package,
                    name: entry
                        .area
                        .path
                        .rsplit_once('/')
                        .map(|(parent, _)| parent.to_owned()),
                    area: parent_area(&entry.area.path, AreaKind::Package),
                    evidence: vec![entry.area.clone()],
                    provenance_class: ProvenanceClass::RepositoryObservation,
                    uncertainty: Uncertainty {
                        level: UncertaintyLevel::Low,
                        reasons: vec![
                            "candidate is based on manifest location; manifest content was not semantically interpreted"
                                .to_owned(),
                        ],
                    },
                });
            }
            component_evidence
                .entry(parent_locator(&entry.area.path))
                .or_default()
                .push(entry.area.clone());
        }
        if is_entry_point_candidate(&entry.area.path) {
            candidates.push(EvidenceCandidate {
                kind: CandidateKind::EntryPoint,
                name: entry.area.path.rsplit('/').next().map(ToOwned::to_owned),
                area: entry.area.clone(),
                evidence: vec![entry.area.clone()],
                provenance_class: ProvenanceClass::RepositoryObservation,
                uncertainty: Uncertainty {
                    level: UncertaintyLevel::Medium,
                    reasons: vec![
                        "candidate is based on a conventional filename, not structural analysis"
                            .to_owned(),
                    ],
                },
            });
        }
    }
    for (path, evidence) in component_evidence {
        candidates.push(EvidenceCandidate {
            kind: CandidateKind::Component,
            name: (path != ".").then_some(path.clone()),
            area: AreaId {
                kind: AreaKind::Component,
                path,
            },
            evidence,
            provenance_class: ProvenanceClass::RepositoryObservation,
            uncertainty: Uncertainty {
                level: UncertaintyLevel::Low,
                reasons: vec![
                    "component candidate is bounded by colocated manifest evidence".to_owned(),
                ],
            },
        });
    }
    observations.sort_by(|left, right| {
        (&left.area, &left.ecosystem, left.kind).cmp(&(&right.area, &right.ecosystem, right.kind))
    });
    candidates.sort_by(|left, right| {
        (left.kind, &left.area, &left.name).cmp(&(right.kind, &right.area, &right.name))
    });
    (observations, candidates)
}

fn manifest_observation(path: &str) -> Option<(Ecosystem, EcosystemObservationKind)> {
    let basename = path.rsplit('/').next().unwrap_or(path);
    match basename {
        "pom.xml" => Some((Ecosystem::Maven, EcosystemObservationKind::PackageManifest)),
        "build.gradle" | "build.gradle.kts" => Some((
            Ecosystem::Gradle,
            EcosystemObservationKind::BuildConfiguration,
        )),
        "settings.gradle" | "settings.gradle.kts" => Some((
            Ecosystem::Gradle,
            EcosystemObservationKind::WorkspaceManifest,
        )),
        "pyproject.toml" | "setup.py" | "setup.cfg" => Some((
            Ecosystem::PythonPackage,
            EcosystemObservationKind::PackageManifest,
        )),
        "package.json" => Some((Ecosystem::Node, EcosystemObservationKind::PackageManifest)),
        "pnpm-workspace.yaml" => {
            Some((Ecosystem::Node, EcosystemObservationKind::WorkspaceManifest))
        }
        "tsconfig.json" => Some((
            Ecosystem::TypeScript,
            EcosystemObservationKind::ToolchainConfiguration,
        )),
        "CMakeLists.txt" => Some((
            Ecosystem::Cmake,
            EcosystemObservationKind::BuildConfiguration,
        )),
        "compile_commands.json" => Some((
            Ecosystem::CompilationDatabase,
            EcosystemObservationKind::BuildConfiguration,
        )),
        "Cargo.toml" => Some((Ecosystem::Cargo, EcosystemObservationKind::PackageManifest)),
        "go.mod" => Some((
            Ecosystem::GoModules,
            EcosystemObservationKind::PackageManifest,
        )),
        _ => None,
    }
}

fn is_entry_point_candidate(path: &str) -> bool {
    let basename = path.rsplit('/').next().unwrap_or(path);
    matches!(
        basename,
        "main.py"
            | "__main__.py"
            | "main.js"
            | "index.js"
            | "main.ts"
            | "index.ts"
            | "main.c"
            | "main.cpp"
            | "main.cc"
            | "main.rs"
            | "main.go"
    ) || path.ends_with("/src/main.rs")
}

fn capability_reports(
    repository_snapshot: RepositorySnapshotId,
    inventory: &InventorySnapshot,
    diagnostics: &[AnalysisDiagnostic],
    observed_at: i64,
    freshness: &FreshnessBasis,
) -> Vec<CapabilityReport> {
    let included = inventory
        .entries
        .iter()
        .filter(|entry| {
            entry.entry_kind == EntryKind::File
                && entry
                    .classifications
                    .contains(&InventoryClassification::Included)
        })
        .map(|entry| entry.area.clone())
        .collect::<Vec<_>>();
    let excluded = inventory
        .entries
        .iter()
        .filter(|entry| {
            entry.classifications.iter().any(|classification| {
                matches!(
                    classification,
                    InventoryClassification::Excluded
                        | InventoryClassification::Ignored
                        | InventoryClassification::Vendor
                        | InventoryClassification::Generated
                )
            })
        })
        .map(|entry| entry.area.clone())
        .collect::<Vec<_>>();
    let unavailable = inventory
        .entries
        .iter()
        .filter(|entry| {
            entry
                .classifications
                .contains(&InventoryClassification::Unavailable)
        })
        .map(|entry| entry.area.clone())
        .collect::<Vec<_>>();
    let inventory_state = if diagnostics.is_empty() {
        CapabilityState::Available
    } else {
        CapabilityState::Partial
    };
    let mut reports = vec![CapabilityReport {
        repository_snapshot,
        language: None,
        area: repository_area(),
        capability: Capability::Inventory,
        state: inventory_state,
        reason: (!diagnostics.is_empty())
            .then_some("some inventory scopes were unavailable".to_owned()),
        usable_remainder: Some("all listed included entries remain usable".to_owned()),
        user_visible_consequence: (!diagnostics.is_empty()).then_some(
            "inventory is partial; inspect diagnostics and unavailable coverage".to_owned(),
        ),
        coverage: Coverage {
            covered_file_count: included.len() as u64,
            included: included.clone(),
            excluded: excluded.clone(),
            unavailable: unavailable.clone(),
            ..Coverage::default()
        },
        diagnostics: diagnostics
            .iter()
            .map(|diagnostic| diagnostic.identity.clone())
            .collect(),
        adapter: Some(inventory_adapter()),
        analyzer: None,
        provenance_class: ProvenanceClass::RepositoryObservation,
        observed_at_unix_micros: observed_at,
        freshness: freshness.clone(),
        uncertainty: Uncertainty::none(),
    }];

    for language in &inventory.languages {
        let language_areas = inventory
            .entries
            .iter()
            .filter(|entry| {
                entry.language.as_ref() == Some(language)
                    && entry
                        .classifications
                        .contains(&InventoryClassification::Included)
            })
            .map(|entry| entry.area.clone())
            .collect::<Vec<_>>();
        reports.push(CapabilityReport {
            repository_snapshot,
            language: Some(language.clone()),
            area: repository_area(),
            capability: Capability::Inventory,
            state: CapabilityState::Available,
            reason: None,
            usable_remainder: None,
            user_visible_consequence: None,
            coverage: Coverage {
                covered_file_count: language_areas.len() as u64,
                included: language_areas.clone(),
                excluded: excluded.clone(),
                ..Coverage::default()
            },
            diagnostics: Vec::new(),
            adapter: Some(inventory_adapter()),
            analyzer: None,
            provenance_class: ProvenanceClass::RepositoryObservation,
            observed_at_unix_micros: observed_at,
            freshness: freshness.clone(),
            uncertainty: Uncertainty::none(),
        });
        reports.push(unavailable_capability_report(
            repository_snapshot,
            language,
            Capability::AgentAssisted,
            &language_areas,
            observed_at,
            freshness,
            "no interactive host interpretation was requested by this inventory operation",
        ));
        if language.is_structural_gate_language() {
            reports.push(unavailable_capability_report(
                repository_snapshot,
                language,
                Capability::Structural,
                &language_areas,
                observed_at,
                freshness,
                "the language is in the structural gate, but no Production structural adapter is installed",
            ));
            reports.push(unavailable_capability_report(
                repository_snapshot,
                language,
                Capability::Semantic,
                &language_areas,
                observed_at,
                freshness,
                "no semantic analyzer is installed",
            ));
        } else {
            reports.push(unsupported_capability_report(
                repository_snapshot,
                language,
                Capability::Structural,
                &language_areas,
                observed_at,
                freshness,
            ));
            reports.push(unsupported_capability_report(
                repository_snapshot,
                language,
                Capability::Semantic,
                &language_areas,
                observed_at,
                freshness,
            ));
        }

        let ecosystem_evidence = inventory
            .ecosystem_observations
            .iter()
            .filter(|observation| ecosystem_matches_language(&observation.ecosystem, language))
            .flat_map(|observation| observation.evidence.iter().cloned())
            .collect::<Vec<_>>();
        if ecosystem_evidence.is_empty() {
            let report = if language.is_structural_gate_language() {
                unavailable_capability_report(
                    repository_snapshot,
                    language,
                    Capability::Ecosystem,
                    &language_areas,
                    observed_at,
                    freshness,
                    "no recognized ecosystem manifest or build evidence was observed",
                )
            } else {
                unsupported_capability_report(
                    repository_snapshot,
                    language,
                    Capability::Ecosystem,
                    &language_areas,
                    observed_at,
                    freshness,
                )
            };
            reports.push(report);
        } else {
            reports.push(CapabilityReport {
                repository_snapshot,
                language: Some(language.clone()),
                area: repository_area(),
                capability: Capability::Ecosystem,
                state: CapabilityState::Partial,
                reason: Some(
                    "manifest/build evidence was inventoried without semantic build interpretation"
                        .to_owned(),
                ),
                usable_remainder: Some("listed ecosystem evidence is directly observed".to_owned()),
                user_visible_consequence: Some(
                    "package/workspace conclusions remain candidates until an ecosystem adapter validates them"
                        .to_owned(),
                ),
                coverage: Coverage {
                    included: ecosystem_evidence,
                    covered_file_count: language_areas.len() as u64,
                    ..Coverage::default()
                },
                diagnostics: Vec::new(),
                adapter: Some(inventory_adapter()),
                analyzer: None,
                provenance_class: ProvenanceClass::RepositoryObservation,
                observed_at_unix_micros: observed_at,
                freshness: freshness.clone(),
                uncertainty: Uncertainty {
                    level: UncertaintyLevel::Medium,
                    reasons: vec![
                        "inventory does not resolve package or workspace semantics".to_owned(),
                    ],
                },
            });
        }
    }
    reports.sort_by(|left, right| {
        (&left.language, left.capability, &left.area).cmp(&(
            &right.language,
            right.capability,
            &right.area,
        ))
    });
    reports
}

fn unavailable_capability_report(
    repository_snapshot: RepositorySnapshotId,
    language: &Language,
    capability: Capability,
    areas: &[AreaId],
    observed_at: i64,
    freshness: &FreshnessBasis,
    reason: &str,
) -> CapabilityReport {
    CapabilityReport {
        repository_snapshot,
        language: Some(language.clone()),
        area: repository_area(),
        capability,
        state: CapabilityState::Unavailable,
        reason: Some(reason.to_owned()),
        usable_remainder: Some("inventory remains available".to_owned()),
        user_visible_consequence: Some(format!("{capability:?} results are not available")),
        coverage: Coverage {
            unavailable: areas.to_vec(),
            ..Coverage::default()
        },
        diagnostics: Vec::new(),
        adapter: None,
        analyzer: None,
        provenance_class: ProvenanceClass::RepositoryObservation,
        observed_at_unix_micros: observed_at,
        freshness: freshness.clone(),
        uncertainty: Uncertainty {
            level: UncertaintyLevel::Unknown,
            reasons: vec!["the capability did not run".to_owned()],
        },
    }
}

fn unsupported_capability_report(
    repository_snapshot: RepositorySnapshotId,
    language: &Language,
    capability: Capability,
    areas: &[AreaId],
    observed_at: i64,
    freshness: &FreshnessBasis,
) -> CapabilityReport {
    CapabilityReport {
        repository_snapshot,
        language: Some(language.clone()),
        area: repository_area(),
        capability,
        state: CapabilityState::Unsupported,
        reason: Some(
            "this inventory boundary provides no adapter contract for the language/capability"
                .to_owned(),
        ),
        usable_remainder: Some("inventory remains available".to_owned()),
        user_visible_consequence: Some(format!("{capability:?} is unsupported for this language")),
        coverage: Coverage {
            unsupported: areas.to_vec(),
            ..Coverage::default()
        },
        diagnostics: Vec::new(),
        adapter: None,
        analyzer: None,
        provenance_class: ProvenanceClass::RepositoryObservation,
        observed_at_unix_micros: observed_at,
        freshness: freshness.clone(),
        uncertainty: Uncertainty {
            level: UncertaintyLevel::Unknown,
            reasons: vec!["the capability is not provided".to_owned()],
        },
    }
}

fn ecosystem_matches_language(ecosystem: &Ecosystem, language: &Language) -> bool {
    matches!(
        (ecosystem, language),
        (Ecosystem::Maven | Ecosystem::Gradle, Language::Java)
            | (Ecosystem::PythonPackage, Language::Python)
            | (Ecosystem::Node, Language::JavaScript | Language::TypeScript)
            | (Ecosystem::TypeScript, Language::TypeScript)
            | (
                Ecosystem::Cmake | Ecosystem::CompilationDatabase,
                Language::C | Language::Cpp
            )
            | (Ecosystem::Cargo, Language::Rust)
            | (Ecosystem::GoModules, Language::Go)
    )
}

fn inventory_adapter() -> AdapterIdentity {
    AdapterIdentity {
        name: INVENTORY_ADAPTER_NAME.to_owned(),
        version: INVENTORY_ADAPTER_VERSION.to_owned(),
    }
}

fn partition_areas(entries: &[InventoryEntry]) -> (Vec<AreaId>, Vec<AreaId>, Vec<AreaId>) {
    let mut included = Vec::new();
    let mut excluded = Vec::new();
    let mut unavailable = Vec::new();
    for entry in entries {
        if entry
            .classifications
            .contains(&InventoryClassification::Included)
        {
            included.push(entry.area.clone());
        }
        if entry.classifications.iter().any(|classification| {
            matches!(
                classification,
                InventoryClassification::Excluded
                    | InventoryClassification::Ignored
                    | InventoryClassification::Vendor
                    | InventoryClassification::Generated
            )
        }) {
            excluded.push(entry.area.clone());
        }
        if entry
            .classifications
            .contains(&InventoryClassification::Unavailable)
        {
            unavailable.push(entry.area.clone());
        }
    }
    (included, excluded, unavailable)
}

fn inventory_fingerprint(entries: &[InventoryEntry]) -> String {
    let mut hasher = Sha256::new();
    for entry in entries {
        hash_part(&mut hasher, entry.area.path.as_bytes());
        hash_part(&mut hasher, format!("{:?}", entry.entry_kind).as_bytes());
        for classification in &entry.classifications {
            hash_part(&mut hasher, format!("{classification:?}").as_bytes());
        }
        if let Some(language) = &entry.language {
            hash_part(&mut hasher, format!("{language:?}").as_bytes());
        }
        if let Some(size) = entry.size_bytes {
            hash_part(&mut hasher, &size.to_be_bytes());
        }
        if let Some(content) = &entry.content_sha256 {
            hash_part(&mut hasher, content.as_bytes());
        }
    }
    hex_digest(hasher.finalize().as_slice())
}

fn repository_snapshot_identity(
    project_id: ProjectId,
    repository_source_id: SourceId,
    content_fingerprint: &str,
    git: Option<&GitObservation>,
) -> RepositorySnapshotId {
    let git_head = git.map_or("", |observation| observation.head.as_str());
    let git_reference = git
        .and_then(|observation| observation.reference.as_deref())
        .unwrap_or("");
    RepositorySnapshotId::digest(&[
        b"volicord.repository_snapshot.v1",
        project_id.as_bytes(),
        repository_source_id.as_bytes(),
        b".",
        content_fingerprint.as_bytes(),
        git_head.as_bytes(),
        git_reference.as_bytes(),
    ])
}

fn analysis_snapshot_identity(
    repository_snapshot: RepositorySnapshotId,
    inventory: &InventorySnapshot,
    capabilities: &[CapabilityReport],
    diagnostics: &[AnalysisDiagnostic],
) -> Result<AnalysisSnapshotId, InventoryError> {
    let inventory_bytes = serde_json::to_vec(inventory)
        .map_err(|error| InventoryError::new(format!("inventory serialization failed: {error}")))?;
    let capability_basis = capabilities
        .iter()
        .map(|report| {
            (
                &report.language,
                report.capability,
                report.state,
                &report.coverage,
                &report.adapter,
                &report.analyzer,
            )
        })
        .collect::<Vec<_>>();
    let capability_bytes = serde_json::to_vec(&capability_basis).map_err(|error| {
        InventoryError::new(format!("capability serialization failed: {error}"))
    })?;
    let diagnostic_basis = diagnostics
        .iter()
        .map(|diagnostic| {
            (
                &diagnostic.identity,
                &diagnostic.code,
                &diagnostic.affected_area,
            )
        })
        .collect::<Vec<_>>();
    let diagnostic_bytes = serde_json::to_vec(&diagnostic_basis).map_err(|error| {
        InventoryError::new(format!("diagnostic serialization failed: {error}"))
    })?;
    Ok(AnalysisSnapshotId::digest(&[
        b"volicord.analysis_snapshot.v1",
        repository_snapshot.as_bytes(),
        &inventory_bytes,
        &capability_bytes,
        &diagnostic_bytes,
    ]))
}

fn observe_git(root: &Path) -> Option<GitObservation> {
    let marker = root.join(".git");
    let git_directory = if marker.is_dir() {
        marker
    } else {
        let marker_text = fs::read_to_string(marker).ok()?;
        let relative = marker_text.trim().strip_prefix("gitdir:")?.trim();
        let path = Path::new(relative);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        }
    };
    let head_text = fs::read_to_string(git_directory.join("HEAD")).ok()?;
    let head = head_text.trim();
    if let Some(reference) = head.strip_prefix("ref: ") {
        let value = fs::read_to_string(git_directory.join(reference))
            .ok()
            .map(|value| value.trim().to_owned())
            .or_else(|| read_packed_reference(&git_directory, reference))?;
        Some(GitObservation {
            head: value,
            reference: Some(reference.to_owned()),
        })
    } else if !head.is_empty() {
        Some(GitObservation {
            head: head.to_owned(),
            reference: None,
        })
    } else {
        None
    }
}

fn read_packed_reference(git_directory: &Path, reference: &str) -> Option<String> {
    fs::read_to_string(git_directory.join("packed-refs"))
        .ok()?
        .lines()
        .filter(|line| !line.starts_with('#') && !line.starts_with('^'))
        .find_map(|line| {
            let (value, name) = line.split_once(' ')?;
            (name == reference).then(|| value.to_owned())
        })
}

#[derive(Clone, Debug)]
struct IgnorePattern {
    pattern: String,
    directory_only: bool,
    negated: bool,
    anchored: bool,
}

fn load_ignore_patterns(root: &Path) -> Vec<IgnorePattern> {
    let Ok(contents) = fs::read_to_string(root.join(".gitignore")) else {
        return Vec::new();
    };
    contents
        .lines()
        .filter_map(|line| {
            let mut pattern = line.trim();
            if pattern.is_empty() || pattern.starts_with('#') {
                return None;
            }
            let negated = pattern.starts_with('!');
            if negated {
                pattern = pattern.get(1..)?;
            }
            let anchored = pattern.starts_with('/');
            if anchored {
                pattern = pattern.get(1..)?;
            }
            let directory_only = pattern.ends_with('/');
            if directory_only {
                pattern = pattern.strip_suffix('/')?;
            }
            (!pattern.is_empty()).then(|| IgnorePattern {
                pattern: pattern.to_owned(),
                directory_only,
                negated,
                anchored,
            })
        })
        .collect()
}

fn is_ignored(locator: &str, is_directory: bool, patterns: &[IgnorePattern]) -> bool {
    let mut ignored = false;
    for pattern in patterns {
        if pattern.directory_only
            && !is_directory
            && !locator.starts_with(&format!("{}/", pattern.pattern))
        {
            continue;
        }
        let matched = if pattern.anchored || pattern.pattern.contains('/') {
            glob_matches(&pattern.pattern, locator)
                || locator.starts_with(&format!("{}/", pattern.pattern))
        } else {
            locator
                .split('/')
                .any(|component| glob_matches(&pattern.pattern, component))
        };
        if matched {
            ignored = !pattern.negated;
        }
    }
    ignored
}

fn glob_matches(pattern: &str, value: &str) -> bool {
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();
    let (mut pattern_index, mut value_index) = (0_usize, 0_usize);
    let (mut star_index, mut star_value_index) = (None, 0_usize);
    while value_index < value.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == b'?' || pattern[pattern_index] == value[value_index])
        {
            pattern_index += 1;
            value_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star_index = Some(pattern_index);
            pattern_index += 1;
            star_value_index = value_index;
        } else if let Some(star) = star_index {
            pattern_index = star + 1;
            star_value_index += 1;
            value_index = star_value_index;
        } else {
            return false;
        }
    }
    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

fn normalize_excluded_paths(values: &[String]) -> Result<Vec<String>, InventoryError> {
    let mut normalized = Vec::new();
    for value in values {
        let path = Path::new(value);
        if path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
        {
            return Err(InventoryError::new(
                "excluded paths must be repository-relative and cannot contain '..'",
            ));
        }
        let locator = portable_locator(path);
        if locator != "." {
            normalized.push(locator);
        }
    }
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn path_is_excluded(locator: &str, excluded: &[String]) -> bool {
    excluded
        .iter()
        .any(|value| locator == value || locator.starts_with(&format!("{value}/")))
}

fn portable_locator(path: &Path) -> String {
    let parts = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            Component::CurDir => None,
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                Some("<invalid>".to_owned())
            }
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        ".".to_owned()
    } else {
        parts.join("/")
    }
}

fn repository_area() -> AreaId {
    AreaId {
        kind: AreaKind::Repository,
        path: ".".to_owned(),
    }
}

fn directory_area(path: &Path) -> AreaId {
    let locator = portable_locator(path);
    if locator == "." {
        repository_area()
    } else {
        AreaId {
            kind: AreaKind::Directory,
            path: locator,
        }
    }
}

fn file_area(path: String) -> AreaId {
    AreaId {
        kind: AreaKind::File,
        path,
    }
}

fn parent_locator(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(|| ".".to_owned(), |(parent, _)| parent.to_owned())
}

fn parent_area(path: &str, kind: AreaKind) -> AreaId {
    AreaId {
        kind,
        path: parent_locator(path),
    }
}

fn diagnostic_identity(code: &str, path: &str) -> String {
    format!("inventory:{code}:{}", sha256_hex(path.as_bytes()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes).as_slice())
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(value, "{byte:02x}");
    }
    value
}

fn hash_part(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

#[cfg(test)]
mod tests {
    use super::{glob_matches, is_ignored, IgnorePattern};

    #[test]
    fn simple_ignore_patterns_are_deterministic() {
        let patterns = vec![
            IgnorePattern {
                pattern: "*.log".to_owned(),
                directory_only: false,
                negated: false,
                anchored: false,
            },
            IgnorePattern {
                pattern: "keep.log".to_owned(),
                directory_only: false,
                negated: true,
                anchored: false,
            },
        ];
        assert!(glob_matches("*.log", "debug.log"));
        assert!(is_ignored("logs/debug.log", false, &patterns));
        assert!(!is_ignored("logs/keep.log", false, &patterns));
    }
}
