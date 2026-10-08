use crate::MapEntity;
use std::collections::{BTreeMap, BTreeSet};
use volicord_context::{Availability, CanonicalReadBasis, SourceFreshness};
use volicord_repository_intelligence::{
    AnalysisSnapshot, AnalysisSnapshotId, BodyControl, BodyObservationKind, BodyObservations,
    BodyValue, BodyValueKind, CanonicalSourceBasis, Capability, CapabilityState, CodeEntity,
    CoordinateConvention, FreshnessState, InventoryClassification, Language, SourceRange,
    StructuralFact, BODY_EXPRESSION_BYTE_LIMIT, BODY_OBSERVATIONS_KEY, BODY_OBSERVATIONS_LIMIT,
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
    pub control: BodyControl,
    pub value: Option<BodyValue>,
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
    pub(crate) fn has_supported_operations(&self) -> bool {
        matches!(
            self.state,
            CodeExplanationState::Current | CodeExplanationState::Partial
        ) && self.claims.iter().any(|claim| {
            matches!(
                claim.kind,
                BodyObservationKind::Return
                    | BodyObservationKind::Call
                    | BodyObservationKind::Binding
                    | BodyObservationKind::Assignment
            )
        })
    }

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
        for (index, claim) in body.observations.iter().enumerate() {
            let condition_valid = |condition: usize| {
                condition < index
                    && body.observations.get(condition).is_some_and(|c| {
                        c.kind == BodyObservationKind::Condition
                            && position(c.end) <= position(claim.start)
                    })
            };
            let valid = match claim.control {
                BodyControl::StraightLine | BodyControl::Unspecified => true,
                BodyControl::Conditional { condition } => condition_valid(condition),
                BodyControl::AfterEarlyReturn {
                    condition,
                    returned,
                } => {
                    condition_valid(condition)
                        && returned < index
                        && body.observations.get(returned).is_some_and(|r| {
                            r.kind == BodyObservationKind::Return
                                && r.control == BodyControl::Conditional { condition }
                                && position(r.end) <= position(claim.start)
                        })
                }
            };
            if !valid
                || claim.value.is_some_and(|v| {
                    v.start_byte >= v.end_byte
                        || claim.expression.get(v.start_byte..v.end_byte).is_none()
                })
            {
                reading.limitations = vec![
                    "Invalid body control or value evidence; behavior explanation withheld.".into(),
                ];
                return reading;
            }
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
                    control: claim.control,
                    value: claim.value,
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
        if !reading.claims.iter().any(|c| {
            matches!(
                c.kind,
                BodyObservationKind::Return
                    | BodyObservationKind::Call
                    | BodyObservationKind::Binding
                    | BodyObservationKind::Assignment
            )
        }) {
            if reading.state == CodeExplanationState::Current {
                reading.state = CodeExplanationState::Unavailable;
            }
            reading.limitations.push("No supported processing or return observation was retained; input declarations alone do not establish behavior. Inspect the exact entity's structural details.".into());
        }
        if reading
            .claims
            .iter()
            .any(|c| c.control == BodyControl::Unspecified)
        {
            reading.limitations.push("Control relationships outside the supported simple branch are unspecified; expression order alone does not establish reachability.".into());
        }
        reading
    }
}

pub(crate) fn behavior_sentences(entity: &MapEntity, korean: bool) -> String {
    entity.behavior.explanation(&entity.display_name, korean)
}

