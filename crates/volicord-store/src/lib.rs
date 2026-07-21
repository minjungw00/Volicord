#![forbid(unsafe_code)]

//! Storage boundary for SQLite records, artifact plumbing, and schema initialization.
//!
//! This crate implements baseline SQLite schema creation and transaction
//! utilities only. Public Volicord method behavior remains outside this crate.

use volicord_types::TypeBoundary;

pub mod agent_connections;
pub mod artifacts;
pub mod bootstrap;
pub mod core_pipeline;
pub mod diagnostic_findings;
pub mod diagnostics;
pub mod error;
pub mod evidence_capture;
pub mod export;
pub mod guards;
pub mod inspection;
pub mod operational_sessions;
pub mod runtime_home;
pub mod schema;
pub mod sqlite;
pub mod workflow_records;

pub use error::{StoreError, StoreFailureRoute, StoreResult};

/// Identifies the shared type boundary this crate depends on.
pub const fn shared_type_boundary() -> TypeBoundary {
    TypeBoundary::Domain
}

#[cfg(test)]
mod tests {
    use super::shared_type_boundary;
    use volicord_types::TypeBoundary;

    #[test]
    fn store_depends_on_domain_types_boundary() {
        assert_eq!(shared_type_boundary(), TypeBoundary::Domain);
    }
}
