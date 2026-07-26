use volicord_types::schema::{NextActionSummary, RequiredNullable, StateRecordRef};
use volicord_types::values::{
    MethodName, NextActionKind, NextActionPresentationRole, OperationCategory,
};

pub(super) fn close_next_action(
    label: &str,
    required_refs: Vec<StateRecordRef>,
) -> NextActionSummary {
    NextActionSummary {
        presentation_role: NextActionPresentationRole::Primary,
        action_kind: NextActionKind::CloseTask,
        owner_method: Some(MethodName::CloseTask),
        allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
        label: label.to_owned(),
        blocking_question: None,
        expected_state_version: RequiredNullable::null(),
        required_refs,
    }
}
