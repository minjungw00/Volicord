use std::{error::Error, fmt, io};

/// Store-layer result type.
pub type StoreResult<T> = Result<T, StoreError>;

/// Errors raised while opening, migrating, or validating store databases.
#[derive(Debug)]
pub enum StoreError {
    /// Filesystem error while preparing a runtime-home path.
    Io(io::Error),
    /// SQLite driver error.
    Sqlite(rusqlite::Error),
    /// A recorded migration row conflicts with the compiled migration catalog.
    MigrationConflict {
        database_kind: &'static str,
        version: i64,
        expected_name: &'static str,
        actual_name: String,
        expected_storage_profile: &'static str,
        actual_storage_profile: String,
    },
    /// A migrated database does not satisfy a required schema invariant.
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
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "filesystem error: {error}"),
            Self::Sqlite(error) => write!(formatter, "sqlite error: {error}"),
            Self::MigrationConflict {
                database_kind,
                version,
                expected_name,
                actual_name,
                expected_storage_profile,
                actual_storage_profile,
            } => write!(
                formatter,
                "migration conflict for {database_kind} version {version}: expected {expected_name}/{expected_storage_profile}, found {actual_name}/{actual_storage_profile}"
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
            Self::MigrationConflict { .. } | Self::SchemaInvariant { .. } => None,
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
