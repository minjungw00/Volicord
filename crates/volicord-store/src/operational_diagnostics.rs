//! Typed diagnostics for Runtime Home and SQLite operational boundaries.

use std::io;

use rusqlite::{ffi, ErrorCode as SqliteErrorCode};
use serde::Serialize;
use volicord_types::{
    DiagnosticAction, DiagnosticCode, DiagnosticDomain, DiagnosticError, DiagnosticFactSource,
    DiagnosticFacts, DiagnosticFinding, DiagnosticFindingId, DiagnosticSeverity, DiagnosticSource,
    DiagnosticStage, DiagnosticSubject, UtcTimestamp,
};

use crate::{
    runtime_home::{
        RuntimeHomeResolutionError, RuntimePathBoundaryError, RuntimePlatformDiagnostic,
    },
    StoreError,
};

/// Closed Runtime Home diagnostic vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeHomeDiagnostic {
    MissingPath,
    EmptyOrRelativePath,
    InvalidPath,
    RegistryMissing,
    PermissionDenied,
    UnsupportedFilesystem,
    OwnerOrBoundaryMismatch,
}

impl RuntimeHomeDiagnostic {
    pub const ALL: [Self; 7] = [
        Self::MissingPath,
        Self::EmptyOrRelativePath,
        Self::InvalidPath,
        Self::RegistryMissing,
        Self::PermissionDenied,
        Self::UnsupportedFilesystem,
        Self::OwnerOrBoundaryMismatch,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::MissingPath => "runtime_home.path.missing",
            Self::EmptyOrRelativePath => "runtime_home.path.empty_or_relative",
            Self::InvalidPath => "runtime_home.path.invalid",
            Self::RegistryMissing => "runtime_home.registry.missing",
            Self::PermissionDenied => "runtime_home.permission.denied",
            Self::UnsupportedFilesystem => "runtime_home.filesystem.unsupported",
            Self::OwnerOrBoundaryMismatch => "runtime_home.boundary.owner_mismatch",
        }
    }

    pub const fn summary(self) -> &'static str {
        match self {
            Self::MissingPath => "The selected Runtime Home path is missing",
            Self::EmptyOrRelativePath => "The selected Runtime Home path is empty or relative",
            Self::InvalidPath => "The selected Runtime Home path is invalid",
            Self::RegistryMissing => "The Runtime Home Registry is missing",
            Self::PermissionDenied => "Runtime Home access was denied",
            Self::UnsupportedFilesystem => {
                "The Runtime Home is on an unsupported filesystem boundary"
            }
            Self::OwnerOrBoundaryMismatch => {
                "Runtime Home ownership or path boundaries do not match"
            }
        }
    }

    pub const fn action(self) -> RuntimeHomeRecommendedAction {
        match self {
            Self::MissingPath | Self::EmptyOrRelativePath | Self::InvalidPath => {
                RuntimeHomeRecommendedAction::CorrectPath
            }
            Self::RegistryMissing => RuntimeHomeRecommendedAction::InitializeRegistry,
            Self::PermissionDenied => RuntimeHomeRecommendedAction::RepairPermissions,
            Self::UnsupportedFilesystem => RuntimeHomeRecommendedAction::MoveToSupportedFilesystem,
            Self::OwnerOrBoundaryMismatch => RuntimeHomeRecommendedAction::SeparateBoundaries,
        }
    }

    pub const fn from_resolution(error: RuntimeHomeResolutionError) -> Self {
        match error {
            RuntimeHomeResolutionError::EmptyVolicordHome
            | RuntimeHomeResolutionError::RelativeVolicordHome => Self::EmptyOrRelativePath,
            RuntimeHomeResolutionError::MissingUserHome => Self::MissingPath,
        }
    }

    pub const fn from_io_kind(kind: io::ErrorKind) -> Option<Self> {
        match kind {
            io::ErrorKind::NotFound => Some(Self::MissingPath),
            io::ErrorKind::PermissionDenied => Some(Self::PermissionDenied),
            io::ErrorKind::InvalidInput | io::ErrorKind::InvalidData => Some(Self::InvalidPath),
            io::ErrorKind::Unsupported => Some(Self::UnsupportedFilesystem),
            _ => None,
        }
    }

    pub const fn from_path_boundary(error: &RuntimePathBoundaryError) -> Option<Self> {
        match error {
            RuntimePathBoundaryError::InvalidPath { .. } => Some(Self::InvalidPath),
            RuntimePathBoundaryError::BoundaryViolation { .. } => {
                Some(Self::OwnerOrBoundaryMismatch)
            }
            RuntimePathBoundaryError::UnsupportedEnvironment {
                diagnostic:
                    RuntimePlatformDiagnostic::Platform(
                        volicord_platform_fs::PlatformBoundaryDiagnostic::UnsupportedFilesystemBoundary,
                    ),
                ..
            } => Some(Self::UnsupportedFilesystem),
            RuntimePathBoundaryError::UnsupportedEnvironment {
                diagnostic: RuntimePlatformDiagnostic::Platform(_),
                ..
            }
            | RuntimePathBoundaryError::PlatformUnavailable { .. } => None,
        }
    }

    pub fn from_store_error(error: &StoreError) -> Option<Self> {
        match error {
            StoreError::Io(error) => Self::from_io_kind(error.kind()),
            StoreError::Sqlite(rusqlite::Error::InvalidPath(_)) => Some(Self::InvalidPath),
            StoreError::NotFound { entity, .. } if *entity == "runtime_home" => {
                Some(Self::RegistryMissing)
            }
            StoreError::InvalidProjectRegistration { .. } => Some(Self::OwnerOrBoundaryMismatch),
            StoreError::UnsupportedPlatformEnvironment { .. }
            | StoreError::PlatformEnvironmentUnavailable { .. }
            | StoreError::Sqlite(_)
            | StoreError::InvalidInput { .. }
            | StoreError::NotFound { .. }
            | StoreError::Conflict { .. }
            | StoreError::CorruptStoredJson { .. }
            | StoreError::CorruptOwnerStateJson { .. }
            | StoreError::CorruptOwnerStateValue { .. }
            | StoreError::CorruptStoredValue { .. }
            | StoreError::UnsupportedStorageProfile { .. }
            | StoreError::SchemaInvariant { .. }
            | StoreError::RuntimeHomeSchemaMismatch(_)
            | StoreError::RuntimeHomeCorruption(_)
            | StoreError::RuntimeHomePublicationConfirmation(_) => None,
        }
    }
}

