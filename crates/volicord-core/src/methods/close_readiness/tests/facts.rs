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

#[test]
fn terminal_summary_is_not_promoted_to_the_current_close_basis() {
    let mut task = test_support::task();
    task.close_summary_json = r#"{
        "close_reason":"none",
        "visible_risks":[{"risk_id":"risk-summary-only","summary":"terminal-only risk"}]
    }"#
    .to_owned();

    let facts = facts_from_projection(
        task,
        None,
        None,
        Vec::new(),
        Vec::new(),
        None,
        volicord_types::values::UtcTimestamp::parse("2026-07-27T00:00:00Z").unwrap(),
    );

    assert!(facts.current_close_basis.is_none());
}
