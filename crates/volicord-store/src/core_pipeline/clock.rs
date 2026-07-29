use rusqlite::{params, Connection, OptionalExtension, Transaction};
use volicord_types::values::UtcTimestamp;

use super::facade::CoreProjectStore;
use crate::{StoreError, StoreResult};

impl CoreProjectStore<'_> {
    /// Returns the monotonic Core current UTC clock for this Store handle.
    pub fn current_timestamp(&self) -> StoreResult<UtcTimestamp> {
        let local_floor = self.last_clock_sample.borrow().clone();
        let timestamp = project_current_utc_timestamp_for_conn(
            &self.conn,
            &self.project.project_id,
            local_floor.as_ref(),
        )?;
        *self.last_clock_sample.borrow_mut() = Some(timestamp.clone());
        Ok(timestamp)
    }

    /// Returns the persisted project clock floor combined with samples already
    /// accepted on this Store handle, without sampling SQLite wall-clock time.
    pub fn current_clock_floor(&self) -> StoreResult<UtcTimestamp> {
        let persisted = self.project_state()?;
        let persisted = persisted.updated_at;
        persisted
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| {
                StoreError::corrupt_owner_state_value(
                    "project_state",
                    &self.project.project_id,
                    "updated_at",
                )
            })?;
        let local = self.last_clock_sample.borrow().as_ref().cloned();
        if local
            .as_ref()
            .is_some_and(|timestamp| timestamp.ensure_canonical_rfc3339_representable().is_err())
        {
            return Err(StoreError::SchemaInvariant {
                database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
                detail: "Core Store handle clock sample is outside the canonical RFC 3339 range"
                    .to_owned(),
            });
        }
        Ok(local.map_or(persisted.clone(), |local| std::cmp::max(persisted, local)))
    }

    /// Carries an injected Core clock sample through this Store handle's next commit.
    pub fn remember_clock_sample(&self, sample: &UtcTimestamp) {
        let current = self.last_clock_sample.borrow().clone();
        if current.as_ref().is_none_or(|current| sample > current) {
            *self.last_clock_sample.borrow_mut() = Some(sample.clone());
        }
    }
}

pub(crate) fn project_current_utc_timestamp_for_conn(
    conn: &Connection,
    project_id: &str,
    local_floor: Option<&UtcTimestamp>,
) -> StoreResult<UtcTimestamp> {
    let (sqlite_now, persisted_floor): (String, String) = conn
        .query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), updated_at
               FROM project_state
              WHERE project_id = ?1",
            params![project_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project_state",
            id: project_id.to_owned(),
        })?;
    let sqlite_now = UtcTimestamp::parse(&sqlite_now).map_err(|_| StoreError::SchemaInvariant {
        database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
        detail: "SQLite returned an invalid Core current UTC timestamp".to_owned(),
    })?;
    sqlite_now
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::SchemaInvariant {
            database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
            detail: "SQLite returned an out-of-range Core current UTC timestamp".to_owned(),
        })?;
    let persisted_floor = UtcTimestamp::parse(&persisted_floor).map_err(|_| {
        StoreError::corrupt_owner_state_value("project_state", project_id, "updated_at")
    })?;
    persisted_floor
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| {
            StoreError::corrupt_owner_state_value("project_state", project_id, "updated_at")
        })?;
    if let Some(local_floor) = local_floor {
        local_floor
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| StoreError::SchemaInvariant {
                database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
                detail: "Core local UTC floor is outside the canonical timestamp range".to_owned(),
            })?;
    }
    Ok([
        Some(sqlite_now),
        Some(persisted_floor),
        local_floor.cloned(),
    ]
    .into_iter()
    .flatten()
    .max()
    .expect("Core current UTC clock always has SQLite and persisted samples"))
}

pub(crate) fn advance_project_utc_floor_tx(
    tx: &Transaction<'_>,
    project_id: &str,
    sample: &UtcTimestamp,
) -> StoreResult<UtcTimestamp> {
    sample
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::SchemaInvariant {
            database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
            detail: "Core UTC floor sample is outside the canonical timestamp range".to_owned(),
        })?;
    let persisted_floor = tx
        .query_row(
            "SELECT updated_at FROM project_state WHERE project_id = ?1",
            params![project_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project_state",
            id: project_id.to_owned(),
        })?;
    let persisted_floor = UtcTimestamp::parse(&persisted_floor).map_err(|_| {
        StoreError::corrupt_owner_state_value("project_state", project_id, "updated_at")
    })?;
    persisted_floor
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| {
            StoreError::corrupt_owner_state_value("project_state", project_id, "updated_at")
        })?;
    let floor = std::cmp::max(persisted_floor, sample.clone());
    let changed = tx.execute(
        "UPDATE project_state SET updated_at = ?2 WHERE project_id = ?1",
        params![project_id, floor.to_string()],
    )?;
    if changed != 1 {
        return Err(StoreError::SchemaInvariant {
            database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
            detail: "Core current UTC floor update changed no rows".to_owned(),
        });
    }
    Ok(floor)
}

#[cfg(test)]
mod behavior_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_utc_floor_never_moves_backward() {
        let mut connection = Connection::open_in_memory().expect("in-memory database must open");
        connection
            .execute_batch(
                "CREATE TABLE project_state (
                    project_id TEXT PRIMARY KEY,
                    updated_at TEXT NOT NULL
                 );
                 INSERT INTO project_state (project_id, updated_at)
                 VALUES ('project', '2026-01-02T00:00:00Z');",
            )
            .expect("clock fixture must initialize");
        let tx = connection.transaction().expect("transaction must begin");
        let earlier =
            UtcTimestamp::parse("2026-01-01T00:00:00Z").expect("timestamp must be canonical");

        let floor = advance_project_utc_floor_tx(&tx, "project", &earlier)
            .expect("earlier sample must preserve the floor");

        assert_eq!(floor.to_string(), "2026-01-02T00:00:00Z");
    }
}
