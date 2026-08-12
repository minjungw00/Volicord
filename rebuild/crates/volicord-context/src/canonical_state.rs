//! Canonical Context full-state invariant boundary.
//!
//! Transition preconditions remain with the command that interprets an intent.
//! This module owns validation of the complete resulting Project state. Portable
//! callers submit the complete portable Project payload; direct commands submit
//! the transaction's complete Project view before commit. Local operation rows
//! and clone bindings remain outside portable Canonical Context.

use crate::portable::{
    export_table, validate_portable_canonical_invariants, Lineage, Payload, TABLES,
};
use crate::{Error, ProjectId};
use rusqlite::Connection;

/// Validate one complete portable Project state through the canonical boundary.
pub(crate) fn validate_payload(payload: &Payload, project_id: ProjectId) -> Result<(), Error> {
    validate_portable_canonical_invariants(payload, project_id)
}

/// Validate the complete canonical view produced by a direct transaction.
///
/// `Connection` is also the deref target of `rusqlite::Transaction`, so callers
/// can run this after all canonical rows are written and before commit.
pub(crate) fn validate_project_state(
    connection: &Connection,
    project_id: ProjectId,
) -> Result<(), Error> {
    let mut tables = Vec::with_capacity(TABLES.len());
    for spec in TABLES {
        tables.push(export_table(connection, spec, project_id)?);
    }
    validate_payload(
        &Payload {
            project_id: project_id.to_string(),
            lineage: Lineage {
                common_base_basis: String::new(),
                history_basis: String::new(),
            },
            tables,
        },
        project_id,
    )
}
