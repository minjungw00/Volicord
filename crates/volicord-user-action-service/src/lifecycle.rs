use crate::model::UserActionAuthority;
use volicord_store::core_pipeline::{ChangeUnitRecord, ProjectStateHeader, TaskRecord};
use volicord_types::ids::{ChangeUnitId, TaskId};

/// Derives the Task lifecycle phase after applying pending UserAction facts.
pub fn projected_user_action_lifecycle_phase(
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

fn is_terminal_lifecycle(value: &str) -> bool {
    matches!(value, "completed" | "cancelled" | "superseded")
}

fn user_action_keeps_task_waiting(
    user_action: &UserActionAuthority,
    task_id: &TaskId,
    current_change_unit_id: Option<&ChangeUnitId>,
    scope_revision: u64,
) -> bool {
    use volicord_types::values::{UserActionBasisStatus, UserActionRequiredFor, UserActionStatus};

    if user_action.status != UserActionStatus::Pending
        || user_action.basis_status != UserActionBasisStatus::Current
        || user_action.task_id != *task_id
        || !user_action
            .required_for
            .iter()
            .any(|target| *target != UserActionRequiredFor::Informational)
    {
        return false;
    }
    user_action.basis.as_ref().is_some_and(|basis| {
        basis.compatibility_status() == UserActionBasisStatus::Current
            && basis.coordinates().task_id == *task_id
            && basis.coordinates().scope_revision == scope_revision
            && basis.coordinates().change_unit_id.as_ref() == current_change_unit_id
    })
}
