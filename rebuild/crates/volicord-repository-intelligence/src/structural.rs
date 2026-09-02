use crate::inventory::{inventory_repository, InventoryError, InventoryRequest};
use crate::model::{
    AdapterIdentity, AnalysisDiagnostic, AnalysisProvenance, AnalysisSnapshot, AnalyzerIdentity,
    AreaId, AreaKind, Capability, CapabilityReport, CapabilityState, CodeEntity, CodeEntityKind,
    CoordinateConvention, Coverage, DiagnosticSeverity, FileAnalysisBasis, FreshnessBasis,
    FreshnessState, InvalidationCategory, InvalidationRecord, InventoryClassification,
    InventoryEntry, Language, LanguageExtension, ProvenanceClass, RangeMeaning, RefreshAction,
    RelationTarget, RepositorySnapshot, SourcePosition, SourceRange, StructuralFact,
    StructuralProvenance, StructuralRefresh, StructuralRelation, StructuralRelationKind,
    Uncertainty, UncertaintyLevel, UnresolvedTarget, ANALYSIS_SNAPSHOT_FORMAT_VERSION,
};
use crate::AnalysisSnapshotId;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error as StdError;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use tree_sitter::{Language as ParserLanguage, Node, Parser};

const STRUCTURAL_ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");
const STRUCTURAL_ANALYZER_NAME: &str = "tree-sitter";
const STRUCTURAL_ANALYZER_VERSION: &str = "0.26.12";

#[derive(Clone, Debug)]
pub struct StructuralAnalysisRequest<'a> {
    pub inventory: InventoryRequest<'a>,
    pub previous: Option<&'a AnalysisSnapshot>,
}

impl<'a> StructuralAnalysisRequest<'a> {
    pub fn new(inventory: InventoryRequest<'a>) -> Self {
        Self {
            inventory,
            previous: None,
        }
    }

    pub fn with_previous(mut self, previous: &'a AnalysisSnapshot) -> Self {
        self.previous = Some(previous);
        self
    }
}

#[derive(Debug)]
pub struct StructuralAnalysisError {
    message: String,
    source: Option<InventoryError>,
}

impl StructuralAnalysisError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    fn inventory(source: InventoryError) -> Self {
        Self {
            message: "repository inventory failed before structural analysis".to_owned(),
            source: Some(source),
        }
    }
}

impl fmt::Display for StructuralAnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl StdError for StructuralAnalysisError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_ref()
            .map(|source| source as &(dyn StdError + 'static))
    }
}

pub fn analyze_repository(
    request: StructuralAnalysisRequest<'_>,
) -> Result<(RepositorySnapshot, AnalysisSnapshot), StructuralAnalysisError> {
    analyze_repository_inner(request, &BTreeSet::new())
}

