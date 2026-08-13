use crate::model::{
    AdapterIdentity, AnalysisDiagnostic, AnalysisProvenance, AnalysisSnapshot, AnalyzerIdentity,
    AreaId, AreaKind, CanonicalReference, Capability, CapabilityReport, CapabilityState,
    CodeEntity, CodeEntityKind, Coverage, DiagnosticSeverity, FileAnalysisBasis, Language,
    ProvenanceClass, RangeMeaning, RelationTarget, SemanticAnalysisResult, SemanticProvenance,
    SemanticRefresh, SemanticRelation, SemanticRelationKind, SourceRange, StructuralRelationKind,
    Uncertainty, UncertaintyLevel, UnresolvedTarget, ANALYSIS_SNAPSHOT_FORMAT_VERSION,
};
use crate::{
    analyze_repository, AnalysisSnapshotId, RepositorySnapshot, StructuralAnalysisError,
    StructuralAnalysisRequest,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error as StdError;
use std::fmt;
use std::fs;
use std::path::Path;

const SEMANTIC_ANALYZER_NAME: &str = "volicord-source-semantic-index";
const SEMANTIC_ANALYZER_VERSION: &str = "1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalLinkSelector {
    pub language: Language,
    pub locator: String,
    pub qualified_name: String,
}

impl CanonicalLinkSelector {
    pub fn new(
        language: Language,
        locator: impl Into<String>,
        qualified_name: impl Into<String>,
    ) -> Self {
        Self {
            language,
            locator: locator.into(),
            qualified_name: qualified_name.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SemanticAnalysisRequest<'a> {
    pub structural: StructuralAnalysisRequest<'a>,
    selected_languages: BTreeSet<Language>,
    unavailable_languages: BTreeMap<Language, String>,
    canonical_links: Vec<(CanonicalLinkSelector, CanonicalReference)>,
}

impl<'a> SemanticAnalysisRequest<'a> {
    pub fn new(structural: StructuralAnalysisRequest<'a>) -> Self {
        Self {
            structural,
            selected_languages: [Language::Java, Language::TypeScript, Language::Rust]
                .into_iter()
                .collect(),
            unavailable_languages: BTreeMap::new(),
            canonical_links: Vec::new(),
        }
    }

    pub fn with_languages(mut self, languages: impl IntoIterator<Item = Language>) -> Self {
        self.selected_languages = languages.into_iter().collect();
        self
    }

    pub fn with_unavailable_language(
        mut self,
        language: Language,
        reason: impl Into<String>,
    ) -> Self {
        self.unavailable_languages.insert(language, reason.into());
        self
    }

    pub fn with_canonical_link(
        mut self,
        selector: CanonicalLinkSelector,
        target: CanonicalReference,
    ) -> Self {
        self.canonical_links.push((selector, target));
        self
    }
}

#[derive(Debug)]
pub struct SemanticAnalysisError {
    message: String,
    source: Option<StructuralAnalysisError>,
}

impl SemanticAnalysisError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    fn structural(source: StructuralAnalysisError) -> Self {
        Self {
            message: "structural analysis failed before semantic analysis".to_owned(),
            source: Some(source),
        }
    }
}

impl fmt::Display for SemanticAnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl StdError for SemanticAnalysisError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_ref()
            .map(|source| source as &(dyn StdError + 'static))
    }
}

pub fn analyze_repository_semantics(
    request: SemanticAnalysisRequest<'_>,
) -> Result<(RepositorySnapshot, AnalysisSnapshot), SemanticAnalysisError> {
    analyze_repository_semantics_inner(request, &BTreeSet::new())
}

fn analyze_repository_semantics_inner(
    request: SemanticAnalysisRequest<'_>,
    injected_failures: &BTreeSet<Language>,
) -> Result<(RepositorySnapshot, AnalysisSnapshot), SemanticAnalysisError> {
    let root = request.structural.inventory.root.to_path_buf();
    let selected_languages = request.selected_languages.clone();
    let unavailable_languages = request.unavailable_languages.clone();
    let canonical_links = request.canonical_links.clone();
    let canonical_grounding = request.structural.inventory.canonical_grounding.clone();
    let (repository, mut analysis) =
        analyze_repository(request.structural).map_err(SemanticAnalysisError::structural)?;
    let facts = analysis.structural_facts.clone();
    let by_identity = facts
        .iter()
        .map(|fact| (fact.entity.identity.clone(), fact.entity.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut results = Vec::new();
    let mut diagnostics = Vec::new();
    let mut bases = Vec::new();
    let mut refresh = SemanticRefresh::default();

    for language in analysis.inventory.languages.clone() {
        if !is_selected_ecosystem(&language) || !selected_languages.contains(&language) {
            continue;
        }
        let language_bases = analysis
            .structural_bases
            .iter()
            .filter(|basis| basis.language == language)
            .cloned()
            .collect::<Vec<_>>();
        if let Some(reason) = unavailable_languages.get(&language) {
            refresh.unavailable_file_count += language_bases.len() as u64;
            let report = semantic_capability_report(
                &analysis,
                &language,
                CapabilityState::Unavailable,
                Some(reason.clone()),
                Vec::new(),
                0,
                0,
            );
            replace_semantic_capability(&mut analysis, report);
            continue;
        }
        if injected_failures.contains(&language) {
            let diagnostic = diagnostic(
                &language,
                repository_area(),
                "semantic.adapter_failed",
                "the semantic adapter failed before publishing any semantic relation",
                DiagnosticSeverity::Error,
                None,
            );
            let diagnostic_id = diagnostic.identity.clone();
            diagnostics.push(diagnostic);
            refresh.failed_file_count += language_bases.len() as u64;
            let report = semantic_capability_report(
                &analysis,
                &language,
                CapabilityState::Failed,
                Some("the selected semantic adapter failed".to_owned()),
                vec![diagnostic_id],
                0,
                0,
            );
            replace_semantic_capability(&mut analysis, report);
            continue;
        }

        let language_facts = facts
            .iter()
            .filter(|fact| fact.entity.language == language)
            .collect::<Vec<_>>();
        let mut language_results = Vec::new();
        let mut language_diagnostics = Vec::new();
        let source_text =
            read_sources(&root, &language_bases, &language, &mut language_diagnostics);
        add_definition_relations(&language_facts, &mut language_results, &analysis);
        add_structural_semantics(
            &language_facts,
            &by_identity,
            &source_text,
            &mut language_results,
            &mut language_diagnostics,
            &analysis,
        );
        add_type_relations(
            &language_facts,
            &by_identity,
            &source_text,
            &mut language_results,
            &analysis,
        );
        language_results
            .sort_by(|left, right| left.relation.identity.cmp(&right.relation.identity));
        language_results.dedup_by(|left, right| left.relation.identity == right.relation.identity);
        language_diagnostics.sort_by(|left, right| left.identity.cmp(&right.identity));
        language_diagnostics.dedup_by(|left, right| left.identity == right.identity);

        let structural_state = structural_state(&analysis, &language);
        let state = if language_diagnostics
            .iter()
            .any(|item| item.severity == DiagnosticSeverity::Error)
            && language_results.is_empty()
        {
            CapabilityState::Failed
        } else if structural_state != CapabilityState::Available || !language_diagnostics.is_empty()
        {
            CapabilityState::Partial
        } else {
            CapabilityState::Available
        };
        let diagnostic_ids = language_diagnostics
            .iter()
            .map(|item| item.identity.clone())
            .collect::<Vec<_>>();
        let relation_count = language_results.len() as u64;
        let entity_count = language_results
            .iter()
            .map(|item| item.relation.source_entity.as_str())
            .collect::<BTreeSet<_>>()
            .len() as u64;
        let report = semantic_capability_report(
            &analysis,
            &language,
            state,
            (state != CapabilityState::Available).then(|| {
                "semantic results have unresolved, incomplete-build, or structural coverage limits"
                    .to_owned()
            }),
            diagnostic_ids.clone(),
            entity_count,
            relation_count,
        );
        replace_semantic_capability(&mut analysis, report);
        update_ecosystem_capability(&mut analysis, &language, state, &diagnostic_ids);
        for mut basis in language_bases {
            basis.adapter = semantic_adapter(&language);
            basis.analyzer = semantic_analyzer();
            basis.state = state;
            basis.diagnostic_ids = diagnostic_ids.clone();
            bases.push(basis);
            refresh.analyzed_file_count += 1;
        }
        results.extend(language_results);
        diagnostics.extend(language_diagnostics);
    }

    apply_canonical_links(&mut analysis, canonical_links, &canonical_grounding)?;
    analysis.semantic_results = results;
    analysis.semantic_bases = bases;
    analysis.semantic_refresh = refresh;
    analysis.diagnostics.extend(diagnostics);
    analysis
        .diagnostics
        .sort_by(|left, right| left.identity.cmp(&right.identity));
    analysis
        .diagnostics
        .dedup_by(|left, right| left.identity == right.identity);
    analysis.capabilities.sort_by(|left, right| {
        (&left.language, left.capability, &left.area).cmp(&(
            &right.language,
            right.capability,
            &right.area,
        ))
    });
    analysis.format_version = ANALYSIS_SNAPSHOT_FORMAT_VERSION;
    let identity = semantic_snapshot_identity(&analysis)?;
    rebind_analysis_snapshot(&mut analysis, identity);
    canonical_grounding
        .validate_repository_snapshot(&repository)
        .and_then(|()| canonical_grounding.validate_analysis_snapshot(&analysis))
        .map_err(|error| {
            SemanticAnalysisError::new(format!(
                "produced analysis has invalid canonical grounding: {error}"
            ))
        })?;
    Ok((repository, analysis))
}

fn is_selected_ecosystem(language: &Language) -> bool {
    matches!(
        language,
        Language::Java | Language::TypeScript | Language::Rust
    )
}

fn read_sources(
    root: &Path,
    bases: &[FileAnalysisBasis],
    language: &Language,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    for basis in bases {
        match fs::read_to_string(root.join(&basis.area.path)) {
            Ok(text) => {
                result.insert(basis.area.path.clone(), text);
            }
            Err(error) => diagnostics.push(diagnostic(
                language,
                basis.area.clone(),
                "semantic.source_unavailable",
                &format!("semantic source read failed: {error}"),
                DiagnosticSeverity::Error,
                None,
            )),
        }
    }
    result
}

fn add_definition_relations(
    facts: &[&crate::StructuralFact],
    results: &mut Vec<SemanticAnalysisResult>,
    analysis: &AnalysisSnapshot,
) {
    for fact in facts.iter().filter(|fact| {
        !matches!(
            fact.entity.kind,
            CodeEntityKind::File | CodeEntityKind::Package | CodeEntityKind::Module
        )
    }) {
        let source = facts
            .iter()
            .find(|candidate| {
                candidate.entity.area == fact.entity.area
                    && candidate.entity.kind == CodeEntityKind::File
            })
            .map_or(&fact.entity, |candidate| &candidate.entity);
        results.push(make_result(
            analysis,
            source,
            RelationTarget::ResolvedEntity(fact.entity.identity.clone()),
            SemanticRelationKind::Defines,
            semantic_range(fact.entity.source_range.as_ref(), &fact.entity.language),
            Uncertainty::none(),
        ));
    }
}

fn add_structural_semantics(
    facts: &[&crate::StructuralFact],
    entities: &BTreeMap<String, CodeEntity>,
    sources: &BTreeMap<String, String>,
    results: &mut Vec<SemanticAnalysisResult>,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
    analysis: &AnalysisSnapshot,
) {
    for fact in facts {
        for relation in &fact.relations {
            match relation.kind {
                StructuralRelationKind::Implements | StructuralRelationKind::Inherits => {
                    let target =
                        resolve_target(&fact.entity, &relation.target, facts, None, sources);
                    let semantic_target = target.clone();
                    results.push(make_result(
                        analysis,
                        &fact.entity,
                        semantic_target,
                        SemanticRelationKind::Implements,
                        semantic_range(relation.supporting_range.as_ref(), &fact.entity.language),
                        uncertainty_for_target(&target),
                    ));
                    if let RelationTarget::ResolvedEntity(target_identity) = target {
                        add_overrides(
                            &fact.entity,
                            &target_identity,
                            facts,
                            entities,
                            sources,
                            results,
                            analysis,
                        );
                    }
                }
                StructuralRelationKind::CallsSyntactically => {
                    let arity =
                        call_arity(&fact.entity, relation.supporting_range.as_ref(), sources);
                    let target =
                        resolve_target(&fact.entity, &relation.target, facts, arity, sources);
                    if let RelationTarget::Unresolved(unresolved) = &target {
                        diagnostics.push(diagnostic(
                            &fact.entity.language,
                            fact.entity.area.clone(),
                            "semantic.unresolved_symbol",
                            &format!(
                                "unresolved symbol `{}`: {}",
                                unresolved.display, unresolved.reason
                            ),
                            DiagnosticSeverity::Warning,
                            Some("other resolved semantic relations remain usable"),
                        ));
                    }
                    let range =
                        semantic_range(relation.supporting_range.as_ref(), &fact.entity.language);
                    results.push(make_result(
                        analysis,
                        &fact.entity,
                        target.clone(),
                        SemanticRelationKind::References,
                        range.clone(),
                        uncertainty_for_target(&target),
                    ));
                    if matches!(target, RelationTarget::ResolvedEntity(_)) {
                        results.push(make_result(
                            analysis,
                            &fact.entity,
                            target,
                            SemanticRelationKind::ResolvesTo,
                            range,
                            Uncertainty::none(),
                        ));
                    }
                }
                StructuralRelationKind::Imports => {
                    let target = resolve_import(&fact.entity, &relation.target, facts);
                    if let RelationTarget::Unresolved(unresolved) = &target {
                        if unresolved.display.starts_with('.')
                            || unresolved.display.starts_with("missing")
                        {
                            diagnostics.push(diagnostic(
                                &fact.entity.language,
                                fact.entity.area.clone(),
                                "semantic.unresolved_dependency",
                                &format!("unresolved dependency `{}`", unresolved.display),
                                DiagnosticSeverity::Warning,
                                Some("locally resolved symbols remain usable"),
                            ));
                        }
                    }
                    results.push(make_result(
                        analysis,
                        &fact.entity,
                        target.clone(),
                        SemanticRelationKind::ResolvesTo,
                        semantic_range(relation.supporting_range.as_ref(), &fact.entity.language),
                        uncertainty_for_target(&target),
                    ));
                }
                _ => {}
            }
        }
    }
}

fn add_overrides(
    implementing: &CodeEntity,
    target_identity: &str,
    facts: &[&crate::StructuralFact],
    entities: &BTreeMap<String, CodeEntity>,
    sources: &BTreeMap<String, String>,
    results: &mut Vec<SemanticAnalysisResult>,
    analysis: &AnalysisSnapshot,
) {
    let Some(target_type) = entities.get(target_identity) else {
        return;
    };
    let implementing_name = implementing.qualified_name.as_deref().unwrap_or_default();
    let target_name = target_type.qualified_name.as_deref().unwrap_or_default();
    let implementing_methods = facts.iter().filter(|fact| {
        fact.entity.kind == CodeEntityKind::Method
            && qualified_parent(&fact.entity) == implementing_name
    });
    for method in implementing_methods {
        let Some(target_method) = facts.iter().find(|candidate| {
            candidate.entity.kind == CodeEntityKind::Method
                && candidate.entity.display_name == method.entity.display_name
                && qualified_parent(&candidate.entity) == target_name
                && declared_arity(&candidate.entity, sources)
                    == declared_arity(&method.entity, sources)
        }) else {
            continue;
        };
        results.push(make_result(
            analysis,
            &method.entity,
            RelationTarget::ResolvedEntity(target_method.entity.identity.clone()),
            SemanticRelationKind::Overrides,
            semantic_range(method.entity.source_range.as_ref(), &method.entity.language),
            Uncertainty::none(),
        ));
    }
}

fn add_type_relations(
    facts: &[&crate::StructuralFact],
    entities: &BTreeMap<String, CodeEntity>,
    sources: &BTreeMap<String, String>,
    results: &mut Vec<SemanticAnalysisResult>,
    analysis: &AnalysisSnapshot,
) {
    for fact in facts.iter().filter(|fact| {
        matches!(
            fact.entity.kind,
            CodeEntityKind::Function
                | CodeEntityKind::Method
                | CodeEntityKind::Field
                | CodeEntityKind::Type
        )
    }) {
        let Some(range) = fact.entity.source_range.as_ref() else {
            continue;
        };
        let Some(source) = sources.get(&fact.entity.area.path) else {
            continue;
        };
        let Some(line) = source.lines().nth(range.start.line as usize) else {
            continue;
        };
        let Some(type_name) = declared_type(&fact.entity, line) else {
            continue;
        };
        let target = entities
            .values()
            .find(|entity| {
                entity.language == fact.entity.language
                    && entity.display_name.as_deref() == Some(type_name.as_str())
            })
            .map(|entity| RelationTarget::ResolvedEntity(entity.identity.clone()))
            .unwrap_or_else(|| {
                RelationTarget::Unresolved(UnresolvedTarget {
                    display: type_name,
                    language: Some(fact.entity.language.clone()),
                    locator_hint: None,
                    reason:
                        "declared type is builtin, external, or absent from this source snapshot"
                            .to_owned(),
                })
            });
        results.push(make_result(
            analysis,
            &fact.entity,
            target.clone(),
            SemanticRelationKind::TypeOf,
            semantic_range(Some(range), &fact.entity.language),
            if is_builtin_target(&target) {
                Uncertainty::none()
            } else {
                uncertainty_for_target(&target)
            },
        ));
    }
}

fn declared_type(entity: &CodeEntity, line: &str) -> Option<String> {
    let name = entity.display_name.as_deref()?;
    match entity.language {
        Language::Java => line
            .split(name)
            .next()
            .and_then(|prefix| prefix.split_whitespace().next_back())
            .filter(|value| !matches!(*value, "class" | "interface"))
            .map(str::to_owned),
        Language::TypeScript => line
            .rsplit_once(':')
            .map(|(_, suffix)| suffix)
            .and_then(|suffix| suffix.split(['{', ';']).next())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        Language::Rust => line
            .split_once("->")
            .map(|(_, suffix)| suffix)
            .and_then(|suffix| suffix.split(['{', ';']).next())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        _ => None,
    }
}

fn resolve_target(
    source: &CodeEntity,
    target: &RelationTarget,
    facts: &[&crate::StructuralFact],
    requested_arity: Option<usize>,
    sources: &BTreeMap<String, String>,
) -> RelationTarget {
    let RelationTarget::Unresolved(unresolved) = target else {
        return target.clone();
    };
    let display = unresolved.display.trim();
    let simple = display
        .split(['.', ':'])
        .rfind(|part| !part.is_empty())
        .unwrap_or(display);
    let mut candidates = facts
        .iter()
        .filter(|fact| fact.entity.display_name.as_deref() == Some(simple))
        .collect::<Vec<_>>();
    if let Some(arity) = requested_arity {
        let arity_matches = candidates
            .iter()
            .filter(|candidate| declared_arity(&candidate.entity, sources) == Some(arity))
            .copied()
            .collect::<Vec<_>>();
        if !arity_matches.is_empty() {
            candidates = arity_matches;
        }
    }
    if candidates.is_empty() {
        return RelationTarget::Unresolved(UnresolvedTarget {
            display: display.to_owned(),
            language: Some(source.language.clone()),
            locator_hint: Some(source.area.path.clone()),
            reason: "no matching declaration exists in the analyzed source snapshot".to_owned(),
        });
    }
    let parent = qualified_parent(source);
    let scoped = candidates
        .iter()
        .filter(|candidate| qualified_parent(&candidate.entity) == parent)
        .copied()
        .collect::<Vec<_>>();
    let selected = if scoped.len() == 1 {
        Some(scoped[0])
    } else if candidates.len() == 1 {
        Some(candidates[0])
    } else {
        candidates
            .iter()
            .find(|candidate| {
                candidate
                    .entity
                    .qualified_name
                    .as_deref()
                    .is_some_and(|qualified| {
                        qualified
                            .replace('.', "::")
                            .ends_with(&display.replace('.', "::"))
                    })
            })
            .copied()
    };
    selected.map_or_else(
        || {
            RelationTarget::Unresolved(UnresolvedTarget {
                display: display.to_owned(),
                language: Some(source.language.clone()),
                locator_hint: Some(source.area.path.clone()),
                reason: format!(
                    "{} same-name declarations remain ambiguous without compiler context",
                    candidates.len()
                ),
            })
        },
        |fact| RelationTarget::ResolvedEntity(fact.entity.identity.clone()),
    )
}

fn declared_arity(entity: &CodeEntity, sources: &BTreeMap<String, String>) -> Option<usize> {
    let range = entity.source_range.as_ref()?;
    let source = sources.get(&entity.area.path)?;
    let line = source.lines().nth(range.start.line as usize)?;
    parameter_arity(line.split_once('(')?.1.split_once(')')?.0)
}

fn call_arity(
    source: &CodeEntity,
    range: Option<&SourceRange>,
    sources: &BTreeMap<String, String>,
) -> Option<usize> {
    let range = range?;
    let text = sources.get(&source.area.path)?;
    let line = text.lines().nth(range.start.line as usize)?;
    let start = usize::try_from(range.start.column).ok()?.min(line.len());
    let call = line.get(start..).unwrap_or(line);
    parameter_arity(call.split_once('(')?.1.split_once(')')?.0)
}

fn parameter_arity(parameters: &str) -> Option<usize> {
    let parameters = parameters.trim();
    if parameters.is_empty() {
        return Some(0);
    }
    let mut depth = 0_usize;
    let mut commas = 0_usize;
    for character in parameters.chars() {
        match character {
            '<' | '(' | '[' | '{' => depth += 1,
            '>' | ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => commas += 1,
            _ => {}
        }
    }
    Some(commas + 1)
}

fn resolve_import(
    source: &CodeEntity,
    target: &RelationTarget,
    facts: &[&crate::StructuralFact],
) -> RelationTarget {
    let RelationTarget::Unresolved(unresolved) = target else {
        return target.clone();
    };
    let display = unresolved.display.trim_matches(['"', '\'']);
    if source.language == Language::TypeScript && display.starts_with('.') {
        let parent = Path::new(&source.area.path)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        let joined = parent.join(display).to_string_lossy().replace('\\', "/");
        let candidate = joined
            .trim_end_matches(".js")
            .trim_end_matches(".ts")
            .to_owned()
            + ".ts";
        if let Some(file) = facts.iter().find(|fact| {
            fact.entity.kind == CodeEntityKind::File && fact.entity.area.path == candidate
        }) {
            return RelationTarget::ResolvedEntity(file.entity.identity.clone());
        }
    }
    RelationTarget::Unresolved(UnresolvedTarget {
        display: display.to_owned(),
        language: Some(source.language.clone()),
        locator_hint: Some(source.area.path.clone()),
        reason: "dependency is external, wildcard-based, or absent from the local snapshot"
            .to_owned(),
    })
}

fn make_result(
    analysis: &AnalysisSnapshot,
    source: &CodeEntity,
    target: RelationTarget,
    kind: SemanticRelationKind,
    supporting_range: Option<SourceRange>,
    uncertainty: Uncertainty,
) -> SemanticAnalysisResult {
    let adapter = semantic_adapter(&source.language);
    let analyzer = semantic_analyzer();
    let target_label = target_label(&target);
    let kind_label = semantic_kind_label(&kind);
    let identity = digest_string(&[
        b"volicord.semantic_relation.v1",
        analysis.repository_snapshot.as_bytes(),
        analysis.identity.as_bytes(),
        source.identity.as_bytes(),
        kind_label.as_bytes(),
        target_label.as_bytes(),
    ]);
    SemanticAnalysisResult {
        relation: SemanticRelation {
            identity,
            repository_snapshot: analysis.repository_snapshot,
            analysis_snapshot: analysis.identity,
            source_entity: source.identity.clone(),
            target,
            kind,
            supporting_range,
            diagnostics: Vec::new(),
            uncertainty,
            freshness: source.freshness.clone(),
            extensions: Vec::new(),
        },
        provenance: SemanticProvenance {
            adapter: adapter.clone(),
            analyzer: analyzer.clone(),
            build_context: Some("local manifest plus source snapshot".to_owned()),
            resolution_basis: "qualified local declarations, lexical scope, explicit implementation, declared types, and local module paths"
                .to_owned(),
            analysis: AnalysisProvenance {
                class: ProvenanceClass::SemanticResult,
                repository_snapshot: analysis.repository_snapshot,
                analysis_snapshot: analysis.identity,
                adapter: Some(adapter),
                analyzer: Some(analyzer),
                source_basis: vec![source.source.clone()],
                observed_or_generated_at_unix_micros: analysis.generated_at_unix_micros,
            },
        },
    }
}

fn semantic_range(range: Option<&SourceRange>, language: &Language) -> Option<SourceRange> {
    range.cloned().map(|mut value| {
        value.adapter = semantic_adapter(language);
        value.meaning = RangeMeaning::Symbol;
        value.precision_limit = Some(
            "source-semantic range; macro/generated expansion and compiler-only coordinates are unavailable"
                .to_owned(),
        );
        value
    })
}

fn semantic_adapter(language: &Language) -> AdapterIdentity {
    AdapterIdentity {
        name: format!("{}-source-semantic", language_label(language)),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

fn semantic_analyzer() -> AnalyzerIdentity {
    AnalyzerIdentity {
        name: SEMANTIC_ANALYZER_NAME.to_owned(),
        version: SEMANTIC_ANALYZER_VERSION.to_owned(),
    }
}

fn semantic_capability_report(
    analysis: &AnalysisSnapshot,
    language: &Language,
    state: CapabilityState,
    reason: Option<String>,
    diagnostics: Vec<String>,
    entities: u64,
    relations: u64,
) -> CapabilityReport {
    let included = analysis
        .inventory
        .entries
        .iter()
        .filter(|entry| entry.language.as_ref() == Some(language))
        .map(|entry| entry.area.clone())
        .collect::<Vec<_>>();
    CapabilityReport {
        repository_snapshot: analysis.repository_snapshot,
        language: Some(language.clone()),
        area: repository_area(),
        capability: Capability::Semantic,
        state,
        reason,
        usable_remainder: (relations > 0)
            .then(|| "source-ranged, unambiguously resolved relations remain usable".to_owned()),
        user_visible_consequence: (state != CapabilityState::Available).then(|| {
            "semantic impact is candidate evidence only; inspect diagnostics and omitted scope"
                .to_owned()
        }),
        coverage: Coverage {
            included: included.clone(),
            unavailable: if state == CapabilityState::Unavailable {
                included.clone()
            } else {
                Vec::new()
            },
            failed: if state == CapabilityState::Failed {
                included.clone()
            } else {
                Vec::new()
            },
            covered_file_count: if matches!(
                state,
                CapabilityState::Unavailable | CapabilityState::Failed
            ) {
                0
            } else {
                included.len() as u64
            },
            covered_entity_count: entities,
            covered_relation_count: relations,
            ..Coverage::default()
        },
        diagnostics,
        adapter: Some(semantic_adapter(language)),
        analyzer: Some(semantic_analyzer()),
        provenance_class: ProvenanceClass::SemanticResult,
        observed_at_unix_micros: analysis.generated_at_unix_micros,
        freshness: analysis.freshness.clone(),
        uncertainty: if state == CapabilityState::Available {
            Uncertainty::none()
        } else {
            Uncertainty {
                level: UncertaintyLevel::Medium,
                reasons: vec!["semantic coverage is incomplete for the declared scope".to_owned()],
            }
        },
    }
}

fn replace_semantic_capability(analysis: &mut AnalysisSnapshot, report: CapabilityReport) {
    analysis.capabilities.retain(|current| {
        !(current.language == report.language && current.capability == Capability::Semantic)
    });
    analysis.capabilities.push(report);
}

fn update_ecosystem_capability(
    analysis: &mut AnalysisSnapshot,
    language: &Language,
    semantic_state: CapabilityState,
    diagnostics: &[String],
) {
    if let Some(report) = analysis.capabilities.iter_mut().find(|report| {
        report.language.as_ref() == Some(language) && report.capability == Capability::Ecosystem
    }) {
        report.state = if report.coverage.included.is_empty() {
            CapabilityState::Unavailable
        } else if semantic_state == CapabilityState::Available {
            CapabilityState::Available
        } else {
            CapabilityState::Partial
        };
        report.reason = (report.state != CapabilityState::Available).then(|| {
            "manifest context was observed, but semantic dependency resolution is incomplete"
                .to_owned()
        });
        report.usable_remainder =
            Some("local manifest and source-module context is available".to_owned());
        report.adapter = Some(semantic_adapter(language));
        report.analyzer = Some(semantic_analyzer());
        report.diagnostics = diagnostics.to_vec();
        report.provenance_class = ProvenanceClass::SemanticResult;
    }
}

fn apply_canonical_links(
    analysis: &mut AnalysisSnapshot,
    links: Vec<(CanonicalLinkSelector, CanonicalReference)>,
    canonical_grounding: &crate::CanonicalGrounding,
) -> Result<(), SemanticAnalysisError> {
    for (selector, target) in links {
        canonical_grounding
            .validate_reference(&target)
            .map_err(|error| {
                SemanticAnalysisError::new(format!(
                    "canonical link target is not grounded: {error}"
                ))
            })?;
        let Some(entity) = analysis.structural_facts.iter_mut().find(|fact| {
            fact.entity.language == selector.language
                && fact.entity.area.path == selector.locator
                && fact.entity.qualified_name.as_deref() == Some(selector.qualified_name.as_str())
        }) else {
            return Err(SemanticAnalysisError::new(format!(
                "canonical link selector did not match an analysis entity: {}:{}",
                selector.locator, selector.qualified_name
            )));
        };
        entity.entity.canonical_links.push(target);
        entity.entity.canonical_links.sort();
        entity.entity.canonical_links.dedup();
    }
    Ok(())
}

fn semantic_snapshot_identity(
    analysis: &AnalysisSnapshot,
) -> Result<AnalysisSnapshotId, SemanticAnalysisError> {
    #[derive(Serialize)]
    struct Basis<'a> {
        format_version: u32,
        repository_snapshot: crate::RepositorySnapshotId,
        capabilities: &'a [CapabilityReport],
        diagnostics: &'a [AnalysisDiagnostic],
        structural_bases: &'a [FileAnalysisBasis],
        semantic_bases: &'a [FileAnalysisBasis],
        semantic_results: &'a [SemanticAnalysisResult],
        links: Vec<(&'a str, &'a [CanonicalReference])>,
    }
    let links = analysis
        .structural_facts
        .iter()
        .filter(|fact| fact.entity.canonical_links.len() > 1)
        .map(|fact| {
            (
                fact.entity.identity.as_str(),
                fact.entity.canonical_links.as_slice(),
            )
        })
        .collect();
    let bytes = serde_json::to_vec(&Basis {
        format_version: ANALYSIS_SNAPSHOT_FORMAT_VERSION,
        repository_snapshot: analysis.repository_snapshot,
        capabilities: &analysis.capabilities,
        diagnostics: &analysis.diagnostics,
        structural_bases: &analysis.structural_bases,
        semantic_bases: &analysis.semantic_bases,
        semantic_results: &analysis.semantic_results,
        links,
    })
    .map_err(|error| {
        SemanticAnalysisError::new(format!(
            "semantic analysis basis serialization failed: {error}"
        ))
    })?;
    Ok(AnalysisSnapshotId::digest(&[
        b"volicord.semantic_analysis_snapshot.v4",
        analysis.repository_snapshot.as_bytes(),
        &bytes,
    ]))
}

fn rebind_analysis_snapshot(analysis: &mut AnalysisSnapshot, identity: AnalysisSnapshotId) {
    let entity_map = analysis
        .structural_facts
        .iter()
        .map(|fact| {
            (
                fact.entity.identity.clone(),
                digest_string(&[
                    b"volicord.semantic_snapshot_entity.v4",
                    identity.as_bytes(),
                    fact.entity.identity.as_bytes(),
                ]),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for fact in &mut analysis.structural_facts {
        fact.entity.identity = entity_map
            .get(&fact.entity.identity)
            .cloned()
            .unwrap_or_else(|| fact.entity.identity.clone());
        fact.entity.analysis_snapshot = identity;
        fact.provenance.analysis.analysis_snapshot = identity;
        for relation in &mut fact.relations {
            relation.identity = digest_string(&[
                b"volicord.semantic_snapshot_structural_relation.v4",
                identity.as_bytes(),
                relation.identity.as_bytes(),
            ]);
            relation.analysis_snapshot = identity;
            rebind_target(&mut relation.target, &entity_map);
            relation.source_entity = entity_map
                .get(&relation.source_entity)
                .cloned()
                .unwrap_or_else(|| relation.source_entity.clone());
        }
    }
    for result in &mut analysis.semantic_results {
        result.relation.identity = digest_string(&[
            b"volicord.semantic_snapshot_relation.v4",
            identity.as_bytes(),
            result.relation.identity.as_bytes(),
        ]);
        result.relation.analysis_snapshot = identity;
        result.relation.source_entity = entity_map
            .get(&result.relation.source_entity)
            .cloned()
            .unwrap_or_else(|| result.relation.source_entity.clone());
        rebind_target(&mut result.relation.target, &entity_map);
        result.provenance.analysis.analysis_snapshot = identity;
    }
    for annotation in &mut analysis.semantic_annotations {
        annotation.analysis_snapshot = identity;
    }
    for interpretation in &mut analysis.agent_interpretations {
        interpretation.analysis_snapshot = identity;
    }
    analysis.identity = identity;
}

fn rebind_target(target: &mut RelationTarget, entity_map: &BTreeMap<String, String>) {
    if let RelationTarget::ResolvedEntity(identity) = target {
        if let Some(rebound) = entity_map.get(identity) {
            *identity = rebound.clone();
        }
    }
}

fn structural_state(analysis: &AnalysisSnapshot, language: &Language) -> CapabilityState {
    analysis
        .capabilities
        .iter()
        .find(|report| {
            report.language.as_ref() == Some(language)
                && report.capability == Capability::Structural
        })
        .map_or(CapabilityState::Unavailable, |report| report.state)
}

fn uncertainty_for_target(target: &RelationTarget) -> Uncertainty {
    match target {
        RelationTarget::ResolvedEntity(_) => Uncertainty::none(),
        RelationTarget::Unresolved(unresolved) => Uncertainty {
            level: UncertaintyLevel::High,
            reasons: vec![unresolved.reason.clone()],
        },
    }
}

fn is_builtin_target(target: &RelationTarget) -> bool {
    match target {
        RelationTarget::Unresolved(target) => matches!(
            target.display.trim_matches('&'),
            "String" | "str" | "string" | "void" | "None" | "Self"
        ),
        RelationTarget::ResolvedEntity(_) => false,
    }
}

fn qualified_parent(entity: &CodeEntity) -> &str {
    entity
        .qualified_name
        .as_deref()
        .and_then(|qualified| qualified.rsplit_once('.').map(|(parent, _)| parent))
        .unwrap_or_default()
}

fn target_label(target: &RelationTarget) -> &str {
    match target {
        RelationTarget::ResolvedEntity(identity) => identity,
        RelationTarget::Unresolved(target) => &target.display,
    }
}

fn semantic_kind_label(kind: &SemanticRelationKind) -> &'static str {
    match kind {
        SemanticRelationKind::Defines => "defines",
        SemanticRelationKind::References => "references",
        SemanticRelationKind::ResolvesTo => "resolves_to",
        SemanticRelationKind::TypeOf => "type_of",
        SemanticRelationKind::Implements => "implements",
        SemanticRelationKind::Overrides => "overrides",
        SemanticRelationKind::InstantiatedBy => "instantiated_by",
        SemanticRelationKind::LanguageSpecific(_) => "language_specific",
    }
}

fn language_label(language: &Language) -> &'static str {
    match language {
        Language::Java => "java",
        Language::TypeScript => "typescript",
        Language::Rust => "rust",
        _ => "unsupported",
    }
}

fn repository_area() -> AreaId {
    AreaId {
        kind: AreaKind::Repository,
        path: ".".to_owned(),
    }
}

fn diagnostic(
    language: &Language,
    area: AreaId,
    code: &str,
    message: &str,
    severity: DiagnosticSeverity,
    usable_remainder: Option<&str>,
) -> AnalysisDiagnostic {
    AnalysisDiagnostic {
        identity: digest_string(&[
            b"volicord.semantic_diagnostic.v1",
            language_label(language).as_bytes(),
            area.path.as_bytes(),
            code.as_bytes(),
            message.as_bytes(),
        ]),
        severity,
        code: code.to_owned(),
        message: message.to_owned(),
        affected_area: area,
        capability: Capability::Semantic,
        adapter: Some(semantic_adapter(language)),
        analyzer: Some(semantic_analyzer()),
        usable_remainder: usable_remainder.map(str::to_owned),
    }
}

fn digest_string(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    let digest = hasher.finalize();
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{analyze_repository_semantics_inner, SemanticAnalysisRequest};
    use crate::{
        Capability, CapabilityState, InventoryRequest, Language, StructuralAnalysisRequest,
    };
    use std::collections::BTreeSet;
    use std::error::Error;
    use std::path::Path;
    use volicord_context::{ProjectId, SourceId};

    #[test]
    fn injected_adapter_failure_publishes_no_semantic_fact_for_failed_language(
    ) -> Result<(), Box<dyn Error>> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../validation/repository-intelligence/polyglot-structural/fixtures/rust");
        let grounding = crate::canonical::test_repository_grounding(
            ProjectId::from_bytes([0x91; 16]),
            SourceId::from_bytes([0x92; 16]),
        )?;
        let inventory = InventoryRequest::new(
            &root,
            &grounding,
            SourceId::from_bytes([0x92; 16]),
            1_725_000_000_000_000,
        )?;
        let request = SemanticAnalysisRequest::new(StructuralAnalysisRequest::new(inventory));
        let (_, analysis) = analyze_repository_semantics_inner(
            request,
            &[Language::Rust].into_iter().collect::<BTreeSet<_>>(),
        )?;
        assert!(analysis.semantic_results.is_empty());
        assert!(analysis.capabilities.iter().any(|report| {
            report.language == Some(Language::Rust)
                && report.capability == Capability::Semantic
                && report.state == CapabilityState::Failed
        }));
        assert!(analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "semantic.adapter_failed"));
        Ok(())
    }
}
