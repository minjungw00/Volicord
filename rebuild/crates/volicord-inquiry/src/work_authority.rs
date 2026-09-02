use crate::{
    evaluate_decision_applicability, ApplicabilityQuery, CandidateKind, CandidateRecord,
    DecisionApplicabilityState, ExploratoryDisposition, LearningDeliberationState,
    LearningParticipation, LearningValueAssessment, MaterialityDimension, MaterialityDisposition,
    WorkAuthorityBasisKind,
};
use std::collections::BTreeSet;
use volicord_context::{
    ApplicabilityScope, CanonicalReadBasis, ContextItem, ContextItemId, ContextItemRole,
    DecisionChoice, DecisionId, PrincipalKind, ProjectId, SourceFreshness, SourcePayload,
    StatementProvenanceRole,
};
use volicord_repository_intelligence::AnalysisSnapshotId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkAuthorityStage {
    MaterialityReview,
    ResearchOrPrototype,
    QuestionRequired,
    LearningDeliberation,
    ReadyForWork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkAuthorityDisposition {
    ReviewMissing,
    ReviewInvalid,
    ResearchRequired,
    QuestionRequired,
    LearningDeliberationPending,
    ReadyForWork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkAuthorityAction {
    RecordMaterialityReview,
    ReviseMaterialityReview,
    ContinueResearchOrPrototype,
    EnterExistingQuestionLifecycle,
    BeginOrContinueLearningDeliberation,
    BeginOrdinaryWork,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkAuthorityRequirement {
    pub dimension_id: Option<String>,
    pub reason: String,
    pub decision_basis: Vec<DecisionId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkAuthorityResult {
    pub project_id: ProjectId,
    pub goal_context_id: ContextItemId,
    pub baseline_analysis_snapshot_id: AnalysisSnapshotId,
    pub review_candidate_id: Option<crate::CandidateId>,
    pub engineering_choice_discovery_candidate_id: Option<crate::CandidateId>,
    pub learning_deliberation_candidate_ids: Vec<crate::CandidateId>,
    pub review_revision: Option<u64>,
    pub stage: WorkAuthorityStage,
    pub disposition: WorkAuthorityDisposition,
    pub next_action: Option<WorkAuthorityAction>,
    pub blocking: bool,
    pub reason: String,
    pub satisfied_requirements: Vec<WorkAuthorityRequirement>,
    pub unresolved_requirements: Vec<WorkAuthorityRequirement>,
}

#[derive(Clone, Copy, Debug)]
pub struct WorkAuthorityCandidateBasis<'a> {
    pub review: Option<&'a CandidateRecord>,
    pub discovery: Option<&'a CandidateRecord>,
    pub learning_deliberations: &'a [CandidateRecord],
}

pub fn materiality_scope_token(dimension_id: &str) -> String {
    format!("work-authority:{dimension_id}")
}

/// Reuses a reviewed user-owned dimension as bounded input to the existing
/// Question Candidate lifecycle. It does not promote a Question or create a
/// Decision.
pub fn bind_question_candidate_to_materiality(
    review_candidate: &CandidateRecord,
    dimension_id: &str,
    mut draft: crate::CandidateDraft,
) -> Result<crate::CandidateDraft, crate::Error> {
    if review_candidate.project_id != draft.project_id
        || review_candidate.kind != CandidateKind::MaterialityReview
        || draft.kind != CandidateKind::QuestionCandidate
    {
        return Err(crate::Error::new(
            crate::ErrorKind::WrongProject,
            "Materiality Review and Question Candidate Project/kind must match",
        ));
    }
    let review = review_candidate
        .content
        .as_ref()
        .and_then(|content| content.materiality_review.as_ref())
        .ok_or_else(|| {
            crate::Error::new(
                crate::ErrorKind::CorruptState,
                "Materiality Review content is unavailable",
            )
        })?;
    let dimension = review
        .dimensions
        .iter()
        .find(|dimension| dimension.dimension_id == dimension_id)
        .ok_or_else(|| {
            crate::Error::new(
                crate::ErrorKind::NotFound,
                "materiality dimension was not found",
            )
        })?;
    if !matches!(
        dimension.disposition,
        MaterialityDisposition::UnresolvedUserOwnedOutcome {
            resolution_decision_id: None
        }
    ) {
        return Err(crate::Error::new(
            crate::ErrorKind::DomainConflict,
            "only an unresolved user-owned dimension enters the Question lifecycle",
        ));
    }
    let question = draft.content.question.as_mut().ok_or_else(|| {
        crate::Error::new(
            crate::ErrorKind::InvalidInput,
            "Question content is missing",
        )
    })?;
    question
        .affected_scope
        .push(materiality_scope_token(dimension_id));
    question
        .affected_scope
        .extend(dimension.affected_scope.iter().cloned());
    question
        .source_basis
        .extend(dimension.basis.source_basis.iter().copied());
    question
        .materiality
        .source_basis
        .extend(dimension.basis.source_basis.iter().copied());
    question
        .recommendation
        .source_basis
        .extend(dimension.basis.source_basis.iter().copied());
    for fact in &mut question.known_facts {
        if fact.source_basis.is_empty() {
            fact.source_basis
                .extend(dimension.basis.source_basis.iter().copied());
        }
    }
    draft
        .observation_basis
        .source_basis
        .extend(dimension.basis.source_basis.iter().copied());
    question.affected_scope.sort();
    question.affected_scope.dedup();
    question.source_basis.sort_unstable();
    question.source_basis.dedup();
    question.materiality.source_basis.sort_unstable();
    question.materiality.source_basis.dedup();
    question.recommendation.source_basis.sort_unstable();
    question.recommendation.source_basis.dedup();
    draft.observation_basis.source_basis.sort_unstable();
    draft.observation_basis.source_basis.dedup();
    Ok(draft)
}

pub fn evaluate_work_authority(
    canonical: &CanonicalReadBasis,
    candidate_basis: WorkAuthorityCandidateBasis<'_>,
    project_id: ProjectId,
    goal_context_id: ContextItemId,
    baseline_analysis_snapshot_id: AnalysisSnapshotId,
    applicability: &ApplicabilityQuery,
) -> WorkAuthorityResult {
    let mut result = WorkAuthorityResult {
        project_id,
        goal_context_id,
        baseline_analysis_snapshot_id,
        review_candidate_id: candidate_basis.review.map(|candidate| candidate.id),
        engineering_choice_discovery_candidate_id: candidate_basis
            .discovery
            .map(|candidate| candidate.id),
        learning_deliberation_candidate_ids: Vec::new(),
        review_revision: candidate_basis.review.map(|candidate| candidate.revision),
        stage: WorkAuthorityStage::MaterialityReview,
        disposition: WorkAuthorityDisposition::ReviewMissing,
        next_action: Some(WorkAuthorityAction::RecordMaterialityReview),
        blocking: true,
        reason: "a current pre-work Materiality Review is required".to_owned(),
        satisfied_requirements: Vec::new(),
        unresolved_requirements: Vec::new(),
    };
    if canonical.project.id != project_id || applicability.project_id != project_id {
        return invalid(result, None, "workflow Project basis does not match");
    }
    let Some(goal) = canonical
        .context_items
        .iter()
        .find(|item| item.id == goal_context_id)
    else {
        return invalid(result, None, "current Goal Context was not found");
    };
    let current_goal = canonical
        .context_items
        .iter()
        .filter(|item| item.role == ContextItemRole::Goal)
        .max_by_key(|item| (item.recorded_at, item.id));
    if current_goal.map(|item| item.id) != Some(goal_context_id)
        || goal.provenance_role != StatementProvenanceRole::UserStatement
        || goal.author.kind != PrincipalKind::User
    {
        return invalid(
            result,
            None,
            "workflow is not bound to the current user-stated Goal",
        );
    }
    let Some(candidate) = candidate_basis.review else {
        result.unresolved_requirements.push(requirement(
            None,
            "no Materiality Review exists; this is not evidence that no Question is required",
            Vec::new(),
        ));
        return result;
    };
    if candidate.project_id != project_id || candidate.kind != CandidateKind::MaterialityReview {
        return invalid(
            result,
            None,
            "review Candidate Project or kind does not match",
        );
    }
    let Some(review) = candidate
        .content
        .as_ref()
        .and_then(|content| content.materiality_review.as_ref())
    else {
        return invalid(result, None, "Materiality Review content is unavailable");
    };
    if review.goal_context_id != goal_context_id
        || review.baseline_analysis_snapshot_id != baseline_analysis_snapshot_id
    {
        return invalid(result, None, "Materiality Review Goal or baseline is stale");
    }
    if !review.first_review_preceded_meaningful_mutation {
        return invalid(
            result,
            None,
            "the first authoritative review was late and cannot retroactively authorize work",
        );
    }
    if review.dimensions.is_empty() {
        return invalid(
            result,
            None,
            "the review omitted owner classification for every outcome dimension",
        );
    }
    if let Err(reason) = validate_discovery_boundary(review, candidate_basis.discovery) {
        return invalid(result, None, reason);
    }

    let mut research_required = false;
    let mut question_required = false;
    for dimension in &review.dimensions {
        match evaluate_dimension(canonical, goal, dimension, applicability) {
            Ok(decisions) => result.satisfied_requirements.push(requirement(
                Some(dimension.dimension_id.clone()),
                "material outcome has inspectable authority or an explicit non-Question disposition",
                decisions,
            )),
            Err(DimensionIssue::Research(reason)) => {
                research_required = true;
                result.unresolved_requirements.push(requirement(
                    Some(dimension.dimension_id.clone()),
                    reason,
                    Vec::new(),
                ));
            }
            Err(DimensionIssue::Question(reason)) => {
                question_required = true;
                result.unresolved_requirements.push(requirement(
                    Some(dimension.dimension_id.clone()),
                    reason,
                    Vec::new(),
                ));
            }
            Err(DimensionIssue::Invalid(reason)) => {
                return invalid(result, Some(dimension.dimension_id.clone()), reason);
            }
        }
    }
    if let Err(reason) = validate_requested_work_scope(review, applicability) {
        return invalid(result, None, reason);
    }
    if question_required {
        result.stage = WorkAuthorityStage::QuestionRequired;
        result.disposition = WorkAuthorityDisposition::QuestionRequired;
        result.next_action = Some(WorkAuthorityAction::EnterExistingQuestionLifecycle);
        result.reason = if review.late_work_authority_revisions.is_empty() {
            "one or more material user-owned outcomes still require explicit authority".to_owned()
        } else {
            "one or more material user-owned outcomes still require explicit authority; that authority is prospective and cannot certify already-observed affected work"
                .to_owned()
        };
    } else if research_required {
        result.stage = WorkAuthorityStage::ResearchOrPrototype;
        result.disposition = WorkAuthorityDisposition::ResearchRequired;
        result.next_action = Some(WorkAuthorityAction::ContinueResearchOrPrototype);
        result.reason =
            "research or prototype evidence must feed a revised Materiality Review".to_owned();
    } else if !review.late_work_authority_revisions.is_empty() {
        let affected = review
            .late_work_authority_revisions
            .iter()
            .map(|correction| correction.dimension_id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return invalid(
            result,
            None,
            format!(
                "affected work preceded a work-authority or readiness revision for dimensions {affected}; the revision is prospective and cannot certify the earlier work"
            ),
        );
    } else if let Err(reason) = evaluate_learning_readiness(
        canonical,
        candidate,
        review,
        candidate_basis.learning_deliberations,
        &mut result,
    ) {
        match reason {
            LearningIssue::Pending(reason) => {
                result.stage = WorkAuthorityStage::LearningDeliberation;
                result.disposition = WorkAuthorityDisposition::LearningDeliberationPending;
                result.next_action = Some(WorkAuthorityAction::BeginOrContinueLearningDeliberation);
                result.reason = reason;
            }
            LearningIssue::Research(reason) => {
                result.stage = WorkAuthorityStage::ResearchOrPrototype;
                result.disposition = WorkAuthorityDisposition::ResearchRequired;
                result.next_action = Some(WorkAuthorityAction::ContinueResearchOrPrototype);
                result.reason = reason;
            }
            LearningIssue::Invalid(reason) => return invalid(result, None, reason),
        }
    } else {
        result.stage = WorkAuthorityStage::ReadyForWork;
        result.disposition = WorkAuthorityDisposition::ReadyForWork;
        result.next_action = Some(WorkAuthorityAction::BeginOrdinaryWork);
        result.blocking = false;
        result.reason =
            "every material outcome dimension has an inspectable disposition".to_owned();
    }
    result
}

fn validate_discovery_boundary(
    review: &crate::MaterialityReview,
    discovery_candidate: Option<&CandidateRecord>,
) -> Result<(), String> {
    let candidate = discovery_candidate.ok_or_else(|| {
        "the referenced Engineering Choice Discovery Candidate is missing".to_owned()
    })?;
    if candidate.id != review.engineering_choice_discovery_candidate_id
        || candidate.kind != CandidateKind::EngineeringChoiceDiscovery
    {
        return Err("Materiality Review references the wrong Engineering Choice Discovery".into());
    }
    let discovery = candidate
        .content
        .as_ref()
        .and_then(|content| content.engineering_choice_discovery.as_ref())
        .ok_or_else(|| "Engineering Choice Discovery content is unavailable".to_owned())?;
    if discovery.goal_context_id != review.goal_context_id
        || discovery.baseline_analysis_snapshot_id != review.baseline_analysis_snapshot_id
    {
        return Err("Engineering Choice Discovery Goal or baseline is stale".into());
    }
    let discovered = discovery
        .choices
        .iter()
        .map(|choice| choice.choice_id.as_str())
        .collect::<BTreeSet<_>>();
    let reviewed = review
        .dimensions
        .iter()
        .flat_map(|dimension| dimension.discovered_choice_ids.iter().map(String::as_str))
        .collect::<Vec<_>>();
    if reviewed.iter().copied().collect::<BTreeSet<_>>() != discovered
        || reviewed.len() != discovered.len()
    {
        return Err(
            "Materiality Review must classify each discovered engineering choice exactly once"
                .into(),
        );
    }
    Ok(())
}

enum LearningIssue {
    Pending(String),
    Research(String),
    Invalid(String),
}

fn evaluate_learning_readiness(
    canonical: &CanonicalReadBasis,
    review_candidate: &CandidateRecord,
    review: &crate::MaterialityReview,
    learning_candidates: &[CandidateRecord],
    result: &mut WorkAuthorityResult,
) -> Result<(), LearningIssue> {
    let participation_active = match &review.learning_participation {
        LearningParticipation::Inactive => false,
        LearningParticipation::Active {
            user_turn_source_id,
            verbatim_statement,
        } => {
            let source = canonical
                .sources
                .iter()
                .find(|basis| basis.source.id == *user_turn_source_id)
                .ok_or_else(|| {
                    LearningIssue::Invalid(
                        "explicit learning participation Source is missing".to_owned(),
                    )
                })?;
            let SourcePayload::CurrentHostUserTurn { turn, .. } = &source.source.payload else {
                return Err(LearningIssue::Invalid(
                    "learning participation is not grounded in a current-host user turn".to_owned(),
                ));
            };
            if source.freshness != SourceFreshness::Current
                || source.source.actor.kind != PrincipalKind::User
                || verbatim_statement.trim().is_empty()
                || !turn.contains(verbatim_statement)
            {
                return Err(LearningIssue::Invalid(
                    "learning participation must be explicit, current, user-authored, and verbatim-grounded"
                        .to_owned(),
                ));
            }
            true
        }
    };
    if !participation_active {
        return Ok(());
    }

    let mut pending = false;
    let mut research = false;
    for dimension in &review.dimensions {
        if !matches!(
            dimension.disposition,
            MaterialityDisposition::AgentOwnedImplementationChoice
                | MaterialityDisposition::DelegatedImplementationChoice
        ) || !matches!(
            dimension.learning_value,
            LearningValueAssessment::DeliberationWorthy { .. }
        ) {
            continue;
        }
        let candidate = learning_candidates
            .iter()
            .filter(|candidate| {
                candidate.project_id == review_candidate.project_id
                    && candidate.kind == CandidateKind::LearningDeliberation
                    && matches!(
                        candidate.disposition,
                        crate::CandidateDisposition::PendingOrRetained
                    )
                    && candidate.content.as_ref().is_some_and(|content| {
                        content
                            .learning_deliberation
                            .as_ref()
                            .is_some_and(|deliberation| {
                                deliberation.materiality_review_candidate_id == review_candidate.id
                                    && deliberation.dimension_id == dimension.dimension_id
                            })
                    })
            })
            .max_by_key(|candidate| (candidate.created_at, candidate.id));
        let Some(candidate) = candidate else {
            pending = true;
            result.unresolved_requirements.push(requirement(
                Some(dimension.dimension_id.clone()),
                "explicit learning participation requires a pre-work Learning Deliberation for this agent-owned choice",
                Vec::new(),
            ));
            continue;
        };
        let deliberation = candidate
            .content
            .as_ref()
            .and_then(|content| content.learning_deliberation.as_ref())
            .ok_or_else(|| {
                LearningIssue::Invalid(
                    "Learning Deliberation Candidate content is unavailable".to_owned(),
                )
            })?;
        if deliberation.goal_context_id != review.goal_context_id
            || deliberation.baseline_analysis_snapshot_id != review.baseline_analysis_snapshot_id
            || deliberation.engineering_choice_discovery_candidate_id
                != review.engineering_choice_discovery_candidate_id
            || deliberation.discovered_choice_ids != dimension.discovered_choice_ids
            || deliberation.affected_scope != dimension.affected_scope
        {
            return Err(LearningIssue::Invalid(
                "Learning Deliberation does not match the exact current choice, Goal, baseline, review, or affected scope"
                    .to_owned(),
            ));
        }
        result
            .learning_deliberation_candidate_ids
            .push(candidate.id);
        match deliberation.state {
            LearningDeliberationState::Completed { .. }
            | LearningDeliberationState::Delegated { .. }
            | LearningDeliberationState::Skipped { .. } => {
                result.satisfied_requirements.push(requirement(
                    Some(dimension.dimension_id.clone()),
                    "the learning opportunity reached a terminal non-Decision state",
                    Vec::new(),
                ));
            }
            LearningDeliberationState::ResearchOrPrototypeRequired { .. } => {
                research = true;
                result.unresolved_requirements.push(requirement(
                    Some(dimension.dimension_id.clone()),
                    "the learning response requested bounded research or prototype evidence",
                    Vec::new(),
                ));
            }
            LearningDeliberationState::AwaitingInitialResponse
            | LearningDeliberationState::AwaitingAgentFeedback { .. }
            | LearningDeliberationState::FeedbackProvided { .. }
            | LearningDeliberationState::ReconsiderationRequested { .. } => {
                pending = true;
                result.unresolved_requirements.push(requirement(
                    Some(dimension.dimension_id.clone()),
                    "the current Learning Deliberation has not reached a valid terminal state",
                    Vec::new(),
                ));
            }
        }
    }
    if research {
        Err(LearningIssue::Research(
            "bounded research or prototype evidence is required by a learning response".to_owned(),
        ))
    } else if pending {
        Err(LearningIssue::Pending(
            "one or more agent-owned learning opportunities require pre-work deliberation"
                .to_owned(),
        ))
    } else {
        Ok(())
    }
}

enum DimensionIssue {
    Research(String),
    Question(String),
    Invalid(String),
}

fn evaluate_dimension(
    canonical: &CanonicalReadBasis,
    goal: &ContextItem,
    dimension: &MaterialityDimension,
    applicability: &ApplicabilityQuery,
) -> Result<Vec<DecisionId>, DimensionIssue> {
    if dimension.dimension_id.trim().is_empty()
        || dimension.summary.trim().is_empty()
        || dimension.affected_scope.is_empty()
        || dimension.material_consequences.is_empty()
        || dimension.basis.summary.trim().is_empty()
        || dimension.basis.source_basis.is_empty()
    {
        return Err(DimensionIssue::Invalid(
            "dimension and bounded Source/consequence basis must be complete".to_owned(),
        ));
    }
    if dimension
        .basis
        .source_basis
        .iter()
        .any(|source_id| !source_is_current(canonical, *source_id))
    {
        return Err(DimensionIssue::Invalid(
            "dimension Source basis is missing, stale, unavailable, or freshness-unknown"
                .to_owned(),
        ));
    }
    if dimension.basis.explicit_delegation.is_some()
        && !matches!(
            dimension.disposition,
            MaterialityDisposition::DelegatedImplementationChoice
        )
    {
        return Err(DimensionIssue::Invalid(
            "explicit delegation evidence cannot authorize a non-delegated disposition".to_owned(),
        ));
    }
    if matches!(
        dimension.disposition,
        MaterialityDisposition::DelegatedImplementationChoice
    ) && dimension.basis.kinds.iter().any(|kind| {
        matches!(
            kind,
            WorkAuthorityBasisKind::AcceptedContract
                | WorkAuthorityBasisKind::AgentRecommendation
                | WorkAuthorityBasisKind::LibraryOrConvention
                | WorkAuthorityBasisKind::ImplementationPreference
        )
    }) {
        return Err(DimensionIssue::Invalid(
            "delegation authority cannot be supplied by an accepted contract, recommendation, convention, or implementation preference"
                .to_owned(),
        ));
    }
    match &dimension.disposition {
        MaterialityDisposition::RepositoryOrEnvironmentFact => {
            require_kind(
                dimension,
                WorkAuthorityBasisKind::RepositoryOrEnvironmentFact,
            )?;
            Ok(Vec::new())
        }
        MaterialityDisposition::SettledAuthority => {
            let contract = dimension
                .basis
                .kinds
                .contains(&WorkAuthorityBasisKind::AcceptedContract)
                && !dimension.basis.contract_basis.is_empty();
            let decisions = applicable_decisions(canonical, dimension, applicability, false)?;
            if !contract && decisions.is_empty() {
                return Err(DimensionIssue::Invalid(
                    "settled authority requires a source-grounded accepted contract or applicable Decision"
                        .to_owned(),
                ));
            }
            Ok(decisions)
        }
        MaterialityDisposition::AgentOwnedImplementationChoice => {
            require_kind(dimension, WorkAuthorityBasisKind::ImplementationPreference)?;
            if dimension.basis.explicit_delegation.is_some()
                || !dimension.basis.contract_basis.is_empty()
                || !dimension.basis.decision_basis.is_empty()
                || dimension.basis.kinds.iter().any(|kind| {
                    matches!(
                        kind,
                        WorkAuthorityBasisKind::AcceptedContract
                            | WorkAuthorityBasisKind::ApplicableDecision
                            | WorkAuthorityBasisKind::ExplicitDelegation
                            | WorkAuthorityBasisKind::AgentRecommendation
                    )
                })
            {
                return Err(DimensionIssue::Invalid(
                    "agent-owned implementation discretion must remain distinct from contract, Decision, delegation, and recommendation authority"
                        .to_owned(),
                ));
            }
            Ok(Vec::new())
        }
        MaterialityDisposition::DelegatedImplementationChoice => {
            require_kind(dimension, WorkAuthorityBasisKind::ExplicitDelegation)?;
            if dimension.basis.decision_basis.is_empty() {
                validate_current_goal_delegation(canonical, goal, dimension)?;
                return Ok(Vec::new());
            }
            if dimension.basis.explicit_delegation.is_some() {
                return Err(DimensionIssue::Invalid(
                    "current-task delegation evidence and Inquiry-time delegation Decision authority cannot be combined"
                        .to_owned(),
                ));
            }
            applicable_delegation_decisions(canonical, dimension, applicability)
        }
        MaterialityDisposition::ExploratoryUncertainty { disposition } => {
            if dimension.basis.research_basis.is_empty() {
                return Err(DimensionIssue::Invalid(
                    "exploratory uncertainty requires bounded research/prototype/defer basis"
                        .to_owned(),
                ));
            }
            match disposition {
                ExploratoryDisposition::ResearchRequired => Err(DimensionIssue::Research(
                    "repository or environment research is still required".to_owned(),
                )),
                ExploratoryDisposition::PrototypeRequired => Err(DimensionIssue::Research(
                    "a bounded prototype is still required".to_owned(),
                )),
                ExploratoryDisposition::DeferredWithRevisit => {
                    require_kind(dimension, WorkAuthorityBasisKind::DeferOrRevisitBasis)?;
                    Ok(Vec::new())
                }
                ExploratoryDisposition::ResolvedByResearch => {
                    require_kind(dimension, WorkAuthorityBasisKind::ResearchEvidence)?;
                    Ok(Vec::new())
                }
            }
        }
        MaterialityDisposition::UnresolvedUserOwnedOutcome {
            resolution_decision_id,
        } => {
            let Some(decision_id) = resolution_decision_id else {
                return Err(DimensionIssue::Question(
                    "material user-owned outcome must enter the existing Question and Decision lifecycle"
                        .to_owned(),
                ));
            };
            if !dimension.basis.decision_basis.contains(decision_id) {
                return Err(DimensionIssue::Invalid(
                    "resolved user-owned outcome must name its Decision in the authority basis"
                        .to_owned(),
                ));
            }
            let decisions = applicable_decisions(canonical, dimension, applicability, false)?;
            if !decisions.contains(decision_id) {
                return Err(DimensionIssue::Invalid(
                    "user-owned outcome Decision is not current and applicable".to_owned(),
                ));
            }
            let lifecycle = canonical
                .active_decisions
                .iter()
                .find(|item| item.decision.id == *decision_id)
                .ok_or_else(|| {
                    DimensionIssue::Invalid("user-owned outcome Decision is missing".to_owned())
                })?;
            let question = canonical
                .active_questions
                .iter()
                .chain(canonical.terminal_question_history.iter())
                .find(|question| question.id == lifecycle.decision.question_id)
                .ok_or_else(|| {
                    DimensionIssue::Invalid("Decision Question basis is missing".to_owned())
                })?;
            if !question
                .material_scope
                .contains(&materiality_scope_token(&dimension.dimension_id))
                || !source_is_current(canonical, lifecycle.decision.user_turn_source_id)
                || !canonical.sources.iter().any(|basis| {
                    basis.source.id == lifecycle.decision.user_turn_source_id
                        && matches!(
                            basis.source.payload,
                            SourcePayload::CurrentHostUserTurn { .. }
                        )
                })
            {
                return Err(DimensionIssue::Invalid(
                    "Decision lacks the exact materiality dimension or current-host response provenance"
                        .to_owned(),
                ));
            }
            Ok(vec![*decision_id])
        }
    }
}

fn validate_current_goal_delegation(
    canonical: &CanonicalReadBasis,
    goal: &ContextItem,
    dimension: &MaterialityDimension,
) -> Result<(), DimensionIssue> {
    let evidence = dimension
        .basis
        .explicit_delegation
        .as_ref()
        .ok_or_else(|| {
            DimensionIssue::Invalid(
                "current-task delegation requires explicit verbatim delegation evidence; a Goal Source alone is insufficient"
                    .to_owned(),
            )
        })?;
    if dimension.basis.kinds.iter().any(|kind| {
        matches!(
            kind,
            WorkAuthorityBasisKind::AcceptedContract
                | WorkAuthorityBasisKind::ApplicableDecision
                | WorkAuthorityBasisKind::AgentRecommendation
                | WorkAuthorityBasisKind::LibraryOrConvention
                | WorkAuthorityBasisKind::ImplementationPreference
        )
    }) || !dimension.basis.contract_basis.is_empty()
    {
        return Err(DimensionIssue::Invalid(
            "explicit delegation must remain distinct from accepted contract, Decision, recommendation, convention, and implementation-preference authority"
                .to_owned(),
        ));
    }
    if evidence.goal_context_id != goal.id
        || evidence.verbatim_statement.trim().is_empty()
        || evidence.dimension_id != dimension.dimension_id
        || evidence.discovered_choice_ids != dimension.discovered_choice_ids
        || evidence.affected_scope.is_empty()
        || evidence.material_consequences != dimension.material_consequences
        || evidence.effect_categories.is_empty()
        || !dimension
            .basis
            .source_basis
            .contains(&evidence.user_turn_source_id)
        || !goal.source_basis.contains(&evidence.user_turn_source_id)
        || !goal.statement.contains(&evidence.verbatim_statement)
        || !scope_values_contain(&evidence.affected_scope, &dimension.affected_scope)
        || !scope_values_are_relevant(&evidence.affected_scope, &dimension.affected_scope)
        || (scope_declared(&goal.applicability)
            && !scope_contains_affected(&goal.applicability, &evidence.affected_scope))
    {
        return Err(DimensionIssue::Invalid(
            "explicit delegation evidence must bind the exact current Goal, verbatim statement, user-turn Source, and bounded dimension/work scope"
                .to_owned(),
        ));
    }
    let source = canonical
        .sources
        .iter()
        .find(|basis| basis.source.id == evidence.user_turn_source_id)
        .ok_or_else(|| {
            DimensionIssue::Invalid("explicit delegation user-turn Source is missing".to_owned())
        })?;
    let SourcePayload::CurrentHostUserTurn { turn, .. } = &source.source.payload else {
        return Err(DimensionIssue::Invalid(
            "explicit delegation evidence is not grounded in a current-host user turn".to_owned(),
        ));
    };
    if source.source.project_id != canonical.project.id
        || source.freshness != SourceFreshness::Current
        || source.source.actor.kind != PrincipalKind::User
        || !turn.contains(&goal.statement)
        || !turn.contains(&evidence.verbatim_statement)
    {
        return Err(DimensionIssue::Invalid(
            "explicit delegation evidence has unrelated, stale, agent-authored, or non-verbatim user provenance"
                .to_owned(),
        ));
    }
    Ok(())
}

fn scope_declared(scope: &ApplicabilityScope) -> bool {
    !scope.paths.is_empty() || !scope.components.is_empty() || !scope.work_contexts.is_empty()
}

fn work_scope_requested(work_scope: &ApplicabilityQuery) -> bool {
    !work_scope.paths.is_empty()
        || !work_scope.components.is_empty()
        || !work_scope.work_contexts.is_empty()
}

fn validate_requested_work_scope(
    review: &crate::MaterialityReview,
    work_scope: &ApplicabilityQuery,
) -> Result<(), String> {
    if !work_scope_requested(work_scope) {
        return Ok(());
    }
    let reviewed_scope = review
        .dimensions
        .iter()
        .filter_map(|dimension| dimension.basis.explicit_delegation.as_ref())
        .flat_map(|delegation| delegation.affected_scope.iter())
        .collect::<Vec<_>>();
    if reviewed_scope.is_empty() {
        return Ok(());
    }
    if let Some(path) = work_scope.paths.iter().find(|path| {
        !reviewed_scope
            .iter()
            .any(|scope| path_is_covered(scope, path))
    }) {
        return Err(format!(
            "requested work path `{path}` is outside the reviewed affected scope"
        ));
    }
    if let Some(component) = work_scope.components.iter().find(|component| {
        !reviewed_scope
            .iter()
            .any(|scope| scope.as_str() == component.as_str())
    }) {
        return Err(format!(
            "requested work component `{component}` is outside the reviewed affected scope"
        ));
    }
    if let Some(work_context) = work_scope.work_contexts.iter().find(|work_context| {
        !reviewed_scope
            .iter()
            .any(|scope| scope.as_str() == work_context.as_str())
    }) {
        return Err(format!(
            "requested work context `{work_context}` is outside the reviewed affected scope"
        ));
    }
    Ok(())
}

fn scope_contains_affected(goal_scope: &ApplicabilityScope, affected_scope: &[String]) -> bool {
    affected_scope.iter().all(|affected| {
        scope_item_matches(&goal_scope.paths, affected, true)
            || scope_item_matches(&goal_scope.components, affected, false)
            || scope_item_matches(&goal_scope.work_contexts, affected, false)
    })
}

fn path_is_covered(declared: &str, requested: &str) -> bool {
    declared == requested
        || requested
            .strip_prefix(declared)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn scope_item_matches(scope: &[String], affected: &str, path_like: bool) -> bool {
    scope.iter().any(|declared| {
        declared == affected
            || (path_like
                && affected
                    .strip_prefix(declared)
                    .is_some_and(|suffix| suffix.starts_with('/')))
    })
}

fn scope_values_contain(scope: &[String], affected_scope: &[String]) -> bool {
    affected_scope.iter().all(|affected| {
        scope.iter().any(|declared| {
            declared == affected
                || affected
                    .strip_prefix(declared)
                    .is_some_and(|suffix| suffix.starts_with('/'))
        })
    })
}

fn scope_values_are_relevant(scope: &[String], affected_scope: &[String]) -> bool {
    scope.iter().all(|declared| {
        affected_scope
            .iter()
            .any(|affected| path_is_covered(declared, affected))
    })
}

fn applicable_delegation_decisions(
    canonical: &CanonicalReadBasis,
    dimension: &MaterialityDimension,
    applicability: &ApplicabilityQuery,
) -> Result<Vec<DecisionId>, DimensionIssue> {
    let decisions = applicable_decisions(canonical, dimension, applicability, true)?;
    if decisions.is_empty() {
        return Err(DimensionIssue::Invalid(
            "delegation requires an applicable explicit delegation Decision".to_owned(),
        ));
    }
    let scope_token = materiality_scope_token(&dimension.dimension_id);
    for decision_id in &decisions {
        let lifecycle = canonical
            .active_decisions
            .iter()
            .find(|item| item.decision.id == *decision_id)
            .ok_or_else(|| DimensionIssue::Invalid("delegation Decision is missing".to_owned()))?;
        let question = canonical
            .active_questions
            .iter()
            .chain(canonical.terminal_question_history.iter())
            .find(|question| question.id == lifecycle.decision.question_id)
            .ok_or_else(|| {
                DimensionIssue::Invalid("delegation Decision Question basis is missing".to_owned())
            })?;
        if !question.material_scope.contains(&scope_token)
            || !source_is_current(canonical, lifecycle.decision.user_turn_source_id)
            || !canonical.sources.iter().any(|basis| {
                basis.source.id == lifecycle.decision.user_turn_source_id
                    && basis.source.actor.kind == PrincipalKind::User
                    && matches!(
                        basis.source.payload,
                        SourcePayload::CurrentHostUserTurn { .. }
                    )
            })
        {
            return Err(DimensionIssue::Invalid(
                "delegation Decision lacks the exact materiality dimension or current-host response provenance"
                    .to_owned(),
            ));
        }
    }
    Ok(decisions)
}

fn applicable_decisions(
    canonical: &CanonicalReadBasis,
    dimension: &MaterialityDimension,
    applicability: &ApplicabilityQuery,
    delegation_only: bool,
) -> Result<Vec<DecisionId>, DimensionIssue> {
    let mut decisions = Vec::new();
    for decision_id in &dimension.basis.decision_basis {
        let lifecycle = canonical
            .active_decisions
            .iter()
            .find(|item| item.decision.id == *decision_id)
            .ok_or_else(|| {
                DimensionIssue::Invalid("authority Decision is not active".to_owned())
            })?;
        if delegation_only
            && !matches!(lifecycle.decision.choice, DecisionChoice::Delegation { .. })
        {
            return Err(DimensionIssue::Invalid(
                "a non-delegation Decision cannot establish explicit delegation".to_owned(),
            ));
        }
        if evaluate_decision_applicability(canonical, lifecycle, applicability).state
            != DecisionApplicabilityState::ReusableCurrent
        {
            return Err(DimensionIssue::Invalid(
                "authority Decision is not reusable in the current work scope".to_owned(),
            ));
        }
        decisions.push(*decision_id);
    }
    Ok(decisions)
}

fn require_kind(
    dimension: &MaterialityDimension,
    required: WorkAuthorityBasisKind,
) -> Result<(), DimensionIssue> {
    if !dimension.basis.kinds.contains(&required) {
        return Err(DimensionIssue::Invalid(format!(
            "disposition requires {required:?} evidence"
        )));
    }
    Ok(())
}

fn source_is_current(
    canonical: &CanonicalReadBasis,
    source_id: volicord_context::SourceId,
) -> bool {
    canonical.sources.iter().any(|basis| {
        basis.source.id == source_id
            && basis.source.project_id == canonical.project.id
            && basis.freshness == SourceFreshness::Current
    })
}

fn invalid(
    mut result: WorkAuthorityResult,
    dimension_id: Option<String>,
    reason: impl Into<String>,
) -> WorkAuthorityResult {
    let reason = reason.into();
    result.disposition = WorkAuthorityDisposition::ReviewInvalid;
    result.next_action = Some(WorkAuthorityAction::ReviseMaterialityReview);
    result.reason = reason.clone();
    result
        .unresolved_requirements
        .push(requirement(dimension_id, reason, Vec::new()));
    result
}

fn requirement(
    dimension_id: Option<String>,
    reason: impl Into<String>,
    decision_basis: Vec<DecisionId>,
) -> WorkAuthorityRequirement {
    WorkAuthorityRequirement {
        dimension_id,
        reason: reason.into(),
        decision_basis,
    }
}
