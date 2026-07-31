use rusqlite::{params, Connection, OptionalExtension, Row};
use volicord_types::integration_verification::{
    GuardIntegrationVerificationStatus, GuardVerificationRepairReason, GuardVerificationRetryPolicy,
};
use volicord_types::values::UtcTimestamp;

use super::{coordinate::VerificationCurrentCoordinate, GuardIntegrationVerificationRunRecord};
use crate::{StoreError, StoreResult};

const RUN_SELECT: &str = "SELECT
    verification_id, connection_internal_id, project_internal_id, project_id,
    runtime_session_id, host_session_id, host_turn_id,
    integration_revision, guard_installation_id, host_contract_profile,
    hook_definition_digest, policy_digest,
    expected_probe_tool, expected_host_callable_name,
    observation_policy_kind, observation_deadline_at,
    allowed_status_reads, status_read_count,
    created_at, cleanup_after, status,
    probe_acknowledged_at, completed_at, matched_prompt_event_id,
    matched_pre_tool_event_id, matched_post_tool_event_id,
    repair_reason, retry_policy,
    terminal_finding_code, terminal_finding_summary
  FROM guard_integration_verification_runs";

pub(super) struct NewVerificationRun<'a> {
    pub verification_id: &'a str,
    pub coordinate: &'a VerificationCurrentCoordinate,
    pub created_at: &'a str,
    pub cleanup_after: &'a str,
    pub matched_prompt_event_id: &'a str,
}

pub(super) struct ActiveEventRunLookup<'a> {
    pub connection_internal_id: &'a str,
    pub host_session_id: &'a str,
    pub host_turn_id: &'a str,
    pub guard_installation_id: &'a str,
    pub integration_revision: &'a str,
    pub policy_digest: &'a str,
}

pub(super) struct ActiveAcquisitionRunLookup<'a> {
    pub connection_internal_id: &'a str,
    pub project_internal_id: &'a str,
    pub guard_installation_id: &'a str,
    pub integration_revision: &'a str,
    pub policy_digest: &'a str,
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
            verification_id, connection_internal_id, project_internal_id, project_id,
            runtime_session_id, host_session_id, host_turn_id,
            integration_revision, guard_installation_id, host_contract_profile,
            hook_definition_digest, policy_digest,
            expected_probe_tool, expected_host_callable_name,
            observation_policy_kind, allowed_status_reads,
            created_at, cleanup_after, status, matched_prompt_event_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                   ?14, ?15, ?16, ?17, ?18, 'awaiting_probe', ?19)",
        params![
            new_run.verification_id,
            caller.connection_internal_id(),
            new_run.coordinate.project_internal_id(),
            new_run.coordinate.semantic().project_id.as_str(),
            caller.runtime_session_id(),
            caller.host_session_id(),
            caller.host_turn_id(),
            new_run.coordinate.integration_revision(),
            new_run.coordinate.guard_installation_id(),
            new_run.coordinate.host_contract_profile().as_str(),
            new_run.coordinate.hook_definition_digest(),
            new_run.coordinate.policy_digest(),
            new_run.coordinate.expected_probe_tool(),
            new_run.coordinate.expected_host_callable_name(),
            new_run.coordinate.observation_policy().kind(),
            new_run
                .coordinate
                .observation_policy()
                .allowed_status_reads(),
            new_run.created_at,
            new_run.cleanup_after,
            new_run.matched_prompt_event_id,
        ],
    )?;
    Ok(())
}

