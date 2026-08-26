use crate::{
    build_resume_brief, inspect_candidate, CandidateContentAccess, CandidateInspection,
    RecallBound, RecallInputs, ResumeBrief,
};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use volicord_context::{
    CanonicalReadBasis, Checkpoint, ContextItemRole, DecisionChoice, DecisionId, DecisionLifecycle,
    ProjectId, QuestionState, SourceFreshness, SourceId, SourcePayload, TimestampMicros, WorkState,
};
use volicord_inquiry::{ApplicabilityQuery, CandidateReadBasis};
use volicord_repository_intelligence::{
    AnalysisSnapshot, AnalysisSnapshotId, Capability, CapabilityReport, CapabilityState,
    CodeEntityKind, FreshnessBasis, Language, RelationTarget, RepositorySnapshotId, SourceRange,
    Uncertainty,
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

pub struct ProjectProjectionInputs<'a> {
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
    pub kind: CodeEntityKind,
    pub language: Language,
    pub source_id: SourceId,
    pub source_range: Option<SourceRange>,
    pub analysis_snapshot: AnalysisSnapshotId,
    pub repository_snapshot: RepositorySnapshotId,
    pub freshness: FreshnessBasis,
    pub uncertainty: Uncertainty,
    pub canonical_links: Vec<String>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BoundedTopology {
    pub entities: Vec<MapEntity>,
    pub relations: Vec<MapRelation>,
    pub omitted_entity_count: usize,
    pub omitted_relation_count: usize,
}

/// Selects relationships together with the endpoint entities required to
/// inspect them. The stable relevance order prefers canonical/Decision-linked
/// endpoints, useful dependency/flow relations, important component kinds,
/// and connected structure before identity tie-breaking.
pub(crate) fn select_bounded_topology(
    entities: &[MapEntity],
    relations: &[MapRelation],
    important_entities: &BTreeSet<String>,
    entity_limit: usize,
    relation_limit: usize,
    include_unresolved: bool,
) -> BoundedTopology {
    let entity_limit = entity_limit.max(1);
    let entity_by_id = entities
        .iter()
        .map(|entity| (entity.identity.as_str(), entity))
        .collect::<BTreeMap<_, _>>();
    let mut degree = BTreeMap::<&str, usize>::new();
    for relation in relations {
        let Some(target) = relation.target_entity.as_deref() else {
            continue;
        };
        if entity_by_id.contains_key(relation.source_entity.as_str())
            && entity_by_id.contains_key(target)
        {
            *degree.entry(relation.source_entity.as_str()).or_default() += 1;
            *degree.entry(target).or_default() += 1;
        }
    }

    let mut candidates = relations
        .iter()
        .filter(|relation| {
            entity_by_id.contains_key(relation.source_entity.as_str())
                && match relation.target_entity.as_deref() {
                    Some(target) => entity_by_id.contains_key(target),
                    None => include_unresolved && relation.unresolved_target.is_some(),
                }
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        relation_selection_key(left, &entity_by_id, important_entities, &degree).cmp(
            &relation_selection_key(right, &entity_by_id, important_entities, &degree),
        )
    });

    let mut selected_entities = BTreeSet::<String>::new();
    let mut selected_relations = Vec::<MapRelation>::new();
    let mut selected_relation_ids = BTreeSet::<String>::new();
    while selected_relations.len() < relation_limit {
        let connected = !selected_entities.is_empty();
        let next = candidates.iter().copied().find(|relation| {
            if selected_relation_ids.contains(&relation.identity) {
                return false;
            }
            let endpoints = relation_endpoints(relation);
            let new_endpoint_count = endpoints
                .iter()
                .filter(|identity| !selected_entities.contains(identity.as_str()))
                .count();
            if selected_entities.len() + new_endpoint_count > entity_limit {
                return false;
            }
            !connected
                || endpoints
                    .iter()
                    .any(|identity| selected_entities.contains(identity.as_str()))
        });
        let next = next.or_else(|| {
            candidates.iter().copied().find(|relation| {
                if selected_relation_ids.contains(&relation.identity) {
                    return false;
                }
                let endpoints = relation_endpoints(relation);
                let new_endpoint_count = endpoints
                    .iter()
                    .filter(|identity| !selected_entities.contains(identity.as_str()))
                    .count();
                selected_entities.len() + new_endpoint_count <= entity_limit
            })
        });
        let Some(relation) = next else {
            break;
        };
        selected_entities.extend(relation_endpoints(relation));
        selected_relation_ids.insert(relation.identity.clone());
        selected_relations.push(relation.clone());
    }

    let mut remaining_entities = entities.iter().collect::<Vec<_>>();
    remaining_entities.sort_by(|left, right| {
        entity_selection_key(left, important_entities, &degree).cmp(&entity_selection_key(
            right,
            important_entities,
            &degree,
        ))
    });
    for entity in remaining_entities {
        if selected_entities.len() == entity_limit {
            break;
        }
        selected_entities.insert(entity.identity.clone());
    }

    let mut selected_entities = selected_entities
        .into_iter()
        .filter_map(|identity| entity_by_id.get(identity.as_str()).copied().cloned())
        .collect::<Vec<_>>();
    selected_entities.sort_by(|left, right| left.identity.cmp(&right.identity));
    selected_relations.sort_by(|left, right| left.identity.cmp(&right.identity));
    BoundedTopology {
        omitted_entity_count: entities.len().saturating_sub(selected_entities.len()),
        omitted_relation_count: relations.len().saturating_sub(selected_relations.len()),
        entities: selected_entities,
        relations: selected_relations,
    }
}

fn relation_endpoints(relation: &MapRelation) -> Vec<String> {
    std::iter::once(relation.source_entity.clone())
        .chain(relation.target_entity.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn relation_selection_key<'a>(
    relation: &'a MapRelation,
    entities: &BTreeMap<&str, &MapEntity>,
    important_entities: &BTreeSet<String>,
    degree: &BTreeMap<&str, usize>,
) -> (usize, Reverse<usize>, usize, usize, Reverse<usize>, &'a str) {
    let endpoints =
        std::iter::once(relation.source_entity.as_str()).chain(relation.target_entity.as_deref());
    let important_endpoint_count = endpoints
        .clone()
        .filter(|identity| important_entities.contains(*identity))
        .count();
    let endpoint_kind_rank = endpoints
        .clone()
        .filter_map(|identity| entities.get(identity).copied())
        .map(|entity| entity_kind_rank(&entity.kind))
        .min()
        .unwrap_or(usize::MAX);
    let connection_degree = endpoints
        .map(|identity| degree.get(identity).copied().unwrap_or_default())
        .sum();
    (
        usize::from(relation.target_entity.is_none()),
        Reverse(important_endpoint_count),
        relation_kind_rank(relation),
        endpoint_kind_rank,
        Reverse(connection_degree),
        relation.identity.as_str(),
    )
}

fn entity_selection_key<'a>(
    entity: &'a MapEntity,
    important_entities: &BTreeSet<String>,
    degree: &BTreeMap<&str, usize>,
) -> (Reverse<bool>, Reverse<usize>, usize, &'a str) {
    (
        Reverse(important_entities.contains(&entity.identity)),
        Reverse(
            degree
                .get(entity.identity.as_str())
                .copied()
                .unwrap_or_default(),
        ),
        entity_kind_rank(&entity.kind),
        entity.identity.as_str(),
    )
}

fn relation_kind_rank(relation: &MapRelation) -> usize {
    match relation.kind.as_str() {
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
pub struct ProjectProjection {
    pub overview: ProjectOverview,
    pub resume: ResumeBrief,
    pub repository_map: RepositoryMap,
    pub decision_context_code: Vec<DecisionContextCodeLink>,
    pub checkpoint_timeline: Vec<CheckpointTimelineEntry>,
    pub canonical_inspection: Vec<CanonicalInspectionItem>,
    pub candidate_inspection: Vec<CandidateInspection>,
    pub candidate_dependency: CandidateDependencyState,
    pub source_catalog: Vec<volicord_context::SourceReadBasis>,
    pub issues: Vec<ProjectionIssue>,
    pub health: ProjectionHealth,
}

/// Builds viewer/host-ready read models from immutable subsystem bases. The
/// function owns no store, analyzer, Candidate lifecycle, or filesystem handle.
pub fn build_project_projection(inputs: ProjectProjectionInputs<'_>) -> ProjectProjection {
    let limit = inputs.bound.max_items_per_section.max(1);
    let resume = build_resume_brief(RecallInputs {
        canonical: inputs.canonical,
        analyses: inputs.analyses,
        scope: inputs.applicability.clone(),
        bound: RecallBound {
            max_items_per_section: limit,
        },
    });
    let mut issues = source_issues(inputs.canonical);
    let repository_map = build_repository_map(
        inputs.canonical.project.id,
        inputs.analyses,
        limit,
        &mut issues,
    );
    let decision_context_code = build_decision_links(
        inputs.canonical,
        &resume,
        &repository_map,
        limit,
        &mut issues,
    );
    let checkpoint_timeline = build_timeline(inputs.canonical, limit, &mut issues);
    let canonical_inspection = build_canonical_inspection(inputs.canonical, limit, &mut issues);
    let mut source_catalog = inputs.canonical.sources.clone();
    source_catalog.sort_by_key(|source| source.source.id);
    bound(&mut source_catalog, limit, "source_catalog", &mut issues);
    let (candidate_basis, candidate_dependency) = match inputs.candidates {
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
                let inspection = inspect_candidate(
                    basis,
                    identity,
                    inputs.candidate_content_access,
                    inputs.observed_at,
                );
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
    let source_status = source_status(inputs.canonical);
    let mut current_goals = inputs
        .canonical
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
    issues.sort_by(|left, right| {
        (&left.affected_scope, &left.identity, &left.reason).cmp(&(
            &right.affected_scope,
            &right.identity,
            &right.reason,
        ))
    });
    issues.dedup();
    let health = health_from_issues(&issues);
    let overview = ProjectOverview {
        project_id: inputs.canonical.project.id,
        project_name: inputs.canonical.project.display_name.clone(),
        canonical_revision: inputs.canonical.project.revision,
        current_goals: current_goals
            .into_iter()
            .map(|(_, statement)| statement)
            .collect(),
        active_decision_count: inputs.canonical.active_decisions.len(),
        superseded_decision_count: inputs.canonical.superseded_decisions.len(),
        open_question_count: inputs
            .canonical
            .active_questions
            .iter()
            .filter(|question| question.state == QuestionState::Open)
            .count(),
        latest_checkpoint_id: inputs
            .canonical
            .latest_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.id),
        source_status,
        capability_reports: repository_map.capabilities.clone(),
        health,
    };
    ProjectProjection {
        overview,
        resume,
        repository_map,
        decision_context_code,
        checkpoint_timeline,
        canonical_inspection,
        candidate_inspection,
        candidate_dependency,
        source_catalog,
        issues,
        health,
    }
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

fn build_repository_map(
    project_id: ProjectId,
    analyses: &[&AnalysisSnapshot],
    limit: usize,
    issues: &mut Vec<ProjectionIssue>,
) -> RepositoryMap {
    let mut entities = Vec::new();
    let mut relations = Vec::new();
    let mut agent_interpretations = Vec::new();
    let mut capabilities = Vec::new();
    let mut gaps = Vec::new();
    for analysis in analyses {
        if analysis.project.identity() != project_id {
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
        for fact in &analysis.structural_facts {
            entities.push(MapEntity {
                identity: fact.entity.identity.clone(),
                display_name: fact
                    .entity
                    .qualified_name
                    .clone()
                    .or_else(|| fact.entity.display_name.clone())
                    .unwrap_or_else(|| fact.entity.identity.clone()),
                kind: fact.entity.kind.clone(),
                language: fact.entity.language.clone(),
                source_id: fact.entity.source.identity(),
                source_range: fact.entity.source_range.clone(),
                analysis_snapshot: fact.entity.analysis_snapshot,
                repository_snapshot: fact.entity.repository_snapshot,
                freshness: fact.entity.freshness.clone(),
                uncertainty: fact.entity.uncertainty.clone(),
                canonical_links: fact
                    .entity
                    .canonical_links
                    .iter()
                    .map(|link| format!("{link:?}"))
                    .collect(),
            });
            relations.extend(fact.relations.iter().map(|relation| {
                MapRelation {
                    identity: relation.identity.clone(),
                    class: MapRelationClass::StructuralFact,
                    kind: format!("{:?}", relation.kind),
                    source_entity: relation.source_entity.clone(),
                    target_entity: resolved_target(&relation.target),
                    unresolved_target: unresolved_target(&relation.target),
                    source_id: relation
                        .supporting_range
                        .as_ref()
                        .map_or(fact.entity.source.identity(), |range| {
                            range.source.identity()
                        }),
                    supporting_range: relation.supporting_range.clone(),
                    analysis_snapshot: relation.analysis_snapshot,
                    repository_snapshot: relation.repository_snapshot,
                    freshness: relation.freshness.clone(),
                    uncertainty: relation.uncertainty.clone(),
                    diagnostics: relation.diagnostics.clone(),
                }
            }));
        }
        for result in &analysis.semantic_results {
            let source_id = result
                .relation
                .supporting_range
                .as_ref()
                .map_or(analysis.repository_source.identity(), |range| {
                    range.source.identity()
                });
            relations.push(MapRelation {
                identity: result.relation.identity.clone(),
                class: MapRelationClass::SemanticResult,
                kind: format!("{:?}", result.relation.kind),
                source_entity: result.relation.source_entity.clone(),
                target_entity: resolved_target(&result.relation.target),
                unresolved_target: unresolved_target(&result.relation.target),
                source_id,
                supporting_range: result.relation.supporting_range.clone(),
                analysis_snapshot: result.relation.analysis_snapshot,
                repository_snapshot: result.relation.repository_snapshot,
                freshness: result.relation.freshness.clone(),
                uncertainty: result.relation.uncertainty.clone(),
                diagnostics: result.relation.diagnostics.clone(),
            });
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
    entities.sort_by(|left, right| left.identity.cmp(&right.identity));
    entities.dedup_by(|left, right| left.identity == right.identity);
    relations.sort_by(|left, right| left.identity.cmp(&right.identity));
    relations.dedup_by(|left, right| left.identity == right.identity);
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
        let important_entities = entities
            .iter()
            .filter(|entity| !entity.canonical_links.is_empty())
            .map(|entity| entity.identity.clone())
            .collect::<BTreeSet<_>>();
        let topology = select_bounded_topology(
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
        entities,
        relations,
        agent_interpretations,
        capabilities,
        gaps,
        health,
    }
}

fn build_decision_links(
    canonical: &CanonicalReadBasis,
    resume: &ResumeBrief,
    repository_map: &RepositoryMap,
    limit: usize,
    issues: &mut Vec<ProjectionIssue>,
) -> Vec<DecisionContextCodeLink> {
    let brief_states = resume
        .decisions
        .iter()
        .map(|decision| (decision.decision_id, decision.state))
        .collect::<BTreeMap<_, _>>();
    let mut values = canonical
        .active_decisions
        .iter()
        .chain(&canonical.superseded_decisions)
        .map(|lifecycle| decision_link(canonical, lifecycle, repository_map, &brief_states))
        .collect::<Vec<_>>();
    values.sort_by_key(|value| value.decision_id);
    bound(&mut values, limit, "decision_context_code", issues);
    values
}

fn decision_link(
    canonical: &CanonicalReadBasis,
    lifecycle: &DecisionLifecycle,
    repository_map: &RepositoryMap,
    brief_states: &BTreeMap<DecisionId, crate::BriefDecisionState>,
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
    for entity in &repository_map.entities {
        let locator = entity
            .source_range
            .as_ref()
            .map(|range| range.locator.as_str())
            .unwrap_or_default();
        let decision_marker = decision.id.to_string();
        if entity
            .canonical_links
            .iter()
            .any(|link| link.contains(&decision_marker))
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
        decision_state: brief_states
            .get(&decision.id)
            .copied()
            .unwrap_or(crate::BriefDecisionState::ReviewRequired),
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
    let mut checkpoints = if canonical.checkpoint_history.is_empty() {
        canonical.latest_checkpoint.iter().cloned().collect()
    } else {
        canonical.checkpoint_history.clone()
    };
    checkpoints.sort_by_key(|checkpoint| (checkpoint.recorded_at, checkpoint.id));
    let mut values = checkpoints
        .into_iter()
        .map(|checkpoint| CheckpointTimelineEntry {
            work_state: checkpoint.work_state,
            verification: checkpoint.verification.clone(),
            user_review: checkpoint.user_review.clone(),
            user_acceptance: checkpoint.user_acceptance.clone(),
            checkpoint,
        })
        .collect::<Vec<_>>();
    bound(&mut values, limit, "checkpoint_timeline", issues);
    values
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
