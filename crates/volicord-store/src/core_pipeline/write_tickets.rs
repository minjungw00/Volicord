use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::ids::TaskId;
use volicord_types::schema::WriteTicketValidityBasis;

use super::{facade::CoreProjectStore, mutations::MutationContext, validation::*};
use crate::{workflow_records::project_write_authority_fingerprint, StoreError, StoreResult};

const WRITE_TICKET_RECORD_COLUMNS: &str = "
    project_id, write_ticket_id, task_id, change_unit_id,
    basis_state_version, status, validity_basis_json,
    allowed_path_prefixes_json, denied_path_prefixes_json,
    attempt_scope_json, idle_expires_at, invalidation_reason, created_at,
    consumed_by_run_id, consumed_at";

/// Write-ticket mutation applied inside one Core commit transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteTicketMutation {
    MarkActiveStale { task_id: String },
    InvalidateActive(WriteTicketInvalidation),
    InvalidateById(WriteTicketByIdInvalidation),
    Insert(Box<WriteTicketInsert>),
    Consume(WriteTicketConsumption),
}

/// Storage input for inserting one open write ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketInsert {
    pub write_ticket_id: String,
    pub task_id: String,
    pub change_unit_id: String,
    pub validity_basis_json: String,
    pub allowed_path_prefixes_json: String,
    pub denied_path_prefixes_json: String,
    pub attempt_scope_json: String,
    pub created_by_actor_source: String,
    pub created_by_user_action_resolution_id: Option<String>,
    pub idle_expires_at: Option<String>,
    pub created_at: String,
    pub metadata_json: String,
}

/// Storage input for invalidating every active write ticket for one Task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketInvalidation {
    pub task_id: String,
    pub invalidation_reason: String,
}

/// Storage input for invalidating one specifically identified active write ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketByIdInvalidation {
    pub write_ticket_id: String,
    pub invalidation_reason: String,
}

/// Storage input for closing one open write ticket through a compatible Run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketConsumption {
    pub write_ticket_id: String,
    pub run_id: String,
    pub expected_basis_state_version: u64,
    pub expected_write_authority_fingerprint: String,
}

impl WriteTicketMutation {
    /// Boxes the largest Write Ticket mutation payload.
    pub fn insert(input: WriteTicketInsert) -> Self {
        Self::Insert(Box::new(input))
    }

    pub(super) fn apply(
        &self,
        context: &mut MutationContext<'_>,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        match self {
            Self::MarkActiveStale { task_id } => context.mark_active_write_tickets_stale(task_id),
            Self::InvalidateActive(input) => context.invalidate_active_write_tickets(input),
            Self::InvalidateById(input) => context.invalidate_write_ticket(input),
            Self::Insert(input) => context.insert_write_ticket(input, committed_state_version),
            Self::Consume(input) => context.consume_write_ticket(input),
        }
    }
}

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

fn validate_write_ticket_invalidation_reason(value: &str) -> StoreResult<()> {
    if matches!(
        value,
        "scope_revision_changed"
            | "change_unit_changed"
            | "baseline_changed"
            | "workspace_changed"
            | "approval_basis_changed"
            | "idle_timeout"
            | "task_closed"
            | "explicit_revoke"
    ) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "write-ticket invalidation reason is not supported".to_owned(),
        })
    }
}

