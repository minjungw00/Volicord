use rusqlite::{params, Connection, OptionalExtension, Row};
use volicord_types::{GuardIntegrationVerificationStatus, UtcTimestamp};

use super::{coordinate::VerificationCurrentCoordinate, GuardIntegrationVerificationRunRecord};
use crate::{StoreError, StoreResult};

const RUN_SELECT: &str = "SELECT
    verification_id, connection_internal_id, project_internal_id,
    runtime_session_id, host_session_id, host_turn_id,
    guard_installation_id, integration_revision, policy_hash,
    hook_contract_digest, expected_probe_tool, expected_host_callable_name,
    created_at, expires_at, status,
    probe_acknowledged_at, completed_at, matched_prompt_event_id,
    matched_pre_tool_event_id, matched_post_tool_event_id,
    terminal_finding_code, terminal_finding_summary
  FROM guard_integration_verification_runs";

pub(super) struct NewVerificationRun<'a> {
    pub verification_id: &'a str,
    pub coordinate: &'a VerificationCurrentCoordinate,
    pub created_at: &'a str,
    pub expires_at: &'a str,
    pub matched_prompt_event_id: &'a str,
}

pub(super) struct ActiveEventRunLookup<'a> {
    pub connection_internal_id: &'a str,
    pub host_session_id: &'a str,
    pub host_turn_id: &'a str,
    pub guard_installation_id: &'a str,
    pub integration_revision: &'a str,
    pub policy_hash: &'a str,
}

pub(super) struct ActiveAcquisitionRunLookup<'a> {
    pub connection_internal_id: &'a str,
    pub project_internal_id: &'a str,
    pub guard_installation_id: &'a str,
    pub integration_revision: &'a str,
    pub policy_hash: &'a str,
}

pub(super) struct CorrelatedEventIds<'a> {
    pub prompt: &'a str,
    pub pre_tool: &'a str,
    pub post_tool: &'a str,
}

pub(super) fn insert_run(conn: &Connection, new_run: NewVerificationRun<'_>) -> StoreResult<()> {
    let caller = new_run.coordinate.caller();
    conn.execute(
        "INSERT INTO guard_integration_verification_runs (
            verification_id, connection_internal_id, project_internal_id,
            runtime_session_id, host_session_id, host_turn_id,
            guard_installation_id, integration_revision, policy_hash,
            hook_contract_digest, expected_probe_tool, expected_host_callable_name,
            created_at, expires_at, status, matched_prompt_event_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                   ?14, 'active', ?15)",
        params![
            new_run.verification_id,
            caller.connection_internal_id(),
            new_run.coordinate.project_internal_id(),
            caller.runtime_session_id(),
            caller.host_session_id(),
            caller.host_turn_id(),
            new_run.coordinate.guard_installation_id(),
            new_run.coordinate.integration_revision(),
            new_run.coordinate.policy_hash(),
            new_run.coordinate.hook_contract_digest(),
            new_run.coordinate.expected_probe_tool(),
            new_run.coordinate.expected_host_callable_name(),
            new_run.created_at,
            new_run.expires_at,
            new_run.matched_prompt_event_id,
        ],
    )?;
    Ok(())
}

pub(super) fn acknowledge_probe_first_write(
    conn: &Connection,
    verification_id: &str,
    observed_at: &str,
) -> StoreResult<()> {
    conn.execute(
        "UPDATE guard_integration_verification_runs
            SET probe_acknowledged_at = COALESCE(probe_acknowledged_at, ?2)
          WHERE verification_id = ?1
            AND status = 'active'
            AND probe_acknowledged_at IS NULL
            AND expires_at > ?2",
        params![verification_id, observed_at],
    )?;
    Ok(())
}

pub(super) fn expire_active_runs(conn: &Connection, observed_at: &str) -> StoreResult<()> {
    conn.execute(
        "UPDATE guard_integration_verification_runs
            SET status = 'expired', completed_at = ?1,
                terminal_finding_code = 'verification_expired',
                terminal_finding_summary = 'The bounded integration-verification window expired.'
          WHERE status = 'active' AND expires_at <= ?1",
        [observed_at],
    )?;
    Ok(())
}

