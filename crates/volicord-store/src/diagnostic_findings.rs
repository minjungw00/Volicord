//! Structured diagnostic findings persisted in the Runtime Home Registry.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use serde_json::{json, Value};
use volicord_types::{
    DiagnosticFinding, DiagnosticFindingId, DiagnosticSeverity, IntegrationRevision,
    MAX_DIAGNOSTIC_FINDINGS,
};

use crate::{
    agent_connections::raw_agent_connection_record_from_conn,
    bootstrap::raw_project_record_from_conn,
    operational_sessions::{connection_integration_revision, runtime_session_from_conn},
    sqlite::{
        begin_immediate_transaction, open_registry_database, open_registry_database_read_only,
        registry_db_path,
    },
    StoreError, StoreResult,
};

/// Maximum caller-selected cause depth accepted by Store traversal.
pub const MAX_DIAGNOSTIC_CAUSE_CHAIN_DEPTH: usize = 32;
/// Maximum distinct findings returned by one cause-chain traversal.
pub const MAX_DIAGNOSTIC_CAUSE_CHAIN_FINDINGS: usize = MAX_DIAGNOSTIC_FINDINGS;

const MAX_SUBJECT_JSON_BYTES: usize = 4_096;
const MAX_FACTS_JSON_BYTES: usize = 16 * 1_024;
const MAX_ACTIONS_JSON_BYTES: usize = 64 * 1_024;

/// One finding reached at its minimum depth from the requested finding.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticCauseChainEntry {
    pub depth: usize,
    pub finding: DiagnosticFinding,
}

/// Deterministic bounded traversal result.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticCauseChain {
    pub entries: Vec<DiagnosticCauseChainEntry>,
    pub depth_limit_reached: bool,
}

struct PreparedFinding<'a> {
    finding: &'a DiagnosticFinding,
    subject_json: String,
    facts_json: String,
    actions_json: String,
}

/// Inserts one validated finding and all of its cause edges atomically.
pub fn insert_diagnostic_finding(
    runtime_home: impl AsRef<Path>,
    finding: &DiagnosticFinding,
) -> StoreResult<DiagnosticFinding> {
    let mut inserted =
        insert_diagnostic_finding_graph(runtime_home, std::slice::from_ref(finding))?;
    Ok(inserted.remove(0))
}

/// Inserts a complete validated finding graph in one Registry transaction.
pub fn insert_diagnostic_finding_graph(
    runtime_home: impl AsRef<Path>,
    findings: &[DiagnosticFinding],
) -> StoreResult<Vec<DiagnosticFinding>> {
    let prepared = prepare_graph(findings)?;
    if prepared.is_empty() {
        return Ok(Vec::new());
    }

    let path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    validate_graph_references(&tx, &prepared)?;
    insert_prepared_graph(&tx, &prepared)?;
    tx.commit()?;

    let mut inserted = findings.to_vec();
    inserted.sort_by(|left, right| left.id().cmp(right.id()));
    Ok(inserted)
}

/// Links one runtime session to an already-persisted terminal error finding.
pub fn link_mcp_runtime_session_terminal_finding(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
    finding_id: &DiagnosticFindingId,
) -> StoreResult<()> {
    let path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    link_terminal_finding_in_tx(&tx, runtime_session_id, finding_id)?;
    tx.commit()?;
    Ok(())
}

/// Inserts one terminal finding and links its runtime session in one transaction.
pub fn insert_and_link_mcp_runtime_session_terminal_finding(
    runtime_home: impl AsRef<Path>,
    finding: &DiagnosticFinding,
) -> StoreResult<DiagnosticFinding> {
    let runtime_session_id = finding
        .runtime_session_id()
        .map(|value| value.as_str())
        .ok_or_else(|| StoreError::InvalidInput {
            detail: "terminal diagnostic finding requires runtime_session_id".to_owned(),
        })?;
    let prepared = prepare_graph(std::slice::from_ref(finding))?;
    let path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    validate_graph_references(&tx, &prepared)?;
    insert_prepared_graph(&tx, &prepared)?;
    link_terminal_finding_in_tx(&tx, runtime_session_id, finding.id())?;
    tx.commit()?;
    Ok(finding.clone())
}

