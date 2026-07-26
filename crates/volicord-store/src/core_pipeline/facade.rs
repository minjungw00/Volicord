use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};

use rusqlite::Connection;
use volicord_types::values::UtcTimestamp;

use crate::{
    bootstrap::ProjectRecord, CanonicalRuntimeHomePath, RuntimeHomeMutationContext, StoreError,
    StoreResult,
};

/// Project-local store handle used by the Core request pipeline.
#[derive(Debug)]
pub struct CoreProjectStore<'mutation> {
    pub(crate) runtime_home: PathBuf,
    pub(crate) canonical_runtime_home: Option<CanonicalRuntimeHomePath>,
    pub(crate) project: ProjectRecord,
    pub(crate) conn: Connection,
    pub(crate) writable: bool,
    pub(crate) mutation_context: Option<RuntimeHomeMutationContext<'mutation>>,
    pub(crate) last_clock_sample: RefCell<Option<UtcTimestamp>>,
}

impl<'mutation> CoreProjectStore<'mutation> {
    /// Returns the live Runtime Home mutation capability retained by a mutation store.
    pub fn mutation_context(&self) -> Option<&RuntimeHomeMutationContext<'mutation>> {
        self.mutation_context.as_ref()
    }

    pub(crate) fn require_mutation_context(
        &self,
    ) -> StoreResult<&RuntimeHomeMutationContext<'mutation>> {
        let context = self
            .mutation_context
            .as_ref()
            .ok_or_else(|| StoreError::InvalidInput {
                detail: "Core project mutation requires a live Runtime Home mutation context"
                    .to_owned(),
            })?;
        if self.canonical_runtime_home.as_ref() != Some(context.runtime_home()) {
            return Err(StoreError::InvalidInput {
                detail:
                    "Core project mutation Store does not retain the admitted Runtime Home identity"
                        .to_owned(),
            });
        }
        Ok(context)
    }

    /// Runs related read-only lookups against one SQLite snapshot.
    ///
    /// The deferred transaction pins its snapshot at the closure's first read,
    /// so callers can attach one project-state version to a compound projection
    /// without mixing rows from a concurrent authority commit.
    pub fn with_read_snapshot<T>(
        &self,
        read: impl FnOnce(&Self) -> StoreResult<T>,
    ) -> StoreResult<T> {
        let transaction = self.conn.unchecked_transaction()?;
        let value = read(self)?;
        transaction.commit()?;
        Ok(value)
    }

    /// Returns the Runtime Home path that selected this project-local store.
    pub fn runtime_home(&self) -> &Path {
        &self.runtime_home
    }

    /// Returns the typed canonical Runtime Home retained by a mutation-capable Store.
    pub fn canonical_runtime_home(&self) -> Option<&CanonicalRuntimeHomePath> {
        self.canonical_runtime_home.as_ref()
    }

    /// Returns the registry project row that selected this project-local store.
    pub const fn project_record(&self) -> &ProjectRecord {
        &self.project
    }

    /// Returns whether this store handle was opened for write-capable Core work.
    pub const fn is_writable(&self) -> bool {
        self.writable
    }
}
