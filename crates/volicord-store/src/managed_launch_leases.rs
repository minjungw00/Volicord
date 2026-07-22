//! One-time Registry leases for the managed MCP bootstrap transition.

use std::{
    path::Path,
    str::FromStr,
    time::{Duration, SystemTime},
};

use rusqlite::{params, Connection, OptionalExtension, Row};
use volicord_types::{
    DurableIdGenerator, DurableIdKind, HostKind, IntegrationRevision, McpRuntimeSessionSource,
    RandomDurableIdGenerator, UtcTimestamp, DURABLE_ID_RETRY_LIMIT,
};

use crate::{
    agent_connections::raw_agent_connection_record_from_conn,
    operational_sessions::{
        connection_integration_revision, insert_mcp_runtime_session_in_transaction,
        McpRuntimeSessionRecord, McpRuntimeSessionStart,
    },
    sqlite::{
        begin_immediate_transaction, open_registry_database, open_registry_database_read_only,
        registry_db_path,
    },
    StoreError, StoreResult,
};

const LAUNCH_LEASE_TTL: Duration = Duration::from_secs(30);
const TERMINAL_LEASE_RETENTION: Duration = Duration::from_secs(60 * 60);
const MAX_FINGERPRINT_BYTES: usize = 1024;

/// Exact persisted lifecycle state of one managed launch lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedMcpLaunchLeaseState {
    Issued,
    Consumed,
    Cancelled,
    Expired,
}

impl ManagedMcpLaunchLeaseState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Issued => "issued",
            Self::Consumed => "consumed",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "issued" => Some(Self::Issued),
            "consumed" => Some(Self::Consumed),
            "cancelled" => Some(Self::Cancelled),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

/// Canonical Registry record for one managed launcher bootstrap lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedMcpLaunchLeaseRecord {
    pub launch_lease_id: String,
    pub connection_internal_id: String,
    pub host_kind: HostKind,
    pub expected_integration_revision: String,
    pub expected_launch_fingerprint: String,
    pub issued_at: String,
    pub expires_at: String,
    pub consumed_at: Option<String>,
    pub terminal_state: ManagedMcpLaunchLeaseState,
}

/// Current launcher facts bound into a newly issued lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedMcpLaunchLeaseIssue {
    pub connection_internal_id: String,
    pub host_kind: HostKind,
    pub expected_integration_revision: String,
    pub expected_launch_fingerprint: String,
}

/// In-memory bootstrap claim required to consume one launch lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedMcpLaunchLeaseConsumption {
    pub launch_lease_id: String,
    pub connection_internal_id: String,
    pub host_kind: HostKind,
    pub expected_integration_revision: String,
    pub expected_launch_fingerprint: String,
}

/// Issues one short-lived lease after revalidating current Connection facts.
pub fn issue_managed_mcp_launch_lease(
    runtime_home: impl AsRef<Path>,
    input: ManagedMcpLaunchLeaseIssue,
) -> StoreResult<ManagedMcpLaunchLeaseRecord> {
    let issued_at = SystemTime::now();
    issue_managed_mcp_launch_lease_at(runtime_home.as_ref(), input, issued_at, LAUNCH_LEASE_TTL)
}

