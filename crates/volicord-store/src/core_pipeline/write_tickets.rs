use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::ids::TaskId;

use super::{facade::CoreProjectStore, validation::nonnegative_i64_to_u64};
use crate::{StoreError, StoreResult};

const WRITE_TICKET_RECORD_COLUMNS: &str = "
    project_id, write_ticket_id, task_id, change_unit_id,
    basis_state_version, status, validity_basis_json,
    allowed_path_prefixes_json, denied_path_prefixes_json,
    attempt_scope_json, idle_expires_at, invalidation_reason, created_at,
    consumed_by_run_id, consumed_at";

/// Stored write ticket facts needed by status and stale-marking responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketRecord {
    pub project_id: String,
    pub write_ticket_id: String,
    pub task_id: String,
    pub change_unit_id: String,
    pub basis_state_version: u64,
    pub status: String,
    pub validity_basis_json: String,
    pub allowed_path_prefixes_json: String,
    pub denied_path_prefixes_json: String,
    pub attempt_scope_json: String,
    pub idle_expires_at: Option<String>,
    pub invalidation_reason: Option<String>,
    pub created_at: String,
    pub consumed_by_run_id: Option<String>,
    pub consumed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WriteTicketRecordRaw {
    project_id: String,
    write_ticket_id: String,
    task_id: String,
    change_unit_id: Option<String>,
    basis_state_version: u64,
    status: String,
    validity_basis_json: String,
    allowed_path_prefixes_json: String,
    denied_path_prefixes_json: String,
    attempt_scope_json: String,
    idle_expires_at: Option<String>,
    invalidation_reason: Option<String>,
    created_at: String,
    consumed_by_run_id: Option<String>,
    consumed_at: Option<String>,
}

impl CoreProjectStore<'_> {
    /// Lists active Write Tickets for a Task.
    pub fn active_write_tickets(&self, task_id: &TaskId) -> StoreResult<Vec<WriteTicketRecord>> {
        active_write_tickets(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Lists Write Tickets for a Task without mutating effective status.
    pub fn write_tickets_for_task(&self, task_id: &TaskId) -> StoreResult<Vec<WriteTicketRecord>> {
        write_tickets_for_task(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Reads one Write Ticket row by exact project-local identity.
    pub fn write_ticket_record(
        &self,
        write_ticket_id: &str,
    ) -> StoreResult<Option<WriteTicketRecord>> {
        write_ticket_record(&self.conn, &self.project.project_id, write_ticket_id)
    }
}

fn active_write_tickets(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<WriteTicketRecord>> {
    let sql = format!(
        "SELECT {WRITE_TICKET_RECORD_COLUMNS}
           FROM write_tickets
          WHERE project_id = ?1
            AND task_id = ?2
            AND status = 'active'
          ORDER BY write_ticket_id"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(
        params![project_id, task_id],
        write_ticket_record_raw_from_row,
    )?;
    let mut records = Vec::new();
    for row in rows {
        records.push(decode_write_ticket_record(row?)?);
    }
    Ok(records)
}

fn write_tickets_for_task(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<WriteTicketRecord>> {
    let sql = format!(
        "SELECT {WRITE_TICKET_RECORD_COLUMNS}
           FROM write_tickets
          WHERE project_id = ?1
            AND task_id = ?2
          ORDER BY basis_state_version DESC, write_ticket_id"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(
        params![project_id, task_id],
        write_ticket_record_raw_from_row,
    )?;
    let mut records = Vec::new();
    for row in rows {
        records.push(decode_write_ticket_record(row?)?);
    }
    Ok(records)
}

fn write_ticket_record(
    conn: &Connection,
    project_id: &str,
    write_ticket_id: &str,
) -> StoreResult<Option<WriteTicketRecord>> {
    let sql = format!(
        "SELECT {WRITE_TICKET_RECORD_COLUMNS}
           FROM write_tickets
          WHERE project_id = ?1
            AND write_ticket_id = ?2"
    );
    conn.query_row(
        &sql,
        params![project_id, write_ticket_id],
        write_ticket_record_raw_from_row,
    )
    .optional()
    .map_err(StoreError::from)?
    .map(decode_write_ticket_record)
    .transpose()
}

fn write_ticket_record_raw_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<WriteTicketRecordRaw> {
    let basis_state_version = row.get::<_, i64>(4)?;
    Ok(WriteTicketRecordRaw {
        project_id: row.get(0)?,
        write_ticket_id: row.get(1)?,
        task_id: row.get(2)?,
        change_unit_id: row.get(3)?,
        basis_state_version: nonnegative_i64_to_u64(
            "write_tickets.basis_state_version",
            basis_state_version,
        )?,
        status: row.get(5)?,
        validity_basis_json: row.get(6)?,
        allowed_path_prefixes_json: row.get(7)?,
        denied_path_prefixes_json: row.get(8)?,
        attempt_scope_json: row.get(9)?,
        idle_expires_at: row.get(10)?,
        invalidation_reason: row.get(11)?,
        created_at: row.get(12)?,
        consumed_by_run_id: row.get(13)?,
        consumed_at: row.get(14)?,
    })
}

fn decode_write_ticket_record(raw: WriteTicketRecordRaw) -> StoreResult<WriteTicketRecord> {
    let change_unit_id = raw
        .change_unit_id
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            StoreError::corrupt_owner_state_value(
                "write_tickets",
                raw.write_ticket_id.clone(),
                "change_unit_id",
            )
        })?;
    Ok(WriteTicketRecord {
        project_id: raw.project_id,
        write_ticket_id: raw.write_ticket_id,
        task_id: raw.task_id,
        change_unit_id,
        basis_state_version: raw.basis_state_version,
        status: raw.status,
        validity_basis_json: raw.validity_basis_json,
        allowed_path_prefixes_json: raw.allowed_path_prefixes_json,
        denied_path_prefixes_json: raw.denied_path_prefixes_json,
        attempt_scope_json: raw.attempt_scope_json,
        idle_expires_at: raw.idle_expires_at,
        invalidation_reason: raw.invalidation_reason,
        created_at: raw.created_at,
        consumed_by_run_id: raw.consumed_by_run_id,
        consumed_at: raw.consumed_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_ticket_decoder_requires_a_change_unit_owner() {
        let error = decode_write_ticket_record(WriteTicketRecordRaw {
            project_id: "project".to_owned(),
            write_ticket_id: "ticket".to_owned(),
            task_id: "task".to_owned(),
            change_unit_id: None,
            basis_state_version: 1,
            status: "active".to_owned(),
            validity_basis_json: "{}".to_owned(),
            allowed_path_prefixes_json: "[]".to_owned(),
            denied_path_prefixes_json: "[]".to_owned(),
            attempt_scope_json: "{}".to_owned(),
            idle_expires_at: None,
            invalidation_reason: None,
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            consumed_by_run_id: None,
            consumed_at: None,
        })
        .expect_err("missing Change Unit owner must fail closed");

        assert!(matches!(
            error,
            StoreError::CorruptOwnerStateValue {
                table: "write_tickets",
                logical_column: "change_unit_id",
                ..
            }
        ));
    }
}
