mod acceptance;
mod blockers;
mod change_control;
mod evidence;
mod facts;
mod guidance;
mod policy;
mod recording;
mod service;
mod summary;

use crate::pipeline::CorePipelineError;
use volicord_user_action_service::UserActionServiceError;

#[derive(Debug)]
pub(crate) enum CloseReadinessError {
    Core(CorePipelineError),
    UserAction(UserActionServiceError),
    NoActiveTask,
}

impl From<CorePipelineError> for CloseReadinessError {
    fn from(error: CorePipelineError) -> Self {
        Self::Core(error)
    }
}

impl From<UserActionServiceError> for CloseReadinessError {
    fn from(error: UserActionServiceError) -> Self {
        Self::UserAction(error)
    }
}

#[cfg(test)]
#[path = "tests/support.rs"]
mod test_support;

pub(crate) use blockers::{normalize_close_blockers, open_write_ticket_close_blocker};
pub(crate) use facts::{
    facts_from_projection, facts_with_pending_authorities,
    facts_with_projected_acceptance_criteria, facts_with_record_run_projection,
    facts_with_resolved_authorities, facts_with_resolved_unrecorded_changes, CloseReadinessFacts,
};
pub(crate) use recording::{
    build_record_run_close_basis, RecordRunCloseBasisContext, RecordRunCloseBasisError,
};
pub(crate) use service::{
    assess_close_readiness, plan_close_readiness, plan_projected_close_readiness,
    CloseReadinessRequest,
};
pub(crate) use summary::{CloseReadinessAssessment, CloseReadinessSummary};