fn issue_managed_mcp_launch_lease_at(
    runtime_home: &Path,
    input: ManagedMcpLaunchLeaseIssue,
    issued_at: SystemTime,
    ttl: Duration,
) -> StoreResult<ManagedMcpLaunchLeaseRecord> {
    validate_issue(&input)?;
    if ttl.is_zero() {
        return Err(invalid_lease("launch lease TTL must be positive"));
    }
    let expires_at = issued_at
        .checked_add(ttl)
        .ok_or_else(|| invalid_lease("launch lease expiry is not representable"))?;
    let issued_at_text = timestamp(issued_at);
    let expires_at_text = timestamp(expires_at);
    let cleanup_cutoff = timestamp(
        issued_at
            .checked_sub(TERMINAL_LEASE_RETENTION)
            .unwrap_or(SystemTime::UNIX_EPOCH),
    );
    let path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(path)?;
    let generator = RandomDurableIdGenerator;
    for _ in 0..DURABLE_ID_RETRY_LIMIT {
        let lease_id = generator
            .generate(DurableIdKind::ManagedMcpLaunchLease)
            .map_err(|error| {
                invalid_lease(format!("could not generate launch lease id: {error}"))
            })?;
        let tx = begin_immediate_transaction(&mut conn)?;
        cleanup_leases(&tx, &issued_at_text, &cleanup_cutoff)?;
        require_current_connection(
            &tx,
            &input.connection_internal_id,
            input.host_kind,
            &input.expected_integration_revision,
            &input.expected_launch_fingerprint,
        )?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO managed_mcp_launch_leases (
                launch_lease_id, connection_internal_id, host_kind,
                expected_integration_revision, expected_launch_fingerprint,
                issued_at, expires_at, terminal_state
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'issued')",
            params![
                lease_id,
                input.connection_internal_id,
                input.host_kind.as_str(),
                input.expected_integration_revision,
                input.expected_launch_fingerprint,
                issued_at_text,
                expires_at_text,
            ],
        )?;
        if inserted == 1 {
            let record = lease_from_conn(&tx, &lease_id)?.ok_or_else(lease_not_found)?;
            tx.commit()?;
            return Ok(record);
        }
        tx.rollback()?;
    }
    Err(lease_conflict(
        "launch lease durable-id collision retry limit was exhausted",
    ))
}

/// Atomically consumes one lease and creates its managed runtime session.
pub fn consume_managed_mcp_launch_lease_and_start_runtime(
    runtime_home: impl AsRef<Path>,
    claim: ManagedMcpLaunchLeaseConsumption,
    runtime: McpRuntimeSessionStart,
) -> StoreResult<McpRuntimeSessionRecord> {
    validate_consumption(&claim)?;
    if runtime.session_source != McpRuntimeSessionSource::ManagedHost {
        return Err(invalid_lease(
            "launch lease consumption requires session_source=managed_host",
        ));
    }
    if runtime.connection_internal_id != claim.connection_internal_id {
        return Err(lease_conflict(
            "launch lease Connection does not match runtime creation",
        ));
    }
    let consumed_at = SystemTime::now();
    consume_managed_mcp_launch_lease_and_start_runtime_at(
        runtime_home.as_ref(),
        claim,
        runtime,
        consumed_at,
    )
}

