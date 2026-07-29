use volicord_store::core_pipeline::ProjectStateHeader;
use volicord_types::schema::ToolEnvelope;
use volicord_user_action_service::UserActionServiceError;

use crate::{
    error_boundary::store::store_error_plan,
    method_execution::PlanError,
    method_rejection::{decision_rejected_response, validation_rejected},
    pipeline::CorePipelineError,
};

pub(crate) fn user_action_service_plan_error(
    envelope: &ToolEnvelope,
    project_state: &ProjectStateHeader,
    error: UserActionServiceError,
) -> PlanError {
    match error {
        UserActionServiceError::Validation(error) => {
            match validation_rejected(
                envelope.dry_run,
                Some(project_state.state_version),
                error.field(),
                error.message(),
            ) {
                Ok(response) => PlanError::Response(Box::new(response)),
                Err(error) => PlanError::Core(error),
            }
        }
        UserActionServiceError::Unavailable(error) => {
            PlanError::Response(Box::new(decision_rejected_response(
                envelope,
                Some(project_state.state_version),
                error.message(),
            )))
        }
        UserActionServiceError::CorruptStoredState(error)
        | UserActionServiceError::Store(error) => store_error_plan(envelope, project_state, error),
        UserActionServiceError::Identity(error) => {
            PlanError::Core(CorePipelineError::InvalidDispatch {
                detail: format!("user-action identity invariant failed: {error:?}"),
            })
        }
        UserActionServiceError::Invariant(error) => {
            PlanError::Core(CorePipelineError::InvalidDispatch {
                detail: format!("user-action service invariant failed: {error:?}"),
            })
        }
    }
}
