#![forbid(unsafe_code)]

//! Storage boundary for SQLite records, artifact plumbing, and schema initialization.
//!
//! This crate implements baseline SQLite schema creation and transaction
//! utilities only. Public Volicord method behavior remains outside this crate.

pub use volicord_platform_fs::CanonicalRuntimeHomePath;

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
pub mod integration_verification;
pub mod managed_launch_leases;
pub mod mutation;
pub mod operational_diagnostics;
pub mod operational_sessions;
pub mod runtime_home;
pub mod schema;
pub mod setup_transaction;
pub mod sqlite;
pub mod workflow_records;

pub use error::{StoreError, StoreFailureRoute, StoreResult};
pub use mutation::{
    RuntimeHomeMutationContext, RuntimeHomeMutationSetupInProgress,
    RUNTIME_HOME_MUTATION_SETUP_IN_PROGRESS,
};