fn consume_managed_mcp_launch_lease_and_start_runtime_at(
    runtime_home: &Path,
    claim: ManagedMcpLaunchLeaseConsumption,
    runtime: McpRuntimeSessionStart,
    consumed_at: SystemTime,
) -> StoreResult<McpRuntimeSessionRecord> {
    let consumed_at_text = timestamp(consumed_at);
    let cleanup_cutoff = timestamp(
        consumed_at
            .checked_sub(TERMINAL_LEASE_RETENTION)
            .unwrap_or(SystemTime::UNIX_EPOCH),
    );
    let path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(path)?;
    let generator = RandomDurableIdGenerator;
    for _ in 0..DURABLE_ID_RETRY_LIMIT {
        let runtime_session_id = generator
            .generate(DurableIdKind::McpRuntimeSession)
            .map_err(|error| invalid_lease(format!("could not generate runtime id: {error}")))?;
        let tx = begin_immediate_transaction(&mut conn)?;
        cleanup_leases(&tx, &consumed_at_text, &cleanup_cutoff)?;
        let lease = lease_from_conn(&tx, &claim.launch_lease_id)?.ok_or_else(lease_not_found)?;
        if lease.terminal_state != ManagedMcpLaunchLeaseState::Issued {
            return Err(lease_conflict("launch lease is no longer consumable"));
        }
        if consumed_at_text >= lease.expires_at {
            tx.execute(
                "UPDATE managed_mcp_launch_leases
                    SET terminal_state = 'expired'
                  WHERE launch_lease_id = ?1 AND terminal_state = 'issued'",
                [&claim.launch_lease_id],
            )?;
            tx.commit()?;
            return Err(lease_conflict("launch lease has expired"));
        }
        if lease.connection_internal_id != claim.connection_internal_id
            || lease.host_kind != claim.host_kind
            || lease.expected_integration_revision != claim.expected_integration_revision
            || lease.expected_launch_fingerprint != claim.expected_launch_fingerprint
        {
            return Err(lease_conflict(
                "launch lease claim does not match the issued contract",
            ));
        }
        require_current_connection(
            &tx,
            &claim.connection_internal_id,
            claim.host_kind,
            &claim.expected_integration_revision,
            &claim.expected_launch_fingerprint,
        )?;
        let changed = tx.execute(
            "UPDATE managed_mcp_launch_leases
                SET consumed_at = ?2, terminal_state = 'consumed'
              WHERE launch_lease_id = ?1
                AND terminal_state = 'issued'
                AND expires_at > ?2",
            params![claim.launch_lease_id, consumed_at_text],
        )?;
        if changed != 1 {
            return Err(lease_conflict("launch lease could not be consumed"));
        }
        if let Some(record) = insert_mcp_runtime_session_in_transaction(
            &tx,
            &runtime_session_id,
            &runtime,
            Some(&claim.expected_integration_revision),
        )? {
            tx.commit()?;
            return Ok(record);
        }
        tx.rollback()?;
    }
    Err(lease_conflict(
        "runtime durable-id collision retry limit was exhausted",
    ))
}

/// Cancels one still-unused lease during deterministic launcher cleanup.
pub fn cancel_managed_mcp_launch_lease(
    runtime_home: impl AsRef<Path>,
    launch_lease_id: &str,
) -> StoreResult<ManagedMcpLaunchLeaseRecord> {
    validate_text("launch_lease_id", launch_lease_id, 192)?;
    let now = SystemTime::now();
    let now_text = timestamp(now);
    let cleanup_cutoff = timestamp(
        now.checked_sub(TERMINAL_LEASE_RETENTION)
            .unwrap_or(SystemTime::UNIX_EPOCH),
    );
    let path = registry_db_path(runtime_home);
    let mut conn = open_registry_database(path)?;
    let tx = begin_immediate_transaction(&mut conn)?;
    cleanup_leases(&tx, &now_text, &cleanup_cutoff)?;
    tx.execute(
        "UPDATE managed_mcp_launch_leases
            SET terminal_state = 'cancelled'
          WHERE launch_lease_id = ?1 AND terminal_state = 'issued'",
        [launch_lease_id],
    )?;
    let record = lease_from_conn(&tx, launch_lease_id)?.ok_or_else(lease_not_found)?;
    tx.commit()?;
    Ok(record)
}

/// Reads one lease for bounded diagnostic and test inspection.
pub fn managed_mcp_launch_lease(
    runtime_home: impl AsRef<Path>,
    launch_lease_id: &str,
) -> StoreResult<Option<ManagedMcpLaunchLeaseRecord>> {
    validate_text("launch_lease_id", launch_lease_id, 192)?;
    let path = registry_db_path(runtime_home);
    if !path.exists() {
        return Ok(None);
    }
    let conn = open_registry_database_read_only(path)?;
    lease_from_conn(&conn, launch_lease_id)
}

fn require_current_connection(
    tx: &Connection,
    connection_id: &str,
    host_kind: HostKind,
    expected_revision: &str,
    expected_fingerprint: &str,
) -> StoreResult<()> {
    let connection =
        raw_agent_connection_record_from_conn(tx, connection_id)?.ok_or_else(|| {
            StoreError::NotFound {
                entity: "agent_connection",
                id: "current".to_owned(),
            }
        })?;
    if !connection.enabled {
        return Err(lease_conflict("launch lease Connection is disabled"));
    }
    if connection.host_kind != host_kind.as_str() {
        return Err(lease_conflict("launch lease host kind does not match"));
    }
    if connection.managed_fingerprint != expected_fingerprint {
        return Err(lease_conflict(
            "launch lease managed fingerprint does not match current configuration",
        ));
    }
    let revision = connection_integration_revision(&connection)?;
    if revision.as_str() != expected_revision {
        return Err(lease_conflict(
            "launch lease integration revision does not match current Connection",
        ));
    }
    Ok(())
}

