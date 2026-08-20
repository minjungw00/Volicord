use crate::Error;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{path::Path, time::Duration};
use volicord_context::{
    CanonicalInvalidation, CanonicalRecordId, CheckpointId, ContextItemId, DecisionId, OperationId,
    ProjectId, QuestionId, SourceId,
};

const SCHEMA_KIND: &str = "volicord-forgetting-operations";
const SCHEMA_VERSION: i64 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForgettingState {
    Prepared,
    CanonicalCommitted,
    RepairRequired,
    Completed,
}

impl ForgettingState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::CanonicalCommitted => "canonical_committed",
            Self::RepairRequired => "repair_required",
            Self::Completed => "completed",
        }
    }

    fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "prepared" => Ok(Self::Prepared),
            "canonical_committed" => Ok(Self::CanonicalCommitted),
            "repair_required" => Ok(Self::RepairRequired),
            "completed" => Ok(Self::Completed),
            _ => Err(Error::new("forgetting operation state is corrupt")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForgettingOperationRecord {
    pub operation_id: OperationId,
    pub project_id: ProjectId,
    pub record: CanonicalRecordId,
    pub authorization_source_id: SourceId,
    pub state: ForgettingState,
    pub candidate_cleanup_completed: bool,
    pub managed_derived_cleanup_completed: bool,
    pub residue_verified: bool,
}

impl ForgettingOperationRecord {
    pub fn invalidation(&self) -> CanonicalInvalidation {
        CanonicalInvalidation {
            project_id: self.project_id,
            record: self.record,
        }
    }
}

pub struct ForgettingStore {
    connection: Connection,
}

impl ForgettingStore {
    pub fn open(path: &Path) -> Result<Self, Error> {
        let existed = path.try_exists().map_err(|error| {
            Error::with_source("cannot inspect forgetting operation store", error)
        })?;
        let mut connection = Connection::open(path)
            .map_err(|error| Error::with_source("cannot open forgetting operation store", error))?;
        connection
            .busy_timeout(Duration::ZERO)
            .map_err(|error| Error::with_source("cannot configure forgetting store", error))?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = FULL;
                 PRAGMA secure_delete = ON;",
            )
            .map_err(|error| Error::with_source("cannot configure forgetting store", error))?;
        if existed {
            validate_schema(&connection)?;
        } else {
            initialize_schema(&mut connection)?;
        }
        Ok(Self { connection })
    }

    pub fn prepare(
        &mut self,
        operation_id: OperationId,
        project_id: ProjectId,
        record: CanonicalRecordId,
        authorization_source_id: SourceId,
        observed_at_unix_micros: i64,
    ) -> Result<ForgettingOperationRecord, Error> {
        if matches!(record, CanonicalRecordId::Project(_)) {
            return Err(Error::new("Project forgetting is not supported"));
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| Error::with_source("cannot begin forgetting preparation", error))?;
        if let Some(existing) = load_by_target(&transaction, project_id, record)? {
            transaction.commit().map_err(|error| {
                Error::with_source("cannot finish forgetting preparation read", error)
            })?;
            return Ok(existing);
        }
        let (record_kind, record_id) = record_parts(record);
        transaction
            .execute(
                "INSERT INTO forgetting_operations(
                     operation_id, project_id, record_kind, record_id,
                     authorization_source_id, state, candidate_cleanup_completed,
                     managed_derived_cleanup_completed, residue_verified,
                     created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'prepared', 0, 0, 0, ?6, ?6)",
                params![
                    operation_id.as_bytes().as_slice(),
                    project_id.as_bytes().as_slice(),
                    record_kind,
                    record_id.as_slice(),
                    authorization_source_id.as_bytes().as_slice(),
                    observed_at_unix_micros,
                ],
            )
            .map_err(|error| Error::with_source("cannot prepare forgetting operation", error))?;
        transaction
            .commit()
            .map_err(|error| Error::with_source("cannot commit forgetting preparation", error))?;
        self.get(operation_id)
    }

    pub fn get(&self, operation_id: OperationId) -> Result<ForgettingOperationRecord, Error> {
        self.connection
            .query_row(
                "SELECT operation_id, project_id, record_kind, record_id,
                        authorization_source_id, state, candidate_cleanup_completed,
                        managed_derived_cleanup_completed, residue_verified
                 FROM forgetting_operations WHERE operation_id = ?1",
                [operation_id.as_bytes().as_slice()],
                decode_row,
            )
            .optional()
            .map_err(|error| Error::with_source("cannot read forgetting operation", error))?
            .ok_or_else(|| Error::new("forgetting operation was not found"))
    }

    pub fn incomplete(
        &self,
        project_id: Option<ProjectId>,
    ) -> Result<Vec<ForgettingOperationRecord>, Error> {
        let mut values = Vec::new();
        if let Some(project_id) = project_id {
            let mut statement = self
                .connection
                .prepare(
                    "SELECT operation_id, project_id, record_kind, record_id,
                            authorization_source_id, state, candidate_cleanup_completed,
                            managed_derived_cleanup_completed, residue_verified
                     FROM forgetting_operations
                     WHERE state != 'completed' AND project_id = ?1
                     ORDER BY created_at, operation_id",
                )
                .map_err(|error| {
                    Error::with_source("cannot inspect forgetting operations", error)
                })?;
            let rows = statement
                .query_map([project_id.as_bytes().as_slice()], decode_row)
                .map_err(|error| {
                    Error::with_source("cannot inspect forgetting operations", error)
                })?;
            for row in rows {
                values.push(row.map_err(|error| {
                    Error::with_source("cannot decode forgetting operation", error)
                })?);
            }
        } else {
            let mut statement = self
                .connection
                .prepare(
                    "SELECT operation_id, project_id, record_kind, record_id,
                            authorization_source_id, state, candidate_cleanup_completed,
                            managed_derived_cleanup_completed, residue_verified
                     FROM forgetting_operations
                     WHERE state != 'completed'
                     ORDER BY created_at, operation_id",
                )
                .map_err(|error| {
                    Error::with_source("cannot inspect forgetting operations", error)
                })?;
            let rows = statement.query_map([], decode_row).map_err(|error| {
                Error::with_source("cannot inspect forgetting operations", error)
            })?;
            for row in rows {
                values.push(row.map_err(|error| {
                    Error::with_source("cannot decode forgetting operation", error)
                })?);
            }
        }
        Ok(values)
    }

    pub fn mark_canonical_committed(
        &mut self,
        operation_id: OperationId,
        observed_at_unix_micros: i64,
    ) -> Result<ForgettingOperationRecord, Error> {
        self.update(
            operation_id,
            ForgettingState::CanonicalCommitted,
            false,
            false,
            false,
            observed_at_unix_micros,
        )
    }

    pub fn mark_repair_required(
        &mut self,
        operation_id: OperationId,
        candidate_cleanup_completed: bool,
        managed_derived_cleanup_completed: bool,
        residue_verified: bool,
        observed_at_unix_micros: i64,
    ) -> Result<ForgettingOperationRecord, Error> {
        let current = self.get(operation_id)?;
        self.update(
            operation_id,
            ForgettingState::RepairRequired,
            current.candidate_cleanup_completed || candidate_cleanup_completed,
            current.managed_derived_cleanup_completed || managed_derived_cleanup_completed,
            current.residue_verified || residue_verified,
            observed_at_unix_micros,
        )
    }

    pub fn mark_completed(
        &mut self,
        operation_id: OperationId,
        observed_at_unix_micros: i64,
    ) -> Result<ForgettingOperationRecord, Error> {
        self.update(
            operation_id,
            ForgettingState::Completed,
            true,
            true,
            true,
            observed_at_unix_micros,
        )
    }

    fn update(
        &mut self,
        operation_id: OperationId,
        state: ForgettingState,
        candidate_cleanup_completed: bool,
        managed_derived_cleanup_completed: bool,
        residue_verified: bool,
        observed_at_unix_micros: i64,
    ) -> Result<ForgettingOperationRecord, Error> {
        let changed = self
            .connection
            .execute(
                "UPDATE forgetting_operations
                 SET state = ?2, candidate_cleanup_completed = ?3,
                     managed_derived_cleanup_completed = ?4, residue_verified = ?5,
                     updated_at = ?6
                 WHERE operation_id = ?1",
                params![
                    operation_id.as_bytes().as_slice(),
                    state.as_str(),
                    candidate_cleanup_completed,
                    managed_derived_cleanup_completed,
                    residue_verified,
                    observed_at_unix_micros,
                ],
            )
            .map_err(|error| Error::with_source("cannot update forgetting operation", error))?;
        if changed != 1 {
            return Err(Error::new("forgetting operation changed concurrently"));
        }
        self.get(operation_id)
    }
}

