use super::*;
use volicord_types::ids::{ProjectId, RecordId, TaskId};
use volicord_types::schema::StateRecordRef;
use volicord_types::values::{MethodName, NextActionKind, OperationCategory, StateRecordKind};

fn task_ref() -> StateRecordRef {
    StateRecordRef {
        record_kind: StateRecordKind::Task,
        record_id: RecordId::new("task_guidance"),
        project_id: ProjectId::new("project_guidance"),
        task_id: Some(TaskId::new("task_guidance")).into(),
        produced_at_state_version: Some(9).into(),
    }
}

#[test]
fn guidance_selects_semantic_owner_and_keeps_pending_action_generic() {
    let action = close_guidance(CloseGuidance::ResolvePendingUserAction, Vec::new());
    assert_eq!(action.action_kind, NextActionKind::ResolveUserAction);
    assert_eq!(action.owner_method, Some(MethodName::ResolveUserAction));
    assert_eq!(
        action.allowed_operation_categories,
        vec![OperationCategory::UserOnly]
    );
    assert_eq!(action.label, "Resolve the pending user action.");
    assert!(action.blocking_question.is_none());
    assert!(action.required_refs.is_empty());
}

#[test]
fn guidance_preserves_required_context_for_reconciliation() {
    let task_ref = task_ref();
    let action = close_guidance(CloseGuidance::ReconcileChanges, vec![task_ref.clone()]);
    assert_eq!(action.action_kind, NextActionKind::ReconcileChanges);
    assert_eq!(action.owner_method, Some(MethodName::ReconcileChanges));
    assert_eq!(action.required_refs, vec![task_ref]);
    assert!(action.blocking_question.is_some());
}