fn cleanup_leases(tx: &Connection, now: &str, cutoff: &str) -> StoreResult<()> {
    tx.execute(
        "UPDATE managed_mcp_launch_leases
            SET terminal_state = 'expired'
          WHERE terminal_state = 'issued' AND expires_at <= ?1",
        [now],
    )?;
    tx.execute(
        "DELETE FROM managed_mcp_launch_leases
          WHERE terminal_state <> 'issued' AND expires_at < ?1",
        [cutoff],
    )?;
    Ok(())
}

fn lease_from_conn(
    conn: &Connection,
    launch_lease_id: &str,
) -> StoreResult<Option<ManagedMcpLaunchLeaseRecord>> {
    conn.query_row(
        "SELECT launch_lease_id, connection_internal_id, host_kind,
                expected_integration_revision, expected_launch_fingerprint,
                issued_at, expires_at, consumed_at, terminal_state
           FROM managed_mcp_launch_leases
          WHERE launch_lease_id = ?1",
        [launch_lease_id],
        lease_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|record| record.map(validate_stored_lease).transpose())
}

fn lease_from_row(row: &Row<'_>) -> rusqlite::Result<ManagedMcpLaunchLeaseRecord> {
    let host = row.get::<_, String>(2)?;
    let host_kind = host.parse::<HostKind>().map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            "invalid managed launch lease host kind".into(),
        )
    })?;
    let state_text = row.get::<_, String>(8)?;
    let terminal_state = ManagedMcpLaunchLeaseState::parse(&state_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            8,
            rusqlite::types::Type::Text,
            "invalid managed launch lease state".into(),
        )
    })?;
    Ok(ManagedMcpLaunchLeaseRecord {
        launch_lease_id: row.get(0)?,
        connection_internal_id: row.get(1)?,
        host_kind,
        expected_integration_revision: row.get(3)?,
        expected_launch_fingerprint: row.get(4)?,
        issued_at: row.get(5)?,
        expires_at: row.get(6)?,
        consumed_at: row.get(7)?,
        terminal_state,
    })
}

fn validate_stored_lease(
    record: ManagedMcpLaunchLeaseRecord,
) -> StoreResult<ManagedMcpLaunchLeaseRecord> {
    validate_text("launch_lease_id", &record.launch_lease_id, 192)
        .map_err(|_| corrupt_lease(&record, "launch_lease_id"))?;
    validate_text(
        "connection_internal_id",
        &record.connection_internal_id,
        1024,
    )
    .map_err(|_| corrupt_lease(&record, "connection_internal_id"))?;
    IntegrationRevision::parse(record.expected_integration_revision.clone())
        .map_err(|_| corrupt_lease(&record, "expected_integration_revision"))?;
    validate_fingerprint(&record.expected_launch_fingerprint)
        .map_err(|_| corrupt_lease(&record, "expected_launch_fingerprint"))?;
    let issued = UtcTimestamp::from_str(&record.issued_at)
        .map_err(|_| corrupt_lease(&record, "issued_at"))?;
    let expires = UtcTimestamp::from_str(&record.expires_at)
        .map_err(|_| corrupt_lease(&record, "expires_at"))?;
    if expires <= issued {
        return Err(corrupt_lease(&record, "expires_at"));
    }
    match (record.terminal_state, record.consumed_at.as_deref()) {
        (ManagedMcpLaunchLeaseState::Consumed, Some(value)) => {
            let consumed =
                UtcTimestamp::from_str(value).map_err(|_| corrupt_lease(&record, "consumed_at"))?;
            if consumed < issued || consumed >= expires {
                return Err(corrupt_lease(&record, "consumed_at"));
            }
        }
        (ManagedMcpLaunchLeaseState::Consumed, None) | (_, Some(_)) => {
            return Err(corrupt_lease(&record, "consumed_at"));
        }
        _ => {}
    }
    Ok(record)
}

