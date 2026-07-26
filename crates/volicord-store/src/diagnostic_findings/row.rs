//! Diagnostic row encoding, decoding, and lifecycle identity validation.

use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use volicord_types::diagnostics::{
    CurrentDiagnosticFinding, CurrentDiagnosticKey, CurrentDiagnosticSnapshot,
    CurrentDiagnosticStatus, DiagnosticAction, DiagnosticCause, DiagnosticCode, DiagnosticDomain,
    DiagnosticFacts, DiagnosticFinding, DiagnosticFindingData, DiagnosticFindingId,
    DiagnosticFindingLifecycle, DiagnosticOccurrenceId, DiagnosticScope, DiagnosticScopeKind,
    DiagnosticSeverity, DiagnosticSource, DiagnosticStage, DiagnosticSubject,
    DiagnosticSubjectIdentity, OccurrenceDiagnosticFinding, StoredDiagnosticFinding,
};
use volicord_types::ids::{AgentConnectionId, AgentRuntimeSessionId, ProjectId};
use volicord_types::integration_revision::IntegrationRevision;
use volicord_types::values::UtcTimestamp;

use crate::{StoreError, StoreResult};

const MAX_SUBJECT_JSON_BYTES: usize = 4_096;
const MAX_FACTS_JSON_BYTES: usize = 16 * 1_024;
const MAX_ACTIONS_JSON_BYTES: usize = 64 * 1_024;

pub(super) struct PreparedFinding {
    pub(super) projection: DiagnosticFinding,
    lifecycle: DiagnosticFindingLifecycle,
    current_identity_digest: Option<String>,
    current_subject_identity: Option<String>,
    scope_kind: Option<DiagnosticScopeKind>,
    scope_identity: Option<String>,
    current_status: Option<CurrentDiagnosticStatus>,
    resolved_at: Option<String>,
    subject_json: String,
    facts_json: String,
    actions_json: String,
}

