use crate::MapEntity;
use std::collections::{BTreeMap, BTreeSet};
use volicord_context::{Availability, CanonicalReadBasis, SourceFreshness};
use volicord_repository_intelligence::{
    AnalysisSnapshot, AnalysisSnapshotId, BodyObservationKind, BodyObservations,
    CanonicalSourceBasis, Capability, CapabilityState, CodeEntity, CoordinateConvention,
    FreshnessState, InventoryClassification, Language, SourceRange, StructuralFact,
    BODY_EXPRESSION_BYTE_LIMIT, BODY_OBSERVATIONS_KEY, BODY_OBSERVATIONS_LIMIT,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeExplanationState {
    Current,
    Partial,
    Stale,
    Unsupported,
    Unavailable,
}

/// Borrowed indexes preserve negative evidence before display bounds. No body
/// payload is materialized for entities that are absent from the read model.
pub(crate) struct CodeEvidenceIndex<'a> {
    entities: BTreeMap<(AnalysisSnapshotId, &'a str), (&'a AnalysisSnapshot, &'a StructuralFact)>,
    ambiguous: BTreeSet<(AnalysisSnapshotId, &'a str)>,
    files: BTreeMap<
        (AnalysisSnapshotId, &'a str),
        &'a volicord_repository_intelligence::FileAnalysisBasis,
    >,
    inventory: BTreeMap<
        (AnalysisSnapshotId, &'a str),
        &'a volicord_repository_intelligence::InventoryEntry,
    >,
    sources: BTreeMap<volicord_context::SourceId, &'a volicord_context::SourceReadBasis>,
    relations: BTreeMap<(AnalysisSnapshotId, &'a str), InterpretationRelation<'a>>,
    ambiguous_relations: BTreeSet<(AnalysisSnapshotId, &'a str)>,
}

struct InterpretationRelation<'a> {
    source: &'a str,
    target: Option<&'a str>,
    range: Option<&'a SourceRange>,
    repository: volicord_repository_intelligence::RepositorySnapshotId,
    analysis: AnalysisSnapshotId,
    freshness: FreshnessState,
    capability: Capability,
}

impl<'a> CodeEvidenceIndex<'a> {
    pub(crate) fn new(
        canonical: &'a CanonicalReadBasis,
        analyses: &[&'a AnalysisSnapshot],
    ) -> Self {
        let mut index = Self {
            entities: BTreeMap::new(),
            ambiguous: BTreeSet::new(),
            files: BTreeMap::new(),
            inventory: BTreeMap::new(),
            sources: canonical.sources.iter().map(|s| (s.source.id, s)).collect(),
            relations: BTreeMap::new(),
            ambiguous_relations: BTreeSet::new(),
        };
        for analysis in analyses
            .iter()
            .filter(|a| a.project.identity() == canonical.project.id)
        {
            for fact in &analysis.structural_facts {
                let key = (analysis.identity, fact.entity.identity.as_str());
                if index.entities.insert(key, (analysis, fact)).is_some() {
                    index.ambiguous.insert(key);
                }
            }
            for basis in &analysis.structural_bases {
                index
                    .files
                    .insert((analysis.identity, &basis.area.path), basis);
            }
            for entry in &analysis.inventory.entries {
                index
                    .inventory
                    .insert((analysis.identity, &entry.area.path), entry);
            }
            for relation in analysis.structural_facts.iter().flat_map(|f| &f.relations) {
                index.add_relation(
                    analysis.identity,
                    &relation.identity,
                    InterpretationRelation {
                        source: &relation.source_entity,
                        target: resolved_entity(&relation.target),
                        range: relation.supporting_range.as_ref(),
                        repository: relation.repository_snapshot,
                        analysis: relation.analysis_snapshot,
                        freshness: relation.freshness.state,
                        capability: Capability::Structural,
                    },
                );
            }
            for result in &analysis.semantic_results {
                let relation = &result.relation;
                index.add_relation(
                    analysis.identity,
                    &relation.identity,
                    InterpretationRelation {
                        source: &relation.source_entity,
                        target: resolved_entity(&relation.target),
                        range: relation.supporting_range.as_ref(),
                        repository: relation.repository_snapshot,
                        analysis: relation.analysis_snapshot,
                        freshness: relation.freshness.state,
                        capability: Capability::Semantic,
                    },
                );
            }
        }
        index
    }

    fn add_relation(
        &mut self,
        analysis: AnalysisSnapshotId,
        identity: &'a str,
        relation: InterpretationRelation<'a>,
    ) {
        if self
            .relations
            .insert((analysis, identity), relation)
            .is_some()
        {
            self.ambiguous_relations.insert((analysis, identity));
        }
    }

    pub(crate) fn interpretation(
        &self,
        analysis: &AnalysisSnapshot,
        input: &volicord_repository_intelligence::AgentInterpretation,
    ) -> crate::MapInterpretation {
        let mut output = crate::MapInterpretation {
            identity: input.identity.clone(),
            text: input.text.clone(),
            source_basis: input.source_basis.iter().map(|s| s.identity()).collect(),
            analysis_snapshot: input.analysis_snapshot,
            repository_snapshot: analysis.repository_snapshot,
            freshness: analysis.freshness.clone(),
            state: CodeExplanationState::Current,
            entity_basis: Vec::new(),
            relation_basis: Vec::new(),
            source_ranges: Vec::new(),
            producer: format!("{} / {} / {}", input.agent, input.host, input.session),
            generated_at_unix_micros: input.generated_at_unix_micros,
            known_gaps: input.known_gaps.clone(),
            uncertainty: input.uncertainty.clone(),
        };
        let mut invalid = input.analysis_snapshot != analysis.identity
            || input.provenance_class
                != volicord_repository_intelligence::ProvenanceClass::AgentInterpretation
            || input.analysis_basis.is_empty()
            || input.source_basis.is_empty()
            || input.source_basis.iter().any(|s| !self.source_valid(s));
        let mut states = Vec::new();
        for basis in &input.analysis_basis {
            let key = (analysis.identity, basis.as_str());
            if let Some((_, fact)) = self.entities.get(&key) {
                output.entity_basis.push(basis.clone());
                output
                    .source_ranges
                    .extend(fact.entity.source_range.iter().cloned());
                invalid |= !input.source_basis.contains(&fact.entity.source);
                let (state, reason) = self.validity(analysis.identity, basis);
                states.push(state);
                output.known_gaps.extend(reason);
            } else if let Some(relation) = self.relations.get(&key) {
                output.relation_basis.push(basis.clone());
                output.entity_basis.push(relation.source.to_owned());
                output.source_ranges.extend(relation.range.cloned());
                invalid |= self.ambiguous_relations.contains(&key)
                    || relation.analysis != analysis.identity
                    || relation.repository != analysis.repository_snapshot;
                if relation.capability == Capability::Semantic {
                    if let Some((_, fact)) =
                        self.entities.get(&(analysis.identity, relation.source))
                    {
                        let file = analysis.semantic_bases.iter().find(|b| {
                            b.area == fact.entity.area && b.language == fact.entity.language
                        });
                        let scope_state = file.map_or(CapabilityState::Unavailable, |b| b.state);
                        states.push(explanation_state(scope_state));
                        if scope_state != CapabilityState::Available {
                            output.known_gaps.push(format!("Semantic source file coverage is {scope_state:?}; generated relation interpretation remains limited."));
                        }
                        for report in analysis.capabilities.iter().filter(|r| {
                            r.capability == Capability::Semantic
                                && r.language.as_ref() == Some(&fact.entity.language)
                        }) {
                            if !matches!(
                                report.state,
                                CapabilityState::Available | CapabilityState::Partial
                            ) {
                                states.push(explanation_state(report.state));
                            }
                        }
                    }
                }
                let source_key = (analysis.identity, relation.source);
                if let Some((_, fact)) = self.entities.get(&source_key) {
                    invalid |= !input.source_basis.contains(&fact.entity.source)
                        || !relation.range.is_some_and(|range| {
                            range.source == fact.entity.source
                                && range.repository_snapshot == analysis.repository_snapshot
                                && range.locator == fact.entity.area.path
                                && fact
                                    .entity
                                    .source_range
                                    .as_ref()
                                    .is_some_and(|entity_range| {
                                        (range.start.line, range.start.column)
                                            >= (entity_range.start.line, entity_range.start.column)
                                            && (range.end.line, range.end.column)
                                                <= (entity_range.end.line, entity_range.end.column)
                                    })
                        });
                } else {
                    invalid = true;
                }
                let endpoints = std::iter::once(relation.source).chain(relation.target);
                for endpoint in endpoints {
                    let (state, reason) = self.validity(analysis.identity, endpoint);
                    states.push(state);
                    output.known_gaps.extend(reason);
                }
                states.push(match relation.freshness {
                    FreshnessState::Current => CodeExplanationState::Current,
                    FreshnessState::Stale => CodeExplanationState::Stale,
                    FreshnessState::Unknown => CodeExplanationState::Unavailable,
                });
            } else {
                invalid = true;
            }
        }
        output.state = if invalid || states.contains(&CodeExplanationState::Unavailable) {
            CodeExplanationState::Unavailable
        } else if states.contains(&CodeExplanationState::Unsupported) {
            CodeExplanationState::Unsupported
        } else if states.contains(&CodeExplanationState::Stale) {
            CodeExplanationState::Stale
        } else if states.contains(&CodeExplanationState::Partial) {
            CodeExplanationState::Partial
        } else {
            CodeExplanationState::Current
        };
        if matches!(
            output.state,
            CodeExplanationState::Unavailable | CodeExplanationState::Unsupported
        ) {
            output.text.clear();
            output.known_gaps.push("Generated prose withheld: its exact Source/entity/relation basis is missing, ambiguous, failed, unsupported or foreign.".into());
        }
        output.entity_basis.sort();
        output.entity_basis.dedup();
        output
    }

    pub(crate) fn source_valid(
        &self,
        source: &volicord_repository_intelligence::CanonicalSourceRef,
    ) -> bool {
        self.sources.get(&source.identity()).is_some_and(|s| {
            s.source.project_id == source.project()
                && s.snapshot_basis
                    .as_ref()
                    .map_or(CanonicalSourceBasis::NotApplicable, |basis| {
                        CanonicalSourceBasis::Snapshot(basis.clone())
                    })
                    == *source.basis()
                && s.availability == Availability::Available
                && s.freshness == SourceFreshness::Current
        })
    }

    pub(crate) fn validity(
        &self,
        analysis_id: AnalysisSnapshotId,
        identity: &str,
    ) -> (CodeExplanationState, Option<String>) {
        let invalid = |reason: &str| (CodeExplanationState::Unavailable, Some(reason.to_owned()));
        if self.ambiguous.contains(&(analysis_id, identity)) {
            return invalid(
                "Ambiguous entity identity: multiple source observations use the same identity.",
            );
        }
        let Some((analysis, fact)) = self.entities.get(&(analysis_id, identity)) else {
            return invalid("Entity identity is absent from the applicable Analysis Snapshot.");
        };
        let entity = &fact.entity;
        let provenance = &fact.provenance.analysis;
        if analysis.freshness.repository_snapshot != analysis.repository_snapshot
            || (analysis.freshness.state == FreshnessState::Current
                && analysis
                    .freshness
                    .compared_repository_snapshot
                    .is_some_and(|compared| compared != analysis.repository_snapshot))
            || entity.analysis_snapshot != analysis.identity
            || entity.repository_snapshot != analysis.repository_snapshot
            || entity.source.project() != analysis.project.identity()
            || entity.source != analysis.repository_source
            || provenance.analysis_snapshot != analysis.identity
            || provenance.repository_snapshot != analysis.repository_snapshot
            || !provenance.source_basis.contains(&entity.source)
            || entity.freshness.repository_snapshot != analysis.repository_snapshot
        {
            return invalid("Entity Source, provenance or snapshot identity does not match its owning analysis.");
        }
        if !self.source_valid(&entity.source) {
            return invalid("Canonical Source is absent, foreign, stale or unavailable.");
        }
        let Some(range) = &entity.source_range else {
            return invalid("Entity has no source range.");
        };
        if range.source != entity.source
            || range.repository_snapshot != analysis.repository_snapshot
            || range.locator != entity.area.path
            || range.adapter != fact.provenance.adapter
            || provenance.adapter.as_ref() != Some(&fact.provenance.adapter)
            || provenance.analyzer.as_ref() != Some(&fact.provenance.analyzer)
        {
            return invalid("Entity range or parser provenance does not match its Source.");
        }
        match (analysis.freshness.state, entity.freshness.state) {
            (FreshnessState::Stale, _) | (_, FreshnessState::Stale) => return (CodeExplanationState::Stale, Some("Stored source differs from current repository evidence; body observations are historical.".into())),
            (FreshnessState::Unknown, _) | (_, FreshnessState::Unknown) => return invalid("Current repository comparison is unavailable; behavior evidence cannot be current."),
            _ => {}
        }
        let key = (analysis_id, entity.area.path.as_str());
        let Some(entry) = self.inventory.get(&key) else {
            return invalid("Source file is removed or absent from the applicable inventory.");
        };
        let Some(basis) = self.files.get(&key) else {
            return invalid("No structural file analysis basis supports this entity.");
        };
        if !entry
            .classifications
            .contains(&InventoryClassification::Included)
            || entry.content_sha256.as_ref() != Some(&basis.content_sha256)
            || basis.language != entity.language
            || basis.adapter != fact.provenance.adapter
            || basis.analyzer != fact.provenance.analyzer
        {
            return invalid(
                "File content, language or analyzer basis differs from the entity evidence.",
            );
        }
        let contains = |area: &volicord_repository_intelligence::AreaId| {
            area.path == "."
                || area.path == entity.area.path
                || entity.area.path.starts_with(&format!("{}/", area.path))
        };
        for report in analysis.capabilities.iter().filter(|r| {
            r.capability == Capability::Structural
                && r.language.as_ref().is_none_or(|l| l == &entity.language)
                && contains(&r.area)
        }) {
            let state = if report.coverage.failed.iter().any(&contains) {
                Some(CapabilityState::Failed)
            } else if report.coverage.unsupported.iter().any(&contains) {
                Some(CapabilityState::Unsupported)
            } else if report.coverage.unavailable.iter().any(&contains) {
                Some(CapabilityState::Unavailable)
            } else if report.coverage.stale.iter().any(&contains) {
                Some(CapabilityState::Stale)
            } else if !matches!(
                report.state,
                CapabilityState::Available | CapabilityState::Partial
            ) {
                Some(report.state)
            } else {
                None
            };
            if let Some(state) = state {
                return (
                    explanation_state(state),
                    Some(format!(
                        "Structural coverage is {state:?} for {}: {}; usable remainder: {}",
                        entity.area.path,
                        report
                            .reason
                            .as_deref()
                            .unwrap_or("inspect analysis diagnostics"),
                        report.usable_remainder.as_deref().unwrap_or("not reported")
                    )),
                );
            }
        }
        if basis.state != CapabilityState::Available {
            return (explanation_state(basis.state), Some(format!("File analysis is {:?}; diagnostics: {:?}. Only the reported usable remainder supports explanation.", basis.state, basis.diagnostic_ids)));
        }
        (CodeExplanationState::Current, None)
    }

    pub(crate) fn apply(&self, entity: &mut MapEntity) {
        let (state, reason) = self.validity(entity.analysis_snapshot, &entity.identity);
        if state != CodeExplanationState::Current
            && (entity.behavior.state == CodeExplanationState::Current
                || state != CodeExplanationState::Partial)
        {
            entity.behavior.state = state;
        }
        if let Some(reason) = reason {
            entity.behavior.limitations.push(reason);
        }
        if matches!(
            state,
            CodeExplanationState::Unavailable | CodeExplanationState::Unsupported
        ) {
            entity.behavior.claims.clear();
        }
    }
}

fn resolved_entity(target: &volicord_repository_intelligence::RelationTarget) -> Option<&str> {
    if let volicord_repository_intelligence::RelationTarget::ResolvedEntity(identity) = target {
        Some(identity)
    } else {
        None
    }
}

fn explanation_state(state: CapabilityState) -> CodeExplanationState {
    match state {
        CapabilityState::Available => CodeExplanationState::Current,
        CapabilityState::Partial => CodeExplanationState::Partial,
        CapabilityState::Stale => CodeExplanationState::Stale,
        CapabilityState::Unsupported => CodeExplanationState::Unsupported,
        CapabilityState::Unavailable | CapabilityState::Failed => CodeExplanationState::Unavailable,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeBehaviorClaim {
    pub kind: BodyObservationKind,
    pub expression: String,
    pub source_range: SourceRange,
}

/// Source syntax, separate from generated interpretations and actual execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeBehaviorReading {
    pub state: CodeExplanationState,
    pub claims: Vec<CodeBehaviorClaim>,
    pub omitted_count: usize,
    pub limitations: Vec<String>,
}

impl CodeBehaviorReading {
    pub fn unavailable() -> Self {
        Self { state: CodeExplanationState::Unavailable, claims: Vec::new(), omitted_count: 0,
            limitations: vec!["No bounded function-body observations were retained; structure alone cannot explain behavior.".into()] }
    }

    pub(crate) fn from_entity(entity: &CodeEntity) -> Self {
        let mut reading = Self::unavailable();
        if !matches!(
            entity.language,
            Language::Rust | Language::Python | Language::JavaScript | Language::TypeScript
        ) {
            reading.state = CodeExplanationState::Unsupported;
            reading.limitations = vec!["Function-body explanation observations are not supported for this language; existing inventory and analysis remain available.".into()];
            return reading;
        }
        let Some(range) = &entity.source_range else {
            return reading;
        };
        let namespace = match entity.language {
            Language::Rust => "rust.syntax",
            Language::Python => "python.syntax",
            Language::JavaScript => "javascript.syntax",
            Language::TypeScript => "typescript.syntax",
            _ => return reading,
        };
        let extensions = entity
            .extensions
            .iter()
            .filter(|extension| extension.values.contains_key(BODY_OBSERVATIONS_KEY))
            .collect::<Vec<_>>();
        let [extension] = extensions.as_slice() else {
            return reading;
        };
        if extension.namespace != namespace
            || extension.language != entity.language
            || extension.owning_adapter != range.adapter
            || extension.source_range.as_ref() != Some(range)
            || range.source != entity.source
            || range.repository_snapshot != entity.repository_snapshot
            || range.locator != entity.area.path
            || range.coordinate_convention != CoordinateConvention::ZeroBasedUtf8Byte
        {
            reading.limitations = vec![
                "Function-body observation Source, range or adapter does not match this entity."
                    .into(),
            ];
            return reading;
        }
        let value = &extension.values[BODY_OBSERVATIONS_KEY];
        if !value
            .get("observations")
            .and_then(|v| v.as_array())
            .is_some_and(|v| v.len() <= BODY_OBSERVATIONS_LIMIT)
        {
            return reading;
        }
        let Ok(body) = serde_json::from_value::<BodyObservations>(value.clone()) else {
            return reading;
        };
        let position = |p: volicord_repository_intelligence::SourcePosition| (p.line, p.column);
        if body.observations.iter().any(|claim| {
            claim.expression.is_empty()
                || claim.expression.len() > BODY_EXPRESSION_BYTE_LIMIT
                || position(claim.start) < position(range.start)
                || position(claim.end) > position(range.end)
                || position(claim.start) >= position(claim.end)
        }) {
            return reading;
        }
        reading.claims = body
            .observations
            .into_iter()
            .map(|claim| {
                let mut source_range = range.clone();
                source_range.start = claim.start;
                source_range.end = claim.end;
                source_range.meaning = volicord_repository_intelligence::RangeMeaning::Symbol;
                CodeBehaviorClaim {
                    kind: claim.kind,
                    expression: claim.expression,
                    source_range,
                }
            })
            .collect();
        reading.omitted_count = body.omitted_count;
        reading.state = match entity.freshness.state {
            FreshnessState::Stale => CodeExplanationState::Stale,
            FreshnessState::Unknown => CodeExplanationState::Unavailable,
            FreshnessState::Current
                if !entity.diagnostics.is_empty()
                    || !extension.diagnostics.is_empty()
                    || body.omitted_count > 0 =>
            {
                CodeExplanationState::Partial
            }
            FreshnessState::Current => CodeExplanationState::Current,
        };
        reading.limitations = vec!["Source syntax only: branch execution, call outcomes, external effects and runtime/data flow were not observed.".into()];
        if reading.state != CodeExplanationState::Current {
            reading.limitations.push(format!("Explanation evidence is {:?}; inspect source freshness, parser diagnostics and omissions.", reading.state));
        }
        reading
    }
}

pub(crate) fn behavior_sentences(entity: &MapEntity, korean: bool) -> String {
    let behavior = &entity.behavior;
    if !matches!(
        behavior.state,
        CodeExplanationState::Current | CodeExplanationState::Partial
    ) || behavior.claims.is_empty()
    {
        return String::new();
    }
    let mut sentences = Vec::new();
    for claim in &behavior.claims {
        let verb = match (claim.kind, korean) {
            (BodyObservationKind::Inputs, false) => "declares inputs",
            (BodyObservationKind::Condition, false) => "contains a condition",
            (BodyObservationKind::Call, false) => "contains a call expression",
            (BodyObservationKind::Binding, false) => "binds a value with",
            (BodyObservationKind::Assignment, false) => "contains an assignment",
            (BodyObservationKind::Return, false) => "has a return expression",
            (BodyObservationKind::Documentation, false) => "documents its responsibility as",
            (BodyObservationKind::Inputs, true) => "입력 선언",
            (BodyObservationKind::Condition, true) => "조건식",
            (BodyObservationKind::Call, true) => "호출식",
            (BodyObservationKind::Binding, true) => "값 바인딩",
            (BodyObservationKind::Assignment, true) => "대입식",
            (BodyObservationKind::Return, true) => "반환식",
            (BodyObservationKind::Documentation, true) => "소스에 기록된 책임",
        };
        sentences.push(format!(
            "{} {verb}: `{}`.",
            entity.display_name, claim.expression
        ));
    }
    sentences.push(if korean { "소스 구문을 설명하며 분기 실행, 호출 결과와 외부 효과는 관찰되지 않았습니다." } else { "These source expressions do not establish branch execution, call outcomes or external effects." }.into());
    sentences.join(" ")
}
