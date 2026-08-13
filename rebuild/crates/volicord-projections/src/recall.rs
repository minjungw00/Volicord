use std::collections::{BTreeMap, BTreeSet};
use volicord_context::{
    CanonicalReadBasis, Checkpoint, ContextItemId, ContextItemRole, DecisionChoice, DecisionId,
    ProjectId, QuestionId, SourceFreshness, SourceId, SourceReadBasis,
};
use volicord_inquiry::{
    compute_frontier, evaluate_decision_applicability, ApplicabilityQuery,
    DecisionApplicabilityState, InquiryScope,
};
use volicord_repository_intelligence::{
    AnalysisSnapshot, AnalysisSnapshotId, CapabilityReport, FreshnessBasis, RepositorySnapshotId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecallBound {
    /// Each repeatable section is independently bounded by this value. This
    /// preserves one representative from every required meaning category.
    pub max_items_per_section: usize,
}

impl Default for RecallBound {
    fn default() -> Self {
        Self {
            max_items_per_section: 8,
        }
    }
}

pub struct RecallInputs<'a> {
    pub canonical: &'a CanonicalReadBasis,
    pub analyses: &'a [&'a AnalysisSnapshot],
    pub scope: ApplicabilityQuery,
    pub bound: RecallBound,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BriefDecisionState {
    Current,
    StaleBasis,
    ReviewRequired,
    Superseded,
    UnavailableBasis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BriefDecision {
    pub decision_id: DecisionId,
    pub revision: u64,
    pub state: BriefDecisionState,
    pub choice: DecisionChoice,
    pub user_rationale: Option<String>,
    pub recommendation_rationale: String,
    pub assumptions: Vec<String>,
    pub revisit_triggers: Vec<String>,
    pub source_basis: Vec<SourceId>,
    pub uncertainty_and_limits: Vec<String>,
    pub review_basis: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BriefContextItem {
    pub identity: ContextItemId,
    pub role: ContextItemRole,
    pub statement: String,
    pub source_basis: Vec<SourceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BriefQuestion {
    pub question_id: QuestionId,
    pub revision: u64,
    pub prompt: String,
    pub on_current_frontier: bool,
    pub blocked_basis: Vec<String>,
    pub what_the_answer_unlocks: Vec<String>,
    pub source_basis: Vec<SourceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BriefSnapshot {
    pub analysis_snapshot: AnalysisSnapshotId,
    pub repository_snapshot: RepositorySnapshotId,
    pub freshness: FreshnessBasis,
    pub capabilities: Vec<CapabilityReport>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OmissionReason {
    Bound,
    Scope,
    SupersededHistory,
    UnavailableBasis,
    FailedBasis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecallOmission {
    pub identity: String,
    pub kind: String,
    pub reason: OmissionReason,
    pub expandable_basis: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecallProposal {
    pub kind: String,
    pub basis: String,
    pub source_ids: Vec<SourceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeBrief {
    pub project_id: ProjectId,
    pub project_name: String,
    pub goals_and_why: Vec<BriefContextItem>,
    pub decisions: Vec<BriefDecision>,
    pub latest_meaningful_checkpoint: Option<Checkpoint>,
    pub open_questions: Vec<BriefQuestion>,
    pub risks_assumptions_and_limits: Vec<BriefContextItem>,
    pub declared_assumptions: Vec<String>,
    pub known_limits: Vec<String>,
    pub next_meaningful_step: Option<String>,
    pub used_sources: Vec<SourceReadBasis>,
    pub snapshots: Vec<BriefSnapshot>,
    pub omissions: Vec<RecallOmission>,
    pub omitted_count: usize,
    pub proposals: Vec<RecallProposal>,
}

/// Builds a deterministic, bounded, read-only resumption view. It accepts no
/// Kernel, CandidateStore, or analyzer mutation handle.
pub fn build_resume_brief(inputs: RecallInputs<'_>) -> ResumeBrief {
    let canonical = inputs.canonical;
    let limit = inputs.bound.max_items_per_section.max(1);
    let mut omissions = Vec::new();

    let mut goals = canonical
        .context_items
        .iter()
        .filter(|item| item.role == ContextItemRole::Goal)
        .map(|item| BriefContextItem {
            identity: item.id,
            role: item.role,
            statement: item.statement.clone(),
            source_basis: item.source_basis.clone(),
        })
        .collect::<Vec<_>>();
    goals.sort_by_key(|item| item.identity);
    bound_items(
        &mut goals,
        limit,
        "context_goal",
        |item| item.identity.to_string(),
        &mut omissions,
    );

    let mut decisions = canonical
        .active_decisions
        .iter()
        .chain(canonical.superseded_decisions.iter())
        .map(|lifecycle| {
            let applicability =
                evaluate_decision_applicability(canonical, lifecycle, &inputs.scope);
            let state = match applicability.state {
                DecisionApplicabilityState::ReusableCurrent => BriefDecisionState::Current,
                DecisionApplicabilityState::ReviewRequiredUncertain => {
                    if applicability.issues.iter().any(|issue| {
                        matches!(issue, volicord_inquiry::ApplicabilityIssue::SourceStale(_))
                    }) {
                        BriefDecisionState::StaleBasis
                    } else {
                        BriefDecisionState::ReviewRequired
                    }
                }
                DecisionApplicabilityState::Superseded => BriefDecisionState::Superseded,
                DecisionApplicabilityState::UnavailableBasis => {
                    BriefDecisionState::UnavailableBasis
                }
            };
            let uncertainty_and_limits = applicability
                .displayed_basis
                .as_ref()
                .map(|basis| {
                    basis
                        .uncertainty
                        .iter()
                        .chain(basis.known_limits.iter())
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            BriefDecision {
                decision_id: lifecycle.decision.id,
                revision: lifecycle.decision.revision,
                state,
                choice: lifecycle.decision.choice.clone(),
                user_rationale: lifecycle.decision.user_rationale.clone(),
                recommendation_rationale: lifecycle
                    .decision
                    .displayed_recommendation
                    .rationale
                    .clone(),
                assumptions: lifecycle.decision.assumptions.clone(),
                revisit_triggers: lifecycle.decision.revisit_triggers.clone(),
                source_basis: applicability.source_basis,
                uncertainty_and_limits,
                review_basis: applicability
                    .issues
                    .iter()
                    .map(|issue| format!("{issue:?}"))
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    decisions.sort_by_key(|item| (decision_state_priority(item.state), item.decision_id));
    bound_items(
        &mut decisions,
        limit,
        "decision",
        |item| item.decision_id.to_string(),
        &mut omissions,
    );

    let inquiry_scope = InquiryScope {
        project_id: inputs.scope.project_id,
        material_scope: inputs
            .scope
            .paths
            .iter()
            .chain(inputs.scope.components.iter())
            .chain(inputs.scope.work_contexts.iter())
            .cloned()
            .collect(),
    };
    let frontier = compute_frontier(canonical, &inquiry_scope);
    let frontier_ids = frontier
        .questions
        .iter()
        .map(|question| question.question_id)
        .collect::<BTreeSet<_>>();
    let diagnostics = frontier.diagnostics.iter().fold(
        BTreeMap::<QuestionId, Vec<String>>::new(),
        |mut grouped, diagnostic| {
            grouped
                .entry(diagnostic.question_id)
                .or_default()
                .push(diagnostic.detail.clone());
            grouped
        },
    );
    let mut questions = canonical
        .active_questions
        .iter()
        .map(|question| BriefQuestion {
            question_id: question.id,
            revision: question.revision,
            prompt: question.prompt_basis.clone(),
            on_current_frontier: frontier_ids.contains(&question.id),
            blocked_basis: diagnostics.get(&question.id).cloned().unwrap_or_default(),
            what_the_answer_unlocks: question.what_the_answer_unlocks.clone(),
            source_basis: question.source_basis.clone(),
        })
        .collect::<Vec<_>>();
    questions.sort_by_key(|question| (!question.on_current_frontier, question.question_id));
    bound_items(
        &mut questions,
        limit,
        "open_question",
        |item| item.question_id.to_string(),
        &mut omissions,
    );

    let mut risks = canonical
        .context_items
        .iter()
        .filter(|item| {
            matches!(
                item.role,
                ContextItemRole::Risk | ContextItemRole::Assumption | ContextItemRole::KnownLimit
            )
        })
        .map(|item| BriefContextItem {
            identity: item.id,
            role: item.role,
            statement: item.statement.clone(),
            source_basis: item.source_basis.clone(),
        })
        .collect::<Vec<_>>();
    risks.sort_by_key(|item| item.identity);
    bound_items(
        &mut risks,
        limit,
        "risk_assumption_or_limit",
        |item| item.identity.to_string(),
        &mut omissions,
    );

    let mut snapshots = inputs
        .analyses
        .iter()
        .filter(|analysis| analysis.project.identity() == canonical.project.id)
        .map(|analysis| BriefSnapshot {
            analysis_snapshot: analysis.identity,
            repository_snapshot: analysis.repository_snapshot,
            freshness: analysis.freshness.clone(),
            capabilities: analysis.capabilities.clone(),
        })
        .collect::<Vec<_>>();
    snapshots.sort_by_key(|snapshot| snapshot.analysis_snapshot);
    bound_items(
        &mut snapshots,
        limit,
        "analysis_snapshot",
        |item| item.analysis_snapshot.to_string(),
        &mut omissions,
    );

    let mut used_source_ids = BTreeSet::new();
    for source in goals
        .iter()
        .flat_map(|item| item.source_basis.iter())
        .chain(decisions.iter().flat_map(|item| item.source_basis.iter()))
        .chain(questions.iter().flat_map(|item| item.source_basis.iter()))
        .chain(risks.iter().flat_map(|item| item.source_basis.iter()))
    {
        used_source_ids.insert(*source);
    }
    if let Some(checkpoint) = &canonical.latest_checkpoint {
        used_source_ids.extend(checkpoint.source_basis.iter().copied());
        used_source_ids.extend(checkpoint.changed_source_basis.iter().copied());
    }
    for snapshot in inputs.analyses {
        if snapshot.project.identity() == canonical.project.id {
            used_source_ids.insert(snapshot.repository_source.identity());
        }
    }
    let mut used_sources = canonical
        .sources
        .iter()
        .filter(|source| used_source_ids.contains(&source.source.id))
        .cloned()
        .collect::<Vec<_>>();
    used_sources.sort_by_key(|source| source.source.id);
    bound_items(
        &mut used_sources,
        limit,
        "source",
        |item| item.source.id.to_string(),
        &mut omissions,
    );

    for lifecycle in &canonical.superseded_decisions {
        if !decisions
            .iter()
            .any(|decision| decision.decision_id == lifecycle.decision.id)
        {
            omissions.push(RecallOmission {
                identity: lifecycle.decision.id.to_string(),
                kind: "decision".to_owned(),
                reason: OmissionReason::SupersededHistory,
                expandable_basis: "inspect superseded Decision history".to_owned(),
            });
        }
    }
    let source_freshness = canonical
        .sources
        .iter()
        .map(|source| (source.source.id, source.freshness))
        .collect::<BTreeMap<_, _>>();
    let unavailable = used_source_ids
        .iter()
        .filter(|source| !matches!(source_freshness.get(source), Some(SourceFreshness::Current)))
        .copied()
        .collect::<Vec<_>>();
    let proposals = if unavailable.is_empty() {
        Vec::new()
    } else {
        vec![RecallProposal {
            kind: "source_investigation".to_owned(),
            basis: "Recall encountered stale, unavailable, or unknown Source basis".to_owned(),
            source_ids: unavailable,
        }]
    };
    omissions.sort_by(|left, right| {
        (&left.kind, &left.identity, omission_priority(left.reason)).cmp(&(
            &right.kind,
            &right.identity,
            omission_priority(right.reason),
        ))
    });
    omissions.dedup_by(|left, right| {
        left.kind == right.kind && left.identity == right.identity && left.reason == right.reason
    });
    let next_meaningful_step = canonical
        .latest_checkpoint
        .as_ref()
        .map(|checkpoint| checkpoint.next_step.clone())
        .or_else(|| {
            questions
                .first()
                .and_then(|question| question.what_the_answer_unlocks.first().cloned())
        });
    let mut declared_assumptions = decisions
        .iter()
        .flat_map(|decision| decision.assumptions.iter().cloned())
        .chain(
            canonical
                .context_items
                .iter()
                .filter(|item| item.role == ContextItemRole::Assumption)
                .map(|item| item.statement.clone()),
        )
        .collect::<Vec<_>>();
    declared_assumptions.sort();
    declared_assumptions.dedup();
    let mut known_limits = decisions
        .iter()
        .flat_map(|decision| decision.uncertainty_and_limits.iter().cloned())
        .chain(
            canonical
                .latest_checkpoint
                .iter()
                .flat_map(|checkpoint| checkpoint.known_limits.iter().cloned()),
        )
        .chain(
            canonical
                .context_items
                .iter()
                .filter(|item| item.role == ContextItemRole::KnownLimit)
                .map(|item| item.statement.clone()),
        )
        .collect::<Vec<_>>();
    known_limits.sort();
    known_limits.dedup();
    let omitted_count = omissions.len();
    ResumeBrief {
        project_id: canonical.project.id,
        project_name: canonical.project.display_name.clone(),
        goals_and_why: goals,
        decisions,
        latest_meaningful_checkpoint: canonical.latest_checkpoint.clone(),
        open_questions: questions,
        risks_assumptions_and_limits: risks,
        declared_assumptions,
        known_limits,
        next_meaningful_step,
        used_sources,
        snapshots,
        omissions,
        omitted_count,
        proposals,
    }
}

fn bound_items<T>(
    values: &mut Vec<T>,
    limit: usize,
    kind: &str,
    identity: impl Fn(&T) -> String,
    omissions: &mut Vec<RecallOmission>,
) {
    if values.len() <= limit {
        return;
    }
    omissions.extend(values[limit..].iter().map(|item| RecallOmission {
        identity: identity(item),
        kind: kind.to_owned(),
        reason: OmissionReason::Bound,
        expandable_basis: format!("expand {kind} by identity"),
    }));
    values.truncate(limit);
}

const fn decision_state_priority(state: BriefDecisionState) -> u8 {
    match state {
        BriefDecisionState::Current => 0,
        BriefDecisionState::StaleBasis => 1,
        BriefDecisionState::ReviewRequired => 2,
        BriefDecisionState::UnavailableBasis => 3,
        BriefDecisionState::Superseded => 4,
    }
}

const fn omission_priority(reason: OmissionReason) -> u8 {
    match reason {
        OmissionReason::Bound => 0,
        OmissionReason::Scope => 1,
        OmissionReason::SupersededHistory => 2,
        OmissionReason::UnavailableBasis => 3,
        OmissionReason::FailedBasis => 4,
    }
}