impl PreparedFinding {
    pub(super) fn occurrence(finding: &OccurrenceDiagnosticFinding) -> StoreResult<Self> {
        Self::from_projection(
            finding.to_diagnostic_finding(),
            DiagnosticFindingLifecycle::Occurrence,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    pub(super) fn current(finding: &CurrentDiagnosticFinding) -> StoreResult<Self> {
        Self::from_projection(
            finding.to_diagnostic_finding(),
            DiagnosticFindingLifecycle::CurrentState,
            Some(finding.identity_digest().to_owned()),
            Some(finding.key().subject_identity().as_str().to_owned()),
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
        current_subject_identity: Option<String>,
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
            current_subject_identity,
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

pub(super) type StoredFinding = StoredDiagnosticFinding;

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

pub(super) fn persisted_finding_exists(conn: &Connection, finding_id: &str) -> StoreResult<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM diagnostic_findings WHERE finding_id = ?1",
            [finding_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub(super) fn insert_prepared_finding(
    tx: &Transaction<'_>,
    item: &PreparedFinding,
) -> StoreResult<()> {
    let finding = &item.projection;
    tx.execute(
        "INSERT INTO diagnostic_findings (
            finding_id, lifecycle, current_identity_digest, current_subject_identity,
            diagnostic_scope_kind, diagnostic_scope_identity,
            current_state_status, resolved_at,
            code, domain, stage, severity, source,
            subject_json, facts_json, actions_json, correlation_id,
            connection_internal_id, project_internal_id, runtime_session_id,
            integration_revision, observed_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
            ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22
         )",
        params![
            finding.id().as_str(),
            item.lifecycle.as_str(),
            item.current_identity_digest.as_deref(),
            item.current_subject_identity.as_deref(),
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

pub(super) fn replace_current_snapshot(
    tx: &Transaction<'_>,
    prepared: &PreparedFinding,
) -> StoreResult<()> {
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
            SET subject_json = ?2,
                severity = ?3,
                facts_json = ?4,
                actions_json = ?5,
                correlation_id = ?6,
                connection_internal_id = ?7,
                project_internal_id = ?8,
                integration_revision = ?9,
                observed_at = ?10,
                current_state_status = 'active',
                resolved_at = NULL
          WHERE finding_id = ?1 AND lifecycle = 'current_state'",
        params![
            finding.id().as_str(),
            prepared.subject_json,
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

pub(super) fn insert_outgoing_causes(
    tx: &Transaction<'_>,
    finding: &DiagnosticFinding,
) -> StoreResult<()> {
    for cause in finding.causes() {
        tx.execute(
            "INSERT INTO diagnostic_cause_edges (finding_id, cause_finding_id)
             VALUES (?1, ?2)",
            params![finding.id().as_str(), cause.finding_id().as_str()],
        )?;
    }
    Ok(())
}

const FINDING_SELECT: &str = "SELECT
    finding_id, lifecycle, current_identity_digest, current_subject_identity,
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
    current_subject_identity: Option<String>,
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

pub(super) fn stored_finding_query<const N: usize>(
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

pub(super) fn stored_finding_from_conn(
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
        current_subject_identity: row.get(3)?,
        scope_kind: row.get(4)?,
        scope_identity: row.get(5)?,
        current_status: row.get(6)?,
        resolved_at: row.get(7)?,
        code: row.get(8)?,
        domain: row.get(9)?,
        stage: row.get(10)?,
        severity: row.get(11)?,
        source: row.get(12)?,
        subject_json: row.get(13)?,
        facts_json: row.get(14)?,
        actions_json: row.get(15)?,
        correlation_id: row.get(16)?,
        connection_id: row.get(17)?,
        project_id: row.get(18)?,
        runtime_session_id: row.get(19)?,
        integration_revision: row.get(20)?,
        observed_at: row.get(21)?,
        causes,
    })
}

fn decode_stored_finding(raw: StoredFindingRaw) -> StoreResult<StoredFinding> {
    let data = decode_finding_data(&raw)?;
    match raw.lifecycle.as_str() {
        "occurrence" => {
            if raw.current_identity_digest.is_some()
                || raw.current_subject_identity.is_some()
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
            let (
                Some(digest),
                Some(subject_identity),
                Some(scope_kind),
                Some(scope_identity),
                Some(status),
            ) = (
                raw.current_identity_digest.as_deref(),
                raw.current_subject_identity.as_deref(),
                raw.scope_kind.as_deref(),
                raw.scope_identity.as_deref(),
                raw.current_status.as_deref(),
            )
            else {
                return Err(corrupt_value(&raw.finding_id, "lifecycle"));
            };
            if raw.runtime_session_id.is_some() {
                return Err(corrupt_value(&raw.finding_id, "runtime_session_id"));
            }
            let scope_kind = parse_scope_kind(scope_kind)
                .ok_or_else(|| corrupt_value(&raw.finding_id, "diagnostic_scope_kind"))?;
            let scope = DiagnosticScope::try_new(scope_kind, scope_identity.to_owned())
                .map_err(|_| corrupt_value(&raw.finding_id, "diagnostic_scope_identity"))?;
            let subject_identity = DiagnosticSubjectIdentity::parse_persisted(subject_identity)
                .map_err(|_| corrupt_value(&raw.finding_id, "current_subject_identity"))?;
            let key = CurrentDiagnosticKey::new(
                scope,
                data.code().clone(),
                data.domain().clone(),
                data.stage().clone(),
                data.source().clone(),
                subject_identity,
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
                data.subject().clone(),
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

pub(super) fn validate_lookup_id(field: &str, value: &str) -> StoreResult<()> {
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

pub(super) fn corrupt_value(record_ref: &str, logical_column: &'static str) -> StoreError {
    StoreError::CorruptOwnerStateValue {
        database_kind: "registry",
        table: "diagnostic_findings",
        record_ref: record_ref.to_owned(),
        logical_column,
    }
}
