use crate::{
    LearningAuthorityAssessment, LearningParticipation, MaterialityDimension,
    MaterialityDisposition, MaterialityReview,
};
use std::collections::BTreeSet;

pub(crate) fn validate(review: &MaterialityReview) -> Result<(), String> {
    for dimension in &review.dimensions {
        match &dimension.learning_authority {
            LearningAuthorityAssessment::Inactive => {
                if matches!(
                    review.learning_participation,
                    LearningParticipation::Active { .. }
                ) {
                    return Err("active learning requires an independent user-authority determination for every dimension".into());
                }
            }
            LearningAuthorityAssessment::Assessed {
                independent_user_authority,
                choice_ids,
                material_outcomes,
                rationale,
                source_basis,
            } => {
                if *independent_user_authority != dimension.ownership.contains_user_owned_outcome
                    || choice_ids.iter().collect::<BTreeSet<_>>()
                        != dimension.discovered_choice_ids.iter().collect()
                    || choice_ids.len() != dimension.discovered_choice_ids.len()
                    || material_outcomes != &dimension.ownership.materially_varying_outcomes
                    || rationale.trim().is_empty()
                    || rationale.len() > 4096
                    || source_basis.is_empty()
                    || source_basis.len() > 128
                    || source_basis.iter().collect::<BTreeSet<_>>().len() != source_basis.len()
                    || source_basis
                        .iter()
                        .any(|source| !dimension.ownership.source_basis.contains(source))
                {
                    return Err("learning authority must agree with current choice/outcome ownership and cite its current Sources with a bounded counterfactual rationale".into());
                }
                if !independent_user_authority && (!dimension.basis.decision_basis.is_empty()
                        || matches!(dimension.disposition, MaterialityDisposition::UnresolvedUserOwnedOutcome { .. })
                        || dimension.alternative_accounting.iter().any(|account| matches!(account.resolution, crate::DiscoveredAlternativeResolution::EliminatedByApplicableDecision { .. })))
                    {
                        return Err("a learning-only dimension cannot use a canonical Decision as product authority".into());
                    }
            }
        }
    }
    Ok(())
}

pub(crate) fn permits_decision(dimension: &MaterialityDimension) -> bool {
    !matches!(
        dimension.learning_authority,
        LearningAuthorityAssessment::Assessed {
            independent_user_authority: false,
            ..
        }
    )
}

/// Check an identity-linked Question against the latest retained review for the current Goal.
/// Unlinked semantic equivalence remains an explicit agent/human judgment.
pub fn validate_question_authority(
    canonical: &volicord_context::CanonicalReadBasis,
    candidates: &[crate::CandidateRecord],
    question_id: volicord_context::QuestionId,
) -> Result<(), crate::Error> {
    let goal = canonical
        .context_items
        .iter()
        .filter(|item| item.role == volicord_context::ContextItemRole::Goal)
        .max_by_key(|item| (item.recorded_at, item.id));
    let review = candidates
        .iter()
        .filter(|candidate| {
            matches!(
                candidate.disposition,
                crate::CandidateDisposition::PendingOrRetained
            )
        })
        .filter_map(|candidate| {
            candidate
                .content
                .as_ref()?
                .materiality_review
                .as_ref()
                .map(|review| (candidate, review))
        })
        .filter(|(_, review)| Some(review.goal_context_id) == goal.map(|goal| goal.id))
        .max_by_key(|(candidate, _)| (candidate.created_at, candidate.id));
    let question = canonical
        .active_questions
        .iter()
        .chain(&canonical.terminal_question_history)
        .find(|question| question.id == question_id);
    if let (Some((_, review)), Some(question)) = (review, question) {
        validate(review)
            .map_err(|message| crate::Error::new(crate::ErrorKind::InvalidInput, message))?;
        if review.dimensions.iter().any(|dimension| {
            !permits_decision(dimension)
                && question
                    .material_scope
                    .contains(&crate::materiality_scope_token(&dimension.dimension_id))
        }) {
            return Err(crate::Error::new(crate::ErrorKind::DomainConflict, "current Materiality marks this dimension learning-only; its selection cannot become a canonical Decision"));
        }
    }
    Ok(())
}