fn analyze_repository_inner(
    request: StructuralAnalysisRequest<'_>,
    injected_failures: &BTreeSet<Language>,
) -> Result<(RepositorySnapshot, AnalysisSnapshot), StructuralAnalysisError> {
    let root = request.inventory.root.to_path_buf();
    let previous = request.previous;
    let canonical_grounding = request.inventory.canonical_grounding.clone();
    if let Some(previous) = previous {
        canonical_grounding
            .validate_analysis_snapshot(previous)
            .map_err(|error| {
                StructuralAnalysisError::new(format!(
                    "previous Analysis Snapshot has invalid canonical grounding: {error}"
                ))
            })?;
    }
    let (repository, mut analysis) =
        inventory_repository(request.inventory).map_err(StructuralAnalysisError::inventory)?;
    let current_files = structural_source_files(&analysis.inventory.entries);
    let current_hashes = current_files
        .iter()
        .map(|entry| {
            (
                entry.area.path.clone(),
                entry.content_sha256.clone().unwrap_or_default(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let previous_hashes = previous
        .map(|snapshot| {
            structural_source_files(&snapshot.inventory.entries)
                .into_iter()
                .map(|entry| {
                    (
                        entry.area.path.clone(),
                        entry.content_sha256.clone().unwrap_or_default(),
                    )
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let changed_paths = current_hashes
        .iter()
        .filter(|(path, hash)| previous_hashes.get(*path) != Some(*hash))
        .map(|(path, _)| path.clone())
        .chain(
            previous_hashes
                .keys()
                .filter(|path| !current_hashes.contains_key(*path))
                .cloned(),
        )
        .collect::<BTreeSet<_>>();
    let previous_bases = previous
        .map(|snapshot| {
            snapshot
                .structural_bases
                .iter()
                .map(|basis| (basis.area.path.clone(), basis))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let mut invalidations = Vec::new();
    let mut parsed = Vec::new();
    let mut reused_paths = BTreeSet::new();
    let mut refresh = StructuralRefresh::default();

    for entry in &current_files {
        let language = entry
            .language
            .clone()
            .ok_or_else(|| StructuralAnalysisError::new("structural source has no language"))?;
        let adapter = adapter_identity(&language);
        let analyzer = analyzer_identity(&language);
        let current_build = build_context_fingerprint(&analysis.inventory.entries, &language);
        let previous_basis = previous_bases.get(&entry.area.path).copied();
        let decision = refresh_decision(
            entry,
            previous_basis,
            &changed_paths,
            current_build.as_deref(),
            &adapter,
            &analyzer,
        );
        if let Some((category, basis, dependency_area)) = &decision.invalidation {
            invalidations.push(InvalidationRecord {
                area: entry.area.clone(),
                language: language.clone(),
                category: *category,
                action: RefreshAction::Parsed,
                basis: basis.clone(),
                dependency_area: dependency_area.clone(),
            });
        }

        if decision.reuse {
            reused_paths.insert(entry.area.path.clone());
            refresh.reused_file_count += 1;
            continue;
        }

        let source_path = root.join(path_from_locator(&entry.area.path));
        let parse_result = if injected_failures.contains(&language) {
            Err(FileFailure::new("injected adapter failure"))
        } else {
            fs::read(&source_path)
                .map_err(|error| FileFailure::new(format!("source read failed: {error}")))
                .and_then(|source| parse_file(&language, &entry.area, &source))
        };
        match parse_result {
            Ok(mut result) => {
                result.content_sha256 = entry.content_sha256.clone().unwrap_or_default();
                result.build_context_sha256 = current_build;
                parsed.push(result);
                refresh.parsed_file_count += 1;
            }
            Err(failure) => {
                let diagnostic = failure_diagnostic(&language, &entry.area, &failure.message);
                parsed.push(ParsedFile::failed(
                    entry.area.clone(),
                    language,
                    entry.content_sha256.clone().unwrap_or_default(),
                    current_build,
                    adapter,
                    analyzer,
                    diagnostic,
                ));
                refresh.failed_file_count += 1;
                if let Some(record) = invalidations
                    .iter_mut()
                    .rev()
                    .find(|record| record.area == entry.area)
                {
                    record.action = RefreshAction::Failed;
                }
            }
        }
    }

    if let Some(previous_snapshot) = previous {
        for basis in &previous_snapshot.structural_bases {
            if !current_hashes.contains_key(&basis.area.path) {
                invalidations.push(InvalidationRecord {
                    area: basis.area.clone(),
                    language: basis.language.clone(),
                    category: InvalidationCategory::Removed,
                    action: RefreshAction::Removed,
                    basis: "the source file is absent from the current Repository Snapshot"
                        .to_owned(),
                    dependency_area: None,
                });
                refresh.removed_file_count += 1;
            }
        }
    }

    let mut structural_diagnostics = parsed
        .iter()
        .flat_map(|result| result.diagnostics.iter().cloned())
        .collect::<Vec<_>>();
    if let Some(previous_snapshot) = previous {
        structural_diagnostics.extend(
            previous_snapshot
                .diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.code.starts_with("structural.")
                        && reused_paths.contains(&diagnostic.affected_area.path)
                })
                .cloned(),
        );
    }
    for language in analysis
        .inventory
        .languages
        .iter()
        .filter(|language| language.is_structural_gate_language())
    {
        structural_diagnostics.push(language_limit_diagnostic(language));
    }
    structural_diagnostics.sort_by(|left, right| {
        (&left.affected_area, &left.code, &left.identity).cmp(&(
            &right.affected_area,
            &right.code,
            &right.identity,
        ))
    });
    structural_diagnostics.dedup_by(|left, right| left.identity == right.identity);
    let structural_counts = structural_counts(&parsed, previous, &reused_paths);

    let mut bases = parsed.iter().map(ParsedFile::basis).collect::<Vec<_>>();
    if let Some(previous_snapshot) = previous {
        for basis in &previous_snapshot.structural_bases {
            if reused_paths.contains(&basis.area.path) {
                let mut reused = basis.clone();
                reused.build_context_sha256 =
                    build_context_fingerprint(&analysis.inventory.entries, &reused.language);
                bases.push(reused);
            }
        }
    }
    bases.sort_by(|left, right| left.area.cmp(&right.area));
    invalidations.sort_by(|left, right| {
        (&left.area, left.category, left.action).cmp(&(&right.area, right.category, right.action))
    });

    analysis.diagnostics.extend(structural_diagnostics.clone());
    analysis.diagnostics.sort_by(|left, right| {
        (&left.affected_area, &left.code, &left.identity).cmp(&(
            &right.affected_area,
            &right.code,
            &right.identity,
        ))
    });
    replace_structural_capabilities(
        &mut analysis,
        &current_files,
        &bases,
        &structural_diagnostics,
        &structural_counts,
    );
    let final_identity = structural_analysis_identity(
        repository.identity,
        &analysis.repository_worktree,
        &analysis.capabilities,
        &analysis.diagnostics,
        &bases,
    )?;
    let mut facts = Vec::new();
    for result in parsed {
        if result.state != CapabilityState::Failed {
            facts.extend(materialize_file(
                result,
                repository.identity,
                final_identity,
                analysis.repository_source.clone(),
                analysis.generated_at_unix_micros,
            ));
        }
    }
    if let Some(previous_snapshot) = previous {
        facts.extend(rebind_reused_facts(
            previous_snapshot,
            &reused_paths,
            repository.identity,
            final_identity,
            analysis.generated_at_unix_micros,
        ));
    }
    facts.sort_by(|left, right| left.entity.identity.cmp(&right.entity.identity));
    for fact in &mut facts {
        fact.relations
            .sort_by(|left, right| left.identity.cmp(&right.identity));
    }
    analysis.identity = final_identity;
    analysis.structural_facts = facts;
    analysis.structural_bases = bases;
    analysis.invalidations = invalidations;
    analysis.refresh = refresh;
    analysis.format_version = ANALYSIS_SNAPSHOT_FORMAT_VERSION;
    canonical_grounding
        .validate_repository_snapshot(&repository)
        .and_then(|()| canonical_grounding.validate_analysis_snapshot(&analysis))
        .map_err(|error| {
            StructuralAnalysisError::new(format!(
                "produced analysis has invalid canonical grounding: {error}"
            ))
        })?;
    Ok((repository, analysis))
}

struct RefreshDecision {
    reuse: bool,
    invalidation: Option<(InvalidationCategory, String, Option<AreaId>)>,
}

fn refresh_decision(
    entry: &InventoryEntry,
    previous: Option<&FileAnalysisBasis>,
    changed_paths: &BTreeSet<String>,
    current_build: Option<&str>,
    adapter: &AdapterIdentity,
    analyzer: &AnalyzerIdentity,
) -> RefreshDecision {
    let Some(previous) = previous else {
        return RefreshDecision {
            reuse: false,
            invalidation: Some((
                InvalidationCategory::Added,
                "the file has no prior structural basis".to_owned(),
                None,
            )),
        };
    };
    if previous.adapter != *adapter || previous.analyzer != *analyzer {
        return RefreshDecision {
            reuse: false,
            invalidation: Some((
                InvalidationCategory::AdapterContract,
                "the structural adapter or analyzer identity changed".to_owned(),
                None,
            )),
        };
    }
    if entry.content_sha256.as_deref() != Some(previous.content_sha256.as_str()) {
        return RefreshDecision {
            reuse: false,
            invalidation: Some((
                InvalidationCategory::FileContent,
                "the source content fingerprint changed".to_owned(),
                Some(entry.area.clone()),
            )),
        };
    }
    if previous.build_context_sha256.as_deref() != current_build {
        return RefreshDecision {
            reuse: false,
            invalidation: Some((
                InvalidationCategory::BuildContext,
                "a language-relevant manifest or build-context fingerprint changed".to_owned(),
                None,
            )),
        };
    }
    if previous.state == CapabilityState::Failed {
        return RefreshDecision {
            reuse: false,
            invalidation: Some((
                InvalidationCategory::PriorFailure,
                "the prior structural attempt failed and is being retried".to_owned(),
                None,
            )),
        };
    }
    if let Some(changed) = changed_paths.iter().find(|changed| {
        previous
            .dependency_locators
            .iter()
            .any(|dependency| dependency_matches_path(dependency, changed, &entry.area.path))
    }) {
        return RefreshDecision {
            reuse: false,
            invalidation: Some((
                InvalidationCategory::Dependency,
                format!("declared import/include dependency changed: {changed}"),
                Some(AreaId {
                    kind: AreaKind::File,
                    path: changed.clone(),
                }),
            )),
        };
    }
    RefreshDecision {
        reuse: true,
        invalidation: None,
    }
}

fn dependency_matches_path(dependency: &str, changed: &str, owner: &str) -> bool {
    let dependency = dependency
        .trim_matches(|character| character == '"' || character == '<' || character == '>');
    let changed_path = Path::new(changed);
    let changed_file = changed_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(changed);
    if dependency == changed || dependency == changed_file || changed.ends_with(dependency) {
        return true;
    }
    if dependency.starts_with('.') {
        let owner_parent = Path::new(owner).parent().unwrap_or_else(|| Path::new(""));
        let joined = normalize_relative(owner_parent.join(dependency));
        let changed_without_extension = changed_path
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");
        return joined == changed || joined.trim_end_matches(".js") == changed_without_extension;
    }
    let module_path = dependency.replace("::", "/").replace('.', "/");
    changed.contains(&module_path)
        || changed_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| module_path.ends_with(stem))
}

fn normalize_relative(path: PathBuf) -> String {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                let _ = components.pop();
            }
            Component::Normal(value) => components.push(value.to_string_lossy().into_owned()),
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    components.join("/")
}

fn structural_source_files(entries: &[InventoryEntry]) -> Vec<InventoryEntry> {
    entries
        .iter()
        .filter(|entry| {
            entry
                .classifications
                .contains(&InventoryClassification::Included)
                && entry
                    .language
                    .as_ref()
                    .is_some_and(Language::is_structural_gate_language)
                && entry.content_sha256.is_some()
        })
        .cloned()
        .collect()
}

fn build_context_fingerprint(entries: &[InventoryEntry], language: &Language) -> Option<String> {
    let mut relevant = entries
        .iter()
        .filter(|entry| {
            entry.content_sha256.is_some() && manifest_matches(language, &entry.area.path)
        })
        .map(|entry| {
            (
                entry.area.path.as_str(),
                entry.content_sha256.as_deref().unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    if relevant.is_empty() {
        return None;
    }
    relevant.sort();
    let mut hasher = Sha256::new();
    for (path, hash) in relevant {
        hash_part(&mut hasher, path.as_bytes());
        hash_part(&mut hasher, hash.as_bytes());
    }
    Some(hex_digest(&hasher.finalize()))
}

fn manifest_matches(language: &Language, path: &str) -> bool {
    let name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    match language {
        Language::Java => matches!(
            name,
            "pom.xml"
                | "build.gradle"
                | "build.gradle.kts"
                | "settings.gradle"
                | "settings.gradle.kts"
        ),
        Language::Python => matches!(
            name,
            "pyproject.toml" | "setup.py" | "setup.cfg" | "requirements.txt"
        ),
        Language::JavaScript => matches!(
            name,
            "package.json" | "package-lock.json" | "pnpm-workspace.yaml" | "yarn.lock"
        ),
        Language::TypeScript => matches!(
            name,
            "package.json"
                | "package-lock.json"
                | "pnpm-workspace.yaml"
                | "yarn.lock"
                | "tsconfig.json"
        ),
        Language::C | Language::Cpp => matches!(name, "CMakeLists.txt" | "compile_commands.json"),
        Language::Rust => matches!(name, "Cargo.toml" | "Cargo.lock"),
        _ => false,
    }
}

#[derive(Debug)]
struct FileFailure {
    message: String,
}

impl FileFailure {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Clone)]
struct EntityDraft {
    key: usize,
    kind: CodeEntityKind,
    name: String,
    qualified_name: String,
    start: tree_sitter::Point,
    end: tree_sitter::Point,
    parser_node_kind: String,
    diagnostic_ids: Vec<String>,
}

#[derive(Clone)]
enum DraftTarget {
    Entity(usize),
    Unresolved(String),
}

#[derive(Clone)]
struct RelationDraft {
    source: usize,
    target: DraftTarget,
    kind: StructuralRelationKind,
    start_byte: usize,
    start: tree_sitter::Point,
    end: tree_sitter::Point,
}

struct ParsedFile {
    area: AreaId,
    language: Language,
    content_sha256: String,
    build_context_sha256: Option<String>,
    adapter: AdapterIdentity,
    analyzer: AnalyzerIdentity,
    entities: Vec<EntityDraft>,
    relations: Vec<RelationDraft>,
    dependencies: Vec<String>,
    diagnostics: Vec<AnalysisDiagnostic>,
    state: CapabilityState,
}

impl ParsedFile {
    fn failed(
        area: AreaId,
        language: Language,
        content_sha256: String,
        build_context_sha256: Option<String>,
        adapter: AdapterIdentity,
        analyzer: AnalyzerIdentity,
        diagnostic: AnalysisDiagnostic,
    ) -> Self {
        Self {
            area,
            language,
            content_sha256,
            build_context_sha256,
            adapter,
            analyzer,
            entities: Vec::new(),
            relations: Vec::new(),
            dependencies: Vec::new(),
            diagnostics: vec![diagnostic],
            state: CapabilityState::Failed,
        }
    }

    fn basis(&self) -> FileAnalysisBasis {
        FileAnalysisBasis {
            area: self.area.clone(),
            language: self.language.clone(),
            content_sha256: self.content_sha256.clone(),
            adapter: self.adapter.clone(),
            analyzer: self.analyzer.clone(),
            dependency_locators: self.dependencies.clone(),
            build_context_sha256: self.build_context_sha256.clone(),
            state: self.state,
            diagnostic_ids: self
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.identity.clone())
                .collect(),
        }
    }
}

struct ParseState<'source> {
    source: &'source [u8],
    area: &'source AreaId,
    language: &'source Language,
    entities: Vec<EntityDraft>,
    relations: Vec<RelationDraft>,
    dependencies: BTreeSet<String>,
    diagnostics: Vec<AnalysisDiagnostic>,
    base_parent: usize,
    base_prefix: String,
}

#[derive(Clone)]
struct VisitContext {
    parent: usize,
    prefix: String,
    callable: Option<usize>,
}

fn parse_file(
    language: &Language,
    area: &AreaId,
    source: &[u8],
) -> Result<ParsedFile, FileFailure> {
    let parser_language = parser_language(language)?;
    let mut parser = Parser::new();
    parser
        .set_language(&parser_language)
        .map_err(|error| FileFailure::new(format!("parser language setup failed: {error}")))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| FileFailure::new("parser was cancelled before producing a tree"))?;
    let adapter = adapter_identity(language);
    let mut state = ParseState {
        source,
        area,
        language,
        entities: Vec::new(),
        relations: Vec::new(),
        dependencies: BTreeSet::new(),
        diagnostics: Vec::new(),
        base_parent: 0,
        base_prefix: String::new(),
    };
    let root = tree.root_node();
    let file_key = state.add_entity(
        CodeEntityKind::File,
        area.path.clone(),
        area.path.clone(),
        root,
        None,
    );
    state.base_parent = file_key;
    let (base_parent, base_prefix) = add_language_root(&mut state, root, file_key);
    state.base_parent = base_parent;
    state.base_prefix = base_prefix.clone();
    let context = VisitContext {
        parent: base_parent,
        prefix: base_prefix,
        callable: None,
    };
    visit_children(root, &mut state, &context);
    let mut state_kind = CapabilityState::Available;
    if root.has_error() {
        let diagnostic = parse_error_diagnostic(language, area);
        let diagnostic_id = diagnostic.identity.clone();
        for entity in &mut state.entities {
            entity.diagnostic_ids.push(diagnostic_id.clone());
        }
        state.diagnostics.push(diagnostic);
        state_kind = CapabilityState::Partial;
    }
    if has_construct_limit(language, root) {
        state
            .diagnostics
            .push(construct_limit_diagnostic(language, area));
        state_kind = CapabilityState::Partial;
    }
    let mut dependencies = state.dependencies.into_iter().collect::<Vec<_>>();
    dependencies.sort();
    state.relations.sort_by(|left, right| {
        (
            left.source,
            relation_kind_label(&left.kind),
            draft_target_label(&left.target),
            left.start_byte,
        )
            .cmp(&(
                right.source,
                relation_kind_label(&right.kind),
                draft_target_label(&right.target),
                right.start_byte,
            ))
    });
    state.relations.dedup_by(|left, right| {
        left.source == right.source
            && left.kind == right.kind
            && draft_target_label(&left.target) == draft_target_label(&right.target)
            && left.start_byte == right.start_byte
    });
    Ok(ParsedFile {
        area: area.clone(),
        language: language.clone(),
        content_sha256: String::new(),
        build_context_sha256: None,
        adapter,
        analyzer: analyzer_identity(language),
        entities: state.entities,
        relations: state.relations,
        dependencies,
        diagnostics: state.diagnostics,
        state: state_kind,
    })
}

impl ParseState<'_> {
    fn add_entity(
        &mut self,
        kind: CodeEntityKind,
        name: String,
        qualified_name: String,
        node: Node<'_>,
        parent: Option<usize>,
    ) -> usize {
        let key = self.entities.len();
        let range_start = exact_named_token(node, self.source, &name).unwrap_or(node);
        self.entities.push(EntityDraft {
            key,
            kind,
            name,
            qualified_name,
            start: range_start.start_position(),
            end: node.end_position(),
            parser_node_kind: node.kind().to_owned(),
            diagnostic_ids: Vec::new(),
        });
        if let Some(parent) = parent {
            self.relations.push(RelationDraft {
                source: parent,
                target: DraftTarget::Entity(key),
                kind: StructuralRelationKind::Contains,
                start_byte: node.start_byte(),
                start: node.start_position(),
                end: node.end_position(),
            });
            self.relations.push(RelationDraft {
                source: parent,
                target: DraftTarget::Entity(key),
                kind: StructuralRelationKind::Declares,
                start_byte: node.start_byte(),
                start: node.start_position(),
                end: node.end_position(),
            });
        }
        key
    }

    fn add_unresolved_relation(
        &mut self,
        source: usize,
        kind: StructuralRelationKind,
        target: String,
        node: Node<'_>,
    ) {
        if target.trim().is_empty() {
            return;
        }
        self.relations.push(RelationDraft {
            source,
            target: DraftTarget::Unresolved(target),
            kind,
            start_byte: node.start_byte(),
            start: node.start_position(),
            end: node.end_position(),
        });
    }
}

fn add_language_root(state: &mut ParseState<'_>, root: Node<'_>, file: usize) -> (usize, String) {
    match state.language {
        Language::Java => {
            if let Some(package) = find_first_kind(root, "package_declaration") {
                let text = node_text(state.source, package)
                    .trim()
                    .trim_start_matches("package")
                    .trim()
                    .trim_end_matches(';')
                    .to_owned();
                if !text.is_empty() {
                    let key = state.add_entity(
                        CodeEntityKind::Package,
                        text.clone(),
                        text.clone(),
                        package,
                        Some(file),
                    );
                    return (key, text);
                }
            }
            (file, String::new())
        }
        Language::Python => {
            let module = module_name_from_path(&state.area.path);
            let key = state.add_entity(
                CodeEntityKind::Module,
                module.clone(),
                module.clone(),
                root,
                Some(file),
            );
            (key, module)
        }
        Language::Rust => (file, String::new()),
        _ => (file, String::new()),
    }
}

fn visit_children(node: Node<'_>, state: &mut ParseState<'_>, context: &VisitContext) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        visit_node(child, state, context);
    }
}

fn visit_node(node: Node<'_>, state: &mut ParseState<'_>, context: &VisitContext) {
    if matches!(node.kind(), "package_declaration") {
        return;
    }
    if let Some((kind, name)) = declaration(node, state, context) {
        let qualified = qualified_name(
            state.language,
            &state.area.path,
            context,
            &kind,
            &name,
            node,
        );
        let key = state.add_entity(
            kind.clone(),
            name.clone(),
            qualified.clone(),
            node,
            Some(context.parent),
        );
        declaration_relations(node, state, key, &kind);
        if kind == CodeEntityKind::Test {
            state.add_unresolved_relation(
                key,
                StructuralRelationKind::Tests,
                language_label(state.language).to_owned(),
                node,
            );
        }
        let child_context = VisitContext {
            parent: if is_container(&kind) {
                key
            } else {
                context.parent
            },
            prefix: if is_container(&kind) {
                qualified
            } else {
                context.prefix.clone()
            },
            callable: if is_callable(&kind) {
                Some(key)
            } else {
                context.callable
            },
        };
        visit_children(node, state, &child_context);
        return;
    }

    if state.language == &Language::Rust && node.kind() == "impl_item" {
        let type_name = node
            .child_by_field_name("type")
            .map(|child| node_text(state.source, child).trim().to_owned())
            .unwrap_or_default();
        let trait_name = node
            .child_by_field_name("trait")
            .map(|child| node_text(state.source, child).trim().to_owned());
        let parent = state
            .entities
            .iter()
            .find(|entity| entity.name == type_name)
            .map_or(context.parent, |entity| entity.key);
        if let Some(trait_name) = trait_name {
            state.add_unresolved_relation(
                parent,
                StructuralRelationKind::Implements,
                trait_name,
                node,
            );
        }
        let impl_context = VisitContext {
            parent,
            prefix: type_name,
            callable: None,
        };
        visit_children(node, state, &impl_context);
        return;
    }

    syntax_relation(node, state, context);
    visit_children(node, state, context);
}

fn declaration(
    node: Node<'_>,
    state: &ParseState<'_>,
    context: &VisitContext,
) -> Option<(CodeEntityKind, String)> {
    let text = node_text(state.source, node);
    let result = match state.language {
        Language::Java => match node.kind() {
            "class_declaration" => {
                named(node, state.source).map(|name| (CodeEntityKind::Class, name))
            }
            "interface_declaration" => {
                named(node, state.source).map(|name| (CodeEntityKind::Interface, name))
            }
            "method_declaration" | "constructor_declaration" => {
                named(node, state.source).map(|name| {
                    let kind = if is_test_name(&state.area.path, &name, text) {
                        CodeEntityKind::Test
                    } else {
                        CodeEntityKind::Method
                    };
                    (kind, name)
                })
            }
            "field_declaration" => descendant_text(
                node,
                state.source,
                &["variable_declarator"],
                &["identifier"],
            )
            .map(|name| (CodeEntityKind::Field, name)),
            _ => None,
        },
        Language::Python => match node.kind() {
            "class_definition" => {
                named(node, state.source).map(|name| (CodeEntityKind::Class, name))
            }
            "function_definition" => named(node, state.source).map(|name| {
                let kind = if is_test_name(&state.area.path, &name, text) {
                    CodeEntityKind::Test
                } else if state
                    .entities
                    .get(context.parent)
                    .is_some_and(|entity| entity.kind == CodeEntityKind::Class)
                {
                    CodeEntityKind::Method
                } else {
                    CodeEntityKind::Function
                };
                (kind, name)
            }),
            "assignment" => assignment_field(node, state.source, "self.")
                .map(|name| (CodeEntityKind::Field, name)),
            _ => None,
        },
        Language::JavaScript => match node.kind() {
            "class_declaration" => {
                named(node, state.source).map(|name| (CodeEntityKind::Class, name))
            }
            "method_definition" => {
                named(node, state.source).map(|name| (CodeEntityKind::Method, name))
            }
            "function_declaration" => named(node, state.source).map(|name| {
                let kind = if is_test_name(&state.area.path, &name, text) {
                    CodeEntityKind::Test
                } else {
                    CodeEntityKind::Function
                };
                (kind, name)
            }),
            "public_field_definition" => {
                named(node, state.source).map(|name| (CodeEntityKind::Field, name))
            }
            "assignment_expression" => assignment_field(node, state.source, "this.")
                .map(|name| (CodeEntityKind::Field, name)),
            _ => None,
        },
        Language::TypeScript => match node.kind() {
            "interface_declaration" => {
                named(node, state.source).map(|name| (CodeEntityKind::Interface, name))
            }
            "type_alias_declaration" => {
                named(node, state.source).map(|name| (CodeEntityKind::Type, name))
            }
            "enum_declaration" => {
                named(node, state.source).map(|name| (CodeEntityKind::Enum, name))
            }
            "class_declaration" | "abstract_class_declaration" => {
                named(node, state.source).map(|name| (CodeEntityKind::Class, name))
            }
            "method_definition" | "method_signature" => {
                named(node, state.source).map(|name| (CodeEntityKind::Method, name))
            }
            "function_declaration" | "function_signature" => {
                named(node, state.source).map(|name| {
                    let kind = if is_test_name(&state.area.path, &name, text) {
                        CodeEntityKind::Test
                    } else {
                        CodeEntityKind::Function
                    };
                    (kind, name)
                })
            }
            "public_field_definition" | "property_signature" => {
                named(node, state.source).map(|name| (CodeEntityKind::Field, name))
            }
            "required_parameter" | "optional_parameter" if parameter_is_field(text) => {
                parameter_name(node, state.source).map(|name| (CodeEntityKind::Field, name))
            }
            _ => None,
        },
        Language::C => c_declaration(node, state.source, &state.area.path, false),
        Language::Cpp => cpp_declaration(node, state, context, text),
        Language::Rust => match node.kind() {
            "mod_item" => named(node, state.source).map(|name| (CodeEntityKind::Module, name)),
            "trait_item" => named(node, state.source).map(|name| (CodeEntityKind::Trait, name)),
            "struct_item" => named(node, state.source).map(|name| (CodeEntityKind::Struct, name)),
            "enum_item" => named(node, state.source).map(|name| (CodeEntityKind::Enum, name)),
            "type_item" => named(node, state.source).map(|name| (CodeEntityKind::Type, name)),
            "function_signature_item" => {
                named(node, state.source).map(|name| (CodeEntityKind::Method, name))
            }
            "function_item" => named(node, state.source).map(|name| {
                let kind = if is_test_name(&state.area.path, &name, text)
                    || context.prefix.split('.').next_back() == Some("tests")
                {
                    CodeEntityKind::Test
                } else if state.entities.get(context.parent).is_some_and(|entity| {
                    matches!(entity.kind, CodeEntityKind::File | CodeEntityKind::Module)
                }) {
                    CodeEntityKind::Function
                } else {
                    CodeEntityKind::Method
                };
                (kind, name)
            }),
            "field_declaration" => {
                named(node, state.source).map(|name| (CodeEntityKind::Field, name))
            }
            _ => None,
        },
        _ => None,
    };
    result
}

fn c_declaration(
    node: Node<'_>,
    source: &[u8],
    path: &str,
    cpp: bool,
) -> Option<(CodeEntityKind, String)> {
    match node.kind() {
        "struct_specifier" => named(node, source).map(|name| (CodeEntityKind::Struct, name)),
        "enum_specifier" => named(node, source).map(|name| (CodeEntityKind::Enum, name)),
        "function_definition" => callable_name(node, source).map(|name| {
            let kind = if is_test_name(path, &name, node_text(source, node)) {
                CodeEntityKind::Test
            } else if cpp {
                CodeEntityKind::Method
            } else {
                CodeEntityKind::Function
            };
            (kind, name)
        }),
        "declaration" if contains_kind(node, "function_declarator") => callable_name(node, source)
            .map(|name| {
                (
                    if cpp {
                        CodeEntityKind::Method
                    } else {
                        CodeEntityKind::Function
                    },
                    name,
                )
            }),
        "field_declaration" if !contains_kind(node, "function_declarator") => {
            descendant_text(node, source, &["field_identifier", "identifier"], &[])
                .map(|name| (CodeEntityKind::Field, name))
        }
        "type_definition" => descendant_text(node, source, &["type_identifier"], &[])
            .map(|name| (CodeEntityKind::Type, name)),
        _ => None,
    }
}

fn cpp_declaration(
    node: Node<'_>,
    state: &ParseState<'_>,
    context: &VisitContext,
    text: &str,
) -> Option<(CodeEntityKind, String)> {
    let source = state.source;
    let path = &state.area.path;
    match node.kind() {
        "namespace_definition" => named(node, source).map(|name| (CodeEntityKind::Namespace, name)),
        "class_specifier" => named(node, source).map(|name| (CodeEntityKind::Class, name)),
        "struct_specifier" => named(node, source).map(|name| (CodeEntityKind::Struct, name)),
        "enum_specifier" => named(node, source).map(|name| (CodeEntityKind::Enum, name)),
        "alias_declaration" | "type_definition" => named(node, source)
            .or_else(|| descendant_text(node, source, &["type_identifier"], &[]))
            .map(|name| (CodeEntityKind::Type, name)),
        "function_definition" => callable_name(node, source)
            .map(|name| (cpp_callable_kind(node, state, context, text, &name), name)),
        "field_declaration" if contains_kind(node, "function_declarator") => {
            callable_name(node, source)
                .map(|name| (cpp_callable_kind(node, state, context, text, &name), name))
        }
        "declaration" if contains_kind(node, "function_declarator") => callable_name(node, source)
            .map(|name| (cpp_callable_kind(node, state, context, text, &name), name)),
        "field_declaration" => {
            descendant_text(node, source, &["field_identifier", "identifier"], &[])
                .map(|name| (CodeEntityKind::Field, name))
        }
        _ => c_declaration(node, source, path, false),
    }
}

fn cpp_callable_kind(
    node: Node<'_>,
    state: &ParseState<'_>,
    context: &VisitContext,
    text: &str,
    name: &str,
) -> CodeEntityKind {
    if is_test_name(&state.area.path, name, text) {
        return CodeEntityKind::Test;
    }
    let declared_in_type = state.entities.get(context.parent).is_some_and(|entity| {
        matches!(entity.kind, CodeEntityKind::Class | CodeEntityKind::Struct)
    });
    if declared_in_type || contains_kind(node, "qualified_identifier") {
        CodeEntityKind::Method
    } else {
        CodeEntityKind::Function
    }
}

fn declaration_relations(
    node: Node<'_>,
    state: &mut ParseState<'_>,
    entity: usize,
    kind: &CodeEntityKind,
) {
    if !matches!(kind, CodeEntityKind::Class | CodeEntityKind::Interface) {
        return;
    }
    let text = node_text(state.source, node);
    match state.language {
        Language::Java | Language::TypeScript => {
            for target in keyword_targets(text, "extends") {
                state.add_unresolved_relation(
                    entity,
                    StructuralRelationKind::Inherits,
                    target,
                    node,
                );
            }
            for target in keyword_targets(text, "implements") {
                state.add_unresolved_relation(
                    entity,
                    StructuralRelationKind::Implements,
                    target,
                    node,
                );
            }
        }
        Language::Python => {
            if let Some(superclasses) = node.child_by_field_name("superclasses") {
                for target in identifier_tokens(node_text(state.source, superclasses)) {
                    state.add_unresolved_relation(
                        entity,
                        StructuralRelationKind::Inherits,
                        target,
                        superclasses,
                    );
                }
            }
        }
        Language::Cpp => {
            if let Some(clause) = find_first_kind(node, "base_class_clause") {
                let targets = identifier_tokens(node_text(state.source, clause));
                if let Some(target) = targets.last() {
                    state.add_unresolved_relation(
                        entity,
                        StructuralRelationKind::Inherits,
                        target.clone(),
                        clause,
                    );
                }
            }
        }
        _ => {}
    }
}

fn syntax_relation(node: Node<'_>, state: &mut ParseState<'_>, context: &VisitContext) {
    let source_entity = context.callable.unwrap_or(context.parent);
    match (state.language, node.kind()) {
        (Language::Java, "import_declaration") => {
            let target = node_text(state.source, node)
                .trim()
                .trim_start_matches("import")
                .trim()
                .trim_start_matches("static")
                .trim()
                .trim_end_matches(';')
                .to_owned();
            state.dependencies.insert(target.clone());
            state.add_unresolved_relation(
                context.parent,
                StructuralRelationKind::Imports,
                target,
                node,
            );
        }
        (Language::Python, "import_statement" | "import_from_statement") => {
            let text = node_text(state.source, node).trim();
            let target = if let Some(rest) = text.strip_prefix("from ") {
                rest.split_whitespace().next().unwrap_or_default()
            } else {
                text.strip_prefix("import ")
                    .unwrap_or(text)
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
            }
            .trim_end_matches(',')
            .to_owned();
            state.dependencies.insert(target.clone());
            state.add_unresolved_relation(
                context.parent,
                StructuralRelationKind::Imports,
                target,
                node,
            );
        }
        (Language::JavaScript | Language::TypeScript, "import_statement") => {
            if let Some(target) = string_literal(node, state.source) {
                state.dependencies.insert(target.clone());
                state.add_unresolved_relation(
                    context.parent,
                    StructuralRelationKind::Imports,
                    target,
                    node,
                );
            }
        }
        (Language::JavaScript | Language::TypeScript, "export_statement") => {
            if let Some(target) = export_name(node, state.source) {
                state.add_unresolved_relation(
                    context.parent,
                    StructuralRelationKind::Exports,
                    target,
                    node,
                );
            }
        }
        (Language::C | Language::Cpp, "preproc_include") => {
            let target = node_text(state.source, node)
                .trim()
                .trim_start_matches("#include")
                .trim()
                .trim_matches(|character| character == '"' || character == '<' || character == '>')
                .to_owned();
            state.dependencies.insert(target.clone());
            state.add_unresolved_relation(
                context.parent,
                StructuralRelationKind::Includes,
                target,
                node,
            );
        }
        (Language::Rust, "use_declaration") => {
            let target = node_text(state.source, node)
                .trim()
                .trim_start_matches("use")
                .trim()
                .trim_end_matches(';')
                .to_owned();
            state.dependencies.insert(target.clone());
            state.add_unresolved_relation(0, StructuralRelationKind::Imports, target, node);
        }
        (Language::Java, "method_invocation") => {
            if let Some(target) = java_call_name(node, state.source) {
                state.add_unresolved_relation(
                    source_entity,
                    StructuralRelationKind::CallsSyntactically,
                    target,
                    node,
                );
            }
        }
        (Language::Python, "call")
        | (
            Language::JavaScript
            | Language::TypeScript
            | Language::C
            | Language::Cpp
            | Language::Rust,
            "call_expression",
        ) => {
            if let Some(function) = node.child_by_field_name("function") {
                let target = node_text(state.source, function).trim().to_owned();
                state.add_unresolved_relation(
                    source_entity,
                    StructuralRelationKind::CallsSyntactically,
                    target,
                    node,
                );
            }
        }
        _ => {}
    }
}

fn materialize_file(
    parsed: ParsedFile,
    repository_snapshot: crate::RepositorySnapshotId,
    analysis_snapshot: AnalysisSnapshotId,
    source: crate::model::CanonicalSourceRef,
    generated_at: i64,
) -> Vec<StructuralFact> {
    let freshness = FreshnessBasis {
        state: FreshnessState::Current,
        repository_snapshot,
        compared_repository_snapshot: None,
        reason: None,
    };
    let mut identities = BTreeMap::new();
    for entity in &parsed.entities {
        identities.insert(
            entity.key,
            entity_identity(
                repository_snapshot,
                analysis_snapshot,
                &parsed.language,
                &parsed.area,
                &entity.kind,
                &entity.qualified_name,
                (point_basis(entity.start), point_basis(entity.end)),
            ),
        );
    }
    let mut outgoing: BTreeMap<usize, Vec<StructuralRelation>> = BTreeMap::new();
    for relation in parsed.relations {
        let Some(source_identity) = identities.get(&relation.source).cloned() else {
            continue;
        };
        let target = match relation.target {
            DraftTarget::Entity(key) => identities
                .get(&key)
                .cloned()
                .map(RelationTarget::ResolvedEntity)
                .unwrap_or_else(|| {
                    unresolved_target("missing normalized target", &parsed.language)
                }),
            DraftTarget::Unresolved(display) => unresolved_target(&display, &parsed.language),
        };
        let range = source_range(
            source.clone(),
            repository_snapshot,
            &parsed.area,
            relation.start,
            relation.end,
            &parsed.adapter,
            RangeMeaning::Symbol,
        );
        let identity = relation_identity(
            repository_snapshot,
            analysis_snapshot,
            &source_identity,
            &target,
            &relation.kind,
            point_basis(relation.start),
            point_basis(relation.end),
        );
        outgoing
            .entry(relation.source)
            .or_default()
            .push(StructuralRelation {
                identity,
                repository_snapshot,
                analysis_snapshot,
                source_entity: source_identity,
                target,
                kind: relation.kind,
                supporting_range: Some(range),
                diagnostics: Vec::new(),
                uncertainty: Uncertainty::none(),
                freshness: freshness.clone(),
                extensions: Vec::new(),
            });
    }
    parsed
        .entities
        .into_iter()
        .filter_map(|entity| {
            let identity = identities.get(&entity.key)?.clone();
            let range = source_range(
                source.clone(),
                repository_snapshot,
                &parsed.area,
                entity.start,
                entity.end,
                &parsed.adapter,
                if entity.kind == CodeEntityKind::File {
                    RangeMeaning::WholeFile
                } else {
                    RangeMeaning::Entity
                },
            );
            let mut values = BTreeMap::new();
            values.insert(
                "parser_node_kind".to_owned(),
                Value::String(entity.parser_node_kind.clone()),
            );
            let extension = LanguageExtension {
                language: parsed.language.clone(),
                owning_adapter: parsed.adapter.clone(),
                namespace: format!("{}.syntax", language_label(&parsed.language)),
                values,
                source_range: Some(range.clone()),
                diagnostics: entity.diagnostic_ids.clone(),
            };
            let code_entity = CodeEntity {
                identity,
                repository_snapshot,
                analysis_snapshot,
                language: parsed.language.clone(),
                area: parsed.area.clone(),
                kind: entity.kind,
                source: source.clone(),
                source_range: Some(range),
                display_name: Some(entity.name),
                qualified_name: Some(entity.qualified_name),
                diagnostics: entity.diagnostic_ids,
                uncertainty: Uncertainty::none(),
                freshness: freshness.clone(),
                extensions: vec![extension],
                canonical_links: vec![crate::CanonicalReference::Source(source.clone())],
            };
            Some(StructuralFact {
                entity: code_entity,
                relations: outgoing.remove(&entity.key).unwrap_or_default(),
                provenance: StructuralProvenance {
                    adapter: parsed.adapter.clone(),
                    analyzer: parsed.analyzer.clone(),
                    supported_construct: entity.parser_node_kind,
                    analysis: AnalysisProvenance {
                        class: ProvenanceClass::StructuralFact,
                        repository_snapshot,
                        analysis_snapshot,
                        adapter: Some(parsed.adapter.clone()),
                        analyzer: Some(parsed.analyzer.clone()),
                        source_basis: vec![source.clone()],
                        observed_or_generated_at_unix_micros: generated_at,
                    },
                },
            })
        })
        .collect()
}

fn rebind_reused_facts(
    previous: &AnalysisSnapshot,
    reused_paths: &BTreeSet<String>,
    repository_snapshot: crate::RepositorySnapshotId,
    analysis_snapshot: AnalysisSnapshotId,
    generated_at: i64,
) -> Vec<StructuralFact> {
    let selected = previous
        .structural_facts
        .iter()
        .filter(|fact| reused_paths.contains(&fact.entity.area.path))
        .collect::<Vec<_>>();
    let mut identity_map = BTreeMap::new();
    for fact in &selected {
        let entity = &fact.entity;
        let (start, end) = entity.source_range.as_ref().map_or((0, 0), |range| {
            (position_basis(range.start), position_basis(range.end))
        });
        identity_map.insert(
            entity.identity.clone(),
            entity_identity(
                repository_snapshot,
                analysis_snapshot,
                &entity.language,
                &entity.area,
                &entity.kind,
                entity.qualified_name.as_deref().unwrap_or_default(),
                (start, end),
            ),
        );
    }
    selected
        .into_iter()
        .filter_map(|fact| {
            let mut rebound = fact.clone();
            rebound.entity.identity = identity_map.get(&fact.entity.identity)?.clone();
            rebound.entity.repository_snapshot = repository_snapshot;
            rebound.entity.analysis_snapshot = analysis_snapshot;
            rebind_range(rebound.entity.source_range.as_mut(), repository_snapshot);
            rebound.entity.freshness = current_freshness(repository_snapshot);
            for extension in &mut rebound.entity.extensions {
                rebind_range(extension.source_range.as_mut(), repository_snapshot);
            }
            rebound.provenance.analysis.repository_snapshot = repository_snapshot;
            rebound.provenance.analysis.analysis_snapshot = analysis_snapshot;
            rebound
                .provenance
                .analysis
                .observed_or_generated_at_unix_micros = generated_at;
            for relation in &mut rebound.relations {
                relation.repository_snapshot = repository_snapshot;
                relation.analysis_snapshot = analysis_snapshot;
                relation.source_entity = rebound.entity.identity.clone();
                if let RelationTarget::ResolvedEntity(target) = &mut relation.target {
                    if let Some(replacement) = identity_map.get(target) {
                        *target = replacement.clone();
                    }
                }
                rebind_range(relation.supporting_range.as_mut(), repository_snapshot);
                relation.freshness = current_freshness(repository_snapshot);
                relation.identity = relation_identity(
                    repository_snapshot,
                    analysis_snapshot,
                    &relation.source_entity,
                    &relation.target,
                    &relation.kind,
                    relation
                        .supporting_range
                        .as_ref()
                        .map_or(0, |range| position_basis(range.start)),
                    relation
                        .supporting_range
                        .as_ref()
                        .map_or(0, |range| position_basis(range.end)),
                );
            }
            Some(rebound)
        })
        .collect()
}

fn replace_structural_capabilities(
    analysis: &mut AnalysisSnapshot,
    files: &[InventoryEntry],
    bases: &[FileAnalysisBasis],
    diagnostics: &[AnalysisDiagnostic],
    counts: &BTreeMap<Language, (u64, u64)>,
) {
    analysis.capabilities.retain(|report| {
        report.capability != Capability::Structural
            || report
                .language
                .as_ref()
                .is_some_and(|language| !language.is_structural_gate_language())
    });
    for language in &analysis.inventory.languages {
        if !language.is_structural_gate_language() {
            continue;
        }
        let language_files = files
            .iter()
            .filter(|entry| entry.language.as_ref() == Some(language))
            .map(|entry| entry.area.clone())
            .collect::<Vec<_>>();
        let language_bases = bases
            .iter()
            .filter(|basis| &basis.language == language)
            .collect::<Vec<_>>();
        let failed = language_bases
            .iter()
            .filter(|basis| basis.state == CapabilityState::Failed)
            .map(|basis| basis.area.clone())
            .collect::<Vec<_>>();
        let included = language_bases
            .iter()
            .filter(|basis| basis.state != CapabilityState::Failed)
            .map(|basis| basis.area.clone())
            .collect::<Vec<_>>();
        let partial = language_bases
            .iter()
            .any(|basis| basis.state == CapabilityState::Partial);
        let state = if !language_files.is_empty() && failed.len() == language_files.len() {
            CapabilityState::Failed
        } else if partial || !failed.is_empty() {
            CapabilityState::Partial
        } else {
            CapabilityState::Available
        };
        let language_diagnostics = diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.adapter.as_ref() == Some(&adapter_identity(language))
                    || language_files.contains(&diagnostic.affected_area)
            })
            .map(|diagnostic| diagnostic.identity.clone())
            .collect::<Vec<_>>();
        let (entity_count, relation_count) = counts.get(language).copied().unwrap_or_default();
        analysis.capabilities.push(CapabilityReport {
            repository_snapshot: analysis.repository_snapshot,
            language: Some(language.clone()),
            area: repository_area(),
            capability: Capability::Structural,
            state,
            reason: (state != CapabilityState::Available).then_some(
                "one or more files have parser errors, unsupported constructs, or adapter failures"
                    .to_owned(),
            ),
            usable_remainder: Some(
                "successful file-scoped structural facts and inventory remain usable".to_owned(),
            ),
            user_visible_consequence: (state != CapabilityState::Available).then_some(
                "inspect failed/partial coverage and diagnostics before navigation".to_owned(),
            ),
            coverage: Coverage {
                included,
                failed,
                covered_file_count: language_bases
                    .iter()
                    .filter(|basis| basis.state != CapabilityState::Failed)
                    .count() as u64,
                covered_entity_count: entity_count,
                covered_relation_count: relation_count,
                ..Coverage::default()
            },
            diagnostics: language_diagnostics,
            adapter: Some(adapter_identity(language)),
            analyzer: Some(analyzer_identity(language)),
            provenance_class: ProvenanceClass::StructuralFact,
            observed_at_unix_micros: analysis.generated_at_unix_micros,
            freshness: analysis.freshness.clone(),
            uncertainty: if state == CapabilityState::Available {
                Uncertainty::none()
            } else {
                Uncertainty {
                    level: UncertaintyLevel::Medium,
                    reasons: vec!["coverage is bounded by the listed diagnostics".to_owned()],
                }
            },
        });
    }
    analysis.capabilities.sort_by(|left, right| {
        (&left.language, left.capability, &left.area).cmp(&(
            &right.language,
            right.capability,
            &right.area,
        ))
    });
}

