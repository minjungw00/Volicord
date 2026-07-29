use super::*;
use crate::close_readiness::test_support;
use volicord_types::values::CloseReadinessBlockerCategory;

#[test]
fn unresolved_changes_produce_the_change_control_blocker() {
    let mut facts = test_support::facts();
    facts
        .unresolved_unrecorded_changes
        .push(test_support::unresolved_change());

    let blockers = unrecorded_change_blockers(
        &test_support::project_state(),
        &test_support::request(),
        &facts,
    );

    assert_eq!(blockers.len(), 1);
    assert_eq!(
        blockers[0].category,
        CloseReadinessBlockerCategory::ConnectionCapability
    );
    assert_eq!(blockers[0].code, "unresolved_unrecorded_changes");
}
