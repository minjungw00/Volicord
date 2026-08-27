use crate::{
    CandidateCollectionMode, CandidateCollectionScope, CandidateContent, CandidateDraft,
    CandidateFreshness, CandidateKind, CandidateObservationBasis, CandidateOrigin,
    CandidateRetention, CandidateStore, DuplicateAssessment, MaterialityAssessment,
    MaterialityStatus, QuestionCandidate, SubmissionOutcome,
};
use std::collections::{BTreeMap, BTreeSet};
use volicord_context::{
    CanonicalReadBasis, DecisionId, DecisionLifecycle, OperationId, Principal, ProjectId,
    QuestionAlternative, QuestionEstablishedFact, QuestionEvidenceFreshness, QuestionResearchState,
    ReviewDue, ReviewDueDraft, ReviewDueKind, SourceFreshness, SourceId, Store, TimestampMicros,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicabilityQuery {
    pub project_id: ProjectId,
    pub paths: Vec<String>,
    pub components: Vec<String>,
    pub work_contexts: Vec<String>,
    pub current_assumptions: Vec<String>,
    pub met_revisit_triggers: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ApplicabilityIssue {
    WrongProject,
    ScopeMismatch,
    AssumptionChanged(String),
    SourceStale(SourceId),
    SourceUnavailable(SourceId),
    SourceUnknown(SourceId),
    RevisitTriggerMet(String),
    Contradiction,
    ExistingReviewDue,
    Superseded,
    MissingQuestionBasis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionApplicabilityState {
    ReusableCurrent,
    ReviewRequiredUncertain,
    Superseded,
    UnavailableBasis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionApplicability {
    pub decision_id: DecisionId,
    pub state: DecisionApplicabilityState,
    pub issues: Vec<ApplicabilityIssue>,
    pub source_basis: Vec<SourceId>,
    pub displayed_basis: Option<DecisionBasisSummary>,
    pub existing_review_due: Option<ReviewDue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionBasisSummary {
    pub alternatives: Vec<QuestionAlternative>,
    pub recommendation_rationale: String,
    pub user_rationale: Option<String>,
    pub expected_consequences: Vec<String>,
    pub uncertainty: Vec<String>,
    pub known_limits: Vec<String>,
    pub revisit_triggers: Vec<String>,
}

/// Evaluates Decision reuse without changing the Decision or review state.
pub fn evaluate_decision_applicability(
    canonical: &CanonicalReadBasis,
    lifecycle: &DecisionLifecycle,
    query: &ApplicabilityQuery,
) -> DecisionApplicability {
    let decision = &lifecycle.decision;
    let mut issues = BTreeSet::new();
    if canonical.project.id != query.project_id || decision.project_id != query.project_id {
        issues.insert(ApplicabilityIssue::WrongProject);
    }
    if lifecycle.superseded_by.is_some() {
        issues.insert(ApplicabilityIssue::Superseded);
    }
    if !scope_matches(&decision.applicability.paths, &query.paths, path_matches)
        || !scope_matches(
            &decision.applicability.components,
            &query.components,
            exact_matches,
        )
        || !scope_matches(
            &decision.applicability.work_contexts,
            &query.work_contexts,
            exact_matches,
        )
    {
        issues.insert(ApplicabilityIssue::ScopeMismatch);
    }
    for assumption in &decision.assumptions {
        if !query.current_assumptions.contains(assumption) {
            issues.insert(ApplicabilityIssue::AssumptionChanged(assumption.clone()));
        }
    }
    for trigger in &decision.revisit_triggers {
        if query.met_revisit_triggers.contains(trigger) {
            issues.insert(ApplicabilityIssue::RevisitTriggerMet(trigger.clone()));
        }
    }
    if !lifecycle.contradictions.is_empty() {
        issues.insert(ApplicabilityIssue::Contradiction);
    }
    if lifecycle.review_due.is_some() {
        issues.insert(ApplicabilityIssue::ExistingReviewDue);
    }

    let question = canonical
        .active_questions
        .iter()
        .chain(canonical.terminal_question_history.iter())
        .find(|question| {
            question.id == decision.question_id && question.revision >= decision.question_revision
        });
    if question.is_none() {
        issues.insert(ApplicabilityIssue::MissingQuestionBasis);
    }
    let mut source_basis = vec![decision.user_turn_source_id];
    source_basis.extend(
        decision
            .displayed_recommendation
            .source_basis
            .iter()
            .copied(),
    );
    if let Some(question) = question {
        source_basis.extend(question.source_basis.iter().copied());
        for fact in &question.established_facts {
            source_basis.extend(fact.source_basis.iter().copied());
        }
    }
    source_basis.sort_unstable();
    source_basis.dedup();
    let freshness = canonical
        .sources
        .iter()
        .map(|basis| (basis.source.id, basis.freshness))
        .collect::<BTreeMap<_, _>>();
    for source in &source_basis {
        match freshness.get(source).copied() {
            Some(SourceFreshness::Current) => {}
            Some(SourceFreshness::Stale) => {
                issues.insert(ApplicabilityIssue::SourceStale(*source));
            }
            Some(SourceFreshness::Unavailable) | None => {
                issues.insert(ApplicabilityIssue::SourceUnavailable(*source));
            }
            Some(SourceFreshness::Unknown) => {
                issues.insert(ApplicabilityIssue::SourceUnknown(*source));
            }
        }
    }
    let state = if issues.contains(&ApplicabilityIssue::Superseded) {
        DecisionApplicabilityState::Superseded
    } else if issues.iter().any(|issue| {
        matches!(
            issue,
            ApplicabilityIssue::SourceUnavailable(_)
                | ApplicabilityIssue::WrongProject
                | ApplicabilityIssue::MissingQuestionBasis
        )
    }) {
        DecisionApplicabilityState::UnavailableBasis
    } else if issues.is_empty() {
        DecisionApplicabilityState::ReusableCurrent
    } else {
        DecisionApplicabilityState::ReviewRequiredUncertain
    };
    let displayed_basis = question.map(|question| DecisionBasisSummary {
        alternatives: decision.displayed_alternatives.clone(),
        recommendation_rationale: decision.displayed_recommendation.rationale.clone(),
        user_rationale: decision.user_rationale.clone(),
        expected_consequences: decision
            .displayed_alternatives
            .iter()
            .map(|alternative| alternative.consequence.clone())
            .collect(),
        uncertainty: question.uncertainty.clone(),
        known_limits: question.known_limits.clone(),
        revisit_triggers: decision.revisit_triggers.clone(),
    });
    DecisionApplicability {
        decision_id: decision.id,
        state,
        issues: issues.into_iter().collect(),
        source_basis,
        displayed_basis,
        existing_review_due: lifecycle.review_due.clone(),
    }
}

fn scope_matches(
    declared: &[String],
    requested: &[String],
    item_matches: fn(&str, &str) -> bool,
) -> bool {
    declared.is_empty()
        || (!requested.is_empty()
            && requested
                .iter()
                .all(|item| declared.iter().any(|scope| item_matches(scope, item))))
}

fn exact_matches(declared: &str, requested: &str) -> bool {
    declared == requested
}

fn path_matches(declared: &str, requested: &str) -> bool {
    declared == requested
        || requested
            .strip_prefix(declared)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewDueIntent {
    pub operation_id: OperationId,
    pub project_id: ProjectId,
    pub decision_id: DecisionId,
    pub kind: ReviewDueKind,
    pub explanation: String,
    pub source_basis: Vec<SourceId>,
}

/// Routes an explicit review intent to the canonical Kernel. The evaluator
/// above remains mutation-free.
pub fn mark_review_due(
    context: &mut Store,
    intent: ReviewDueIntent,
) -> Result<volicord_context::OperationResult<ReviewDue>, volicord_context::Error> {
    context.mark_decision_review_due(
        intent.operation_id,
        intent.project_id,
        intent.decision_id,
        ReviewDueDraft {
            kind: intent.kind,
            explanation: intent.explanation,
            source_basis: intent.source_basis,
        },
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestioningProposal {
    pub session: String,
    pub source_operation: String,
    pub observed_at: TimestampMicros,
    pub retention: CandidateRetention,
    pub actor: Principal,
    pub review_explanation: String,
}

/// Creates a Question Candidate for evidence-driven review. It never creates
/// a replacement canonical Question or Decision.
pub fn propose_requestioning(
    candidates: &mut CandidateStore,
    canonical: &CanonicalReadBasis,
    applicability: &DecisionApplicability,
    proposal: RequestioningProposal,
) -> Result<SubmissionOutcome, crate::Error> {
    if !matches!(
        applicability.state,
        DecisionApplicabilityState::ReviewRequiredUncertain
            | DecisionApplicabilityState::UnavailableBasis
    ) {
        return Err(crate::Error::new(
            crate::ErrorKind::DomainConflict,
            "only a Decision with inspectable review basis can be re-questioned",
        ));
    }
    let lifecycle = canonical
        .active_decisions
        .iter()
        .chain(canonical.superseded_decisions.iter())
        .find(|item| item.decision.id == applicability.decision_id)
        .ok_or_else(|| crate::Error::new(crate::ErrorKind::NotFound, "Decision was not found"))?;
    let question = canonical
        .active_questions
        .iter()
        .chain(canonical.terminal_question_history.iter())
        .find(|question| question.id == lifecycle.decision.question_id)
        .ok_or_else(|| {
            crate::Error::new(
                crate::ErrorKind::StaleBasis,
                "Decision Question basis is unavailable",
            )
        })?;
    let source_basis = applicability.source_basis.clone();
    if source_basis.is_empty() {
        return Err(crate::Error::new(
            crate::ErrorKind::StaleBasis,
            "review proposal requires canonical Source basis",
        ));
    }
    let facts = question
        .established_facts
        .iter()
        .cloned()
        .chain(std::iter::once(QuestionEstablishedFact {
            statement: proposal.review_explanation.clone(),
            source_basis: source_basis.clone(),
            capability: None,
            freshness: QuestionEvidenceFreshness::Unknown,
        }))
        .collect();
    candidates.submit(CandidateDraft {
        project_id: canonical.project.id,
        kind: CandidateKind::QuestionCandidate,
        collection_mode: CandidateCollectionMode::ExplicitUserDirected,
        origin: CandidateOrigin {
            actor: proposal.actor.clone(),
            subsystem: "inquiry".to_owned(),
            session: Some(proposal.session.clone()),
            provenance_summary: "Decision applicability review proposal".to_owned(),
        },
        collection_scope: CandidateCollectionScope {
            project_id: canonical.project.id,
            session: Some(proposal.session),
            source_operation: Some(proposal.source_operation),
            candidate_kind: CandidateKind::QuestionCandidate,
        },
        observation_basis: CandidateObservationBasis {
            source_basis: source_basis.clone(),
            other: Some(proposal.review_explanation.clone()),
            ..CandidateObservationBasis::default()
        },
        observed_at: proposal.observed_at,
        retention: proposal.retention,
        content: CandidateContent {
            bounded_summary: proposal.review_explanation.clone(),
            question: Some(QuestionCandidate {
                prompt_basis: format!("Review Decision {} for current applicability", lifecycle.decision.id),
                known_facts: facts,
                assumptions: lifecycle.decision.assumptions.clone(),
                uncertainty: applicability
                    .issues
                    .iter()
                    .map(|issue| format!("{issue:?}"))
                    .collect(),
                affected_scope: question.material_scope.clone(),
                possible_prerequisites: Vec::new(),
                source_basis: source_basis.clone(),
                repository_basis: Vec::new(),
                freshness: CandidateFreshness::Unknown,
                duplicate_assessment: DuplicateAssessment::NoDuplicate {
                    basis: "review is linked to an existing terminal Decision, not a duplicate open Question"
                        .to_owned(),
                },
                materiality: MaterialityAssessment {
                    status: MaterialityStatus::Material,
                    rationale: Some(proposal.review_explanation),
                    source_basis,
                    assessed_by: Some(proposal.actor),
                    assessed_at: Some(proposal.observed_at),
                },
                presentation_order: Some(question.presentation_order),
                why_it_matters_now: "the prior Decision has an inspectable changed or unavailable basis"
                    .to_owned(),
                alternatives: lifecycle.decision.displayed_alternatives.clone(),
                recommendation: lifecycle.decision.displayed_recommendation.clone(),
                trade_offs: question.trade_offs.clone(),
                known_limits: question.known_limits.clone(),
                what_the_answer_unlocks: question.what_the_answer_unlocks.clone(),
                allowed_non_choice_dispositions: question.allowed_non_choice_dispositions.clone(),
                research_state: QuestionResearchState::ReadyToAsk,
            }),
            materiality_review: None,
        },
    })
}
