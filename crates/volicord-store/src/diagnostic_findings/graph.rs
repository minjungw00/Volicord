//! Diagnostic cause validation and bounded deterministic traversal.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use rusqlite::{Connection, Transaction};
use volicord_types::diagnostics::{
    CurrentDiagnosticFinding, DiagnosticFinding, DiagnosticFindingId, OccurrenceDiagnosticFinding,
    StoredDiagnosticGraph, StoredDiagnosticGraphEntry, MAX_DIAGNOSTIC_FINDINGS,
    MAX_DIAGNOSTIC_ROOT_CAUSES,
};
use volicord_types::integration_revision::IntegrationRevision;

use crate::{
    agent_connections::raw_agent_connection_record_from_conn,
    bootstrap::raw_project_record_from_conn,
    operational_sessions::runtime_session_from_conn,
    sqlite::{open_registry_database_read_only, registry_db_path},
    StoreError, StoreResult,
};

use super::row::{
    corrupt_value, insert_outgoing_causes, insert_prepared_finding, persisted_finding_exists,
    stored_finding_from_conn, PreparedFinding, StoredFinding,
};

/// Maximum caller-selected cause depth accepted by Store traversal.
pub const MAX_DIAGNOSTIC_CAUSE_CHAIN_DEPTH: usize = 32;
/// Maximum distinct findings returned by one cause-graph traversal.
pub const MAX_DIAGNOSTIC_CAUSE_CHAIN_FINDINGS: usize = MAX_DIAGNOSTIC_FINDINGS;

/// Traverses a bounded lifecycle-aware diagnostic graph from one or more seed IDs.
pub fn bounded_stored_diagnostic_graph_from_seeds(
    runtime_home: impl AsRef<Path>,
    seed_ids: &[DiagnosticFindingId],
    max_depth: usize,
) -> StoreResult<StoredDiagnosticGraph> {
    if max_depth > MAX_DIAGNOSTIC_CAUSE_CHAIN_DEPTH {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "diagnostic cause depth must not exceed {MAX_DIAGNOSTIC_CAUSE_CHAIN_DEPTH}"
            ),
        });
    }
    if seed_ids.len() > MAX_DIAGNOSTIC_FINDINGS {
        return Err(StoreError::InvalidInput {
            detail: format!("diagnostic seed set exceeds {MAX_DIAGNOSTIC_FINDINGS} findings"),
        });
    }
    let path = registry_db_path(runtime_home);
    let conn = open_registry_database_read_only(path)?;
    let mut seeds = seed_ids.to_vec();
    seeds.sort();
    seeds.dedup();
    let mut depths = BTreeMap::new();
    let mut explored_remaining = BTreeMap::new();
    let mut depth_limit_reached = false;
    for seed_id in seeds {
        if stored_finding_from_conn(&conn, seed_id.as_str())?.is_none() {
            return Err(StoreError::NotFound {
                entity: "diagnostic_finding",
                id: seed_id.to_string(),
            });
        }
        let mut path_ids = BTreeSet::new();
        traverse_causes(
            &conn,
            seed_id.as_str(),
            0,
            max_depth,
            &mut path_ids,
            &mut depths,
            &mut explored_remaining,
            &mut depth_limit_reached,
        )?;
        if depths.len() > MAX_DIAGNOSTIC_CAUSE_CHAIN_FINDINGS {
            return Err(StoreError::InvalidInput {
                detail: format!(
                    "diagnostic cause traversal exceeds {MAX_DIAGNOSTIC_CAUSE_CHAIN_FINDINGS} findings"
                ),
            });
        }
    }

    let mut ordered = depths
        .into_iter()
        .map(|(id, depth)| (depth, id))
        .collect::<Vec<_>>();
    ordered.sort();
    let mut entries = Vec::with_capacity(ordered.len());
    for (depth, id) in ordered {
        let finding = stored_finding_from_conn(&conn, &id)?
            .ok_or_else(|| corrupt_value(&id, "cause_finding_id"))?;
        entries.push(
            StoredDiagnosticGraphEntry::try_new(depth, finding).map_err(|error| {
                StoreError::InvalidInput {
                    detail: error.to_string(),
                }
            })?,
        );
    }
    StoredDiagnosticGraph::try_new(entries, depth_limit_reached).map_err(|error| {
        StoreError::InvalidInput {
            detail: error.to_string(),
        }
    })
}