fn initialize_schema(connection: &mut Connection) -> Result<(), Error> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| Error::with_source("cannot initialize forgetting schema", error))?;
    transaction
        .execute_batch(
            "CREATE TABLE metadata(
                 schema_kind TEXT NOT NULL,
                 schema_version INTEGER NOT NULL
             );
             CREATE TABLE forgetting_operations(
                 operation_id BLOB PRIMARY KEY NOT NULL CHECK(length(operation_id) = 16),
                 project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                 record_kind TEXT NOT NULL CHECK(record_kind IN
                     ('source','question','decision','context_item','checkpoint')),
                 record_id BLOB NOT NULL CHECK(length(record_id) = 16),
                 authorization_source_id BLOB NOT NULL CHECK(length(authorization_source_id) = 16),
                 state TEXT NOT NULL CHECK(state IN
                     ('prepared','canonical_committed','repair_required','completed')),
                 candidate_cleanup_completed INTEGER NOT NULL CHECK(candidate_cleanup_completed IN (0,1)),
                 managed_derived_cleanup_completed INTEGER NOT NULL CHECK(managed_derived_cleanup_completed IN (0,1)),
                 residue_verified INTEGER NOT NULL CHECK(residue_verified IN (0,1)),
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL,
                 UNIQUE(project_id, record_kind, record_id)
             );",
        )
        .map_err(|error| Error::with_source("cannot create forgetting schema", error))?;
    transaction
        .execute(
            "INSERT INTO metadata(schema_kind, schema_version) VALUES (?1, ?2)",
            params![SCHEMA_KIND, SCHEMA_VERSION],
        )
        .map_err(|error| Error::with_source("cannot identify forgetting schema", error))?;
    transaction
        .commit()
        .map_err(|error| Error::with_source("cannot commit forgetting schema", error))
}

