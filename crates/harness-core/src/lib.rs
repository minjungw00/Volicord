#![forbid(unsafe_code)]

//! Core-facing services for owner-defined Harness behavior.
//!
//! Core owns public method behavior and coordinates storage-facing work.
//! Adapters may depend on this crate; this crate does not depend on adapter
//! crates.

use harness_store::{artifacts::ArtifactStoreBoundary, sqlite::SqliteStoreBoundary};
use harness_types::TypeBoundary;

mod methods;
pub mod pipeline;
mod policy;

pub use pipeline::{
    dry_run_response, method_result_base, method_result_value, rejected_response, tool_error,
    CorePipelineError, CoreResult, CoreService, InvocationContext, PipelineResponse,
    VerifiedSurfaceContext,
};

/// Minimal Core service marker for validating crate boundaries.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CoreBoundary {
    store: SqliteStoreBoundary,
    artifacts: ArtifactStoreBoundary,
}

impl CoreBoundary {
    /// Creates a Core boundary marker.
    pub const fn new() -> Self {
        Self {
            store: SqliteStoreBoundary,
            artifacts: ArtifactStoreBoundary,
        }
    }

    /// Identifies the shared type boundary used by Core-facing APIs.
    pub const fn api_type_boundary(self) -> TypeBoundary {
        let _ = self.store;
        let _ = self.artifacts;
        TypeBoundary::Api
    }
}

#[cfg(test)]
mod tests {
    use super::CoreBoundary;
    use harness_types::TypeBoundary;

    #[test]
    fn core_boundary_points_to_api_types() {
        assert_eq!(CoreBoundary::new().api_type_boundary(), TypeBoundary::Api);
    }
}
