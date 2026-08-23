use crate::{
    BriefContextItem, BriefDecision, BriefQuestion, BriefSnapshot, CapabilityGap,
    CheckpointTimelineEntry, DecisionContextCodeLink, MapEntity, MapInterpretation, MapRelation,
    ProjectProjection, ProjectionHealth, ProjectionIssue, SourceStatusSummary,
};
use volicord_context::{
    CheckpointId, DecisionId, ProjectId, SourceId, SourceReadBasis, VerificationFact, WorkState,
};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnderstandingEvidence {
    pub sources: Vec<SourceReadBasis>,
    pub snapshots: Vec<BriefSnapshot>,
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

    let mut components = projection.repository_map.entities.clone();
    components.sort_by(|left, right| left.identity.cmp(&right.identity));
    bound_section(
        &mut components,
        limit,
        "architecture.components",
        &mut omissions,
    );
    let visible = components
        .iter()
        .map(|entity| entity.identity.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut relationships = projection
        .repository_map
        .relations
        .iter()
        .filter(|relation| {
            visible.contains(relation.source_entity.as_str())
                && relation
                    .target_entity
                    .as_deref()
                    .is_some_and(|target| visible.contains(target))
        })
        .cloned()
        .collect::<Vec<_>>();
    relationships.sort_by(|left, right| left.identity.cmp(&right.identity));
    bound_section(
        &mut relationships,
        limit,
        "architecture.relationships",
        &mut omissions,
    );
    let mut gaps = projection.repository_map.gaps.clone();
    bound_section(&mut gaps, limit, "architecture.gaps", &mut omissions);

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
        generated_interpretations,
        evidence: UnderstandingEvidence {
            sources,
            snapshots,
            source_status: projection.overview.source_status.clone(),
            issues,
        },
        omissions,
        health: projection.health,
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
