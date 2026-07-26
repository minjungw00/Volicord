use crate::policy::close_readiness::{is_terminal_lifecycle, UserActionAuthority};
use crate::policy::user_action_relevance::user_action_keeps_task_waiting;
use volicord_store::core_pipeline::{ChangeUnitRecord, ProjectStateHeader, TaskRecord};
use volicord_types::ids::{ChangeUnitId, TaskId};

/// Derives the Task lifecycle phase after applying pending UserAction facts.
pub(crate) fn projected_user_action_lifecycle_phase(
    project_state: &ProjectStateHeader,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    pending_authorities: &[UserActionAuthority],
) -> Option<&'static str> {
    if project_state.active_task_id.as_deref() != Some(task.task_id.as_str())
        || is_terminal_lifecycle(&task.lifecycle_phase)
    {
        return None;
    }

    let task_id = TaskId::new(task.task_id.clone());
    let current_change_unit_id =
        current_change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let waits_for_user = pending_authorities.iter().any(|authority| {
        user_action_keeps_task_waiting(
            authority,
            &task_id,
            current_change_unit_id.as_ref(),
            task.scope_revision,
        )
    });
    let next_phase = if waits_for_user {
        "waiting_user"
    } else if task.lifecycle_phase == "waiting_user" {
        if current_change_unit.is_some() {
            "ready"
        } else {
            "shaping"
        }
    } else {
        return None;
    };

    (task.lifecycle_phase != next_phase).then_some(next_phase)
}