pub(super) fn acknowledge_probe_first_write(
    conn: &Connection,
    verification_id: &str,
    observed_at: &str,
    observation_deadline_at: Option<&str>,
) -> StoreResult<()> {
    conn.execute(
        "UPDATE guard_integration_verification_runs
            SET probe_acknowledged_at = ?2,
                observation_deadline_at = ?3,
                status = 'awaiting_observation'
          WHERE verification_id = ?1
            AND status = 'awaiting_probe'
            AND probe_acknowledged_at IS NULL",
        params![verification_id, observed_at, observation_deadline_at],
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
            SET status = 'complete', completed_at = ?2,
                matched_prompt_event_id = ?3,
                matched_pre_tool_event_id = ?4,
                matched_post_tool_event_id = ?5
          WHERE verification_id = ?1 AND status = 'awaiting_observation'",
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

pub(super) fn mark_repair_required(
    conn: &Connection,
    verification_id: &str,
    completed_at: &str,
    reason: GuardVerificationRepairReason,
    retry_policy: GuardVerificationRetryPolicy,
    finding_code: &str,
    finding_summary: &str,
) -> StoreResult<()> {
    conn.execute(
        "UPDATE guard_integration_verification_runs
            SET status = 'repair_required', completed_at = ?2,
                repair_reason = ?3, retry_policy = ?4,
                terminal_finding_code = ?5, terminal_finding_summary = ?6
          WHERE verification_id = ?1
            AND status IN ('awaiting_probe', 'awaiting_observation')",
        params![
            verification_id,
            completed_at,
            reason.as_str(),
            retry_policy.as_str(),
            finding_code,
            finding_summary,
        ],
    )?;
    Ok(())
}

pub(super) fn increment_status_read_count(
    conn: &Connection,
    verification_id: &str,
) -> StoreResult<()> {
    conn.execute(
        "UPDATE guard_integration_verification_runs
            SET status_read_count = status_read_count + 1
          WHERE verification_id = ?1
            AND status = 'awaiting_observation'
            AND status_read_count < allowed_status_reads",
        [verification_id],
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

pub(super) fn run_for_coordinate(
    conn: &Connection,
    coordinate: &VerificationCurrentCoordinate,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    let caller = coordinate.caller();
    conn.query_row(
        &format!(
            "{RUN_SELECT}
              WHERE connection_internal_id = ?1
                AND project_id = ?2
                AND runtime_session_id = ?3
                AND host_session_id = ?4
                AND host_turn_id = ?5
                AND integration_revision = ?6
                AND guard_installation_id = ?7
                AND host_contract_profile = ?8
                AND hook_definition_digest = ?9
                AND policy_digest = ?10
              LIMIT 1"
        ),
        params![
            caller.connection_internal_id(),
            coordinate.semantic().project_id.as_str(),
            caller.runtime_session_id(),
            caller.host_session_id(),
            caller.host_turn_id(),
            coordinate.integration_revision(),
            coordinate.guard_installation_id(),
            coordinate.host_contract_profile().as_str(),
            coordinate.hook_definition_digest(),
            coordinate.policy_digest(),
        ],
        run_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn latest_run_for_project(
    conn: &Connection,
    connection_internal_id: &str,
    project_internal_id: &str,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    conn.query_row(
        &format!(
            "{RUN_SELECT}
              WHERE connection_internal_id = ?1
                AND project_internal_id = ?2
              ORDER BY created_at DESC, verification_id DESC
              LIMIT 1"
        ),
        params![connection_internal_id, project_internal_id],
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
                AND policy_digest = ?6
                AND status IN ('awaiting_probe', 'awaiting_observation')
              ORDER BY created_at DESC, verification_id DESC
              LIMIT 1"
        ),
        params![
            lookup.connection_internal_id,
            lookup.host_session_id,
            lookup.host_turn_id,
            lookup.guard_installation_id,
            lookup.integration_revision,
            lookup.policy_digest,
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
                AND policy_digest = ?5
                AND status IN ('awaiting_probe', 'awaiting_observation')
              ORDER BY created_at DESC, verification_id DESC
              LIMIT 1"
        ),
        params![
            lookup.connection_internal_id,
            lookup.project_internal_id,
            lookup.guard_installation_id,
            lookup.integration_revision,
            lookup.policy_digest,
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

pub(super) fn latest_run_for_membership(
    conn: &Connection,
    connection_internal_id: &str,
    project_internal_id: &str,
    integration_revision: &str,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    conn.query_row(
        &format!(
            "{RUN_SELECT}
              WHERE connection_internal_id = ?1
                AND project_internal_id = ?2
                AND integration_revision = ?3
              ORDER BY created_at DESC, verification_id DESC
              LIMIT 1"
        ),
        [
            connection_internal_id,
            project_internal_id,
            integration_revision,
        ],
        run_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn latest_completed_run_for_connection(
    conn: &Connection,
    connection_internal_id: &str,
    integration_revision: &str,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    conn.query_row(
        &format!(
            "{RUN_SELECT}
              WHERE connection_internal_id = ?1
                AND integration_revision = ?2
                AND status = 'complete'
              ORDER BY completed_at DESC, created_at DESC, verification_id DESC
              LIMIT 1"
        ),
        [connection_internal_id, integration_revision],
        run_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn latest_completed_run_for_membership(
    conn: &Connection,
    connection_internal_id: &str,
    project_internal_id: &str,
    integration_revision: &str,
) -> StoreResult<Option<GuardIntegrationVerificationRunRecord>> {
    conn.query_row(
        &format!(
            "{RUN_SELECT}
              WHERE connection_internal_id = ?1
                AND project_internal_id = ?2
                AND integration_revision = ?3
                AND status = 'complete'
              ORDER BY completed_at DESC, created_at DESC, verification_id DESC
              LIMIT 1"
        ),
        [
            connection_internal_id,
            project_internal_id,
            integration_revision,
        ],
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
        project_id: row.get(3)?,
        runtime_session_id: row.get(4)?,
        host_session_id: row.get(5)?,
        host_turn_id: row.get(6)?,
        integration_revision: row.get(7)?,
        guard_installation_id: row.get(8)?,
        host_contract_profile: row.get(9)?,
        hook_definition_digest: row.get(10)?,
        policy_digest: row.get(11)?,
        expected_probe_tool: row.get(12)?,
        expected_host_callable_name: row.get(13)?,
        observation_policy_kind: row.get(14)?,
        observation_deadline_at: row.get(15)?,
        allowed_status_reads: row.get(16)?,
        status_read_count: row.get(17)?,
        created_at: row.get(18)?,
        cleanup_after: row.get(19)?,
        status: row.get(20)?,
        probe_acknowledged_at: row.get(21)?,
        completed_at: row.get(22)?,
        matched_prompt_event_id: row.get(23)?,
        matched_pre_tool_event_id: row.get(24)?,
        matched_post_tool_event_id: row.get(25)?,
        repair_reason: row.get(26)?,
        retry_policy: row.get(27)?,
        terminal_finding_code: row.get(28)?,
        terminal_finding_summary: row.get(29)?,
    })
}

pub(super) fn parse_timestamp(field: &str, value: &str) -> StoreResult<UtcTimestamp> {
    UtcTimestamp::parse(value).map_err(|_| StoreError::InvalidInput {
        detail: format!("{field} must be an RFC 3339 timestamp"),
    })
}

pub(super) fn parse_status(value: &str) -> StoreResult<GuardIntegrationVerificationStatus> {
    match value {
        "awaiting_probe" => Ok(GuardIntegrationVerificationStatus::AwaitingProbe),
        "awaiting_observation" => Ok(GuardIntegrationVerificationStatus::AwaitingObservation),
        "complete" => Ok(GuardIntegrationVerificationStatus::Complete),
        "repair_required" => Ok(GuardIntegrationVerificationStatus::RepairRequired),
        _ => Err(StoreError::CorruptStoredValue {
            database_kind: "registry",
            field: "guard_integration_verification_runs.status",
        }),
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use volicord_types::integration_verification::GuardIntegrationVerificationStatus;

    use super::*;

    const HASH: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn every_status_spelling_decodes() {
        for (stored, expected) in [
            (
                "awaiting_probe",
                GuardIntegrationVerificationStatus::AwaitingProbe,
            ),
            (
                "awaiting_observation",
                GuardIntegrationVerificationStatus::AwaitingObservation,
            ),
            ("complete", GuardIntegrationVerificationStatus::Complete),
            (
                "repair_required",
                GuardIntegrationVerificationStatus::RepairRequired,
            ),
        ] {
            assert_eq!(parse_status(stored).expect("status"), expected);
        }
        assert!(matches!(
            parse_status("expired"),
            Err(StoreError::CorruptStoredValue { .. })
        ));
    }

    #[test]
    fn malformed_timestamp_is_rejected() {
        assert!(matches!(
            parse_timestamp("cleanup_after", "not-a-timestamp"),
            Err(StoreError::InvalidInput { .. })
        ));
    }

    #[test]
    fn required_columns_and_optional_fields_decode_from_sql_row() {
        let conn = Connection::open_in_memory().expect("database");
        let sql = format!(
            "SELECT
                'guard_verification_row', 'connection', 'project-internal', 'project',
                'runtime', 'session', 'turn', '{HASH}', 'installation',
                'codex-command-hooks', '{HASH}', '{HASH}',
                'volicord.guard_probe', 'mcp__volicord__volicord_guard_probe',
                'synchronous', NULL, 1, 1,
                '2026-07-23T00:00:00Z',
                '2026-07-23T00:05:00Z', 'repair_required', NULL,
                '2026-07-23T00:01:00Z', 'prompt', NULL, NULL,
                'hook_event_not_observed', 'host_reload_required',
                'hook_event_not_observed', ?1"
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
                NULL, 'connection', 'project-internal', 'project', 'runtime',
                'session', 'turn', '{HASH}', 'installation',
                'codex-command-hooks', '{HASH}', '{HASH}',
                'volicord.guard_probe', 'mcp__volicord__volicord_guard_probe',
                'synchronous', NULL, 1, 0,
                '2026-07-23T00:00:00Z',
                '2026-07-23T00:05:00Z', 'awaiting_probe',
                NULL, NULL, 'prompt', NULL, NULL, NULL, NULL, NULL, NULL"
        );
        assert!(conn.query_row(&missing, [], run_from_row).is_err());
    }
}
