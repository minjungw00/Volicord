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