fn validate_issue(input: &ManagedMcpLaunchLeaseIssue) -> StoreResult<()> {
    validate_text(
        "connection_internal_id",
        &input.connection_internal_id,
        1024,
    )?;
    IntegrationRevision::parse(input.expected_integration_revision.clone())
        .map_err(|_| invalid_lease("expected integration revision must be canonical sha256"))?;
    validate_fingerprint(&input.expected_launch_fingerprint)
}

fn validate_consumption(input: &ManagedMcpLaunchLeaseConsumption) -> StoreResult<()> {
    validate_text("launch_lease_id", &input.launch_lease_id, 192)?;
    validate_text(
        "connection_internal_id",
        &input.connection_internal_id,
        1024,
    )?;
    IntegrationRevision::parse(input.expected_integration_revision.clone())
        .map_err(|_| invalid_lease("expected integration revision must be canonical sha256"))?;
    validate_fingerprint(&input.expected_launch_fingerprint)
}

fn validate_fingerprint(value: &str) -> StoreResult<()> {
    validate_text("expected_launch_fingerprint", value, MAX_FINGERPRINT_BYTES)
}

fn validate_text(field: &'static str, value: &str, max: usize) -> StoreResult<()> {
    if value.is_empty() || value.len() > max || value.chars().any(char::is_control) {
        Err(invalid_lease(format!(
            "{field} must be 1 through {max} non-control UTF-8 bytes"
        )))
    } else {
        Ok(())
    }
}

fn timestamp(value: SystemTime) -> String {
    UtcTimestamp::from_datetime(chrono::DateTime::<chrono::Utc>::from(value)).to_canonical_string()
}

fn lease_not_found() -> StoreError {
    StoreError::NotFound {
        entity: "managed_mcp_launch_lease",
        id: "current".to_owned(),
    }
}

fn lease_conflict(detail: impl Into<String>) -> StoreError {
    StoreError::Conflict {
        entity: "managed_mcp_launch_lease",
        id: "current".to_owned(),
        detail: detail.into(),
    }
}

fn invalid_lease(detail: impl Into<String>) -> StoreError {
    StoreError::InvalidInput {
        detail: detail.into(),
    }
}

