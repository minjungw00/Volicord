use crate::policy::evidence::{state_record_ref_identity_key, unique_state_record_refs};
use std::collections::BTreeSet;
use volicord_types::schema::{
    CloseReadinessBlocker, NextActionSummary, RequiredNullable, StateRecordRef,
};
use volicord_types::values::{
    MethodName, NextActionKind, NextActionPresentationRole, OperationCategory, TaskMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StateGuidance {
    RecordAdvisoryUpdate,
    PrepareWrite,
    CreateAdvisoryChangeUnit,
    CreateWriteChangeUnit,
}

pub(crate) fn guidance_for_state(task_mode: TaskMode, has_change_unit: bool) -> StateGuidance {
    match (task_mode, has_change_unit) {
        (TaskMode::Advisor, true) => StateGuidance::RecordAdvisoryUpdate,
        (_, true) => StateGuidance::PrepareWrite,
        (TaskMode::Advisor, false) => StateGuidance::CreateAdvisoryChangeUnit,
        (_, false) => StateGuidance::CreateWriteChangeUnit,
    }
}

pub(crate) fn next_actions_for_state(
    task_mode: TaskMode,
    task_ref: &StateRecordRef,
    change_unit_ref: Option<&StateRecordRef>,
    expected_state_version: u64,
) -> Vec<NextActionSummary> {
    let guidance = guidance_for_state(task_mode, change_unit_ref.is_some());
    let (action_kind, owner_method, label, required_refs) = match guidance {
        StateGuidance::RecordAdvisoryUpdate => (
            NextActionKind::RecordRun,
            MethodName::RecordRun,
            "Record an advisory shaping update for the current Change Unit.",
            vec![
                task_ref.clone(),
                change_unit_ref
                    .expect("typed guidance requires the selected Change Unit")
                    .clone(),
            ],
        ),
        StateGuidance::PrepareWrite => (
            NextActionKind::PrepareWrite,
            MethodName::PrepareWrite,
            "Check the current change against current scope.",
            vec![
                task_ref.clone(),
                change_unit_ref
                    .expect("typed guidance requires the selected Change Unit")
                    .clone(),
            ],
        ),
        StateGuidance::CreateAdvisoryChangeUnit => (
            NextActionKind::UpdateScope,
            MethodName::UpdateScope,
            "Create the first currently applied Change Unit before recording advisory shaping.",
            vec![task_ref.clone()],
        ),
        StateGuidance::CreateWriteChangeUnit => (
            NextActionKind::UpdateScope,
            MethodName::UpdateScope,
            "Create the first currently applied Change Unit before write-ticket preparation.",
            vec![task_ref.clone()],
        ),
    };
    vec![NextActionSummary {
        presentation_role: NextActionPresentationRole::Primary,
        action_kind,
        owner_method: Some(owner_method),
        allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
        label: label.to_owned(),
        blocking_question: None,
        expected_state_version: RequiredNullable::some(expected_state_version),
        required_refs,
    }]
}

pub(crate) fn normalize_next_action_collection(
    actions: &mut [NextActionSummary],
    expected_state_version: u64,
) {
    for (index, action) in actions.iter_mut().enumerate() {
        action.presentation_role = if index == 0 {
            NextActionPresentationRole::Primary
        } else {
            NextActionPresentationRole::Additional
        };
        action.allowed_operation_categories = allowed_operation_categories(action.owner_method);
        action.expected_state_version = expected_state_version_for(
            &action.allowed_operation_categories,
            expected_state_version,
        );
    }
}

pub(crate) fn unique_next_actions(actions: Vec<NextActionSummary>) -> Vec<NextActionSummary> {
    let mut seen = BTreeSet::new();
    actions
        .into_iter()
        .filter_map(|mut action| {
            action.required_refs = unique_state_record_refs(action.required_refs);
            let mut required_ref_keys = action
                .required_refs
                .iter()
                .map(state_record_ref_identity_key)
                .collect::<Vec<_>>();
            required_ref_keys.sort();
            let key = serde_json::to_string(&(
                &action.action_kind,
                &action.owner_method,
                &action.allowed_operation_categories,
                &action.label,
                &action.blocking_question,
                required_ref_keys,
            ))
            .expect("serializing the closed action identity tuple cannot fail");
            seen.insert(key).then_some(action)
        })
        .collect()
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

pub(crate) fn primary_next_action<'a>(
    next_actions: &'a [NextActionSummary],
    close_blockers: &'a [CloseReadinessBlocker],
) -> Option<&'a NextActionSummary> {
    next_actions
        .iter()
        .find(|action| action.presentation_role == NextActionPresentationRole::Primary)
        .or_else(|| {
            close_blockers
                .iter()
                .flat_map(|blocker| blocker.next_actions.iter())
                .find(|action| action.presentation_role == NextActionPresentationRole::Primary)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::{
        ids::{ProjectId, RecordId, TaskId},
        values::StateRecordKind,
    };

    #[test]
    fn state_guidance_selection_is_typed() {
        assert_eq!(
            guidance_for_state(TaskMode::Advisor, true),
            StateGuidance::RecordAdvisoryUpdate
        );
        assert_eq!(
            guidance_for_state(TaskMode::Work, true),
            StateGuidance::PrepareWrite
        );
        assert_eq!(
            guidance_for_state(TaskMode::Advisor, false),
            StateGuidance::CreateAdvisoryChangeUnit
        );
        assert_eq!(
            guidance_for_state(TaskMode::Direct, false),
            StateGuidance::CreateWriteChangeUnit
        );
    }

    #[test]
    fn semantic_next_actions_are_normalized_deduplicated_and_selected_by_role() {
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
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::RecordRun,
            owner_method: Some(MethodName::RecordRun),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label: "Record the current result.".to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::null(),
            required_refs: Vec::new(),
        };
        let mut additional_duplicate = primary.clone();
        additional_duplicate.presentation_role = NextActionPresentationRole::Additional;
        additional_duplicate.expected_state_version = RequiredNullable::some(41);

        let deduplicated = unique_next_actions(vec![additional_duplicate.clone(), primary.clone()]);
        assert_eq!(deduplicated.len(), 1);

        let distinct_additional = NextActionSummary {
            label: "Additional action.".to_owned(),
            ..additional_duplicate
        };
        let reordered = [distinct_additional, primary.clone()];
        let selected = primary_next_action(&reordered, &[])
            .expect("primary action should be selected by role");
        assert_eq!(selected, &primary);

        let older_ref = StateRecordRef {
            record_kind: StateRecordKind::Task,
            record_id: RecordId::new("task_same_identity"),
            project_id: ProjectId::new("project_guidance"),
            task_id: Some(TaskId::new("task_context_old")).into(),
            produced_at_state_version: Some(3).into(),
        };
        let newer_ref = StateRecordRef {
            task_id: Some(TaskId::new("task_context_new")).into(),
            produced_at_state_version: Some(8).into(),
            ..older_ref.clone()
        };
        let deduplicated_refs = unique_next_actions(vec![NextActionSummary {
            required_refs: vec![newer_ref.clone(), older_ref],
            ..primary.clone()
        }]);
        assert_eq!(deduplicated_refs[0].required_refs, vec![newer_ref]);

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
