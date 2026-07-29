use rusqlite::{params, Connection};

use super::{
    facade::CoreProjectStore, validation::nonnegative_i64_to_u64, write_tickets::write_ticket_count,
};
use crate::{StoreError, StoreResult};

/// Storage counters used to verify no-effect request branches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageEffectCounts {
    pub state_version: u64,
    pub tasks: u64,
    pub acceptance_criteria: u64,
    pub evidence_claims: u64,
    pub change_units: u64,
    pub authority_events: u64,
    pub tool_invocations: u64,
    pub user_action_requests: u64,
    pub user_action_resolutions: u64,
    pub write_tickets: u64,
    pub runs: u64,
    pub evidence_capture_intents: u64,
    pub evidence_capture_receipts: u64,
    pub evidence_capture_source_claims: u64,
    pub artifact_staging: u64,
    pub artifacts: u64,
    pub artifact_links: u64,
    pub evidence_summaries: u64,
    pub evidence_observations: u64,
    pub evidence_producers: u64,
    pub blockers: u64,
    pub project_continuity_records: u64,
}

impl CoreProjectStore<'_> {
    /// Reads the current storage-effect counters for this project.
    pub fn effect_counts(&self) -> StoreResult<StorageEffectCounts> {
        let state = self.project_state()?;
        Ok(StorageEffectCounts {
            state_version: state.state_version,
            tasks: table_count(&self.conn, "tasks", &self.project.project_id)?,
            acceptance_criteria: table_count(
                &self.conn,
                "acceptance_criteria",
                &self.project.project_id,
            )?,
            evidence_claims: table_count(&self.conn, "evidence_claims", &self.project.project_id)?,
            change_units: table_count(&self.conn, "change_units", &self.project.project_id)?,
            authority_events: table_count(
                &self.conn,
                "authority_events",
                &self.project.project_id,
            )?,
            tool_invocations: table_count(
                &self.conn,
                "tool_invocations",
                &self.project.project_id,
            )?,
            user_action_requests: table_count(
                &self.conn,
                "user_action_requests",
                &self.project.project_id,
            )?,
            user_action_resolutions: table_count(
                &self.conn,
                "user_action_resolutions",
                &self.project.project_id,
            )?,
            write_tickets: write_ticket_count(&self.conn, &self.project.project_id)?,
            runs: table_count(&self.conn, "runs", &self.project.project_id)?,
            evidence_capture_intents: table_count(
                &self.conn,
                "evidence_capture_intents",
                &self.project.project_id,
            )?,
            evidence_capture_receipts: table_count(
                &self.conn,
                "evidence_capture_receipts",
                &self.project.project_id,
            )?,
            evidence_capture_source_claims: table_count(
                &self.conn,
                "evidence_capture_source_claims",
                &self.project.project_id,
            )?,
            artifact_staging: table_count(
                &self.conn,
                "artifact_staging",
                &self.project.project_id,
            )?,
            artifacts: table_count(&self.conn, "artifacts", &self.project.project_id)?,
            artifact_links: table_count(&self.conn, "artifact_links", &self.project.project_id)?,
            evidence_summaries: table_count(
                &self.conn,
                "evidence_summaries",
                &self.project.project_id,
            )?,
            evidence_observations: table_count(
                &self.conn,
                "evidence_observations",
                &self.project.project_id,
            )?,
            evidence_producers: table_count(
                &self.conn,
                "evidence_producers",
                &self.project.project_id,
            )?,
            blockers: table_count(&self.conn, "blockers", &self.project.project_id)?,
            project_continuity_records: table_count(
                &self.conn,
                "project_continuity_records",
                &self.project.project_id,
            )?,
        })
    }
}

fn table_count(conn: &Connection, table: &str, project_id: &str) -> StoreResult<u64> {
    let escaped_table = table.replace('"', "\"\"");
    let sql = format!("SELECT COUNT(*) FROM \"{escaped_table}\" WHERE project_id = ?1");
    let count: i64 = conn.query_row(&sql, params![project_id], |row| row.get(0))?;
    nonnegative_i64_to_u64("table count", count).map_err(StoreError::from)
}