/// Computes independent root causes for selected persisted findings.
pub fn diagnostic_root_cause_ids(
    runtime_home: impl AsRef<Path>,
    finding_ids: &[DiagnosticFindingId],
    max_depth: usize,
) -> StoreResult<Vec<DiagnosticFindingId>> {
    if finding_ids.len() > MAX_DIAGNOSTIC_ROOT_CAUSES {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "diagnostic root-cause selection exceeds {MAX_DIAGNOSTIC_ROOT_CAUSES} findings"
            ),
        });
    }
    let graph = bounded_stored_diagnostic_graph_from_seeds(runtime_home, finding_ids, max_depth)?;
    if graph.depth_limit_reached() {
        return Err(StoreError::InvalidInput {
            detail: format!("diagnostic root-cause traversal exceeded depth {max_depth}"),
        });
    }
    let roots = graph
        .entries()
        .iter()
        .filter_map(|entry| {
            let finding = entry.finding().to_diagnostic_finding();
            finding.causes().is_empty().then(|| finding.id().clone())
        })
        .collect::<BTreeSet<_>>();
    if roots.len() > MAX_DIAGNOSTIC_ROOT_CAUSES {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "diagnostic root-cause selection exceeds {MAX_DIAGNOSTIC_ROOT_CAUSES} roots"
            ),
        });
    }
    Ok(roots.into_iter().collect())
}

pub(super) fn prepare_occurrence_graph(
    findings: &[OccurrenceDiagnosticFinding],
) -> StoreResult<Vec<PreparedFinding>> {
    if findings.len() > MAX_DIAGNOSTIC_FINDINGS {
        return Err(StoreError::InvalidInput {
            detail: format!("diagnostic graph exceeds {MAX_DIAGNOSTIC_FINDINGS} findings"),
        });
    }
    let mut ids = BTreeSet::new();
    let mut prepared = Vec::with_capacity(findings.len());
    for finding in findings {
        let projection = finding.to_diagnostic_finding();
        if !ids.insert(projection.id().clone()) {
            return Err(StoreError::InvalidInput {
                detail: format!("duplicate diagnostic finding id {}", projection.id()),
            });
        }
        prepared.push(PreparedFinding::occurrence(finding)?);
    }
    validate_new_graph_cycles(&prepared)?;
    Ok(prepared)
}

