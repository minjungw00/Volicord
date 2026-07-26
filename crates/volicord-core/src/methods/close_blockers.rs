use super::{allowed_operation_categories, next_action_expected_state_version};
use volicord_types::schema::{
    CloseReadinessBlocker, NextActionSummary, RequiredNullable, StateRecordRef,
};
use volicord_types::values::{
    CloseReadinessBlockerCategory, MethodName, NextActionKind, NextActionPresentationRole,
    OperationCategory,
};

pub(super) fn close_blocker(
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

pub(super) fn open_write_ticket_close_blocker(
    task_ref: StateRecordRef,
    write_ticket_ref: StateRecordRef,
) -> CloseReadinessBlocker {
    close_blocker(
        CloseReadinessBlockerCategory::WriteCompatibility,
        "open_write_ticket",
        "An open write ticket remains unresolved for this Task.",
        vec![write_ticket_ref],
        vec![NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::RecordRun,
            owner_method: Some(MethodName::RecordRun),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label: "Record the ticket-backed run or reconcile observed changes before close."
                .to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::null(),
            required_refs: vec![task_ref],
        }],
    )
}

pub(super) fn normalize_close_blockers(
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
