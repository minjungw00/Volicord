use volicord_store::{core_pipeline::ProjectStateHeader, StoreError};
use volicord_types::schema::ToolEnvelope;

use crate::{
    method_execution::PlanError,
    method_rejection::rejected_pipeline_response,
    pipeline::{store_failure_error, CorePipelineError, CoreResult, PipelineResponse},
};

pub(crate) fn store_error_plan(
    envelope: &ToolEnvelope,
    project_state: &ProjectStateHeader,
    error: StoreError,
) -> PlanError {
    match core_error_response(
        envelope,
        Some(project_state.state_version),
        CorePipelineError::from(error),
    ) {
        Ok(response) => PlanError::Response(Box::new(response)),
        Err(error) => PlanError::Core(error),
    }
}

pub(crate) fn core_error_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    error: CorePipelineError,
) -> CoreResult<PipelineResponse> {
    match error {
        CorePipelineError::Store(error) => match CorePipelineError::from(error) {
            CorePipelineError::Store(error) => rejected_pipeline_response(
                envelope.dry_run,
                state_version,
                vec![store_failure_error(error)],
            ),
            error => Err(error),
        },
        error => Err(error),
    }
}

pub(crate) fn plan_error_response(
    envelope: &ToolEnvelope,
    project_state: &ProjectStateHeader,
    error: PlanError,
) -> CoreResult<PipelineResponse> {
    match error {
        PlanError::Response(response) => Ok(*response),
        PlanError::Core(error) => {
            core_error_response(envelope, Some(project_state.state_version), error)
        }
    }
}
