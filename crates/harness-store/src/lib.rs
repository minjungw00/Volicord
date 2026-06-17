#![forbid(unsafe_code)]

//! Storage boundary for SQLite records, artifact plumbing, and migrations.
//!
//! This crate implements baseline SQLite schema creation and transaction
//! utilities only. Public Harness method behavior remains outside this crate.

use harness_types::TypeBoundary;

pub mod artifacts;
pub mod error;
pub mod migrations;
pub mod sqlite;

pub use error::{StoreError, StoreResult};

/// Identifies the shared type boundary this crate depends on.
pub const fn shared_type_boundary() -> TypeBoundary {
    TypeBoundary::Domain
}

#[cfg(test)]
mod tests {
    use super::shared_type_boundary;
    use harness_types::TypeBoundary;

    #[test]
    fn store_depends_on_domain_types_boundary() {
        assert_eq!(shared_type_boundary(), TypeBoundary::Domain);
    }
}