fn structural_counts(
    parsed: &[ParsedFile],
    previous: Option<&AnalysisSnapshot>,
    reused_paths: &BTreeSet<String>,
) -> BTreeMap<Language, (u64, u64)> {
    let mut counts = BTreeMap::new();
    for file in parsed
        .iter()
        .filter(|file| file.state != CapabilityState::Failed)
    {
        let entry = counts
            .entry(file.language.clone())
            .or_insert((0_u64, 0_u64));
        entry.0 += file.entities.len() as u64;
        entry.1 += file.relations.len() as u64;
    }
    if let Some(previous) = previous {
        for fact in previous
            .structural_facts
            .iter()
            .filter(|fact| reused_paths.contains(&fact.entity.area.path))
        {
            let entry = counts
                .entry(fact.entity.language.clone())
                .or_insert((0_u64, 0_u64));
            entry.0 += 1;
            entry.1 += fact.relations.len() as u64;
        }
    }
    counts
}

fn structural_analysis_identity(
    repository_snapshot: crate::RepositorySnapshotId,
    repository_worktree: &crate::RepositoryWorktreeObservation,
    capabilities: &[CapabilityReport],
    diagnostics: &[AnalysisDiagnostic],
    bases: &[FileAnalysisBasis],
) -> Result<AnalysisSnapshotId, StructuralAnalysisError> {
    #[derive(Serialize)]
    struct Basis<'a> {
        format_version: u32,
        repository_snapshot: crate::RepositorySnapshotId,
        repository_worktree: &'a crate::RepositoryWorktreeObservation,
        capabilities: &'a [CapabilityReport],
        diagnostics: &'a [AnalysisDiagnostic],
        files: &'a [FileAnalysisBasis],
    }
    let bytes = serde_json::to_vec(&Basis {
        format_version: ANALYSIS_SNAPSHOT_FORMAT_VERSION,
        repository_snapshot,
        repository_worktree,
        capabilities,
        diagnostics,
        files: bases,
    })
    .map_err(|error| {
        StructuralAnalysisError::new(format!("analysis basis serialization failed: {error}"))
    })?;
    Ok(AnalysisSnapshotId::digest(&[
        b"volicord.structural_analysis_snapshot.v5",
        repository_snapshot.as_bytes(),
        &bytes,
    ]))
}

