use std::{error::Error, fmt, io};

use rusqlite::{ffi, ErrorCode as SqliteErrorCode};

/// Store-layer result type.
pub type StoreResult<T> = Result<T, StoreError>;

/// Errors raised while opening, initializing, or validating store databases.
#[derive(Debug)]
pub enum StoreError {
    /// Filesystem error while preparing a runtime-home path.
    Io(io::Error),
    /// SQLite driver error.
    Sqlite(rusqlite::Error),
    /// Local administrative setup input is not valid for the storage record.
    InvalidInput { detail: String },
    /// The observed platform topology is outside the first-release contract.
    UnsupportedPlatformEnvironment {
        reason: &'static str,
        detail: String,
    },
    /// A required platform-topology observation could not be completed.
    PlatformEnvironmentUnavailable {
        reason: &'static str,
        detail: String,
    },
    /// A stored project registration is not valid for execution.
    InvalidProjectRegistration {
        project_id: String,
        field: &'static str,
        relationship: &'static str,
        detail: String,
    },
    /// A required local setup record was not found.
    NotFound { entity: &'static str, id: String },
    /// A local setup identity is already bound to incompatible facts.
    Conflict {
        entity: &'static str,
        id: String,
        detail: String,
    },
    /// Stored owner JSON could not be parsed or validated.
    CorruptStoredJson {
        database_kind: &'static str,
        field: &'static str,
    },
    /// Persisted typed owner JSON could not be decoded for an authority decision.
    CorruptOwnerStateJson {
        database_kind: &'static str,
        table: &'static str,
        record_ref: String,
        logical_column: &'static str,
    },
    /// A persisted typed owner value could not be decoded for an authority decision.
    CorruptOwnerStateValue {
        database_kind: &'static str,
        table: &'static str,
        record_ref: String,
        logical_column: &'static str,
    },
    /// A stored owner field has a value outside the owner-defined set.
    CorruptStoredValue {
        database_kind: &'static str,
        field: &'static str,
    },
    /// A database uses a storage profile that this build does not support.
    UnsupportedStorageProfile {
        database_kind: &'static str,
        actual_storage_profile: String,
        expected_storage_profile: &'static str,
    },
    /// A persisted external payload names no exact current boundary adapter.
    UnsupportedExternalContract {
        contract_id: String,
        reason: &'static str,
    },
    /// An opened database does not satisfy a required schema invariant.
    SchemaInvariant {
        database_kind: &'static str,
        detail: String,
    },
}

impl StoreError {
    pub(crate) fn schema_invariant(database_kind: &'static str, detail: impl Into<String>) -> Self {
        Self::SchemaInvariant {
            database_kind,
            detail: detail.into(),
        }
    }

    pub(crate) fn unsupported_storage_profile(
        database_kind: &'static str,
        actual_storage_profile: impl Into<String>,
        expected_storage_profile: &'static str,
    ) -> Self {
        Self::UnsupportedStorageProfile {
            database_kind,
            actual_storage_profile: actual_storage_profile.into(),
            expected_storage_profile,
        }
    }

    pub fn corrupt_stored_json(database_kind: &'static str, field: &'static str) -> Self {
        Self::CorruptStoredJson {
            database_kind,
            field,
        }
    }

    pub fn corrupt_owner_state_json(
        table: &'static str,
        record_ref: impl Into<String>,
        logical_column: &'static str,
    ) -> Self {
        Self::CorruptOwnerStateJson {
            database_kind: "project_state",
            table,
            record_ref: record_ref.into(),
            logical_column,
        }
    }

    pub fn corrupt_owner_state_value(
        table: &'static str,
        record_ref: impl Into<String>,
        logical_column: &'static str,
    ) -> Self {
        Self::CorruptOwnerStateValue {
            database_kind: "project_state",
            table,
            record_ref: record_ref.into(),
            logical_column,
        }
    }

    pub fn corrupt_stored_value(database_kind: &'static str, field: &'static str) -> Self {
        Self::CorruptStoredValue {
            database_kind,
            field,
        }
    }