/// Closed recommended actions for Runtime Home findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeHomeRecommendedAction {
    CorrectPath,
    InitializeRegistry,
    RepairPermissions,
    MoveToSupportedFilesystem,
    SeparateBoundaries,
}

impl RuntimeHomeRecommendedAction {
    pub const fn code(self) -> &'static str {
        match self {
            Self::CorrectPath => "action.runtime_home.correct_path",
            Self::InitializeRegistry => "action.runtime_home.initialize_registry",
            Self::RepairPermissions => "action.runtime_home.repair_permissions",
            Self::MoveToSupportedFilesystem => "action.runtime_home.move_to_supported_filesystem",
            Self::SeparateBoundaries => "action.runtime_home.separate_boundaries",
        }
    }

    pub const fn summary(self) -> &'static str {
        match self {
            Self::CorrectPath => "Select an existing absolute Runtime Home path",
            Self::InitializeRegistry => "Initialize or restore the Runtime Home Registry",
            Self::RepairPermissions => "Repair Runtime Home ownership and permissions",
            Self::MoveToSupportedFilesystem => {
                "Move the Runtime Home to a supported filesystem boundary"
            }
            Self::SeparateBoundaries => {
                "Correct Runtime Home, project, and repository path boundaries"
            }
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct RuntimeHomeDiagnosticFacts {
    pub observed_state: Option<&'static str>,
    pub path_role: Option<&'static str>,
    pub boundary_violation: Option<&'static str>,
    pub io_error_kind: Option<&'static str>,
}

impl DiagnosticFactSource for RuntimeHomeDiagnosticFacts {}

#[derive(Serialize)]
struct ProjectedRuntimeHomeDiagnosticFacts<'a> {
    summary: &'static str,
    observed_state: Option<&'static str>,
    path_role: Option<&'static str>,
    boundary_violation: Option<&'static str>,
    io_error_kind: Option<&'static str>,
    #[serde(skip)]
    _source: std::marker::PhantomData<&'a RuntimeHomeDiagnosticFacts>,
}