impl MutationContext<'_> {
    fn mark_active_write_tickets_stale(&mut self, task_id: &str) -> StoreResult<()> {
        self.invalidate_active_write_tickets(&WriteTicketInvalidation {
            task_id: task_id.to_owned(),
            invalidation_reason: "scope_revision_changed".to_owned(),
        })
    }

    fn invalidate_active_write_tickets(
        &mut self,
        input: &WriteTicketInvalidation,
    ) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        validate_write_ticket_invalidation_reason(&input.invalidation_reason)?;
        self.tx.execute(
            "UPDATE write_tickets
                SET status = 'invalidated',
                    invalidation_reason = ?3
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'",
            params![self.project_id, input.task_id, input.invalidation_reason],
        )?;
        Ok(())
    }

    fn invalidate_write_ticket(&mut self, input: &WriteTicketByIdInvalidation) -> StoreResult<()> {
        validate_identifier("write_ticket_id", &input.write_ticket_id)?;
        validate_write_ticket_invalidation_reason(&input.invalidation_reason)?;
        let changed = self.tx.execute(
            "UPDATE write_tickets
                SET status = 'invalidated',
                    invalidation_reason = ?3
              WHERE project_id = ?1
                AND write_ticket_id = ?2
                AND status = 'active'",
            params![
                self.project_id,
                input.write_ticket_id,
                input.invalidation_reason
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "identified active write ticket invalidation changed no rows".to_owned(),
            })
        }
    }

    fn insert_write_ticket(
        &mut self,
        input: &WriteTicketInsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        validate_identifier("write_ticket_id", &input.write_ticket_id)?;
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("change_unit_id", &input.change_unit_id)?;
        validate_json_text(
            "write_tickets.validity_basis_json",
            &input.validity_basis_json,
        )?;
        validate_json_text(
            "write_tickets.allowed_path_prefixes_json",
            &input.allowed_path_prefixes_json,
        )?;
        validate_json_text(
            "write_tickets.denied_path_prefixes_json",
            &input.denied_path_prefixes_json,
        )?;
        validate_json_text(
            "write_tickets.attempt_scope_json",
            &input.attempt_scope_json,
        )?;
        validate_identifier("created_by_actor_source", &input.created_by_actor_source)?;
        if let Some(resolution_id) = &input.created_by_user_action_resolution_id {
            validate_identifier("created_by_user_action_resolution_id", resolution_id)?;
        }
        if let Some(idle_expires_at) = &input.idle_expires_at {
            validate_timestamp("write_tickets.idle_expires_at", idle_expires_at)?;
        }
        validate_timestamp("write_tickets.created_at", &input.created_at)?;
        validate_json_text("write_tickets.metadata_json", &input.metadata_json)?;
        let basis_state_version = u64_to_i64("basis_state_version", committed_state_version)?;

        self.tx.execute(
            "INSERT INTO write_tickets (
                project_id,
                write_ticket_id,
                task_id,
                change_unit_id,
                basis_state_version,
                status,
                validity_basis_json,
                allowed_path_prefixes_json,
                denied_path_prefixes_json,
                attempt_scope_json,
                created_by_actor_source,
                created_by_user_action_resolution_id,
                idle_expires_at,
                invalidation_reason,
                consumed_by_run_id,
                consumed_at,
                revoked_at,
                created_at,
                metadata_json
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                'active',
                ?6,
                ?7,
                ?8,
                ?9,
                ?10,
                ?11,
                ?12,
                NULL,
                NULL,
                NULL,
                NULL,
                ?13,
                ?14
            )",
            params![
                self.project_id,
                input.write_ticket_id,
                input.task_id,
                input.change_unit_id,
                basis_state_version,
                input.validity_basis_json,
                input.allowed_path_prefixes_json,
                input.denied_path_prefixes_json,
                input.attempt_scope_json,
                input.created_by_actor_source,
                input.created_by_user_action_resolution_id,
                input.idle_expires_at,
                input.created_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn consume_write_ticket(&mut self, input: &WriteTicketConsumption) -> StoreResult<()> {
        validate_identifier("write_ticket_id", &input.write_ticket_id)?;
        validate_identifier("run_id", &input.run_id)?;
        let (basis_state_version, status, validity_basis_json) = self
            .tx
            .query_row(
                "SELECT basis_state_version, status, validity_basis_json
                   FROM write_tickets
                  WHERE project_id = ?1
                    AND write_ticket_id = ?2",
                params![self.project_id, input.write_ticket_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| StoreError::NotFound {
                entity: "write_ticket",
                id: input.write_ticket_id.clone(),
            })?;
        let basis_state_version =
            nonnegative_i64_to_u64("write_tickets.basis_state_version", basis_state_version)?;
        let validity_basis: WriteTicketValidityBasis = serde_json::from_str(&validity_basis_json)
            .map_err(|_| {
            StoreError::corrupt_owner_state_json(
                "write_tickets",
                &input.write_ticket_id,
                "validity_basis_json",
            )
        })?;
        let policy_json = self
            .tx
            .query_row(
                "SELECT policy_json
                   FROM project_workflow_policies
                  WHERE project_id = ?1",
                [self.project_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let current_write_authority_fingerprint =
            project_write_authority_fingerprint(policy_json.as_deref())?;
        if status != "active"
            || basis_state_version != input.expected_basis_state_version
            || validity_basis.write_authority_fingerprint
                != input.expected_write_authority_fingerprint
            || current_write_authority_fingerprint != input.expected_write_authority_fingerprint
        {
            return Err(StoreError::Conflict {
                entity: "write_ticket",
                id: input.write_ticket_id.clone(),
                detail: "write ticket authority changed before consumption".to_owned(),
            });
        }
        let expected_basis_state_version = u64_to_i64(
            "write_tickets.basis_state_version",
            input.expected_basis_state_version,
        )?;
        let changed = self.tx.execute(
            "UPDATE write_tickets
                SET status = 'consumed',
                    consumed_by_run_id = ?3,
                    consumed_at = ?4
              WHERE project_id = ?1
                AND write_ticket_id = ?2
                AND status = 'active'
                AND basis_state_version = ?5",
            params![
                self.project_id,
                input.write_ticket_id,
                input.run_id,
                self.committed_at,
                expected_basis_state_version,
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "active Write Ticket consumption changed no rows".to_owned(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_pipeline::mutations::with_empty_mutation_context;

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

    #[test]
    fn write_ticket_mutation_validates_its_reason_before_sql() {
        let error = with_empty_mutation_context(|context| {
            WriteTicketMutation::InvalidateById(WriteTicketByIdInvalidation {
                write_ticket_id: "ticket".to_owned(),
                invalidation_reason: "unsupported".to_owned(),
            })
            .apply(context, 1)
            .expect_err("unknown invalidation reason must fail before SQL")
        });

        assert!(matches!(error, StoreError::InvalidInput { .. }));
    }
}
