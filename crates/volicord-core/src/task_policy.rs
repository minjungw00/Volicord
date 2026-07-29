use crate::pipeline::{CorePipelineError, CoreResult};
use volicord_store::core_pipeline::{
    CoreStorageMutation, TaskMutation, TaskRecord, TaskScopeUpdate,
};
use volicord_types::ids::TaskId;
use volicord_types::values::{RequestedMode, TaskLifecyclePhase, TaskMode, WorkPhase};

pub(crate) fn resolve_requested_mode(requested_mode: RequestedMode) -> TaskMode {
    match requested_mode {
        RequestedMode::Advisor => TaskMode::Advisor,
        RequestedMode::Direct => TaskMode::Direct,
        RequestedMode::Work | RequestedMode::Auto => TaskMode::Work,
    }
}

pub(crate) fn initial_work_phase(mode: TaskMode) -> WorkPhase {
    match mode {
        TaskMode::Direct => WorkPhase::Implementation,
        TaskMode::Advisor | TaskMode::Work => WorkPhase::Shaping,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskLifecycleTransition {
    task_id: TaskId,
    from: TaskLifecyclePhase,
    to: TaskLifecyclePhase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskLifecycleFacts {
    task_id: TaskId,
    current: TaskLifecyclePhase,
}

impl From<&TaskRecord> for TaskLifecycleFacts {
    fn from(task: &TaskRecord) -> Self {
        Self {
            task_id: TaskId::new(task.task_id.clone()),
            current: task.lifecycle_phase,
        }
    }
}

impl TaskLifecycleTransition {
    pub(crate) fn target(&self) -> TaskLifecyclePhase {
        self.to
    }

    pub(crate) fn task_mutation(&self) -> TaskMutation {
        TaskMutation::UpdateScope(TaskScopeUpdate {
            task_id: self.task_id.as_str().to_owned(),
            work_phase: None,
            lifecycle_phase: Some(self.to),
            result: None,
            title: None,
            summary: None,
            shaping: None,
            bounded_context: None,
            autonomy_boundary: None,
            close_summary: None,
        })
    }

    pub(crate) fn storage_mutation(&self) -> CoreStorageMutation {
        CoreStorageMutation::Task(self.task_mutation())
    }
}

pub(crate) fn plan_user_action_lifecycle_transition(
    facts: TaskLifecycleFacts,
    target: TaskLifecyclePhase,
) -> CoreResult<Option<TaskLifecycleTransition>> {
    let from = facts.current;
    if from == target {
        return Ok(None);
    }
    let valid = matches!(
        (from, target),
        (
            TaskLifecyclePhase::Shaping
                | TaskLifecyclePhase::Ready
                | TaskLifecyclePhase::Executing
                | TaskLifecyclePhase::Blocked,
            TaskLifecyclePhase::WaitingUser,
        ) | (
            TaskLifecyclePhase::WaitingUser,
            TaskLifecyclePhase::Shaping | TaskLifecyclePhase::Ready,
        )
    );
    if !valid {
        return Err(CorePipelineError::Invariant {
            detail: format!(
                "typed UserAction lifecycle facts cannot transition Task `{}` from `{}` to `{}`",
                facts.task_id,
                from.as_str(),
                target.as_str()
            ),
        });
    }
    Ok(Some(TaskLifecycleTransition {
        task_id: facts.task_id,
        from,
        to: target,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(current: TaskLifecyclePhase) -> TaskLifecycleFacts {
        TaskLifecycleFacts {
            task_id: TaskId::new("task_lifecycle_policy"),
            current,
        }
    }

    #[test]
    fn user_action_transition_is_typed_and_builds_the_task_mutation() {
        let transition = plan_user_action_lifecycle_transition(
            facts(TaskLifecyclePhase::Ready),
            TaskLifecyclePhase::WaitingUser,
        )
        .expect("transition facts should be valid")
        .expect("phase change should produce a transition");
        assert_eq!(transition.target(), TaskLifecyclePhase::WaitingUser);
        assert!(matches!(
            transition.task_mutation(),
            TaskMutation::UpdateScope(TaskScopeUpdate {
                lifecycle_phase: Some(TaskLifecyclePhase::WaitingUser),
                ..
            })
        ));
    }

    #[test]
    fn terminal_user_action_transition_is_rejected() {
        assert!(plan_user_action_lifecycle_transition(
            facts(TaskLifecyclePhase::Completed),
            TaskLifecyclePhase::WaitingUser
        )
        .is_err());
    }
}
