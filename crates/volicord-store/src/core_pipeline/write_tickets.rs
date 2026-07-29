use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::{
    ids::TaskId,
    product_path::ProductRelativePath,
    schema::{JsonObject, WriteTicketAttemptScope, WriteTicketValidityBasis},
    values::{ActorSource, UtcTimestamp, WriteTicketInvalidationReason, WriteTicketStatus},
};

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
    pub validity_basis: WriteTicketValidityBasis,
    pub allowed_path_prefixes: Vec<ProductRelativePath>,
    pub denied_path_prefixes: Vec<ProductRelativePath>,
    pub attempt_scope: WriteTicketAttemptScope,
    pub created_by_actor_source: ActorSource,
    pub created_by_user_action_resolution_id: Option<String>,
    pub idle_expires_at: Option<UtcTimestamp>,
    pub created_at: UtcTimestamp,
    pub metadata: JsonObject,
}

/// Storage input for invalidating every active write ticket for one Task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketInvalidation {
    pub task_id: String,
    pub invalidation_reason: WriteTicketInvalidationReason,
}

/// Storage input for invalidating one specifically identified active write ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteTicketByIdInvalidation {
    pub write_ticket_id: String,
    pub invalidation_reason: WriteTicketInvalidationReason,
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
    pub status: WriteTicketStatus,
    pub validity_basis: WriteTicketValidityBasis,
    pub allowed_path_prefixes: Vec<ProductRelativePath>,
    pub denied_path_prefixes: Vec<ProductRelativePath>,
    pub attempt_scope: WriteTicketAttemptScope,
    pub idle_expires_at: Option<UtcTimestamp>,
    pub invalidation_reason: Option<WriteTicketInvalidationReason>,
    pub created_at: UtcTimestamp,
    pub consumed_by_run_id: Option<String>,
    pub consumed_at: Option<UtcTimestamp>,
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
    let record_ref = raw.write_ticket_id.clone();
    let corrupt_value =
        |column| StoreError::corrupt_owner_state_value("write_tickets", record_ref.clone(), column);
    let corrupt_json =
        |column| StoreError::corrupt_owner_state_json("write_tickets", record_ref.clone(), column);
    let change_unit_id = raw
        .change_unit_id
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| corrupt_value("change_unit_id"))?;
    let status = decode_owner_closed_value("write_tickets", &record_ref, "status", &raw.status)?;
    let validity_basis: WriteTicketValidityBasis = decode_owner_json_text(
        "write_tickets",
        &record_ref,
        "validity_basis_json",
        &raw.validity_basis_json,
    )?;
    let allowed_path_prefixes: Vec<ProductRelativePath> = decode_owner_json_text(
        "write_tickets",
        &record_ref,
        "allowed_path_prefixes_json",
        &raw.allowed_path_prefixes_json,
    )?;
    let denied_path_prefixes: Vec<ProductRelativePath> = decode_owner_json_text(
        "write_tickets",
        &record_ref,
        "denied_path_prefixes_json",
        &raw.denied_path_prefixes_json,
    )?;
    let attempt_scope: WriteTicketAttemptScope = decode_owner_json_text(
        "write_tickets",
        &record_ref,
        "attempt_scope_json",
        &raw.attempt_scope_json,
    )?;
    let idle_expires_at = raw
        .idle_expires_at
        .as_deref()
        .map(|value| UtcTimestamp::parse(value).map_err(|_| corrupt_value("idle_expires_at")))
        .transpose()?;
    let invalidation_reason = raw
        .invalidation_reason
        .as_deref()
        .map(|value| {
            decode_owner_closed_value("write_tickets", &record_ref, "invalidation_reason", value)
        })
        .transpose()?;
    let created_at =
        UtcTimestamp::parse(&raw.created_at).map_err(|_| corrupt_value("created_at"))?;
    let consumed_at = raw
        .consumed_at
        .as_deref()
        .map(|value| UtcTimestamp::parse(value).map_err(|_| corrupt_value("consumed_at")))
        .transpose()?;
    created_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| corrupt_value("created_at"))?;
    if let Some(timestamp) = idle_expires_at.as_ref() {
        timestamp
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| corrupt_value("idle_expires_at"))?;
    }
    if let Some(timestamp) = consumed_at.as_ref() {
        timestamp
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| corrupt_value("consumed_at"))?;
    }
    if validity_basis.task_id.as_str() != raw.task_id
        || validity_basis.change_unit_id.as_str() != change_unit_id
        || attempt_scope.task_id.as_str() != raw.task_id
        || attempt_scope.change_unit_id.as_str() != change_unit_id
    {
        return Err(corrupt_json("validity_basis_json"));
    }
    let intended_paths = attempt_scope
        .intended_paths
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    let allowed_paths = allowed_path_prefixes
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    let denied_paths = denied_path_prefixes
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    if raw.basis_state_version == 0
        || validity_basis.scope_revision == 0
        || attempt_scope.intended_operation.trim().is_empty()
        || validity_basis.write_authority_fingerprint.len() != 71
        || !validity_basis
            .write_authority_fingerprint
            .starts_with("sha256:")
        || !validity_basis.write_authority_fingerprint[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || validity_basis
            .workspace_context_sha256
            .as_ref()
            .is_some_and(|digest| {
                digest.len() != 64
                    || !digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        || idle_expires_at
            .as_ref()
            .is_some_and(|expires_at| expires_at <= &created_at)
        || intended_paths.len() != attempt_scope.intended_paths.len()
        || allowed_paths.len() != allowed_path_prefixes.len()
        || denied_paths.len() != denied_path_prefixes.len()
        || !allowed_paths.is_disjoint(&denied_paths)
        || !allowed_paths.is_subset(&intended_paths)
        || !denied_paths.is_subset(&intended_paths)
    {
        return Err(corrupt_json("allowed_path_prefixes_json"));
    }
    let status_columns_are_valid = match status {
        WriteTicketStatus::Active => {
            invalidation_reason.is_none()
                && raw.consumed_by_run_id.is_none()
                && consumed_at.is_none()
        }
        WriteTicketStatus::Consumed => {
            invalidation_reason.is_none()
                && raw.consumed_by_run_id.is_some()
                && consumed_at.is_some()
        }
        WriteTicketStatus::Invalidated | WriteTicketStatus::Revoked => {
            invalidation_reason.is_some()
                && raw.consumed_by_run_id.is_none()
                && consumed_at.is_none()
        }
    };
    if !status_columns_are_valid {
        return Err(corrupt_value("status"));
    }
    Ok(WriteTicketRecord {
        project_id: raw.project_id,
        write_ticket_id: raw.write_ticket_id,
        task_id: raw.task_id,
        change_unit_id,
        basis_state_version: raw.basis_state_version,
        status,
        validity_basis,
        allowed_path_prefixes,
        denied_path_prefixes,
        attempt_scope,
        idle_expires_at,
        invalidation_reason,
        created_at,
        consumed_by_run_id: raw.consumed_by_run_id,
        consumed_at,
    })
}

impl MutationContext<'_> {
    fn mark_active_write_tickets_stale(&mut self, task_id: &str) -> StoreResult<()> {
        self.invalidate_active_write_tickets(&WriteTicketInvalidation {
            task_id: task_id.to_owned(),
            invalidation_reason: WriteTicketInvalidationReason::ScopeRevisionChanged,
        })
    }

    fn invalidate_active_write_tickets(
        &mut self,
        input: &WriteTicketInvalidation,
    ) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        let invalidation_reason = encode_closed_value(
            "write_tickets.invalidation_reason",
            &input.invalidation_reason,
        )?;
        self.tx.execute(
            "UPDATE write_tickets
                SET status = 'invalidated',
                    invalidation_reason = ?3
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'",
            params![self.project_id, input.task_id, invalidation_reason],
        )?;
        Ok(())
    }

    fn invalidate_write_ticket(&mut self, input: &WriteTicketByIdInvalidation) -> StoreResult<()> {
        validate_identifier("write_ticket_id", &input.write_ticket_id)?;
        let invalidation_reason = encode_closed_value(
            "write_tickets.invalidation_reason",
            &input.invalidation_reason,
        )?;
        let changed = self.tx.execute(
            "UPDATE write_tickets
                SET status = 'invalidated',
                    invalidation_reason = ?3
              WHERE project_id = ?1
                AND write_ticket_id = ?2
                AND status = 'active'",
            params![self.project_id, input.write_ticket_id, invalidation_reason],
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
        if input.validity_basis.task_id.as_str() != input.task_id
            || input.validity_basis.change_unit_id.as_str() != input.change_unit_id
            || input.attempt_scope.task_id.as_str() != input.task_id
            || input.attempt_scope.change_unit_id.as_str() != input.change_unit_id
        {
            return Err(StoreError::InvalidInput {
                detail: "write-ticket typed identities must match their row owner".to_owned(),
            });
        }
        if committed_state_version == 0
            || input.validity_basis.scope_revision == 0
            || input.attempt_scope.intended_operation.trim().is_empty()
            || input.validity_basis.write_authority_fingerprint.len() != 71
            || !input
                .validity_basis
                .write_authority_fingerprint
                .starts_with("sha256:")
            || !input.validity_basis.write_authority_fingerprint[7..]
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || input
                .validity_basis
                .workspace_context_sha256
                .as_ref()
                .is_some_and(|digest| {
                    digest.len() != 64
                        || !digest
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                })
            || input
                .idle_expires_at
                .as_ref()
                .is_some_and(|expires_at| expires_at <= &input.created_at)
        {
            return Err(StoreError::InvalidInput {
                detail: "write-ticket typed authority facts are internally inconsistent".to_owned(),
            });
        }
        input
            .created_at
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| StoreError::InvalidInput {
                detail: "write-ticket created_at is outside the supported range".to_owned(),
            })?;
        let validity_basis_json = volicord_types::canonical::canonical_json_string(
            &input.validity_basis,
        )
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("write-ticket validity basis cannot be serialized: {error}"),
        })?;
        let allowed_path_prefixes_json =
            volicord_types::canonical::canonical_json_string(&input.allowed_path_prefixes)
                .map_err(|error| StoreError::InvalidInput {
                    detail: format!("write-ticket allowed paths cannot be serialized: {error}"),
                })?;
        let denied_path_prefixes_json = volicord_types::canonical::canonical_json_string(
            &input.denied_path_prefixes,
        )
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("write-ticket denied paths cannot be serialized: {error}"),
        })?;
        let attempt_scope_json = volicord_types::canonical::canonical_json_string(
            &input.attempt_scope,
        )
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("write-ticket attempt scope cannot be serialized: {error}"),
        })?;
        let created_by_actor_source = input.created_by_actor_source.to_canonical_string();
        if let Some(resolution_id) = &input.created_by_user_action_resolution_id {
            validate_identifier("created_by_user_action_resolution_id", resolution_id)?;
        }
        let idle_expires_at = input.idle_expires_at.as_ref().map(ToString::to_string);
        let created_at = input.created_at.to_string();
        let metadata_json = volicord_types::canonical::canonical_json_string(&input.metadata)
            .map_err(|error| StoreError::InvalidInput {
                detail: format!("write-ticket metadata cannot be serialized: {error}"),
            })?;
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
                validity_basis_json,
                allowed_path_prefixes_json,
                denied_path_prefixes_json,
                attempt_scope_json,
                created_by_actor_source,
                input.created_by_user_action_resolution_id,
                idle_expires_at,
                created_at,
                metadata_json
            ],
        )?;
        Ok(())
    }

    fn consume_write_ticket(&mut self, input: &WriteTicketConsumption) -> StoreResult<()> {
        validate_identifier("write_ticket_id", &input.write_ticket_id)?;
        validate_identifier("run_id", &input.run_id)?;
        let sql = format!(
            "SELECT {WRITE_TICKET_RECORD_COLUMNS}
               FROM write_tickets
              WHERE project_id = ?1
                AND write_ticket_id = ?2"
        );
        let raw = self
            .tx
            .query_row(
                &sql,
                params![self.project_id, input.write_ticket_id],
                write_ticket_record_raw_from_row,
            )
            .optional()?
            .ok_or_else(|| StoreError::NotFound {
                entity: "write_ticket",
                id: input.write_ticket_id.clone(),
            })?;
        let ticket = decode_write_ticket_record(raw)?;
        let policy =
            crate::workflow_records::project_workflow_policy_from_conn(self.tx, self.project_id)?;
        let current_write_authority_fingerprint =
            project_write_authority_fingerprint(policy.as_ref().map(|record| &record.policy))?;
        if ticket.status != WriteTicketStatus::Active
            || ticket.basis_state_version != input.expected_basis_state_version
            || ticket.validity_basis.write_authority_fingerprint
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
mod behavior_tests;

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
    fn write_ticket_mutation_validates_its_identifier_before_sql() {
        let error = with_empty_mutation_context(|context| {
            WriteTicketMutation::InvalidateById(WriteTicketByIdInvalidation {
                write_ticket_id: String::new(),
                invalidation_reason: WriteTicketInvalidationReason::ExplicitRevoke,
            })
            .apply(context, 1)
            .expect_err("empty write-ticket identity must fail before SQL")
        });

        assert!(matches!(error, StoreError::InvalidInput { .. }));
    }
}