fn validate_new_graph_cycles(prepared: &[PreparedFinding]) -> StoreResult<()> {
    let adjacency = prepared
        .iter()
        .map(|item| {
            (
                item.projection.id().as_str(),
                item.projection
                    .causes()
                    .iter()
                    .map(|cause| cause.finding_id().as_str())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for id in adjacency.keys().copied() {
        visit_new_graph(id, &adjacency, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn visit_new_graph<'a>(
    id: &'a str,
    adjacency: &BTreeMap<&'a str, Vec<&'a str>>,
    visiting: &mut BTreeSet<&'a str>,
    visited: &mut BTreeSet<&'a str>,
) -> StoreResult<()> {
    if visited.contains(id) {
        return Ok(());
    }
    if !visiting.insert(id) {
        return Err(StoreError::InvalidInput {
            detail: format!("diagnostic cause graph contains a cycle at {id}"),
        });
    }
    if let Some(causes) = adjacency.get(id) {
        for cause in causes {
            if adjacency.contains_key(cause) {
                visit_new_graph(cause, adjacency, visiting, visited)?;
            }
        }
    }
    visiting.remove(id);
    visited.insert(id);
    Ok(())
}

pub(super) fn validate_graph_references(
    tx: &Transaction<'_>,
    prepared: &[PreparedFinding],
) -> StoreResult<()> {
    let new_ids = prepared
        .iter()
        .map(|item| item.projection.id().as_str())
        .collect::<BTreeSet<_>>();
    for id in &new_ids {
        if persisted_finding_exists(tx, id)? {
            return Err(StoreError::Conflict {
                entity: "diagnostic_finding",
                id: (*id).to_owned(),
                detail: "occurrence finding id is already persisted".to_owned(),
            });
        }
    }
    for item in prepared {
        validate_correlation_references(tx, &item.projection)?;
        for cause in item.projection.causes() {
            let cause_id = cause.finding_id().as_str();
            if !new_ids.contains(cause_id) && !persisted_finding_exists(tx, cause_id)? {
                return Err(StoreError::InvalidInput {
                    detail: format!(
                        "diagnostic finding {} references missing cause {cause_id}",
                        item.projection.id()
                    ),
                });
            }
        }
    }
    Ok(())
}

pub(super) fn validate_current_references(
    tx: &Transaction<'_>,
    finding: &CurrentDiagnosticFinding,
    prepared: &PreparedFinding,
) -> StoreResult<()> {
    if let Some(stored) = stored_finding_from_conn(tx, finding.id().as_str())? {
        match stored {
            StoredFinding::Occurrence(_) => {
                return Err(StoreError::Conflict {
                    entity: "diagnostic_finding",
                    id: finding.id().to_string(),
                    detail: "immutable occurrence cannot be replaced".to_owned(),
                })
            }
            StoredFinding::Current(existing) if existing.key() != finding.key() => {
                return Err(StoreError::Conflict {
                    entity: "diagnostic_finding",
                    id: finding.id().to_string(),
                    detail: "current diagnostic identity fields do not match".to_owned(),
                })
            }
            StoredFinding::Current(_) => {}
        }
    }
    validate_correlation_references(tx, &prepared.projection)?;
    for cause in prepared.projection.causes() {
        if !persisted_finding_exists(tx, cause.finding_id().as_str())? {
            return Err(StoreError::InvalidInput {
                detail: format!(
                    "diagnostic finding {} references missing cause {}",
                    finding.id(),
                    cause.finding_id()
                ),
            });
        }
    }
    Ok(())
}

fn validate_correlation_references(
    conn: &Connection,
    finding: &DiagnosticFinding,
) -> StoreResult<()> {
    if let Some(connection_id) = finding.connection_id() {
        if raw_agent_connection_record_from_conn(conn, connection_id.as_str())?.is_none() {
            return Err(StoreError::InvalidInput {
                detail: format!(
                    "diagnostic finding {} references missing Agent Connection {}",
                    finding.id(),
                    connection_id
                ),
            });
        }
    }
    if let Some(project_id) = finding.project_id() {
        if raw_project_record_from_conn(conn, project_id.as_str())?.is_none() {
            return Err(StoreError::InvalidInput {
                detail: format!(
                    "diagnostic finding {} references missing project {}",
                    finding.id(),
                    project_id
                ),
            });
        }
    }
    if let Some(runtime_session_id) = finding.runtime_session_id() {
        let runtime =
            runtime_session_from_conn(conn, runtime_session_id.as_str())?.ok_or_else(|| {
                StoreError::InvalidInput {
                    detail: format!(
                        "diagnostic finding {} references missing runtime session {}",
                        finding.id(),
                        runtime_session_id
                    ),
                }
            })?;
        if finding.connection_id().map(|value| value.as_str())
            != Some(runtime.connection_internal_id.as_str())
            || finding
                .integration_revision()
                .map(IntegrationRevision::as_str)
                != Some(runtime.connection_integration_revision.as_str())
        {
            return Err(StoreError::InvalidInput {
                detail: format!(
                    "diagnostic finding {} does not match its runtime session coordinates",
                    finding.id()
                ),
            });
        }
    }
    Ok(())
}

pub(super) fn insert_prepared_graph(
    tx: &Transaction<'_>,
    prepared: &[PreparedFinding],
) -> StoreResult<()> {
    for item in prepared {
        insert_prepared_finding(tx, item)?;
    }
    for item in prepared {
        insert_outgoing_causes(tx, &item.projection)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn traverse_causes(
    conn: &Connection,
    finding_id: &str,
    depth: usize,
    max_depth: usize,
    path_ids: &mut BTreeSet<String>,
    depths: &mut BTreeMap<String, usize>,
    explored_remaining: &mut BTreeMap<String, usize>,
    depth_limit_reached: &mut bool,
) -> StoreResult<()> {
    if !path_ids.insert(finding_id.to_owned()) {
        return Err(corrupt_value(finding_id, "cause_cycle"));
    }
    depths
        .entry(finding_id.to_owned())
        .and_modify(|prior| *prior = (*prior).min(depth))
        .or_insert(depth);
    let remaining = max_depth - depth;
    if explored_remaining
        .get(finding_id)
        .is_some_and(|prior| *prior >= remaining)
    {
        path_ids.remove(finding_id);
        return Ok(());
    }
    explored_remaining.insert(finding_id.to_owned(), remaining);
    let causes = cause_ids(conn, finding_id)?;
    if depth == max_depth {
        *depth_limit_reached |= !causes.is_empty();
    } else {
        for cause_id in causes {
            traverse_causes(
                conn,
                &cause_id,
                depth + 1,
                max_depth,
                path_ids,
                depths,
                explored_remaining,
                depth_limit_reached,
            )?;
        }
    }
    path_ids.remove(finding_id);
    Ok(())
}

fn cause_ids(conn: &Connection, finding_id: &str) -> StoreResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT cause_finding_id FROM diagnostic_cause_edges
          WHERE finding_id = ?1 ORDER BY cause_finding_id",
    )?;
    let causes = stmt
        .query_map([finding_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(causes)
}