pub(super) fn complete_run(
    conn: &Connection,
    verification_id: &str,
    completed_at: &str,
    events: CorrelatedEventIds<'_>,
) -> StoreResult<()> {
    conn.execute(
        "UPDATE guard_integration_verification_runs
            SET status = 'passed', completed_at = ?2,
                matched_prompt_event_id = ?3,
                matched_pre_tool_event_id = ?4,
                matched_post_tool_event_id = ?5
          WHERE verification_id = ?1 AND status = 'active'",
        params![
            verification_id,
            completed_at,
            events.prompt,
            events.pre_tool,
            events.post_tool,
        ],
    )?;
    Ok(())
}

pub(super) fn run_by_id(
    conn: &Connection,
    verification_id: &str,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    conn.query_row(
        &format!("{RUN_SELECT} WHERE verification_id = ?1"),
        [verification_id],
        run_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn resumable_run(
    conn: &Connection,
    coordinate: &VerificationCurrentCoordinate,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    let caller = coordinate.caller();
    conn.query_row(
        &format!(
            "{RUN_SELECT}
              WHERE connection_internal_id = ?1
                AND runtime_session_id = ?2
                AND host_turn_id = ?3
                AND integration_revision = ?4
                AND status IN ('active', 'passed')
              ORDER BY CASE status WHEN 'active' THEN 0 ELSE 1 END,
                       created_at DESC
              LIMIT 1"
        ),
        params![
            caller.connection_internal_id(),
            caller.runtime_session_id(),
            caller.host_turn_id(),
            coordinate.integration_revision(),
        ],
        run_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn active_run_for_event(
    conn: &Connection,
    lookup: ActiveEventRunLookup<'_>,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    conn.query_row(
        &format!(
            "{RUN_SELECT}
              WHERE connection_internal_id = ?1
                AND host_session_id = ?2
                AND host_turn_id = ?3
                AND guard_installation_id = ?4
                AND integration_revision = ?5
                AND policy_hash = ?6
                AND status IN ('active', 'passed')
              ORDER BY CASE status WHEN 'active' THEN 0 ELSE 1 END,
                       created_at DESC, verification_id DESC
              LIMIT 1"
        ),
        params![
            lookup.connection_internal_id,
            lookup.host_session_id,
            lookup.host_turn_id,
            lookup.guard_installation_id,
            lookup.integration_revision,
            lookup.policy_hash,
        ],
        run_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn active_run_for_acquisition(
    conn: &Connection,
    lookup: ActiveAcquisitionRunLookup<'_>,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    conn.query_row(
        &format!(
            "{RUN_SELECT}
              WHERE connection_internal_id = ?1
                AND project_internal_id = ?2
                AND guard_installation_id = ?3
                AND integration_revision = ?4
                AND policy_hash = ?5
                AND status = 'active'
              ORDER BY created_at DESC, verification_id DESC
              LIMIT 1"
        ),
        params![
            lookup.connection_internal_id,
            lookup.project_internal_id,
            lookup.guard_installation_id,
            lookup.integration_revision,
            lookup.policy_hash,
        ],
        run_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn latest_run_for_connection(
    conn: &Connection,
    connection_internal_id: &str,
    integration_revision: &str,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    conn.query_row(
        &format!(
            "{RUN_SELECT}
              WHERE connection_internal_id = ?1
                AND integration_revision = ?2
              ORDER BY created_at DESC, verification_id DESC
              LIMIT 1"
        ),
        [connection_internal_id, integration_revision],
        run_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn run_from_row(row: &Row<'_>) -> rusqlite::Result<GuardIntegrationVerificationRunRecord> {
    Ok(GuardIntegrationVerificationRunRecord {
        verification_id: row.get(0)?,
        connection_internal_id: row.get(1)?,
        project_internal_id: row.get(2)?,
        runtime_session_id: row.get(3)?,
        host_session_id: row.get(4)?,
        host_turn_id: row.get(5)?,
        guard_installation_id: row.get(6)?,
        integration_revision: row.get(7)?,
        policy_hash: row.get(8)?,
        hook_contract_digest: row.get(9)?,
        expected_probe_tool: row.get(10)?,
        expected_host_callable_name: row.get(11)?,
        created_at: row.get(12)?,
        expires_at: row.get(13)?,
        status: row.get(14)?,
        probe_acknowledged_at: row.get(15)?,
        completed_at: row.get(16)?,
        matched_prompt_event_id: row.get(17)?,
        matched_pre_tool_event_id: row.get(18)?,
        matched_post_tool_event_id: row.get(19)?,
        terminal_finding_code: row.get(20)?,
        terminal_finding_summary: row.get(21)?,
    })
}

pub(super) fn parse_timestamp(field: &str, value: &str) -> StoreResult<UtcTimestamp> {
    UtcTimestamp::parse(value).map_err(|_| StoreError::InvalidInput {
        detail: format!("{field} must be an RFC 3339 timestamp"),
    })
}

pub(super) fn parse_status(value: &str) -> StoreResult<GuardIntegrationVerificationStatus> {
    match value {
        "active" => Ok(GuardIntegrationVerificationStatus::Active),
        "passed" => Ok(GuardIntegrationVerificationStatus::Passed),
        "failed" => Ok(GuardIntegrationVerificationStatus::Failed),
        "expired" => Ok(GuardIntegrationVerificationStatus::Expired),
        _ => Err(StoreError::CorruptStoredValue {
            database_kind: "registry",
            field: "guard_integration_verification_runs.status",
        }),
    }
}

#[cfg(test)]
pub(super) enum StoredOwnerField {
    PolicyHash,
    HookContractDigest,
    IntegrationRevision,
}

#[cfg(test)]
pub(super) fn overwrite_owner_field_for_test(
    conn: &Connection,
    verification_id: &str,
    field: StoredOwnerField,
    value: &str,
) -> StoreResult<()> {
    let statement = match field {
        StoredOwnerField::PolicyHash => {
            "UPDATE guard_integration_verification_runs
                SET policy_hash = ?2
              WHERE verification_id = ?1"
        }
        StoredOwnerField::HookContractDigest => {
            "UPDATE guard_integration_verification_runs
                SET hook_contract_digest = ?2
              WHERE verification_id = ?1"
        }
        StoredOwnerField::IntegrationRevision => {
            "UPDATE guard_integration_verification_runs
                SET integration_revision = ?2
              WHERE verification_id = ?1"
        }
    };
    conn.execute(statement, params![verification_id, value])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use volicord_types::GuardIntegrationVerificationStatus;

    use super::*;

    const HASH: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn every_status_spelling_decodes() {
        for (stored, expected) in [
            ("active", GuardIntegrationVerificationStatus::Active),
            ("passed", GuardIntegrationVerificationStatus::Passed),
            ("failed", GuardIntegrationVerificationStatus::Failed),
            ("expired", GuardIntegrationVerificationStatus::Expired),
        ] {
            assert_eq!(parse_status(stored).expect("status"), expected);
        }
        assert!(matches!(
            parse_status("complete"),
            Err(StoreError::CorruptStoredValue { .. })
        ));
    }

    #[test]
    fn malformed_timestamp_is_rejected() {
        assert!(matches!(
            parse_timestamp("expires_at", "not-a-timestamp"),
            Err(StoreError::InvalidInput { .. })
        ));
    }

    #[test]
    fn required_columns_and_optional_fields_decode_from_sql_row() {
        let conn = Connection::open_in_memory().expect("database");
        let sql = format!(
            "SELECT
                'guard_verification_row', 'connection', 'project', 'runtime',
                'session', 'turn', 'installation', '{HASH}', '{HASH}', '{HASH}',
                'volicord.guard_probe', 'mcp__volicord__volicord_guard_probe',
                '2026-07-23T00:00:00Z',
                '2026-07-23T00:05:00Z', 'failed', NULL,
                '2026-07-23T00:01:00Z', 'prompt', NULL, NULL,
                'verification_coordinate_stale', ?1"
        );
        let summary = "x".repeat(4096);
        let record = conn
            .query_row(&sql, [summary.as_str()], run_from_row)
            .expect("row");
        assert_eq!(
            record.terminal_finding_summary.as_deref(),
            Some(summary.as_str())
        );
        assert!(record.probe_acknowledged_at.is_none());

        let missing = format!(
            "SELECT
                NULL, 'connection', 'project', 'runtime', 'session', 'turn',
                'installation', '{HASH}', '{HASH}', '{HASH}',
                'volicord.guard_probe', 'mcp__volicord__volicord_guard_probe',
                '2026-07-23T00:00:00Z',
                '2026-07-23T00:05:00Z', 'active',
                NULL, NULL, 'prompt', NULL, NULL, NULL, NULL"
        );
        assert!(conn.query_row(&missing, [], run_from_row).is_err());
    }
}