fn corrupt_lease(_record: &ManagedMcpLaunchLeaseRecord, field: &'static str) -> StoreError {
    StoreError::CorruptOwnerStateValue {
        database_kind: "registry",
        table: "managed_mcp_launch_leases",
        record_ref: "current".to_owned(),
        logical_column: field,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent_connections::{agent_connection_record, AgentConnectionRegistration},
        bootstrap::{initialize_runtime_home, register_project, ProjectRegistration},
        operational_sessions::connection_integration_revision,
    };
    use volicord_types::McpRuntimeSessionSource;

    fn fixture(
        name: &str,
    ) -> Result<(tempfile::TempDir, std::path::PathBuf, String, String), Box<dyn std::error::Error>>
    {
        let temp = tempfile::Builder::new().prefix(name).tempdir()?;
        let runtime_home = temp.path().join("runtime");
        initialize_runtime_home(&runtime_home, "runtime_home_launch_lease", "{}")?;
        let repo = temp.path().join("repo");
        std::fs::create_dir_all(&repo)?;
        register_project(
            &runtime_home,
            ProjectRegistration {
                project_id: "project_alpha".to_owned(),
                repo_root: repo,
                project_home: None,
                status: "active".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        crate::agent_connections::ensure_agent_connection(
            &runtime_home,
            AgentConnectionRegistration {
                connection_internal_id: "connection_alpha".to_owned(),
                host_kind: "codex".to_owned(),
                intent: "personal".to_owned(),
                host_scope: "user".to_owned(),
                server_name: "volicord".to_owned(),
                config_target: temp.path().join("config.toml").display().to_string(),
                mode: "workflow".to_owned(),
                enabled: true,
                managed_fingerprint: "fingerprint-alpha".to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )?;
        let connection =
            agent_connection_record(&runtime_home, "connection_alpha")?.expect("connection");
        let revision = connection_integration_revision(&connection)?.into_inner();
        Ok((temp, runtime_home, revision, connection.managed_fingerprint))
    }

    fn issue(
        runtime_home: &Path,
        revision: &str,
        fingerprint: &str,
        issued_at: SystemTime,
        ttl: Duration,
    ) -> StoreResult<ManagedMcpLaunchLeaseRecord> {
        issue_managed_mcp_launch_lease_at(
            runtime_home,
            ManagedMcpLaunchLeaseIssue {
                connection_internal_id: "connection_alpha".to_owned(),
                host_kind: HostKind::Codex,
                expected_integration_revision: revision.to_owned(),
                expected_launch_fingerprint: fingerprint.to_owned(),
            },
            issued_at,
            ttl,
        )
    }

    fn runtime(started_at: SystemTime) -> McpRuntimeSessionStart {
        McpRuntimeSessionStart {
            connection_internal_id: "connection_alpha".to_owned(),
            session_source: McpRuntimeSessionSource::ManagedHost,
            observed_host_executable_version: None,
            process_id: 7,
            process_started_at: timestamp(started_at),
        }
    }

    fn claim(lease: &ManagedMcpLaunchLeaseRecord) -> ManagedMcpLaunchLeaseConsumption {
        ManagedMcpLaunchLeaseConsumption {
            launch_lease_id: lease.launch_lease_id.clone(),
            connection_internal_id: lease.connection_internal_id.clone(),
            host_kind: lease.host_kind,
            expected_integration_revision: lease.expected_integration_revision.clone(),
            expected_launch_fingerprint: lease.expected_launch_fingerprint.clone(),
        }
    }

    fn runtime_count(runtime_home: &Path) -> StoreResult<i64> {
        let conn = open_registry_database_read_only(registry_db_path(runtime_home))?;
        Ok(
            conn.query_row("SELECT COUNT(*) FROM mcp_runtime_sessions", [], |row| {
                row.get(0)
            })?,
        )
    }

    #[test]
    fn lease_consumption_is_one_time_and_creates_the_managed_runtime_atomically(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, runtime_home, revision, fingerprint) = fixture("launch-lease-once")?;
        let now = SystemTime::now();
        let lease = issue(
            &runtime_home,
            &revision,
            &fingerprint,
            now,
            Duration::from_secs(30),
        )?;
        let consumed_at = now.checked_add(Duration::from_secs(1)).unwrap();
        let created = consume_managed_mcp_launch_lease_and_start_runtime_at(
            &runtime_home,
            claim(&lease),
            runtime(consumed_at),
            consumed_at,
        )?;
        assert_eq!(created.session_source, McpRuntimeSessionSource::ManagedHost);
        assert_eq!(
            managed_mcp_launch_lease(&runtime_home, &lease.launch_lease_id)?
                .expect("lease")
                .terminal_state,
            ManagedMcpLaunchLeaseState::Consumed
        );
        let replay = consume_managed_mcp_launch_lease_and_start_runtime_at(
            &runtime_home,
            claim(&lease),
            runtime(consumed_at),
            consumed_at,
        );
        assert!(replay.is_err());
        assert_eq!(runtime_count(&runtime_home)?, 1);
        Ok(())
    }

    #[test]
    fn expired_and_mismatched_leases_create_no_runtime() -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, runtime_home, revision, fingerprint) = fixture("launch-lease-rejections")?;
        let now = SystemTime::now();
        let expired = issue(
            &runtime_home,
            &revision,
            &fingerprint,
            now,
            Duration::from_secs(1),
        )?;
        let after_expiry = now.checked_add(Duration::from_secs(2)).unwrap();
        assert!(consume_managed_mcp_launch_lease_and_start_runtime_at(
            &runtime_home,
            claim(&expired),
            runtime(after_expiry),
            after_expiry,
        )
        .is_err());
        assert_eq!(runtime_count(&runtime_home)?, 0);

        let lease = issue(
            &runtime_home,
            &revision,
            &fingerprint,
            now,
            Duration::from_secs(30),
        )?;
        let mut wrong_connection = claim(&lease);
        wrong_connection.connection_internal_id = "connection_other".to_owned();
        assert!(consume_managed_mcp_launch_lease_and_start_runtime_at(
            &runtime_home,
            wrong_connection,
            runtime(now),
            now,
        )
        .is_err());
        for mutation in ["revision", "fingerprint"] {
            let mut mismatch = claim(&lease);
            if mutation == "revision" {
                mismatch.expected_integration_revision = format!("sha256:{}", "0".repeat(64));
            } else {
                mismatch.expected_launch_fingerprint = "other-fingerprint".to_owned();
            }
            assert!(consume_managed_mcp_launch_lease_and_start_runtime_at(
                &runtime_home,
                mismatch,
                runtime(now),
                now,
            )
            .is_err());
        }
        assert_eq!(runtime_count(&runtime_home)?, 0);
        Ok(())
    }

    #[test]
    fn current_connection_revision_and_fingerprint_drift_reject_issued_leases(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for mutation in ["revision", "fingerprint"] {
            let (_temp, runtime_home, revision, fingerprint) =
                fixture(&format!("launch-lease-current-{mutation}"))?;
            let now = SystemTime::now();
            let lease = issue(
                &runtime_home,
                &revision,
                &fingerprint,
                now,
                Duration::from_secs(30),
            )?;
            let conn = open_registry_database(registry_db_path(&runtime_home))?;
            if mutation == "revision" {
                conn.execute(
                    "UPDATE agent_connections
                        SET integration_generation = integration_generation + 1
                      WHERE connection_internal_id = 'connection_alpha'",
                    [],
                )?;
            } else {
                conn.execute(
                    "UPDATE agent_connections
                        SET managed_fingerprint = 'fingerprint-drifted'
                      WHERE connection_internal_id = 'connection_alpha'",
                    [],
                )?;
            }
            assert!(consume_managed_mcp_launch_lease_and_start_runtime_at(
                &runtime_home,
                claim(&lease),
                runtime(now),
                now,
            )
            .is_err());
            assert_eq!(runtime_count(&runtime_home)?, 0);
        }
        Ok(())
    }

    #[test]
    fn direct_managed_runtime_creation_is_rejected_without_a_lease(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, runtime_home, _revision, _fingerprint) = fixture("launch-lease-required")?;
        assert!(crate::operational_sessions::start_mcp_runtime_session(
            &runtime_home,
            runtime(SystemTime::now()),
        )
        .is_err());
        assert_eq!(runtime_count(&runtime_home)?, 0);
        Ok(())
    }

    #[test]
    fn launcher_cleanup_cancels_only_an_unused_lease() -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, runtime_home, revision, fingerprint) = fixture("launch-lease-cancel")?;
        let lease = issue_managed_mcp_launch_lease(
            &runtime_home,
            ManagedMcpLaunchLeaseIssue {
                connection_internal_id: "connection_alpha".to_owned(),
                host_kind: HostKind::Codex,
                expected_integration_revision: revision,
                expected_launch_fingerprint: fingerprint,
            },
        )?;
        let cancelled = cancel_managed_mcp_launch_lease(&runtime_home, &lease.launch_lease_id)?;
        assert_eq!(
            cancelled.terminal_state,
            ManagedMcpLaunchLeaseState::Cancelled
        );
        Ok(())
    }
}
