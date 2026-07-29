use volicord_store::core_pipeline::ProjectStateHeader;
use volicord_types::schema::ToolEnvelope;

use crate::{
    close_readiness::CloseReadinessError,
    error_boundary::{store::store_error_plan, user_action::user_action_service_plan_error},
    method_execution::PlanError,
    method_rejection::no_active_task_response,
    pipeline::CorePipelineError,
};

pub(crate) fn close_readiness_plan_error(
    envelope: &ToolEnvelope,
    project_state: &ProjectStateHeader,
    error: CloseReadinessError,
) -> PlanError {
    match error {
        CloseReadinessError::Core(CorePipelineError::Store(error)) => {
            store_error_plan(envelope, project_state, error)
        }
        CloseReadinessError::Core(error) => PlanError::Core(error),
        CloseReadinessError::UserAction(error) => {
            user_action_service_plan_error(envelope, project_state, error)
        }
        CloseReadinessError::NoActiveTask => {
            PlanError::Response(Box::new(no_active_task_response(envelope, project_state)))
        }
    }
}