fn validate_schema(connection: &Connection) -> Result<(), Error> {
    let metadata = connection
        .query_row(
            "SELECT schema_kind, schema_version FROM metadata",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|error| Error::with_source("cannot validate forgetting schema", error))?;
    if metadata != Some((SCHEMA_KIND.to_owned(), SCHEMA_VERSION)) {
        return Err(Error::new(
            "forgetting operation store has an unsupported schema",
        ));
    }
    Ok(())
}

fn load_by_target(
    connection: &Connection,
    project_id: ProjectId,
    record: CanonicalRecordId,
) -> Result<Option<ForgettingOperationRecord>, Error> {
    let (record_kind, record_id) = record_parts(record);
    connection
        .query_row(
            "SELECT operation_id, project_id, record_kind, record_id,
                    authorization_source_id, state, candidate_cleanup_completed,
                    managed_derived_cleanup_completed, residue_verified
             FROM forgetting_operations
             WHERE project_id = ?1 AND record_kind = ?2 AND record_id = ?3",
            params![
                project_id.as_bytes().as_slice(),
                record_kind,
                record_id.as_slice()
            ],
            decode_row,
        )
        .optional()
        .map_err(|error| Error::with_source("cannot find forgetting operation", error))
}

fn decode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ForgettingOperationRecord> {
    let operation_id = id_bytes(row.get::<_, Vec<u8>>(0)?)?;
    let project_id = id_bytes(row.get::<_, Vec<u8>>(1)?)?;
    let record_kind = row.get::<_, String>(2)?;
    let record_id = id_bytes(row.get::<_, Vec<u8>>(3)?)?;
    let authorization = id_bytes(row.get::<_, Vec<u8>>(4)?)?;
    let state = ForgettingState::parse(&row.get::<_, String>(5)?).map_err(to_sql_error)?;
    Ok(ForgettingOperationRecord {
        operation_id: OperationId::from_bytes(operation_id),
        project_id: ProjectId::from_bytes(project_id),
        record: record_from_parts(&record_kind, record_id).map_err(to_sql_error)?,
        authorization_source_id: SourceId::from_bytes(authorization),
        state,
        candidate_cleanup_completed: row.get(6)?,
        managed_derived_cleanup_completed: row.get(7)?,
        residue_verified: row.get(8)?,
    })
}