    /// Classifies a store failure for safe public error routing.
    pub fn classification(&self) -> StoreFailureClassification {
        match self {
            Self::Io(_) => StoreFailureClassification {
                route: StoreFailureRoute::OperationalUnavailable,
                category: "runtime_home_io",
                retryable: true,
                database_kind: None,
                entity: None,
                field: None,
                owner_state_error: None,
            },
            Self::Sqlite(error) => sqlite_classification(error),
            Self::InvalidInput { .. } => StoreFailureClassification {
                route: StoreFailureRoute::OperationalUnavailable,
                category: "invalid_store_input",
                retryable: false,
                database_kind: None,
                entity: None,
                field: None,
                owner_state_error: None,
            },
            Self::UnsupportedPlatformEnvironment { reason, .. } => StoreFailureClassification {
                route: StoreFailureRoute::UnsupportedContract,
                category: reason,
                retryable: false,
                database_kind: None,
                entity: None,
                field: None,
                owner_state_error: None,
            },
            Self::PlatformEnvironmentUnavailable { reason, .. } => StoreFailureClassification {
                route: StoreFailureRoute::OperationalUnavailable,
                category: reason,
                retryable: true,
                database_kind: None,
                entity: None,
                field: None,
                owner_state_error: None,
            },
            Self::InvalidProjectRegistration { field, .. } => StoreFailureClassification {
                route: StoreFailureRoute::OperationalUnavailable,
                category: "invalid_project_registration",
                retryable: false,
                database_kind: Some("registry"),
                entity: Some("project"),
                field: Some(field),
                owner_state_error: None,
            },
            Self::NotFound { entity, .. } => {
                let (route, category, retryable, database_kind) = match *entity {
                    "project" => (
                        StoreFailureRoute::InvocationContextMismatch,
                        "project_binding_missing",
                        false,
                        None,
                    ),
                    "project_state_database" => (
                        StoreFailureRoute::OperationalUnavailable,
                        "project_state_database_missing",
                        true,
                        Some("project_state"),
                    ),
                    "project_state" => (
                        StoreFailureRoute::OperationalUnavailable,
                        "project_state_missing",
                        true,
                        Some("project_state"),
                    ),
                    "runtime_home" => (
                        StoreFailureRoute::OperationalUnavailable,
                        "runtime_home_missing",
                        true,
                        Some("registry"),
                    ),
                    _ => (
                        StoreFailureRoute::OperationalUnavailable,
                        "store_record_missing",
                        true,
                        None,
                    ),
                };
                StoreFailureClassification {
                    route,
                    category,
                    retryable,
                    database_kind,
                    entity: Some(entity),
                    field: None,
                    owner_state_error: None,
                }
            }
            Self::Conflict { entity, .. } => StoreFailureClassification {
                route: StoreFailureRoute::OperationalUnavailable,
                category: "store_conflict",
                retryable: false,
                database_kind: Some("registry"),
                entity: Some(entity),
                field: None,
                owner_state_error: None,
            },
            Self::CorruptStoredJson {
                database_kind,
                field,
            } => StoreFailureClassification {
                route: StoreFailureRoute::PersistedDataCorrupt,
                category: "corrupt_stored_json",
                retryable: false,
                database_kind: Some(database_kind),
                entity: None,
                field: Some(field),
                owner_state_error: None,
            },
            Self::CorruptOwnerStateJson {
                database_kind,
                table,
                record_ref,
                logical_column,
            } => StoreFailureClassification {
                route: StoreFailureRoute::PersistedDataCorrupt,
                category: "corrupt_stored_json",
                retryable: false,
                database_kind: Some(database_kind),
                entity: None,
                field: None,
                owner_state_error: Some(OwnerStateFailureDetails {
                    table,
                    record_ref: record_ref.clone(),
                    logical_column,
                    corruption_category: "corrupt_stored_json",
                }),
            },
            Self::CorruptOwnerStateValue {
                database_kind,
                table,
                record_ref,
                logical_column,
            } => StoreFailureClassification {
                route: StoreFailureRoute::PersistedDataCorrupt,
                category: "corrupt_stored_value",
                retryable: false,
                database_kind: Some(database_kind),
                entity: None,
                field: None,
                owner_state_error: Some(OwnerStateFailureDetails {
                    table,
                    record_ref: record_ref.clone(),
                    logical_column,
                    corruption_category: "corrupt_stored_value",
                }),
            },
            Self::CorruptStoredValue {
                database_kind,
                field,
            } => StoreFailureClassification {
                route: StoreFailureRoute::PersistedDataCorrupt,
                category: "corrupt_stored_value",
                retryable: false,
                database_kind: Some(database_kind),
                entity: None,
                field: Some(field),
                owner_state_error: None,
            },
            Self::UnsupportedStorageProfile { database_kind, .. } => StoreFailureClassification {
                route: StoreFailureRoute::UnsupportedContract,
                category: "unsupported_storage_profile",
                retryable: false,
                database_kind: Some(database_kind),
                entity: None,
                field: Some("storage_profile"),
                owner_state_error: None,
            },
            Self::UnsupportedExternalContract { .. } => StoreFailureClassification {
                route: StoreFailureRoute::UnsupportedContract,
                category: "unsupported_external_contract",
                retryable: false,
                database_kind: Some("registry"),
                entity: None,
                field: None,
                owner_state_error: None,
            },
            Self::SchemaInvariant { database_kind, .. } => StoreFailureClassification {
                route: StoreFailureRoute::PersistedDataCorrupt,
                category: "schema_invariant",
                retryable: false,
                database_kind: Some(database_kind),
                entity: None,
                field: None,
                owner_state_error: None,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreFailureRoute {
    OperationalUnavailable,
    InvocationContextMismatch,
    PersistedDataCorrupt,
    UnsupportedContract,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreFailureClassification {
    pub route: StoreFailureRoute,
    pub category: &'static str,
    pub retryable: bool,
    pub database_kind: Option<&'static str>,
    pub entity: Option<&'static str>,
    pub field: Option<&'static str>,
    pub owner_state_error: Option<OwnerStateFailureDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerStateFailureDetails {
    pub table: &'static str,
    pub record_ref: String,
    pub logical_column: &'static str,
    pub corruption_category: &'static str,
}

fn sqlite_classification(error: &rusqlite::Error) -> StoreFailureClassification {
    let (route, category, retryable) = match error {
        rusqlite::Error::SqliteFailure(sqlite_error, _) => match sqlite_error.code {
            SqliteErrorCode::ConstraintViolation => (
                StoreFailureRoute::OperationalUnavailable,
                sqlite_constraint_category(sqlite_error.extended_code),
                false,
            ),
            SqliteErrorCode::CannotOpen => (
                StoreFailureRoute::OperationalUnavailable,
                "database_open_failed",
                true,
            ),
            SqliteErrorCode::DatabaseCorrupt | SqliteErrorCode::NotADatabase => (
                StoreFailureRoute::PersistedDataCorrupt,
                "database_corrupt",
                false,
            ),
            SqliteErrorCode::DatabaseBusy | SqliteErrorCode::DatabaseLocked => (
                StoreFailureRoute::OperationalUnavailable,
                "database_locked",
                true,
            ),
            SqliteErrorCode::ReadOnly | SqliteErrorCode::PermissionDenied => (
                StoreFailureRoute::OperationalUnavailable,
                "database_access_denied",
                false,
            ),
            SqliteErrorCode::SystemIoFailure | SqliteErrorCode::DiskFull => (
                StoreFailureRoute::OperationalUnavailable,
                "database_io",
                true,
            ),
            SqliteErrorCode::SchemaChanged => (
                StoreFailureRoute::OperationalUnavailable,
                "database_schema_changed",
                true,
            ),
            _ => (
                StoreFailureRoute::OperationalUnavailable,
                "sqlite_driver_error",
                true,
            ),
        },
        rusqlite::Error::FromSqlConversionFailure(_, _, _)
        | rusqlite::Error::IntegralValueOutOfRange(_, _)
        | rusqlite::Error::Utf8Error(_)
        | rusqlite::Error::InvalidColumnType(_, _, _) => (
            StoreFailureRoute::PersistedDataCorrupt,
            "stored_value_decode_failed",
            false,
        ),
        rusqlite::Error::QueryReturnedNoRows => (
            StoreFailureRoute::OperationalUnavailable,
            "store_record_missing",
            true,
        ),
        rusqlite::Error::InvalidPath(_) => (
            StoreFailureRoute::OperationalUnavailable,
            "runtime_path_invalid",
            false,
        ),
        rusqlite::Error::InvalidColumnIndex(_)
        | rusqlite::Error::InvalidColumnName(_)
        | rusqlite::Error::StatementChangedRows(_)
        | rusqlite::Error::InvalidQuery
        | rusqlite::Error::MultipleStatement
        | rusqlite::Error::InvalidParameterCount(_, _)
        | rusqlite::Error::InvalidParameterName(_)
        | rusqlite::Error::ExecuteReturnedResults => (
            StoreFailureRoute::OperationalUnavailable,
            "store_programming_error",
            false,
        ),
        _ => (
            StoreFailureRoute::OperationalUnavailable,
            "sqlite_driver_error",
            true,
        ),
    };

    StoreFailureClassification {
        route,
        category,
        retryable,
        database_kind: None,
        entity: None,
        field: None,
        owner_state_error: None,
    }
}

fn sqlite_constraint_category(extended_code: i32) -> &'static str {
    match extended_code {
        ffi::SQLITE_CONSTRAINT_UNIQUE => "constraint_unique",
        ffi::SQLITE_CONSTRAINT_PRIMARYKEY => "constraint_primary_key",
        ffi::SQLITE_CONSTRAINT_FOREIGNKEY => "constraint_foreign_key",
        ffi::SQLITE_CONSTRAINT_NOTNULL => "constraint_not_null",
        ffi::SQLITE_CONSTRAINT_CHECK => "constraint_check",
        _ => "constraint_violation",
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "filesystem error: {error}"),
            Self::Sqlite(error) => write!(formatter, "sqlite error: {error}"),
            Self::InvalidInput { detail } => write!(formatter, "invalid setup input: {detail}"),
            Self::UnsupportedPlatformEnvironment { reason, detail }
            | Self::PlatformEnvironmentUnavailable { reason, detail } => {
                write!(formatter, "{reason}: {detail}")
            }
            Self::InvalidProjectRegistration {
                project_id,
                field,
                relationship,
                detail,
            } => {
                let subject = match *field {
                    "project_id" => "registered project id is invalid",
                    "repo_root" => "registered Product Repository conflicts with Runtime Home",
                    "state_db_path" => {
                        "registered project state database path conflicts with project_home"
                    }
                    _ => "registered project paths conflict with Runtime Home or Product Repository",
                };
                write!(
                    formatter,
                    "{subject} for project {project_id}: field {field}, relationship {relationship}: {detail}"
                )
            }
            Self::NotFound { entity, id } => write!(formatter, "{entity} not found: {id}"),
            Self::Conflict { entity, id, detail } => {
                write!(formatter, "{entity} conflict for {id}: {detail}")
            }
            Self::CorruptStoredJson {
                database_kind,
                field,
            } => write!(
                formatter,
                "stored JSON field {field} is invalid in {database_kind}"
            ),
            Self::CorruptOwnerStateJson {
                database_kind,
                table,
                record_ref,
                logical_column,
            } => write!(
                formatter,
                "stored owner JSON {table}.{logical_column} for {record_ref} is invalid in {database_kind}"
            ),
            Self::CorruptOwnerStateValue {
                database_kind,
                table,
                record_ref,
                logical_column,
            } => write!(
                formatter,
                "stored owner value {table}.{logical_column} for {record_ref} is invalid in {database_kind}"
            ),
            Self::CorruptStoredValue {
                database_kind,
                field,
            } => write!(
                formatter,
                "stored field {field} has an unsupported value in {database_kind}"
            ),
            Self::UnsupportedStorageProfile {
                database_kind,
                actual_storage_profile,
                expected_storage_profile,
            } => write!(
                formatter,
                "unsupported storage profile for {database_kind}: found {actual_storage_profile}, expected {expected_storage_profile}; explicitly reinitialize the Runtime Home"
            ),
            Self::UnsupportedExternalContract {
                contract_id,
                reason,
            } => write!(
                formatter,
                "{reason}: unsupported external contract {contract_id}"
            ),
            Self::SchemaInvariant {
                database_kind,
                detail,
            } => write!(
                formatter,
                "schema invariant failed for {database_kind}: {detail}"
            ),
        }
    }
}

impl Error for StoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Sqlite(error) => Some(error),
            Self::InvalidInput { .. }
            | Self::UnsupportedPlatformEnvironment { .. }
            | Self::PlatformEnvironmentUnavailable { .. }
            | Self::InvalidProjectRegistration { .. }
            | Self::NotFound { .. }
            | Self::Conflict { .. }
            | Self::CorruptStoredJson { .. }
            | Self::CorruptOwnerStateJson { .. }
            | Self::CorruptOwnerStateValue { .. }
            | Self::CorruptStoredValue { .. }
            | Self::UnsupportedStorageProfile { .. }
            | Self::UnsupportedExternalContract { .. }
            | Self::SchemaInvariant { .. } => None,
        }
    }
}

impl From<io::Error> for StoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

#[cfg(test)]
mod tests {
    use super::{StoreError, StoreFailureRoute};

    #[test]
    fn persisted_contract_violations_have_a_corrupt_route() {
        for error in [
            StoreError::corrupt_stored_json("project_state", "typed_json"),
            StoreError::corrupt_stored_value("registry", "typed_value"),
            StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "fixture".to_owned(),
            },
        ] {
            assert_eq!(
                error.classification().route,
                StoreFailureRoute::PersistedDataCorrupt
            );
        }
    }

    #[test]
    fn unknown_storage_profile_has_an_unsupported_contract_route() {
        let error = StoreError::unsupported_storage_profile(
            "registry",
            "unknown-contract",
            "current-contract",
        );

        assert_eq!(
            error.classification().route,
            StoreFailureRoute::UnsupportedContract
        );
    }
}
