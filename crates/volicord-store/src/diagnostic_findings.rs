//! Lifecycle-specific structured diagnostic persistence in the Runtime Home Registry.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use volicord_types::{
    AgentConnectionId, AgentRuntimeSessionId, CurrentDiagnosticFinding, CurrentDiagnosticKey,
    CurrentDiagnosticSnapshot, CurrentDiagnosticStatus, DiagnosticAction, DiagnosticCause,
    DiagnosticCode, DiagnosticDomain, DiagnosticFacts, DiagnosticFinding, DiagnosticFindingData,
    DiagnosticFindingId, DiagnosticFindingLifecycle, DiagnosticOccurrenceId, DiagnosticScope,
    DiagnosticScopeKind, DiagnosticSeverity, DiagnosticSource, DiagnosticStage, DiagnosticSubject,
    IntegrationRevision, OccurrenceDiagnosticFinding, ProjectId, UtcTimestamp,
    MAX_DIAGNOSTIC_FINDINGS, MAX_DIAGNOSTIC_ROOT_CAUSES,
};

use crate::{
    agent_connections::raw_agent_connection_record_from_conn,
    bootstrap::raw_project_record_from_conn,
    operational_sessions::runtime_session_from_conn,
    sqlite::{
        begin_immediate_transaction, open_registry_database, open_registry_database_read_only,
        registry_db_path,
    },
    StoreError, StoreResult,
};

/// Maximum caller-selected cause depth accepted by Store traversal.
pub const MAX_DIAGNOSTIC_CAUSE_CHAIN_DEPTH: usize = 32;
/// Maximum distinct findings returned by one cause-graph traversal.
pub const MAX_DIAGNOSTIC_CAUSE_CHAIN_FINDINGS: usize = MAX_DIAGNOSTIC_FINDINGS;

const MAX_SUBJECT_JSON_BYTES: usize = 4_096;
const MAX_FACTS_JSON_BYTES: usize = 16 * 1_024;
const MAX_ACTIONS_JSON_BYTES: usize = 64 * 1_024;

/// One finding reached at its minimum depth from the requested seed set.
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

struct PreparedFinding {
    projection: DiagnosticFinding,
    lifecycle: DiagnosticFindingLifecycle,
    current_identity_digest: Option<String>,
    scope_kind: Option<DiagnosticScopeKind>,
    scope_identity: Option<String>,
    current_status: Option<CurrentDiagnosticStatus>,
    resolved_at: Option<String>,
    subject_json: String,
    facts_json: String,
    actions_json: String,
}

impl PreparedFinding {
    fn occurrence(finding: &OccurrenceDiagnosticFinding) -> StoreResult<Self> {
        Self::from_projection(
            finding.to_diagnostic_finding(),
            DiagnosticFindingLifecycle::Occurrence,
            None,
            None,
            None,
            None,
            None,
        )
    }