fn id_bytes(bytes: Vec<u8>) -> rusqlite::Result<[u8; 16]> {
    let length = bytes.len();
    bytes.try_into().map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            length,
            rusqlite::types::Type::Blob,
            "identity is not 16 bytes".into(),
        )
    })
}

fn to_sql_error(error: Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn record_parts(record: CanonicalRecordId) -> (&'static str, [u8; 16]) {
    match record {
        CanonicalRecordId::Project(value) => ("project", *value.as_bytes()),
        CanonicalRecordId::Source(value) => ("source", *value.as_bytes()),
        CanonicalRecordId::Question(value) => ("question", *value.as_bytes()),
        CanonicalRecordId::Decision(value) => ("decision", *value.as_bytes()),
        CanonicalRecordId::ContextItem(value) => ("context_item", *value.as_bytes()),
        CanonicalRecordId::Checkpoint(value) => ("checkpoint", *value.as_bytes()),
    }
}

fn record_from_parts(kind: &str, identity: [u8; 16]) -> Result<CanonicalRecordId, Error> {
    match kind {
        "source" => Ok(CanonicalRecordId::Source(SourceId::from_bytes(identity))),
        "question" => Ok(CanonicalRecordId::Question(QuestionId::from_bytes(
            identity,
        ))),
        "decision" => Ok(CanonicalRecordId::Decision(DecisionId::from_bytes(
            identity,
        ))),
        "context_item" => Ok(CanonicalRecordId::ContextItem(ContextItemId::from_bytes(
            identity,
        ))),
        "checkpoint" => Ok(CanonicalRecordId::Checkpoint(CheckpointId::from_bytes(
            identity,
        ))),
        _ => Err(Error::new("forgetting record kind is corrupt")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durable_boundaries_reopen_without_losing_identity_or_progress(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("forgetting.sqlite3");
        let operation_id = OperationId::from_bytes([1; 16]);
        let project_id = ProjectId::from_bytes([2; 16]);
        let record = CanonicalRecordId::Source(SourceId::from_bytes([3; 16]));
        let authorization = SourceId::from_bytes([4; 16]);

        let prepared = ForgettingStore::open(&path)?.prepare(
            operation_id,
            project_id,
            record,
            authorization,
            10,
        )?;
        assert_eq!(prepared.state, ForgettingState::Prepared);
        drop(prepared);
        let mut reopened = ForgettingStore::open(&path)?;
        assert_eq!(reopened.get(operation_id)?.state, ForgettingState::Prepared);
        assert_eq!(
            reopened
                .prepare(
                    OperationId::from_bytes([5; 16]),
                    project_id,
                    record,
                    authorization,
                    11,
                )?
                .operation_id,
            operation_id
        );

        reopened.mark_canonical_committed(operation_id, 12)?;
        drop(reopened);
        let mut reopened = ForgettingStore::open(&path)?;
        assert_eq!(
            reopened.get(operation_id)?.state,
            ForgettingState::CanonicalCommitted
        );
        let repair = reopened.mark_repair_required(operation_id, true, false, false, 13)?;
        assert_eq!(repair.state, ForgettingState::RepairRequired);
        assert!(repair.candidate_cleanup_completed);
        drop(reopened);

        let mut reopened = ForgettingStore::open(&path)?;
        let completed = reopened.mark_completed(operation_id, 14)?;
        assert_eq!(completed.state, ForgettingState::Completed);
        assert!(completed.candidate_cleanup_completed);
        assert!(completed.managed_derived_cleanup_completed);
        assert!(completed.residue_verified);
        assert!(reopened.incomplete(Some(project_id))?.is_empty());
        Ok(())
    }
}