impl DiagnosticFactSource for ProjectedRuntimeHomeDiagnosticFacts<'_> {}

/// Builds one shared Runtime Home finding from a typed Runtime Home diagnostic.
pub fn runtime_home_diagnostic_finding(
    diagnostic: RuntimeHomeDiagnostic,
    finding_id: impl Into<String>,
    facts: &RuntimeHomeDiagnosticFacts,
    observed_at: UtcTimestamp,
) -> Result<DiagnosticFinding, DiagnosticError> {
    let action = diagnostic.action();
    DiagnosticFinding::try_new(
        DiagnosticFindingId::parse(finding_id)?,
        DiagnosticCode::parse(diagnostic.code())?,
        DiagnosticDomain::parse("runtime_home")?,
        DiagnosticStage::parse("runtime_home_resolution")?,
        DiagnosticSeverity::Error,
        DiagnosticSource::parse("store_runtime_home")?,
        DiagnosticSubject::try_new("runtime_home", "selected")?,
        DiagnosticFacts::project(&ProjectedRuntimeHomeDiagnosticFacts {
            summary: diagnostic.summary(),
            observed_state: facts.observed_state,
            path_role: facts.path_role,
            boundary_violation: facts.boundary_violation,
            io_error_kind: facts.io_error_kind,
            _source: std::marker::PhantomData,
        })?,
        observed_at,
    )?
    .with_actions(vec![DiagnosticAction::try_new(
        DiagnosticCode::parse(action.code())?,
        action.summary(),
    )?])
}

/// Closed Store diagnostic vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreDiagnostic {
    SqliteReadonly,
    SqliteBusy,
    SqliteLocked,
    SchemaMismatch,
    IntegrityOrCorruptionFailure,
    MissingRecord,
    TransactionFailure,
    SerializationFailure,
    ConstraintViolation,
    Unexpected,
}

