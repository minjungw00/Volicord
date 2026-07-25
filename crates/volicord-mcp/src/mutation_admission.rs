use crate::errors::McpAdapterError;
use std::path::Path;
use volicord_platform_fs::{
    RuntimeHomeMutationLease, RuntimeHomeMutationLeaseMode, RuntimeHomeMutationLeaseOutcome,
    RuntimeHomeMutationWaitPolicy,
};
use volicord_store::{RuntimeHomeMutationContext, RuntimeHomeMutationSetupInProgress};

pub(crate) fn with_mcp_runtime_home_mutation<T>(
    runtime_home: &Path,
    mutation_domain: &'static str,
    operation: impl FnOnce(&RuntimeHomeMutationContext<'_>) -> Result<T, McpAdapterError>,
) -> Result<T, McpAdapterError> {
    let outcome = RuntimeHomeMutationLease::acquire(
        runtime_home,
        RuntimeHomeMutationLeaseMode::SharedWriter,
        RuntimeHomeMutationWaitPolicy::Immediate,
    )
    .map_err(|source| McpAdapterError::MutationAdmissionAcquisition {
        mutation_domain,
        source,
    })?;
    let lease = match outcome {
        RuntimeHomeMutationLeaseOutcome::Acquired(lease) => lease,
        RuntimeHomeMutationLeaseOutcome::Busy(busy) => {
            return Err(McpAdapterError::MutationAdmission(
                RuntimeHomeMutationSetupInProgress::from_busy(mutation_domain, busy),
            ));
        }
    };
    let context = RuntimeHomeMutationContext::new(lease.permit(), runtime_home)
        .map_err(McpAdapterError::Store)?;
    operation(&context)
}
