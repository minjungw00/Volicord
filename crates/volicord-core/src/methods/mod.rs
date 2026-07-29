use volicord_store::core_pipeline::CoreStorageMutation;
use volicord_types::ids::{ChangeUnitId, TaskId};
use volicord_types::schema::{JsonObject, NextActionSummary};

mod close_task;
mod intake;
mod operation_result;
mod prepare_evidence_capture;
mod prepare_write;
mod reconcile_changes;
mod record_run;
mod stage_artifact;
mod status;
#[cfg(test)]
mod tests;
mod update_scope;
mod user_action;
mod user_action_read;

pub(super) struct MethodPlan<F> {
    pub(super) task_id: TaskId,
    pub(super) change_unit_id: Option<ChangeUnitId>,
    pub(super) storage_mutations: Vec<CoreStorageMutation>,
    pub(super) event_payload: JsonObject,
    pub(super) result_fields: F,
    pub(super) next_actions: Vec<NextActionSummary>,
}
