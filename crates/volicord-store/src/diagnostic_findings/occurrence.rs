//! Insert-only diagnostic occurrence persistence.

use rusqlite::{params, Transaction};
use volicord_types::{DiagnosticSeverity, IntegrationRevision, OccurrenceDiagnosticFinding};

use crate::{
    operational_sessions::runtime_session_from_conn,
    sqlite::{begin_immediate_transaction, open_registry_database_for_mutation},
    RuntimeHomeMutationContext, StoreError, StoreResult,
};

use super::{
    graph::{insert_prepared_graph, prepare_occurrence_graph, validate_graph_references},
    row::{stored_finding_from_conn, validate_lookup_id, StoredFinding},
};

/// Inserts one immutable occurrence and all of its cause edges atomically.
pub fn insert_occurrence_finding(
    context: &RuntimeHomeMutationContext<'_>,
    finding: &OccurrenceDiagnosticFinding,
) -> StoreResult<OccurrenceDiagnosticFinding> {
    let mut inserted = insert_occurrence_finding_graph(context, std::slice::from_ref(finding))?;
    Ok(inserted.remove(0))
}

/// Inserts a complete immutable occurrence graph in one Registry transaction.
pub fn insert_occurrence_finding_graph(
    context: &RuntimeHomeMutationContext<'_>,
    findings: &[OccurrenceDiagnosticFinding],
) -> StoreResult<Vec<OccurrenceDiagnosticFinding>> {
    let prepared = prepare_occurrence_graph(findings)?;
    if prepared.is_empty() {
        return Ok(Vec::new());
    }

    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    validate_graph_references(&tx, &prepared)?;
    insert_prepared_graph(&tx, &prepared)?;
    tx.commit()?;

    let mut inserted = findings.to_vec();
    inserted.sort_by_key(OccurrenceDiagnosticFinding::id);
    Ok(inserted)
}

/// Inserts one terminal occurrence and links its runtime session in one transaction.
pub fn insert_and_link_runtime_terminal_occurrence(
    context: &RuntimeHomeMutationContext<'_>,
    finding: &OccurrenceDiagnosticFinding,
) -> StoreResult<OccurrenceDiagnosticFinding> {
    let runtime_session_id = finding
        .runtime_session_id()
        .map(|value| value.as_str())
        .ok_or_else(|| StoreError::InvalidInput {
            detail: "terminal diagnostic occurrence requires runtime_session_id".to_owned(),
        })?;
    let prepared = prepare_occurrence_graph(std::slice::from_ref(finding))?;
    let mut conn = open_registry_database_for_mutation(context)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    validate_graph_references(&tx, &prepared)?;
    insert_prepared_graph(&tx, &prepared)?;
    link_terminal_occurrence_in_tx(&tx, runtime_session_id, finding.id().as_str())?;
    tx.commit()?;
    Ok(finding.clone())
}

fn link_terminal_occurrence_in_tx(
    tx: &Transaction<'_>,
    runtime_session_id: &str,
    finding_id: &str,
) -> StoreResult<()> {
    validate_lookup_id("runtime_session_id", runtime_session_id)?;
    let runtime =
        runtime_session_from_conn(tx, runtime_session_id)?.ok_or_else(|| StoreError::NotFound {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
        })?;
    let stored = stored_finding_from_conn(tx, finding_id)?.ok_or_else(|| StoreError::NotFound {
        entity: "diagnostic_finding",
        id: finding_id.to_owned(),
    })?;
    let StoredFinding::Occurrence(finding) = stored else {
        return Err(StoreError::Conflict {
            entity: "diagnostic_finding",
            id: finding_id.to_owned(),
            detail: "terminal finding must be an occurrence".to_owned(),
        });
    };
    let projection = finding.to_diagnostic_finding();
    if projection.severity() != DiagnosticSeverity::Error
        || projection.runtime_session_id().map(|value| value.as_str()) != Some(runtime_session_id)
        || projection.connection_id().map(|value| value.as_str())
            != Some(runtime.connection_internal_id.as_str())
        || projection
            .integration_revision()
            .map(IntegrationRevision::as_str)
            != Some(runtime.connection_integration_revision.as_str())
    {
        return Err(StoreError::Conflict {
            entity: "diagnostic_finding",
            id: finding_id.to_owned(),
            detail: "terminal occurrence does not match the runtime session coordinates".to_owned(),
        });
    }
    if runtime.graceful_close_at.is_some() {
        return Err(StoreError::Conflict {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
            detail: "terminal occurrence cannot follow graceful close".to_owned(),
        });
    }
    if let Some(existing) = runtime.terminal_finding_id.as_deref() {
        if existing == finding_id {
            return Ok(());
        }
        return Err(StoreError::Conflict {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
            detail: "runtime session already has another terminal occurrence".to_owned(),
        });
    }
    let observed_at = projection.observed_at().to_canonical_string();
    if observed_at < runtime.last_observed_at {
        return Err(StoreError::Conflict {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
            detail: "terminal occurrence predates the last runtime observation".to_owned(),
        });
    }
    tx.execute(
        "UPDATE mcp_runtime_sessions
            SET terminal_finding_id = ?2, last_observed_at = ?3
          WHERE runtime_session_id = ?1",
        params![runtime_session_id, finding_id, observed_at],
    )?;
    Ok(())
}
