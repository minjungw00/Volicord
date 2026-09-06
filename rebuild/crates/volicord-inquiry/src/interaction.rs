//! Structural completeness of active-agent interaction judgments.
use crate::{
    EngineeringChoice, EngineeringChoiceDiscovery, Error, ErrorKind, InteractionAxis,
    InteractionConclusion, NoIndependentForkBasis, ResidualForkClosure,
};
use std::collections::BTreeSet;

fn invalid(message: &str) -> Error {
    Error::new(ErrorKind::InvalidInput, message)
}
fn text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 4096
}
fn identities(values: &[String]) -> bool {
    values.len() <= 64
        && values.iter().all(|v| text(v))
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

pub(crate) fn validate_interactions(discovery: &EngineeringChoiceDiscovery) -> Result<(), Error> {
    if discovery.interaction_review.len() != InteractionAxis::ALL.len()
        || discovery
            .interaction_review
            .iter()
            .map(|r| r.axis)
            .collect::<BTreeSet<_>>()
            != InteractionAxis::ALL.into_iter().collect()
    {
        return Err(invalid(
            "interaction review must challenge every bounded interaction axis exactly once",
        ));
    }
    let choices = discovery
        .choices
        .iter()
        .map(|c| &c.choice_id)
        .collect::<BTreeSet<_>>();
    let mut outcomes = BTreeSet::new();
    for review in &discovery.interaction_review {
        if review.outcomes.is_empty() || review.outcomes.len() > 64 {
            return Err(invalid("each interaction axis requires bounded concrete outcomes, including source-grounded outside-scope conclusions"));
        }
        for outcome in &review.outcomes {
            if !text(&outcome.outcome_id)
                || !outcomes.insert(&outcome.outcome_id)
                || !text(&outcome.scenario)
                || outcome.credible_outcomes.is_empty()
                || outcome.credible_outcomes.len() > 64
                || outcome
                    .credible_outcomes
                    .iter()
                    .any(|r| !text(&r.result_id) || !text(&r.description))
                || outcome
                    .credible_outcomes
                    .iter()
                    .map(|r| &r.result_id)
                    .collect::<BTreeSet<_>>()
                    .len()
                    != outcome.credible_outcomes.len()
                || !identities(&outcome.affected_choice_ids)
                || outcome
                    .affected_choice_ids
                    .iter()
                    .any(|id| !choices.contains(id))
                || outcome.source_basis.is_empty()
                || outcome.source_basis.len() > 64
                || outcome.source_basis.iter().collect::<BTreeSet<_>>().len()
                    != outcome.source_basis.len()
            {
                return Err(invalid("interaction outcomes require unique identities, concrete scenario/results, real affected choices and bounded Source basis"));
            }
            match &outcome.conclusion {
                InteractionConclusion::RepresentedByChoices { choice_ids } => {
                    if choice_ids.is_empty()
                        || !identities(choice_ids)
                        || choice_ids
                            .iter()
                            .any(|id| !outcome.affected_choice_ids.contains(id))
                        || outcome.credible_outcomes.len() < 2
                    {
                        return Err(invalid("independent interaction outcomes require real affected representing choices and distinct credible result identities"));
                    }
                }
                InteractionConclusion::NoIndependentFork {
                    basis,
                    rationale,
                    result_id,
                } => {
                    if !text(rationale)
                        || !outcome
                            .credible_outcomes
                            .iter()
                            .any(|r| &r.result_id == result_id)
                        || (outcome.affected_choice_ids.is_empty()
                            && *basis != NoIndependentForkBasis::OutsideAffectedScope)
                    {
                        return Err(invalid("no-independent-fork interaction requires typed source-grounded rationale and affected choices unless outside scope"));
                    }
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_atomic_interactions(
    discovery: &EngineeringChoiceDiscovery,
    choice: &EngineeringChoice,
    closure: &ResidualForkClosure,
) -> Result<(), Error> {
    let applicable = discovery
        .interaction_review
        .iter()
        .flat_map(|r| &r.outcomes)
        .filter(|o| o.affected_choice_ids.contains(&choice.choice_id))
        .collect::<Vec<_>>();
    if closure.interaction_comparisons.len() != applicable.len()
        || closure
            .interaction_comparisons
            .iter()
            .map(|c| &c.outcome_id)
            .collect::<BTreeSet<_>>()
            != applicable.iter().map(|o| &o.outcome_id).collect()
    {
        return Err(invalid(
            "atomic closure must compare every applicable interaction outcome exactly once",
        ));
    }
    for comparison in &closure.interaction_comparisons {
        let outcome = applicable
            .iter()
            .find(|o| o.outcome_id == comparison.outcome_id)
            .ok_or_else(|| invalid("atomic interaction comparison refers to an unknown outcome"))?;
        if let InteractionConclusion::RepresentedByChoices { choice_ids } = &outcome.conclusion {
            if choice_ids.as_slice() != [choice.choice_id.clone()] {
                return Err(invalid("atomic closure contradicts an independent interaction outcome; decompose into its representing subordinate choices"));
            }
        }
        if let InteractionConclusion::NoIndependentFork { result_id, .. } = &outcome.conclusion {
            if comparison
                .implementation_outcome_ids
                .iter()
                .any(|id| id != result_id)
            {
                return Err(invalid(
                    "atomic comparison contradicts the Source-grounded interaction result",
                ));
            }
        }
        if comparison.implementation_outcome_ids.len() != closure.credible_implementations.len()
            || comparison
                .implementation_outcome_ids
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != 1
            || comparison
                .implementation_outcome_ids
                .iter()
                .any(|id| !outcome.credible_outcomes.iter().any(|r| &r.result_id == id))
            || !text(&comparison.equivalence_rationale)
            || !outcome
                .source_basis
                .iter()
                .any(|source| closure.source_basis.contains(source))
        {
            return Err(invalid("atomic interaction comparison must ground identical result identities for all credible implementations in current closure Sources"));
        }
    }
    Ok(())
}

pub(crate) fn validate_decomposed_interactions(
    discovery: &EngineeringChoiceDiscovery,
    choice: &EngineeringChoice,
    children: &[String],
) -> Result<(), Error> {
    let mut reachable = BTreeSet::new();
    let mut pending = children.to_vec();
    while let Some(id) = pending.pop() {
        if reachable.insert(id.clone()) {
            if let Some(child) = discovery.choices.iter().find(|c| c.choice_id == id) {
                for alternative in &child.alternatives {
                    if let crate::MaterialDecomposition::Decomposed { choice_ids } =
                        &alternative.material_decomposition
                    {
                        pending.extend(choice_ids.iter().cloned());
                    }
                }
            }
        }
    }
    for outcome in discovery
        .interaction_review
        .iter()
        .flat_map(|r| &r.outcomes)
        .filter(|o| o.affected_choice_ids.contains(&choice.choice_id))
    {
        if let InteractionConclusion::RepresentedByChoices { choice_ids } = &outcome.conclusion {
            if choice_ids
                .iter()
                .any(|id| id != &choice.choice_id && !reachable.contains(id))
            {
                return Err(invalid(
                    "material decomposition omits an applicable independent interaction choice",
                ));
            }
        }
    }
    Ok(())
}

/// A declared independent result cannot disappear behind two equal descriptions.
pub(crate) fn validate_result_coverage(
    discovery: &EngineeringChoiceDiscovery,
) -> Result<(), Error> {
    for outcome in discovery
        .interaction_review
        .iter()
        .flat_map(|r| &r.outcomes)
    {
        let InteractionConclusion::RepresentedByChoices { choice_ids } = &outcome.conclusion else {
            continue;
        };
        let mut pending = choice_ids.clone();
        let mut visited = BTreeSet::new();
        let mut represented = BTreeSet::new();
        let mut evidence_required = false;
        while let Some(id) = pending.pop() {
            if !visited.insert(id.clone()) {
                continue;
            }
            if let Some(choice) = discovery.choices.iter().find(|c| c.choice_id == id) {
                evidence_required |=
                    choice.evidence_state != crate::EngineeringChoiceEvidenceState::Sufficient;
                for alternative in &choice.alternatives {
                    match &alternative.material_decomposition {
                        crate::MaterialDecomposition::Decomposed { choice_ids } => {
                            pending.extend(choice_ids.iter().cloned())
                        }
                        crate::MaterialDecomposition::MateriallyAtomic {
                            residual_fork_closure,
                            ..
                        } => {
                            for comparison in &residual_fork_closure.interaction_comparisons {
                                if comparison.outcome_id == outcome.outcome_id {
                                    represented
                                        .extend(comparison.implementation_outcome_ids.iter());
                                }
                            }
                        }
                    }
                }
            }
        }
        if !evidence_required
            && represented
                != outcome
                    .credible_outcomes
                    .iter()
                    .map(|r| &r.result_id)
                    .collect()
        {
            return Err(invalid("every declared independent interaction result must be represented by an alternative in its current choice decomposition"));
        }
    }
    Ok(())
}
