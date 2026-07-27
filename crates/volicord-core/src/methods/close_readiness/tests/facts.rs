use super::*;
use crate::methods::close_readiness::test_support;
use volicord_types::ids::AcceptanceCriterionId;
use volicord_types::schema::AcceptanceCriterion;
use volicord_types::values::EvidenceRequirement;

#[test]
fn projected_acceptance_facts_cache_only_required_criterion_ids() {
    let criteria = vec![
        AcceptanceCriterion {
            acceptance_criterion_id: AcceptanceCriterionId::new("criterion_required"),
            statement: "required".to_owned(),
            evidence_requirement: EvidenceRequirement::Required,
        },
        AcceptanceCriterion {
            acceptance_criterion_id: AcceptanceCriterionId::new("criterion_optional"),
            statement: "optional".to_owned(),
            evidence_requirement: EvidenceRequirement::Optional,
        },
    ];

    let facts = facts_with_projected_acceptance_criteria(test_support::facts(), &criteria);
    let required =
        required_criteria_for_close_context(&facts).expect("projected criteria are cached");

    assert_eq!(
        facts.acceptance_criteria.as_deref(),
        Some(criteria.as_slice())
    );
    assert!(required.contains("criterion_required"));
    assert!(!required.contains("criterion_optional"));
}
