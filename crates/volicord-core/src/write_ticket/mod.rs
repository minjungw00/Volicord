mod admission;
pub(crate) mod current_validity;
mod facts;
mod planning;
mod policy;
pub(crate) mod read_model;
pub(crate) mod selection;
pub(crate) mod semantic;
pub(crate) mod service;
pub(crate) mod summary;

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

pub(crate) use admission::{admit_record_run, RecordRunWriteAdmission, WriteTicketAdmissionError};
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