impl StoreDiagnostic {
    pub const ALL: [Self; 10] = [
        Self::SqliteReadonly,
        Self::SqliteBusy,
        Self::SqliteLocked,
        Self::SchemaMismatch,
        Self::IntegrityOrCorruptionFailure,
        Self::MissingRecord,
        Self::TransactionFailure,
        Self::SerializationFailure,
        Self::ConstraintViolation,
        Self::Unexpected,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::SqliteReadonly => "store.sqlite.readonly",
            Self::SqliteBusy => "store.sqlite.busy",
            Self::SqliteLocked => "store.sqlite.locked",
            Self::SchemaMismatch => "store.schema.mismatch",
            Self::IntegrityOrCorruptionFailure => "store.integrity.corruption_failure",
            Self::MissingRecord => "store.record.missing",
            Self::TransactionFailure => "store.transaction.failed",
            Self::SerializationFailure => "store.serialization.failed",
            Self::ConstraintViolation => "store.constraint.violation",
            Self::Unexpected => volicord_types::INTERNAL_UNEXPECTED_FAILURE_CODE,
        }
    }

    pub const fn summary(self) -> &'static str {
        match self {
            Self::SqliteReadonly => "SQLite rejected a required write as read-only",
            Self::SqliteBusy => "SQLite reported that the database is busy",
            Self::SqliteLocked => "SQLite reported that the database is locked",
            Self::SchemaMismatch => "The database schema does not match this build",
            Self::IntegrityOrCorruptionFailure => {
                "SQLite or a typed stored record failed integrity validation"
            }
            Self::MissingRecord => "A required stored record is missing",
            Self::TransactionFailure => "A SQLite transaction failed",
            Self::SerializationFailure => "A typed stored value could not be encoded or decoded",
            Self::ConstraintViolation => "A SQLite constraint was violated",
            Self::Unexpected => "An unexpected Store failure occurred",
        }
    }

    pub const fn action(self) -> Option<StoreRecommendedAction> {
        match self {
            Self::SqliteReadonly => Some(StoreRecommendedAction::RepairWriteAccess),
            Self::SqliteBusy | Self::SqliteLocked => {
                Some(StoreRecommendedAction::FreeLockedDatabase)
            }
            Self::SchemaMismatch => Some(StoreRecommendedAction::RepairSchema),
            Self::IntegrityOrCorruptionFailure => Some(StoreRecommendedAction::RestoreDatabase),
            Self::MissingRecord => Some(StoreRecommendedAction::RestoreMissingRecord),
            Self::TransactionFailure => Some(StoreRecommendedAction::RetryTransaction),
            Self::SerializationFailure => Some(StoreRecommendedAction::RepairStoredRecord),
            Self::ConstraintViolation => Some(StoreRecommendedAction::CorrectConstraintInput),
            Self::Unexpected => None,
        }
    }

    /// Classifies Store failures without inspecting any rendered error text.
    pub fn from_error(error: &StoreError) -> Self {
        match error {
            StoreError::Sqlite(error) => Self::from_sqlite(error),
            StoreError::NotFound { .. } => Self::MissingRecord,
            StoreError::CorruptStoredJson { .. }
            | StoreError::CorruptOwnerStateJson { .. }
            | StoreError::CorruptOwnerStateValue { .. }
            | StoreError::CorruptStoredValue { .. } => Self::SerializationFailure,
            StoreError::UnsupportedStorageProfile { .. }
            | StoreError::SchemaInvariant { .. }
            | StoreError::RuntimeHomeSchemaMismatch(_) => Self::SchemaMismatch,
            StoreError::RuntimeHomeCorruption(_) => Self::IntegrityOrCorruptionFailure,
            StoreError::Io(_)
            | StoreError::InvalidInput { .. }
            | StoreError::UnsupportedPlatformEnvironment { .. }
            | StoreError::PlatformEnvironmentUnavailable { .. }
            | StoreError::InvalidProjectRegistration { .. }
            | StoreError::Conflict { .. } => Self::Unexpected,
            StoreError::RuntimeHomePublicationConfirmation(_) => Self::TransactionFailure,
        }
    }

    fn from_sqlite(error: &rusqlite::Error) -> Self {
        match error {
            rusqlite::Error::SqliteFailure(sqlite, _) => match sqlite.code {
                SqliteErrorCode::ReadOnly => Self::SqliteReadonly,
                SqliteErrorCode::DatabaseBusy => Self::SqliteBusy,
                SqliteErrorCode::DatabaseLocked => Self::SqliteLocked,
                SqliteErrorCode::SchemaChanged => Self::SchemaMismatch,
                SqliteErrorCode::DatabaseCorrupt | SqliteErrorCode::NotADatabase => {
                    Self::IntegrityOrCorruptionFailure
                }
                SqliteErrorCode::ConstraintViolation => Self::ConstraintViolation,
                SqliteErrorCode::OperationAborted | SqliteErrorCode::OperationInterrupted => {
                    Self::TransactionFailure
                }
                _ => Self::Unexpected,
            },
            rusqlite::Error::FromSqlConversionFailure(_, _, _)
            | rusqlite::Error::IntegralValueOutOfRange(_, _)
            | rusqlite::Error::Utf8Error(_)
            | rusqlite::Error::InvalidColumnType(_, _, _) => Self::SerializationFailure,
            rusqlite::Error::QueryReturnedNoRows => Self::MissingRecord,
            _ => Self::Unexpected,
        }
    }
}

/// Closed recommended actions for Store findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreRecommendedAction {
    RepairWriteAccess,
    FreeLockedDatabase,
    RepairSchema,
    RestoreDatabase,
    RestoreMissingRecord,
    RetryTransaction,
    RepairStoredRecord,
    CorrectConstraintInput,
}

