use super::*;
use crate::close_readiness::test_support;
use crate::write_ticket::current_validity::test_support::stored_evaluation;
use volicord_types::values::{
    CloseReadinessBlockerCategory, WriteTicketInvalidationReason, WriteTicketStatus,
};

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

#[test]
fn write_ticket_blockers_consume_only_evaluated_stored_states() {
    let evaluations = [
        stored_evaluation("ticket-current", WriteTicketStatus::Active, 6),
        stored_evaluation("ticket-invalidated", WriteTicketStatus::Invalidated, 7),
    ];
    let task_ref = task_ref_for_close(
        &test_support::request(),
        test_support::project_state().state_version,
    );

    let blockers = open_write_ticket_blockers_from_evaluated(task_ref, 7, &evaluations);

    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0].code, "open_write_ticket");
    assert_eq!(
        evaluations[1].invalidation(),
        Some(WriteTicketInvalidationReason::ExplicitRevoke)
    );
}
