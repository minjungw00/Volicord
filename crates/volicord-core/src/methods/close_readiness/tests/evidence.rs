use super::*;
use volicord_types::ids::{AcceptanceCriterionId, EvidenceClaimId};

#[test]
fn evidence_requirement_filter_selects_only_current_required_criteria() {
    let required = BTreeSet::from(["criterion_required".to_owned()]);
    let required_target = EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id: AcceptanceCriterionId::new("criterion_required"),
    };
    let optional_target = EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id: AcceptanceCriterionId::new("criterion_optional"),
    };
    let supplemental_target = EvidenceTarget::SupplementalClaim {
        evidence_claim_id: EvidenceClaimId::new("claim_supplemental"),
        statement: "supplemental".to_owned(),
    };

    assert!(evidence_target_required_by(&required_target, &required));
    assert!(!evidence_target_required_by(&optional_target, &required));
    assert!(!evidence_target_required_by(
        &supplemental_target,
        &required
    ));
}