impl StoreRecommendedAction {
    pub const fn code(self) -> &'static str {
        match self {
            Self::RepairWriteAccess => "action.store.repair_write_access",
            Self::FreeLockedDatabase => "action.store.free_locked_database",
            Self::RepairSchema => "action.store.repair_schema",
            Self::RestoreDatabase => "action.store.restore_database",
            Self::RestoreMissingRecord => "action.store.restore_missing_record",
            Self::RetryTransaction => "action.store.retry_transaction",
            Self::RepairStoredRecord => "action.store.repair_stored_record",
            Self::CorrectConstraintInput => "action.store.correct_constraint_input",
        }
    }

    pub const fn summary(self) -> &'static str {
        match self {
            Self::RepairWriteAccess => "Repair database ownership or write permissions",
            Self::FreeLockedDatabase => "Finish or stop the process holding the database lock",
            Self::RepairSchema => "Use a compatible build or explicitly repair the schema",
            Self::RestoreDatabase => "Restore a verified database backup or reinitialize it",
            Self::RestoreMissingRecord => "Restore or recreate the required stored record",
            Self::RetryTransaction => "Retry the bounded transaction after its cause is resolved",
            Self::RepairStoredRecord => "Repair or restore the invalid typed stored record",
            Self::CorrectConstraintInput => "Correct the conflicting record input and retry",
        }
    }
}

/// Safe structured facts retained for one Store finding.
#[derive(Debug, Serialize)]
pub struct StoreDiagnosticFacts {
    pub database_kind: Option<&'static str>,
    pub observed_state: Option<&'static str>,
    pub sqlite_primary_code: Option<i32>,
    pub sqlite_extended_code: Option<i32>,
    pub constraint_kind: Option<&'static str>,
    pub entity: Option<&'static str>,
    pub field: Option<&'static str>,
    pub io_error_kind: Option<&'static str>,
}

impl DiagnosticFactSource for StoreDiagnosticFacts {}

#[derive(Serialize)]
struct ProjectedStoreDiagnosticFacts<'a> {
    summary: &'static str,
    database_kind: Option<&'static str>,
    observed_state: Option<&'static str>,
    sqlite_primary_code: Option<i32>,
    sqlite_extended_code: Option<i32>,
    constraint_kind: Option<&'static str>,
    entity: Option<&'static str>,
    field: Option<&'static str>,
    io_error_kind: Option<&'static str>,
    #[serde(skip)]
    _source: std::marker::PhantomData<&'a StoreDiagnosticFacts>,
}

impl DiagnosticFactSource for ProjectedStoreDiagnosticFacts<'_> {}

impl StoreDiagnosticFacts {
    pub fn from_error(error: &StoreError, database_kind: Option<&'static str>) -> Self {
        let mut facts = Self {
            database_kind: database_kind.or_else(|| error.classification().database_kind),
            observed_state: None,
            sqlite_primary_code: None,
            sqlite_extended_code: None,
            constraint_kind: None,
            entity: error.classification().entity,
            field: error.classification().field,
            io_error_kind: None,
        };
        match error {
            StoreError::Sqlite(rusqlite::Error::SqliteFailure(sqlite, _)) => {
                facts.sqlite_primary_code = Some(sqlite.extended_code & 0xff);
                facts.sqlite_extended_code = Some(sqlite.extended_code);
                facts.constraint_kind = sqlite_constraint_kind(sqlite.extended_code);
            }
            StoreError::Io(error) => facts.io_error_kind = Some(io_error_kind(error.kind())),
            _ => {}
        }
        facts
    }
}

/// Builds one shared structured Store finding from a typed Store error.
pub fn store_diagnostic_finding(
    error: &StoreError,
    finding_id: impl Into<String>,
    database_kind: Option<&'static str>,
    observed_at: UtcTimestamp,
) -> Result<DiagnosticFinding, DiagnosticError> {
    let diagnostic = StoreDiagnostic::from_error(error);
    store_diagnostic_finding_from_kind(
        diagnostic,
        finding_id,
        database_kind,
        &StoreDiagnosticFacts::from_error(error, database_kind),
        observed_at,
    )
}