fn source_range(
    source: crate::model::CanonicalSourceRef,
    repository_snapshot: crate::RepositorySnapshotId,
    area: &AreaId,
    start: tree_sitter::Point,
    end: tree_sitter::Point,
    adapter: &AdapterIdentity,
    meaning: RangeMeaning,
) -> SourceRange {
    SourceRange {
        source,
        repository_snapshot,
        locator: area.path.clone(),
        start: SourcePosition {
            line: start.row as u64,
            column: start.column as u64,
        },
        end: SourcePosition {
            line: end.row as u64,
            column: end.column as u64,
        },
        coordinate_convention: CoordinateConvention::ZeroBasedUtf8Byte,
        meaning,
        adapter: adapter.clone(),
        precision_limit: None,
    }
}

fn entity_identity(
    repository_snapshot: crate::RepositorySnapshotId,
    analysis_snapshot: AnalysisSnapshotId,
    language: &Language,
    area: &AreaId,
    kind: &CodeEntityKind,
    qualified_name: &str,
    coordinates: (usize, usize),
) -> String {
    let (start, end) = coordinates;
    digest_string(&[
        b"volicord.code_entity.v1",
        repository_snapshot.as_bytes(),
        analysis_snapshot.as_bytes(),
        language_label(language).as_bytes(),
        area.path.as_bytes(),
        format!("{kind:?}").as_bytes(),
        qualified_name.as_bytes(),
        &start.to_be_bytes(),
        &end.to_be_bytes(),
    ])
}

