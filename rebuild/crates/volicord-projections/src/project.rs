use crate::{
    inspect_candidate, CandidateContentAccess, CandidateInspection, RecallBound, RecallInputs,
    ResumeBrief,
};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use volicord_context::{
    CanonicalReadBasis, Checkpoint, CheckpointId, ContextItemId, ContextItemRole, DecisionChoice,
    DecisionId, DecisionLifecycle, ProjectId, QuestionState, SourceFreshness, SourceId,
    SourcePayload, TimestampMicros, WorkState,
};
use volicord_inquiry::{ApplicabilityQuery, CandidateReadBasis};
use volicord_repository_intelligence::{
    AnalysisMetadata, AnalysisSnapshot, AnalysisSnapshotId, CanonicalReference, Capability,
    CapabilityReport, CapabilityState, CodeEntityKind, FreshnessBasis, Language, RelationTarget,
    RepositorySnapshotId, SourceRange, Uncertainty,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectionBound {
    pub max_items_per_section: usize,
}

impl Default for ProjectionBound {
    fn default() -> Self {
        Self {
            max_items_per_section: 64,
        }
    }
}

/// Read materialization requirements. Full Recall/document consumers keep the default.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectionReadRequirements {
    pub code: bool,
    pub inspection: bool,
}
impl Default for ProjectionReadRequirements {
    fn default() -> Self {
        Self {
            code: true,
            inspection: true,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadSectionState {
    NotRequested,
    Available,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectReadSections {
    pub code: ReadSectionState,
    pub inspection: ReadSectionState,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectionDetail {
    pub work_page: usize,
    pub decision_page: usize,
    pub decision: Option<DecisionId>,
    pub entity: Option<String>,
}

pub struct ProjectProjectionInputs<'a> {
    pub selection: crate::WorkSelector,
    pub detail: ProjectionDetail,
    pub requirements: ProjectionReadRequirements,
    pub metadata: &'a [&'a AnalysisMetadata],
    pub analysis_issues: &'a [ProjectionIssue],
    pub canonical: &'a CanonicalReadBasis,
    pub analyses: &'a [&'a AnalysisSnapshot],
    pub applicability: ApplicabilityQuery,
    pub candidates: CandidateProjectionInput<'a>,
    pub candidate_content_access: CandidateContentAccess,
    pub observed_at: TimestampMicros,
    pub bound: ProjectionBound,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateDependencyState {
    NotRequested,
    Available,
    Unavailable,
    Unsupported,
    Corrupt,
    RepairRequired,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateDependencyFailureKind {
    Unavailable,
    Unsupported,
    Corrupt,
    RepairRequired,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateDependencyFailure {
    pub kind: CandidateDependencyFailureKind,
    pub affected_scope: String,
    pub reason: String,
}

pub enum CandidateProjectionInput<'a> {
    Available(&'a CandidateReadBasis),
    Degraded {
        usable_basis: Option<&'a CandidateReadBasis>,
        failure: CandidateDependencyFailure,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionHealth {
    Complete,
    Partial,
    Degraded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionIssueKind {
    Bound,
    WrongProject,
    PartialCapability,
    UnavailableCapability,
    UnsupportedCapability,
    FailedCapability,
    StaleCapability,
    SourceUnavailable,
    SourceStale,
    CandidateInspection,
    CandidateUnavailable,
    CandidateUnsupported,
    CandidateCorrupt,
    CandidateRepairRequired,
    CandidateFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionIssue {
    pub kind: ProjectionIssueKind,
    pub identity: String,
    pub affected_scope: String,
    pub reason: String,
    /// Exact number of items omitted by a deterministic bound. Non-bound
    /// issues retain their concrete identity and report zero here.
    pub omitted_count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceStatusSummary {
    pub current: usize,
    pub stale: usize,
    pub unavailable: usize,
    pub unknown: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectOverview {
    pub project_id: ProjectId,
    pub project_name: String,
    pub canonical_revision: u64,
    pub current_goals: Vec<String>,
    pub active_decision_count: usize,
    pub superseded_decision_count: usize,
    pub open_question_count: usize,
    pub latest_checkpoint_id: Option<volicord_context::CheckpointId>,
    pub source_status: SourceStatusSummary,
    pub capability_reports: Vec<CapabilityReport>,
    pub health: ProjectionHealth,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapEntity {
    pub identity: String,
    pub display_name: String,
    /// Portable repository-relative area locator retained even when a symbol
    /// does not expose a precise Source range.
    pub locator: String,
    pub kind: CodeEntityKind,
    pub language: Language,
    pub source_id: SourceId,
    pub source_range: Option<SourceRange>,
    pub analysis_snapshot: AnalysisSnapshotId,
    pub repository_snapshot: RepositorySnapshotId,
    pub freshness: FreshnessBasis,
    pub uncertainty: Uncertainty,
    pub canonical_links: Vec<CanonicalReference>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapRelationClass {
    StructuralFact,
    SemanticResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapRelation {
    pub identity: String,
    pub class: MapRelationClass,
    pub kind: String,
    pub source_entity: String,
    pub target_entity: Option<String>,
    pub unresolved_target: Option<String>,
    pub source_id: SourceId,
    pub supporting_range: Option<SourceRange>,
    pub analysis_snapshot: AnalysisSnapshotId,
    pub repository_snapshot: RepositorySnapshotId,
    pub freshness: FreshnessBasis,
    pub uncertainty: Uncertainty,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityGap {
    pub analysis_snapshot: AnalysisSnapshotId,
    pub repository_snapshot: RepositorySnapshotId,
    pub capability: Capability,
    pub language: Option<Language>,
    pub state: CapabilityState,
    pub area: String,
    pub reason: String,
    pub affected_areas: Vec<String>,
    pub usable_remainder: Option<String>,
    pub user_visible_consequence: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapInterpretation {
    pub identity: String,
    pub text: String,
    pub source_basis: Vec<SourceId>,
    pub analysis_snapshot: AnalysisSnapshotId,
    pub repository_snapshot: RepositorySnapshotId,
    pub known_gaps: Vec<String>,
    pub uncertainty: Uncertainty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryMap {
    pub entities: Vec<MapEntity>,
    pub relations: Vec<MapRelation>,
    pub agent_interpretations: Vec<MapInterpretation>,
    pub capabilities: Vec<CapabilityReport>,
    pub gaps: Vec<CapabilityGap>,
    pub health: ProjectionHealth,
}

pub(crate) fn select_bounded_topology(
    entities: &[MapEntity],
    relations: &[MapRelation],
    important_entities: &BTreeSet<String>,
    entity_limit: usize,
    relation_limit: usize,
    include_unresolved: bool,
) -> BoundedTopology {
    select_topology(
        entities,
        relations,
        important_entities,
        entity_limit,
        relation_limit,
        include_unresolved,
    )
}

trait EntityView {
    fn identity(&self) -> &str;
    fn kind(&self) -> &CodeEntityKind;
}
trait RelationView {
    fn identity(&self) -> &str;
    fn source(&self) -> &str;
    fn target(&self) -> Option<&str>;
    fn unresolved(&self) -> bool;
    fn rank(&self) -> usize;
}
impl EntityView for MapEntity {
    fn identity(&self) -> &str {
        &self.identity
    }
    fn kind(&self) -> &CodeEntityKind {
        &self.kind
    }
}
impl RelationView for MapRelation {
    fn identity(&self) -> &str {
        &self.identity
    }
    fn source(&self) -> &str {
        &self.source_entity
    }
    fn target(&self) -> Option<&str> {
        self.target_entity.as_deref()
    }
    fn unresolved(&self) -> bool {
        self.unresolved_target.is_some()
    }
    fn rank(&self) -> usize {
        relation_kind_rank(self)
    }
}
impl EntityView for &volicord_repository_intelligence::CodeEntity {
    fn identity(&self) -> &str {
        &self.identity
    }
    fn kind(&self) -> &CodeEntityKind {
        &self.kind
    }
}

#[derive(Clone, Copy)]
enum RelationRef<'a> {
    Structural(
        &'a volicord_repository_intelligence::StructuralRelation,
        SourceId,
    ),
    Semantic(
        &'a volicord_repository_intelligence::SemanticRelation,
        SourceId,
    ),
}

struct ProjectionGraph<'a> {
    entities: Vec<&'a volicord_repository_intelligence::CodeEntity>,
    relations: Vec<RelationRef<'a>>,
}

fn projection_graph<'a>(
    canonical: &CanonicalReadBasis,
    analyses: &[&'a AnalysisSnapshot],
) -> ProjectionGraph<'a> {
    let mut entities = Vec::new();
    let mut relations = Vec::new();
    for analysis in analyses
        .iter()
        .copied()
        .filter(|analysis| analysis.project.identity() == canonical.project.id)
    {
        for fact in &analysis.structural_facts {
            entities.push(&fact.entity);
            relations.extend(
                fact.relations.iter().map(|relation| {
                    RelationRef::Structural(relation, fact.entity.source.identity())
                }),
            );
        }
        relations.extend(analysis.semantic_results.iter().map(|result| {
            let source_id = result
                .relation
                .supporting_range
                .as_ref()
                .map_or(analysis.repository_source.identity(), |range| {
                    range.source.identity()
                });
            RelationRef::Semantic(&result.relation, source_id)
        }));
    }
    entities.sort_by(|left, right| left.identity.cmp(&right.identity));
    entities.dedup_by(|left, right| left.identity == right.identity);
    relations.sort_by(|left, right| left.identity().cmp(right.identity()));
    relations.dedup_by(|left, right| left.identity() == right.identity());
    ProjectionGraph {
        entities,
        relations,
    }
}

impl RelationView for RelationRef<'_> {
    fn identity(&self) -> &str {
        match self {
            Self::Structural(r, _) => &r.identity,
            Self::Semantic(r, _) => &r.identity,
        }
    }
    fn source(&self) -> &str {
        match self {
            Self::Structural(r, _) => &r.source_entity,
            Self::Semantic(r, _) => &r.source_entity,
        }
    }
    fn target(&self) -> Option<&str> {
        let target = match self {
            Self::Structural(r, _) => &r.target,
            Self::Semantic(r, _) => &r.target,
        };
        match target {
            RelationTarget::ResolvedEntity(id) => Some(id),
            _ => None,
        }
    }
    fn unresolved(&self) -> bool {
        matches!(self, Self::Structural(r, _) if matches!(r.target, RelationTarget::Unresolved(_)))
            || matches!(self, Self::Semantic(r, _) if matches!(r.target, RelationTarget::Unresolved(_)))
    }
    fn rank(&self) -> usize {
        let kind = match self {
            Self::Structural(r, _) => format!("{:?}", r.kind),
            Self::Semantic(r, _) => format!("{:?}", r.kind),
        };
        relation_kind_rank_name(&kind)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BoundedTopology<E = MapEntity, R = MapRelation> {
    pub entities: Vec<E>,
    pub relations: Vec<R>,
    pub omitted_entity_count: usize,
    pub omitted_relation_count: usize,
}

/// Selects relationships together with the endpoint entities required to
/// inspect them. The stable relevance order prefers canonical/Decision-linked
/// endpoints, useful dependency/flow relations, important component kinds,
/// and connected structure before identity tie-breaking.
fn select_topology<E: EntityView + Clone, R: RelationView + Clone>(
    entities: &[E],
    relations: &[R],
    important_entities: &BTreeSet<String>,
    entity_limit: usize,
    relation_limit: usize,
    include_unresolved: bool,
) -> BoundedTopology<E, R> {
    let entity_limit = entity_limit.max(1);
    let entity_by_id = entities
        .iter()
        .map(|entity| (entity.identity(), entity))
        .collect::<BTreeMap<_, _>>();
    let mut degree = BTreeMap::<&str, usize>::new();
    for relation in relations {
        let Some(target) = relation.target() else {
            continue;
        };
        if entity_by_id.contains_key(relation.source()) && entity_by_id.contains_key(target) {
            *degree.entry(relation.source()).or_default() += 1;
            *degree.entry(target).or_default() += 1;
        }
    }

    let mut candidates = relations
        .iter()
        .filter(|relation| {
            entity_by_id.contains_key(relation.source())
                && match relation.target() {
                    Some(target) => entity_by_id.contains_key(target),
                    None => include_unresolved && relation.unresolved(),
                }
        })
        .collect::<Vec<_>>();
    candidates.sort_by_cached_key(|relation| {
        relation_selection_key(*relation, &entity_by_id, important_entities, &degree)
    });

    let mut ranked_important_entities = important_entities
        .iter()
        .filter_map(|identity| {
            entity_by_id
                .get(identity.as_str())
                .map(|entity| (identity, entity_kind_rank(entity.kind())))
        })
        .collect::<Vec<_>>();
    ranked_important_entities.sort_by_key(|(identity, kind_rank)| (*kind_rank, identity.as_str()));
    let mut selected_entities = ranked_important_entities
        .into_iter()
        .take(entity_limit)
        .map(|(identity, _)| identity.clone())
        .collect::<BTreeSet<_>>();
    let mut incident = BTreeMap::<&str, Vec<usize>>::new();
    for (index, relation) in candidates.iter().enumerate() {
        for endpoint in relation_endpoints(*relation) {
            incident.entry(endpoint).or_default().push(index);
        }
    }
    let mut connected_candidates = BTreeSet::<usize>::new();
    for identity in &selected_entities {
        if let Some(indices) = incident.get(identity.as_str()) {
            connected_candidates.extend(indices);
        }
    }
    let mut selected_relations = Vec::<R>::new();
    let mut selected_candidate = vec![false; candidates.len()];
    let mut fallback_at = 0;
    while selected_relations.len() < relation_limit {
        let connected = !selected_entities.is_empty();
        let fits = |index: usize| {
            if selected_candidate[index] {
                return false;
            }
            let endpoints = relation_endpoints(candidates[index]);
            let new_endpoint_count = endpoints
                .clone()
                .filter(|identity| !selected_entities.contains(*identity))
                .count();
            selected_entities.len() + new_endpoint_count <= entity_limit
        };
        let next = connected
            .then(|| {
                connected_candidates
                    .iter()
                    .copied()
                    .find(|index| fits(*index))
            })
            .flatten()
            .or_else(|| {
                while fallback_at < candidates.len() && !fits(fallback_at) {
                    fallback_at += 1;
                }
                (fallback_at < candidates.len()).then_some(fallback_at)
            });
        let Some(index) = next else {
            break;
        };
        let relation = candidates[index];
        selected_candidate[index] = true;
        connected_candidates.remove(&index);
        let new_entities = relation_endpoints(relation)
            .filter(|identity| !selected_entities.contains(*identity))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        for identity in &new_entities {
            if let Some(indices) = incident.get(identity.as_str()) {
                connected_candidates.extend(indices);
            }
        }
        selected_entities.extend(new_entities);
        selected_relations.push(relation.clone());
    }

    let mut remaining_entities = entities.iter().collect::<Vec<_>>();
    remaining_entities
        .sort_by_cached_key(|entity| entity_selection_key(*entity, important_entities, &degree));
    for entity in remaining_entities {
        if selected_entities.len() == entity_limit {
            break;
        }
        selected_entities.insert(entity.identity().to_owned());
    }

    let mut selected_entities = selected_entities
        .into_iter()
        .filter_map(|identity| entity_by_id.get(identity.as_str()).copied().cloned())
        .collect::<Vec<_>>();
    selected_entities.sort_by(|left, right| left.identity().cmp(right.identity()));
    selected_relations.sort_by(|left, right| left.identity().cmp(right.identity()));
    BoundedTopology {
        omitted_entity_count: entities.len().saturating_sub(selected_entities.len()),
        omitted_relation_count: relations.len().saturating_sub(selected_relations.len()),
        entities: selected_entities,
        relations: selected_relations,
    }
}

/// Selects only real one-hop relations touching a grounded current-work seed.
/// Unlike the generic Repository Map selector, relation endpoints consume the
/// current-work budget before disconnected seed entities do.
fn select_grounded_current_work_topology<E: EntityView + Clone, R: RelationView + Clone>(
    entities: &[E],
    relations: &[R],
    grounded_seeds: &BTreeSet<String>,
    entity_limit: usize,
    relation_limit: usize,
) -> BoundedTopology<E, R> {
    let entity_limit = entity_limit.max(1);
    let relation_limit = relation_limit.max(1);
    let entity_by_id = entities
        .iter()
        .map(|entity| (entity.identity(), entity))
        .collect::<BTreeMap<_, _>>();
    let mut relation_candidates = relations
        .iter()
        .filter(|relation| {
            entity_by_id.contains_key(relation.source())
                && match relation.target() {
                    Some(target) => {
                        entity_by_id.contains_key(target)
                            && (grounded_seeds.contains(relation.source())
                                || grounded_seeds.contains(target))
                    }
                    None => relation.unresolved() && grounded_seeds.contains(relation.source()),
                }
        })
        .collect::<Vec<_>>();
    let mut degree = BTreeMap::<&str, usize>::new();
    for relation in &relation_candidates {
        *degree.entry(relation.source()).or_default() += 1;
        if let Some(target) = relation.target() {
            *degree.entry(target).or_default() += 1;
        }
    }
    relation_candidates.sort_by_cached_key(|relation| {
        let important_endpoint_count = relation_endpoints(*relation)
            .filter(|identity| grounded_seeds.contains(*identity))
            .count();
        let connection_degree = relation_endpoints(*relation)
            .map(|identity| degree.get(identity).copied().unwrap_or_default())
            .sum::<usize>();
        (
            relation.rank(),
            usize::from(relation.target().is_none()),
            Reverse(important_endpoint_count),
            Reverse(connection_degree),
            relation.identity(),
        )
    });

    let mut relevant_entities = grounded_seeds.clone();
    for relation in &relation_candidates {
        relevant_entities.insert(relation.source().to_owned());
        if let Some(target) = relation.target() {
            relevant_entities.insert(target.to_owned());
        }
    }
    let relevant_relation_count = relation_candidates.len();
    let mut selected_ids = BTreeSet::<String>::new();
    let mut selected_relations = Vec::<R>::new();
    for relation in relation_candidates {
        if selected_relations.len() == relation_limit {
            break;
        }
        let endpoints = relation_endpoints(relation).collect::<Vec<_>>();
        let new_endpoint_count = endpoints
            .iter()
            .filter(|identity| !selected_ids.contains(**identity))
            .count();
        if selected_ids.len() + new_endpoint_count > entity_limit {
            continue;
        }
        selected_ids.extend(endpoints.into_iter().map(str::to_owned));
        selected_relations.push((*relation).clone());
    }

    let mut remaining_seeds = grounded_seeds
        .iter()
        .filter_map(|identity| entity_by_id.get(identity.as_str()).copied())
        .collect::<Vec<_>>();
    remaining_seeds
        .sort_by_cached_key(|entity| entity_selection_key(*entity, grounded_seeds, &degree));
    for entity in remaining_seeds {
        if selected_ids.len() == entity_limit {
            break;
        }
        selected_ids.insert(entity.identity().to_owned());
    }

    let mut selected_entities = selected_ids
        .iter()
        .filter_map(|identity| entity_by_id.get(identity.as_str()).copied().cloned())
        .collect::<Vec<_>>();
    selected_entities.sort_by(|left, right| left.identity().cmp(right.identity()));
    selected_relations.sort_by(|left, right| left.identity().cmp(right.identity()));
    BoundedTopology {
        omitted_entity_count: relevant_entities
            .len()
            .saturating_sub(selected_entities.len()),
        omitted_relation_count: relevant_relation_count.saturating_sub(selected_relations.len()),
        entities: selected_entities,
        relations: selected_relations,
    }
}

fn relation_endpoints<R: RelationView>(relation: &R) -> impl Iterator<Item = &str> + Clone {
    let source = relation.source();
    std::iter::once(source).chain(relation.target().filter(|target| *target != source))
}

fn relation_selection_key<'a, E: EntityView, R: RelationView>(
    relation: &'a R,
    entities: &BTreeMap<&str, &E>,
    important_entities: &BTreeSet<String>,
    degree: &BTreeMap<&str, usize>,
) -> (usize, Reverse<usize>, usize, usize, Reverse<usize>, &'a str) {
    let endpoints = std::iter::once(relation.source()).chain(relation.target());
    let important_endpoint_count = endpoints
        .clone()
        .filter(|identity| important_entities.contains(*identity))
        .count();
    let endpoint_kind_rank = endpoints
        .clone()
        .filter_map(|identity| entities.get(identity).copied())
        .map(|entity| entity_kind_rank(entity.kind()))
        .min()
        .unwrap_or(usize::MAX);
    let connection_degree = endpoints
        .map(|identity| degree.get(identity).copied().unwrap_or_default())
        .sum();
    (
        usize::from(relation.target().is_none()),
        Reverse(important_endpoint_count),
        relation.rank(),
        endpoint_kind_rank,
        Reverse(connection_degree),
        relation.identity(),
    )
}

fn entity_selection_key<'a, E: EntityView>(
    entity: &'a E,
    important_entities: &BTreeSet<String>,
    degree: &BTreeMap<&str, usize>,
) -> (Reverse<bool>, Reverse<usize>, usize, &'a str) {
    (
        Reverse(important_entities.contains(entity.identity())),
        Reverse(degree.get(entity.identity()).copied().unwrap_or_default()),
        entity_kind_rank(entity.kind()),
        entity.identity(),
    )
}

fn relation_kind_rank(relation: &MapRelation) -> usize {
    relation_kind_rank_name(&relation.kind)
}

fn relation_kind_rank_name(kind: &str) -> usize {
    match kind {
        "Imports" | "Includes" | "CallsSyntactically" | "References" | "ResolvesTo"
        | "InstantiatedBy" | "Implements" | "Overrides" => 0,
        "Inherits" | "Tests" | "Configures" | "Exports" => 1,
        "Contains" | "Declares" | "Defines" => 2,
        _ => 3,
    }
}

const fn entity_kind_rank(kind: &CodeEntityKind) -> usize {
    match kind {
        CodeEntityKind::Repository => 0,
        CodeEntityKind::Package => 1,
        CodeEntityKind::Module | CodeEntityKind::Namespace => 2,
        CodeEntityKind::File | CodeEntityKind::Configuration | CodeEntityKind::Document => 3,
        CodeEntityKind::Class
        | CodeEntityKind::Interface
        | CodeEntityKind::Trait
        | CodeEntityKind::Struct
        | CodeEntityKind::Enum
        | CodeEntityKind::Type => 4,
        CodeEntityKind::Function | CodeEntityKind::Method | CodeEntityKind::Test => 5,
        CodeEntityKind::Field | CodeEntityKind::LanguageSpecific(_) => 6,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionContextCodeLink {
    pub decision_id: DecisionId,
    pub decision_revision: u64,
    pub decision_state: crate::BriefDecisionState,
    pub declared_paths: Vec<String>,
    pub declared_components: Vec<String>,
    pub declared_work_contexts: Vec<String>,
    pub assumption_context: Vec<String>,
    pub related_context_items: Vec<volicord_context::ContextItemId>,
    pub related_code_entities: Vec<String>,
    pub supporting_sources: Vec<SourceId>,
    pub link_basis: Vec<String>,
    pub missing_or_uncertain_links: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointTimelineEntry {
    pub checkpoint: Checkpoint,
    pub work_state: volicord_context::WorkState,
    pub verification: Vec<volicord_context::VerificationFact>,
    pub user_review: volicord_context::UserReviewFact,
    pub user_acceptance: volicord_context::UserAcceptanceFact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalInspectionKind {
    Project,
    Source,
    Question,
    Decision,
    ContextItem,
    Checkpoint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalInspectionItem {
    pub kind: CanonicalInspectionKind,
    pub identity: String,
    pub revision: u64,
    pub lifecycle_state: String,
    pub statement_role: Option<String>,
    pub summary: String,
    pub source_basis: Vec<SourceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentWorkPathBasis {
    pub checkpoint_id: CheckpointId,
    pub checkpoint_revision: u64,
    pub path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentWorkCodeLink {
    pub changed_path_basis: Vec<CurrentWorkPathBasis>,
    pub entity_identity: String,
    pub changed_paths: Vec<String>,
    pub checkpoint_basis: Vec<CheckpointId>,
    pub goal_context_basis: Vec<ContextItemId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentWorkTopology {
    /// Grounded current-work seeds and real relation endpoints selected from
    /// Analysis Snapshots before the generic Repository Map presentation bound.
    pub entities: Vec<MapEntity>,
    pub relations: Vec<MapRelation>,
    pub omitted_entity_count: usize,
    pub omitted_relation_count: usize,
}

/// Projection Work evidence construction, independent of displayed bounds.
/// Operations separately counts retained-explanation freshness preparations.
/// Canonical store reads remain complete; input bytes are not allocated-byte/RSS measurements.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkReadCost {
    pub classified_works: usize,
    pub indexed_checkpoints: usize,
    pub materialized_works: usize,
    pub materialized_checkpoints: usize,
    pub evidence_input_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectProjection {
    pub repository_analysis: crate::RepositoryAnalysisReading,
    /// Contextual answer limits, selected before capability/list bounds. Raw
    /// repository diagnostics remain in repository_map and issues.
    pub answer_capability_gaps: Vec<CapabilityGap>,
    pub answer_issues: Vec<ProjectionIssue>,
    /// Ephemeral equality binding for new read/export; not canonical authority.
    pub canonical_read_fingerprint: String,
    pub sections: ProjectReadSections,
    pub work_count: usize,
    pub work_read_cost: WorkReadCost,
    pub decision_count: usize,
    pub decision_catalog: Vec<crate::UnderstandingDecision>,
    pub selected_decision: Option<crate::UnderstandingDecision>,
    pub selected_entity: Option<MapEntity>,
    pub selected_entity_relations: Vec<MapRelation>,
    pub selected_entity_neighbors: Vec<MapEntity>,
    pub omitted_selected_relation_count: usize,
    pub selection: crate::WorkSelection,
    /// Explicit selection is independent of bounded parent Work lists.
    pub selected_work: Option<crate::UnderstandingWork>,
    pub selected_work_decisions: Vec<crate::UnderstandingDecision>,
    pub work_overview: crate::WorkOverview,
    pub work_history: Vec<crate::UnderstandingWork>,
    pub unresolved_work_grouping: Vec<crate::UnresolvedWorkGrouping>,
    pub overview: ProjectOverview,
    pub resume: ResumeBrief,
    pub repository_map: RepositoryMap,
    /// A distinct bounded view of actual Repository Intelligence topology for
    /// current work. It does not synthesize relations or replace Repository Map.
    pub current_work_topology: CurrentWorkTopology,
    /// Bounded current Goal/Checkpoint-to-code seed evidence. This is not a
    /// second repository map; Project Understanding uses it to select a
    /// current-work neighborhood from `current_work_topology`.
    pub current_work_code: Vec<CurrentWorkCodeLink>,
    pub decision_context_code: Vec<DecisionContextCodeLink>,
    pub checkpoint_timeline: Vec<CheckpointTimelineEntry>,
    pub canonical_inspection: Vec<CanonicalInspectionItem>,
    pub candidate_inspection: Vec<CandidateInspection>,
    pub candidate_dependency: CandidateDependencyState,
    pub source_catalog: Vec<volicord_context::SourceReadBasis>,
    pub issues: Vec<ProjectionIssue>,
    pub health: ProjectionHealth,
}

/// Canonical and Candidate inspection sections share the same selection and
/// failure contract as the full Project projection without requiring code graphs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryInspectionProjection {
    pub canonical_inspection: Vec<CanonicalInspectionItem>,
    pub candidate_inspection: Vec<CandidateInspection>,
    pub candidate_dependency: CandidateDependencyState,
    pub issues: Vec<ProjectionIssue>,
}

pub fn build_memory_inspection(
    canonical: &CanonicalReadBasis,
    candidates: CandidateProjectionInput<'_>,
    content_access: CandidateContentAccess,
    observed_at: TimestampMicros,
    bound: ProjectionBound,
) -> MemoryInspectionProjection {
    let limit = bound.max_items_per_section.max(1);
    let mut issues = Vec::new();
    let canonical_inspection = build_canonical_inspection(canonical, limit, &mut issues);
    let (candidate_basis, candidate_dependency) = match candidates {
        CandidateProjectionInput::Available(basis) => {
            (Some(basis), CandidateDependencyState::Available)
        }
        CandidateProjectionInput::Degraded {
            usable_basis,
            failure,
        } => {
            let state = candidate_dependency_failure_state(failure.kind);
            issues.push(candidate_dependency_issue(failure));
            (usable_basis, state)
        }
    };
    let candidate_inspection = candidate_basis.map_or_else(Vec::new, |basis| {
        let mut identities = basis
            .candidates
            .iter()
            .map(|candidate| candidate.id)
            .collect::<Vec<_>>();
        identities.sort();
        if identities.len() > limit {
            issues.push(bound_issue(
                "candidate_inspection",
                identities.len() - limit,
            ));
            identities.truncate(limit);
        }
        identities
            .into_iter()
            .map(|identity| {
                let inspection = inspect_candidate(basis, identity, content_access, observed_at);
                if inspection.health != crate::InspectionHealth::Complete {
                    issues.push(ProjectionIssue {
                        kind: ProjectionIssueKind::CandidateInspection,
                        identity: identity.to_string(),
                        affected_scope: "candidate_inspection".to_owned(),
                        reason: format!(
                            "Candidate inspection is {}",
                            inspection_health_key(inspection.health)
                        ),
                        omitted_count: 0,
                    });
                }
                inspection
            })
            .collect()
    });
    sort_projection_issues(&mut issues);
    MemoryInspectionProjection {
        canonical_inspection,
        candidate_inspection,
        candidate_dependency,
        issues,
    }
}

fn sort_projection_issues(issues: &mut Vec<ProjectionIssue>) {
    issues.sort_by(|left, right| {
        (&left.affected_scope, &left.identity, &left.reason).cmp(&(
            &right.affected_scope,
            &right.identity,
            &right.reason,
        ))
    });
    issues.dedup();
}

/// Builds viewer/host-ready read models from immutable subsystem bases. The
/// function owns no store, analyzer, Candidate lifecycle, or filesystem handle.
pub fn build_project_projection(
    inputs: ProjectProjectionInputs<'_>,
) -> Result<ProjectProjection, crate::WorkSelectionError> {
    let analyses = if inputs.requirements.code {
        inputs.analyses
    } else {
        &[]
    };
    let selection = inputs.selection.resolve(inputs.canonical)?;
    let scoped = selection
        .work_item_id
        .map(|work| crate::reading::scope_to_work(inputs.canonical, work));
    let reading_canonical = if matches!(inputs.selection, crate::WorkSelector::ExactWork(_)) {
        scoped.as_ref().unwrap_or(inputs.canonical)
    } else {
        inputs.canonical
    };
    let seed_canonical;
    let topology_canonical = if let Some(scoped) = &scoped {
        scoped
    } else {
        seed_canonical = crate::reading::topology_seed_without_work(inputs.canonical);
        &seed_canonical
    };
    let limit = inputs.bound.max_items_per_section.max(1);
    let mut resume = if inputs.requirements.code {
        crate::recall::build_resume_brief_coordinated(
            RecallInputs {
                analysis_issues: inputs.analysis_issues,
                canonical: reading_canonical,
                analyses,
                scope: inputs.applicability.clone(),
                bound: RecallBound {
                    max_items_per_section: limit.min(RecallBound::default().max_items_per_section),
                },
            },
            false,
        )
    } else {
        crate::recall::build_resume_brief_metadata_coordinated(
            crate::RecallMetadataInputs {
                analysis_issues: inputs.analysis_issues,
                canonical: reading_canonical,
                analyses: inputs.metadata,
                scope: inputs.applicability.clone(),
                bound: RecallBound {
                    max_items_per_section: limit.min(RecallBound::default().max_items_per_section),
                },
            },
            false,
        )
    };
    let mut issues = source_issues(reading_canonical);
    issues.extend_from_slice(inputs.analysis_issues);
    let graph = projection_graph(reading_canonical, analyses);
    let repository_analysis = inputs
        .analyses
        .iter()
        .filter(|a| a.project.identity() == reading_canonical.project.id)
        .map(|a| {
            crate::RepositoryAnalysisReading::stored(
                a.identity,
                a.repository_snapshot,
                a.generated_at_unix_micros,
                &a.freshness,
                &a.capabilities,
                &a.inventory.entries,
            )
        })
        .chain(
            inputs
                .metadata
                .iter()
                .filter(|a| a.project.identity() == reading_canonical.project.id)
                .map(|a| {
                    crate::RepositoryAnalysisReading::stored(
                        a.identity,
                        a.repository_snapshot,
                        a.generated_at_unix_micros,
                        &a.freshness,
                        &a.capabilities,
                        &a.inventory.entries,
                    )
                }),
        )
        .max_by_key(|a| (a.generated_at_unix_micros, a.analysis_snapshot))
        .unwrap_or_else(|| {
            crate::RepositoryAnalysisReading::absent(
                inputs
                    .analysis_issues
                    .iter()
                    .any(|i| i.affected_scope == "derived_analysis"),
            )
        });
    let answer_capability_gaps = contextual_capability_gaps(&inputs, topology_canonical, &graph);
    let selected_entity = inputs.detail.entity.as_deref().and_then(|id| {
        graph
            .entities
            .iter()
            .find(|entity| {
                entity.identity == id
                    && (inputs.selection == crate::WorkSelector::Repository
                        || entity_matches_current_work(entity, topology_canonical)
                        || graph.relations.iter().any(|relation| {
                            relation_endpoints(relation).any(|end| end == id)
                                && graph.entities.iter().any(|seed| {
                                    entity_matches_current_work(seed, topology_canonical)
                                        && relation_endpoints(relation)
                                            .any(|end| end == seed.identity)
                                })
                        }))
            })
            .map(|entity| materialize_entity(entity))
    });
    let mut selected_entity_relations = Vec::new();
    let mut selected_entity_neighbors = Vec::new();
    let mut omitted_selected_relation_count = 0;
    if let Some(entity) = &selected_entity {
        let mut related = graph
            .relations
            .iter()
            .filter(|relation| {
                relation.source() == entity.identity
                    || relation.target() == Some(entity.identity.as_str())
            })
            .copied()
            .collect::<Vec<_>>();
        related.sort_by_key(|relation| relation.identity().to_owned());
        omitted_selected_relation_count = related.len().saturating_sub(limit);
        related.truncate(limit);
        let endpoints = related
            .iter()
            .flat_map(relation_endpoints)
            .collect::<BTreeSet<_>>();
        selected_entity_neighbors = graph
            .entities
            .iter()
            .filter(|e| endpoints.contains(e.identity.as_str()))
            .map(|e| materialize_entity(e))
            .collect();
        selected_entity_relations = related.into_iter().map(materialize_relation).collect();
    }
    let mut current_work_topology =
        build_current_work_topology(topology_canonical, &graph, limit, &mut issues);
    let repository_map = build_repository_map(
        reading_canonical,
        analyses,
        inputs.metadata,
        &graph,
        limit,
        &mut issues,
    );
    if inputs.selection == crate::WorkSelector::Repository {
        current_work_topology = CurrentWorkTopology {
            entities: repository_map.entities.clone(),
            relations: repository_map.relations.clone(),
            omitted_entity_count: graph
                .entities
                .len()
                .saturating_sub(repository_map.entities.len()),
            omitted_relation_count: graph
                .relations
                .len()
                .saturating_sub(repository_map.relations.len()),
        };
    }
    let current_work_code = build_current_work_code_links(
        topology_canonical,
        &current_work_topology.entities,
        limit,
        &mut issues,
    );
    let mut decision_context_code = build_decision_links(
        reading_canonical,
        &inputs.applicability,
        &current_work_topology.entities,
        &repository_map.entities,
        usize::MAX,
        &mut issues,
    );
    let checkpoint_timeline = build_timeline(reading_canonical, limit, &mut issues);
    let memory = if inputs.requirements.inspection {
        build_memory_inspection(
            reading_canonical,
            inputs.candidates,
            inputs.candidate_content_access,
            inputs.observed_at,
            inputs.bound,
        )
    } else {
        MemoryInspectionProjection {
            canonical_inspection: Vec::new(),
            candidate_inspection: Vec::new(),
            candidate_dependency: CandidateDependencyState::NotRequested,
            issues: Vec::new(),
        }
    };
    let canonical_inspection = memory.canonical_inspection;
    let candidate_inspection = memory.candidate_inspection;
    let candidate_dependency = memory.candidate_dependency;
    issues.extend(memory.issues);
    let mut source_catalog = reading_canonical.sources.clone();
    source_catalog.sort_by_key(|source| source.source.id);
    bound(&mut source_catalog, limit, "source_catalog", &mut issues);
    let source_status = source_status(reading_canonical);
    let mut current_goals = reading_canonical
        .context_items
        .iter()
        .filter(|item| item.role == ContextItemRole::Goal)
        .map(|item| (item.id, item.statement.clone()))
        .collect::<Vec<_>>();
    current_goals.sort_by_key(|(identity, _)| *identity);
    bound(
        &mut current_goals,
        limit,
        "project_overview.goal",
        &mut issues,
    );
    let mut unresolved_work_grouping =
        crate::reading::derive_unresolved_grouping(reading_canonical);
    bound(
        &mut unresolved_work_grouping,
        limit,
        "unresolved_work_grouping",
        &mut issues,
    );
    let work_index = crate::reading::WorkHistoryIndex::new(reading_canonical);
    let overview_selection = work_index.overview_selection(8);
    let work_count = work_index.0.len();
    let work_offset = inputs
        .detail
        .work_page
        .saturating_mul(limit)
        .min(work_count);
    let page_ids = work_index
        .0
        .keys()
        .skip(work_offset)
        .take(limit)
        .copied()
        .collect::<Vec<_>>();
    let resume_id = crate::WorkSelector::LatestWork
        .resolve(reading_canonical)?
        .work_item_id;
    let required_ids = page_ids
        .iter()
        .copied()
        .chain(
            overview_selection
                .iter()
                .flat_map(|(ids, _)| ids.iter().copied()),
        )
        .chain(selection.work_item_id)
        .chain(resume_id)
        .collect::<BTreeSet<_>>();
    let work_read_cost = WorkReadCost {
        classified_works: work_count,
        indexed_checkpoints: work_index.0.values().map(|g| g.checkpoints.len()).sum(),
        materialized_works: required_ids.len(),
        materialized_checkpoints: required_ids
            .iter()
            .filter_map(|id| work_index.0.get(id))
            .map(|g| g.checkpoints.len())
            .sum(),
        evidence_input_bytes: required_ids
            .iter()
            .filter_map(|id| work_index.0.get(id))
            .map(|g| {
                g.goal.statement.len()
                    + g.checkpoints
                        .iter()
                        .map(|cp| {
                            cp.state_change.as_deref().map_or(0, str::len) + cp.next_step.len()
                        })
                        .sum::<usize>()
            })
            .sum(),
    };
    let mut materialized = required_ids
        .iter()
        .filter_map(|id| {
            work_index
                .0
                .get(id)
                .map(|g| (*id, g.materialize(reading_canonical)))
        })
        .collect::<BTreeMap<_, _>>();
    resume.selected_work = resume_id.and_then(|id| materialized.get(&id).cloned());
    let mut selected_work = selection
        .work_item_id
        .and_then(|id| materialized.remove(&id));
    if let Some(work) = &mut selected_work {
        work.reading.code_freshness = inputs
            .analyses
            .iter()
            .filter(|a| a.project.identity() == reading_canonical.project.id)
            .map(|a| a.freshness.clone())
            .collect();
        work.reading.analysis_snapshot_basis = current_work_topology
            .entities
            .iter()
            .map(|e| e.analysis_snapshot)
            .chain(
                inputs
                    .analyses
                    .iter()
                    .filter(|a| a.project.identity() == reading_canonical.project.id)
                    .map(|a| a.identity),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        work.reading.repository_snapshot_basis = current_work_topology
            .entities
            .iter()
            .map(|e| e.repository_snapshot)
            .chain(
                inputs
                    .analyses
                    .iter()
                    .filter(|a| a.project.identity() == reading_canonical.project.id)
                    .map(|a| a.repository_snapshot),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        work.reading.code_source_basis = current_work_topology
            .entities
            .iter()
            .map(|e| e.source_id)
            .chain(current_work_topology.relations.iter().map(|r| r.source_id))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        work.reading.code_availability = if current_work_topology.entities.is_empty() {
            crate::ReadingAvailability::Unavailable
        } else if work
            .reading
            .code_freshness
            .iter()
            .any(|f| f.state != volicord_repository_intelligence::FreshnessState::Current)
        {
            crate::ReadingAvailability::Degraded
        } else {
            crate::ReadingAvailability::Available
        };
        work.reading.code_gap = if current_work_topology.entities.is_empty() {
            Some(if analyses.is_empty() {
                crate::WorkCodeGap::AnalysisUnavailable
            } else {
                crate::WorkCodeGap::NoMatchingCode
            })
        } else if work.reading.code_availability == crate::ReadingAvailability::Degraded {
            Some(crate::WorkCodeGap::AnalysisNotCurrent)
        } else {
            None
        };
    }
    if !inputs.requirements.code {
        if let Some(work) = &mut selected_work {
            work.reading.code_availability = crate::ReadingAvailability::NotRequested;
            work.reading.code_gap = None;
            work.reading.analysis_snapshot_basis = resume
                .snapshots
                .iter()
                .map(|s| s.analysis_snapshot)
                .collect();
            work.reading.repository_snapshot_basis = resume
                .snapshots
                .iter()
                .map(|s| s.repository_snapshot)
                .collect();
            work.reading.code_freshness = resume
                .snapshots
                .iter()
                .map(|s| s.freshness.clone())
                .collect();
        }
        for link in &mut decision_context_code {
            link.missing_or_uncertain_links.retain(|gap| {
                gap != "No snapshot-bound Code Entity matches the declared Decision scope"
            });
            link.missing_or_uncertain_links.push(
                "Code links were not requested; open Code Understanding for stored evidence".into(),
            );
        }
    }
    let selected_decision = inputs.detail.decision.and_then(|id| {
        inputs
            .canonical
            .active_decisions
            .iter()
            .chain(&inputs.canonical.superseded_decisions)
            .find(|lifecycle| lifecycle.decision.id == id)
            .map(|lifecycle| {
                let decision = crate::recall::brief_decision(
                    inputs.canonical,
                    lifecycle,
                    &inputs.applicability,
                );
                let link = decision_context_code
                    .iter()
                    .find(|link| link.decision_id == id);
                crate::understanding::decision_understanding(&decision, link)
            })
    });
    let selected_work_decisions = scoped.as_ref().map_or_else(Vec::new, |basis| {
        basis
            .active_decisions
            .iter()
            .chain(&basis.superseded_decisions)
            .map(|lifecycle| {
                let decision =
                    crate::recall::brief_decision(basis, lifecycle, &inputs.applicability);
                let link = decision_context_code
                    .iter()
                    .find(|link| link.decision_id == decision.decision_id);
                crate::understanding::decision_understanding(&decision, link)
            })
            .collect()
    });
    bound(
        &mut decision_context_code,
        limit,
        "decision_context_code",
        &mut issues,
    );
    if let Some(selected) = &selected_work {
        materialized.insert(selected.work_item_id, selected.clone());
    }
    let work_overview = crate::WorkOverview::from_selection(overview_selection, &materialized);
    let work_history = page_ids
        .into_iter()
        .filter_map(|id| materialized.remove(&id))
        .collect::<Vec<_>>();
    let omitted = work_count.saturating_sub(work_offset + work_history.len());
    if omitted > 0 {
        issues.push(bound_issue("work_history", omitted));
    }
    let mut decision_lifecycles = reading_canonical
        .active_decisions
        .iter()
        .chain(&reading_canonical.superseded_decisions)
        .collect::<Vec<_>>();
    decision_lifecycles.sort_by_key(|l| l.decision.id);
    let decision_count = decision_lifecycles.len();
    let decision_catalog = decision_lifecycles
        .into_iter()
        .skip(inputs.detail.decision_page.saturating_mul(limit))
        .take(limit)
        .map(|lifecycle| {
            let decision =
                crate::recall::brief_decision(reading_canonical, lifecycle, &inputs.applicability);
            crate::understanding::decision_understanding(
                &decision,
                decision_context_code
                    .iter()
                    .find(|l| l.decision_id == decision.decision_id),
            )
        })
        .collect();
    sort_projection_issues(&mut issues);
    let health = health_from_issues(&issues);
    let answer_sources = if let Some(work) = &selected_work {
        work.source_basis.clone()
    } else if let Some(decision) = &selected_decision {
        decision.decision.source_basis.clone()
    } else {
        reading_canonical
            .sources
            .iter()
            .map(|s| s.source.id)
            .collect()
    };
    let answer_issues = issues
        .iter()
        .filter(|issue| {
            issue.kind == ProjectionIssueKind::WrongProject
                || (issue.affected_scope == "derived_analysis" && inputs.requirements.code)
                || (issue.affected_scope == "canonical_source"
                    && answer_sources
                        .iter()
                        .any(|id| id.to_string() == issue.identity))
        })
        .cloned()
        .collect();
    let overview = ProjectOverview {
        project_id: reading_canonical.project.id,
        project_name: reading_canonical.project.display_name.clone(),
        canonical_revision: reading_canonical.project.revision,
        current_goals: current_goals
            .into_iter()
            .map(|(_, statement)| statement)
            .collect(),
        active_decision_count: reading_canonical.active_decisions.len(),
        superseded_decision_count: reading_canonical.superseded_decisions.len(),
        open_question_count: reading_canonical
            .active_questions
            .iter()
            .filter(|question| question.state == QuestionState::Open)
            .count(),
        latest_checkpoint_id: reading_canonical
            .latest_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.id),
        source_status,
        capability_reports: repository_map.capabilities.clone(),
        health,
    };
    Ok(ProjectProjection {
        repository_analysis,
        answer_capability_gaps,
        answer_issues,
        canonical_read_fingerprint: crate::canonical_read_fingerprint(inputs.canonical),
        sections: ProjectReadSections {
            code: if !inputs.requirements.code {
                ReadSectionState::NotRequested
            } else if analyses.is_empty() {
                ReadSectionState::Unavailable
            } else {
                ReadSectionState::Available
            },
            inspection: if !inputs.requirements.inspection {
                ReadSectionState::NotRequested
            } else if candidate_dependency == CandidateDependencyState::Available {
                ReadSectionState::Available
            } else {
                ReadSectionState::Unavailable
            },
        },
        work_count,
        work_read_cost,
        decision_count,
        decision_catalog,
        selected_decision,
        selected_entity,
        selected_entity_relations,
        selected_entity_neighbors,
        omitted_selected_relation_count,
        selection,
        selected_work,
        selected_work_decisions,
        work_overview,
        work_history,
        unresolved_work_grouping,
        overview,
        resume,
        repository_map,
        current_work_topology,
        current_work_code,
        decision_context_code,
        checkpoint_timeline,
        canonical_inspection,
        candidate_inspection,
        candidate_dependency,
        source_catalog,
        issues,
        health,
    })
}

fn candidate_dependency_issue(failure: CandidateDependencyFailure) -> ProjectionIssue {
    let kind = match failure.kind {
        CandidateDependencyFailureKind::Unavailable => ProjectionIssueKind::CandidateUnavailable,
        CandidateDependencyFailureKind::Unsupported => ProjectionIssueKind::CandidateUnsupported,
        CandidateDependencyFailureKind::Corrupt => ProjectionIssueKind::CandidateCorrupt,
        CandidateDependencyFailureKind::RepairRequired => {
            ProjectionIssueKind::CandidateRepairRequired
        }
        CandidateDependencyFailureKind::Failed => ProjectionIssueKind::CandidateFailed,
    };
    ProjectionIssue {
        kind,
        identity: format!(
            "candidate_dependency:{}",
            candidate_dependency_failure_key(failure.kind)
        ),
        affected_scope: failure.affected_scope,
        reason: failure.reason,
        omitted_count: 0,
    }
}

const fn candidate_dependency_failure_state(
    kind: CandidateDependencyFailureKind,
) -> CandidateDependencyState {
    match kind {
        CandidateDependencyFailureKind::Unavailable => CandidateDependencyState::Unavailable,
        CandidateDependencyFailureKind::Unsupported => CandidateDependencyState::Unsupported,
        CandidateDependencyFailureKind::Corrupt => CandidateDependencyState::Corrupt,
        CandidateDependencyFailureKind::RepairRequired => CandidateDependencyState::RepairRequired,
        CandidateDependencyFailureKind::Failed => CandidateDependencyState::Failed,
    }
}

const fn candidate_dependency_failure_key(kind: CandidateDependencyFailureKind) -> &'static str {
    match kind {
        CandidateDependencyFailureKind::Unavailable => "unavailable",
        CandidateDependencyFailureKind::Unsupported => "unsupported",
        CandidateDependencyFailureKind::Corrupt => "corrupt",
        CandidateDependencyFailureKind::RepairRequired => "repair_required",
        CandidateDependencyFailureKind::Failed => "failed",
    }
}

#[cfg(test)]
thread_local! {
    static MATERIALIZED: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}

fn materialize_entity(entity: &volicord_repository_intelligence::CodeEntity) -> MapEntity {
    #[cfg(test)]
    MATERIALIZED.with(|count| {
        let (entities, relations) = count.get();
        count.set((entities + 1, relations));
    });
    MapEntity {
        identity: entity.identity.clone(),
        display_name: entity
            .qualified_name
            .clone()
            .or_else(|| entity.display_name.clone())
            .unwrap_or_else(|| entity.identity.clone()),
        locator: entity.area.path.clone(),
        kind: entity.kind.clone(),
        language: entity.language.clone(),
        source_id: entity.source.identity(),
        source_range: entity.source_range.clone(),
        analysis_snapshot: entity.analysis_snapshot,
        repository_snapshot: entity.repository_snapshot,
        freshness: entity.freshness.clone(),
        uncertainty: entity.uncertainty.clone(),
        canonical_links: entity.canonical_links.clone(),
    }
}

fn materialize_relation(reference: RelationRef<'_>) -> MapRelation {
    #[cfg(test)]
    MATERIALIZED.with(|count| {
        let (entities, relations) = count.get();
        count.set((entities, relations + 1));
    });
    match reference {
        RelationRef::Structural(relation, fallback) => MapRelation {
            identity: relation.identity.clone(),
            class: MapRelationClass::StructuralFact,
            kind: format!("{:?}", relation.kind),
            source_entity: relation.source_entity.clone(),
            target_entity: resolved_target(&relation.target),
            unresolved_target: unresolved_target(&relation.target),
            source_id: relation
                .supporting_range
                .as_ref()
                .map_or(fallback, |range| range.source.identity()),
            supporting_range: relation.supporting_range.clone(),
            analysis_snapshot: relation.analysis_snapshot,
            repository_snapshot: relation.repository_snapshot,
            freshness: relation.freshness.clone(),
            uncertainty: relation.uncertainty.clone(),
            diagnostics: relation.diagnostics.clone(),
        },
        RelationRef::Semantic(relation, source_id) => MapRelation {
            identity: relation.identity.clone(),
            class: MapRelationClass::SemanticResult,
            kind: format!("{:?}", relation.kind),
            source_entity: relation.source_entity.clone(),
            target_entity: resolved_target(&relation.target),
            unresolved_target: unresolved_target(&relation.target),
            source_id,
            supporting_range: relation.supporting_range.clone(),
            analysis_snapshot: relation.analysis_snapshot,
            repository_snapshot: relation.repository_snapshot,
            freshness: relation.freshness.clone(),
            uncertainty: relation.uncertainty.clone(),
            diagnostics: relation.diagnostics.clone(),
        },
    }
}

fn build_current_work_topology(
    canonical: &CanonicalReadBasis,
    graph: &ProjectionGraph<'_>,
    limit: usize,
    issues: &mut Vec<ProjectionIssue>,
) -> CurrentWorkTopology {
    let grounded_seeds = graph
        .entities
        .iter()
        .filter(|entity| entity_matches_current_work(entity, canonical))
        .map(|entity| entity.identity.clone())
        .collect::<BTreeSet<_>>();
    let selected = select_grounded_current_work_topology(
        &graph.entities,
        &graph.relations,
        &grounded_seeds,
        limit,
        limit,
    );
    if selected.omitted_entity_count > 0 {
        issues.push(bound_issue(
            "current_work_topology.entity",
            selected.omitted_entity_count,
        ));
    }
    if selected.omitted_relation_count > 0 {
        issues.push(bound_issue(
            "current_work_topology.relation",
            selected.omitted_relation_count,
        ));
    }
    CurrentWorkTopology {
        omitted_entity_count: selected.omitted_entity_count,
        omitted_relation_count: selected.omitted_relation_count,
        entities: selected
            .entities
            .into_iter()
            .map(materialize_entity)
            .collect(),
        relations: selected
            .relations
            .into_iter()
            .map(materialize_relation)
            .collect(),
    }
}

fn build_repository_map(
    canonical: &CanonicalReadBasis,
    analyses: &[&AnalysisSnapshot],
    metadata: &[&AnalysisMetadata],
    graph: &ProjectionGraph<'_>,
    limit: usize,
    issues: &mut Vec<ProjectionIssue>,
) -> RepositoryMap {
    let mut entities = graph.entities.clone();
    let mut relations = graph.relations.clone();
    let mut agent_interpretations = Vec::new();
    let mut capabilities = Vec::new();
    let mut gaps = Vec::new();
    for analysis in analyses {
        if analysis.project.identity() != canonical.project.id {
            issues.push(ProjectionIssue {
                kind: ProjectionIssueKind::WrongProject,
                identity: analysis.identity.to_string(),
                affected_scope: "analysis_snapshot".to_owned(),
                reason: "Analysis Snapshot belongs to another Project".to_owned(),
                omitted_count: 0,
            });
            continue;
        }
        capabilities.extend(analysis.capabilities.iter().cloned());
        for report in &analysis.capabilities {
            if report.state != CapabilityState::Available {
                gaps.push(capability_gap(analysis.identity, report));
                issues.push(capability_issue(analysis.identity, report));
            }
        }
        agent_interpretations.extend(analysis.agent_interpretations.iter().map(|interpretation| {
            MapInterpretation {
                identity: interpretation.identity.clone(),
                text: interpretation.text.clone(),
                source_basis: interpretation
                    .source_basis
                    .iter()
                    .map(|source| source.identity())
                    .collect(),
                analysis_snapshot: interpretation.analysis_snapshot,
                repository_snapshot: analysis.repository_snapshot,
                known_gaps: interpretation.known_gaps.clone(),
                uncertainty: interpretation.uncertainty.clone(),
            }
        }));
    }
    for analysis in metadata
        .iter()
        .filter(|a| a.project.identity() == canonical.project.id)
    {
        capabilities.extend(analysis.capabilities.iter().cloned());
        for report in &analysis.capabilities {
            if report.state != CapabilityState::Available {
                gaps.push(capability_gap(analysis.identity, report));
                issues.push(capability_issue(analysis.identity, report));
            }
        }
    }
    agent_interpretations.sort_by(|left, right| left.identity.cmp(&right.identity));
    agent_interpretations.dedup_by(|left, right| left.identity == right.identity);
    capabilities.sort_by(|left, right| {
        (
            left.repository_snapshot,
            &left.language,
            &left.area,
            left.capability,
        )
            .cmp(&(
                right.repository_snapshot,
                &right.language,
                &right.area,
                right.capability,
            ))
    });
    gaps.sort_by(|left, right| {
        (
            left.analysis_snapshot,
            left.capability,
            &left.language,
            &left.area,
        )
            .cmp(&(
                right.analysis_snapshot,
                right.capability,
                &right.language,
                &right.area,
            ))
    });
    if entities.len() > limit || relations.len() > limit {
        let current_work_entities = entities
            .iter()
            .filter(|entity| entity_matches_current_work(entity, canonical))
            .map(|entity| entity.identity.clone())
            .collect::<BTreeSet<_>>();
        let important_entities = if current_work_entities.is_empty() {
            entities
                .iter()
                .filter(|entity| !entity.canonical_links.is_empty())
                .map(|entity| entity.identity.clone())
                .collect::<BTreeSet<_>>()
        } else {
            current_work_entities
        };
        let topology = select_topology(
            &entities,
            &relations,
            &important_entities,
            limit,
            limit,
            true,
        );
        if topology.omitted_entity_count > 0 {
            issues.push(bound_issue(
                "repository_map.entity",
                topology.omitted_entity_count,
            ));
        }
        if topology.omitted_relation_count > 0 {
            issues.push(bound_issue(
                "repository_map.relation",
                topology.omitted_relation_count,
            ));
        }
        entities = topology.entities;
        relations = topology.relations;
    }
    bound(
        &mut agent_interpretations,
        limit,
        "repository_map.agent_interpretation",
        issues,
    );
    bound(
        &mut capabilities,
        limit,
        "repository_map.capability",
        issues,
    );
    bound(&mut gaps, limit, "repository_map.gap", issues);
    let health = if gaps.iter().any(|gap| {
        matches!(
            gap.state,
            CapabilityState::Failed | CapabilityState::Unavailable | CapabilityState::Stale
        )
    }) {
        ProjectionHealth::Degraded
    } else if gaps.is_empty() {
        ProjectionHealth::Complete
    } else {
        ProjectionHealth::Partial
    };
    RepositoryMap {
        entities: entities.into_iter().map(materialize_entity).collect(),
        relations: relations.into_iter().map(materialize_relation).collect(),
        agent_interpretations,
        capabilities,
        gaps,
        health,
    }
}

fn build_decision_links(
    canonical: &CanonicalReadBasis,
    applicability: &ApplicabilityQuery,
    current_work_entities: &[MapEntity],
    repository_entities: &[MapEntity],
    limit: usize,
    issues: &mut Vec<ProjectionIssue>,
) -> Vec<DecisionContextCodeLink> {
    let mut historical_entities = repository_entities.to_vec();
    historical_entities.extend_from_slice(current_work_entities);
    historical_entities.sort_by(|a, b| a.identity.cmp(&b.identity));
    historical_entities.dedup_by(|a, b| a.identity == b.identity);
    let mut values = canonical
        .active_decisions
        .iter()
        .map(|lifecycle| decision_link(canonical, lifecycle, current_work_entities, applicability))
        .chain(canonical.superseded_decisions.iter().map(|lifecycle| {
            decision_link(canonical, lifecycle, &historical_entities, applicability)
        }))
        .collect::<Vec<_>>();
    values.sort_by_key(|value| value.decision_id);
    bound(&mut values, limit, "decision_context_code", issues);
    values
}

fn build_current_work_code_links(
    canonical: &CanonicalReadBasis,
    entities: &[MapEntity],
    limit: usize,
    issues: &mut Vec<ProjectionIssue>,
) -> Vec<CurrentWorkCodeLink> {
    let checkpoints = current_work_checkpoints(canonical);
    let goals = canonical
        .context_items
        .iter()
        .filter(|context| context.role == ContextItemRole::Goal)
        .collect::<Vec<_>>();
    let mut links = entities
        .iter()
        .filter_map(|entity| {
            let locator = entity
                .source_range
                .as_ref()
                .map(|range| range.locator.as_str())
                .unwrap_or(entity.locator.as_str());
            let mut changed_paths = checkpoints
                .iter()
                .copied()
                .flat_map(|checkpoint| checkpoint.changed_paths.iter())
                .filter(|path| path_matches(path, locator))
                .cloned()
                .collect::<Vec<_>>();
            let mut checkpoint_basis = checkpoints
                .iter()
                .copied()
                .filter(|checkpoint| {
                    source_matches_code(canonical, &checkpoint.changed_source_basis, entity.source_id, locator) || checkpoint
                        .changed_paths
                        .iter()
                        .any(|path| path_matches(path, locator))
                        || entity.canonical_links.iter().any(|link| {
                            matches!(link, CanonicalReference::Checkpoint(reference) if reference.identity() == checkpoint.id)
                        })
                })
                .map(|checkpoint| checkpoint.id)
                .collect::<Vec<_>>();
            let mut goal_context_basis = goals
                .iter()
                .filter(|context| {
                    source_matches_code(canonical, &context.source_basis, entity.source_id, locator) || map_entity_matches_scope(
                        entity,
                        &context.applicability.paths,
                        &context.applicability.components,
                    ) || entity.canonical_links.iter().any(|link| {
                        matches!(link, CanonicalReference::ContextItem(reference) if reference.identity() == context.id)
                    })
                })
                .map(|context| context.id)
                .collect::<Vec<_>>();
            changed_paths.sort();
            changed_paths.dedup();
            checkpoint_basis.sort();
            checkpoint_basis.dedup();
            goal_context_basis.sort();
            goal_context_basis.dedup();
            (!changed_paths.is_empty()
                || !checkpoint_basis.is_empty()
                || !goal_context_basis.is_empty())
            .then(|| CurrentWorkCodeLink {
                changed_path_basis: checkpoints.iter().flat_map(|cp| cp.changed_paths.iter()
                    .filter(|path| path_matches(path, locator))
                    .map(|path| CurrentWorkPathBasis { checkpoint_id: cp.id, checkpoint_revision: cp.revision, path: path.clone() })).collect(),
                entity_identity: entity.identity.clone(),
                changed_paths,
                checkpoint_basis,
                goal_context_basis,
            })
        })
        .collect::<Vec<_>>();
    links.sort_by_key(|link| {
        (
            Reverse(!link.changed_paths.is_empty()),
            Reverse(!link.goal_context_basis.is_empty()),
            entities
                .iter()
                .find(|entity| entity.identity == link.entity_identity)
                .map_or(usize::MAX, |entity| entity_kind_rank(&entity.kind)),
            link.entity_identity.clone(),
        )
    });
    bound(&mut links, limit, "current_work_code", issues);
    links
}

fn map_entity_matches_scope(entity: &MapEntity, paths: &[String], components: &[String]) -> bool {
    let locator = entity
        .source_range
        .as_ref()
        .map(|range| range.locator.as_str())
        .unwrap_or(entity.locator.as_str());
    paths.iter().any(|path| path_matches(path, locator))
        || components
            .iter()
            .any(|component| locator.contains(component) || entity.display_name.contains(component))
}

fn decision_link(
    canonical: &CanonicalReadBasis,
    lifecycle: &DecisionLifecycle,
    entities: &[MapEntity],
    applicability: &ApplicabilityQuery,
) -> DecisionContextCodeLink {
    let decision = &lifecycle.decision;
    let mut related_context_items = canonical
        .context_items
        .iter()
        .filter(|context| {
            shares_any(
                &context.source_basis,
                &decision.displayed_recommendation.source_basis,
            ) || scope_intersects(&context.applicability.paths, &decision.applicability.paths)
                || scope_intersects(
                    &context.applicability.components,
                    &decision.applicability.components,
                )
                || decision.assumptions.contains(&context.statement)
        })
        .map(|context| context.id)
        .collect::<Vec<_>>();
    related_context_items.sort();
    related_context_items.dedup();
    let mut related_code_entities = Vec::new();
    let mut link_basis = Vec::new();
    for entity in entities {
        let locator = entity
            .source_range
            .as_ref()
            .map(|range| range.locator.as_str())
            .unwrap_or_default();
        if entity
            .canonical_links
            .iter()
            .any(|link| {
                matches!(link, CanonicalReference::Decision(reference) if reference.identity() == decision.id)
            })
        {
            related_code_entities.push(entity.identity.clone());
            link_basis.push(format!(
                "{} has an explicit canonical Decision reference",
                entity.identity
            ));
        } else if decision
            .applicability
            .paths
            .iter()
            .any(|path| path_matches(path, locator))
            || decision.applicability.components.iter().any(|component| {
                locator.contains(component) || entity.display_name.contains(component)
            })
        {
            related_code_entities.push(entity.identity.clone());
            link_basis.push(format!(
                "{} overlaps the Decision's declared path/component scope; this is not proof of implementation",
                entity.identity
            ));
        }
    }
    related_code_entities.sort();
    related_code_entities.dedup();
    link_basis.sort();
    link_basis.dedup();
    let mut supporting_sources = decision.displayed_recommendation.source_basis.clone();
    supporting_sources.push(decision.user_turn_source_id);
    supporting_sources.sort();
    supporting_sources.dedup();
    let mut missing_or_uncertain_links = Vec::new();
    if related_code_entities.is_empty() {
        missing_or_uncertain_links
            .push("No snapshot-bound Code Entity matches the declared Decision scope".to_owned());
    }
    if lifecycle.review_due.is_some() {
        missing_or_uncertain_links.push("Decision applicability requires review".to_owned());
    }
    if lifecycle.superseded_by.is_some() {
        missing_or_uncertain_links
            .push("Decision is superseded and retained as history".to_owned());
    }
    DecisionContextCodeLink {
        decision_id: decision.id,
        decision_revision: decision.revision,
        decision_state: crate::recall::brief_decision(canonical, lifecycle, applicability).state,
        declared_paths: decision.applicability.paths.clone(),
        declared_components: decision.applicability.components.clone(),
        declared_work_contexts: decision.applicability.work_contexts.clone(),
        assumption_context: decision.assumptions.clone(),
        related_context_items,
        related_code_entities,
        supporting_sources,
        link_basis,
        missing_or_uncertain_links,
    }
}

fn build_timeline(
    canonical: &CanonicalReadBasis,
    limit: usize,
    issues: &mut Vec<ProjectionIssue>,
) -> Vec<CheckpointTimelineEntry> {
    let mut checkpoints = canonical
        .checkpoint_history
        .iter()
        .chain(canonical.latest_checkpoint.iter())
        .collect::<Vec<_>>();
    checkpoints.sort_by_key(|checkpoint| (checkpoint.recorded_at, checkpoint.id));
    checkpoints.dedup_by_key(|cp| cp.id);
    let omitted = checkpoints.len().saturating_sub(limit);
    if omitted > 0 {
        issues.push(bound_issue("checkpoint_timeline", omitted));
    }
    checkpoints
        .into_iter()
        .skip(omitted)
        .map(|checkpoint| CheckpointTimelineEntry {
            work_state: checkpoint.work_state,
            verification: checkpoint.verification.clone(),
            user_review: checkpoint.user_review.clone(),
            user_acceptance: checkpoint.user_acceptance.clone(),
            checkpoint: checkpoint.clone(),
        })
        .collect()
}

fn build_canonical_inspection(
    canonical: &CanonicalReadBasis,
    limit: usize,
    issues: &mut Vec<ProjectionIssue>,
) -> Vec<CanonicalInspectionItem> {
    let mut values = vec![CanonicalInspectionItem {
        kind: CanonicalInspectionKind::Project,
        identity: canonical.project.id.to_string(),
        revision: canonical.project.revision,
        lifecycle_state: "current".to_owned(),
        statement_role: None,
        summary: canonical.project.display_name.clone(),
        source_basis: Vec::new(),
    }];
    values.extend(
        canonical
            .sources
            .iter()
            .map(|basis| CanonicalInspectionItem {
                kind: CanonicalInspectionKind::Source,
                identity: basis.source.id.to_string(),
                revision: revision_for(canonical, "source", &basis.source.id.to_string()),
                lifecycle_state: source_freshness_key(basis.freshness).to_owned(),
                statement_role: Some("source_basis".to_owned()),
                summary: source_summary(&basis.source.payload),
                source_basis: vec![basis.source.id],
            }),
    );
    values.extend(
        canonical
            .active_questions
            .iter()
            .chain(&canonical.terminal_question_history)
            .map(|question| CanonicalInspectionItem {
                kind: CanonicalInspectionKind::Question,
                identity: question.id.to_string(),
                revision: question.revision,
                lifecycle_state: question_state_key(question.state).to_owned(),
                statement_role: Some("material_question".to_owned()),
                summary: question.prompt_basis.clone(),
                source_basis: question.source_basis.clone(),
            }),
    );
    values.extend(
        canonical
            .active_decisions
            .iter()
            .chain(&canonical.superseded_decisions)
            .map(|lifecycle| CanonicalInspectionItem {
                kind: CanonicalInspectionKind::Decision,
                identity: lifecycle.decision.id.to_string(),
                revision: lifecycle.decision.revision,
                lifecycle_state: if lifecycle.superseded_by.is_some() {
                    "superseded".to_owned()
                } else if lifecycle.review_due.is_some() {
                    "review_due".to_owned()
                } else {
                    "active".to_owned()
                },
                statement_role: Some("user_judgment".to_owned()),
                summary: decision_choice_summary(&lifecycle.decision.choice),
                source_basis: {
                    let mut sources = lifecycle
                        .decision
                        .displayed_recommendation
                        .source_basis
                        .clone();
                    sources.push(lifecycle.decision.user_turn_source_id);
                    sources
                },
            }),
    );
    values.extend(
        canonical
            .context_items
            .iter()
            .map(|item| CanonicalInspectionItem {
                kind: CanonicalInspectionKind::ContextItem,
                identity: item.id.to_string(),
                revision: item.revision,
                lifecycle_state: "current".to_owned(),
                statement_role: Some(context_role_key(item.role).to_owned()),
                summary: item.statement.clone(),
                source_basis: item.source_basis.clone(),
            }),
    );
    let checkpoints = if canonical.checkpoint_history.is_empty() {
        canonical.latest_checkpoint.iter().collect::<Vec<_>>()
    } else {
        canonical.checkpoint_history.iter().collect::<Vec<_>>()
    };
    values.extend(
        checkpoints
            .into_iter()
            .map(|checkpoint| CanonicalInspectionItem {
                kind: CanonicalInspectionKind::Checkpoint,
                identity: checkpoint.id.to_string(),
                revision: checkpoint.revision,
                lifecycle_state: work_state_key(checkpoint.work_state).to_owned(),
                statement_role: Some("source_grounded_checkpoint".to_owned()),
                summary: checkpoint.goal.clone(),
                source_basis: checkpoint.source_basis.clone(),
            }),
    );
    values.sort_by(|left, right| {
        (inspection_priority(left.kind), &left.identity)
            .cmp(&(inspection_priority(right.kind), &right.identity))
    });
    bound(&mut values, limit, "canonical_inspection", issues);
    values
}

fn source_summary(payload: &SourcePayload) -> String {
    match payload {
        SourcePayload::RepositorySnapshot { revision } => {
            format!("Repository snapshot {revision}")
        }
        SourcePayload::RepositoryCommit { commit } => format!("Repository commit {commit}"),
        SourcePayload::File { locator, snapshot } => format!("File {locator} @ {snapshot}"),
        SourcePayload::Symbol { locator, snapshot } => format!("Symbol {locator} @ {snapshot}"),
        SourcePayload::CommandExecution {
            command_label,
            outcome,
            ..
        } => match outcome.exit_code {
            Some(exit_code) => format!("Command {command_label} (exit {exit_code})"),
            None => format!("Command {command_label} (no exit code)"),
        },
        SourcePayload::CurrentHostUserTurn { host, turn, .. } => {
            format!("User input on {host}: {turn}")
        }
        SourcePayload::Url { url } => format!("URL {url}"),
        SourcePayload::AdoptedArtifact { locator, revision } => {
            format!("Adopted artifact {locator} @ {revision}")
        }
    }
}

fn decision_choice_summary(choice: &DecisionChoice) -> String {
    match choice {
        DecisionChoice::Alternative { alternative_key } => {
            format!("Alternative: {alternative_key}")
        }
        DecisionChoice::Delegation { delegate_to } => format!("Delegated to: {delegate_to}"),
    }
}

const fn source_freshness_key(freshness: SourceFreshness) -> &'static str {
    match freshness {
        SourceFreshness::Current => "current",
        SourceFreshness::Stale => "stale",
        SourceFreshness::Unavailable => "unavailable",
        SourceFreshness::Unknown => "unknown",
    }
}

const fn question_state_key(state: QuestionState) -> &'static str {
    match state {
        QuestionState::Open => "open",
        QuestionState::Terminal(_) => "terminal",
    }
}

const fn context_role_key(role: ContextItemRole) -> &'static str {
    match role {
        ContextItemRole::ProjectPurpose => "project_purpose",
        ContextItemRole::Goal => "goal",
        ContextItemRole::Fact => "fact",
        ContextItemRole::Assumption => "assumption",
        ContextItemRole::Constraint => "constraint",
        ContextItemRole::Preference => "preference",
        ContextItemRole::Risk => "risk",
        ContextItemRole::Learning => "learning",
        ContextItemRole::KnownLimit => "known_limit",
    }
}

const fn work_state_key(state: WorkState) -> &'static str {
    match state {
        WorkState::InProgress => "in_progress",
        WorkState::Paused => "paused",
        WorkState::Completed => "completed",
        WorkState::Abandoned => "abandoned",
        WorkState::Superseded => "superseded",
    }
}

const fn inspection_health_key(health: crate::InspectionHealth) -> &'static str {
    match health {
        crate::InspectionHealth::Complete => "complete",
        crate::InspectionHealth::Partial => "partial",
        crate::InspectionHealth::Degraded => "degraded",
        crate::InspectionHealth::NotFound => "not_found",
    }
}

const fn capability_key(capability: Capability) -> &'static str {
    match capability {
        Capability::Inventory => "inventory",
        Capability::AgentAssisted => "agent_assisted",
        Capability::Structural => "structural",
        Capability::Semantic => "semantic",
        Capability::Ecosystem => "ecosystem",
    }
}

fn language_key(language: &Language) -> String {
    match language {
        Language::Java => "java".to_owned(),
        Language::Python => "python".to_owned(),
        Language::JavaScript => "javascript".to_owned(),
        Language::TypeScript => "typescript".to_owned(),
        Language::C => "c".to_owned(),
        Language::Cpp => "cpp".to_owned(),
        Language::Rust => "rust".to_owned(),
        Language::Markdown => "markdown".to_owned(),
        Language::Json => "json".to_owned(),
        Language::Yaml => "yaml".to_owned(),
        Language::Toml => "toml".to_owned(),
        Language::Xml => "xml".to_owned(),
        Language::Shell => "shell".to_owned(),
        Language::Go => "go".to_owned(),
        Language::OtherText(value) => format!("other:{value}"),
        Language::UnknownText => "unknown_text".to_owned(),
    }
}

fn source_status(canonical: &CanonicalReadBasis) -> SourceStatusSummary {
    canonical
        .sources
        .iter()
        .fold(SourceStatusSummary::default(), |mut summary, source| {
            match source.freshness {
                SourceFreshness::Current => summary.current += 1,
                SourceFreshness::Stale => summary.stale += 1,
                SourceFreshness::Unavailable => summary.unavailable += 1,
                SourceFreshness::Unknown => summary.unknown += 1,
            }
            summary
        })
}

fn source_issues(canonical: &CanonicalReadBasis) -> Vec<ProjectionIssue> {
    canonical
        .sources
        .iter()
        .filter_map(|source| match source.freshness {
            SourceFreshness::Current => None,
            SourceFreshness::Stale => {
                Some((source, ProjectionIssueKind::SourceStale, "Source is stale"))
            }
            SourceFreshness::Unavailable => Some((
                source,
                ProjectionIssueKind::SourceUnavailable,
                "Source is unavailable",
            )),
            SourceFreshness::Unknown => Some((
                source,
                ProjectionIssueKind::SourceUnavailable,
                "Source freshness is unknown",
            )),
        })
        .map(|(source, kind, reason)| ProjectionIssue {
            kind,
            identity: source.source.id.to_string(),
            affected_scope: "canonical_source".to_owned(),
            reason: reason.to_owned(),
            omitted_count: 0,
        })
        .collect()
}

/// Relevance requires an actual inventory/Source/entity intersection in the
/// selected answer scope. A language label alone is never an intersection.
fn contextual_capability_gaps(
    inputs: &ProjectProjectionInputs<'_>,
    canonical: &CanonicalReadBasis,
    graph: &ProjectionGraph<'_>,
) -> Vec<CapabilityGap> {
    let repository_scope = inputs.selection == crate::WorkSelector::Repository
        && inputs.detail.entity.is_none()
        && inputs.detail.decision.is_none();
    let mut paths = Vec::new();
    if let Some(id) = &inputs.detail.entity {
        paths.extend(
            graph
                .entities
                .iter()
                .filter(|e| e.identity == *id)
                .map(|e| e.area.path.clone()),
        );
    } else if let Some(id) = inputs.detail.decision {
        for d in canonical
            .active_decisions
            .iter()
            .chain(&canonical.superseded_decisions)
            .filter(|d| d.decision.id == id)
        {
            paths.extend(d.decision.applicability.paths.clone());
            paths.extend(
                graph
                    .entities
                    .iter()
                    .filter(|e| {
                        d.decision
                            .applicability
                            .components
                            .iter()
                            .any(|c| c == &e.area.path || e.display_name.as_ref() == Some(c))
                    })
                    .map(|e| e.area.path.clone()),
            );
        }
    } else if !repository_scope {
        for cp in current_work_checkpoints(canonical) {
            paths.extend(cp.changed_paths.clone());
        }
        for goal in canonical
            .context_items
            .iter()
            .filter(|c| c.role == ContextItemRole::Goal)
        {
            paths.extend(goal.applicability.paths.clone());
        }
        for source in &canonical.sources {
            if canonical
                .context_items
                .iter()
                .any(|c| c.source_basis.contains(&source.source.id))
                || current_work_checkpoints(canonical)
                    .iter()
                    .any(|c| c.changed_source_basis.contains(&source.source.id))
            {
                if let SourcePayload::File { locator, .. } | SourcePayload::Symbol { locator, .. } =
                    &source.source.payload
                {
                    paths.push(locator.clone());
                }
            }
        }
        // Coarse RepositorySnapshot Source equality cannot make every file
        // a code seed. Explicit canonical links and component identities can.
        paths.extend(
            graph
                .entities
                .iter()
                .filter(|e| {
                    e.canonical_links.iter().any(|link| match link {
                        CanonicalReference::ContextItem(r) => {
                            canonical.context_items.iter().any(|c| c.id == r.identity())
                        }
                        CanonicalReference::Checkpoint(r) => current_work_checkpoints(canonical)
                            .iter()
                            .any(|c| c.id == r.identity()),
                        _ => false,
                    }) || canonical.context_items.iter().any(|c| {
                        c.applicability.components.iter().any(|component| {
                            component == &e.area.path || e.display_name.as_ref() == Some(component)
                        })
                    })
                })
                .map(|e| e.area.path.clone()),
        );
    }
    let overlaps = |a: &str, b: &str| {
        a.is_empty()
            || a == "."
            || b.is_empty()
            || b == "."
            || path_matches(a, b)
            || path_matches(b, a)
    };
    let mut gaps = Vec::new();
    let bases = inputs
        .analyses
        .iter()
        .filter(|a| a.project.identity() == canonical.project.id)
        .map(|a| (a.identity, &a.capabilities, &a.inventory.entries))
        .chain(
            inputs
                .metadata
                .iter()
                .filter(|a| a.project.identity() == canonical.project.id)
                .map(|a| (a.identity, &a.capabilities, &a.inventory.entries)),
        );
    for (identity, capabilities, entries) in bases {
        for report in capabilities
            .iter()
            .filter(|r| r.state != CapabilityState::Available)
        {
            let affected = report
                .coverage
                .failed
                .iter()
                .chain(&report.coverage.unavailable)
                .chain(&report.coverage.unsupported)
                .chain(&report.coverage.stale)
                .collect::<Vec<_>>();
            let area_affected = |path: &str| {
                overlaps(&report.area.path, path)
                    && (affected.is_empty() || affected.iter().any(|a| overlaps(&a.path, path)))
            };
            let relevant = entries.iter().any(|entry| {
                entry.entry_kind == volicord_repository_intelligence::EntryKind::File
                    && !entry.classifications.iter().any(|c| {
                        matches!(c,
                        volicord_repository_intelligence::InventoryClassification::Ignored
                        | volicord_repository_intelligence::InventoryClassification::Generated
                        | volicord_repository_intelligence::InventoryClassification::Vendor
                        | volicord_repository_intelligence::InventoryClassification::Binary)
                    })
                    && report
                        .language
                        .as_ref()
                        .is_none_or(|l| entry.language.as_ref() == Some(l))
                    && area_affected(&entry.area.path)
                    && (repository_scope || paths.iter().any(|p| overlaps(p, &entry.area.path)))
            }) || (report.language.is_none()
                && report.capability == Capability::Inventory
                && (repository_scope || paths.iter().any(|p| area_affected(p))));
            if relevant {
                gaps.push(capability_gap(identity, report));
            }
        }
    }
    gaps.sort_by(|a, b| {
        (&a.analysis_snapshot, &a.area, &a.language, a.capability).cmp(&(
            &b.analysis_snapshot,
            &b.area,
            &b.language,
            b.capability,
        ))
    });
    gaps.dedup();
    gaps
}

fn capability_gap(
    analysis_snapshot: AnalysisSnapshotId,
    report: &CapabilityReport,
) -> CapabilityGap {
    CapabilityGap {
        analysis_snapshot,
        repository_snapshot: report.repository_snapshot,
        capability: report.capability,
        language: report.language.clone(),
        state: report.state,
        area: report.area.path.clone(),
        reason: report
            .reason
            .clone()
            .unwrap_or_else(|| "capability is not fully available".to_owned()),
        affected_areas: report
            .coverage
            .excluded
            .iter()
            .chain(&report.coverage.unsupported)
            .chain(&report.coverage.unavailable)
            .chain(&report.coverage.failed)
            .chain(&report.coverage.stale)
            .map(|area| area.path.clone())
            .collect(),
        usable_remainder: report.usable_remainder.clone(),
        user_visible_consequence: report.user_visible_consequence.clone(),
    }
}

fn capability_issue(
    analysis_snapshot: AnalysisSnapshotId,
    report: &CapabilityReport,
) -> ProjectionIssue {
    let kind = match report.state {
        CapabilityState::Available => ProjectionIssueKind::PartialCapability,
        CapabilityState::Partial => ProjectionIssueKind::PartialCapability,
        CapabilityState::Unavailable => ProjectionIssueKind::UnavailableCapability,
        CapabilityState::Unsupported => ProjectionIssueKind::UnsupportedCapability,
        CapabilityState::Failed => ProjectionIssueKind::FailedCapability,
        CapabilityState::Stale => ProjectionIssueKind::StaleCapability,
    };
    ProjectionIssue {
        kind,
        identity: analysis_snapshot.to_string(),
        affected_scope: format!(
            "{}:{}:{}",
            report.area.path,
            report
                .language
                .as_ref()
                .map_or("all_languages".to_owned(), language_key),
            capability_key(report.capability)
        ),
        reason: report
            .reason
            .clone()
            .unwrap_or_else(|| "capability is not fully available".to_owned()),
        omitted_count: 0,
    }
}

fn resolved_target(target: &RelationTarget) -> Option<String> {
    match target {
        RelationTarget::ResolvedEntity(identity) => Some(identity.clone()),
        RelationTarget::Unresolved(_) => None,
    }
}

fn unresolved_target(target: &RelationTarget) -> Option<String> {
    match target {
        RelationTarget::ResolvedEntity(_) => None,
        RelationTarget::Unresolved(target) => {
            Some(format!("{}: {}", target.display, target.reason))
        }
    }
}

fn shares_any(left: &[SourceId], right: &[SourceId]) -> bool {
    left.iter().any(|value| right.contains(value))
}

fn scope_intersects(left: &[String], right: &[String]) -> bool {
    left.iter().any(|value| right.contains(value))
}

fn path_matches(scope: &str, locator: &str) -> bool {
    locator == scope
        || locator
            .strip_prefix(scope)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn source_matches_code(
    canonical: &CanonicalReadBasis,
    ids: &[SourceId],
    entity_source: SourceId,
    locator: &str,
) -> bool {
    ids.iter().any(|id| {
        *id == entity_source
            || canonical.sources.iter().any(|source| {
                source.source.id == *id
                    && match &source.source.payload {
                        SourcePayload::File { locator: path, .. }
                        | SourcePayload::Symbol { locator: path, .. } => {
                            path_matches(path, locator)
                        }
                        _ => false,
                    }
            })
    })
}

fn entity_matches_current_work(
    entity: &volicord_repository_intelligence::CodeEntity,
    canonical: &CanonicalReadBasis,
) -> bool {
    let locator = entity
        .source_range
        .as_ref()
        .map(|range| range.locator.as_str())
        .unwrap_or(entity.area.path.as_str());
    let display_name = entity
        .qualified_name
        .as_deref()
        .or(entity.display_name.as_deref())
        .unwrap_or(entity.identity.as_str());
    let matches_scope = |paths: &[String], components: &[String]| {
        paths.iter().any(|path| path_matches(path, locator))
            || components
                .iter()
                .any(|component| locator.contains(component) || display_name.contains(component))
    };

    let source_matches =
        |ids: &[SourceId]| source_matches_code(canonical, ids, entity.source.identity(), locator);
    if current_work_checkpoints(canonical).iter().any(|checkpoint| {
        source_matches(&checkpoint.changed_source_basis) ||
        checkpoint
            .changed_paths
            .iter()
            .any(|path| path_matches(path, locator))
            || entity.canonical_links.iter().any(|link| {
                matches!(link, CanonicalReference::Checkpoint(reference) if reference.identity() == checkpoint.id)
            })
    }) {
        return true;
    }
    if canonical.active_decisions.iter().any(|lifecycle| {
        matches_scope(
            &lifecycle.decision.applicability.paths,
            &lifecycle.decision.applicability.components,
        ) || entity.canonical_links.iter().any(|link| {
            matches!(link, CanonicalReference::Decision(reference) if reference.identity() == lifecycle.decision.id)
        })
    }) {
        return true;
    }
    canonical
        .context_items
        .iter()
        .filter(|context| context.role == ContextItemRole::Goal)
        .any(|context| {
            source_matches(&context.source_basis) || matches_scope(&context.applicability.paths, &context.applicability.components)
                || entity.canonical_links.iter().any(|link| {
                    matches!(link, CanonicalReference::ContextItem(reference) if reference.identity() == context.id)
                })
        })
}

/// Returns the Checkpoint basis for the latest Work Item without allowing a
/// later verification/handoff record to erase meaningful paths recorded by an
/// earlier Checkpoint in that same stable work history.
fn current_work_checkpoints(canonical: &CanonicalReadBasis) -> Vec<&Checkpoint> {
    let Some(latest) = canonical.latest_checkpoint.as_ref() else {
        return Vec::new();
    };
    let Some(work_item_id) = latest.work_item_id else {
        return vec![latest];
    };
    let mut checkpoints = canonical
        .checkpoint_history
        .iter()
        .filter(|checkpoint| checkpoint.work_item_id == Some(work_item_id))
        .collect::<Vec<_>>();
    if !checkpoints
        .iter()
        .any(|checkpoint| checkpoint.id == latest.id)
    {
        checkpoints.push(latest);
    }
    checkpoints.sort_by_key(|checkpoint| (checkpoint.recorded_at, checkpoint.id));
    checkpoints
}

fn revision_for(canonical: &CanonicalReadBasis, kind: &str, identity: &str) -> u64 {
    canonical
        .revisions
        .iter()
        .find(|revision| {
            canonical_kind_name(revision.record_kind) == kind
                && revision.record_identity == identity
        })
        .and_then(|revision| revision.revisions.last().copied())
        .unwrap_or(1)
}

fn bound<T>(values: &mut Vec<T>, limit: usize, scope: &str, issues: &mut Vec<ProjectionIssue>) {
    if values.len() <= limit {
        return;
    }
    issues.push(bound_issue(scope, values.len() - limit));
    values.truncate(limit);
}

fn bound_issue(scope: &str, omitted_count: usize) -> ProjectionIssue {
    ProjectionIssue {
        kind: ProjectionIssueKind::Bound,
        identity: format!("bound:{scope}"),
        affected_scope: scope.to_owned(),
        reason: format!("{omitted_count} items omitted by deterministic projection bound"),
        omitted_count,
    }
}

fn health_from_issues(issues: &[ProjectionIssue]) -> ProjectionHealth {
    if issues.iter().any(|issue| {
        matches!(
            issue.kind,
            ProjectionIssueKind::WrongProject
                | ProjectionIssueKind::UnavailableCapability
                | ProjectionIssueKind::FailedCapability
                | ProjectionIssueKind::StaleCapability
                | ProjectionIssueKind::SourceUnavailable
                | ProjectionIssueKind::CandidateInspection
                | ProjectionIssueKind::CandidateUnavailable
                | ProjectionIssueKind::CandidateUnsupported
                | ProjectionIssueKind::CandidateCorrupt
                | ProjectionIssueKind::CandidateRepairRequired
                | ProjectionIssueKind::CandidateFailed
        )
    }) {
        ProjectionHealth::Degraded
    } else if issues.is_empty() {
        ProjectionHealth::Complete
    } else {
        ProjectionHealth::Partial
    }
}

const fn inspection_priority(kind: CanonicalInspectionKind) -> u8 {
    match kind {
        CanonicalInspectionKind::Project => 0,
        CanonicalInspectionKind::Source => 1,
        CanonicalInspectionKind::Question => 2,
        CanonicalInspectionKind::Decision => 3,
        CanonicalInspectionKind::ContextItem => 4,
        CanonicalInspectionKind::Checkpoint => 5,
    }
}

const fn canonical_kind_name(kind: volicord_context::CanonicalRecordKind) -> &'static str {
    match kind {
        volicord_context::CanonicalRecordKind::Project => "project",
        volicord_context::CanonicalRecordKind::Source => "source",
        volicord_context::CanonicalRecordKind::Question => "question",
        volicord_context::CanonicalRecordKind::Decision => "decision",
        volicord_context::CanonicalRecordKind::ContextItem => "context_item",
        volicord_context::CanonicalRecordKind::Checkpoint => "checkpoint",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bound, health_from_issues, select_bounded_topology, MapEntity, MapRelation,
        MapRelationClass, ProjectionHealth, ProjectionIssueKind,
    };
    use std::collections::BTreeSet;
    use volicord_context::SourceId;
    use volicord_repository_intelligence::{
        AnalysisSnapshotId, CodeEntityKind, FreshnessBasis, FreshnessState, Language,
        RepositorySnapshotId, Uncertainty,
    };

    #[test]
    fn graph_payload_materialization_is_bounded_and_preserves_selected_provenance(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use volicord_context::{
            Availability, CanonicalReadOptions, OperationId, Principal, PrincipalKind, SourceDraft,
            SourcePayload, Store,
        };
        use volicord_repository_intelligence::{
            analyze_repository_semantics, CanonicalGrounding, InventoryRequest,
            SemanticAnalysisRequest, StructuralAnalysisRequest,
        };
        let home = tempfile::tempdir()?;
        let root = home.path().join("repository");
        std::fs::create_dir(&root)?;
        let source = (0..128)
            .map(|n| format!("pub fn f{n}() {{ f{}(); }}\n", (n + 1) % 128))
            .collect::<String>();
        std::fs::write(root.join("graph.rs"), source)?;
        let mut store = Store::open(home.path().join("canonical.sqlite3"))?;
        let project = store
            .create_project(OperationId::from_bytes([1; 16]), "Topology bound")?
            .value;
        let source = store
            .record_source(
                OperationId::from_bytes([2; 16]),
                project.id,
                SourceDraft {
                    expected_project_revision: project.revision,
                    payload: SourcePayload::RepositorySnapshot {
                        revision: "fixture".into(),
                    },
                    actor: Principal {
                        kind: PrincipalKind::Repository,
                        identity: "fixture".into(),
                    },
                    observer: None,
                    availability: Availability::Available,
                },
            )?
            .value;
        let canonical = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
        let grounding = CanonicalGrounding::from_read_basis(&canonical)?;
        let (_, analysis) = analyze_repository_semantics(SemanticAnalysisRequest::new(
            StructuralAnalysisRequest::new(InventoryRequest::new(&root, &grounding, source.id, 1)?),
        ))?;
        let mut issues = Vec::new();
        let graph = super::projection_graph(&canonical, &[&analysis]);
        let full = super::build_repository_map(
            &canonical,
            &[&analysis],
            &[],
            &graph,
            usize::MAX,
            &mut issues,
        );
        let important = full
            .entities
            .iter()
            .filter(|e| !e.canonical_links.is_empty())
            .map(|e| e.identity.clone())
            .collect();
        let expected =
            select_bounded_topology(&full.entities, &full.relations, &important, 8, 8, true);
        super::MATERIALIZED.with(|count| count.set((0, 0)));
        issues.clear();
        let actual =
            super::build_repository_map(&canonical, &[&analysis], &[], &graph, 8, &mut issues);
        assert_eq!(actual.entities, expected.entities);
        assert_eq!(actual.relations, expected.relations);
        assert!(full.entities.len() > 100 && full.relations.len() > 100);
        super::MATERIALIZED
            .with(|count| assert_eq!(count.get(), (actual.entities.len(), actual.relations.len())));
        let retained: BTreeSet<_> = actual
            .entities
            .iter()
            .map(|e| e.identity.as_str())
            .collect();
        assert!(actual
            .relations
            .iter()
            .all(|r| retained.contains(r.source_entity.as_str())
                && r.target_entity
                    .as_deref()
                    .is_none_or(|id| retained.contains(id))));
        assert!(issues
            .iter()
            .any(|i| i.affected_scope == "repository_map.entity"
                && i.omitted_count == full.entities.len() - actual.entities.len()));
        assert!(issues
            .iter()
            .any(|i| i.affected_scope == "repository_map.relation"
                && i.omitted_count == full.relations.len() - actual.relations.len()));
        Ok(())
    }

    #[test]
    fn deterministic_bound_keeps_one_scoped_issue_as_cardinality_grows() {
        let limit = 8;
        for cardinality in [9, 100_008] {
            let mut values = (0..cardinality).collect::<Vec<_>>();
            let mut issues = Vec::new();

            bound(&mut values, limit, "repository_map.entity", &mut issues);

            assert_eq!(values.len(), limit);
            assert_eq!(issues.len(), 1);
            assert_eq!(issues[0].kind, ProjectionIssueKind::Bound);
            assert_eq!(issues[0].identity, "bound:repository_map.entity");
            assert_eq!(issues[0].affected_scope, "repository_map.entity");
            assert_eq!(issues[0].omitted_count, cardinality - limit);
            assert_eq!(health_from_issues(&issues), ProjectionHealth::Partial);
        }
    }

    #[test]
    fn bounded_topology_never_displaces_an_isolated_current_work_seed() {
        let mut entities = vec![map_entity("current-change".into())];
        entities.extend((0..12).map(|index| map_entity(format!("generic-{index:02}"))));
        let relations = (1..12)
            .map(|index| {
                map_relation(
                    format!("generic:hub-{index:02}"),
                    "generic-00".into(),
                    format!("generic-{index:02}"),
                )
            })
            .collect::<Vec<_>>();

        let selected = select_bounded_topology(
            &entities,
            &relations,
            &BTreeSet::from(["current-change".to_owned()]),
            4,
            4,
            false,
        );

        assert!(selected
            .entities
            .iter()
            .any(|entity| entity.identity == "current-change"));
    }

    #[test]
    fn bounded_topology_keeps_connected_relations_under_input_permutation() {
        let mut entities = (0..8)
            .map(|index| map_entity(format!("a{index:02}")))
            .chain((0..6).map(|index| map_entity(format!("z{index:02}"))))
            .collect::<Vec<_>>();
        let mut relations = (0..5)
            .map(|index| {
                map_relation(
                    format!("relation:{index:02}"),
                    format!("z{index:02}"),
                    format!("z{:02}", index + 1),
                )
            })
            .collect::<Vec<_>>();
        let important = BTreeSet::from(["z02".to_owned()]);

        let selected = select_bounded_topology(&entities, &relations, &important, 4, 4, false);
        entities.reverse();
        relations.rotate_left(2);
        let shuffled = select_bounded_topology(&entities, &relations, &important, 4, 4, false);

        assert_eq!(selected, shuffled);
        assert_eq!(selected.entities.len(), 4);
        assert!(!selected.relations.is_empty());
        assert_eq!(selected.omitted_entity_count, 10);
        assert_eq!(selected.omitted_relation_count, 2);
        let visible = selected
            .entities
            .iter()
            .map(|entity| entity.identity.as_str())
            .collect::<BTreeSet<_>>();
        assert!(selected.relations.iter().all(|relation| {
            visible.contains(relation.source_entity.as_str())
                && relation
                    .target_entity
                    .as_deref()
                    .is_some_and(|target| visible.contains(target))
        }));
        assert!(selected
            .relations
            .iter()
            .any(|relation| relation.source_entity == "z02"
                || relation.target_entity.as_deref() == Some("z02")));
        assert!(selected
            .entities
            .iter()
            .all(|entity| entity.identity.starts_with('z')));
    }

    #[test]
    fn resolved_topology_precedes_unresolved_relation_evidence() {
        let entities = vec![map_entity("z00".to_owned()), map_entity("z01".to_owned())];
        let mut unresolved = map_relation(
            "relation:00".to_owned(),
            "z00".to_owned(),
            "unused".to_owned(),
        );
        unresolved.target_entity = None;
        unresolved.unresolved_target = Some("external::unused".to_owned());
        let resolved = map_relation("relation:99".to_owned(), "z00".to_owned(), "z01".to_owned());

        let selected = select_bounded_topology(
            &entities,
            &[unresolved, resolved.clone()],
            &BTreeSet::new(),
            2,
            1,
            true,
        );

        assert_eq!(selected.relations, vec![resolved]);
    }

    fn map_entity(identity: String) -> MapEntity {
        let repository_snapshot = snapshot_id();
        MapEntity {
            display_name: identity.clone(),
            locator: format!("src/{identity}.rs"),
            identity,
            kind: CodeEntityKind::Module,
            language: Language::Rust,
            source_id: SourceId::from_bytes([1; 16]),
            source_range: None,
            analysis_snapshot: analysis_id(),
            repository_snapshot,
            freshness: FreshnessBasis {
                state: FreshnessState::Current,
                repository_snapshot,
                compared_repository_snapshot: None,
                reason: None,
            },
            uncertainty: Uncertainty::none(),
            canonical_links: Vec::new(),
        }
    }

    fn map_relation(identity: String, source_entity: String, target_entity: String) -> MapRelation {
        let repository_snapshot = snapshot_id();
        MapRelation {
            identity,
            class: MapRelationClass::StructuralFact,
            kind: "Imports".to_owned(),
            source_entity,
            target_entity: Some(target_entity),
            unresolved_target: None,
            source_id: SourceId::from_bytes([1; 16]),
            supporting_range: None,
            analysis_snapshot: analysis_id(),
            repository_snapshot,
            freshness: FreshnessBasis {
                state: FreshnessState::Current,
                repository_snapshot,
                compared_repository_snapshot: None,
                reason: None,
            },
            uncertainty: Uncertainty::none(),
            diagnostics: Vec::new(),
        }
    }

    fn snapshot_id() -> RepositorySnapshotId {
        RepositorySnapshotId::from_hex(&"11".repeat(32)).expect("snapshot identity")
    }

    fn analysis_id() -> AnalysisSnapshotId {
        AnalysisSnapshotId::from_hex(&"22".repeat(32)).expect("analysis identity")
    }
}