/// Reads one finding by its stable ID.
pub fn diagnostic_finding(
    runtime_home: impl AsRef<Path>,
    finding_id: &DiagnosticFindingId,
) -> StoreResult<Option<DiagnosticFinding>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    diagnostic_finding_from_conn(&conn, finding_id.as_str())
}

/// Reads findings for one runtime session in observation/ID order.
pub fn diagnostic_findings_for_runtime_session(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
) -> StoreResult<Vec<DiagnosticFinding>> {
    validate_lookup_id("runtime_session_id", runtime_session_id)?;
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database_read_only(path)?;
    finding_query(
        &conn,
        "WHERE runtime_session_id = ?1 ORDER BY observed_at, finding_id",
        [runtime_session_id],
    )
}

/// Reads findings for the Connection's exact current integration revision.
pub fn current_diagnostic_findings_for_connection(
    runtime_home: impl AsRef<Path>,
    connection_internal_id: &str,
) -> StoreResult<Vec<DiagnosticFinding>> {
    validate_lookup_id("connection_internal_id", connection_internal_id)?;
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database_read_only(path)?;
    let connection = raw_agent_connection_record_from_conn(&conn, connection_internal_id)?
        .ok_or_else(|| StoreError::NotFound {
            entity: "agent_connection",
            id: connection_internal_id.to_owned(),
        })?;
    let revision = connection_integration_revision(&connection)?;
    finding_query(
        &conn,
        "WHERE connection_internal_id = ?1 AND integration_revision = ?2
         ORDER BY observed_at, finding_id",
        [connection_internal_id, revision.as_str()],
    )
}

/// Traverses cause edges deterministically up to the caller-selected depth.
pub fn diagnostic_cause_chain(
    runtime_home: impl AsRef<Path>,
    finding_id: &DiagnosticFindingId,
    max_depth: usize,
) -> StoreResult<DiagnosticCauseChain> {
    if max_depth > MAX_DIAGNOSTIC_CAUSE_CHAIN_DEPTH {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "diagnostic cause depth must not exceed {MAX_DIAGNOSTIC_CAUSE_CHAIN_DEPTH}"
            ),
        });
    }
    let path = registry_db_path(runtime_home);
    let conn = open_registry_database_read_only(path)?;
    if diagnostic_finding_from_conn(&conn, finding_id.as_str())?.is_none() {
        return Err(StoreError::NotFound {
            entity: "diagnostic_finding",
            id: finding_id.to_string(),
        });
    }

    let mut depths = BTreeMap::new();
    let mut explored_remaining = BTreeMap::new();
    let mut path_ids = BTreeSet::new();
    let mut depth_limit_reached = false;
    traverse_causes(
        &conn,
        finding_id.as_str(),
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

    let mut ordered = depths
        .into_iter()
        .map(|(id, depth)| (depth, id))
        .collect::<Vec<_>>();
    ordered.sort();
    let mut entries = Vec::with_capacity(ordered.len());
    for (depth, id) in ordered {
        let finding = diagnostic_finding_from_conn(&conn, &id)?
            .ok_or_else(|| corrupt_value(&id, "cause_finding_id"))?;
        entries.push(DiagnosticCauseChainEntry { depth, finding });
    }
    Ok(DiagnosticCauseChain {
        entries,
        depth_limit_reached,
    })
}

