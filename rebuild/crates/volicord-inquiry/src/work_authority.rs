use crate::{
    evaluate_decision_applicability, ApplicabilityQuery, CandidateKind, CandidateRecord,
    DecisionApplicabilityState, ExploratoryDisposition, MaterialityDimension,
    MaterialityDisposition, WorkAuthorityBasisKind,
};
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
    ReadyForWork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkAuthorityDisposition {
    ReviewMissing,
    ReviewInvalid,
    ResearchRequired,
    QuestionRequired,
    ReadyForWork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkAuthorityAction {
    RecordMaterialityReview,
    ReviseMaterialityReview,
    ContinueResearchOrPrototype,
    EnterExistingQuestionLifecycle,
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
    pub review_revision: Option<u64>,
    pub stage: WorkAuthorityStage,
    pub disposition: WorkAuthorityDisposition,
    pub next_action: Option<WorkAuthorityAction>,
    pub blocking: bool,
    pub reason: String,
    pub satisfied_requirements: Vec<WorkAuthorityRequirement>,
    pub unresolved_requirements: Vec<WorkAuthorityRequirement>,
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
    review_candidate: Option<&CandidateRecord>,
    project_id: ProjectId,
    goal_context_id: ContextItemId,
    baseline_analysis_snapshot_id: AnalysisSnapshotId,
    applicability: &ApplicabilityQuery,
) -> WorkAuthorityResult {
    let mut result = WorkAuthorityResult {
        project_id,
        goal_context_id,
        baseline_analysis_snapshot_id,
        review_candidate_id: review_candidate.map(|candidate| candidate.id),
        review_revision: review_candidate.map(|candidate| candidate.revision),
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
    let Some(candidate) = review_candidate else {
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
    if question_required {
        result.stage = WorkAuthorityStage::QuestionRequired;
        result.disposition = WorkAuthorityDisposition::QuestionRequired;
        result.next_action = Some(WorkAuthorityAction::EnterExistingQuestionLifecycle);
        result.reason =
            "one or more material user-owned outcomes still require explicit authority".to_owned();
    } else if research_required {
        result.stage = WorkAuthorityStage::ResearchOrPrototype;
        result.disposition = WorkAuthorityDisposition::ResearchRequired;
        result.next_action = Some(WorkAuthorityAction::ContinueResearchOrPrototype);
        result.reason =
            "research or prototype evidence must feed a revised Materiality Review".to_owned();
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
        MaterialityDisposition::DelegatedImplementationChoice => {
            require_kind(dimension, WorkAuthorityBasisKind::ExplicitDelegation)?;
            if dimension.basis.decision_basis.is_empty() {
                if current_goal_delegates_dimension(canonical, goal, dimension, applicability) {
                    return Ok(Vec::new());
                }
                return Err(DimensionIssue::Invalid(
                    "delegation requires the exact current Goal user-turn Source within the current work scope or an applicable explicit delegation Decision"
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

fn current_goal_delegates_dimension(
    canonical: &CanonicalReadBasis,
    goal: &ContextItem,
    dimension: &MaterialityDimension,
    applicability: &ApplicabilityQuery,
) -> bool {
    let exact_goal_user_turn = goal.source_basis.iter().copied().any(|source_id| {
        dimension.basis.source_basis.contains(&source_id)
            && canonical.sources.iter().any(|basis| {
                basis.source.id == source_id
                    && basis.source.project_id == canonical.project.id
                    && basis.freshness == SourceFreshness::Current
                    && basis.source.actor.kind == PrincipalKind::User
                    && matches!(
                        basis.source.payload,
                        SourcePayload::CurrentHostUserTurn { .. }
                    )
            })
    });
    exact_goal_user_turn
        && scope_contains_dimension(
            &goal.applicability,
            applicability,
            &dimension.affected_scope,
        )
}

fn scope_contains_dimension(
    goal_scope: &ApplicabilityScope,
    work_scope: &ApplicabilityQuery,
    affected_scope: &[String],
) -> bool {
    affected_scope.iter().all(|affected| {
        scope_item_matches(&goal_scope.paths, affected, true)
            || scope_item_matches(&goal_scope.components, affected, false)
            || scope_item_matches(&goal_scope.work_contexts, affected, false)
            || scope_item_matches(&work_scope.paths, affected, true)
            || scope_item_matches(&work_scope.components, affected, false)
            || scope_item_matches(&work_scope.work_contexts, affected, false)
    })
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
