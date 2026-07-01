#![forbid(unsafe_code)]

//! Storage boundary for SQLite records, artifact plumbing, and migrations.
//!
//! This crate implements baseline SQLite schema creation and transaction
//! utilities only. Public Volicord method behavior remains outside this crate.

use volicord_types::TypeBoundary;

pub mod agent_connections;
pub mod artifacts;
pub mod bootstrap;
pub mod core_pipeline;
pub mod error;
pub mod guards;
pub mod inspection;
pub mod local_consent;
pub mod migrations;
pub mod runtime_home;
pub mod session_watch;
pub mod sqlite;

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