fn prepare_graph(findings: &[DiagnosticFinding]) -> StoreResult<Vec<PreparedFinding<'_>>> {
    if findings.len() > MAX_DIAGNOSTIC_FINDINGS {
        return Err(StoreError::InvalidInput {
            detail: format!("diagnostic graph exceeds {MAX_DIAGNOSTIC_FINDINGS} findings"),
        });
    }
    let mut ids = BTreeSet::new();
    let mut prepared = Vec::with_capacity(findings.len());
    for finding in findings {
        if !ids.insert(finding.id().as_str()) {
            return Err(StoreError::InvalidInput {
                detail: format!("duplicate diagnostic finding id {}", finding.id()),
            });
        }
        let subject_json = bounded_json(
            "diagnostic subject",
            finding.subject(),
            MAX_SUBJECT_JSON_BYTES,
        )?;
        let facts_json = bounded_json("diagnostic facts", finding.facts(), MAX_FACTS_JSON_BYTES)?;
        let actions_json = bounded_json(
            "diagnostic actions",
            finding.actions(),
            MAX_ACTIONS_JSON_BYTES,
        )?;
        prepared.push(PreparedFinding {
            finding,
            subject_json,
            facts_json,
            actions_json,
        });
    }
    validate_new_graph_cycles(&prepared)?;
    Ok(prepared)
}

fn bounded_json<T: serde::Serialize + ?Sized>(
    label: &str,
    value: &T,
    max_bytes: usize,
) -> StoreResult<String> {
    let json = serde_json::to_string(value).map_err(|_| StoreError::InvalidInput {
        detail: format!("{label} could not be serialized"),
    })?;
    if json.len() > max_bytes {
        return Err(StoreError::InvalidInput {
            detail: format!("{label} exceeds {max_bytes} serialized bytes"),
        });
    }
    Ok(json)
}