pub(crate) fn meaning_changed(
    previous: &LearningAuthorityAssessment,
    current: &LearningAuthorityAssessment,
) -> bool {
    match (previous, current) {
        (LearningAuthorityAssessment::Inactive, LearningAuthorityAssessment::Inactive) => false,
        (
            LearningAuthorityAssessment::Assessed {
                independent_user_authority: a,
                choice_ids: ac,
                material_outcomes: ao,
                source_basis: ab,
                ..
            },
            LearningAuthorityAssessment::Assessed {
                independent_user_authority: b,
                choice_ids: bc,
                material_outcomes: bo,
                source_basis: bb,
                ..
            },
        ) => {
            a != b
                || ac.iter().collect::<BTreeSet<_>>() != bc.iter().collect()
                || ao != bo
                || ab.iter().collect::<BTreeSet<_>>() != bb.iter().collect()
        }
        _ => true,
    }
}

/// Deterministic continuity of a bounded learning choice. Descriptive paths and
/// presentation summaries are not outcome identities. Executable scope still
/// requires the independent pre-write materiality/artifact closure.
pub(crate) fn equivalent_dimension(
    previous: &MaterialityReview,
    previous_dimension: &MaterialityDimension,
    previous_choices: &[crate::EngineeringChoice],
    current: &MaterialityReview,
    current_dimension: &MaterialityDimension,
    current_choices: &[crate::EngineeringChoice],
) -> bool {
    let ids = |dimension: &MaterialityDimension| {
        dimension
            .discovered_choice_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
    };
    if previous.goal_context_id != current.goal_context_id
        || previous.baseline_analysis_snapshot_id != current.baseline_analysis_snapshot_id
        || previous_dimension.dimension_id != current_dimension.dimension_id
        || ids(previous_dimension) != ids(current_dimension)
        || previous_dimension.material_consequences != current_dimension.material_consequences
        || previous_dimension.observable_signals != current_dimension.observable_signals
        || previous_dimension.ownership.materially_varying_outcomes
            != current_dimension.ownership.materially_varying_outcomes
        || previous_dimension.ownership.contains_user_owned_outcome
            != current_dimension.ownership.contains_user_owned_outcome
    {
        return false;
    }
    let basis = |choices: &[crate::EngineeringChoice], dimension: &MaterialityDimension| {
        let mut selected = choices
            .iter()
            .filter(|choice| dimension.discovered_choice_ids.contains(&choice.choice_id))
            .cloned()
            .collect::<Vec<_>>();
        for choice in &mut selected {
            choice.summary.clear();
            choice.affected_scope.clear();
            for alternative in &mut choice.alternatives {
                alternative.summary.clear();
            }
            choice
                .alternatives
                .sort_by(|a, b| a.alternative_id.cmp(&b.alternative_id));
            choice.effect_categories.sort();
            choice.source_basis.sort();
        }
        selected.sort_by(|a, b| a.choice_id.cmp(&b.choice_id));
        selected
    };
    let before = basis(previous_choices, previous_dimension);
    let after = basis(current_choices, current_dimension);
    // Missing or duplicate identity evidence cannot prove equivalence.
    before.len() == ids(previous_dimension).len()
        && after.len() == ids(current_dimension).len()
        && before
            .iter()
            .map(|choice| choice.choice_id.clone())
            .collect::<BTreeSet<_>>()
            == ids(previous_dimension)
        && after
            .iter()
            .map(|choice| choice.choice_id.clone())
            .collect::<BTreeSet<_>>()
            == ids(current_dimension)
        && before == after
}

pub(crate) fn retained_discovery<'a>(
    candidates: &'a [crate::CandidateRecord],
    project_id: volicord_context::ProjectId,
    review: &MaterialityReview,
) -> Option<&'a crate::EngineeringChoiceDiscovery> {
    candidates
        .iter()
        .find(|candidate| {
            candidate.project_id == project_id
                && candidate.id == review.engineering_choice_discovery_candidate_id
                && matches!(
                    candidate.disposition,
                    crate::CandidateDisposition::PendingOrRetained
                )
        })?
        .content
        .as_ref()?
        .engineering_choice_discovery
        .as_ref()
}

/// A changed learning assessment may name a new requested learning outcome even
/// when engineering alternatives remain stable. Only presentation rationale is ignored.
pub(crate) fn same_learning_requirement(
    previous: &crate::LearningValueAssessment,
    current: &crate::LearningValueAssessment,
) -> bool {
    let normalize = |value: &crate::LearningValueAssessment| {
        let mut value = value.clone();
        match &mut value {
            crate::LearningValueAssessment::Routine { rationale }
            | crate::LearningValueAssessment::DeliberationWorthy { rationale, .. } => {
                rationale.clear()
            }
        }
        value
    };
    normalize(previous) == normalize(current)
}
