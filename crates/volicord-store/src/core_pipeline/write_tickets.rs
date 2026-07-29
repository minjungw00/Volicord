use std::collections::BTreeSet;

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use volicord_types::{
    ids::{ChangeUnitId, TaskId, UserActionResolutionId, WriteTicketId},
    product_path::{ProductRelativePath, WriteTicketPathScope, WriteTicketPathScopeError},
    schema::{JsonObject, WriteTicketAttemptScope, WriteTicketValidityBasis},
    values::{ActorSource, UtcTimestamp, WriteTicketInvalidationReason, WriteTicketStatus},
};

use super::{facade::CoreProjectStore, mutations::MutationContext, validation::*};
use crate::{
    workflow_records::project_write_authority_fingerprint, StoreError, StoreResult,
    WriteTicketInvariant,
};

const WRITE_TICKET_RECORD_COLUMNS: &str = "
    project_id, write_ticket_id, task_id, change_unit_id,
    basis_state_version, status, validity_basis_json,
    allowed_path_prefixes_json, denied_path_prefixes_json,
    attempt_scope_json, created_by_actor_source,
    created_by_user_action_resolution_id, idle_expires_at,
    invalidation_reason, consumed_by_run_id, consumed_at, revoked_at,
    created_at, metadata_json";

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
    pub write_ticket_id: WriteTicketId,
    pub task_id: TaskId,
    pub change_unit_id: ChangeUnitId,
    pub validity_basis: WriteTicketValidityBasis,
    pub path_scope: WriteTicketPathScope,
    pub attempt_scope: WriteTicketAttemptScope,
    pub created_by_actor_source: ActorSource,
    pub created_by_user_action_resolution_id: Option<UserActionResolutionId>,
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

/// Store-validated persisted Write Ticket.
///
/// The fields are private so external crates cannot construct a value that
/// bypasses Store-owned physical decoding and persisted-record validation.
///
/// ```
/// use volicord_store::core_pipeline::StoredWriteTicket;
///
/// fn inspect(ticket: &StoredWriteTicket) {
///     let _ = ticket.write_ticket_id();
///     let _ = ticket.validity_basis();
///     let _ = ticket.status();
/// }
/// ```
///
/// ```compile_fail,E0451
/// use volicord_store::core_pipeline::StoredWriteTicket;
///
/// let _ = StoredWriteTicket {
///     project_id: String::new(),
///     ..todo!()
/// };
/// ```
///
/// ```compile_fail,E0616
/// use volicord_store::core_pipeline::StoredWriteTicket;
///
/// fn inspect(ticket: &StoredWriteTicket) {
///     let _ = &ticket.validity_basis;
/// }
/// ```
///
/// ```compile_fail,E0451
/// use volicord_store::core_pipeline::StoredWriteTicket;
///
/// fn inspect(ticket: StoredWriteTicket) {
///     let StoredWriteTicket { status, .. } = ticket;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredWriteTicket {
    project_id: String,
    write_ticket_id: String,
    task_id: String,
    change_unit_id: String,
    basis_state_version: u64,
    status: WriteTicketStatus,
    validity_basis: WriteTicketValidityBasis,
    path_scope: WriteTicketPathScope,
    attempt_scope: WriteTicketAttemptScope,
    created_by_actor_source: ActorSource,
    created_by_user_action_resolution_id: Option<String>,
    idle_expires_at: Option<UtcTimestamp>,
    invalidation_reason: Option<WriteTicketInvalidationReason>,
    consumed_by_run_id: Option<String>,
    consumed_at: Option<UtcTimestamp>,
    revoked_at: Option<UtcTimestamp>,
    created_at: UtcTimestamp,
    metadata: JsonObject,
}