    fn current(finding: &CurrentDiagnosticFinding) -> StoreResult<Self> {
        Self::from_projection(
            finding.to_diagnostic_finding(),
            DiagnosticFindingLifecycle::CurrentState,
            Some(finding.identity_digest().to_owned()),
            Some(finding.key().scope().kind()),
            Some(finding.key().scope().identity().to_owned()),
            Some(finding.snapshot().status()),
            finding
                .snapshot()
                .resolved_at()
                .map(UtcTimestamp::to_canonical_string),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_projection(
        projection: DiagnosticFinding,
        lifecycle: DiagnosticFindingLifecycle,
        current_identity_digest: Option<String>,
        scope_kind: Option<DiagnosticScopeKind>,
        scope_identity: Option<String>,
        current_status: Option<CurrentDiagnosticStatus>,
        resolved_at: Option<String>,
    ) -> StoreResult<Self> {
        let subject_json = bounded_json(
            "diagnostic subject",
            projection.subject(),
            MAX_SUBJECT_JSON_BYTES,
        )?;
        let facts_json =
            bounded_json("diagnostic facts", projection.facts(), MAX_FACTS_JSON_BYTES)?;
        let actions_json = bounded_json(
            "diagnostic actions",
            projection.actions(),
            MAX_ACTIONS_JSON_BYTES,
        )?;
        Ok(Self {
            projection,
            lifecycle,
            current_identity_digest,
            scope_kind,
            scope_identity,
            current_status,
            resolved_at,
            subject_json,
            facts_json,
            actions_json,
        })
    }
}

enum StoredFinding {
    Occurrence(OccurrenceDiagnosticFinding),
    Current(CurrentDiagnosticFinding),
}

impl StoredFinding {
    fn projection(&self) -> DiagnosticFinding {
        match self {
            Self::Occurrence(finding) => finding.to_diagnostic_finding(),
            Self::Current(finding) => finding.to_diagnostic_finding(),
        }
    }
}

/// Inserts one immutable occurrence and all of its cause edges atomically.
pub fn insert_occurrence_finding(
    runtime_home: impl AsRef<Path>,
    finding: &OccurrenceDiagnosticFinding,
) -> StoreResult<OccurrenceDiagnosticFinding> {
    let mut inserted =
        insert_occurrence_finding_graph(runtime_home, std::slice::from_ref(finding))?;
    Ok(inserted.remove(0))
}

/// Inserts a complete immutable occurrence graph in one Registry transaction.
pub fn insert_occurrence_finding_graph(
    runtime_home: impl AsRef<Path>,
    findings: &[OccurrenceDiagnosticFinding],
) -> StoreResult<Vec<OccurrenceDiagnosticFinding>> {
    let prepared = prepare_occurrence_graph(findings)?;
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
    inserted.sort_by_key(OccurrenceDiagnosticFinding::id);
    Ok(inserted)
}

/// Inserts or refreshes only the replaceable snapshot for one current key.
///
/// The key derives the finding ID. Existing identity fields are compared and
/// never updated. A successful refresh always leaves the condition active and
/// atomically replaces its outgoing causes.
pub fn upsert_current_snapshot(
    runtime_home: impl AsRef<Path>,
    finding: &CurrentDiagnosticFinding,
) -> StoreResult<CurrentDiagnosticFinding> {
    if finding.snapshot().status() != CurrentDiagnosticStatus::Active
        || finding.snapshot().resolved_at().is_some()
    {
        return Err(StoreError::InvalidInput {
            detail: "current diagnostic upsert requires an active snapshot".to_owned(),
        });
    }
    let prepared = PreparedFinding::current(finding)?;
    let path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    validate_current_references(&tx, finding, &prepared)?;
    replace_current_snapshot(&tx, &prepared)?;
    tx.commit()?;
    Ok(finding.clone())
}

/// Marks one current condition resolved and removes current actions and causes.
pub fn resolve_current_finding(
    runtime_home: impl AsRef<Path>,
    key: &CurrentDiagnosticKey,
    resolved_at: UtcTimestamp,
) -> StoreResult<CurrentDiagnosticFinding> {
    let path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    let finding_id = key.finding_id();
    let stored = stored_finding_from_conn(&tx, finding_id.as_str())?.ok_or_else(|| {
        StoreError::NotFound {
            entity: "diagnostic_finding",
            id: finding_id.to_string(),
        }
    })?;
    let StoredFinding::Current(existing) = stored else {
        return Err(StoreError::Conflict {
            entity: "diagnostic_finding",
            id: finding_id.to_string(),
            detail: "diagnostic finding is not a current-state record".to_owned(),
        });
    };
    if existing.key() != key {
        return Err(StoreError::Conflict {
            entity: "diagnostic_finding",
            id: finding_id.to_string(),
            detail: "current diagnostic identity fields do not match the persisted key".to_owned(),
        });
    }
    if existing.snapshot().status() == CurrentDiagnosticStatus::Resolved {
        tx.commit()?;
        return Ok(existing);
    }

    tx.execute(
        "DELETE FROM diagnostic_cause_edges WHERE finding_id = ?1",
        [finding_id.as_str()],
    )?;
    tx.execute(
        "UPDATE diagnostic_findings
            SET actions_json = '[]',
                current_state_status = 'resolved',
                resolved_at = ?2
          WHERE finding_id = ?1 AND lifecycle = 'current_state'",
        params![finding_id.as_str(), resolved_at.to_canonical_string()],
    )?;
    let resolved = stored_finding_from_conn(&tx, finding_id.as_str())?
        .and_then(|finding| match finding {
            StoredFinding::Current(current) => Some(current),
            StoredFinding::Occurrence(_) => None,
        })
        .ok_or_else(|| corrupt_value(finding_id.as_str(), "lifecycle"))?;
    tx.commit()?;
    Ok(resolved)
}

/// Inserts one terminal occurrence and links its runtime session in one transaction.
pub fn insert_and_link_runtime_terminal_occurrence(
    runtime_home: impl AsRef<Path>,
    finding: &OccurrenceDiagnosticFinding,
) -> StoreResult<OccurrenceDiagnosticFinding> {
    let runtime_session_id = finding
        .runtime_session_id()
        .map(|value| value.as_str())
        .ok_or_else(|| StoreError::InvalidInput {
            detail: "terminal diagnostic occurrence requires runtime_session_id".to_owned(),
        })?;
    let prepared = prepare_occurrence_graph(std::slice::from_ref(finding))?;
    let path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    validate_graph_references(&tx, &prepared)?;
    insert_prepared_graph(&tx, &prepared)?;
    link_terminal_occurrence_in_tx(&tx, runtime_session_id, finding.id().as_str())?;
    tx.commit()?;
    Ok(finding.clone())
}

/// Reads existing findings by ID after lifecycle-specific persisted validation.
pub fn diagnostic_findings_by_ids(
    runtime_home: impl AsRef<Path>,
    finding_ids: &[DiagnosticFindingId],
) -> StoreResult<Vec<DiagnosticFinding>> {
    if finding_ids.len() > MAX_DIAGNOSTIC_FINDINGS {
        return Err(StoreError::InvalidInput {
            detail: format!("diagnostic lookup exceeds {MAX_DIAGNOSTIC_FINDINGS} finding IDs"),
        });
    }
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database_read_only(path)?;
    let mut ids = finding_ids.to_vec();
    ids.sort();
    ids.dedup();
    let mut findings = Vec::new();
    for finding_id in ids {
        if let Some(stored) = stored_finding_from_conn(&conn, finding_id.as_str())? {
            findings.push(stored.projection());
        }
    }
    Ok(findings)
}

/// Reads findings eligible for a current report by explicit ID.
///
/// Immutable occurrences and active current-state findings are eligible.
/// Resolved current-state findings remain available through exact historical
/// lookup but are deliberately absent from this current-report selection.
pub fn reportable_diagnostic_findings_by_ids(
    runtime_home: impl AsRef<Path>,
    finding_ids: &[DiagnosticFindingId],
) -> StoreResult<Vec<DiagnosticFinding>> {
    if finding_ids.len() > MAX_DIAGNOSTIC_FINDINGS {
        return Err(StoreError::InvalidInput {
            detail: format!("diagnostic lookup exceeds {MAX_DIAGNOSTIC_FINDINGS} finding IDs"),
        });
    }
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database_read_only(path)?;
    let mut ids = finding_ids.to_vec();
    ids.sort();
    ids.dedup();
    let mut findings = Vec::new();
    for finding_id in ids {
        let Some(stored) = stored_finding_from_conn(&conn, finding_id.as_str())? else {
            continue;
        };
        match stored {
            StoredFinding::Occurrence(occurrence) => {
                findings.push(occurrence.to_diagnostic_finding());
            }
            StoredFinding::Current(current)
                if current.snapshot().status() == CurrentDiagnosticStatus::Active =>
            {
                findings.push(current.to_diagnostic_finding());
            }
            StoredFinding::Current(_) => {}
        }
    }
    Ok(findings)
}

/// Reads immutable occurrences for one runtime session in observation/ID order.
pub fn diagnostic_occurrences_for_runtime_session(
    runtime_home: impl AsRef<Path>,
    runtime_session_id: &str,
) -> StoreResult<Vec<OccurrenceDiagnosticFinding>> {
    validate_lookup_id("runtime_session_id", runtime_session_id)?;
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database_read_only(path)?;
    stored_finding_query(
        &conn,
        "WHERE lifecycle = 'occurrence' AND runtime_session_id = ?1
         ORDER BY observed_at, finding_id",
        [runtime_session_id],
    )?
    .into_iter()
    .map(|finding| match finding {
        StoredFinding::Occurrence(occurrence) => Ok(occurrence),
        StoredFinding::Current(current) => Err(corrupt_value(current.id().as_str(), "lifecycle")),
    })
    .collect()
}

/// Reads active current findings for one exact diagnostic scope.
pub fn active_current_findings_for_scope(
    runtime_home: impl AsRef<Path>,
    scope: &DiagnosticScope,
) -> StoreResult<Vec<CurrentDiagnosticFinding>> {
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_registry_database_read_only(path)?;
    stored_finding_query(
        &conn,
        "WHERE lifecycle = 'current_state'
           AND current_state_status = 'active'
           AND diagnostic_scope_kind = ?1
           AND diagnostic_scope_identity = ?2
         ORDER BY observed_at, finding_id",
        [scope.kind().as_str(), scope.identity()],
    )?
    .into_iter()
    .map(|finding| match finding {
        StoredFinding::Current(current) => Ok(current),
        StoredFinding::Occurrence(occurrence) => Err(corrupt_value(
            occurrence.occurrence_id().as_str(),
            "lifecycle",
        )),
    })
    .collect()
}

/// Traverses a bounded diagnostic graph from one or more seed IDs.
pub fn bounded_diagnostic_graph_from_seeds(
    runtime_home: impl AsRef<Path>,
    seed_ids: &[DiagnosticFindingId],
    max_depth: usize,
) -> StoreResult<DiagnosticCauseChain> {
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
            .ok_or_else(|| corrupt_value(&id, "cause_finding_id"))?
            .projection();
        entries.push(DiagnosticCauseChainEntry { depth, finding });
    }
    Ok(DiagnosticCauseChain {
        entries,
        depth_limit_reached,
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
    let graph = bounded_diagnostic_graph_from_seeds(runtime_home, finding_ids, max_depth)?;
    if graph.depth_limit_reached {
        return Err(StoreError::InvalidInput {
            detail: format!("diagnostic root-cause traversal exceeded depth {max_depth}"),
        });
    }
    let roots = graph
        .entries
        .into_iter()
        .filter(|entry| entry.finding.causes().is_empty())
        .map(|entry| entry.finding.id().clone())
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

fn prepare_occurrence_graph(
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

fn validate_graph_references(
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

fn validate_current_references(
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

fn persisted_finding_exists(conn: &Connection, finding_id: &str) -> StoreResult<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM diagnostic_findings WHERE finding_id = ?1",
            [finding_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn insert_prepared_graph(tx: &Transaction<'_>, prepared: &[PreparedFinding]) -> StoreResult<()> {
    for item in prepared {
        insert_prepared_finding(tx, item)?;
    }
    for item in prepared {
        insert_outgoing_causes(tx, &item.projection)?;
    }
    Ok(())
}

fn insert_prepared_finding(tx: &Transaction<'_>, item: &PreparedFinding) -> StoreResult<()> {
    let finding = &item.projection;
    tx.execute(
        "INSERT INTO diagnostic_findings (
            finding_id, lifecycle, current_identity_digest,
            diagnostic_scope_kind, diagnostic_scope_identity,
            current_state_status, resolved_at,
            code, domain, stage, severity, source,
            subject_json, facts_json, actions_json, correlation_id,
            connection_internal_id, project_internal_id, runtime_session_id,
            integration_revision, observed_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
            ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21
         )",
        params![
            finding.id().as_str(),
            item.lifecycle.as_str(),
            item.current_identity_digest.as_deref(),
            item.scope_kind.map(DiagnosticScopeKind::as_str),
            item.scope_identity.as_deref(),
            item.current_status.map(CurrentDiagnosticStatus::as_str),
            item.resolved_at.as_deref(),
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
    Ok(())
}

fn replace_current_snapshot(tx: &Transaction<'_>, prepared: &PreparedFinding) -> StoreResult<()> {
    let finding = &prepared.projection;
    if !persisted_finding_exists(tx, finding.id().as_str())? {
        insert_prepared_finding(tx, prepared)?;
        insert_outgoing_causes(tx, finding)?;
        return Ok(());
    }

    tx.execute(
        "DELETE FROM diagnostic_cause_edges WHERE finding_id = ?1",
        [finding.id().as_str()],
    )?;
    tx.execute(
        "UPDATE diagnostic_findings
            SET severity = ?2,
                facts_json = ?3,
                actions_json = ?4,
                correlation_id = ?5,
                connection_internal_id = ?6,
                project_internal_id = ?7,
                integration_revision = ?8,
                observed_at = ?9,
                current_state_status = 'active',
                resolved_at = NULL
          WHERE finding_id = ?1 AND lifecycle = 'current_state'",
        params![
            finding.id().as_str(),
            severity_str(finding.severity()),
            prepared.facts_json,
            prepared.actions_json,
            finding.correlation_id(),
            finding.connection_id().map(|value| value.as_str()),
            finding.project_id().map(|value| value.as_str()),
            finding
                .integration_revision()
                .map(IntegrationRevision::as_str),
            finding.observed_at().to_canonical_string(),
        ],
    )?;
    insert_outgoing_causes(tx, finding)
}

fn insert_outgoing_causes(tx: &Transaction<'_>, finding: &DiagnosticFinding) -> StoreResult<()> {
    for cause in finding.causes() {
        tx.execute(
            "INSERT INTO diagnostic_cause_edges (finding_id, cause_finding_id)
             VALUES (?1, ?2)",
            params![finding.id().as_str(), cause.finding_id().as_str()],
        )?;
    }
    Ok(())
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

const FINDING_SELECT: &str = "SELECT
    finding_id, lifecycle, current_identity_digest,
    diagnostic_scope_kind, diagnostic_scope_identity,
    current_state_status, resolved_at,
    code, domain, stage, severity, source,
    subject_json, facts_json, actions_json, correlation_id,
    connection_internal_id, project_internal_id, runtime_session_id,
    integration_revision, observed_at
  FROM diagnostic_findings";

struct StoredFindingRaw {
    finding_id: String,
    lifecycle: String,
    current_identity_digest: Option<String>,
    scope_kind: Option<String>,
    scope_identity: Option<String>,
    current_status: Option<String>,
    resolved_at: Option<String>,
    code: String,
    domain: String,
    stage: String,
    severity: String,
    source: String,
    subject_json: String,
    facts_json: String,
    actions_json: String,
    correlation_id: Option<String>,
    connection_id: Option<String>,
    project_id: Option<String>,
    runtime_session_id: Option<String>,
    integration_revision: Option<String>,
    observed_at: String,
    causes: Vec<String>,
}

fn stored_finding_query<const N: usize>(
    conn: &Connection,
    suffix: &str,
    values: [&str; N],
) -> StoreResult<Vec<StoredFinding>> {
    let mut stmt = conn.prepare(&format!("{FINDING_SELECT} {suffix}"))?;
    let raw = stmt
        .query_map(rusqlite::params_from_iter(values), |row| {
            stored_finding_raw_from_row(conn, row)
        })?
        .collect::<Result<Vec<_>, _>>()?;
    raw.into_iter().map(decode_stored_finding).collect()
}

fn stored_finding_from_conn(
    conn: &Connection,
    finding_id: &str,
) -> StoreResult<Option<StoredFinding>> {
    let raw = conn
        .query_row(
            &format!("{FINDING_SELECT} WHERE finding_id = ?1"),
            [finding_id],
            |row| stored_finding_raw_from_row(conn, row),
        )
        .optional()?;
    raw.map(decode_stored_finding).transpose()
}

fn stored_finding_raw_from_row(
    conn: &Connection,
    row: &Row<'_>,
) -> rusqlite::Result<StoredFindingRaw> {
    let finding_id = row.get::<_, String>(0)?;
    let mut cause_stmt = conn.prepare(
        "SELECT cause_finding_id FROM diagnostic_cause_edges
          WHERE finding_id = ?1 ORDER BY cause_finding_id",
    )?;
    let causes = cause_stmt
        .query_map([&finding_id], |cause_row| cause_row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(StoredFindingRaw {
        finding_id,
        lifecycle: row.get(1)?,
        current_identity_digest: row.get(2)?,
        scope_kind: row.get(3)?,
        scope_identity: row.get(4)?,
        current_status: row.get(5)?,
        resolved_at: row.get(6)?,
        code: row.get(7)?,
        domain: row.get(8)?,
        stage: row.get(9)?,
        severity: row.get(10)?,
        source: row.get(11)?,
        subject_json: row.get(12)?,
        facts_json: row.get(13)?,
        actions_json: row.get(14)?,
        correlation_id: row.get(15)?,
        connection_id: row.get(16)?,
        project_id: row.get(17)?,
        runtime_session_id: row.get(18)?,
        integration_revision: row.get(19)?,
        observed_at: row.get(20)?,
        causes,
    })
}

fn decode_stored_finding(raw: StoredFindingRaw) -> StoreResult<StoredFinding> {
    let data = decode_finding_data(&raw)?;
    match raw.lifecycle.as_str() {
        "occurrence" => {
            if raw.current_identity_digest.is_some()
                || raw.scope_kind.is_some()
                || raw.scope_identity.is_some()
                || raw.current_status.is_some()
                || raw.resolved_at.is_some()
            {
                return Err(corrupt_value(&raw.finding_id, "lifecycle"));
            }
            let id = DiagnosticOccurrenceId::parse(raw.finding_id.clone())
                .map_err(|_| corrupt_value(&raw.finding_id, "finding_id"))?;
            let runtime_session_id = raw.runtime_session_id.map(AgentRuntimeSessionId::new);
            OccurrenceDiagnosticFinding::from_persisted_parts(id, data, runtime_session_id)
                .map(StoredFinding::Occurrence)
                .map_err(|_| corrupt_value(&raw.finding_id, "finding"))
        }
        "current_state" => {
            let (Some(digest), Some(scope_kind), Some(scope_identity), Some(status)) = (
                raw.current_identity_digest.as_deref(),
                raw.scope_kind.as_deref(),
                raw.scope_identity.as_deref(),
                raw.current_status.as_deref(),
            ) else {
                return Err(corrupt_value(&raw.finding_id, "lifecycle"));
            };
            if raw.runtime_session_id.is_some() {
                return Err(corrupt_value(&raw.finding_id, "runtime_session_id"));
            }
            let scope_kind = parse_scope_kind(scope_kind)
                .ok_or_else(|| corrupt_value(&raw.finding_id, "diagnostic_scope_kind"))?;
            let scope = DiagnosticScope::try_new(scope_kind, scope_identity.to_owned())
                .map_err(|_| corrupt_value(&raw.finding_id, "diagnostic_scope_identity"))?;
            let key = CurrentDiagnosticKey::new(
                scope,
                data.code().clone(),
                data.domain().clone(),
                data.stage().clone(),
                data.source().clone(),
                data.subject().clone(),
            );
            if digest != key.identity_digest() || raw.finding_id != key.finding_id().as_str() {
                return Err(corrupt_value(&raw.finding_id, "current_identity_digest"));
            }
            let status = match status {
                "active" => CurrentDiagnosticStatus::Active,
                "resolved" => CurrentDiagnosticStatus::Resolved,
                _ => return Err(corrupt_value(&raw.finding_id, "current_state_status")),
            };
            let resolved_at = raw
                .resolved_at
                .as_deref()
                .map(UtcTimestamp::parse)
                .transpose()
                .map_err(|_| corrupt_value(&raw.finding_id, "resolved_at"))?;
            let mut snapshot = CurrentDiagnosticSnapshot::try_new(
                data.severity(),
                data.facts().clone(),
                data.observed_at().clone(),
            )
            .and_then(|snapshot| snapshot.with_causes(data.causes().to_vec()))
            .and_then(|snapshot| snapshot.with_actions(data.actions().to_vec()))
            .map_err(|_| corrupt_value(&raw.finding_id, "snapshot"))?;
            if let Some(correlation_id) = data.correlation_id() {
                snapshot = snapshot
                    .with_correlation_id(correlation_id.to_owned())
                    .map_err(|_| corrupt_value(&raw.finding_id, "correlation_id"))?;
            }
            if let Some(connection_id) = data.connection_id() {
                snapshot = snapshot
                    .with_connection_id(connection_id.clone())
                    .map_err(|_| corrupt_value(&raw.finding_id, "connection_internal_id"))?;
            }
            if let Some(project_id) = data.project_id() {
                snapshot = snapshot
                    .with_project_id(project_id.clone())
                    .map_err(|_| corrupt_value(&raw.finding_id, "project_internal_id"))?;
            }
            if let Some(revision) = data.integration_revision() {
                snapshot = snapshot.with_integration_revision(revision.clone());
            }
            snapshot = snapshot
                .with_persisted_state(status, resolved_at)
                .map_err(|_| corrupt_value(&raw.finding_id, "current_state_status"))?;
            CurrentDiagnosticFinding::try_new(key, snapshot)
                .map(StoredFinding::Current)
                .map_err(|_| corrupt_value(&raw.finding_id, "finding"))
        }
        _ => Err(corrupt_value(&raw.finding_id, "lifecycle")),
    }
}

fn decode_finding_data(raw: &StoredFindingRaw) -> StoreResult<DiagnosticFindingData> {
    let code = DiagnosticCode::parse(raw.code.clone())
        .map_err(|_| corrupt_value(&raw.finding_id, "code"))?;
    let domain = DiagnosticDomain::parse(raw.domain.clone())
        .map_err(|_| corrupt_value(&raw.finding_id, "domain"))?;
    let stage = DiagnosticStage::parse(raw.stage.clone())
        .map_err(|_| corrupt_value(&raw.finding_id, "stage"))?;
    let severity =
        parse_severity(&raw.severity).ok_or_else(|| corrupt_value(&raw.finding_id, "severity"))?;
    let source = DiagnosticSource::parse(raw.source.clone())
        .map_err(|_| corrupt_value(&raw.finding_id, "source"))?;
    let subject = serde_json::from_str::<DiagnosticSubject>(&raw.subject_json)
        .map_err(|_| corrupt_json(&raw.finding_id, "subject_json"))?;
    let facts = serde_json::from_str::<DiagnosticFacts>(&raw.facts_json)
        .map_err(|_| corrupt_json(&raw.finding_id, "facts_json"))?;
    let actions = serde_json::from_str::<Vec<DiagnosticAction>>(&raw.actions_json)
        .map_err(|_| corrupt_json(&raw.finding_id, "actions_json"))?;
    let observed_at = UtcTimestamp::parse(&raw.observed_at)
        .map_err(|_| corrupt_value(&raw.finding_id, "observed_at"))?;
    let causes = raw
        .causes
        .iter()
        .map(|id| {
            DiagnosticFindingId::parse(id.clone())
                .map(DiagnosticCause::new)
                .map_err(|_| corrupt_value(&raw.finding_id, "cause_finding_id"))
        })
        .collect::<StoreResult<Vec<_>>>()?;
    let mut data = DiagnosticFindingData::try_new(
        code,
        domain,
        stage,
        severity,
        source,
        subject,
        facts,
        observed_at,
    )
    .and_then(|data| data.with_causes(causes))
    .and_then(|data| data.with_actions(actions))
    .map_err(|_| corrupt_value(&raw.finding_id, "finding"))?;
    if let Some(correlation_id) = raw.correlation_id.as_deref() {
        data = data
            .with_correlation_id(correlation_id.to_owned())
            .map_err(|_| corrupt_value(&raw.finding_id, "correlation_id"))?;
    }
    if let Some(connection_id) = raw.connection_id.as_deref() {
        data = data
            .with_connection_id(AgentConnectionId::new(connection_id))
            .map_err(|_| corrupt_value(&raw.finding_id, "connection_internal_id"))?;
    }
    if let Some(project_id) = raw.project_id.as_deref() {
        data = data
            .with_project_id(ProjectId::new(project_id))
            .map_err(|_| corrupt_value(&raw.finding_id, "project_internal_id"))?;
    }
    if let Some(revision) = raw.integration_revision.as_deref() {
        data = data.with_integration_revision(
            IntegrationRevision::parse(revision)
                .map_err(|_| corrupt_value(&raw.finding_id, "integration_revision"))?,
        );
    }
    Ok(data)
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

const fn parse_severity(value: &str) -> Option<DiagnosticSeverity> {
    match value.as_bytes() {
        b"info" => Some(DiagnosticSeverity::Info),
        b"warning" => Some(DiagnosticSeverity::Warning),
        b"error" => Some(DiagnosticSeverity::Error),
        _ => None,
    }
}

const fn parse_scope_kind(value: &str) -> Option<DiagnosticScopeKind> {
    match value.as_bytes() {
        b"connection" => Some(DiagnosticScopeKind::Connection),
        b"project" => Some(DiagnosticScopeKind::Project),
        b"runtime_home" => Some(DiagnosticScopeKind::RuntimeHome),
        b"installation" => Some(DiagnosticScopeKind::Installation),
        b"process" => Some(DiagnosticScopeKind::Process),
        _ => None,
    }
}

fn validate_lookup_id(field: &str, value: &str) -> StoreResult<()> {
    if value.is_empty() || value.len() > 1_024 || value.chars().any(char::is_control) {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must be 1 through 1024 non-control UTF-8 bytes"),
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
