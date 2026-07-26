use rusqlite::params;

use super::facade::CoreProjectStore;
use crate::{StoreError, StoreResult};

impl CoreProjectStore<'_> {
    /// Returns whether a committed event id already exists in this project.
    pub fn event_id_exists(&self, event_id: &str) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT COUNT(*)
                   FROM authority_events
                  WHERE project_id = ?1
                    AND event_id = ?2",
                params![self.project.project_id, event_id],
                |row| Ok(row.get::<_, i64>(0)? > 0),
            )
            .map_err(StoreError::from)
    }
}

#[cfg(test)]
mod behavior_tests;