impl StoredWriteTicket {
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn write_ticket_id(&self) -> &str {
        &self.write_ticket_id
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn change_unit_id(&self) -> &str {
        &self.change_unit_id
    }

    pub fn basis_state_version(&self) -> u64 {
        self.basis_state_version
    }

    pub fn status(&self) -> WriteTicketStatus {
        self.status
    }

    pub fn validity_basis(&self) -> &WriteTicketValidityBasis {
        &self.validity_basis
    }

    pub fn path_scope(&self) -> &WriteTicketPathScope {
        &self.path_scope
    }

    pub fn attempt_scope(&self) -> &WriteTicketAttemptScope {
        &self.attempt_scope
    }

    pub fn created_by_actor_source(&self) -> &ActorSource {
        &self.created_by_actor_source
    }

    pub fn created_by_user_action_resolution_id(&self) -> Option<&str> {
        self.created_by_user_action_resolution_id.as_deref()
    }

    pub fn idle_expires_at(&self) -> Option<&UtcTimestamp> {
        self.idle_expires_at.as_ref()
    }

    pub fn invalidation_reason(&self) -> Option<WriteTicketInvalidationReason> {
        self.invalidation_reason
    }

    pub fn consumed_by_run_id(&self) -> Option<&str> {
        self.consumed_by_run_id.as_deref()
    }

    pub fn consumed_at(&self) -> Option<&UtcTimestamp> {
        self.consumed_at.as_ref()
    }

    pub fn revoked_at(&self) -> Option<&UtcTimestamp> {
        self.revoked_at.as_ref()
    }

    pub fn created_at(&self) -> &UtcTimestamp {
        &self.created_at
    }

    pub fn metadata(&self) -> &JsonObject {
        &self.metadata
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WriteTicketRecordRaw {
    project_id: String,
    write_ticket_id: String,
    task_id: String,
    change_unit_id: Option<String>,
    basis_state_version: i64,
    status: String,
    validity_basis_json: String,
    allowed_path_prefixes_json: String,
    denied_path_prefixes_json: String,
    attempt_scope_json: String,
    created_by_actor_source: String,
    created_by_user_action_resolution_id: Option<String>,
    idle_expires_at: Option<String>,
    invalidation_reason: Option<String>,
    consumed_by_run_id: Option<String>,
    consumed_at: Option<String>,
    revoked_at: Option<String>,
    created_at: String,
    metadata_json: String,
}

/// Validated Write Ticket authority facts consumed by workflow-policy storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WriteTicketAuthorityBinding {
    pub(crate) write_ticket_id: WriteTicketId,
    pub(crate) task_id: TaskId,
    pub(crate) write_authority_fingerprint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteTicketValidationFailure {
    Field {
        logical_column: &'static str,
        kind: WriteTicketFieldKind,
    },
    Invariant(WriteTicketInvariant),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteTicketFieldKind {
    Json,
    Value,
}

impl CoreProjectStore<'_> {
    /// Lists active Write Tickets for a Task.
    pub fn active_write_tickets(&self, task_id: &TaskId) -> StoreResult<Vec<StoredWriteTicket>> {
        active_write_tickets(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Lists Write Tickets for a Task without mutating effective status.
    pub fn write_tickets_for_task(&self, task_id: &TaskId) -> StoreResult<Vec<StoredWriteTicket>> {
        write_tickets_for_task(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Reads one Write Ticket row by exact project-local identity.
    pub fn write_ticket_record(
        &self,
        write_ticket_id: &str,
    ) -> StoreResult<Option<StoredWriteTicket>> {
        write_ticket_record(&self.conn, &self.project.project_id, write_ticket_id)
    }

    /// Inserts one fully typed Write Ticket fixture through the aggregate owner.
    #[cfg(any(test, feature = "test-support"))]
    pub fn insert_write_ticket_fixture(
        &self,
        input: &WriteTicketInsert,
        basis_state_version: u64,
    ) -> StoreResult<()> {
        validate_identifier("write_ticket_id", input.write_ticket_id.as_str())?;
        validate_identifier("task_id", input.task_id.as_str())?;
        validate_identifier("change_unit_id", input.change_unit_id.as_str())?;
        if let Some(resolution_id) = &input.created_by_user_action_resolution_id {
            validate_identifier(
                "created_by_user_action_resolution_id",
                resolution_id.as_str(),
            )?;
        }
        let record =
            active_write_ticket_record(&self.project.project_id, input, basis_state_version);
        validate_write_ticket_record(&record).map_err(|failure| StoreError::InvalidInput {
            detail: format!("write-ticket fixture is internally inconsistent: {failure:?}"),
        })?;
        insert_write_ticket_record(&self.conn, &record)
    }

    /// Replaces typed authority facts for semantic-policy fixtures.
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_write_ticket_authority_fixture(
        &self,
        write_ticket_id: &str,
        validity_basis: WriteTicketValidityBasis,
        attempt_scope: WriteTicketAttemptScope,
    ) -> StoreResult<()> {
        let mut record =
            write_ticket_record(&self.conn, &self.project.project_id, write_ticket_id)?
                .ok_or_else(|| StoreError::NotFound {
                    entity: "write_ticket",
                    id: write_ticket_id.to_owned(),
                })?;
        record.validity_basis = validity_basis;
        record.attempt_scope = attempt_scope;
        validate_write_ticket_record(&record).map_err(|failure| StoreError::InvalidInput {
            detail: format!("write-ticket fixture is internally inconsistent: {failure:?}"),
        })?;
        let validity_basis_json = volicord_types::canonical::canonical_json_string(
            &record.validity_basis,
        )
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("write-ticket fixture validity basis cannot be serialized: {error}"),
        })?;
        let attempt_scope_json = volicord_types::canonical::canonical_json_string(
            &record.attempt_scope,
        )
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("write-ticket fixture attempt scope cannot be serialized: {error}"),
        })?;
        self.conn.execute(
            "UPDATE write_tickets
                SET validity_basis_json = ?3,
                    attempt_scope_json = ?4
              WHERE project_id = ?1
                AND write_ticket_id = ?2",
            params![
                self.project.project_id,
                write_ticket_id,
                validity_basis_json,
                attempt_scope_json
            ],
        )?;
        Ok(())
    }

    /// Replaces persisted lifecycle timestamps for aggregate timestamp fixtures.
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_write_ticket_timestamps_fixture(
        &self,
        write_ticket_id: &str,
        created_at: &str,
        idle_expires_at: Option<&str>,
    ) -> StoreResult<()> {
        self.conn.execute(
            "UPDATE write_tickets
                SET created_at = ?3,
                    idle_expires_at = ?4
              WHERE project_id = ?1
                AND write_ticket_id = ?2",
            params![
                self.project.project_id,
                write_ticket_id,
                created_at,
                idle_expires_at
            ],
        )?;
        Ok(())
    }
}

fn active_write_tickets(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<StoredWriteTicket>> {
    let sql = format!(
        "SELECT {WRITE_TICKET_RECORD_COLUMNS}
           FROM write_tickets
          WHERE project_id = ?1
            AND task_id = ?2
          ORDER BY write_ticket_id"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(
        params![project_id, task_id],
        write_ticket_record_raw_from_row,
    )?;
    let mut records = Vec::new();
    for row in rows {
        let record = decode_write_ticket_record(row?)?;
        if record.status == WriteTicketStatus::Active {
            records.push(record);
        }
    }
    Ok(records)
}

fn write_tickets_for_task(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<StoredWriteTicket>> {
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
) -> StoreResult<Option<StoredWriteTicket>> {
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

fn write_ticket_record_in_tx(
    tx: &Transaction<'_>,
    project_id: &str,
    write_ticket_id: &str,
) -> StoreResult<Option<StoredWriteTicket>> {
    write_ticket_record(tx, project_id, write_ticket_id)
}

pub(crate) fn active_write_ticket_authority_bindings_in_tx(
    tx: &Transaction<'_>,
    project_id: &str,
) -> StoreResult<Vec<WriteTicketAuthorityBinding>> {
    let sql = format!(
        "SELECT {WRITE_TICKET_RECORD_COLUMNS}
           FROM write_tickets
          WHERE project_id = ?1
          ORDER BY write_ticket_id"
    );
    let mut statement = tx.prepare(&sql)?;
    let rows = statement.query_map([project_id], write_ticket_record_raw_from_row)?;
    let mut bindings = Vec::new();
    for row in rows {
        let record = decode_write_ticket_record(row?)?;
        if record.status != WriteTicketStatus::Active {
            continue;
        }
        bindings.push(WriteTicketAuthorityBinding {
            write_ticket_id: WriteTicketId::new(record.write_ticket_id),
            task_id: TaskId::new(record.task_id),
            write_authority_fingerprint: record.validity_basis.write_authority_fingerprint,
        });
    }
    Ok(bindings)
}

pub(crate) fn invalidate_active_write_ticket_ids_in_tx(
    tx: &Transaction<'_>,
    project_id: &str,
    write_ticket_ids: &[WriteTicketId],
    invalidation_reason: WriteTicketInvalidationReason,
) -> StoreResult<Vec<String>> {
    let invalidation_reason =
        encode_closed_value("write_tickets.invalidation_reason", &invalidation_reason)?;
    let mut invalidated = Vec::with_capacity(write_ticket_ids.len());
    for write_ticket_id in write_ticket_ids {
        let changed = tx.execute(
            "UPDATE write_tickets
                SET status = 'invalidated',
                    invalidation_reason = ?3
              WHERE project_id = ?1
                AND write_ticket_id = ?2
                AND status = 'active'",
            params![project_id, write_ticket_id.as_str(), invalidation_reason],
        )?;
        if changed != 1 {
            return Err(StoreError::schema_invariant(
                "project_state",
                "identified active Write Ticket invalidation changed no rows",
            ));
        }
        invalidated.push(write_ticket_id.as_str().to_owned());
    }
    Ok(invalidated)
}

pub(super) fn write_ticket_count(conn: &Connection, project_id: &str) -> StoreResult<u64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM write_tickets WHERE project_id = ?1",
        [project_id],
        |row| row.get(0),
    )?;
    nonnegative_i64_to_u64("write ticket count", count).map_err(StoreError::from)
}

fn write_ticket_record_raw_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<WriteTicketRecordRaw> {
    Ok(WriteTicketRecordRaw {
        project_id: row.get(0)?,
        write_ticket_id: row.get(1)?,
        task_id: row.get(2)?,
        change_unit_id: row.get(3)?,
        basis_state_version: row.get(4)?,
        status: row.get(5)?,
        validity_basis_json: row.get(6)?,
        allowed_path_prefixes_json: row.get(7)?,
        denied_path_prefixes_json: row.get(8)?,
        attempt_scope_json: row.get(9)?,
        created_by_actor_source: row.get(10)?,
        created_by_user_action_resolution_id: row.get(11)?,
        idle_expires_at: row.get(12)?,
        invalidation_reason: row.get(13)?,
        consumed_by_run_id: row.get(14)?,
        consumed_at: row.get(15)?,
        revoked_at: row.get(16)?,
        created_at: row.get(17)?,
        metadata_json: row.get(18)?,
    })
}

fn decode_write_ticket_record(raw: WriteTicketRecordRaw) -> StoreResult<StoredWriteTicket> {
    let record_ref = raw.write_ticket_id.clone();
    let corrupt_value =
        |column| StoreError::corrupt_owner_state_value("write_tickets", record_ref.clone(), column);
    let corrupt_json =
        |column| StoreError::corrupt_owner_state_json("write_tickets", record_ref.clone(), column);
    let project_id = nonempty_stored_identifier(&raw.project_id)
        .ok_or_else(|| corrupt_value("project_id"))?
        .to_owned();
    let write_ticket_id = nonempty_stored_identifier(&raw.write_ticket_id)
        .ok_or_else(|| corrupt_value("write_ticket_id"))?
        .to_owned();
    let task_id = nonempty_stored_identifier(&raw.task_id)
        .ok_or_else(|| corrupt_value("task_id"))?
        .to_owned();
    let change_unit_id = raw
        .change_unit_id
        .as_deref()
        .and_then(nonempty_stored_identifier)
        .ok_or_else(|| corrupt_value("change_unit_id"))?;
    let change_unit_id = change_unit_id.to_owned();
    let basis_state_version =
        u64::try_from(raw.basis_state_version).map_err(|_| corrupt_value("basis_state_version"))?;
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
    let path_scope = WriteTicketPathScope::new(allowed_path_prefixes, denied_path_prefixes)
        .map_err(|error| {
            StoreError::corrupt_write_ticket_invariant(
                record_ref.clone(),
                write_ticket_path_scope_invariant(error),
            )
        })?;
    let attempt_scope: WriteTicketAttemptScope = decode_owner_json_text(
        "write_tickets",
        &record_ref,
        "attempt_scope_json",
        &raw.attempt_scope_json,
    )?;
    let created_by_actor_source = raw
        .created_by_actor_source
        .parse::<ActorSource>()
        .map_err(|_| corrupt_value("created_by_actor_source"))?;
    let created_by_user_action_resolution_id = raw
        .created_by_user_action_resolution_id
        .map(|value| {
            nonempty_stored_identifier(&value)
                .map(str::to_owned)
                .ok_or_else(|| corrupt_value("created_by_user_action_resolution_id"))
        })
        .transpose()?;
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
    let revoked_at = raw
        .revoked_at
        .as_deref()
        .map(|value| UtcTimestamp::parse(value).map_err(|_| corrupt_value("revoked_at")))
        .transpose()?;
    let consumed_by_run_id = raw
        .consumed_by_run_id
        .map(|value| {
            nonempty_stored_identifier(&value)
                .map(str::to_owned)
                .ok_or_else(|| corrupt_value("consumed_by_run_id"))
        })
        .transpose()?;
    let metadata: JsonObject = decode_owner_json_text(
        "write_tickets",
        &record_ref,
        "metadata_json",
        &raw.metadata_json,
    )?;
    let record = StoredWriteTicket {
        project_id,
        write_ticket_id,
        task_id,
        change_unit_id,
        basis_state_version,
        status,
        validity_basis,
        path_scope,
        attempt_scope,
        created_by_actor_source,
        created_by_user_action_resolution_id,
        idle_expires_at,
        invalidation_reason,
        consumed_by_run_id,
        consumed_at,
        revoked_at,
        created_at,
        metadata,
    };
    validate_write_ticket_record(&record).map_err(|failure| match failure {
        WriteTicketValidationFailure::Field {
            logical_column,
            kind: WriteTicketFieldKind::Json,
        } => corrupt_json(logical_column),
        WriteTicketValidationFailure::Field {
            logical_column,
            kind: WriteTicketFieldKind::Value,
        } => corrupt_value(logical_column),
        WriteTicketValidationFailure::Invariant(invariant) => {
            StoreError::corrupt_write_ticket_invariant(record_ref.clone(), invariant)
        }
    })?;
    Ok(record)
}

fn nonempty_stored_identifier(value: &str) -> Option<&str> {
    (!value.trim().is_empty()).then_some(value)
}

fn write_ticket_path_scope_invariant(error: WriteTicketPathScopeError) -> WriteTicketInvariant {
    match error {
        WriteTicketPathScopeError::DuplicateAllowedPath => {
            WriteTicketInvariant::DuplicateAllowedPaths
        }
        WriteTicketPathScopeError::DuplicateDeniedPath => {
            WriteTicketInvariant::DuplicateDeniedPaths
        }
        WriteTicketPathScopeError::AllowedDeniedOverlap => {
            WriteTicketInvariant::AllowedDeniedPathDisjointness
        }
    }
}

fn validate_write_ticket_record(
    record: &StoredWriteTicket,
) -> Result<(), WriteTicketValidationFailure> {
    let invalid_json = |logical_column| WriteTicketValidationFailure::Field {
        logical_column,
        kind: WriteTicketFieldKind::Json,
    };
    let invalid_value = |logical_column| WriteTicketValidationFailure::Field {
        logical_column,
        kind: WriteTicketFieldKind::Value,
    };
    if record.basis_state_version == 0 {
        return Err(invalid_value("basis_state_version"));
    }
    if record.validity_basis.scope_revision == 0 {
        return Err(invalid_json("validity_basis_json"));
    }
    if record.attempt_scope.intended_operation.trim().is_empty() {
        return Err(invalid_json("attempt_scope_json"));
    }
    if !canonical_write_authority_fingerprint(&record.validity_basis.write_authority_fingerprint)
        || record
            .validity_basis
            .workspace_context_sha256
            .as_ref()
            .is_some_and(|digest| !lowercase_sha256_hex(digest))
    {
        return Err(invalid_json("validity_basis_json"));
    }
    for (logical_column, timestamp) in [
        ("created_at", Some(&record.created_at)),
        ("idle_expires_at", record.idle_expires_at.as_ref()),
        ("consumed_at", record.consumed_at.as_ref()),
        ("revoked_at", record.revoked_at.as_ref()),
    ] {
        if timestamp
            .is_some_and(|timestamp| timestamp.ensure_canonical_rfc3339_representable().is_err())
        {
            return Err(invalid_value(logical_column));
        }
    }
    let task_id = record.task_id.as_str();
    if record.validity_basis.task_id.as_str() != task_id
        || record.attempt_scope.task_id.as_str() != task_id
    {
        return Err(WriteTicketValidationFailure::Invariant(
            WriteTicketInvariant::TaskIdentityAgreement,
        ));
    }
    if record
        .validity_basis
        .approval_basis_refs
        .iter()
        .any(|reference| {
            reference.project_id().as_str() != record.project_id.as_str()
                || reference.task_id().as_str() != record.task_id.as_str()
        })
    {
        return Err(WriteTicketValidationFailure::Invariant(
            WriteTicketInvariant::ApprovalOwnerAgreement,
        ));
    }
    if record
        .validity_basis
        .approval_basis_refs
        .iter()
        .any(|reference| {
            nonempty_stored_identifier(reference.project_id().as_str()).is_none()
                || nonempty_stored_identifier(reference.task_id().as_str()).is_none()
                || nonempty_stored_identifier(reference.resolution_id().as_str()).is_none()
                || reference.produced_at_state_version().is_none()
        })
    {
        return Err(WriteTicketValidationFailure::Invariant(
            WriteTicketInvariant::ApprovalReferenceMetadata,
        ));
    }
    let approval_identities = record
        .validity_basis
        .approval_basis_refs
        .iter()
        .map(|reference| reference.identity())
        .collect::<BTreeSet<_>>();
    if approval_identities.len() != record.validity_basis.approval_basis_refs.len() {
        return Err(WriteTicketValidationFailure::Invariant(
            WriteTicketInvariant::DuplicateApprovalResolutionIdentity,
        ));
    }
    let change_unit_id = record.change_unit_id.as_str();
    if record.validity_basis.change_unit_id.as_str() != change_unit_id
        || record.attempt_scope.change_unit_id.as_str() != change_unit_id
    {
        return Err(WriteTicketValidationFailure::Invariant(
            WriteTicketInvariant::ChangeUnitIdentityAgreement,
        ));
    }
    if record.validity_basis.scope_revision > record.basis_state_version {
        return Err(WriteTicketValidationFailure::Invariant(
            WriteTicketInvariant::ScopeRevisionAgreement,
        ));
    }
    if record.validity_basis.baseline_ref != record.attempt_scope.baseline_ref {
        return Err(WriteTicketValidationFailure::Invariant(
            WriteTicketInvariant::BaselineAgreement,
        ));
    }
    if record
        .idle_expires_at
        .as_ref()
        .is_some_and(|expires_at| expires_at <= &record.created_at)
        || record
            .consumed_at
            .as_ref()
            .is_some_and(|consumed_at| consumed_at < &record.created_at)
        || record
            .revoked_at
            .as_ref()
            .is_some_and(|revoked_at| revoked_at < &record.created_at)
    {
        return Err(WriteTicketValidationFailure::Invariant(
            WriteTicketInvariant::TimestampOrder,
        ));
    }
    let intended_paths = record
        .attempt_scope
        .intended_paths
        .iter()
        .collect::<BTreeSet<_>>();
    if intended_paths.len() != record.attempt_scope.intended_paths.len() {
        return Err(WriteTicketValidationFailure::Invariant(
            WriteTicketInvariant::DuplicateIntendedPaths,
        ));
    }
    if record.attempt_scope.intended_paths.iter().any(|intended| {
        !record
            .path_scope
            .allowed()
            .iter()
            .any(|allowed| intended.is_within(allowed))
            || record
                .path_scope
                .denied()
                .iter()
                .any(|denied| intended.is_within(denied))
    }) {
        return Err(WriteTicketValidationFailure::Invariant(
            WriteTicketInvariant::IntendedPathCoverage,
        ));
    }
    if record.attempt_scope.product_file_write_intended
        != !record.attempt_scope.intended_paths.is_empty()
    {
        return Err(WriteTicketValidationFailure::Invariant(
            WriteTicketInvariant::ProductFileWriteIntentAgreement,
        ));
    }
    let status_columns_are_valid = match record.status {
        WriteTicketStatus::Active => {
            record.invalidation_reason.is_none()
                && record.consumed_by_run_id.is_none()
                && record.consumed_at.is_none()
                && record.revoked_at.is_none()
        }
        WriteTicketStatus::Consumed => {
            record.invalidation_reason.is_none()
                && record.consumed_by_run_id.is_some()
                && record.consumed_at.is_some()
                && record.revoked_at.is_none()
        }
        WriteTicketStatus::Invalidated => {
            record.invalidation_reason.is_some()
                && record.consumed_by_run_id.is_none()
                && record.consumed_at.is_none()
                && record.revoked_at.is_none()
        }
        WriteTicketStatus::Revoked => {
            record.invalidation_reason.is_some()
                && record.consumed_by_run_id.is_none()
                && record.consumed_at.is_none()
                && record.revoked_at.is_some()
        }
    };
    if !status_columns_are_valid {
        return Err(WriteTicketValidationFailure::Invariant(
            WriteTicketInvariant::StatusLifecycleAgreement,
        ));
    }
    Ok(())
}

fn canonical_write_authority_fingerprint(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn lowercase_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn active_write_ticket_record(
    project_id: &str,
    input: &WriteTicketInsert,
    basis_state_version: u64,
) -> StoredWriteTicket {
    StoredWriteTicket {
        project_id: project_id.to_owned(),
        write_ticket_id: input.write_ticket_id.as_str().to_owned(),
        task_id: input.task_id.as_str().to_owned(),
        change_unit_id: input.change_unit_id.as_str().to_owned(),
        basis_state_version,
        status: WriteTicketStatus::Active,
        validity_basis: input.validity_basis.clone(),
        path_scope: input.path_scope.clone(),
        attempt_scope: input.attempt_scope.clone(),
        created_by_actor_source: input.created_by_actor_source.clone(),
        created_by_user_action_resolution_id: input
            .created_by_user_action_resolution_id
            .as_ref()
            .map(|resolution_id| resolution_id.as_str().to_owned()),
        idle_expires_at: input.idle_expires_at.clone(),
        invalidation_reason: None,
        consumed_by_run_id: None,
        consumed_at: None,
        revoked_at: None,
        created_at: input.created_at.clone(),
        metadata: input.metadata.clone(),
    }
}

fn insert_write_ticket_record(conn: &Connection, record: &StoredWriteTicket) -> StoreResult<()> {
    let validity_basis_json = volicord_types::canonical::canonical_json_string(
        &record.validity_basis,
    )
    .map_err(|error| StoreError::InvalidInput {
        detail: format!("write-ticket validity basis cannot be serialized: {error}"),
    })?;
    let allowed_path_prefixes_json = volicord_types::canonical::canonical_json_string(
        &record.path_scope.allowed(),
    )
    .map_err(|error| StoreError::InvalidInput {
        detail: format!("write-ticket allowed paths cannot be serialized: {error}"),
    })?;
    let denied_path_prefixes_json = volicord_types::canonical::canonical_json_string(
        &record.path_scope.denied(),
    )
    .map_err(|error| StoreError::InvalidInput {
        detail: format!("write-ticket denied paths cannot be serialized: {error}"),
    })?;
    let attempt_scope_json = volicord_types::canonical::canonical_json_string(
        &record.attempt_scope,
    )
    .map_err(|error| StoreError::InvalidInput {
        detail: format!("write-ticket attempt scope cannot be serialized: {error}"),
    })?;
    let metadata_json = volicord_types::canonical::canonical_json_string(&record.metadata)
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("write-ticket metadata cannot be serialized: {error}"),
        })?;
    let basis_state_version = u64_to_i64("basis_state_version", record.basis_state_version)?;
    let status = encode_closed_value("write_tickets.status", &record.status)?;
    let created_by_actor_source = record.created_by_actor_source.to_canonical_string();
    let invalidation_reason = record
        .invalidation_reason
        .as_ref()
        .map(|reason| encode_closed_value("write_tickets.invalidation_reason", reason))
        .transpose()?;

    conn.execute(
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
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
        )",
        params![
            record.project_id,
            record.write_ticket_id,
            record.task_id,
            record.change_unit_id,
            basis_state_version,
            status,
            validity_basis_json,
            allowed_path_prefixes_json,
            denied_path_prefixes_json,
            attempt_scope_json,
            created_by_actor_source,
            record.created_by_user_action_resolution_id,
            record.idle_expires_at.as_ref().map(ToString::to_string),
            invalidation_reason,
            record.consumed_by_run_id,
            record.consumed_at.as_ref().map(ToString::to_string),
            record.revoked_at.as_ref().map(ToString::to_string),
            record.created_at.to_string(),
            metadata_json,
        ],
    )?;
    Ok(())
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
        let write_ticket_ids = active_write_tickets(self.tx, self.project_id, &input.task_id)?
            .into_iter()
            .map(|record| WriteTicketId::new(record.write_ticket_id))
            .collect::<Vec<_>>();
        invalidate_active_write_ticket_ids_in_tx(
            self.tx,
            self.project_id,
            &write_ticket_ids,
            input.invalidation_reason,
        )?;
        Ok(())
    }

    fn invalidate_write_ticket(&mut self, input: &WriteTicketByIdInvalidation) -> StoreResult<()> {
        validate_identifier("write_ticket_id", &input.write_ticket_id)?;
        let record = write_ticket_record_in_tx(self.tx, self.project_id, &input.write_ticket_id)?
            .ok_or_else(|| StoreError::SchemaInvariant {
            database_kind: "project_state",
            detail: "identified active write ticket invalidation changed no rows".to_owned(),
        })?;
        if record.status != WriteTicketStatus::Active {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "identified active write ticket invalidation changed no rows".to_owned(),
            });
        }
        invalidate_active_write_ticket_ids_in_tx(
            self.tx,
            self.project_id,
            &[WriteTicketId::new(input.write_ticket_id.clone())],
            input.invalidation_reason,
        )?;
        Ok(())
    }

    fn insert_write_ticket(
        &mut self,
        input: &WriteTicketInsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        validate_identifier("write_ticket_id", input.write_ticket_id.as_str())?;
        validate_identifier("task_id", input.task_id.as_str())?;
        validate_identifier("change_unit_id", input.change_unit_id.as_str())?;
        if let Some(resolution_id) = &input.created_by_user_action_resolution_id {
            validate_identifier(
                "created_by_user_action_resolution_id",
                resolution_id.as_str(),
            )?;
        }
        let record = active_write_ticket_record(self.project_id, input, committed_state_version);
        validate_write_ticket_record(&record).map_err(|failure| StoreError::InvalidInput {
            detail: format!("write-ticket typed authority facts are inconsistent: {failure:?}"),
        })?;
        insert_write_ticket_record(self.tx, &record)
    }

    fn consume_write_ticket(&mut self, input: &WriteTicketConsumption) -> StoreResult<()> {
        validate_identifier("write_ticket_id", &input.write_ticket_id)?;
        validate_identifier("run_id", &input.run_id)?;
        let ticket = write_ticket_record_in_tx(self.tx, self.project_id, &input.write_ticket_id)?
            .ok_or_else(|| StoreError::NotFound {
            entity: "write_ticket",
            id: input.write_ticket_id.clone(),
        })?;
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
    use crate::StoreAggregateInvariant;
    use serde_json::{json, Value};

    #[derive(Debug, Clone, Copy)]
    enum ExpectedCorruption {
        Json(&'static str),
        Value(&'static str),
        Invariant(WriteTicketInvariant),
    }

    fn validity_basis_with_approval_refs(approval_basis_refs: Value) -> String {
        serde_json::to_string(&json!({
            "task_id": "task",
            "change_unit_id": "change",
            "scope_revision": 2,
            "baseline_ref": "baseline",
            "workspace_context_sha256": "a".repeat(64),
            "write_authority_fingerprint": format!("sha256:{}", "b".repeat(64)),
            "approval_basis_refs": approval_basis_refs,
        }))
        .expect("validity basis fixture should serialize")
    }

    fn approval_ref(
        project_id: &str,
        task_id: &str,
        resolution_id: &str,
        produced_at_state_version: Value,
    ) -> Value {
        json!({
            "record_kind": "user_action_resolution",
            "record_id": resolution_id,
            "project_id": project_id,
            "task_id": task_id,
            "produced_at_state_version": produced_at_state_version,
        })
    }

    fn valid_raw_write_ticket() -> WriteTicketRecordRaw {
        WriteTicketRecordRaw {
            project_id: "project".to_owned(),
            write_ticket_id: "ticket".to_owned(),
            task_id: "task".to_owned(),
            change_unit_id: Some("change".to_owned()),
            basis_state_version: 3,
            status: "active".to_owned(),
            validity_basis_json: validity_basis_with_approval_refs(json!([approval_ref(
                "project",
                "task",
                "resolution",
                json!(6),
            )])),
            allowed_path_prefixes_json: r#"["src"]"#.to_owned(),
            denied_path_prefixes_json: r#"["tests"]"#.to_owned(),
            attempt_scope_json: r#"{"task_id":"task","change_unit_id":"change","intended_operation":"edit","intended_paths":["src/lib.rs"],"product_file_write_intended":true,"sensitive_categories":[],"baseline_ref":"baseline"}"#.to_owned(),
            created_by_actor_source: "agent_connection:connection".to_owned(),
            created_by_user_action_resolution_id: Some("resolution".to_owned()),
            idle_expires_at: Some("2026-07-29T02:00:00Z".to_owned()),
            invalidation_reason: None,
            consumed_by_run_id: None,
            consumed_at: None,
            revoked_at: None,
            created_at: "2026-07-29T00:00:00Z".to_owned(),
            metadata_json: r#"{"owner":"write_ticket"}"#.to_owned(),
        }
    }

    fn assert_corruption(error: StoreError, expected: ExpectedCorruption) {
        match expected {
            ExpectedCorruption::Json(logical_column) => assert!(matches!(
                error,
                StoreError::CorruptOwnerStateJson {
                    table: "write_tickets",
                    logical_column: actual,
                    ..
                } if actual == logical_column
            )),
            ExpectedCorruption::Value(logical_column) => assert!(matches!(
                error,
                StoreError::CorruptOwnerStateValue {
                    table: "write_tickets",
                    logical_column: actual,
                    ..
                } if actual == logical_column
            )),
            ExpectedCorruption::Invariant(expected) => assert!(matches!(
                error,
                StoreError::CorruptOwnerStateInvariant {
                    invariant: StoreAggregateInvariant::WriteTicket(actual),
                    ..
                } if actual == expected
            )),
        }
    }

    #[test]
    fn write_ticket_decoder_accepts_a_complete_valid_row() {
        let record = decode_write_ticket_record(valid_raw_write_ticket())
            .expect("complete valid Write Ticket row must decode");

        assert_eq!(record.write_ticket_id, "ticket");
        assert_eq!(record.validity_basis.scope_revision, 2);
        let approval_ref = record
            .validity_basis
            .approval_basis_refs
            .first()
            .expect("valid row must preserve its approval reference");
        assert_eq!(approval_ref.project_id().as_str(), "project");
        assert_eq!(approval_ref.task_id().as_str(), "task");
        assert_eq!(approval_ref.resolution_id().as_str(), "resolution");
        assert_eq!(approval_ref.produced_at_state_version(), Some(6));
        assert_eq!(
            record.created_by_user_action_resolution_id.as_deref(),
            Some("resolution")
        );
        assert_eq!(
            record
                .metadata
                .get("owner")
                .and_then(serde_json::Value::as_str),
            Some("write_ticket")
        );
    }

    #[test]
    fn write_ticket_decoder_reports_precise_field_and_invariant_corruption() {
        type MutateRaw = Box<dyn Fn(&mut WriteTicketRecordRaw)>;
        let cases: Vec<(&str, MutateRaw, ExpectedCorruption)> = vec![
            (
                "invalid status",
                Box::new(|raw| raw.status = "unknown".to_owned()),
                ExpectedCorruption::Value("status"),
            ),
            (
                "malformed validity basis",
                Box::new(|raw| raw.validity_basis_json = "{".to_owned()),
                ExpectedCorruption::Json("validity_basis_json"),
            ),
            (
                "wrong approval record kind",
                Box::new(|raw| {
                    let mut reference = approval_ref("project", "task", "resolution", json!(6));
                    reference["record_kind"] = json!("task");
                    raw.validity_basis_json = validity_basis_with_approval_refs(json!([reference]));
                }),
                ExpectedCorruption::Json("validity_basis_json"),
            ),
            (
                "missing approval task owner",
                Box::new(|raw| {
                    let mut reference = approval_ref("project", "task", "resolution", json!(6));
                    reference
                        .as_object_mut()
                        .expect("approval fixture should be an object")
                        .remove("task_id");
                    raw.validity_basis_json = validity_basis_with_approval_refs(json!([reference]));
                }),
                ExpectedCorruption::Json("validity_basis_json"),
            ),
            (
                "approval project owner disagreement",
                Box::new(|raw| {
                    raw.validity_basis_json =
                        validity_basis_with_approval_refs(json!([approval_ref(
                            "other-project",
                            "task",
                            "resolution",
                            json!(6),
                        )]));
                }),
                ExpectedCorruption::Invariant(WriteTicketInvariant::ApprovalOwnerAgreement),
            ),
            (
                "approval task owner disagreement",
                Box::new(|raw| {
                    raw.validity_basis_json =
                        validity_basis_with_approval_refs(json!([approval_ref(
                            "project",
                            "other-task",
                            "resolution",
                            json!(6),
                        )]));
                }),
                ExpectedCorruption::Invariant(WriteTicketInvariant::ApprovalOwnerAgreement),
            ),
            (
                "missing approval projection metadata",
                Box::new(|raw| {
                    raw.validity_basis_json =
                        validity_basis_with_approval_refs(json!([approval_ref(
                            "project",
                            "task",
                            "resolution",
                            Value::Null,
                        )]));
                }),
                ExpectedCorruption::Invariant(WriteTicketInvariant::ApprovalReferenceMetadata),
            ),
            (
                "empty approval resolution identity",
                Box::new(|raw| {
                    raw.validity_basis_json =
                        validity_basis_with_approval_refs(json!([approval_ref(
                            "project",
                            "task",
                            "",
                            json!(6),
                        )]));
                }),
                ExpectedCorruption::Invariant(WriteTicketInvariant::ApprovalReferenceMetadata),
            ),
            (
                "duplicate approval resolution identity",
                Box::new(|raw| {
                    raw.validity_basis_json = validity_basis_with_approval_refs(json!([
                        approval_ref("project", "task", "resolution", json!(6)),
                        approval_ref("project", "task", "resolution", json!(7)),
                    ]));
                }),
                ExpectedCorruption::Invariant(
                    WriteTicketInvariant::DuplicateApprovalResolutionIdentity,
                ),
            ),
            (
                "malformed attempt scope",
                Box::new(|raw| raw.attempt_scope_json = "{".to_owned()),
                ExpectedCorruption::Json("attempt_scope_json"),
            ),
            (
                "invalid allowed path JSON",
                Box::new(|raw| raw.allowed_path_prefixes_json = r#"["/src"]"#.to_owned()),
                ExpectedCorruption::Json("allowed_path_prefixes_json"),
            ),
            (
                "invalid denied path JSON",
                Box::new(|raw| raw.denied_path_prefixes_json = r#"["../tests"]"#.to_owned()),
                ExpectedCorruption::Json("denied_path_prefixes_json"),
            ),
            (
                "invalid intended path JSON",
                Box::new(|raw| {
                    raw.attempt_scope_json = r#"{"task_id":"task","change_unit_id":"change","intended_operation":"edit","intended_paths":["/src/lib.rs"],"product_file_write_intended":true,"sensitive_categories":[],"baseline_ref":"baseline"}"#.to_owned()
                }),
                ExpectedCorruption::Json("attempt_scope_json"),
            ),
            (
                "invalid authority fingerprint",
                Box::new(|raw| {
                    raw.validity_basis_json = format!(
                        r#"{{"task_id":"task","change_unit_id":"change","scope_revision":2,"baseline_ref":"baseline","workspace_context_sha256":"{}","write_authority_fingerprint":"sha256:not-a-digest","approval_basis_refs":[]}}"#,
                        "a".repeat(64),
                    )
                }),
                ExpectedCorruption::Json("validity_basis_json"),
            ),
            (
                "invalid workspace digest",
                Box::new(|raw| {
                    raw.validity_basis_json = format!(
                        r#"{{"task_id":"task","change_unit_id":"change","scope_revision":2,"baseline_ref":"baseline","workspace_context_sha256":"NOT-A-DIGEST","write_authority_fingerprint":"sha256:{}","approval_basis_refs":[]}}"#,
                        "b".repeat(64),
                    )
                }),
                ExpectedCorruption::Json("validity_basis_json"),
            ),
            (
                "invalid basis state version",
                Box::new(|raw| raw.basis_state_version = 0),
                ExpectedCorruption::Value("basis_state_version"),
            ),
            (
                "invalid scope revision",
                Box::new(|raw| {
                    raw.validity_basis_json = format!(
                        r#"{{"task_id":"task","change_unit_id":"change","scope_revision":0,"baseline_ref":"baseline","workspace_context_sha256":"{}","write_authority_fingerprint":"sha256:{}","approval_basis_refs":[]}}"#,
                        "a".repeat(64),
                        "b".repeat(64),
                    )
                }),
                ExpectedCorruption::Json("validity_basis_json"),
            ),
            (
                "empty intended operation",
                Box::new(|raw| {
                    raw.attempt_scope_json = r#"{"task_id":"task","change_unit_id":"change","intended_operation":"","intended_paths":["src/lib.rs"],"product_file_write_intended":true,"sensitive_categories":[],"baseline_ref":"baseline"}"#.to_owned()
                }),
                ExpectedCorruption::Json("attempt_scope_json"),
            ),
            (
                "task identity disagreement",
                Box::new(|raw| raw.task_id = "other-task".to_owned()),
                ExpectedCorruption::Invariant(WriteTicketInvariant::TaskIdentityAgreement),
            ),
            (
                "change-unit identity disagreement",
                Box::new(|raw| raw.change_unit_id = Some("other-change".to_owned())),
                ExpectedCorruption::Invariant(WriteTicketInvariant::ChangeUnitIdentityAgreement),
            ),
            (
                "scope revision disagreement",
                Box::new(|raw| raw.basis_state_version = 1),
                ExpectedCorruption::Invariant(WriteTicketInvariant::ScopeRevisionAgreement),
            ),
            (
                "baseline disagreement",
                Box::new(|raw| {
                    raw.attempt_scope_json = r#"{"task_id":"task","change_unit_id":"change","intended_operation":"edit","intended_paths":["src/lib.rs"],"product_file_write_intended":true,"sensitive_categories":[],"baseline_ref":"other-baseline"}"#.to_owned()
                }),
                ExpectedCorruption::Invariant(WriteTicketInvariant::BaselineAgreement),
            ),
            (
                "malformed timestamp",
                Box::new(|raw| raw.idle_expires_at = Some("tomorrow".to_owned())),
                ExpectedCorruption::Value("idle_expires_at"),
            ),
            (
                "invalid timestamp order",
                Box::new(|raw| raw.idle_expires_at = Some("2026-07-28T23:59:59Z".to_owned())),
                ExpectedCorruption::Invariant(WriteTicketInvariant::TimestampOrder),
            ),
            (
                "duplicate intended paths",
                Box::new(|raw| {
                    raw.attempt_scope_json = r#"{"task_id":"task","change_unit_id":"change","intended_operation":"edit","intended_paths":["src/lib.rs","src/lib.rs"],"product_file_write_intended":true,"sensitive_categories":[],"baseline_ref":"baseline"}"#.to_owned()
                }),
                ExpectedCorruption::Invariant(WriteTicketInvariant::DuplicateIntendedPaths),
            ),
            (
                "duplicate allowed paths",
                Box::new(|raw| raw.allowed_path_prefixes_json = r#"["src","src"]"#.to_owned()),
                ExpectedCorruption::Invariant(WriteTicketInvariant::DuplicateAllowedPaths),
            ),
            (
                "duplicate denied paths",
                Box::new(|raw| raw.denied_path_prefixes_json = r#"["tests","tests"]"#.to_owned()),
                ExpectedCorruption::Invariant(WriteTicketInvariant::DuplicateDeniedPaths),
            ),
            (
                "allowed and denied overlap",
                Box::new(|raw| raw.denied_path_prefixes_json = r#"["src/private"]"#.to_owned()),
                ExpectedCorruption::Invariant(WriteTicketInvariant::AllowedDeniedPathDisjointness),
            ),
            (
                "intended path outside scope",
                Box::new(|raw| {
                    raw.attempt_scope_json = r#"{"task_id":"task","change_unit_id":"change","intended_operation":"edit","intended_paths":["docs/guide.md"],"product_file_write_intended":true,"sensitive_categories":[],"baseline_ref":"baseline"}"#.to_owned()
                }),
                ExpectedCorruption::Invariant(WriteTicketInvariant::IntendedPathCoverage),
            ),
            (
                "product write intent disagreement",
                Box::new(|raw| {
                    raw.attempt_scope_json = r#"{"task_id":"task","change_unit_id":"change","intended_operation":"edit","intended_paths":["src/lib.rs"],"product_file_write_intended":false,"sensitive_categories":[],"baseline_ref":"baseline"}"#.to_owned()
                }),
                ExpectedCorruption::Invariant(
                    WriteTicketInvariant::ProductFileWriteIntentAgreement,
                ),
            ),
            (
                "status and invalidation inconsistency",
                Box::new(|raw| raw.invalidation_reason = Some("explicit_revoke".to_owned())),
                ExpectedCorruption::Invariant(WriteTicketInvariant::StatusLifecycleAgreement),
            ),
        ];

        for (name, mutate, expected) in cases {
            let mut raw = valid_raw_write_ticket();
            mutate(&mut raw);
            let error = match decode_write_ticket_record(raw) {
                Ok(_) => panic!("{name} must fail closed"),
                Err(error) => error,
            };
            assert_corruption(error, expected);
        }
    }

    #[test]
    fn normal_and_transaction_reads_share_the_same_typed_decoder() -> StoreResult<()> {
        let mut conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE write_tickets (
                project_id TEXT NOT NULL,
                write_ticket_id TEXT NOT NULL,
                task_id TEXT NOT NULL,
                change_unit_id TEXT,
                basis_state_version INTEGER NOT NULL,
                status TEXT NOT NULL,
                validity_basis_json TEXT NOT NULL,
                allowed_path_prefixes_json TEXT NOT NULL,
                denied_path_prefixes_json TEXT NOT NULL,
                attempt_scope_json TEXT NOT NULL,
                created_by_actor_source TEXT NOT NULL,
                created_by_user_action_resolution_id TEXT,
                idle_expires_at TEXT,
                invalidation_reason TEXT,
                consumed_by_run_id TEXT,
                consumed_at TEXT,
                revoked_at TEXT,
                created_at TEXT NOT NULL,
                metadata_json TEXT NOT NULL,
                PRIMARY KEY (project_id, write_ticket_id)
            );",
        )?;
        let expected = decode_write_ticket_record(valid_raw_write_ticket())?;
        insert_write_ticket_record(&conn, &expected)?;
        let normal = write_ticket_record(&conn, "project", "ticket")?
            .expect("normal read must find the fixture");
        let tx = conn.transaction()?;
        let in_tx = write_ticket_record_in_tx(&tx, "project", "ticket")?
            .expect("transaction read must find the fixture");
        let authority_bindings = active_write_ticket_authority_bindings_in_tx(&tx, "project")?;

        assert_eq!(normal, expected);
        assert_eq!(in_tx, expected);
        assert_eq!(
            authority_bindings,
            vec![WriteTicketAuthorityBinding {
                write_ticket_id: WriteTicketId::new("ticket"),
                task_id: TaskId::new("task"),
                write_authority_fingerprint: format!("sha256:{}", "b".repeat(64)),
            }]
        );
        Ok(())
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
