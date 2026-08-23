use crate::{
    project::select_bounded_topology, BriefContextItem, BriefDecision, BriefQuestion,
    BriefSnapshot, CapabilityGap, CheckpointTimelineEntry, DecisionContextCodeLink, MapEntity,
    MapInterpretation, MapRelation, ProjectProjection, ProjectionHealth, ProjectionIssue,
    SourceStatusSummary,
};
use volicord_context::{
    CheckpointId, DecisionId, ProjectId, SourceId, SourceReadBasis, VerificationFact, WorkState,
};
use volicord_repository_intelligence::{AnalysisSnapshotId, CodeEntityKind, RepositorySnapshotId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnderstandingBound {
    pub max_items_per_section: usize,
}

impl Default for UnderstandingBound {
    fn default() -> Self {
        Self {
            max_items_per_section: 24,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnderstandingWork {
    pub checkpoint_id: CheckpointId,
    pub goal: String,
    pub state: WorkState,
    pub meaningful_change: Option<String>,
    pub changed_paths: Vec<String>,
    pub verification: Vec<VerificationFact>,
    pub next_step: String,
    pub source_basis: Vec<SourceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnderstandingDecision {
    pub decision: BriefDecision,
    pub declared_paths: Vec<String>,
    pub declared_components: Vec<String>,
    pub declared_work_contexts: Vec<String>,
    pub affected_code_entities: Vec<String>,
    pub link_basis: Vec<String>,
    pub known_link_gaps: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnderstandingNextStep {
    pub identity: String,
    pub text: String,
    pub source_basis: Vec<SourceId>,
    pub decision_basis: Vec<DecisionId>,
    pub uncertainty: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnderstandingArchitecture {
    /// Snapshot-bound nodes copied from inspectable Repository Intelligence
    /// entities. Narrative realization cannot add nodes to this collection.
    pub components: Vec<MapEntity>,
    /// Snapshot-bound inspectable dependency/flow relations. Their identity,
    /// endpoints, fact class, freshness, and evidence remain available.
    pub relationships: Vec<MapRelation>,
    pub gaps: Vec<CapabilityGap>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnderstandingExplanationKind {
    Component,
    Relationship,
    Flow,
    DecisionImpact,
    Gap,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UnderstandingEvidenceClass {
    StructuralFact,
    SemanticResult,
    CanonicalDecision,
    CapabilityGap,
}

/// A fixed-locale deterministic explanation composed only from inspectable
/// canonical and Repository Intelligence facts. It is derived presentation,
/// not an observed fact or an optional model/agent interpretation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnderstandingExplanation {
    pub identity: String,
    pub kind: UnderstandingExplanationKind,
    pub english: String,
    pub korean: String,
    pub evidence_classes: Vec<UnderstandingEvidenceClass>,
    pub entity_basis: Vec<String>,
    pub relation_basis: Vec<String>,
    pub decision_basis: Vec<DecisionId>,
    pub source_basis: Vec<SourceId>,
    pub analysis_snapshot_basis: Vec<AnalysisSnapshotId>,
    pub repository_snapshot_basis: Vec<RepositorySnapshotId>,
    pub known_gaps: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnderstandingEvidence {
    pub sources: Vec<SourceReadBasis>,
    pub snapshots: Vec<BriefSnapshot>,
    /// Bounded unresolved Repository Intelligence relationships retained as
    /// inspectable explanation evidence. They are not architecture edges and
    /// no target entity is invented for them.
    pub unresolved_relationships: Vec<MapRelation>,
    pub source_status: SourceStatusSummary,
    pub issues: Vec<ProjectionIssue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnderstandingOmission {
    pub section: String,
    pub omitted_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUnderstanding {
    pub project_id: ProjectId,
    pub project_name: String,
    pub canonical_revision: u64,
    pub goals_and_why: Vec<BriefContextItem>,
    pub current_work: Option<UnderstandingWork>,
    pub completed_work: Vec<UnderstandingWork>,
    pub remaining_work: Vec<UnderstandingWork>,
    pub next_steps: Vec<UnderstandingNextStep>,
    pub active_decisions: Vec<UnderstandingDecision>,
    pub open_questions: Vec<BriefQuestion>,
    pub risks_assumptions_and_limits: Vec<BriefContextItem>,
    pub known_limits: Vec<String>,
    pub architecture: UnderstandingArchitecture,
    /// Deterministic fixed-locale explanation derived from the verified
    /// topology and canonical Decision applicability. Optional model/agent
    /// interpretation remains in `generated_interpretations`.
    pub deterministic_explanations: Vec<UnderstandingExplanation>,
    /// Model/agent interpretations are deliberately not merged into verified
    /// canonical, structural, or semantic facts.
    pub generated_interpretations: Vec<MapInterpretation>,
    pub evidence: UnderstandingEvidence,
    pub omissions: Vec<UnderstandingOmission>,
    pub health: ProjectionHealth,
}

/// Builds the human-oriented Project Understanding read model from an
/// immutable Project projection. This accepts no canonical, Candidate,
/// analyzer, publication, or provider mutation capability.
pub fn build_project_understanding(
    projection: &ProjectProjection,
    bound: UnderstandingBound,
) -> ProjectUnderstanding {
    let limit = bound.max_items_per_section.max(1);
    let mut omissions = Vec::new();

    let mut goals_and_why = projection.resume.goals_and_why.clone();
    bound_section(&mut goals_and_why, limit, "goals_and_why", &mut omissions);

    let mut timeline = projection.checkpoint_timeline.clone();
    timeline.sort_by_key(|entry| (entry.checkpoint.recorded_at, entry.checkpoint.id));
    let current_work = timeline.last().map(work_from_checkpoint);
    let mut completed_work = timeline
        .iter()
        .filter(|entry| entry.work_state == WorkState::Completed)
        .map(work_from_checkpoint)
        .collect::<Vec<_>>();
    completed_work.reverse();
    bound_section(&mut completed_work, limit, "completed_work", &mut omissions);
    let mut remaining_work = timeline
        .iter()
        .filter(|entry| matches!(entry.work_state, WorkState::InProgress | WorkState::Paused))
        .map(work_from_checkpoint)
        .collect::<Vec<_>>();
    remaining_work.reverse();
    bound_section(&mut remaining_work, limit, "remaining_work", &mut omissions);

    let links = projection
        .decision_context_code
        .iter()
        .map(|link| (link.decision_id, link))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut active_decisions = projection
        .resume
        .decisions
        .iter()
        .filter(|decision| decision.state != crate::BriefDecisionState::Superseded)
        .map(|decision| decision_understanding(decision, links.get(&decision.decision_id).copied()))
        .collect::<Vec<_>>();
    active_decisions.sort_by_key(|decision| decision.decision.decision_id);
    bound_section(
        &mut active_decisions,
        limit,
        "active_decisions",
        &mut omissions,
    );

    let mut open_questions = projection.resume.open_questions.clone();
    open_questions.sort_by_key(|question| (!question.on_current_frontier, question.question_id));
    bound_section(&mut open_questions, limit, "open_questions", &mut omissions);

    let mut next_steps = next_steps(projection);
    bound_section(&mut next_steps, limit, "next_steps", &mut omissions);

    let mut risks_assumptions_and_limits = projection.resume.risks_assumptions_and_limits.clone();
    bound_section(
        &mut risks_assumptions_and_limits,
        limit,
        "risks_assumptions_and_limits",
        &mut omissions,
    );
    let mut known_limits = projection.resume.known_limits.clone();
    known_limits.sort();
    known_limits.dedup();
    bound_section(&mut known_limits, limit, "known_limits", &mut omissions);

    let important_entities = projection
        .decision_context_code
        .iter()
        .filter(|link| link.decision_state != crate::BriefDecisionState::Superseded)
        .flat_map(|link| link.related_code_entities.iter().cloned())
        .chain(
            projection
                .repository_map
                .entities
                .iter()
                .filter(|entity| !entity.canonical_links.is_empty())
                .map(|entity| entity.identity.clone()),
        )
        .collect::<std::collections::BTreeSet<_>>();
    let topology = select_bounded_topology(
        &projection.repository_map.entities,
        &projection.repository_map.relations,
        &important_entities,
        limit,
        limit,
        false,
    );
    let all_entities = projection
        .repository_map
        .entities
        .iter()
        .map(|entity| entity.identity.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let resolved_relation_count = projection
        .repository_map
        .relations
        .iter()
        .filter(|relation| {
            all_entities.contains(relation.source_entity.as_str())
                && relation
                    .target_entity
                    .as_deref()
                    .is_some_and(|target| all_entities.contains(target))
        })
        .count();
    if topology.omitted_entity_count > 0 {
        omissions.push(UnderstandingOmission {
            section: "architecture.components".to_owned(),
            omitted_count: topology.omitted_entity_count,
        });
    }
    let omitted_resolved_relations =
        resolved_relation_count.saturating_sub(topology.relations.len());
    if omitted_resolved_relations > 0 {
        omissions.push(UnderstandingOmission {
            section: "architecture.relationships".to_owned(),
            omitted_count: omitted_resolved_relations,
        });
    }
    let components = topology.entities;
    let relationships = topology.relations;
    let visible_components = components
        .iter()
        .map(|entity| entity.identity.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let unresolved_relation_count = projection
        .repository_map
        .relations
        .iter()
        .filter(|relation| relation.target_entity.is_none() && relation.unresolved_target.is_some())
        .count();
    let mut unresolved_relationships = projection
        .repository_map
        .relations
        .iter()
        .filter(|relation| {
            relation.target_entity.is_none()
                && relation.unresolved_target.is_some()
                && visible_components.contains(relation.source_entity.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    unresolved_relationships.sort_by(|left, right| {
        (!is_flow_relation(left), &left.identity).cmp(&(!is_flow_relation(right), &right.identity))
    });
    unresolved_relationships.truncate(limit);
    let omitted_unresolved_relations =
        unresolved_relation_count.saturating_sub(unresolved_relationships.len());
    if omitted_unresolved_relations > 0 {
        omissions.push(UnderstandingOmission {
            section: "evidence.unresolved_relationships".to_owned(),
            omitted_count: omitted_unresolved_relations,
        });
    }
    let mut gaps = projection.repository_map.gaps.clone();
    bound_section(&mut gaps, limit, "architecture.gaps", &mut omissions);

    let explanation_relationships = relationships
        .iter()
        .chain(&unresolved_relationships)
        .cloned()
        .collect::<Vec<_>>();
    let deterministic_explanations = deterministic_explanations(
        &components,
        &explanation_relationships,
        &active_decisions,
        &gaps,
        limit,
        &mut omissions,
    );

    let mut generated_interpretations = projection.repository_map.agent_interpretations.clone();
    generated_interpretations.sort_by(|left, right| left.identity.cmp(&right.identity));
    bound_section(
        &mut generated_interpretations,
        limit,
        "generated_interpretations",
        &mut omissions,
    );

    let mut sources = projection.source_catalog.clone();
    sources.sort_by_key(|source| source.source.id);
    bound_section(&mut sources, limit, "evidence.sources", &mut omissions);
    let mut snapshots = projection.resume.snapshots.clone();
    snapshots.sort_by_key(|snapshot| snapshot.analysis_snapshot);
    bound_section(&mut snapshots, limit, "evidence.snapshots", &mut omissions);
    let mut issues = projection.issues.clone();
    bound_section(&mut issues, limit, "evidence.issues", &mut omissions);

    ProjectUnderstanding {
        project_id: projection.overview.project_id,
        project_name: projection.overview.project_name.clone(),
        canonical_revision: projection.overview.canonical_revision,
        goals_and_why,
        current_work,
        completed_work,
        remaining_work,
        next_steps,
        active_decisions,
        open_questions,
        risks_assumptions_and_limits,
        known_limits,
        architecture: UnderstandingArchitecture {
            components,
            relationships,
            gaps,
        },
        deterministic_explanations,
        generated_interpretations,
        evidence: UnderstandingEvidence {
            sources,
            snapshots,
            unresolved_relationships,
            source_status: projection.overview.source_status.clone(),
            issues,
        },
        omissions,
        health: projection.health,
    }
}

fn deterministic_explanations(
    components: &[MapEntity],
    relationships: &[MapRelation],
    decisions: &[UnderstandingDecision],
    gaps: &[CapabilityGap],
    limit: usize,
    omissions: &mut Vec<UnderstandingOmission>,
) -> Vec<UnderstandingExplanation> {
    let entities = components
        .iter()
        .map(|entity| (entity.identity.as_str(), entity))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut component_explanations = components
        .iter()
        .filter(|entity| is_explainable_component(entity, relationships))
        .map(|entity| component_explanation(entity, relationships, &entities))
        .collect::<Vec<_>>();
    let mut relationship_explanations = relationships
        .iter()
        .filter(|relation| is_explanatory_relation(relation))
        .filter_map(|relation| relation_explanation(relation, &entities))
        .collect::<Vec<_>>();
    let mut decision_explanations = decisions
        .iter()
        .map(|decision| decision_explanation(decision, &entities))
        .collect::<Vec<_>>();

    component_explanations.sort_by(|left, right| left.identity.cmp(&right.identity));
    relationship_explanations.sort_by(|left, right| {
        (
            left.kind != UnderstandingExplanationKind::Flow,
            &left.identity,
        )
            .cmp(&(
                right.kind != UnderstandingExplanationKind::Flow,
                &right.identity,
            ))
    });
    decision_explanations.sort_by(|left, right| left.identity.cmp(&right.identity));

    if !relationship_explanations
        .iter()
        .any(|explanation| explanation.kind == UnderstandingExplanationKind::Flow)
    {
        relationship_explanations.push(flow_gap_explanation(components, gaps));
    }

    let mut explanations = Vec::new();
    let mut groups = [
        component_explanations.into_iter(),
        relationship_explanations.into_iter(),
        decision_explanations.into_iter(),
    ];
    loop {
        let mut added = false;
        for group in &mut groups {
            if let Some(explanation) = group.next() {
                explanations.push(explanation);
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    bound_section(
        &mut explanations,
        limit,
        "deterministic_explanations",
        omissions,
    );
    explanations
}

fn is_explainable_component(entity: &MapEntity, relationships: &[MapRelation]) -> bool {
    matches!(
        entity.kind,
        CodeEntityKind::Repository
            | CodeEntityKind::Package
            | CodeEntityKind::Module
            | CodeEntityKind::Namespace
            | CodeEntityKind::File
            | CodeEntityKind::Class
            | CodeEntityKind::Interface
            | CodeEntityKind::Trait
            | CodeEntityKind::Struct
            | CodeEntityKind::Test
            | CodeEntityKind::Configuration
    ) || relationships
        .iter()
        .any(|relation| relation.source_entity == entity.identity && is_flow_relation(relation))
}

fn component_explanation(
    entity: &MapEntity,
    relationships: &[MapRelation],
    entities: &std::collections::BTreeMap<&str, &MapEntity>,
) -> UnderstandingExplanation {
    let mut supporting_relations = relationships
        .iter()
        .filter(|relation| {
            relation.source_entity == entity.identity
                || relation.target_entity.as_deref() == Some(entity.identity.as_str())
        })
        .collect::<Vec<_>>();
    supporting_relations.sort_by(|left, right| left.identity.cmp(&right.identity));
    let parent = supporting_relations.iter().find_map(|relation| {
        (matches!(relation.kind.as_str(), "Contains" | "Declares")
            && relation.target_entity.as_deref() == Some(entity.identity.as_str()))
        .then(|| entities.get(relation.source_entity.as_str()).copied())
        .flatten()
    });
    let mut children = supporting_relations
        .iter()
        .filter_map(|relation| {
            (matches!(relation.kind.as_str(), "Contains" | "Declares")
                && relation.source_entity == entity.identity)
                .then(|| {
                    relation
                        .target_entity
                        .as_deref()
                        .and_then(|id| entities.get(id).copied())
                })
                .flatten()
        })
        .map(|child| child.display_name.clone())
        .collect::<Vec<_>>();
    children.sort();
    children.dedup();
    children.truncate(3);
    let mut flows = supporting_relations
        .iter()
        .filter(|relation| relation.source_entity == entity.identity && is_flow_relation(relation))
        .filter_map(|relation| {
            relation
                .target_entity
                .as_deref()
                .and_then(|id| entities.get(id).copied())
                .map(|target| target.display_name.clone())
        })
        .collect::<Vec<_>>();
    flows.sort();
    flows.dedup();
    flows.truncate(3);

    let kind_en = entity_kind_label(&entity.kind, false);
    let kind_ko = entity_kind_label(&entity.kind, true);
    let (english, korean) = if !children.is_empty() {
        (
            format!(
                "`{}` is a source-grounded {} that contains or declares {}.",
                entity.display_name,
                kind_en,
                quoted_names(&children)
            ),
            format!(
                "`{}`은(는) {}(으)로 분석되며 {}을(를) 포함하거나 선언합니다.",
                entity.display_name,
                kind_ko,
                quoted_names(&children)
            ),
        )
    } else if !flows.is_empty() {
        (
            format!(
                "`{}` is a source-grounded {} connected by analyzed flow relations to {}.",
                entity.display_name,
                kind_en,
                quoted_names(&flows)
            ),
            format!(
                "`{}`은(는) {}이며 분석된 흐름 관계를 통해 {}와(과) 연결됩니다.",
                entity.display_name,
                kind_ko,
                quoted_names(&flows)
            ),
        )
    } else if let Some(parent) = parent {
        (
            format!(
                "`{}` is a source-grounded {} declared within `{}`; the available facts do not establish a more specific responsibility.",
                entity.display_name, kind_en, parent.display_name
            ),
            format!(
                "`{}`은(는) `{}` 안에 선언된 {}입니다. 사용 가능한 사실만으로는 더 구체적인 책임을 확정할 수 없습니다.",
                entity.display_name, parent.display_name, kind_ko
            ),
        )
    } else {
        (
            format!(
                "`{}` is verified as a source-grounded {}; no more specific responsibility is established by the visible relations.",
                entity.display_name, kind_en
            ),
            format!(
                "`{}`은(는) 근거가 있는 {}(으)로 확인됩니다. 표시된 관계만으로는 더 구체적인 책임을 확정할 수 없습니다.",
                entity.display_name, kind_ko
            ),
        )
    };
    explanation_from_entity_and_relations(
        format!("deterministic:component:{}", entity.identity),
        UnderstandingExplanationKind::Component,
        english,
        korean,
        entity,
        &supporting_relations,
    )
}

fn relation_explanation(
    relation: &MapRelation,
    entities: &std::collections::BTreeMap<&str, &MapEntity>,
) -> Option<UnderstandingExplanation> {
    let source = entities.get(relation.source_entity.as_str()).copied()?;
    let target = relation
        .target_entity
        .as_deref()
        .and_then(|identity| entities.get(identity).copied());
    let (english, korean) = if let Some(target) = target {
        relation_narrative(relation, source, target)
    } else {
        unresolved_relation_narrative(relation, source, relation.unresolved_target.as_deref()?)
    };
    let mut explanation = explanation_from_entity_and_relations(
        format!("deterministic:relation:{}", relation.identity),
        if is_flow_relation(relation) {
            UnderstandingExplanationKind::Flow
        } else {
            UnderstandingExplanationKind::Relationship
        },
        english,
        korean,
        source,
        &[relation],
    );
    if let Some(target) = target {
        explanation.entity_basis.push(target.identity.clone());
        explanation.source_basis.push(target.source_id);
    } else {
        explanation.known_gaps.push(format!(
            "relation target `{}` is unresolved and is not a Code Entity",
            relation.unresolved_target.as_deref().unwrap_or_default()
        ));
    }
    normalize_explanation_basis(&mut explanation);
    Some(explanation)
}

fn decision_explanation(
    decision: &UnderstandingDecision,
    entities: &std::collections::BTreeMap<&str, &MapEntity>,
) -> UnderstandingExplanation {
    let mut affected = decision
        .affected_code_entities
        .iter()
        .filter_map(|identity| entities.get(identity.as_str()).copied())
        .collect::<Vec<_>>();
    affected.sort_by(|left, right| left.identity.cmp(&right.identity));
    let affected_names = affected
        .iter()
        .take(4)
        .map(|entity| entity.display_name.clone())
        .collect::<Vec<_>>();
    let choice = match &decision.decision.choice {
        volicord_context::DecisionChoice::Alternative { alternative_key } => {
            format!("alternative `{alternative_key}`")
        }
        volicord_context::DecisionChoice::Delegation { delegate_to } => {
            format!("delegation to `{delegate_to}`")
        }
    };
    let korean_choice = match &decision.decision.choice {
        volicord_context::DecisionChoice::Alternative { alternative_key } => {
            format!("대안 `{alternative_key}`")
        }
        volicord_context::DecisionChoice::Delegation { delegate_to } => {
            format!("`{delegate_to}`에게 위임")
        }
    };
    let scope = declared_scope(decision, false);
    let korean_scope = declared_scope(decision, true);
    let (english, korean) = if affected_names.is_empty() {
        (
            format!(
                "The Decision for {choice} declares {scope}, but no snapshot-bound code entity currently matches that scope; no code effect is inferred."
            ),
            format!(
                "{korean_choice} 결정은 {korean_scope}을(를) 적용 범위로 선언하지만 현재 그 범위와 일치하는 snapshot 기반 코드 엔터티가 없습니다. 코드 영향을 추론하지 않습니다."
            ),
        )
    } else {
        (
            format!(
                "The Decision for {choice} declares {scope} and is connected to {} by an explicit reference or declared-scope overlap; overlap identifies affected-code candidates, not proof of implementation.",
                quoted_names(&affected_names)
            ),
            format!(
                "{korean_choice} 결정은 {korean_scope}을(를) 적용 범위로 선언하며 명시적 참조 또는 선언 범위 중첩으로 {}와(과) 연결됩니다. 범위 중첩은 영향받는 코드 후보이지 구현 완료의 증거가 아닙니다.",
                quoted_names(&affected_names)
            ),
        )
    };
    let mut explanation = UnderstandingExplanation {
        identity: format!(
            "deterministic:decision:{}@{}",
            decision.decision.decision_id, decision.decision.revision
        ),
        kind: UnderstandingExplanationKind::DecisionImpact,
        english,
        korean,
        evidence_classes: vec![UnderstandingEvidenceClass::CanonicalDecision],
        entity_basis: decision.affected_code_entities.clone(),
        relation_basis: Vec::new(),
        decision_basis: vec![decision.decision.decision_id],
        source_basis: decision.decision.source_basis.clone(),
        analysis_snapshot_basis: affected
            .iter()
            .map(|entity| entity.analysis_snapshot)
            .collect(),
        repository_snapshot_basis: affected
            .iter()
            .map(|entity| entity.repository_snapshot)
            .collect(),
        known_gaps: decision.known_link_gaps.clone(),
    };
    if !affected.is_empty() {
        explanation
            .evidence_classes
            .push(UnderstandingEvidenceClass::StructuralFact);
    }
    normalize_explanation_basis(&mut explanation);
    explanation
}

fn flow_gap_explanation(
    components: &[MapEntity],
    gaps: &[CapabilityGap],
) -> UnderstandingExplanation {
    let mut explanation = UnderstandingExplanation {
        identity: "deterministic:gap:visible-flow".to_owned(),
        kind: UnderstandingExplanationKind::Gap,
        english: "No resolved import, include, call, reference, or implementation flow is available among the displayed entities; no execution or data-flow path is inferred.".to_owned(),
        korean: "표시된 엔터티 사이에 확인된 import, include, call, reference 또는 implementation 흐름이 없습니다. 실행 경로나 데이터 흐름을 추론하지 않습니다.".to_owned(),
        evidence_classes: if gaps.is_empty() {
            vec![UnderstandingEvidenceClass::StructuralFact]
        } else {
            vec![UnderstandingEvidenceClass::CapabilityGap]
        },
        entity_basis: components.iter().map(|entity| entity.identity.clone()).collect(),
        relation_basis: Vec::new(),
        decision_basis: Vec::new(),
        source_basis: components.iter().map(|entity| entity.source_id).collect(),
        analysis_snapshot_basis: components
            .iter()
            .map(|entity| entity.analysis_snapshot)
            .collect(),
        repository_snapshot_basis: components
            .iter()
            .map(|entity| entity.repository_snapshot)
            .collect(),
        known_gaps: gaps.iter().map(|gap| gap.reason.clone()).collect(),
    };
    normalize_explanation_basis(&mut explanation);
    explanation
}

fn explanation_from_entity_and_relations(
    identity: String,
    kind: UnderstandingExplanationKind,
    english: String,
    korean: String,
    entity: &MapEntity,
    relations: &[&MapRelation],
) -> UnderstandingExplanation {
    let mut explanation = UnderstandingExplanation {
        identity,
        kind,
        english,
        korean,
        evidence_classes: vec![UnderstandingEvidenceClass::StructuralFact],
        entity_basis: std::iter::once(entity.identity.clone())
            .chain(
                relations
                    .iter()
                    .filter_map(|relation| relation.target_entity.clone()),
            )
            .collect(),
        relation_basis: relations
            .iter()
            .map(|relation| relation.identity.clone())
            .collect(),
        decision_basis: Vec::new(),
        source_basis: std::iter::once(entity.source_id)
            .chain(relations.iter().map(|relation| relation.source_id))
            .collect(),
        analysis_snapshot_basis: std::iter::once(entity.analysis_snapshot)
            .chain(relations.iter().map(|relation| relation.analysis_snapshot))
            .collect(),
        repository_snapshot_basis: std::iter::once(entity.repository_snapshot)
            .chain(
                relations
                    .iter()
                    .map(|relation| relation.repository_snapshot),
            )
            .collect(),
        known_gaps: entity
            .uncertainty
            .reasons
            .iter()
            .cloned()
            .chain(relations.iter().flat_map(|relation| {
                relation
                    .uncertainty
                    .reasons
                    .iter()
                    .cloned()
                    .chain(relation.diagnostics.iter().cloned())
            }))
            .collect(),
    };
    if relations
        .iter()
        .any(|relation| relation.class == crate::MapRelationClass::SemanticResult)
    {
        explanation
            .evidence_classes
            .push(UnderstandingEvidenceClass::SemanticResult);
    }
    normalize_explanation_basis(&mut explanation);
    explanation
}

fn normalize_explanation_basis(explanation: &mut UnderstandingExplanation) {
    explanation.evidence_classes.sort();
    explanation.evidence_classes.dedup();
    explanation.entity_basis.sort();
    explanation.entity_basis.dedup();
    explanation.relation_basis.sort();
    explanation.relation_basis.dedup();
    explanation.decision_basis.sort();
    explanation.decision_basis.dedup();
    explanation.source_basis.sort();
    explanation.source_basis.dedup();
    explanation.analysis_snapshot_basis.sort();
    explanation.analysis_snapshot_basis.dedup();
    explanation.repository_snapshot_basis.sort();
    explanation.repository_snapshot_basis.dedup();
    explanation.known_gaps.sort();
    explanation.known_gaps.dedup();
}

fn is_explanatory_relation(relation: &MapRelation) -> bool {
    (relation.target_entity.is_some() || relation.unresolved_target.is_some())
        && !matches!(relation.kind.as_str(), "Contains" | "Declares" | "Defines")
}

fn is_flow_relation(relation: &MapRelation) -> bool {
    matches!(
        relation.kind.as_str(),
        "Imports"
            | "Includes"
            | "CallsSyntactically"
            | "References"
            | "ResolvesTo"
            | "InstantiatedBy"
            | "Implements"
            | "Overrides"
    )
}

fn relation_narrative(
    relation: &MapRelation,
    source: &MapEntity,
    target: &MapEntity,
) -> (String, String) {
    let source_name = &source.display_name;
    let target_name = &target.display_name;
    match relation.kind.as_str() {
        "Imports" => (
            format!("`{source_name}` imports `{target_name}` through a parser-observed dependency edge."),
            format!("`{source_name}`은(는) parser가 확인한 의존 관계를 통해 `{target_name}`을(를) import합니다."),
        ),
        "Includes" => (
            format!("`{source_name}` includes `{target_name}` through a parser-observed source dependency."),
            format!("`{source_name}`은(는) parser가 확인한 source 의존 관계를 통해 `{target_name}`을(를) include합니다."),
        ),
        "CallsSyntactically" => (
            format!("`{source_name}` has a parser-observed call to `{target_name}`; this is source structure, not a guarantee of runtime execution."),
            format!("`{source_name}`에서 `{target_name}`을(를) 호출하는 구문이 parser로 확인되었습니다. 이는 source 구조이며 runtime 실행을 보장하지 않습니다."),
        ),
        "References" => (
            format!("Semantic analysis links a reference from `{source_name}` to `{target_name}` within this Analysis Snapshot."),
            format!("의미 분석은 현재 Analysis Snapshot에서 `{source_name}`의 reference를 `{target_name}`에 연결합니다."),
        ),
        "ResolvesTo" => (
            format!("Semantic analysis resolves `{source_name}` to `{target_name}` within the recorded build and source context."),
            format!("의미 분석은 기록된 build/source context에서 `{source_name}`을(를) `{target_name}`에 resolve합니다."),
        ),
        "Implements" => (
            format!("`{source_name}` implements `{target_name}` according to the recorded structural or semantic relation."),
            format!("기록된 구조 또는 의미 관계에 따르면 `{source_name}`은(는) `{target_name}`을(를) 구현합니다."),
        ),
        "Overrides" => (
            format!("`{source_name}` overrides `{target_name}` according to semantic analysis."),
            format!("의미 분석에 따르면 `{source_name}`은(는) `{target_name}`을(를) override합니다."),
        ),
        "Inherits" => (
            format!("`{source_name}` inherits from `{target_name}` according to the parser-observed structure."),
            format!("parser가 확인한 구조에 따르면 `{source_name}`은(는) `{target_name}`을(를) 상속합니다."),
        ),
        "Tests" => (
            format!("`{source_name}` is connected to `{target_name}` by a verified test relation."),
            format!("`{source_name}`은(는) 검증된 test 관계로 `{target_name}`와(과) 연결됩니다."),
        ),
        "Configures" => (
            format!("`{source_name}` configures `{target_name}` according to the parser-observed repository structure."),
            format!("parser가 확인한 저장소 구조에 따르면 `{source_name}`은(는) `{target_name}`을(를) 설정합니다."),
        ),
        "Exports" => (
            format!("`{source_name}` exports `{target_name}` according to the parser-observed module structure."),
            format!("parser가 확인한 module 구조에 따르면 `{source_name}`은(는) `{target_name}`을(를) export합니다."),
        ),
        other => (
            format!("`{source_name}` has the inspectable `{other}` relationship to `{target_name}`."),
            format!("`{source_name}`에서 `{target_name}`으로 검사 가능한 `{other}` 관계가 있습니다."),
        ),
    }
}

fn unresolved_relation_narrative(
    relation: &MapRelation,
    source: &MapEntity,
    target: &str,
) -> (String, String) {
    let source_name = &source.display_name;
    match relation.kind.as_str() {
        "Imports" => (
            format!("`{source_name}` has a parser-observed import of `{target}`, but the target is unresolved; no resolved component dependency is claimed."),
            format!("`{source_name}`에서 `{target}` import 구문이 parser로 확인되었지만 target은 resolve되지 않았습니다. 확인된 컴포넌트 의존 관계라고 주장하지 않습니다."),
        ),
        "Includes" => (
            format!("`{source_name}` has a parser-observed include of `{target}`, but the target is unresolved; no resolved source dependency is claimed."),
            format!("`{source_name}`에서 `{target}` include 구문이 parser로 확인되었지만 target은 resolve되지 않았습니다. 확인된 source 의존 관계라고 주장하지 않습니다."),
        ),
        "CallsSyntactically" => (
            format!("`{source_name}` contains a parser-observed call spelling `{target}`, but the target is unresolved; runtime execution and a resolved call edge are not claimed."),
            format!("`{source_name}`에서 `{target}` 호출 구문이 parser로 확인되었지만 target은 resolve되지 않았습니다. runtime 실행이나 resolve된 호출 edge라고 주장하지 않습니다."),
        ),
        "References" | "ResolvesTo" => (
            format!("Analysis records a `{}` relation from `{source_name}` toward `{target}`, but the target remains unresolved and no resolved flow edge is claimed.", relation.kind),
            format!("분석은 `{source_name}`에서 `{target}` 방향의 `{}` 관계를 기록하지만 target은 resolve되지 않았습니다. resolve된 flow edge라고 주장하지 않습니다.", relation.kind),
        ),
        other => (
            format!("`{source_name}` has a source-grounded `{other}` relation toward unresolved target spelling `{target}`; no target entity is invented."),
            format!("`{source_name}`에서 resolve되지 않은 target spelling `{target}` 방향의 근거 있는 `{other}` 관계가 확인됩니다. target 엔터티를 발명하지 않습니다."),
        ),
    }
}

fn declared_scope(decision: &UnderstandingDecision, korean: bool) -> String {
    let mut values = decision
        .declared_paths
        .iter()
        .chain(&decision.declared_components)
        .chain(&decision.declared_work_contexts)
        .cloned()
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    if values.is_empty() {
        if korean {
            "프로젝트 전체".to_owned()
        } else {
            "Project-wide applicability".to_owned()
        }
    } else if korean {
        format!("범위 {}", quoted_names(&values))
    } else {
        format!("scope {}", quoted_names(&values))
    }
}

fn quoted_names(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("`{value}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn entity_kind_label(kind: &CodeEntityKind, korean: bool) -> String {
    match (kind, korean) {
        (CodeEntityKind::Repository, false) => "repository".to_owned(),
        (CodeEntityKind::Repository, true) => "저장소".to_owned(),
        (CodeEntityKind::Package, false) => "package".to_owned(),
        (CodeEntityKind::Package, true) => "패키지".to_owned(),
        (CodeEntityKind::Module, false) => "module".to_owned(),
        (CodeEntityKind::Module, true) => "모듈".to_owned(),
        (CodeEntityKind::Namespace, false) => "namespace".to_owned(),
        (CodeEntityKind::Namespace, true) => "네임스페이스".to_owned(),
        (CodeEntityKind::File, false) => "file".to_owned(),
        (CodeEntityKind::File, true) => "파일".to_owned(),
        (CodeEntityKind::Class, false) => "class".to_owned(),
        (CodeEntityKind::Class, true) => "클래스".to_owned(),
        (CodeEntityKind::Interface, false) => "interface".to_owned(),
        (CodeEntityKind::Interface, true) => "인터페이스".to_owned(),
        (CodeEntityKind::Trait, _) => "trait".to_owned(),
        (CodeEntityKind::Struct, _) => "struct".to_owned(),
        (CodeEntityKind::Enum, _) => "enum".to_owned(),
        (CodeEntityKind::Type, false) => "type".to_owned(),
        (CodeEntityKind::Type, true) => "타입".to_owned(),
        (CodeEntityKind::Function, false) => "function".to_owned(),
        (CodeEntityKind::Function, true) => "함수".to_owned(),
        (CodeEntityKind::Method, false) => "method".to_owned(),
        (CodeEntityKind::Method, true) => "메서드".to_owned(),
        (CodeEntityKind::Field, false) => "field".to_owned(),
        (CodeEntityKind::Field, true) => "필드".to_owned(),
        (CodeEntityKind::Test, false) => "test".to_owned(),
        (CodeEntityKind::Test, true) => "테스트".to_owned(),
        (CodeEntityKind::Configuration, false) => "configuration".to_owned(),
        (CodeEntityKind::Configuration, true) => "설정".to_owned(),
        (CodeEntityKind::Document, false) => "document".to_owned(),
        (CodeEntityKind::Document, true) => "문서".to_owned(),
        (CodeEntityKind::LanguageSpecific(value), _) => value.clone(),
    }
}

fn work_from_checkpoint(entry: &CheckpointTimelineEntry) -> UnderstandingWork {
    UnderstandingWork {
        checkpoint_id: entry.checkpoint.id,
        goal: entry.checkpoint.goal.clone(),
        state: entry.work_state,
        meaningful_change: entry.checkpoint.state_change.clone(),
        changed_paths: entry.checkpoint.changed_paths.clone(),
        verification: entry.verification.clone(),
        next_step: entry.checkpoint.next_step.clone(),
        source_basis: entry.checkpoint.source_basis.clone(),
    }
}

fn decision_understanding(
    decision: &BriefDecision,
    link: Option<&DecisionContextCodeLink>,
) -> UnderstandingDecision {
    UnderstandingDecision {
        decision: decision.clone(),
        declared_paths: link.map_or_else(Vec::new, |value| value.declared_paths.clone()),
        declared_components: link.map_or_else(Vec::new, |value| value.declared_components.clone()),
        declared_work_contexts: link
            .map_or_else(Vec::new, |value| value.declared_work_contexts.clone()),
        affected_code_entities: link
            .map_or_else(Vec::new, |value| value.related_code_entities.clone()),
        link_basis: link.map_or_else(Vec::new, |value| value.link_basis.clone()),
        known_link_gaps: link
            .map_or_else(Vec::new, |value| value.missing_or_uncertain_links.clone()),
    }
}

fn next_steps(projection: &ProjectProjection) -> Vec<UnderstandingNextStep> {
    let mut steps = Vec::new();
    if let Some(checkpoint) = projection.resume.latest_meaningful_checkpoint.as_ref() {
        steps.push(UnderstandingNextStep {
            identity: format!("checkpoint:{}", checkpoint.id),
            text: checkpoint.next_step.clone(),
            source_basis: checkpoint.source_basis.clone(),
            decision_basis: checkpoint.applied_decisions.clone(),
            uncertainty: checkpoint.known_limits.clone(),
        });
    }
    for question in &projection.resume.open_questions {
        for (index, unlocked) in question.what_the_answer_unlocks.iter().enumerate() {
            steps.push(UnderstandingNextStep {
                identity: format!("question:{}:{index}", question.question_id),
                text: unlocked.clone(),
                source_basis: question.source_basis.clone(),
                decision_basis: Vec::new(),
                uncertainty: question.blocked_basis.clone(),
            });
        }
    }
    steps.sort_by(|left, right| left.identity.cmp(&right.identity));
    steps.dedup_by(|left, right| left.identity == right.identity);
    steps
}

fn bound_section<T>(
    values: &mut Vec<T>,
    limit: usize,
    section: &str,
    omissions: &mut Vec<UnderstandingOmission>,
) {
    if values.len() > limit {
        omissions.push(UnderstandingOmission {
            section: section.to_owned(),
            omitted_count: values.len() - limit,
        });
        values.truncate(limit);
    }
}
