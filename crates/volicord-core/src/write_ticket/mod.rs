mod facts;
mod planning;
mod policy;
mod projection;

use crate::pipeline::CorePipelineError;
use volicord_types::ids::TaskId;
use volicord_user_action_service::UserActionServiceError;

#[derive(Debug)]
pub(crate) enum WriteTicketPlanningError {
    Core(CorePipelineError),
    UserAction(UserActionServiceError),
    NoActiveTask,
    CurrentChangeUnitRequired {
        task_id: TaskId,
    },
    Validation {
        field: &'static str,
        message: &'static str,
    },
    ProductPathContainment {
        field: &'static str,
        message: &'static str,
    },
}

impl From<CorePipelineError> for WriteTicketPlanningError {
    fn from(error: CorePipelineError) -> Self {
        Self::Core(error)
    }
}

impl From<UserActionServiceError> for WriteTicketPlanningError {
    fn from(error: UserActionServiceError) -> Self {
        Self::UserAction(error)
    }
}

pub(crate) use facts::{
    baseline_matches, change_unit_effect_contract, matching_sensitive_approval,
    paths_match_current_change_unit, resolve_prepare_write_task,
    validate_prepare_write_change_unit, workspace_context_matches, SensitiveApprovalSearch,
};
pub(crate) use planning::{plan_prepare_write, PrepareWritePlannedMutations};
pub(crate) use policy::{
    normalized_string_set, prepare_write_decision, run_write_ticket_mismatch,
    write_decision_reason, write_ticket_is_idle_expired, RunWriteTicketAttempt,
};
pub(crate) use projection::{
    effective_write_ticket_status, projected_write_ticket_summary,
    write_ticket_is_current_for_projection, write_ticket_summary_for_record,
};
