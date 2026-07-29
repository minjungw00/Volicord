use volicord_store::core_pipeline::CoreStorageMutation;
use volicord_types::ids::{ChangeUnitId, TaskId};
use volicord_types::methods::MethodResponseContract;
use volicord_types::schema::JsonObject;

use crate::pipeline::CommitMutationBranch;

/// Method-neutral execution inputs for one task-scoped Core mutation.
pub(crate) struct OperationPlan {
    task_id: TaskId,
    change_unit_id: Option<ChangeUnitId>,
    storage_mutations: Vec<CoreStorageMutation>,
    event_payload: JsonObject,
}

impl OperationPlan {
    pub(crate) fn new(
        task_id: TaskId,
        change_unit_id: Option<ChangeUnitId>,
        storage_mutations: Vec<CoreStorageMutation>,
        event_payload: JsonObject,
    ) -> Self {
        Self {
            task_id,
            change_unit_id,
            storage_mutations,
            event_payload,
        }
    }

    pub(crate) fn into_commit_branch<M>(
        self,
        result_fields: M::ResultFields,
        event_kind: impl Into<String>,
    ) -> CommitMutationBranch<M>
    where
        M: MethodResponseContract,
    {
        CommitMutationBranch {
            result_fields,
            event_kind: event_kind.into(),
            event_payload: self.event_payload,
            task_id: Some(self.task_id),
            change_unit_id: self.change_unit_id,
            storage_mutations: self.storage_mutations,
        }
    }
}

#[cfg(test)]
mod tests {
    use volicord_types::ids::{ChangeUnitId, TaskId};
    use volicord_types::schema::JsonObject;

    use super::OperationPlan;

    #[test]
    fn operation_plan_preserves_method_neutral_commit_inputs() {
        let plan = OperationPlan::new(
            TaskId::new("task_operation_plan"),
            Some(ChangeUnitId::new("change_operation_plan")),
            Vec::new(),
            JsonObject::new(),
        );

        assert_eq!(plan.task_id.as_str(), "task_operation_plan");
        assert_eq!(
            plan.change_unit_id.as_ref().map(ChangeUnitId::as_str),
            Some("change_operation_plan")
        );
        assert!(plan.storage_mutations.is_empty());
        assert!(plan.event_payload.is_empty());
    }
}