impl CodeBehaviorReading {
    /// Fixed locale realization of retained syntax; no source reads or generation.
    pub fn explanation(&self, display_name: &str, korean: bool) -> String {
        let behavior = self;
        if !matches!(
            behavior.state,
            CodeExplanationState::Current | CodeExplanationState::Partial
        ) || behavior.claims.is_empty()
        {
            return String::new();
        }
        let mut sentences = Vec::new();
        for claim in &behavior.claims {
            // Calls already described as a retained value do not need a second sentence.
            if claim.kind == BodyObservationKind::Call
                && behavior.claims.iter().any(|owner| {
                    matches!(
                        owner.kind,
                        BodyObservationKind::Return
                            | BodyObservationKind::Binding
                            | BodyObservationKind::Assignment
                    ) && (
                        owner.source_range.start.line,
                        owner.source_range.start.column,
                    ) <= (
                        claim.source_range.start.line,
                        claim.source_range.start.column,
                    ) && (owner.source_range.end.line, owner.source_range.end.column)
                        >= (claim.source_range.end.line, claim.source_range.end.column)
                })
            {
                continue;
            }
            let prefix = match claim.control {
                BodyControl::Conditional { condition } => {
                    let Some(condition) = behavior
                        .claims
                        .get(condition)
                        .filter(|c| c.kind == BodyObservationKind::Condition)
                    else {
                        return String::new();
                    };
                    let condition = &condition.expression;
                    if korean {
                        format!("`{condition}` 조건이 참인 분기에서, ")
                    } else {
                        format!("When `{condition}` holds, ")
                    }
                }
                BodyControl::AfterEarlyReturn { condition, .. } => {
                    let Some(condition) = behavior
                        .claims
                        .get(condition)
                        .filter(|c| c.kind == BodyObservationKind::Condition)
                    else {
                        return String::new();
                    };
                    let condition = &condition.expression;
                    if korean {
                        format!("`{condition}` 조건의 조기 반환을 지나 계속하는 경로에서, ")
                    } else {
                        format!("On the path continuing past the early return for `{condition}`, ")
                    }
                }
                BodyControl::Unspecified => {
                    if korean {
                        "분기 관계가 확인되지 않은 소스 표현식에서, ".into()
                    } else {
                        "With its control relationship unspecified, the source ".into()
                    }
                }
                BodyControl::StraightLine => String::new(),
            };
            let value = claim.value.and_then(|v| {
                claim
                    .expression
                    .get(v.start_byte..v.end_byte)
                    .map(|text| (v.kind, text))
            });
            let action = match (claim.kind, korean) {
                (BodyObservationKind::Inputs, false) => format!(
                    "`{}` accepts the declared inputs `{}`",
                    display_name, claim.expression
                ),
                (BodyObservationKind::Inputs, true) => format!(
                    "`{}`의 선언된 입력은 `{}`입니다",
                    display_name, claim.expression
                ),
                (BodyObservationKind::Documentation, false) => format!(
                    "Its source-authored responsibility is `{}`",
                    claim.expression
                ),
                (BodyObservationKind::Documentation, true) => {
                    format!("소스에 기록된 책임은 `{}`입니다", claim.expression)
                }
                (BodyObservationKind::Condition, _) => continue,
                (BodyObservationKind::Return, false) => match value {
                    Some((BodyValueKind::Call, v)) => format!(
                        "returns the result of calling `{v}` (source: `{}`)",
                        claim.expression
                    ),
                    Some((BodyValueKind::Calculation, v)) => format!(
                        "computes `{v}` and returns the result (source: `{}`)",
                        claim.expression
                    ),
                    Some((_, v)) => format!("returns `{v}` (source: `{}`)", claim.expression),
                    None => format!("returns without a value (source: `{}`)", claim.expression),
                },
                (BodyObservationKind::Return, true) => match value {
                    Some((BodyValueKind::Call, v)) => format!(
                        "`{v}` 호출의 결과를 반환합니다 (소스: `{}`)",
                        claim.expression
                    ),
                    Some((BodyValueKind::Calculation, v)) => format!(
                        "`{v}`를 계산하고 결과를 반환합니다 (소스: `{}`)",
                        claim.expression
                    ),
                    Some((_, v)) => format!("`{v}`를 반환합니다 (소스: `{}`)", claim.expression),
                    None => format!("값 없이 반환합니다 (소스: `{}`)", claim.expression),
                },
                (BodyObservationKind::Assignment, false) if value.is_none() => format!(
                    "updates the assignment target according to `{}`",
                    claim.expression
                ),
                (BodyObservationKind::Assignment, true) if value.is_none() => {
                    format!("`{}`에 따라 대입 대상을 갱신합니다", claim.expression)
                }
                (BodyObservationKind::Binding | BodyObservationKind::Assignment, false) => {
                    let processing = match value {
                        Some((BodyValueKind::Call, v)) => format!("requests the result of `{v}`"),
                        Some((BodyValueKind::Calculation, v)) => format!("computes `{v}`"),
                        Some((_, v)) => format!("evaluates `{v}`"),
                        None => "evaluates the source value".into(),
                    };
                    format!("{processing} and stores it with `{}`", claim.expression)
                }
                (BodyObservationKind::Binding | BodyObservationKind::Assignment, true) => {
                    let processing = match value {
                        Some((BodyValueKind::Call, v)) => format!("`{v}`의 호출 결과를 요청하고"),
                        Some((BodyValueKind::Calculation, v)) => format!("`{v}`를 계산하고"),
                        Some((_, v)) => format!("`{v}`를 평가하고"),
                        None => "소스 값을 평가하고".into(),
                    };
                    format!("{processing} `{}`로 저장합니다", claim.expression)
                }
                (BodyObservationKind::Call, false) => format!(
                    "requests the call `{}`; the callee's behavior is not established",
                    claim.expression
                ),
                (BodyObservationKind::Call, true) => format!(
                    "`{}` 호출을 요청합니다. 호출 대상의 동작은 확인되지 않았습니다",
                    claim.expression
                ),
            };
            sentences.push(format!("{prefix}{action}."));
        }
        sentences.push(if korean { "이 설명은 소스가 지정한 동작이며 분기 실행, 호출 결과와 외부 효과는 관찰되지 않았습니다. 호출 대상의 효과와 지원 범위 밖의 제어 흐름은 확인되지 않았습니다." } else { "These source operations do not establish branch execution, call outcomes or external effects. Callee effects and control flow outside the supported structure remain unknown." }.into());
        sentences.join(" ")
    }
}