fn relation_identity(
    repository_snapshot: crate::RepositorySnapshotId,
    analysis_snapshot: AnalysisSnapshotId,
    source: &str,
    target: &RelationTarget,
    kind: &StructuralRelationKind,
    start: usize,
    end: usize,
) -> String {
    digest_string(&[
        b"volicord.structural_relation.v1",
        repository_snapshot.as_bytes(),
        analysis_snapshot.as_bytes(),
        source.as_bytes(),
        relation_target_label(target).as_bytes(),
        relation_kind_label(kind).as_bytes(),
        &start.to_be_bytes(),
        &end.to_be_bytes(),
    ])
}

fn parser_language(language: &Language) -> Result<ParserLanguage, FileFailure> {
    let language = match language {
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::C => tree_sitter_c::LANGUAGE.into(),
        Language::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        other => {
            return Err(FileFailure::new(format!(
                "no structural grammar for {other:?}"
            )))
        }
    };
    Ok(language)
}

fn adapter_identity(language: &Language) -> AdapterIdentity {
    AdapterIdentity {
        name: format!("volicord-{}-structural-adapter", language_label(language)),
        version: STRUCTURAL_ADAPTER_VERSION.to_owned(),
    }
}

fn analyzer_identity(language: &Language) -> AnalyzerIdentity {
    let grammar = match language {
        Language::Java => "tree-sitter-java/0.23.5",
        Language::Python => "tree-sitter-python/0.25.0",
        Language::JavaScript => "tree-sitter-javascript/0.25.0",
        Language::TypeScript => "tree-sitter-typescript/0.23.2",
        Language::C => "tree-sitter-c/0.24.2",
        Language::Cpp => "tree-sitter-cpp/0.23.4",
        Language::Rust => "tree-sitter-rust/0.24.2",
        _ => "unsupported",
    };
    AnalyzerIdentity {
        name: format!("{STRUCTURAL_ANALYZER_NAME}:{grammar}"),
        version: STRUCTURAL_ANALYZER_VERSION.to_owned(),
    }
}