fn validate_new_graph_cycles(prepared: &[PreparedFinding<'_>]) -> StoreResult<()> {
    let adjacency = prepared
        .iter()
        .map(|item| {
            (
                item.finding.id().as_str(),
                item.finding
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

fn validate_graph_references(
    tx: &Transaction<'_>,
    prepared: &[PreparedFinding<'_>],
) -> StoreResult<()> {
    let new_ids = prepared
        .iter()
        .map(|item| item.finding.id().as_str())
        .collect::<BTreeSet<_>>();
    for id in &new_ids {
        let exists = tx
            .query_row(
                "SELECT 1 FROM diagnostic_findings WHERE finding_id = ?1",
                [id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if exists {
            return Err(StoreError::Conflict {
                entity: "diagnostic_finding",
                id: (*id).to_owned(),
                detail: "finding id is already persisted".to_owned(),
            });
        }
    }
    for item in prepared {
        let finding = item.finding;
        if let Some(connection_id) = finding.connection_id() {
            if raw_agent_connection_record_from_conn(tx, connection_id.as_str())?.is_none() {
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
            if raw_project_record_from_conn(tx, project_id.as_str())?.is_none() {
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
                runtime_session_from_conn(tx, runtime_session_id.as_str())?.ok_or_else(|| {
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
        for cause in item.finding.causes() {
            let cause_id = cause.finding_id().as_str();
            if !new_ids.contains(cause_id)
                && tx
                    .query_row(
                        "SELECT 1 FROM diagnostic_findings WHERE finding_id = ?1",
                        [cause_id],
                        |_| Ok(()),
                    )
                    .optional()?
                    .is_none()
            {
                return Err(StoreError::InvalidInput {
                    detail: format!(
                        "diagnostic finding {} references missing cause {cause_id}",
                        item.finding.id()
                    ),
                });
            }
        }
    }
    Ok(())
}

fn insert_prepared_graph(
    tx: &Transaction<'_>,
    prepared: &[PreparedFinding<'_>],
) -> StoreResult<()> {
    for item in prepared {
        let finding = item.finding;
        tx.execute(
            "INSERT INTO diagnostic_findings (
                finding_id, code, domain, stage, severity, source,
                subject_json, facts_json, actions_json, correlation_id,
                connection_internal_id, project_internal_id, runtime_session_id,
                integration_revision, observed_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15
             )",
            params![
                finding.id().as_str(),
                finding.code().as_str(),
                finding.domain().as_str(),
                finding.stage().as_str(),
                severity_str(finding.severity()),
                finding.source().as_str(),
                item.subject_json,
                item.facts_json,
                item.actions_json,
                finding.correlation_id(),
                finding.connection_id().map(|value| value.as_str()),
                finding.project_id().map(|value| value.as_str()),
                finding.runtime_session_id().map(|value| value.as_str()),
                finding
                    .integration_revision()
                    .map(IntegrationRevision::as_str),
                finding.observed_at().to_canonical_string(),
            ],
        )?;
    }
    for item in prepared {
        for cause in item.finding.causes() {
            tx.execute(
                "INSERT INTO diagnostic_cause_edges (finding_id, cause_finding_id)
                 VALUES (?1, ?2)",
                params![item.finding.id().as_str(), cause.finding_id().as_str()],
            )?;
        }
    }
    Ok(())
}

fn link_terminal_finding_in_tx(
    tx: &Transaction<'_>,
    runtime_session_id: &str,
    finding_id: &DiagnosticFindingId,
) -> StoreResult<()> {
    validate_lookup_id("runtime_session_id", runtime_session_id)?;
    let runtime =
        runtime_session_from_conn(tx, runtime_session_id)?.ok_or_else(|| StoreError::NotFound {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
        })?;
    let finding = diagnostic_finding_from_conn(tx, finding_id.as_str())?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "diagnostic_finding",
            id: finding_id.to_string(),
        }
    })?;
    if finding.severity() != DiagnosticSeverity::Error
        || finding.runtime_session_id().map(|value| value.as_str()) != Some(runtime_session_id)
        || finding.connection_id().map(|value| value.as_str())
            != Some(runtime.connection_internal_id.as_str())
        || finding
            .integration_revision()
            .map(IntegrationRevision::as_str)
            != Some(runtime.connection_integration_revision.as_str())
    {
        return Err(StoreError::Conflict {
            entity: "diagnostic_finding",
            id: finding_id.to_string(),
            detail: "terminal finding does not match the runtime session coordinates".to_owned(),
        });
    }
    if runtime.graceful_close_at.is_some() {
        return Err(StoreError::Conflict {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
            detail: "terminal finding cannot follow graceful close".to_owned(),
        });
    }
    if let Some(existing) = runtime.terminal_finding_id.as_deref() {
        if existing == finding_id.as_str() {
            return Ok(());
        }
        return Err(StoreError::Conflict {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
            detail: "runtime session already has another terminal finding".to_owned(),
        });
    }
    let observed_at = finding.observed_at().to_canonical_string();
    if observed_at < runtime.last_observed_at {
        return Err(StoreError::Conflict {
            entity: "mcp_runtime_session",
            id: runtime_session_id.to_owned(),
            detail: "terminal finding predates the last runtime observation".to_owned(),
        });
    }
    tx.execute(
        "UPDATE mcp_runtime_sessions
            SET terminal_finding_id = ?2, last_observed_at = ?3
          WHERE runtime_session_id = ?1",
        params![runtime_session_id, finding_id.as_str(), observed_at],
    )?;
    Ok(())
}

const FINDING_SELECT: &str = "SELECT
    finding_id, code, domain, stage, severity, source,
    subject_json, facts_json, actions_json, correlation_id,
    connection_internal_id, project_internal_id, runtime_session_id,
    integration_revision, observed_at
  FROM diagnostic_findings";

fn finding_query<const N: usize>(
    conn: &Connection,
    suffix: &str,
    values: [&str; N],
) -> StoreResult<Vec<DiagnosticFinding>> {
    let mut stmt = conn.prepare(&format!("{FINDING_SELECT} {suffix}"))?;
    let rows = stmt.query_map(rusqlite::params_from_iter(values), |row| {
        stored_finding_from_row(conn, row)
    })?;
    rows.collect::<Result<Vec<_>, _>>()?.map_err_store_json()
}

fn diagnostic_finding_from_conn(
    conn: &Connection,
    finding_id: &str,
) -> StoreResult<Option<DiagnosticFinding>> {
    let stored = conn
        .query_row(
            &format!("{FINDING_SELECT} WHERE finding_id = ?1"),
            [finding_id],
            |row| stored_finding_from_row(conn, row),
        )
        .optional()?;
    stored
        .map(|result| result.map_err(|_| corrupt_json(finding_id, "finding")))
        .transpose()
}

fn stored_finding_from_row(
    conn: &Connection,
    row: &Row<'_>,
) -> rusqlite::Result<Result<DiagnosticFinding, serde_json::Error>> {
    let finding_id = row.get::<_, String>(0)?;
    let mut cause_stmt = conn.prepare(
        "SELECT cause_finding_id FROM diagnostic_cause_edges
          WHERE finding_id = ?1 ORDER BY cause_finding_id",
    )?;
    let causes = cause_stmt
        .query_map([&finding_id], |cause_row| cause_row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let severity = row.get::<_, String>(4)?;
    let subject_json = row.get::<_, String>(6)?;
    let facts_json = row.get::<_, String>(7)?;
    let actions_json = row.get::<_, String>(8)?;
    let code = row.get::<_, String>(1)?;
    let domain = row.get::<_, String>(2)?;
    let stage = row.get::<_, String>(3)?;
    let source = row.get::<_, String>(5)?;
    let observed_at = row.get::<_, String>(14)?;
    let correlation_id = row.get::<_, Option<String>>(9)?;
    let connection_id = row.get::<_, Option<String>>(10)?;
    let project_id = row.get::<_, Option<String>>(11)?;
    let runtime_session_id = row.get::<_, Option<String>>(12)?;
    let integration_revision = row.get::<_, Option<String>>(13)?;
    Ok((|| {
        let subject = serde_json::from_str::<Value>(&subject_json)?;
        let facts = serde_json::from_str::<Value>(&facts_json)?;
        let actions = serde_json::from_str::<Value>(&actions_json)?;
        serde_json::from_value(json!({
            "id": finding_id,
            "code": code,
            "domain": domain,
            "stage": stage,
            "severity": severity,
            "source": source,
            "subject": subject,
            "facts": facts,
            "causes": causes.into_iter().map(|id| json!({"finding_id": id})).collect::<Vec<_>>(),
            "actions": actions,
            "observed_at": observed_at,
            "correlation_id": correlation_id,
            "connection_id": connection_id,
            "project_id": project_id,
            "runtime_session_id": runtime_session_id,
            "integration_revision": integration_revision,
        }))
    })())
}

trait StoredFindingResults {
    fn map_err_store_json(self) -> StoreResult<Vec<DiagnosticFinding>>;
}

impl StoredFindingResults for Vec<Result<DiagnosticFinding, serde_json::Error>> {
    fn map_err_store_json(self) -> StoreResult<Vec<DiagnosticFinding>> {
        self.into_iter()
            .map(|result| result.map_err(|_| corrupt_json("query", "finding")))
            .collect()
    }
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

const fn severity_str(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Info => "info",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Error => "error",
    }
}

fn validate_lookup_id(field: &str, value: &str) -> StoreResult<()> {
    if value.is_empty() || value.len() > 192 || value.chars().any(char::is_control) {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must be 1 through 192 non-control UTF-8 bytes"),
        });
    }
    Ok(())
}

fn corrupt_json(record_ref: &str, logical_column: &'static str) -> StoreError {
    StoreError::CorruptOwnerStateJson {
        database_kind: "registry",
        table: "diagnostic_findings",
        record_ref: record_ref.to_owned(),
        logical_column,
    }
}

fn corrupt_value(record_ref: &str, logical_column: &'static str) -> StoreError {
    StoreError::CorruptOwnerStateValue {
        database_kind: "registry",
        table: "diagnostic_findings",
        record_ref: record_ref.to_owned(),
        logical_column,
    }
}