/// Builds one shared Store finding from an already typed Store observation.
pub fn store_diagnostic_finding_from_kind(
    diagnostic: StoreDiagnostic,
    finding_id: impl Into<String>,
    database_kind: Option<&'static str>,
    facts: &StoreDiagnosticFacts,
    observed_at: UtcTimestamp,
) -> Result<DiagnosticFinding, DiagnosticError> {
    let actions = diagnostic
        .action()
        .map(diagnostic_action)
        .transpose()?
        .into_iter()
        .collect();
    DiagnosticFinding::try_new(
        DiagnosticFindingId::parse(finding_id)?,
        DiagnosticCode::parse(diagnostic.code())?,
        DiagnosticDomain::parse("store")?,
        DiagnosticStage::parse("storage")?,
        DiagnosticSeverity::Error,
        DiagnosticSource::parse("store")?,
        DiagnosticSubject::try_new("database", database_kind.unwrap_or("unknown"))?,
        DiagnosticFacts::project(&ProjectedStoreDiagnosticFacts {
            summary: diagnostic.summary(),
            database_kind: facts.database_kind,
            observed_state: facts.observed_state,
            sqlite_primary_code: facts.sqlite_primary_code,
            sqlite_extended_code: facts.sqlite_extended_code,
            constraint_kind: facts.constraint_kind,
            entity: facts.entity,
            field: facts.field,
            io_error_kind: facts.io_error_kind,
            _source: std::marker::PhantomData,
        })?,
        observed_at,
    )?
    .with_actions(actions)
}

fn diagnostic_action(action: StoreRecommendedAction) -> Result<DiagnosticAction, DiagnosticError> {
    DiagnosticAction::try_new(DiagnosticCode::parse(action.code())?, action.summary())
}

fn sqlite_constraint_kind(extended_code: i32) -> Option<&'static str> {
    match extended_code {
        ffi::SQLITE_CONSTRAINT_UNIQUE => Some("unique"),
        ffi::SQLITE_CONSTRAINT_PRIMARYKEY => Some("primary_key"),
        ffi::SQLITE_CONSTRAINT_FOREIGNKEY => Some("foreign_key"),
        ffi::SQLITE_CONSTRAINT_NOTNULL => Some("not_null"),
        ffi::SQLITE_CONSTRAINT_CHECK => Some("check"),
        _ => None,
    }
}