fn language_label(language: &Language) -> &'static str {
    match language {
        Language::Java => "java",
        Language::Python => "python",
        Language::JavaScript => "javascript",
        Language::TypeScript => "typescript",
        Language::C => "c",
        Language::Cpp => "cpp",
        Language::Rust => "rust",
        _ => "other",
    }
}

fn named(node: Node<'_>, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .map(|child| node_text(source, child).trim().to_owned())
        .filter(|name| !name.is_empty())
}

fn exact_named_token<'tree>(
    node: Node<'tree>,
    source: &[u8],
    expected: &str,
) -> Option<Node<'tree>> {
    if let Some(name) = node.child_by_field_name("name") {
        if node_text(source, name) == expected {
            return Some(name);
        }
    }
    let mut stack = vec![node];
    while let Some(candidate) = stack.pop() {
        if candidate.is_named() && node_text(source, candidate) == expected {
            return Some(candidate);
        }
        let mut cursor = candidate.walk();
        let children = candidate.named_children(&mut cursor).collect::<Vec<_>>();
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
    None
}

fn callable_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let function = if node.kind() == "function_declarator" {
        node
    } else {
        find_first_kind(node, "function_declarator")?
    };
    let declarator = function
        .child_by_field_name("declarator")
        .unwrap_or(function);
    let mut names = Vec::new();
    collect_kind_text(
        declarator,
        source,
        &[
            "identifier",
            "field_identifier",
            "operator_name",
            "destructor_name",
        ],
        &mut names,
    );
    names.last().cloned()
}

