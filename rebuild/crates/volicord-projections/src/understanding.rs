use crate::{
    project::BoundedTopology, BriefContextItem, BriefDecision, BriefQuestion, BriefSnapshot,
    CapabilityGap, DecisionContextCodeLink, MapEntity, MapInterpretation, MapRelation,
    ProjectProjection, ProjectionHealth, ProjectionIssue, SourceStatusSummary,
};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use volicord_context::{
    CheckpointId, ContextItemId, DecisionId, DecisionWorkScope, ProjectId, QuestionId, SourceId,
    SourceReadBasis, WorkState,
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
    pub observed_at: volicord_context::TimestampMicros,
    pub reading: crate::WorkReading,
    pub work_item_id: ContextItemId,
    pub title: String,
    pub state: UnderstandingWorkState,
    pub checkpoint_ids: Vec<CheckpointId>,
    pub decision_ids: Vec<DecisionId>,
    pub changed_paths: Vec<String>,
    pub changed_components: Vec<String>,
    pub next_step: Option<String>,
    pub open_question_ids: Vec<QuestionId>,
    pub source_basis: Vec<SourceId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnderstandingWorkState {
    Open,
    InProgress,
    Paused,
    Completed,
    Abandoned,
    Superseded,
}

impl From<WorkState> for UnderstandingWorkState {
    fn from(value: WorkState) -> Self {
        match value {
            WorkState::InProgress => Self::InProgress,
            WorkState::Paused => Self::Paused,
            WorkState::Completed => Self::Completed,
            WorkState::Abandoned => Self::Abandoned,
            WorkState::Superseded => Self::Superseded,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnresolvedWorkGrouping {
    pub record_kind: &'static str,
    pub identity: String,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnderstandingDecision {
    pub reading: crate::DecisionReading,
    pub decision: BriefDecision,
    pub declared_paths: Vec<String>,
    pub declared_components: Vec<String>,
    pub declared_work_contexts: Vec<String>,
    pub affected_code_entities: Vec<String>,
    pub link_basis: Vec<String>,
    pub known_link_gaps: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnderstandingArchitecture {
    pub flow_evidence: ArchitectureFlowEvidence,
    /// Snapshot-bound nodes copied from inspectable Repository Intelligence
    /// entities. Narrative realization cannot add nodes to this collection.
    pub components: Vec<MapEntity>,
    /// Snapshot-bound inspectable dependency/flow relations. Their identity,
    /// endpoints, fact class, freshness, and evidence remain available.
    pub relationships: Vec<MapRelation>,
    /// Bounded, inspectable reasons that tie each displayed component to the
    /// current Goal, latest meaningful Checkpoint, active Decision, or one
    /// grounded relation hop from one of those seeds.
    pub selection_basis: Vec<UnderstandingArchitectureSelection>,
    pub gaps: Vec<CapabilityGap>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchitectureFlowState {
    NotRequested,
    AnalysisUnavailable,
    NoResolvedCalls,
    SyntacticCalls,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchitectureFlowEvidence {
    pub state: ArchitectureFlowState,
    pub relation_ids: Vec<String>,
    pub missing_evidence: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UnderstandingArchitectureSelectionBasis {
    ChangedPath {
        checkpoint_id: CheckpointId,
        path: String,
    },
    DecisionCodeLink {
        decision_id: DecisionId,
    },
    GoalContextLink {
        context_item_id: ContextItemId,
    },
    CheckpointLink {
        checkpoint_id: CheckpointId,
    },
    GroundedOneHop {
        relation_id: String,
        seed_entity: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnderstandingArchitectureSelection {
    pub entity_identity: String,
    pub basis: Vec<UnderstandingArchitectureSelectionBasis>,
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

/// The canonical reader supplies complete history; catalog paging has no role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkSection {
    pub items: Vec<UnderstandingWork>,
    pub total: usize,
    pub omitted: usize,
    pub complete: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkOverview {
    pub current: WorkSection,
    pub completed: WorkSection,
    pub remaining: WorkSection,
    pub next_steps: WorkSection,
}
impl WorkOverview {
    pub(crate) fn from_selection(
        selections: crate::reading::WorkOverviewSelection,
        materialized: &BTreeMap<ContextItemId, UnderstandingWork>,
    ) -> Self {
        let [current, completed, remaining, next_steps] = selections.map(|(ids, total)| {
            let items = ids
                .into_iter()
                .filter_map(|id| materialized.get(&id).cloned())
                .collect::<Vec<_>>();
            WorkSection {
                omitted: total - items.len(),
                items,
                total,
                complete: true,
            }
        });
        Self {
            current,
            completed,
            remaining,
            next_steps,
        }
    }
    fn bound(&mut self, limit: usize) {
        for section in [
            &mut self.current,
            &mut self.completed,
            &mut self.remaining,
            &mut self.next_steps,
        ] {
            section.items.truncate(limit);
            section.omitted = section.total - section.items.len();
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUnderstanding {
    pub repository_analysis: crate::RepositoryAnalysisReading,
    pub work_overview: WorkOverview,
    pub selection: crate::WorkSelection,
    pub selected_work: Option<UnderstandingWork>,
    pub selected_work_decisions: Vec<UnderstandingDecision>,
    pub project_id: ProjectId,
    pub project_name: String,
    pub canonical_revision: u64,
    pub project_purpose: Vec<BriefContextItem>,
    pub current_work: Vec<UnderstandingWork>,
    pub completed_work: Vec<UnderstandingWork>,
    pub remaining_work: Vec<UnderstandingWork>,
    pub work_history: Vec<UnderstandingWork>,
    pub unresolved_work_grouping: Vec<UnresolvedWorkGrouping>,
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

    let mut project_purpose = projection.resume.project_purpose.clone();
    bound_section(
        &mut project_purpose,
        limit,
        "project_purpose",
        &mut omissions,
    );

    let mut timeline = projection.checkpoint_timeline.clone();
    timeline.sort_by_key(|entry| (entry.checkpoint.recorded_at, entry.checkpoint.id));
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

    let mut work_overview = projection.work_overview.clone();
    work_overview.bound(limit);
    let current_work = work_overview.current.items.clone();
    let completed_work = work_overview.completed.items.clone();
    let remaining_work = work_overview.remaining.items.clone();
    for (name, section) in [
        ("current_work", &work_overview.current),
        ("completed_work", &work_overview.completed),
        ("remaining_work", &work_overview.remaining),
        ("work_next_steps", &work_overview.next_steps),
    ] {
        if section.omitted > 0 {
            omissions.push(UnderstandingOmission {
                section: name.into(),
                omitted_count: section.omitted,
            });
        }
    }
    let mut work_history = projection.work_history.clone();
    bound_section(&mut work_history, limit, "work_history", &mut omissions);
    let mut unresolved_work_grouping = projection.unresolved_work_grouping.clone();
    bound_section(
        &mut unresolved_work_grouping,
        limit,
        "unresolved_work_grouping",
        &mut omissions,
    );

    let mut open_questions = projection.resume.open_questions.clone();
    open_questions.sort_by_key(|question| (!question.on_current_frontier, question.question_id));
    bound_section(&mut open_questions, limit, "open_questions", &mut omissions);

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

    let topology_decisions = projection
        .resume
        .decisions
        .iter()
        .filter(|decision| {
            projection
                .selected_work
                .as_ref()
                .is_none_or(|work| work.decision_ids.contains(&decision.decision_id))
        })
        .map(|decision| decision_understanding(decision, links.get(&decision.decision_id).copied()))
        .collect::<Vec<_>>();
    // Exact entity detail is a separate grounded neighborhood, selected before
    // the parent map bound. Keep the Work selection and canonical meanings intact.
    let focused_topology = projection.selected_entity.as_ref().map(|entity| {
        let mut entities = projection.selected_entity_neighbors.clone();
        entities.push(entity.clone());
        entities.sort_by(|left, right| left.identity.cmp(&right.identity));
        entities.dedup_by(|left, right| left.identity == right.identity);
        crate::CurrentWorkTopology {
            entities,
            relations: projection.selected_entity_relations.clone(),
            omitted_entity_count: 0,
            omitted_relation_count: projection.omitted_selected_relation_count,
        }
    });
    let architecture_topology = focused_topology
        .as_ref()
        .unwrap_or(&projection.current_work_topology);
    let architecture_selection = if let Some(entity) = &projection.selected_entity {
        CurrentWorkArchitectureSelection {
            topology: crate::project::select_bounded_topology(
                &architecture_topology.entities,
                &architecture_topology.relations,
                &BTreeSet::from([entity.identity.clone()]),
                limit,
                limit,
                false,
            ),
            selection_basis: Vec::new(),
        }
    } else {
        select_current_work_architecture(
            projection,
            if projection.selected_work.is_some() {
                &projection.selected_work_decisions
            } else {
                &topology_decisions
            },
            limit,
        )
    };
    let topology = architecture_selection.topology;
    let all_entities = architecture_topology
        .entities
        .iter()
        .map(|entity| entity.identity.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let resolved_relation_count = architecture_topology
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
    let omitted_current_work_entities = architecture_topology
        .omitted_entity_count
        .saturating_add(topology.omitted_entity_count);
    if omitted_current_work_entities > 0 {
        omissions.push(UnderstandingOmission {
            section: "architecture.components".to_owned(),
            omitted_count: omitted_current_work_entities,
        });
    }
    let omitted_resolved_relations = architecture_topology
        .omitted_relation_count
        .saturating_add(resolved_relation_count.saturating_sub(topology.relations.len()));
    if omitted_resolved_relations > 0 {
        omissions.push(UnderstandingOmission {
            section: "architecture.relationships".to_owned(),
            omitted_count: omitted_resolved_relations,
        });
    }
    let components = topology.entities;
    let relationships = topology.relations;
    let selection_basis = architecture_selection.selection_basis;
    let visible_components = components
        .iter()
        .map(|entity| entity.identity.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let unresolved_relation_count = architecture_topology
        .relations
        .iter()
        .filter(|relation| relation.target_entity.is_none() && relation.unresolved_target.is_some())
        .count();
    let mut unresolved_relationships = architecture_topology
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
    let mut gaps = projection.answer_capability_gaps.clone();
    bound_section(&mut gaps, limit, "architecture.gaps", &mut omissions);

    let explanation_relationships = relationships
        .iter()
        .chain(&unresolved_relationships)
        .cloned()
        .collect::<Vec<_>>();
    let mut deterministic_explanations = deterministic_explanations(
        &components,
        &explanation_relationships,
        &selection_basis,
        &active_decisions,
        &gaps,
        limit,
        &mut omissions,
    );

    if projection.sections.code == crate::ReadSectionState::NotRequested {
        deterministic_explanations.retain(|e| e.kind != UnderstandingExplanationKind::Gap);
    }
    let call_relations = relationships
        .iter()
        .filter(|r| is_flow_relation(r))
        .map(|r| r.identity.clone())
        .collect::<Vec<_>>();
    let flow_evidence = ArchitectureFlowEvidence {
        state: match projection.sections.code {
            crate::ReadSectionState::NotRequested => ArchitectureFlowState::NotRequested,
            crate::ReadSectionState::Unavailable => ArchitectureFlowState::AnalysisUnavailable,
            crate::ReadSectionState::Available if call_relations.is_empty() => {
                ArchitectureFlowState::NoResolvedCalls
            }
            crate::ReadSectionState::Available => ArchitectureFlowState::SyntacticCalls,
        },
        relation_ids: call_relations,
        missing_evidence: if projection.sections.code == crate::ReadSectionState::NotRequested {
            Vec::new()
        } else {
            let mut missing = vec!["Stored static analysis does not observe runtime execution, data flow or control flow.".into()];
            if !relationships.iter().any(is_flow_relation) {
                missing.push("No resolved CallsSyntactically relation connects entities in the selected scope; dependencies and symbol references do not establish calls.".into());
                missing.extend(
                    gaps.iter()
                        .filter(|g| {
                            matches!(
                                g.capability,
                                volicord_repository_intelligence::Capability::Structural
                                    | volicord_repository_intelligence::Capability::Semantic
                            )
                        })
                        .map(|g| {
                            format!(
                                "{} / {:?} / {:?}: {}",
                                g.area, g.capability, g.state, g.reason
                            )
                        }),
                );
            }
            missing
        },
    };

    let mut generated_interpretations = projection.repository_map.agent_interpretations.clone();
    generated_interpretations.retain(|interpretation| {
        interpretation
            .entity_basis
            .iter()
            .any(|id| components.iter().any(|e| &e.identity == id))
    });
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
        repository_analysis: projection.repository_analysis.clone(),
        selection: projection.selection,
        selected_work: projection.selected_work.clone(),
        selected_work_decisions: projection.selected_work_decisions.clone(),
        project_id: projection.overview.project_id,
        project_name: projection.overview.project_name.clone(),
        canonical_revision: projection.overview.canonical_revision,
        project_purpose,
        work_overview,
        current_work,
        completed_work,
        remaining_work,
        work_history,
        unresolved_work_grouping,
        active_decisions,
        open_questions,
        risks_assumptions_and_limits,
        known_limits,
        architecture: UnderstandingArchitecture {
            flow_evidence,
            components,
            relationships,
            selection_basis,
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

struct CurrentWorkArchitectureSelection {
    topology: BoundedTopology,
    selection_basis: Vec<UnderstandingArchitectureSelection>,
}

fn select_current_work_architecture(
    projection: &ProjectProjection,
    active_decisions: &[UnderstandingDecision],
    limit: usize,
) -> CurrentWorkArchitectureSelection {
    let limit = limit.max(1);
    if projection.selection.selector == crate::WorkSelector::Repository {
        return CurrentWorkArchitectureSelection {
            topology: crate::project::select_bounded_topology(
                &projection.repository_map.entities,
                &projection.repository_map.relations,
                &BTreeSet::new(),
                limit,
                limit,
                false,
            ),
            selection_basis: Vec::new(),
        };
    }
    let entities = projection
        .current_work_topology
        .entities
        .iter()
        .map(|entity| (entity.identity.as_str(), entity))
        .collect::<BTreeMap<_, _>>();
    let mut basis = BTreeMap::<String, BTreeSet<UnderstandingArchitectureSelectionBasis>>::new();

    for link in &projection.current_work_code {
        if !entities.contains_key(link.entity_identity.as_str()) {
            continue;
        }
        for path in &link.changed_path_basis {
            basis
                .entry(link.entity_identity.clone())
                .or_default()
                .insert(UnderstandingArchitectureSelectionBasis::ChangedPath {
                    checkpoint_id: path.checkpoint_id,
                    path: path.path.clone(),
                });
        }
        for checkpoint_id in &link.checkpoint_basis {
            if !link
                .changed_path_basis
                .iter()
                .any(|path| path.checkpoint_id == *checkpoint_id)
            {
                basis
                    .entry(link.entity_identity.clone())
                    .or_default()
                    .insert(UnderstandingArchitectureSelectionBasis::CheckpointLink {
                        checkpoint_id: *checkpoint_id,
                    });
            }
        }
        for context_item_id in &link.goal_context_basis {
            basis
                .entry(link.entity_identity.clone())
                .or_default()
                .insert(UnderstandingArchitectureSelectionBasis::GoalContextLink {
                    context_item_id: *context_item_id,
                });
        }
    }

    for decision in active_decisions {
        for identity in &decision.affected_code_entities {
            if entities.contains_key(identity.as_str()) {
                basis.entry(identity.clone()).or_default().insert(
                    UnderstandingArchitectureSelectionBasis::DecisionCodeLink {
                        decision_id: decision.decision.decision_id,
                    },
                );
            }
        }
    }

    let seed_ids = select_seed_entities(&basis, active_decisions, &entities, limit);
    let mut selected_ids = seed_ids.iter().cloned().collect::<BTreeSet<_>>();
    let mut relation_candidates = projection
        .current_work_topology
        .relations
        .iter()
        .filter(|relation| {
            relation.target_entity.as_deref().is_some_and(|target| {
                entities.contains_key(relation.source_entity.as_str())
                    && entities.contains_key(target)
                    && (basis.contains_key(&relation.source_entity) || basis.contains_key(target))
            })
        })
        .collect::<Vec<_>>();
    relation_candidates.sort_by_key(|relation| {
        let target_is_seed = relation
            .target_entity
            .as_deref()
            .is_some_and(|target| selected_ids.contains(target));
        let target_is_grounded = relation
            .target_entity
            .as_deref()
            .is_some_and(|target| basis.contains_key(target));
        (
            Reverse(selected_ids.contains(&relation.source_entity) && target_is_seed),
            Reverse(selected_ids.contains(&relation.source_entity) || target_is_seed),
            Reverse(is_flow_relation(relation)),
            Reverse(
                usize::from(basis.contains_key(&relation.source_entity))
                    + usize::from(target_is_grounded),
            ),
            relation.identity.as_str(),
        )
    });

    let mut selected_relations = Vec::new();
    for relation in relation_candidates {
        if selected_relations.len() == limit {
            break;
        }
        let Some(target) = relation.target_entity.as_deref() else {
            continue;
        };
        let new_endpoint_count = [&relation.source_entity, target]
            .into_iter()
            .filter(|identity| !selected_ids.contains(*identity))
            .count();
        if selected_ids.len() + new_endpoint_count > limit {
            continue;
        }
        for (endpoint, other) in [
            (relation.source_entity.as_str(), target),
            (target, relation.source_entity.as_str()),
        ] {
            if selected_ids.insert(endpoint.to_owned()) && !basis.contains_key(endpoint) {
                basis.entry(endpoint.to_owned()).or_default().insert(
                    UnderstandingArchitectureSelectionBasis::GroundedOneHop {
                        relation_id: relation.identity.clone(),
                        seed_entity: other.to_owned(),
                    },
                );
            }
        }
        selected_relations.push(relation.clone());
    }

    for identity in ranked_seed_entities(&basis, &entities) {
        if selected_ids.len() == limit {
            break;
        }
        selected_ids.insert(identity);
    }

    let mut selected_entities = selected_ids
        .iter()
        .filter_map(|identity| entities.get(identity.as_str()).copied().cloned())
        .collect::<Vec<_>>();
    selected_entities.sort_by_key(|entity| {
        (
            seed_ids
                .iter()
                .position(|identity| identity == &entity.identity)
                .unwrap_or(usize::MAX),
            entity.identity.clone(),
        )
    });
    selected_relations.sort_by(|left, right| left.identity.cmp(&right.identity));
    let selection_basis = selected_entities
        .iter()
        .map(|entity| UnderstandingArchitectureSelection {
            entity_identity: entity.identity.clone(),
            basis: basis
                .remove(&entity.identity)
                .unwrap_or_default()
                .into_iter()
                .collect(),
        })
        .collect();
    CurrentWorkArchitectureSelection {
        topology: BoundedTopology {
            omitted_entity_count: projection
                .current_work_topology
                .entities
                .len()
                .saturating_sub(selected_entities.len()),
            omitted_relation_count: projection
                .current_work_topology
                .relations
                .len()
                .saturating_sub(selected_relations.len()),
            entities: selected_entities,
            relations: selected_relations,
        },
        selection_basis,
    }
}

fn select_seed_entities(
    basis: &BTreeMap<String, BTreeSet<UnderstandingArchitectureSelectionBasis>>,
    active_decisions: &[UnderstandingDecision],
    entities: &BTreeMap<&str, &MapEntity>,
    limit: usize,
) -> Vec<String> {
    let mut selected = Vec::<String>::new();
    let mut push_first = |predicate: &dyn Fn(&UnderstandingArchitectureSelectionBasis) -> bool| {
        if selected.len() == limit {
            return;
        }
        if let Some(identity) = basis
            .iter()
            .filter(|(identity, reasons)| {
                reasons.iter().any(predicate) && !selected.contains(identity)
            })
            .min_by_key(|(identity, _)| {
                (
                    entities
                        .get(identity.as_str())
                        .map_or(usize::MAX, |entity| {
                            architecture_entity_kind_rank(&entity.kind)
                        }),
                    identity.as_str(),
                )
            })
            .map(|(identity, _)| (*identity).clone())
        {
            selected.push(identity);
        }
    };
    push_first(&|reason| {
        matches!(
            reason,
            UnderstandingArchitectureSelectionBasis::ChangedPath { .. }
        )
    });
    for decision in active_decisions {
        let decision_id = decision.decision.decision_id;
        push_first(
            &|reason| matches!(reason, UnderstandingArchitectureSelectionBasis::DecisionCodeLink { decision_id: candidate } if *candidate == decision_id),
        );
    }
    push_first(&|reason| {
        matches!(
            reason,
            UnderstandingArchitectureSelectionBasis::GoalContextLink { .. }
        )
    });
    push_first(&|reason| {
        matches!(
            reason,
            UnderstandingArchitectureSelectionBasis::CheckpointLink { .. }
        )
    });

    selected
}

fn ranked_seed_entities(
    basis: &BTreeMap<String, BTreeSet<UnderstandingArchitectureSelectionBasis>>,
    entities: &BTreeMap<&str, &MapEntity>,
) -> Vec<String> {
    let mut ranked = basis.keys().cloned().collect::<Vec<_>>();
    ranked.sort_by_key(|identity| {
        let reasons = basis
            .get(identity)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        (
            Reverse(reasons.iter().any(|reason| {
                matches!(
                    reason,
                    UnderstandingArchitectureSelectionBasis::ChangedPath { .. }
                )
            })),
            Reverse(reasons.iter().any(|reason| {
                matches!(
                    reason,
                    UnderstandingArchitectureSelectionBasis::DecisionCodeLink { .. }
                )
            })),
            Reverse(reasons.len()),
            entities
                .get(identity.as_str())
                .map_or(usize::MAX, |entity| {
                    architecture_entity_kind_rank(&entity.kind)
                }),
            identity.clone(),
        )
    });
    ranked
}

const fn architecture_entity_kind_rank(kind: &CodeEntityKind) -> usize {
    match kind {
        CodeEntityKind::Package => 0,
        CodeEntityKind::Module | CodeEntityKind::Namespace => 1,
        CodeEntityKind::File | CodeEntityKind::Configuration => 2,
        CodeEntityKind::Class
        | CodeEntityKind::Interface
        | CodeEntityKind::Trait
        | CodeEntityKind::Struct => 3,
        CodeEntityKind::Repository | CodeEntityKind::Test | CodeEntityKind::Document => 4,
        CodeEntityKind::Function
        | CodeEntityKind::Method
        | CodeEntityKind::Enum
        | CodeEntityKind::Type => 5,
        CodeEntityKind::Field | CodeEntityKind::LanguageSpecific(_) => 6,
    }
}

fn deterministic_explanations(
    components: &[MapEntity],
    relationships: &[MapRelation],
    selection_basis: &[UnderstandingArchitectureSelection],
    decisions: &[UnderstandingDecision],
    gaps: &[CapabilityGap],
    limit: usize,
    omissions: &mut Vec<UnderstandingOmission>,
) -> Vec<UnderstandingExplanation> {
    let entities = components
        .iter()
        .map(|entity| (entity.identity.as_str(), entity))
        .collect::<std::collections::BTreeMap<_, _>>();
    let selection_by_entity = selection_basis
        .iter()
        .map(|selection| (selection.entity_identity.as_str(), selection))
        .collect::<BTreeMap<_, _>>();
    let mut component_explanations = components
        .iter()
        .filter(|entity| is_explainable_component(entity, relationships))
        .map(|entity| {
            component_explanation(
                entity,
                relationships,
                &entities,
                selection_by_entity.get(entity.identity.as_str()).copied(),
            )
        })
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

    if !relationships
        .iter()
        .any(|r| is_flow_relation(r) && r.target_entity.is_some())
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
    selection: Option<&UnderstandingArchitectureSelection>,
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
    let mut explanation = explanation_from_entity_and_relations(
        format!("deterministic:component:{}", entity.identity),
        UnderstandingExplanationKind::Component,
        format!("{} {}", selection_explanation(selection, false), english),
        format!("{} {}", selection_explanation(selection, true), korean),
        entity,
        &supporting_relations,
    );
    if let Some(selection) = selection {
        explanation
            .decision_basis
            .extend(selection.basis.iter().filter_map(|basis| match basis {
                UnderstandingArchitectureSelectionBasis::DecisionCodeLink { decision_id } => {
                    Some(*decision_id)
                }
                _ => None,
            }));
    }
    for (text, korean) in [
        (&mut explanation.english, false),
        (&mut explanation.korean, true),
    ] {
        let behavior = crate::code_behavior::behavior_sentences(entity, korean);
        if !behavior.is_empty() {
            *text = format!("{} {}", selection_explanation(selection, korean), behavior);
        }
    }
    explanation
        .known_gaps
        .extend(entity.behavior.limitations.clone());
    normalize_explanation_basis(&mut explanation);
    explanation
}

fn selection_explanation(
    selection: Option<&UnderstandingArchitectureSelection>,
    korean: bool,
) -> String {
    let Some(selection) = selection else {
        return if korean {
            "표시된 저장소 토폴로지의 실제 엔터티 및 관계를 사용한 설명입니다.".to_owned()
        } else {
            "This explanation uses the displayed stored repository entities and relationships."
                .to_owned()
        };
    };
    let mut changed_paths = BTreeSet::new();
    let mut has_decision = false;
    let mut has_goal = false;
    let mut has_checkpoint = false;
    let mut has_one_hop_relation = false;
    for basis in &selection.basis {
        match basis {
            UnderstandingArchitectureSelectionBasis::ChangedPath { path, .. } => {
                changed_paths.insert(format!("`{path}`"));
            }
            UnderstandingArchitectureSelectionBasis::DecisionCodeLink { .. } => {
                has_decision = true;
            }
            UnderstandingArchitectureSelectionBasis::GoalContextLink { .. } => has_goal = true,
            UnderstandingArchitectureSelectionBasis::CheckpointLink { .. } => {
                has_checkpoint = true;
            }
            UnderstandingArchitectureSelectionBasis::GroundedOneHop { .. } => {
                has_one_hop_relation = true;
            }
        }
    }
    let mut parts = Vec::new();
    if !changed_paths.is_empty() {
        parts.push(if korean {
            format!(
                "변경 경로 {}",
                changed_paths.into_iter().collect::<Vec<_>>().join(", ")
            )
        } else {
            format!(
                "changed path {}",
                changed_paths.into_iter().collect::<Vec<_>>().join(", ")
            )
        });
    }
    if has_decision {
        parts.push(if korean {
            "active Decision의 code link".to_owned()
        } else {
            "an active Decision's code link".to_owned()
        });
    }
    if has_goal {
        parts.push(if korean {
            "현재 Goal Context link".to_owned()
        } else {
            "a current Goal Context link".to_owned()
        });
    }
    if has_checkpoint {
        parts.push(if korean {
            "latest meaningful Checkpoint link".to_owned()
        } else {
            "a latest meaningful Checkpoint link".to_owned()
        });
    }
    if has_one_hop_relation {
        parts.push(if korean {
            "근거 있는 한 홉 repository relation".to_owned()
        } else {
            "a grounded one-hop repository relation".to_owned()
        });
    }
    if korean {
        format!("현재 작업 근거({})로 선택되었습니다.", parts.join("; "))
    } else {
        format!("Selected for current work by {}.", parts.join("; "))
    }
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
        english: "No resolved syntactic call connects the displayed entities. Containment, imports, symbol references and implementation relationships do not establish execution, data or control flow.".to_owned(),
        korean: "표시된 엔터티 사이에 확인된 구문 호출이 없습니다. 포함, import, 심볼 참조 및 구현 관계는 실행·데이터·제어 흐름의 근거가 아닙니다.".to_owned(),
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
    relation.role() == crate::CodeRelationshipRole::SyntacticCall
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
    // Fine-grained applicability narrows the declaration; it cannot supply or
    // broaden canonical Work grouping. Only the Work identity is available here.
    let work_scope = match (&decision.decision.work_scope, korean) {
        (DecisionWorkScope::ProjectWide, false) => "Project-wide applicability".to_owned(),
        (DecisionWorkScope::ProjectWide, true) => "프로젝트 전체".to_owned(),
        (DecisionWorkScope::WorkItem(id), false) => {
            format!("Work-bounded applicability to Work Item `{id}`")
        }
        (DecisionWorkScope::WorkItem(id), true) => format!("Work Item `{id}`에 한정된 적용"),
        (DecisionWorkScope::Unresolved, false) => "Unresolved work applicability".to_owned(),
        (DecisionWorkScope::Unresolved, true) => "작업 적용 범위 미확정".to_owned(),
    };
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
        work_scope
    } else if korean {
        format!("{work_scope}, 세부 선언 범위 {}", quoted_names(&values))
    } else {
        format!(
            "{work_scope}, with declared fine-grained scope {}",
            quoted_names(&values)
        )
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

pub(crate) fn decision_understanding(
    decision: &BriefDecision,
    link: Option<&DecisionContextCodeLink>,
) -> UnderstandingDecision {
    UnderstandingDecision {
        reading: crate::reading::decision_reading(decision),
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

#[cfg(test)]
mod tests {
    use super::{
        build_project_understanding, UnderstandingArchitectureSelectionBasis, UnderstandingBound,
        UnderstandingExplanationKind,
    };
    use crate::{
        BriefDecision, CandidateDependencyState, CheckpointTimelineEntry, CurrentWorkCodeLink,
        CurrentWorkTopology, DecisionContextCodeLink, MapEntity, MapRelation, MapRelationClass,
        ProjectOverview, ProjectProjection, ProjectionHealth, RepositoryMap, ResumeBrief,
        SourceStatusSummary,
    };
    use volicord_context::{
        Checkpoint, CheckpointId, CheckpointKind, ContextItemId, DecisionChoice, DecisionId,
        DecisionWorkScope, ProjectId, SourceId, TimestampMicros, UserAcceptanceFact,
        UserAcceptanceState, UserReviewFact, UserReviewState, WorkState,
    };
    use volicord_repository_intelligence::{
        AnalysisSnapshotId, CodeEntityKind, FreshnessBasis, FreshnessState, Language,
        RepositorySnapshotId, Uncertainty,
    };

    #[test]
    fn decision_explanations_preserve_typed_work_scope_and_fine_grained_applicability() {
        let work_id = ContextItemId::from_bytes([7; 16]);
        let decision_id = DecisionId::from_bytes([3; 16]);
        for work_scope in [
            DecisionWorkScope::ProjectWide,
            DecisionWorkScope::WorkItem(work_id),
            DecisionWorkScope::Unresolved,
        ] {
            // Empty, each independent dimension, and all three together.
            for dimensions in [0, 1, 2, 4, 7] {
                for code_linked in [false, true] {
                    let mut projection = projection(
                        vec![entity("policy", "core/policy.rs", Language::Rust)],
                        Vec::new(),
                        checkpoint(CheckpointId::from_bytes([2; 16]), Vec::new(), Vec::new()),
                        Some((decision_id, "policy")),
                    );
                    projection.resume.decisions[0].work_scope = work_scope;
                    let link = &mut projection.decision_context_code[0];
                    link.declared_paths = if dimensions & 1 != 0 {
                        vec!["core/policy.rs".into()]
                    } else {
                        Vec::new()
                    };
                    link.declared_components = if dimensions & 2 != 0 {
                        vec!["policy-component".into()]
                    } else {
                        Vec::new()
                    };
                    link.declared_work_contexts = if dimensions & 4 != 0 {
                        vec!["release-context".into()]
                    } else {
                        Vec::new()
                    };
                    if !code_linked {
                        link.related_code_entities.clear();
                    }
                    let original = projection.clone();
                    let understanding =
                        build_project_understanding(&projection, UnderstandingBound::default());
                    let explanation = understanding
                        .deterministic_explanations
                        .iter()
                        .find(|item| item.kind == UnderstandingExplanationKind::DecisionImpact)
                        .unwrap_or_else(|| panic!("missing Decision explanation"));
                    for (text, project_wide, work_bounded, unresolved) in [
                        (
                            &explanation.english,
                            "Project-wide applicability",
                            "Work-bounded applicability",
                            "Unresolved work applicability",
                        ),
                        (
                            &explanation.korean,
                            "프로젝트 전체",
                            "한정된 적용",
                            "작업 적용 범위 미확정",
                        ),
                    ] {
                        assert_eq!(
                            text.contains(project_wide),
                            work_scope == DecisionWorkScope::ProjectWide
                        );
                        assert_eq!(
                            text.contains(work_bounded),
                            matches!(work_scope, DecisionWorkScope::WorkItem(_))
                        );
                        assert_eq!(
                            text.contains(unresolved),
                            work_scope == DecisionWorkScope::Unresolved
                        );
                        assert_eq!(
                            text.contains(&format!("`{work_id}`")),
                            matches!(work_scope, DecisionWorkScope::WorkItem(_))
                        );
                        for (mask, declared) in [
                            (1, "core/policy.rs"),
                            (2, "policy-component"),
                            (4, "release-context"),
                        ] {
                            assert_eq!(
                                text.contains(&format!("`{declared}`")),
                                dimensions & mask != 0
                            );
                        }
                    }
                    if code_linked {
                        assert!(explanation.english.contains("not proof of implementation"));
                        assert!(explanation.korean.contains("구현 완료의 증거가 아닙니다"));
                    } else {
                        assert!(explanation.english.contains("no code effect is inferred"));
                        assert!(explanation.korean.contains("코드 영향을 추론하지 않습니다"));
                    }
                    assert_eq!(explanation.decision_basis, vec![decision_id]);
                    assert_eq!(
                        explanation.source_basis,
                        vec![SourceId::from_bytes([8; 16])]
                    );
                    assert_eq!(
                        explanation.analysis_snapshot_basis,
                        if code_linked {
                            vec![analysis_snapshot()]
                        } else {
                            Vec::new()
                        }
                    );
                    assert_eq!(projection, original);
                }
            }
        }
    }

    #[test]
    fn current_work_seeds_outrank_disconnected_high_connectivity() {
        let checkpoint_id = CheckpointId::from_bytes([2; 16]);
        let decision_id = DecisionId::from_bytes([3; 16]);
        let mut entities = vec![
            entity("task", "src/task.py", Language::Python),
            entity("service", "web/service.ts", Language::TypeScript),
            entity("policy", "core/policy.rs", Language::Rust),
        ];
        entities.extend((0..16).map(|index| {
            entity(
                &format!("generic-{index:02}"),
                &format!("vendor/generic-{index:02}.js"),
                Language::JavaScript,
            )
        }));
        let mut relations = vec![relation(
            "flow:task-service",
            "task",
            "service",
            "CallsSyntactically",
        )];
        relations.extend((1..16).map(|index| {
            relation(
                &format!("generic:hub-{index:02}"),
                "generic-00",
                &format!("generic-{index:02}"),
                "Imports",
            )
        }));
        let projection = projection(
            entities,
            relations,
            checkpoint(checkpoint_id, vec!["src/task.py".into()], vec![decision_id]),
            Some((decision_id, "policy")),
        );

        let understanding = build_project_understanding(
            &projection,
            UnderstandingBound {
                max_items_per_section: 4,
            },
        );
        let identities = understanding
            .architecture
            .components
            .iter()
            .map(|entity| entity.identity.as_str())
            .collect::<Vec<_>>();
        assert!(identities.contains(&"task"));
        assert!(identities.contains(&"policy"));
        assert!(identities.contains(&"service"));
        assert!(identities
            .iter()
            .all(|identity| !identity.starts_with("generic")));
        assert_eq!(
            understanding
                .architecture
                .relationships
                .iter()
                .map(|relation| relation.identity.as_str())
                .collect::<Vec<_>>(),
            vec!["flow:task-service"]
        );
        let task_basis = selection(&understanding, "task");
        assert!(task_basis.iter().any(|basis| matches!(
            basis,
            UnderstandingArchitectureSelectionBasis::ChangedPath {
                checkpoint_id: candidate,
                path
            } if *candidate == checkpoint_id && path == "src/task.py"
        )));
        let policy_basis = selection(&understanding, "policy");
        assert!(policy_basis.iter().any(|basis| matches!(
            basis,
            UnderstandingArchitectureSelectionBasis::DecisionCodeLink {
                decision_id: candidate
            } if *candidate == decision_id
        )));
        let service_basis = selection(&understanding, "service");
        assert!(service_basis.iter().any(|basis| matches!(
            basis,
            UnderstandingArchitectureSelectionBasis::GroundedOneHop {
                relation_id,
                seed_entity
            } if relation_id == "flow:task-service" && seed_entity == "task"
        )));
        assert!(understanding.architecture.components.len() <= 4);
        assert!(understanding.architecture.relationships.len() <= 4);
        assert!(understanding.architecture.components.iter().all(|entity| {
            entity.source_id == SourceId::from_bytes([9; 16])
                && entity.analysis_snapshot == analysis_snapshot()
        }));

        let mut reordered = projection.clone();
        reordered.repository_map.entities.reverse();
        reordered.repository_map.relations.reverse();
        assert_eq!(
            understanding.architecture,
            build_project_understanding(
                &reordered,
                UnderstandingBound {
                    max_items_per_section: 4,
                },
            )
            .architecture
        );
    }

    #[test]
    fn no_grounded_current_work_flow_is_an_explicit_reduced_result() {
        let checkpoint_id = CheckpointId::from_bytes([4; 16]);
        let projection = projection(
            vec![
                entity("task", "src/task.py", Language::Python),
                entity("generic-a", "vendor/a.js", Language::JavaScript),
                entity("generic-b", "vendor/b.js", Language::JavaScript),
            ],
            vec![relation(
                "generic:edge",
                "generic-a",
                "generic-b",
                "Imports",
            )],
            checkpoint(checkpoint_id, vec!["src/task.py".into()], Vec::new()),
            None,
        );

        let understanding = build_project_understanding(
            &projection,
            UnderstandingBound {
                max_items_per_section: 8,
            },
        );
        assert_eq!(understanding.architecture.components.len(), 1);
        assert_eq!(understanding.architecture.components[0].identity, "task");
        assert!(understanding.architecture.relationships.is_empty());
        let flow_gap = understanding
            .deterministic_explanations
            .iter()
            .find(|explanation| explanation.kind == UnderstandingExplanationKind::Gap)
            .unwrap_or_else(|| panic!("missing explicit grounded-flow gap"));
        assert!(flow_gap.relation_basis.is_empty());
        assert!(flow_gap
            .english
            .contains("do not establish execution, data or control flow"));
    }

    #[test]
    fn goal_link_works_without_a_decision_or_changed_path() {
        let checkpoint_id = CheckpointId::from_bytes([5; 16]);
        let goal_id = ContextItemId::from_bytes([6; 16]);
        let mut projection = projection(
            vec![
                entity("goal-component", "app/main.cpp", Language::Cpp),
                entity("generic", "vendor/helper.js", Language::JavaScript),
            ],
            Vec::new(),
            checkpoint(checkpoint_id, Vec::new(), Vec::new()),
            None,
        );
        projection.current_work_code = vec![CurrentWorkCodeLink {
            changed_path_basis: Vec::new(),
            entity_identity: "goal-component".into(),
            changed_paths: Vec::new(),
            checkpoint_basis: Vec::new(),
            goal_context_basis: vec![goal_id],
        }];

        let understanding = build_project_understanding(
            &projection,
            UnderstandingBound {
                max_items_per_section: 4,
            },
        );
        assert_eq!(understanding.architecture.components.len(), 1);
        assert_eq!(
            understanding.architecture.components[0].identity,
            "goal-component"
        );
        assert!(selection(&understanding, "goal-component")
            .iter()
            .any(|basis| matches!(
                basis,
                UnderstandingArchitectureSelectionBasis::GoalContextLink {
                    context_item_id
                } if *context_item_id == goal_id
            )));
    }

    fn selection<'a>(
        understanding: &'a super::ProjectUnderstanding,
        identity: &str,
    ) -> &'a [UnderstandingArchitectureSelectionBasis] {
        &understanding
            .architecture
            .selection_basis
            .iter()
            .find(|selection| selection.entity_identity == identity)
            .unwrap_or_else(|| panic!("missing selection basis for {identity}"))
            .basis
    }

    fn projection(
        entities: Vec<MapEntity>,
        relations: Vec<MapRelation>,
        checkpoint: Checkpoint,
        decision: Option<(DecisionId, &str)>,
    ) -> ProjectProjection {
        let current_work_code = entities
            .iter()
            .filter_map(|entity| {
                let changed_paths = checkpoint
                    .changed_paths
                    .iter()
                    .filter(|path| entity.locator == path.as_str())
                    .cloned()
                    .collect::<Vec<_>>();
                (!changed_paths.is_empty()).then(|| CurrentWorkCodeLink {
                    changed_path_basis: changed_paths
                        .iter()
                        .map(|path| crate::CurrentWorkPathBasis {
                            checkpoint_id: checkpoint.id,
                            checkpoint_revision: checkpoint.revision,
                            path: path.clone(),
                        })
                        .collect(),
                    entity_identity: entity.identity.clone(),
                    changed_paths,
                    checkpoint_basis: vec![checkpoint.id],
                    goal_context_basis: Vec::new(),
                })
            })
            .collect();
        let decisions = decision
            .map(|(decision_id, _)| BriefDecision {
                question_reference: volicord_context::QuestionReference {
                    question_id: volicord_context::QuestionId::from_bytes([0; 16]),
                    revision: 1,
                },
                question_context: None,
                question_source_status: Vec::new(),
                explanations: Vec::new(),
                user_source_basis: Vec::new(),
                user_source_status: Vec::new(),
                recommendation_source_status: Vec::new(),
                recommendation_source_basis: Vec::new(),
                available_revisions: vec![1],
                decision_id,
                revision: 1,
                state: crate::BriefDecisionState::Current,
                choice: DecisionChoice::Alternative {
                    alternative_key: "bounded-current-work".into(),
                },
                chosen_alternative_key: Some("bounded-current-work".into()),
                recommended_alternative_key: Some("bounded-current-work".into()),
                displayed_alternatives: vec![volicord_context::QuestionAlternative {
                    key: "bounded-current-work".into(),
                    label: "Keep current work grounded".into(),
                    consequence: "The current-work component remains source-grounded".into(),
                }],
                work_scope: DecisionWorkScope::Unresolved,
                user_rationale: Some("keep the current work explainable".into()),
                recommendation_rationale: "retain grounded code".into(),
                assumptions: Vec::new(),
                revisit_triggers: Vec::new(),
                source_basis: vec![SourceId::from_bytes([8; 16])],
                question_uncertainty: Vec::new(),
                known_limits: Vec::new(),
                review_issues: Vec::new(),
            })
            .into_iter()
            .collect::<Vec<_>>();
        let decision_context_code = decision
            .map(|(decision_id, entity)| DecisionContextCodeLink {
                decision_id,
                decision_revision: 1,
                decision_state: crate::BriefDecisionState::Current,
                declared_paths: vec!["core/policy.rs".into()],
                declared_components: Vec::new(),
                declared_work_contexts: Vec::new(),
                assumption_context: Vec::new(),
                related_context_items: Vec::new(),
                related_code_entities: vec![entity.into()],
                supporting_sources: vec![SourceId::from_bytes([8; 16])],
                link_basis: vec!["explicit fixture Decision/code link".into()],
                missing_or_uncertain_links: Vec::new(),
            })
            .into_iter()
            .collect();
        let checkpoint_entry = CheckpointTimelineEntry {
            work_state: checkpoint.work_state,
            verification: checkpoint.verification.clone(),
            user_review: checkpoint.user_review.clone(),
            user_acceptance: checkpoint.user_acceptance.clone(),
            checkpoint: checkpoint.clone(),
        };
        ProjectProjection {
            repository_scope_metadata: crate::RepositoryScopeMetadata::default(),
            repository_analysis: crate::RepositoryAnalysisReading::absent(false),
            answer_capability_gaps: Vec::new(),
            answer_issues: Vec::new(),
            work_read_cost: crate::WorkReadCost::default(),
            canonical_read_fingerprint: String::new(),
            sections: crate::ProjectReadSections {
                code: crate::ReadSectionState::Available,
                inspection: crate::ReadSectionState::Available,
            },
            work_count: 0,
            decision_count: 0,
            decision_catalog: Vec::new(),
            selected_decision: None,
            selected_entity: None,
            selected_entity_relations: Vec::new(),
            selected_entity_neighbors: Vec::new(),
            omitted_selected_relation_count: 0,
            selection: crate::WorkSelection {
                selector: crate::WorkSelector::LatestWork,
                work_item_id: None,
                basis: crate::WorkSelectionBasis::UnassociatedCheckpoint {
                    checkpoint_id: checkpoint.id,
                    revision: checkpoint.revision,
                },
            },
            selected_work: None,
            selected_work_decisions: Vec::new(),
            unresolved_work_grouping: Vec::new(),
            work_overview: crate::WorkOverview::from_selection(
                std::array::from_fn(|_| (Vec::new(), 0)),
                &std::collections::BTreeMap::new(),
            ),
            work_history: Vec::new(),
            overview: ProjectOverview {
                project_id: project_id(),
                project_name: "Current work fixture".into(),
                canonical_revision: 1,
                current_goals: vec![checkpoint.goal.clone()],
                active_decision_count: decisions.len(),
                superseded_decision_count: 0,
                open_question_count: 0,
                latest_checkpoint_id: Some(checkpoint.id),
                source_status: SourceStatusSummary::default(),
                capability_reports: Vec::new(),
                health: ProjectionHealth::Complete,
            },
            resume: ResumeBrief {
                selected_work: None,
                project_purpose: Vec::new(),
                project_id: project_id(),
                project_name: "Current work fixture".into(),
                goals_and_why: Vec::new(),
                behaviorally_relevant_context: Vec::new(),
                decisions,
                latest_meaningful_checkpoint: Some(checkpoint),
                open_questions: Vec::new(),
                risks_assumptions_and_limits: Vec::new(),
                declared_assumptions: Vec::new(),
                known_limits: Vec::new(),
                next_meaningful_step: Some("inspect the selected flow".into()),
                used_sources: Vec::new(),
                snapshots: Vec::new(),
                omissions: Vec::new(),
                omitted_count: 0,
                proposals: Vec::new(),
            },
            current_work_topology: CurrentWorkTopology {
                entities: entities.clone(),
                relations: relations.clone(),
                omitted_entity_count: 0,
                omitted_relation_count: 0,
            },
            repository_map: RepositoryMap {
                entities,
                relations,
                agent_interpretations: Vec::new(),
                capabilities: Vec::new(),
                gaps: Vec::new(),
                health: ProjectionHealth::Complete,
            },
            current_work_code,
            decision_context_code,
            checkpoint_timeline: vec![checkpoint_entry],
            canonical_inspection: Vec::new(),
            candidate_inspection: Vec::new(),
            candidate_dependency: CandidateDependencyState::Available,
            source_catalog: Vec::new(),
            issues: Vec::new(),
            health: ProjectionHealth::Complete,
        }
    }

    fn checkpoint(
        id: CheckpointId,
        changed_paths: Vec<String>,
        decisions: Vec<DecisionId>,
    ) -> Checkpoint {
        Checkpoint {
            id,
            project_id: project_id(),
            revision: 1,
            work_item_id: Some(ContextItemId::from_bytes([6; 16])),
            kind: CheckpointKind::Pause,
            goal: "Explain the current task architecture".into(),
            work_state: WorkState::Paused,
            state_change: Some("current work changed".into()),
            source_basis: vec![SourceId::from_bytes([7; 16])],
            changed_source_basis: vec![SourceId::from_bytes([9; 16])],
            changed_paths,
            applied_decisions: decisions,
            verification: Vec::new(),
            user_review: UserReviewFact {
                state: UserReviewState::Pending,
                source_id: None,
            },
            user_acceptance: UserAcceptanceFact {
                state: UserAcceptanceState::NotRequested,
                source_id: None,
            },
            known_limits: Vec::new(),
            non_goals: Vec::new(),
            open_questions: Vec::new(),
            next_step: "inspect the selected flow".into(),
            handoff_to: None,
            recorded_at: TimestampMicros::from_unix_micros(10),
        }
    }

    fn entity(identity: &str, locator: &str, language: Language) -> MapEntity {
        MapEntity {
            behavior: crate::CodeBehaviorReading::unavailable(),
            identity: identity.into(),
            display_name: identity.into(),
            locator: locator.into(),
            kind: CodeEntityKind::Module,
            language,
            source_id: SourceId::from_bytes([9; 16]),
            source_range: None,
            analysis_snapshot: analysis_snapshot(),
            repository_snapshot: repository_snapshot(),
            freshness: freshness(),
            uncertainty: Uncertainty::none(),
            canonical_links: Vec::new(),
        }
    }

    fn relation(identity: &str, source: &str, target: &str, kind: &str) -> MapRelation {
        MapRelation {
            identity: identity.into(),
            class: MapRelationClass::StructuralFact,
            kind: kind.into(),
            source_entity: source.into(),
            target_entity: Some(target.into()),
            unresolved_target: None,
            source_id: SourceId::from_bytes([9; 16]),
            supporting_range: None,
            analysis_snapshot: analysis_snapshot(),
            repository_snapshot: repository_snapshot(),
            freshness: freshness(),
            uncertainty: Uncertainty::none(),
            diagnostics: Vec::new(),
        }
    }

    fn project_id() -> ProjectId {
        ProjectId::from_bytes([1; 16])
    }

    fn repository_snapshot() -> RepositorySnapshotId {
        RepositorySnapshotId::from_hex(&"11".repeat(32))
            .unwrap_or_else(|error| panic!("valid Repository Snapshot fixture: {error}"))
    }

    fn analysis_snapshot() -> AnalysisSnapshotId {
        AnalysisSnapshotId::from_hex(&"22".repeat(32))
            .unwrap_or_else(|error| panic!("valid Analysis Snapshot fixture: {error}"))
    }

    fn freshness() -> FreshnessBasis {
        FreshnessBasis {
            state: FreshnessState::Current,
            repository_snapshot: repository_snapshot(),
            compared_repository_snapshot: None,
            reason: None,
        }
    }
}
