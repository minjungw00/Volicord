use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::ids::TaskId;

use super::facade::CoreProjectStore;
use crate::{StoreError, StoreResult};

const CHANGE_UNIT_RECORD_COLUMNS: &str = "
    project_id, change_unit_id, task_id, status, is_current,
    basis_state_version, scope_summary_json, bounded_paths_json,
    write_basis_json, effect_contract_json, lifecycle_json";

/// Current Change Unit row data needed by Core method implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeUnitRecord {
    pub project_id: String,
    pub change_unit_id: String,
    pub task_id: String,
    pub status: String,
    pub is_current: bool,
    pub basis_state_version: u64,
    pub scope_summary_json: String,
    pub bounded_paths_json: String,
    pub write_basis_json: String,
    pub effect_contract_json: String,
    pub lifecycle_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawChangeUnitRecord {
    project_id: String,
    change_unit_id: String,
    task_id: String,
    status: String,
    is_current: i64,
    basis_state_version: Option<i64>,
    scope_summary_json: String,
    bounded_paths_json: String,
    write_basis_json: String,
    effect_contract_json: String,
    lifecycle_json: String,
}

impl CoreProjectStore<'_> {
    /// Reads the current active Change Unit row for a Task.
    pub fn current_change_unit(&self, task_id: &TaskId) -> StoreResult<Option<ChangeUnitRecord>> {
        current_change_unit(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Reads one Change Unit row by exact Task and Change Unit identity.
    pub fn change_unit_record(
        &self,
        task_id: &TaskId,
        change_unit_id: &str,
    ) -> StoreResult<Option<ChangeUnitRecord>> {
        change_unit_record(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            change_unit_id,
        )
    }

    /// Returns whether a Change Unit id already exists in this project.
    pub fn change_unit_id_exists(&self, change_unit_id: &str) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT COUNT(*)
                   FROM change_units
                  WHERE project_id = ?1
                    AND change_unit_id = ?2",
                params![self.project.project_id, change_unit_id],
                |row| Ok(row.get::<_, i64>(0)? > 0),
            )
            .map_err(StoreError::from)
    }
}

fn current_change_unit(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Option<ChangeUnitRecord>> {
    let sql = format!(
        "SELECT {CHANGE_UNIT_RECORD_COLUMNS}
           FROM change_units
          WHERE project_id = ?1
            AND task_id = ?2
            AND status = 'active'
            AND is_current = 1"
    );
    conn.query_row(
        &sql,
        params![project_id, task_id],
        raw_change_unit_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)?
    .map(validate_decoded_change_unit_record)
    .transpose()
}

fn change_unit_record(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    change_unit_id: &str,
) -> StoreResult<Option<ChangeUnitRecord>> {
    let sql = format!(
        "SELECT {CHANGE_UNIT_RECORD_COLUMNS}
           FROM change_units
          WHERE project_id = ?1
            AND task_id = ?2
            AND change_unit_id = ?3"
    );
    conn.query_row(
        &sql,
        params![project_id, task_id, change_unit_id],
        raw_change_unit_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)?
    .map(validate_decoded_change_unit_record)
    .transpose()
}

fn raw_change_unit_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RawChangeUnitRecord> {
    Ok(RawChangeUnitRecord {
        project_id: row.get(0)?,
        change_unit_id: row.get(1)?,
        task_id: row.get(2)?,
        status: row.get(3)?,
        is_current: row.get(4)?,
        basis_state_version: row.get(5)?,
        scope_summary_json: row.get(6)?,
        bounded_paths_json: row.get(7)?,
        write_basis_json: row.get(8)?,
        effect_contract_json: row.get(9)?,
        lifecycle_json: row.get(10)?,
    })
}

fn validate_decoded_change_unit_record(
    record: RawChangeUnitRecord,
) -> StoreResult<ChangeUnitRecord> {
    let corrupt_value = |logical_column| {
        StoreError::corrupt_owner_state_value(
            "change_units",
            record.change_unit_id.clone(),
            logical_column,
        )
    };
    let basis_state_version = record
        .basis_state_version
        .ok_or_else(|| corrupt_value("basis_state_version"))
        .and_then(|value| u64::try_from(value).map_err(|_| corrupt_value("basis_state_version")))?;
    let is_current = match record.is_current {
        0 => false,
        1 => true,
        _ => return Err(corrupt_value("is_current")),
    };
    if !matches!(
        record.status.as_str(),
        "proposed" | "active" | "replaced" | "closed"
    ) {
        return Err(corrupt_value("status"));
    }
    Ok(ChangeUnitRecord {
        project_id: record.project_id,
        change_unit_id: record.change_unit_id,
        task_id: record.task_id,
        status: record.status,
        is_current,
        basis_state_version,
        scope_summary_json: record.scope_summary_json,
        bounded_paths_json: record.bounded_paths_json,
        write_basis_json: record.write_basis_json,
        effect_contract_json: record.effect_contract_json,
        lifecycle_json: record.lifecycle_json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoded_change_unit_requires_a_basis_state_version() {
        let error = validate_decoded_change_unit_record(RawChangeUnitRecord {
            project_id: "project".to_owned(),
            change_unit_id: "change".to_owned(),
            task_id: "task".to_owned(),
            status: "active".to_owned(),
            is_current: 1,
            basis_state_version: None,
            scope_summary_json: "{}".to_owned(),
            bounded_paths_json: "[]".to_owned(),
            write_basis_json: "{}".to_owned(),
            effect_contract_json: "null".to_owned(),
            lifecycle_json: "{}".to_owned(),
        })
        .expect_err("missing basis state version must fail closed");

        assert!(matches!(error, StoreError::CorruptOwnerStateValue { .. }));
    }
}