const fn io_error_kind(kind: io::ErrorKind) -> &'static str {
    match kind {
        io::ErrorKind::NotFound => "not_found",
        io::ErrorKind::PermissionDenied => "permission_denied",
        io::ErrorKind::AlreadyExists => "already_exists",
        io::ErrorKind::InvalidInput => "invalid_input",
        io::ErrorKind::InvalidData => "invalid_data",
        io::ErrorKind::Unsupported => "unsupported",
        io::ErrorKind::TimedOut => "timed_out",
        io::ErrorKind::Interrupted => "interrupted",
        io::ErrorKind::OutOfMemory => "out_of_memory",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::Duration};

    use rusqlite::ffi;
    use volicord_test_support::TempRuntimeHome;

    use super::*;
    use crate::runtime_home::{RuntimePathBoundaryViolation, RuntimePlatformDiagnostic};
    use crate::sqlite::{
        begin_immediate_transaction, open_project_state_database,
        open_project_state_database_read_only, open_read_only_database,
        open_registry_database_read_only,
    };

    fn sqlite_error(code: SqliteErrorCode, extended_code: i32, message: &str) -> StoreError {
        StoreError::Sqlite(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code,
                extended_code,
            },
            Some(message.to_owned()),
        ))
    }

    #[test]
    fn every_store_diagnostic_has_a_stable_code_and_deterministic_action() {
        assert_eq!(StoreDiagnostic::ALL.len(), 10);
        for diagnostic in StoreDiagnostic::ALL {
            assert!(diagnostic.code().contains('.'));
            assert_eq!(
                diagnostic.action().map(StoreRecommendedAction::code),
                diagnostic.action().map(StoreRecommendedAction::code)
            );
        }
    }

    #[test]
    fn every_runtime_home_diagnostic_has_a_stable_code_and_action() {
        assert_eq!(RuntimeHomeDiagnostic::ALL.len(), 7);
        for diagnostic in RuntimeHomeDiagnostic::ALL {
            assert!(diagnostic.code().starts_with("runtime_home."));
            assert!(diagnostic
                .action()
                .code()
                .starts_with("action.runtime_home."));
        }
    }

    #[test]
    fn sqlite_primary_and_extended_codes_drive_classification_not_english_prose() {
        for message in [
            "database is busy",
            "texto arbitrario",
            "not the SQLite message",
        ] {
            let error = sqlite_error(SqliteErrorCode::DatabaseBusy, ffi::SQLITE_BUSY, message);
            assert_eq!(
                StoreDiagnostic::from_error(&error),
                StoreDiagnostic::SqliteBusy
            );
        }
        let constraint = sqlite_error(
            SqliteErrorCode::ConstraintViolation,
            ffi::SQLITE_CONSTRAINT_FOREIGNKEY,
            "unrelated prose",
        );
        let facts = StoreDiagnosticFacts::from_error(&constraint, Some("registry"));
        assert_eq!(
            StoreDiagnostic::from_error(&constraint),
            StoreDiagnostic::ConstraintViolation
        );
        assert_eq!(facts.constraint_kind, Some("foreign_key"));
        assert_eq!(
            facts.sqlite_extended_code,
            Some(ffi::SQLITE_CONSTRAINT_FOREIGNKEY)
        );
    }

    #[test]
    fn readonly_busy_locked_corrupt_and_schema_scenarios_are_typed() {
        let scenarios = [
            (
                sqlite_error(SqliteErrorCode::ReadOnly, ffi::SQLITE_READONLY, "ignored"),
                StoreDiagnostic::SqliteReadonly,
            ),
            (
                sqlite_error(SqliteErrorCode::DatabaseBusy, ffi::SQLITE_BUSY, "ignored"),
                StoreDiagnostic::SqliteBusy,
            ),
            (
                sqlite_error(
                    SqliteErrorCode::DatabaseLocked,
                    ffi::SQLITE_LOCKED,
                    "ignored",
                ),
                StoreDiagnostic::SqliteLocked,
            ),
            (
                sqlite_error(
                    SqliteErrorCode::DatabaseCorrupt,
                    ffi::SQLITE_CORRUPT,
                    "ignored",
                ),
                StoreDiagnostic::IntegrityOrCorruptionFailure,
            ),
            (
                StoreError::SchemaInvariant {
                    database_kind: "registry",
                    detail: "ignored prose".to_owned(),
                },
                StoreDiagnostic::SchemaMismatch,
            ),
        ];
        for (error, expected) in scenarios {
            assert_eq!(StoreDiagnostic::from_error(&error), expected);
        }
    }

    #[test]
    fn missing_registry_and_runtime_boundary_scenarios_are_typed() {
        let missing = StoreError::NotFound {
            entity: "runtime_home",
            id: "/not/exposed/in/facts".to_owned(),
        };
        assert_eq!(
            RuntimeHomeDiagnostic::from_store_error(&missing),
            Some(RuntimeHomeDiagnostic::RegistryMissing)
        );

        let boundary = RuntimePathBoundaryError::BoundaryViolation {
            violation: RuntimePathBoundaryViolation::SamePath,
            runtime_home: PathBuf::from("/runtime"),
            repo_root: PathBuf::from("/runtime"),
            project_home: None,
        };
        assert_eq!(
            RuntimeHomeDiagnostic::from_path_boundary(&boundary),
            Some(RuntimeHomeDiagnostic::OwnerOrBoundaryMismatch)
        );

        let unsupported = RuntimePathBoundaryError::UnsupportedEnvironment {
            diagnostic: RuntimePlatformDiagnostic::Platform(
                volicord_platform_fs::PlatformBoundaryDiagnostic::UnsupportedFilesystemBoundary,
            ),
            reason: "prose-independent-reason",
            detail: "prose-independent-detail".to_owned(),
        };
        assert_eq!(
            RuntimeHomeDiagnostic::from_path_boundary(&unsupported),
            Some(RuntimeHomeDiagnostic::UnsupportedFilesystem)
        );
    }

    #[test]
    fn unexpected_store_fallback_uses_the_shared_stable_code() {
        let error = StoreError::InvalidInput {
            detail: "arbitrary internal detail".to_owned(),
        };
        let diagnostic = StoreDiagnostic::from_error(&error);
        assert_eq!(diagnostic, StoreDiagnostic::Unexpected);
        assert_eq!(
            diagnostic.code(),
            volicord_types::INTERNAL_UNEXPECTED_FAILURE_CODE
        );
        assert_eq!(diagnostic.action(), None);
    }

    #[test]
    fn real_readonly_and_busy_databases_use_sqlite_codes() -> Result<(), Box<dyn std::error::Error>>
    {
        let runtime_home = TempRuntimeHome::new("operational-readonly-busy")?;
        let path = runtime_home.project_state_db_path("PRJ-diagnostic");
        let mut first = open_project_state_database(&path)?;

        let readonly = open_read_only_database(&path)?;
        let readonly_error = readonly
            .execute("CREATE TABLE forbidden_write (id INTEGER)", [])
            .expect_err("read-only connection must reject writes");
        assert_eq!(
            StoreDiagnostic::from_error(&StoreError::from(readonly_error)),
            StoreDiagnostic::SqliteReadonly
        );

        let mut second = open_project_state_database(&path)?;
        first.busy_timeout(Duration::from_millis(0))?;
        second.busy_timeout(Duration::from_millis(0))?;
        let transaction = begin_immediate_transaction(&mut first)?;
        let busy_error = begin_immediate_transaction(&mut second)
            .expect_err("concurrent immediate writer must be busy or locked");
        assert!(matches!(
            StoreDiagnostic::from_error(&StoreError::from(busy_error)),
            StoreDiagnostic::SqliteBusy | StoreDiagnostic::SqliteLocked
        ));
        transaction.rollback()?;
        Ok(())
    }

    #[test]
    fn real_corrupt_schema_mismatch_and_missing_registry_are_typed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let missing = TempRuntimeHome::new("operational-missing-registry")?;
        let error = open_registry_database_read_only(missing.registry_db_path())
            .expect_err("missing Registry must fail closed");
        assert_eq!(
            RuntimeHomeDiagnostic::from_store_error(&error),
            Some(RuntimeHomeDiagnostic::RegistryMissing)
        );

        let corrupt = TempRuntimeHome::new("operational-corrupt")?;
        let corrupt_path = corrupt.project_state_db_path("PRJ-corrupt");
        fs::create_dir_all(corrupt_path.parent().expect("state parent"))?;
        fs::write(&corrupt_path, b"not a SQLite database")?;
        let error = open_project_state_database_read_only(&corrupt_path)
            .expect_err("corrupt database must fail closed");
        assert_eq!(
            StoreDiagnostic::from_error(&error),
            StoreDiagnostic::IntegrityOrCorruptionFailure
        );

        let mismatch = TempRuntimeHome::new("operational-schema-mismatch")?;
        let mismatch_path = mismatch.project_state_db_path("PRJ-mismatch");
        let conn = open_project_state_database(&mismatch_path)?;
        conn.execute("ALTER TABLE tasks ADD COLUMN unexpected TEXT", [])?;
        drop(conn);
        let error = open_project_state_database_read_only(&mismatch_path)
            .expect_err("physical schema mismatch must fail closed");
        assert_eq!(
            StoreDiagnostic::from_error(&error),
            StoreDiagnostic::SchemaMismatch
        );
        Ok(())
    }
}
