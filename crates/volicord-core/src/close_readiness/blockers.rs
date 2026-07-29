use super::guidance::{close_guidance, CloseGuidance};
use crate::projection::{allowed_operation_categories, next_action_expected_state_version};
use volicord_types::schema::{CloseReadinessBlocker, NextActionSummary, StateRecordRef};
use volicord_types::values::{CloseReadinessBlockerCategory, NextActionPresentationRole};

pub(crate) fn close_blocker(
    category: CloseReadinessBlockerCategory,
    code: &'static str,
    message: impl Into<String>,
    related_refs: Vec<StateRecordRef>,
    next_actions: Vec<NextActionSummary>,
) -> CloseReadinessBlocker {
    CloseReadinessBlocker {
        category,
        code: code.to_owned(),
        message: message.into(),
        related_refs,
        next_actions,
    }
}

pub(crate) fn open_write_ticket_close_blocker(
    task_ref: StateRecordRef,
    write_ticket_ref: StateRecordRef,
) -> CloseReadinessBlocker {
    close_blocker(
        CloseReadinessBlockerCategory::WriteCompatibility,
        "open_write_ticket",
        "An open write ticket remains unresolved for this Task.",
        vec![write_ticket_ref],
        vec![close_guidance(
            CloseGuidance::RecordOpenTicket,
            vec![task_ref],
        )],
    )
}

pub(crate) fn normalize_close_blockers(
    blockers: &mut [CloseReadinessBlocker],
    expected_state_version: u64,
) {
    for (action_index, action) in blockers
        .iter_mut()
        .flat_map(|blocker| blocker.next_actions.iter_mut())
        .enumerate()
    {
        action.presentation_role = if action_index == 0 {
            NextActionPresentationRole::Primary
        } else {
            NextActionPresentationRole::Additional
        };
        action.allowed_operation_categories = allowed_operation_categories(action.owner_method);
        action.expected_state_version = next_action_expected_state_version(
            &action.allowed_operation_categories,
            expected_state_version,
        );
    }
}

#[cfg(test)]
#[path = "tests/blockers.rs"]
mod tests;
