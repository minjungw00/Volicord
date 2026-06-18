use harness_types::{
    CloseReadinessBlocker, CloseReadinessBlockerCategory, MethodName, NextActionKind,
    NextActionSummary, StateRecordRef,
};

pub(crate) fn is_terminal_lifecycle(value: &str) -> bool {
    matches!(value, "completed" | "cancelled" | "superseded")
}

pub(crate) fn close_blocker(
    category: CloseReadinessBlockerCategory,
    code: &'static str,
    message: &'static str,
    related_refs: Vec<StateRecordRef>,
    next_actions: Vec<NextActionSummary>,
) -> CloseReadinessBlocker {
    CloseReadinessBlocker {
        category,
        code: code.to_owned(),
        message: message.to_owned(),
        related_refs,
        next_actions,
    }
}

pub(crate) fn close_next_action(
    label: &str,
    required_refs: Vec<StateRecordRef>,
) -> NextActionSummary {
    NextActionSummary {
        action_kind: NextActionKind::CloseTask,
        owner_method: Some(MethodName::CloseTask),
        label: label.to_owned(),
        blocking_question: None,
        required_refs,
    }
}
