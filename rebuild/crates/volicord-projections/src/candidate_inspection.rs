use volicord_context::TimestampMicros;
use volicord_inquiry::{
    CandidateCleanup, CandidateDisposition, CandidateId, CandidateReadBasis, CandidateRecord,
    CollectionOptOut, CollectionOptOutScope, EngineeringChoiceDiscovery,
    EngineeringChoiceEvidenceState, EngineeringEffectCategory, ExplicitDelegationEvidence,
    LearningAlternativeSelection, LearningDeliberation, LearningDeliberationState,
    LearningInitialResponse, LearningRecommendation, MaterialityReview,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateContentAccess {
    AllowBoundedSummary,
    PolicyWithheld,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InspectionHealth {
    Complete,
    Partial,
    Degraded,
    NotFound,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateContentOmission {
    PolicyWithheld,
    CanonicalForgettingPending,
    RetentionCleaned,
    ContentUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetentionInspection {
    RetainedIndefinitely {
        basis: String,
    },
    RetainedUntil {
        retained_until: TimestampMicros,
        expired_at_observation: bool,
        basis: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplicitDelegationInspection {
    pub dimension_id: String,
    pub evidence: ExplicitDelegationEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LearningExplanationAvailability {
    Available,
    Degraded,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearningExplanationAlternative {
    pub choice_id: String,
    pub choice_summary: String,
    pub alternative_id: String,
    pub alternative_summary: String,
    pub technical_consequences: Vec<String>,
    pub affected_scope: Vec<String>,
    pub effect_categories: Vec<EngineeringEffectCategory>,
    pub evidence_state: EngineeringChoiceEvidenceState,
    pub source_basis: Vec<volicord_context::SourceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LearningSelectionOutcome {
    NotRecorded,
    Selected {
        selections: Vec<LearningAlternativeSelection>,
        completed: bool,
    },
    Delegated,
    Skipped,
    ResearchOrPrototypeRequired {
        evidence_state: EngineeringChoiceEvidenceState,
    },
    ReconsiderationRequested,
}

/// Read-side material retained for an agent to explain what a learning
/// interaction established. Availability is independent from participation,
/// selection, canonical authority, and the quality of prose an agent produces.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearningExplanationBasis {
    pub availability: LearningExplanationAvailability,
    pub availability_reasons: Vec<String>,
    pub problem: Option<String>,
    pub established_facts: Vec<String>,
    pub alternatives: Vec<LearningExplanationAlternative>,
    pub affected_scope: Vec<String>,
    pub source_basis: Vec<volicord_context::SourceId>,
    pub analysis_snapshot_id: Option<volicord_repository_intelligence::AnalysisSnapshotId>,
    pub selection_outcome: LearningSelectionOutcome,
    pub latest_user_rationale: Option<String>,
    pub latest_agent_feedback: Option<String>,
    pub latest_agent_recommendation: Option<LearningRecommendation>,
    pub remaining_uncertainty: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateInspection {
    pub candidate_id: CandidateId,
    pub exists: bool,
    pub health: InspectionHealth,
    pub revision: Option<u64>,
    pub kind: Option<volicord_inquiry::CandidateKind>,
    pub origin: Option<volicord_inquiry::CandidateOrigin>,
    pub collection_scope: Option<volicord_inquiry::CandidateCollectionScope>,
    pub observation_basis: Option<volicord_inquiry::CandidateObservationBasis>,
    pub created_at: Option<TimestampMicros>,
    pub observed_at: Option<TimestampMicros>,
    pub retention: Option<RetentionInspection>,
    pub promotion_disposition: Option<CandidateDisposition>,
    pub promotion_target: Option<volicord_context::QuestionId>,
    pub content_cleaned: bool,
    pub cleanup: Option<CandidateCleanup>,
    pub current_applicable_opt_out: Vec<CollectionOptOut>,
    pub bounded_summary: Option<String>,
    pub question_research_state: Option<volicord_context::QuestionResearchState>,
    pub repository_research_basis: Vec<volicord_inquiry::RepositoryResearchBasis>,
    pub explicit_delegation_evidence: Vec<ExplicitDelegationInspection>,
    pub engineering_choice_discovery: Option<EngineeringChoiceDiscovery>,
    pub materiality_review: Option<MaterialityReview>,
    pub learning_deliberation: Option<LearningDeliberation>,
    pub learning_explanation_basis: Option<LearningExplanationBasis>,
    pub content_omission: Option<CandidateContentOmission>,
}

/// Reads one named Candidate from an owned immutable basis. No store or
/// lifecycle handle is accepted, so inspection and failure cannot mutate it.
pub fn inspect_candidate(
    basis: &CandidateReadBasis,
    candidate_id: CandidateId,
    content_access: CandidateContentAccess,
    observed_at: TimestampMicros,
) -> CandidateInspection {
    let Some(candidate) = basis
        .candidates
        .iter()
        .find(|candidate| candidate.id == candidate_id)
    else {
        return CandidateInspection {
            candidate_id,
            exists: false,
            health: InspectionHealth::NotFound,
            revision: None,
            kind: None,
            origin: None,
            collection_scope: None,
            observation_basis: None,
            created_at: None,
            observed_at: None,
            retention: None,
            promotion_disposition: None,
            promotion_target: None,
            content_cleaned: false,
            cleanup: None,
            current_applicable_opt_out: Vec::new(),
            bounded_summary: None,
            question_research_state: None,
            repository_research_basis: Vec::new(),
            explicit_delegation_evidence: Vec::new(),
            engineering_choice_discovery: None,
            materiality_review: None,
            learning_deliberation: None,
            learning_explanation_basis: None,
            content_omission: None,
        };
    };
    inspect_existing(basis, candidate, content_access, observed_at)
}

fn inspect_existing(
    basis: &CandidateReadBasis,
    candidate: &CandidateRecord,
    content_access: CandidateContentAccess,
    observed_at: TimestampMicros,
) -> CandidateInspection {
    let current_applicable_opt_out = basis
        .collection_policies
        .iter()
        .filter(|policy| scope_matches(&policy.scope, &candidate.collection_scope))
        .cloned()
        .collect();
    let cleaned = candidate.cleanup.is_some();
    let forgetting_pending = basis
        .withheld_for_canonical_forgetting
        .contains(&candidate.id);
    let retention = if let Some(retained_until) = candidate.retention.retained_until {
        RetentionInspection::RetainedUntil {
            retained_until,
            expired_at_observation: retained_until <= observed_at,
            basis: candidate.retention.basis.clone(),
        }
    } else {
        RetentionInspection::RetainedIndefinitely {
            basis: candidate.retention.basis.clone(),
        }
    };
    let (health, bounded_summary, content_omission) = if forgetting_pending {
        (
            InspectionHealth::Degraded,
            None,
            Some(CandidateContentOmission::CanonicalForgettingPending),
        )
    } else {
        match content_access {
            CandidateContentAccess::PolicyWithheld => (
                InspectionHealth::Partial,
                None,
                Some(CandidateContentOmission::PolicyWithheld),
            ),
            CandidateContentAccess::AllowBoundedSummary => match candidate.content.as_ref() {
                Some(content) => (
                    InspectionHealth::Complete,
                    Some(content.bounded_summary.clone()),
                    None,
                ),
                None if cleaned => (
                    InspectionHealth::Partial,
                    None,
                    Some(CandidateContentOmission::RetentionCleaned),
                ),
                None => (
                    InspectionHealth::Degraded,
                    None,
                    Some(CandidateContentOmission::ContentUnavailable),
                ),
            },
        }
    };
    let (question_research_state, repository_research_basis) = if forgetting_pending {
        (None, Vec::new())
    } else {
        match content_access {
            CandidateContentAccess::AllowBoundedSummary => candidate
                .content
                .as_ref()
                .and_then(|content| content.question.as_ref())
                .map_or_else(
                    || (None, Vec::new()),
                    |question| {
                        (
                            Some(question.research_state),
                            question.repository_basis.clone(),
                        )
                    },
                ),
            CandidateContentAccess::PolicyWithheld => (None, Vec::new()),
        }
    };
    let explicit_delegation_evidence = if forgetting_pending {
        Vec::new()
    } else {
        match content_access {
            CandidateContentAccess::AllowBoundedSummary => candidate
                .content
                .as_ref()
                .and_then(|content| content.materiality_review.as_ref())
                .map(|review| {
                    review
                        .dimensions
                        .iter()
                        .filter_map(|dimension| {
                            dimension
                                .basis
                                .explicit_delegation
                                .as_ref()
                                .map(|evidence| ExplicitDelegationInspection {
                                    dimension_id: dimension.dimension_id.clone(),
                                    evidence: evidence.clone(),
                                })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            CandidateContentAccess::PolicyWithheld => Vec::new(),
        }
    };
    let (engineering_choice_discovery, materiality_review, learning_deliberation) =
        if forgetting_pending || matches!(content_access, CandidateContentAccess::PolicyWithheld) {
            (None, None, None)
        } else {
            candidate
                .content
                .as_ref()
                .map_or((None, None, None), |content| {
                    (
                        content.engineering_choice_discovery.clone(),
                        content.materiality_review.clone(),
                        content.learning_deliberation.clone(),
                    )
                })
        };
    let learning_explanation_basis =
        (candidate.kind == volicord_inquiry::CandidateKind::LearningDeliberation).then(|| {
            learning_deliberation.as_ref().map_or_else(
                || unavailable_learning_explanation_basis(content_omission.as_ref()),
                |deliberation| {
                    build_learning_explanation_basis(
                        deliberation,
                        &candidate.observation_basis.source_basis,
                    )
                },
            )
        });
    CandidateInspection {
        candidate_id: candidate.id,
        exists: true,
        health,
        revision: Some(candidate.revision),
        kind: Some(candidate.kind),
        origin: Some(candidate.origin.clone()),
        collection_scope: Some(candidate.collection_scope.clone()),
        observation_basis: Some(candidate.observation_basis.clone()),
        created_at: Some(candidate.created_at),
        observed_at: Some(candidate.observed_at),
        retention: Some(retention),
        promotion_disposition: Some(candidate.disposition.clone()),
        promotion_target: candidate.promotion_target,
        content_cleaned: cleaned,
        cleanup: candidate.cleanup.clone(),
        current_applicable_opt_out,
        bounded_summary,
        question_research_state,
        repository_research_basis,
        explicit_delegation_evidence,
        engineering_choice_discovery,
        materiality_review,
        learning_deliberation,
        learning_explanation_basis,
        content_omission,
    }
}

pub fn build_learning_explanation_basis(
    deliberation: &LearningDeliberation,
    additional_source_basis: &[volicord_context::SourceId],
) -> LearningExplanationBasis {
    let mut availability_reasons = Vec::new();
    if deliberation.problem.trim().is_empty() {
        availability_reasons.push("the retained problem statement is empty".to_owned());
    }
    if deliberation.established_facts.is_empty() {
        availability_reasons.push("no established facts are retained".to_owned());
    }
    if deliberation.choices.is_empty() {
        availability_reasons.push("no source-grounded alternatives are retained".to_owned());
    }
    let mut alternatives = Vec::new();
    let mut source_basis = additional_source_basis.to_vec();
    let mut remaining_uncertainty = Vec::new();
    for choice in &deliberation.choices {
        source_basis.extend(choice.source_basis.iter().copied());
        if choice.source_basis.is_empty() {
            availability_reasons.push(format!(
                "choice `{}` has no retained Source basis",
                choice.choice_id
            ));
        }
        if choice.alternatives.is_empty() {
            availability_reasons.push(format!(
                "choice `{}` has no retained alternatives",
                choice.choice_id
            ));
        }
        if !matches!(
            choice.evidence_state,
            EngineeringChoiceEvidenceState::Sufficient
        ) {
            remaining_uncertainty.push(format!(
                "choice `{}` requires {:?}",
                choice.choice_id, choice.evidence_state
            ));
        }
        alternatives.extend(choice.alternatives.iter().map(|alternative| {
            LearningExplanationAlternative {
                choice_id: choice.choice_id.clone(),
                choice_summary: choice.summary.clone(),
                alternative_id: alternative.alternative_id.clone(),
                alternative_summary: alternative.summary.clone(),
                technical_consequences: alternative.technical_consequences.clone(),
                affected_scope: choice.affected_scope.clone(),
                effect_categories: choice.effect_categories.clone(),
                evidence_state: choice.evidence_state,
                source_basis: choice.source_basis.clone(),
            }
        }));
    }
    let latest_round = deliberation.rounds.last();
    if let Some(round) = latest_round {
        source_basis.push(round.initial_response_source_id);
        source_basis.extend(round.reconsideration_source_id);
    }
    source_basis.sort_unstable();
    source_basis.dedup();
    if source_basis.is_empty() {
        availability_reasons.push("no Source basis is retained for this explanation".to_owned());
    }
    let selection_outcome = learning_selection_outcome(deliberation, latest_round);
    match deliberation.state {
        LearningDeliberationState::AwaitingInitialResponse => {
            remaining_uncertainty.push("the learner selection is not recorded".to_owned());
        }
        LearningDeliberationState::AwaitingAgentFeedback { .. } => {
            remaining_uncertainty
                .push("agent feedback and implication are not recorded".to_owned());
        }
        LearningDeliberationState::ResearchOrPrototypeRequired { .. } => {
            remaining_uncertainty
                .push("requested research or prototype evidence is not yet resolved".to_owned());
        }
        LearningDeliberationState::ReconsiderationRequested { .. } => {
            remaining_uncertainty
                .push("the prior learning selection is under reconsideration".to_owned());
        }
        LearningDeliberationState::FeedbackProvided { .. }
        | LearningDeliberationState::Completed { .. }
        | LearningDeliberationState::Delegated { .. }
        | LearningDeliberationState::Skipped { .. } => {}
    }
    remaining_uncertainty.sort();
    remaining_uncertainty.dedup();
    availability_reasons.sort();
    availability_reasons.dedup();
    LearningExplanationBasis {
        availability: if availability_reasons.is_empty() {
            LearningExplanationAvailability::Available
        } else {
            LearningExplanationAvailability::Degraded
        },
        availability_reasons,
        problem: Some(deliberation.problem.clone()),
        established_facts: deliberation.established_facts.clone(),
        alternatives,
        affected_scope: deliberation.affected_scope.clone(),
        source_basis,
        analysis_snapshot_id: Some(deliberation.baseline_analysis_snapshot_id),
        selection_outcome,
        latest_user_rationale: latest_round.and_then(|round| round.user_rationale.clone()),
        latest_agent_feedback: latest_round.and_then(|round| round.agent_feedback.clone()),
        latest_agent_recommendation: latest_round
            .and_then(|round| round.agent_recommendation.clone()),
        remaining_uncertainty,
    }
}

fn learning_selection_outcome(
    deliberation: &LearningDeliberation,
    latest_round: Option<&volicord_inquiry::LearningDeliberationRound>,
) -> LearningSelectionOutcome {
    match &deliberation.state {
        LearningDeliberationState::Completed {
            selected_alternatives,
            ..
        } => LearningSelectionOutcome::Selected {
            selections: selected_alternatives.clone(),
            completed: true,
        },
        LearningDeliberationState::Delegated { .. } => LearningSelectionOutcome::Delegated,
        LearningDeliberationState::Skipped { .. } => LearningSelectionOutcome::Skipped,
        LearningDeliberationState::ResearchOrPrototypeRequired { evidence_state, .. } => {
            LearningSelectionOutcome::ResearchOrPrototypeRequired {
                evidence_state: *evidence_state,
            }
        }
        LearningDeliberationState::ReconsiderationRequested { .. } => {
            LearningSelectionOutcome::ReconsiderationRequested
        }
        LearningDeliberationState::AwaitingAgentFeedback { .. }
        | LearningDeliberationState::FeedbackProvided { .. } => latest_round
            .map(|round| match &round.response {
                LearningInitialResponse::Select { selections } => {
                    LearningSelectionOutcome::Selected {
                        selections: selections.clone(),
                        completed: false,
                    }
                }
                LearningInitialResponse::DelegateToAgent => LearningSelectionOutcome::Delegated,
                LearningInitialResponse::Skip => LearningSelectionOutcome::Skipped,
                LearningInitialResponse::RequestResearchOrPrototype { evidence_state } => {
                    LearningSelectionOutcome::ResearchOrPrototypeRequired {
                        evidence_state: *evidence_state,
                    }
                }
            })
            .unwrap_or(LearningSelectionOutcome::NotRecorded),
        LearningDeliberationState::AwaitingInitialResponse => LearningSelectionOutcome::NotRecorded,
    }
}

fn unavailable_learning_explanation_basis(
    omission: Option<&CandidateContentOmission>,
) -> LearningExplanationBasis {
    LearningExplanationBasis {
        availability: LearningExplanationAvailability::Unavailable,
        availability_reasons: vec![omission.map_or_else(
            || "Learning Deliberation content is unavailable".to_owned(),
            |omission| format!("Learning Deliberation content is unavailable: {omission:?}"),
        )],
        problem: None,
        established_facts: Vec::new(),
        alternatives: Vec::new(),
        affected_scope: Vec::new(),
        source_basis: Vec::new(),
        analysis_snapshot_id: None,
        selection_outcome: LearningSelectionOutcome::NotRecorded,
        latest_user_rationale: None,
        latest_agent_feedback: None,
        latest_agent_recommendation: None,
        remaining_uncertainty: vec!["the retained explanation basis cannot be inspected".to_owned()],
    }
}

fn scope_matches(
    policy: &CollectionOptOutScope,
    candidate: &volicord_inquiry::CandidateCollectionScope,
) -> bool {
    policy.project_id == candidate.project_id
        && policy
            .session
            .as_ref()
            .is_none_or(|value| candidate.session.as_ref() == Some(value))
        && policy
            .source_operation
            .as_ref()
            .is_none_or(|value| candidate.source_operation.as_ref() == Some(value))
        && policy
            .candidate_kind
            .is_none_or(|value| candidate.candidate_kind == value)
}

/// Candidate Inspection's compact continuation view. It deliberately has no
/// discovery graph, Materiality dimensions, interaction review or round history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearningResumeItem {
    pub candidate_id: CandidateId,
    pub revision: u64,
    pub goal_context_id: volicord_context::ContextItemId,
    pub baseline_analysis_snapshot_id: volicord_repository_intelligence::AnalysisSnapshotId,
    pub discovery_candidate_id: CandidateId,
    pub review_candidate_id: CandidateId,
    pub dimension_id: String,
    pub state: volicord_inquiry::LearningDeliberationState,
    pub response_source_id: Option<volicord_context::SourceId>,
    pub current_implication: Option<String>,
    pub implication_omitted: bool,
}

pub struct LearningResumeProjection {
    pub items: Vec<LearningResumeItem>,
    pub omitted_count: usize,
    pub withheld_count: usize,
}

/// Shares Candidate Inspection's content/forgetting boundary without materializing
/// full inspections or repository graphs. Pending learning precedes terminal history;
/// newest observation, then stable identity breaks ties. No durable lesson is inferred.
pub fn learning_resume_projection(basis: &CandidateReadBasis) -> LearningResumeProjection {
    use volicord_inquiry::{CandidateKind, LearningDeliberationState};
    let mut candidates = basis
        .candidates
        .iter()
        .filter(|candidate| candidate.kind == CandidateKind::LearningDeliberation)
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| {
        let terminal = candidate
            .content
            .as_ref()
            .and_then(|c| c.learning_deliberation.as_ref())
            .is_some_and(|learning| {
                matches!(
                    learning.state,
                    LearningDeliberationState::Completed { .. }
                        | LearningDeliberationState::Delegated { .. }
                        | LearningDeliberationState::Skipped { .. }
                )
            });
        (
            terminal,
            std::cmp::Reverse(candidate.observed_at),
            candidate.id,
        )
    });
    let mut items = Vec::new();
    let mut withheld_count = 0;
    let mut omitted_count = 0;
    for candidate in candidates {
        if candidate.cleanup.is_some()
            || basis
                .withheld_for_canonical_forgetting
                .contains(&candidate.id)
        {
            withheld_count += 1;
            continue;
        }
        let Some(learning) = candidate
            .content
            .as_ref()
            .and_then(|c| c.learning_deliberation.as_ref())
        else {
            withheld_count += 1;
            continue;
        };
        if items.len() == 64 {
            omitted_count += 1;
            continue;
        }
        let round = learning.rounds.last();
        let implication = round.and_then(|round| round.agent_feedback.as_ref());
        items.push(LearningResumeItem {
            candidate_id: candidate.id,
            revision: candidate.revision,
            goal_context_id: learning.goal_context_id,
            baseline_analysis_snapshot_id: learning.baseline_analysis_snapshot_id,
            discovery_candidate_id: learning.engineering_choice_discovery_candidate_id,
            review_candidate_id: learning.materiality_review_candidate_id,
            dimension_id: learning.dimension_id.clone(),
            state: learning.state.clone(),
            response_source_id: round.map(|round| round.initial_response_source_id),
            current_implication: implication.filter(|text| text.len() <= 2048).cloned(),
            implication_omitted: implication.is_some_and(|text| text.len() > 2048),
        });
    }
    LearningResumeProjection {
        items,
        omitted_count,
        withheld_count,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        unavailable_learning_explanation_basis, CandidateContentOmission,
        LearningExplanationAvailability, LearningSelectionOutcome,
    };

    #[test]
    fn unavailable_learning_content_is_not_reported_as_a_completed_explanation_basis() {
        let basis =
            unavailable_learning_explanation_basis(Some(&CandidateContentOmission::PolicyWithheld));

        assert_eq!(
            basis.availability,
            LearningExplanationAvailability::Unavailable
        );
        assert!(basis.problem.is_none());
        assert!(basis.alternatives.is_empty());
        assert_eq!(
            basis.selection_outcome,
            LearningSelectionOutcome::NotRecorded
        );
        assert!(basis
            .availability_reasons
            .iter()
            .any(|reason| reason.contains("PolicyWithheld")));
        assert!(!basis.remaining_uncertainty.is_empty());
    }
}
