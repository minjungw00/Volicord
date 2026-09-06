//! Prospective graph binding, not inference of semantic truth from prose.
use crate::{
    DiscoveredAlternativeResolution, EngineeringChoiceDiscovery, Error, ErrorKind,
    InteractionConclusion, MaterialDecomposition, MaterialityReview, NoIndependentForkBasis,
    PlannedCommitment, PlannedOutcomeBinding,
};
use std::collections::BTreeSet;
use volicord_context::ApplicabilityScope;

fn text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 4096
}
pub(crate) fn validate_shape(
    commitments: &[PlannedCommitment],
    scope: &ApplicabilityScope,
) -> Result<(), Error> {
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    if commitments.is_empty() || commitments.len() > 64 {
        return Err(Error::new(ErrorKind::InvalidInput, "pre-write closure requires bounded concrete planned commitments or explicit private equivalence"));
    }
    for commitment in commitments {
        let binding_valid = match &commitment.outcome_binding {
            PlannedOutcomeBinding::ReviewedChoice {
                dimension_id,
                choice_id,
                alternative_id,
            } => [dimension_id, choice_id, alternative_id]
                .into_iter()
                .all(|id| text(id)),
            PlannedOutcomeBinding::ReviewedInteraction {
                outcome_id,
                result_id,
            } => text(outcome_id) && text(result_id),
            PlannedOutcomeBinding::PrivateEquivalent {
                equivalence_rationale,
            } => text(equivalence_rationale),
        };
        if !text(&commitment.commitment_id)
            || !ids.insert(&commitment.commitment_id)
            || !text(&commitment.description)
            || !binding_valid
            || commitment.repository_paths.len() > 64
            || commitment
                .repository_paths
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != commitment.repository_paths.len()
            || (commitment.repository_paths.is_empty() && !scope.paths.is_empty())
            || commitment
                .repository_paths
                .iter()
                .any(|p| !scope.paths.contains(p))
        {
            return Err(Error::new(ErrorKind::InvalidInput, "planned commitments require unique identities, concrete descriptions, exact planned paths and closed outcome bindings"));
        }
        paths.extend(commitment.repository_paths.iter());
    }
    if paths != scope.paths.iter().collect() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "planned commitments must account for every executable artifact path",
        ));
    }
    Ok(())
}

pub(crate) fn unmapped_commitments(
    commitments: &[PlannedCommitment],
    review: &MaterialityReview,
    discovery: &EngineeringChoiceDiscovery,
) -> Vec<String> {
    let allowed = |dimension_id: &str, choice_id: &str, alternative_id: &str| {
        review.dimensions.iter().any(|dimension| {
            dimension.dimension_id == dimension_id
                && dimension
                    .discovered_choice_ids
                    .iter()
                    .any(|id| id == choice_id)
                && dimension.alternative_accounting.iter().any(|account| {
                    account.choice_id == choice_id
                        && account.alternative_id == alternative_id
                        && matches!(
                            account.resolution,
                            DiscoveredAlternativeResolution::Selected
                                | DiscoveredAlternativeResolution::Unresolved
                        )
                })
        })
    };
    commitments.iter().filter(|commitment| !match &commitment.outcome_binding {
        PlannedOutcomeBinding::ReviewedChoice { dimension_id, choice_id, alternative_id } => {
            allowed(dimension_id, choice_id, alternative_id)
                && discovery.choices.iter().any(|choice| &choice.choice_id == choice_id
                    && choice.alternatives.iter().any(|alt| &alt.alternative_id == alternative_id))
        }
        PlannedOutcomeBinding::ReviewedInteraction { outcome_id, result_id } => {
            discovery.interaction_review.iter().flat_map(|r| &r.outcomes).any(|outcome| {
                if &outcome.outcome_id != outcome_id { return false; }
                match &outcome.conclusion {
                    InteractionConclusion::NoIndependentFork { basis, result_id: fixed, .. } =>
                        *basis != NoIndependentForkBasis::OutsideAffectedScope && fixed == result_id,
                    InteractionConclusion::RepresentedByChoices { choice_ids } => choice_ids.iter().all(|id| {
                        discovery.choices.iter().find(|c| &c.choice_id == id).is_some_and(|choice| {
                            choice.alternatives.iter().any(|alt| {
                                let dimension = review.dimensions.iter().find(|d| d.discovered_choice_ids.contains(id));
                                dimension.is_some_and(|d| allowed(&d.dimension_id, id, &alt.alternative_id))
                                    && matches!(&alt.material_decomposition, MaterialDecomposition::MateriallyAtomic { residual_fork_closure, .. }
                                        if residual_fork_closure.interaction_comparisons.iter().any(|c| &c.outcome_id == outcome_id && c.implementation_outcome_ids.iter().all(|r| r == result_id)))
                            })
                        })
                    }),
                }
            })
        }
        PlannedOutcomeBinding::PrivateEquivalent { .. } => true,
    }).map(|c| format!("{}: {}", c.commitment_id, c.description)).collect()
}
