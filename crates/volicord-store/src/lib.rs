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

pub use error::{
    StoreAggregate, StoreAggregateInvariant, StoreCorruptionLocation, StoreError,
    StoreFailureRoute, StoreResult, WriteTicketInvariant,
};
pub use mutation::{
    RuntimeHomeMutationContext, RuntimeHomeMutationSetupInProgress,
    RUNTIME_HOME_MUTATION_SETUP_IN_PROGRESS,
};

#[cfg(test)]
#[test]
fn project_schema_declares_stable_ddl_vector() -> Result<(), Box<dyn std::error::Error>> {
    use volicord_types::storage_contract::{GeneratedRelationKind, StorageDatabaseKind};

    let metadata = schema::generated_schema_metadata()?;
    assert_eq!(metadata.tables.len(), 84);
    assert_eq!(metadata.columns.len(), 669);
    assert_eq!(metadata.indexes.len(), 78);
    assert_eq!(metadata.constraints.len(), 52);

    for table in [
        "shaping_checkpoints",
        "shaping_checkpoint_gaps",
        "shaping_checkpoint_user_actions",
        "shaping_decision_applications",
        "shaping_checkpoint_applications",
        "shaping_authority_reauthorizations",
    ] {
        assert!(metadata.tables.iter().any(|relation| {
            relation.database == StorageDatabaseKind::ProjectState
                && relation.name == table
                && relation.relation_kind == GeneratedRelationKind::Table
        }));
    }
    assert!(metadata.indexes.iter().any(|index| {
        index.database == StorageDatabaseKind::ProjectState
            && index.table == "shaping_checkpoints"
            && index.name == "idx_shaping_checkpoints_one_current"
            && index.unique
            && index.partial
    }));
    for trigger in [
        "trg_shaping_checkpoint_predecessor_immutable",
        "trg_shaping_checkpoint_successor_requires_exact_predecessor",
        "trg_shaping_checkpoint_live_user_action_not_detached",
        "trg_shaping_gap_not_added_to_ready_checkpoint",
        "trg_shaping_gap_insert_is_current",
        "trg_shaping_gap_disposition_transition",
        "trg_shaping_checkpoint_ready_has_no_current_gap",
        "trg_shaping_gap_disposition_requires_matching_user_resolution",
        "trg_shaping_gap_application_requires_accepted_gap",
        "trg_applied_shaping_gap_is_terminal",
        "trg_shaping_decision_application_requires_accepted_resolution",
        "trg_shaping_checkpoint_application_lineage",
    ] {
        assert!(metadata.tables.iter().any(|relation| {
            relation.database == StorageDatabaseKind::ProjectState
                && relation.name == trigger
                && relation.relation_kind == GeneratedRelationKind::Trigger
        }));
    }
    Ok(())
}
