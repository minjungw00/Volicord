use std::path::Path;

use volicord_platform_fs::PlatformDiagnosticClass;
use volicord_types::values::ErrorCode;

use crate::{
    method_execution::PlanError,
    method_rejection::{rejected_pipeline_response, validation_rejected},
    pipeline::{tool_error, CorePipelineError},
    product_path::{observe_product_paths, ProductPathValidationError},
};

pub(crate) fn observe_request_product_paths(
    repository_root: &Path,
    raw_paths: &[String],
    dry_run: volicord_types::schema::DryRunIntent,
    state_version: Option<u64>,
    field: &'static str,
    invalid_message: &'static str,
    containment_message: &'static str,
) -> Result<Vec<String>, PlanError> {
    match observe_product_paths(repository_root, raw_paths) {
        Ok(paths) => Ok(paths),
        Err(ProductPathValidationError::Lexical(_)) => {
            let response = validation_rejected(dry_run, state_version, field, invalid_message)
                .map_err(PlanError::Core)?;
            Err(PlanError::Response(Box::new(response)))
        }
        Err(error @ ProductPathValidationError::Platform(_))
            if error.platform_class() == Some(PlatformDiagnosticClass::Rejected) =>
        {
            let response = rejected_pipeline_response(
                dry_run,
                state_version,
                vec![tool_error(
                    ErrorCode::InvocationContextMismatch,
                    containment_message,
                    false,
                    None,
                )],
            )
            .map_err(PlanError::Core)?;
            Err(PlanError::Response(Box::new(response)))
        }
        Err(ProductPathValidationError::Platform(error)) => {
            Err(PlanError::Core(CorePipelineError::from(error)))
        }
    }
}