fn assignment_field(node: Node<'_>, source: &[u8], prefix: &str) -> Option<String> {
    let left = node.child_by_field_name("left")?;
    let text = node_text(source, left).trim();
    text.strip_prefix(prefix)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
}

fn parameter_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    node.child_by_field_name("pattern")
        .or_else(|| node.child_by_field_name("name"))
        .map(|child| node_text(source, child).trim().to_owned())
        .or_else(|| descendant_text(node, source, &["identifier"], &[]))
}

fn parameter_is_field(text: &str) -> bool {
    ["private ", "public ", "protected ", "readonly "]
        .iter()
        .any(|marker| text.contains(marker))
}

fn is_test_name(path: &str, name: &str, text: &str) -> bool {
    name.starts_with("test")
        || text.contains("@Test")
        || text.contains("#[test]")
        || (path.contains("/test") && name.starts_with("test"))
}

fn qualified_name(
    language: &Language,
    path: &str,
    context: &VisitContext,
    kind: &CodeEntityKind,
    name: &str,
    node: Node<'_>,
) -> String {
    if *kind == CodeEntityKind::Test {
        return match language {
            Language::Java | Language::Python => join_qualified(&context.prefix, name),
            _ => format!("{path}.{name}"),
        };
    }
    match language {
        Language::C => {
            if node.kind() == "function_definition" {
                format!("{path}.{name}")
            } else {
                join_qualified(&context.prefix, name)
            }
        }
        Language::Cpp if path.starts_with("src/") || path.starts_with("tests/") => {
            if matches!(kind, CodeEntityKind::Method | CodeEntityKind::Function) {
                format!("{path}.{name}")
            } else {
                join_qualified(&context.prefix, name)
            }
        }
        _ => join_qualified(&context.prefix, name),
    }
}

fn join_qualified(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}.{name}")
    }
}

fn module_name_from_path(path: &str) -> String {
    let without_extension = path.rsplit_once('.').map_or(path, |(value, _)| value);
    let module = without_extension.replace('/', ".");
    module
        .strip_suffix(".__init__")
        .unwrap_or(&module)
        .to_owned()
}

fn is_container(kind: &CodeEntityKind) -> bool {
    matches!(
        kind,
        CodeEntityKind::Package
            | CodeEntityKind::Module
            | CodeEntityKind::Namespace
            | CodeEntityKind::Class
            | CodeEntityKind::Interface
            | CodeEntityKind::Trait
            | CodeEntityKind::Struct
            | CodeEntityKind::Enum
    )
}

fn is_callable(kind: &CodeEntityKind) -> bool {
    matches!(
        kind,
        CodeEntityKind::Function | CodeEntityKind::Method | CodeEntityKind::Test
    )
}

fn node_text<'a>(source: &'a [u8], node: Node<'_>) -> &'a str {
    if node.end_byte() > source.len() {
        return "";
    }
    std::str::from_utf8(&source[node.start_byte()..node.end_byte()]).unwrap_or_default()
}

fn find_first_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut stack = vec![node];
    while let Some(candidate) = stack.pop() {
        if candidate.kind() == kind {
            return Some(candidate);
        }
        let mut cursor = candidate.walk();
        for child in candidate.named_children(&mut cursor) {
            stack.push(child);
        }
    }
    None
}

fn contains_kind(node: Node<'_>, kind: &str) -> bool {
    find_first_kind(node, kind).is_some()
}

fn descendant_text(
    node: Node<'_>,
    source: &[u8],
    container_kinds: &[&str],
    child_kinds: &[&str],
) -> Option<String> {
    let container = if container_kinds.contains(&node.kind()) {
        node
    } else {
        let mut found = None;
        for kind in container_kinds {
            if let Some(candidate) = find_first_kind(node, kind) {
                found = Some(candidate);
                break;
            }
        }
        found?
    };
    if child_kinds.is_empty() {
        let text = node_text(source, container).trim().to_owned();
        return (!text.is_empty()).then_some(text);
    }
    let mut values = Vec::new();
    collect_kind_text(container, source, child_kinds, &mut values);
    values.last().cloned()
}

fn collect_kind_text(node: Node<'_>, source: &[u8], kinds: &[&str], output: &mut Vec<String>) {
    if kinds.contains(&node.kind()) {
        let text = node_text(source, node).trim();
        if !text.is_empty() {
            output.push(text.to_owned());
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_kind_text(child, source, kinds, output);
    }
}

fn identifier_tokens(text: &str) -> Vec<String> {
    text.split(|character: char| {
        !(character.is_alphanumeric() || character == '_' || character == '.' || character == ':')
    })
    .filter(|token| {
        !token.is_empty()
            && !matches!(
                *token,
                "extends" | "implements" | "public" | "private" | "protected" | "virtual"
            )
    })
    .map(ToOwned::to_owned)
    .collect()
}

fn keyword_targets(text: &str, keyword: &str) -> Vec<String> {
    let Some((_, tail)) = text.split_once(keyword) else {
        return Vec::new();
    };
    let until = if keyword == "extends" {
        tail.split("implements").next().unwrap_or(tail)
    } else {
        tail.split('{').next().unwrap_or(tail)
    };
    identifier_tokens(until)
}

fn string_literal(node: Node<'_>, source: &[u8]) -> Option<String> {
    let string =
        find_first_kind(node, "string").or_else(|| find_first_kind(node, "string_literal"))?;
    Some(
        node_text(source, string)
            .trim()
            .trim_matches(|character| character == '"' || character == '\'')
            .to_owned(),
    )
}

fn export_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let mut values = Vec::new();
    collect_kind_text(
        node,
        source,
        &["type_identifier", "identifier"],
        &mut values,
    );
    values.into_iter().next()
}

fn java_call_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let name = node.child_by_field_name("name")?;
    let name = node_text(source, name).trim();
    let object = node
        .child_by_field_name("object")
        .map(|object| node_text(source, object).trim())
        .filter(|object| !object.is_empty());
    Some(object.map_or_else(|| name.to_owned(), |object| format!("{object}.{name}")))
}

