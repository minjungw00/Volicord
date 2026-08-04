use volicord_types::schema::{NextActionSummary, RequiredNullable};
use volicord_types::values::{MethodName, OperationCategory};

pub(crate) fn normalize_next_action_collection(
    actions: &mut [NextActionSummary],
    expected_state_version: u64,
) {
    for action in actions.iter_mut() {
        action.allowed_operation_categories = allowed_operation_categories(action.owner_method);
        action.expected_state_version = expected_state_version_for(
            &action.allowed_operation_categories,
            expected_state_version,
        );
    }
}

pub(crate) fn expected_state_version_for(
    allowed_operation_categories: &[OperationCategory],
    expected_state_version: u64,
) -> RequiredNullable<u64> {
    if allowed_operation_categories.contains(&OperationCategory::AgentWorkflow) {
        RequiredNullable::some(expected_state_version)
    } else {
        RequiredNullable::null()
    }
}

pub(crate) fn allowed_operation_categories(
    owner_method: Option<MethodName>,
) -> Vec<OperationCategory> {
    match owner_method {
        Some(MethodName::ResolveUserAction) => vec![OperationCategory::UserOnly],
        Some(MethodName::ReconcileChanges) => vec![
            OperationCategory::AgentWorkflow,
            OperationCategory::LocalRecovery,
        ],
        Some(
            MethodName::UpdateScope
            | MethodName::RecordShapingCheckpoint
            | MethodName::FinalizeAdvice
            | MethodName::AdvanceTask
            | MethodName::PrepareEvidenceCapture
            | MethodName::PrepareWrite
            | MethodName::StageArtifact
            | MethodName::RecordRun
            | MethodName::RequestUserAction
            | MethodName::CloseTask,
        ) => vec![OperationCategory::AgentWorkflow],
        Some(
            MethodName::Intake
            | MethodName::Status
            | MethodName::GetOperationResult
            | MethodName::CheckClose,
        )
        | None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::values::NextActionKind;

    #[test]
    fn semantic_next_actions_are_normalized() {
        for owner_method in [
            MethodName::UpdateScope,
            MethodName::PrepareWrite,
            MethodName::StageArtifact,
            MethodName::RecordRun,
            MethodName::RequestUserAction,
            MethodName::CloseTask,
        ] {
            assert_eq!(
                allowed_operation_categories(Some(owner_method)),
                vec![OperationCategory::AgentWorkflow]
            );
        }
        assert_eq!(
            allowed_operation_categories(Some(MethodName::ResolveUserAction)),
            vec![OperationCategory::UserOnly]
        );
        assert_eq!(
            allowed_operation_categories(Some(MethodName::ReconcileChanges)),
            vec![
                OperationCategory::AgentWorkflow,
                OperationCategory::LocalRecovery,
            ]
        );
        assert!(allowed_operation_categories(None).is_empty());

        let primary = NextActionSummary {
            action_kind: NextActionKind::RecordRun,
            owner_method: Some(MethodName::RecordRun),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label: "Record the current result.".to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::null(),
            required_refs: Vec::new(),
        };
        let mut user_only_action = NextActionSummary {
            owner_method: Some(MethodName::ResolveUserAction),
            expected_state_version: RequiredNullable::some(99),
            ..primary.clone()
        };
        normalize_next_action_collection(std::slice::from_mut(&mut user_only_action), 8);
        assert_eq!(
            user_only_action.allowed_operation_categories,
            vec![OperationCategory::UserOnly]
        );
        assert!(user_only_action.expected_state_version.is_none());

        let mut read_action = NextActionSummary {
            owner_method: Some(MethodName::Status),
            expected_state_version: RequiredNullable::some(99),
            ..primary
        };
        normalize_next_action_collection(std::slice::from_mut(&mut read_action), 8);
        assert!(read_action.allowed_operation_categories.is_empty());
        assert!(read_action.expected_state_version.is_none());
    }
}