fn has_construct_limit(language: &Language, root: Node<'_>) -> bool {
    let kinds: &[&str] = match language {
        Language::C | Language::Cpp => &[
            "preproc_if",
            "preproc_ifdef",
            "preproc_def",
            "preproc_function_def",
        ],
        Language::Rust => &["macro_invocation", "attribute_item"],
        _ => return false,
    };
    kinds
        .iter()
        .any(|kind| find_first_kind(root, kind).is_some())
}

fn parse_error_diagnostic(language: &Language, area: &AreaId) -> AnalysisDiagnostic {
    structural_diagnostic(
        language,
        area,
        "structural.parse_error",
        DiagnosticSeverity::Warning,
        "the parser recovered from syntax errors; retained nodes are partial",
        Some("parser-owned nodes outside the error region remain usable"),
    )
}

fn construct_limit_diagnostic(language: &Language, area: &AreaId) -> AnalysisDiagnostic {
    structural_diagnostic(
        language,
        area,
        "structural.construct_limit",
        DiagnosticSeverity::Warning,
        match language {
            Language::C | Language::Cpp => "preprocessor syntax is present; macro expansion and active conditional-build selection are not resolved",
            Language::Rust => "macro or attribute syntax is present; macro expansion and cfg-selected build meaning are not resolved",
            _ => "language-specific syntax has unsupported structural meaning",
        },
        Some("direct source syntax remains available with this limitation"),
    )
}

fn language_limit_diagnostic(language: &Language) -> AnalysisDiagnostic {
    structural_diagnostic(
        language,
        &repository_area(),
        "structural.language_limit",
        DiagnosticSeverity::Information,
        match language {
            Language::Java => "annotation processing, generated sources, reflection, and resolved calls are outside structural syntax",
            Language::Python => "dynamic dispatch, monkey-patching, imports at runtime, and resolved calls are outside structural syntax",
            Language::JavaScript => "prototype mutation, dynamic properties, runtime module resolution, and resolved calls are outside structural syntax",
            Language::TypeScript => "type-level evaluation, declaration merging, runtime module resolution, and resolved calls are outside structural syntax",
            Language::C => "macro expansion, active conditional compilation, compile flags, and resolved calls require build/semantic context",
            Language::Cpp => "macro expansion, template instantiation, active conditional compilation, compile flags, and resolved calls require build/semantic context",
            Language::Rust => "macro expansion, cfg-selected code, trait resolution, and resolved calls require build/semantic context",
            _ => "no Production structural adapter is available",
        },
        Some("reported entities and syntax relations remain snapshot-bound structural facts"),
    )
}

fn failure_diagnostic(language: &Language, area: &AreaId, message: &str) -> AnalysisDiagnostic {
    structural_diagnostic(
        language,
        area,
        "structural.adapter_failed",
        DiagnosticSeverity::Error,
        message,
        Some("inventory and successful language/area results remain usable"),
    )
}

fn structural_diagnostic(
    language: &Language,
    area: &AreaId,
    code: &str,
    severity: DiagnosticSeverity,
    message: &str,
    usable_remainder: Option<&str>,
) -> AnalysisDiagnostic {
    AnalysisDiagnostic {
        identity: format!(
            "structural:{}",
            digest_string(&[
                code.as_bytes(),
                language_label(language).as_bytes(),
                area.path.as_bytes()
            ])
        ),
        severity,
        code: code.to_owned(),
        message: message.to_owned(),
        affected_area: area.clone(),
        capability: Capability::Structural,
        adapter: Some(adapter_identity(language)),
        analyzer: Some(analyzer_identity(language)),
        usable_remainder: usable_remainder.map(ToOwned::to_owned),
    }
}

fn unresolved_target(display: &str, language: &Language) -> RelationTarget {
    RelationTarget::Unresolved(UnresolvedTarget {
        display: display.to_owned(),
        language: Some(language.clone()),
        locator_hint: None,
        reason: "syntax identifies a target spelling but structural analysis does not resolve symbol identity"
            .to_owned(),
    })
}

fn relation_kind_label(kind: &StructuralRelationKind) -> &'static str {
    match kind {
        StructuralRelationKind::Contains => "contains",
        StructuralRelationKind::Declares => "declares",
        StructuralRelationKind::Imports => "imports",
        StructuralRelationKind::Includes => "includes",
        StructuralRelationKind::Exports => "exports",
        StructuralRelationKind::Inherits => "inherits",
        StructuralRelationKind::Implements => "implements",
        StructuralRelationKind::CallsSyntactically => "calls_syntactically",
        StructuralRelationKind::Tests => "tests",
        StructuralRelationKind::Configures => "configures",
        StructuralRelationKind::LanguageSpecific(_) => "language_specific",
    }
}

fn relation_target_label(target: &RelationTarget) -> &str {
    match target {
        RelationTarget::ResolvedEntity(identity) => identity,
        RelationTarget::Unresolved(target) => &target.display,
    }
}

fn draft_target_label(target: &DraftTarget) -> String {
    match target {
        DraftTarget::Entity(key) => format!("entity:{key}"),
        DraftTarget::Unresolved(display) => display.clone(),
    }
}

fn current_freshness(repository_snapshot: crate::RepositorySnapshotId) -> FreshnessBasis {
    FreshnessBasis {
        state: FreshnessState::Current,
        repository_snapshot,
        compared_repository_snapshot: None,
        reason: None,
    }
}

fn rebind_range(range: Option<&mut SourceRange>, repository_snapshot: crate::RepositorySnapshotId) {
    if let Some(range) = range {
        range.repository_snapshot = repository_snapshot;
    }
}

fn position_basis(position: SourcePosition) -> usize {
    ((position.line as usize) << 32) | position.column as usize
}

fn point_basis(position: tree_sitter::Point) -> usize {
    (position.row << 32) | position.column
}

fn repository_area() -> AreaId {
    AreaId {
        kind: AreaKind::Repository,
        path: ".".to_owned(),
    }
}

fn path_from_locator(locator: &str) -> PathBuf {
    locator.split('/').collect()
}

fn digest_string(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hash_part(&mut hasher, part);
    }
    hex_digest(&hasher.finalize())
}

fn hash_part(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(value, "{byte:02x}");
    }
    value
}

#[cfg(test)]
mod tests {
    use super::{analyze_repository_inner, StructuralAnalysisRequest};
    use crate::{Capability, CapabilityState, InvalidationCategory, InventoryRequest, Language};
    use std::collections::BTreeSet;
    use std::path::Path;
    use volicord_context::{ProjectId, SourceId};

    #[test]
    fn injected_adapter_failure_is_bounded_to_one_language(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../validation/repository-intelligence/realistic-qualification/fixtures/polyglot",
        );
        let grounding = crate::canonical::test_repository_grounding(
            ProjectId::from_bytes([0x31; 16]),
            SourceId::from_bytes([0x32; 16]),
        )?;
        let request = InventoryRequest::new(
            &root,
            &grounding,
            SourceId::from_bytes([0x32; 16]),
            1_725_000_000_000_000,
        )?;
        let (_, analysis) = analyze_repository_inner(
            StructuralAnalysisRequest::new(request),
            &BTreeSet::from([Language::TypeScript]),
        )?;
        let typescript = analysis.capabilities.iter().find(|report| {
            report.language == Some(Language::TypeScript)
                && report.capability == Capability::Structural
        });
        assert!(typescript.is_some_and(|report| report.state == CapabilityState::Failed));
        assert!(typescript.is_some_and(|report| {
            !report.coverage.failed.is_empty()
                && report.usable_remainder.is_some()
                && report.user_visible_consequence.is_some()
                && !report.diagnostics.is_empty()
        }));
        assert!(analysis.capabilities.iter().any(|report| {
            report.language.is_none()
                && report.capability == Capability::Inventory
                && report.state == CapabilityState::Available
                && report.coverage.covered_file_count > 0
        }));
        assert!(analysis.capabilities.iter().any(|report| {
            report.language == Some(Language::Java)
                && report.capability == Capability::Structural
                && matches!(
                    report.state,
                    CapabilityState::Available | CapabilityState::Partial
                )
                && report.coverage.covered_entity_count > 0
        }));
        assert!(analysis
            .structural_facts
            .iter()
            .any(|fact| fact.entity.language == Language::Java));
        assert!(analysis
            .structural_facts
            .iter()
            .any(|fact| fact.entity.language == Language::Python));
        assert!(analysis.inventory.languages.contains(&Language::TypeScript));

        let retry_request = InventoryRequest::new(
            &root,
            &grounding,
            SourceId::from_bytes([0x32; 16]),
            1_725_000_000_000_000,
        )?;
        let (_, retried) = analyze_repository_inner(
            StructuralAnalysisRequest::new(retry_request).with_previous(&analysis),
            &BTreeSet::new(),
        )?;
        assert!(retried.invalidations.iter().any(|record| {
            record.language == Language::TypeScript
                && record.category == InvalidationCategory::PriorFailure
        }));
        assert!(retried
            .structural_facts
            .iter()
            .any(|fact| fact.entity.language == Language::TypeScript));
        Ok(())
    }
}
