use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use volicord_types::ids::{BaselineRef, TaskId};
use volicord_types::schema::ChangeUnitEffectContract;

use super::{facade::CoreProjectStore, mutations::MutationContext, validation::*};
use crate::{StoreError, StoreResult};

const CHANGE_UNIT_RECORD_COLUMNS: &str = "
    project_id, change_unit_id, task_id, status, is_current,
    basis_state_version, scope_summary_json, bounded_paths_json,
    write_basis_json, effect_contract_json, lifecycle_json";

/// Change-unit mutation applied inside one Core commit transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeUnitMutation {
    InsertCurrent(ChangeUnitInsert),
    ReplaceCurrent(ChangeUnitInsert),
}

/// Storage input for inserting a current Change Unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeUnitInsert {
    pub change_unit_id: String,
    pub task_id: String,
    pub scope_summary: StoredChangeUnitScopeSummary,
    pub bounded_paths: Vec<String>,
    pub write_basis: StoredChangeUnitWriteBasis,
    pub effect_contract: Option<ChangeUnitEffectContract>,
    pub lifecycle: StoredChangeUnitLifecycle,
}

impl ChangeUnitMutation {
    pub(super) fn apply(
        &self,
        context: &mut MutationContext<'_>,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        match self {
            Self::InsertCurrent(input) => {
                context.insert_current_change_unit(input, committed_state_version)
            }
            Self::ReplaceCurrent(input) => {
                context.replace_current_change_unit(input, committed_state_version)
            }
        }
    }
}

/// Current Change Unit row data needed by Core method implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeUnitRecord {
    pub project_id: String,
    pub change_unit_id: String,
    pub task_id: String,
    pub status: ChangeUnitStatus,
    pub is_current: bool,
    pub basis_state_version: u64,
    pub scope_summary: StoredChangeUnitScopeSummary,
    pub bounded_paths: Vec<String>,
    pub write_basis: StoredChangeUnitWriteBasis,
    pub effect_contract: Option<ChangeUnitEffectContract>,
    pub lifecycle: StoredChangeUnitLifecycle,
}

/// Closed lifecycle status of a persisted Change Unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeUnitStatus {
    Proposed,
    Active,
    Replaced,
    Closed,
}

/// Strictly decoded Change Unit scope-summary fields owned by Store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredChangeUnitScopeSummary {
    #[serde(default)]
    pub scope_summary: Option<String>,
    #[serde(default)]
    pub affected_areas: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
}

/// Strictly decoded Change Unit write-basis fields owned by Store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredChangeUnitWriteBasis {
    #[serde(default)]
    pub baseline_ref: Option<BaselineRef>,
    #[serde(default)]
    pub git_workspace_context: Option<StoredGitWorkspaceContext>,
}

/// Strictly decoded Git coordinate carried by a Change Unit write basis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredGitWorkspaceContext {
    pub git_common_dir: String,
    pub worktree_id: String,
    pub branch_ref: Option<String>,
    pub head_sha: Option<String>,
    pub workspace_fingerprint: String,
}

/// Strictly decoded Change Unit lifecycle fields owned by Store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredChangeUnitLifecycle {
    #[serde(default)]
    pub recovery_required: bool,
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
    let status = decode_owner_closed_value(
        "change_units",
        record.change_unit_id.clone(),
        "status",
        &record.status,
    )?;
    let scope_summary = decode_owner_json_text(
        "change_units",
        record.change_unit_id.clone(),
        "scope_summary_json",
        &record.scope_summary_json,
    )?;
    let bounded_paths = decode_owner_json_text(
        "change_units",
        record.change_unit_id.clone(),
        "bounded_paths_json",
        &record.bounded_paths_json,
    )?;
    let write_basis = decode_owner_json_text(
        "change_units",
        record.change_unit_id.clone(),
        "write_basis_json",
        &record.write_basis_json,
    )?;
    let effect_contract = decode_owner_json_text(
        "change_units",
        record.change_unit_id.clone(),
        "effect_contract_json",
        &record.effect_contract_json,
    )?;
    let lifecycle = decode_owner_json_text(
        "change_units",
        record.change_unit_id.clone(),
        "lifecycle_json",
        &record.lifecycle_json,
    )?;
    Ok(ChangeUnitRecord {
        project_id: record.project_id,
        change_unit_id: record.change_unit_id,
        task_id: record.task_id,
        status,
        is_current,
        basis_state_version,
        scope_summary,
        bounded_paths,
        write_basis,
        effect_contract,
        lifecycle,
    })
}

impl MutationContext<'_> {
    fn insert_current_change_unit(
        &mut self,
        input: &ChangeUnitInsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        self.insert_change_unit(input, committed_state_version)?;
        self.set_task_current_change_unit(&input.task_id, Some(&input.change_unit_id))
    }

    fn replace_current_change_unit(
        &mut self,
        input: &ChangeUnitInsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        validate_identifier("task_id", &input.task_id)?;
        self.tx.execute(
            "UPDATE change_units
                SET status = 'replaced',
                    is_current = 0,
                    closed_at = ?3,
                    updated_at = ?3
              WHERE project_id = ?1
                AND task_id = ?2
                AND status = 'active'
                AND is_current = 1",
            params![self.project_id, input.task_id, self.committed_at],
        )?;
        self.insert_current_change_unit(input, committed_state_version)
    }

    fn insert_change_unit(
        &mut self,
        input: &ChangeUnitInsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        validate_identifier("change_unit_id", &input.change_unit_id)?;
        validate_identifier("task_id", &input.task_id)?;
        let scope_summary_json =
            encode_json_column("change_units.scope_summary_json", &input.scope_summary)?;
        let bounded_paths_json =
            encode_json_column("change_units.bounded_paths_json", &input.bounded_paths)?;
        let write_basis_json =
            encode_json_column("change_units.write_basis_json", &input.write_basis)?;
        let effect_contract_json =
            encode_json_column("change_units.effect_contract_json", &input.effect_contract)?;
        let lifecycle_json = encode_json_column("change_units.lifecycle_json", &input.lifecycle)?;
        let basis_state_version = u64_to_i64("basis_state_version", committed_state_version)?;

        self.tx.execute(
            "INSERT INTO change_units (
                project_id,
                change_unit_id,
                task_id,
                status,
                is_current,
                basis_state_version,
                scope_summary_json,
                bounded_paths_json,
                write_basis_json,
                effect_contract_json,
                lifecycle_json,
                created_at,
                updated_at
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                'active',
                1,
                ?4,
                ?5,
                ?6,
                ?7,
                ?8,
                ?9,
                ?10,
                ?10
            )",
            params![
                self.project_id,
                input.change_unit_id,
                input.task_id,
                basis_state_version,
                scope_summary_json,
                bounded_paths_json,
                write_basis_json,
                effect_contract_json,
                lifecycle_json,
                self.committed_at
            ],
        )?;
        Ok(())
    }

    fn set_task_current_change_unit(
        &mut self,
        task_id: &str,
        change_unit_id: Option<&str>,
    ) -> StoreResult<()> {
        validate_identifier("task_id", task_id)?;
        let changed = self.tx.execute(
            "UPDATE tasks
                SET current_change_unit_id = ?3,
                    lifecycle_phase = CASE
                        WHEN ?3 IS NULL THEN lifecycle_phase
                        ELSE 'ready'
                    END,
                    updated_at = ?4
              WHERE project_id = ?1
                AND task_id = ?2",
            params![self.project_id, task_id, change_unit_id, self.committed_at],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "Task current Change Unit update changed no rows".to_owned(),
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

    #[test]
    fn change_unit_decoder_owns_structured_column_corruption() {
        let valid = || RawChangeUnitRecord {
            project_id: "project".to_owned(),
            change_unit_id: "change".to_owned(),
            task_id: "task".to_owned(),
            status: "active".to_owned(),
            is_current: 1,
            basis_state_version: Some(1),
            scope_summary_json: "{}".to_owned(),
            bounded_paths_json: "[]".to_owned(),
            write_basis_json: "{}".to_owned(),
            effect_contract_json: "null".to_owned(),
            lifecycle_json: "{}".to_owned(),
        };
        let cases = [
            ("bounded_paths_json", "{"),
            ("write_basis_json", "{"),
            ("effect_contract_json", "{"),
            ("lifecycle_json", "{"),
        ];

        for (column, malformed) in cases {
            let mut record = valid();
            match column {
                "bounded_paths_json" => record.bounded_paths_json = malformed.to_owned(),
                "write_basis_json" => record.write_basis_json = malformed.to_owned(),
                "effect_contract_json" => record.effect_contract_json = malformed.to_owned(),
                "lifecycle_json" => record.lifecycle_json = malformed.to_owned(),
                _ => unreachable!(),
            }
            let error = validate_decoded_change_unit_record(record)
                .expect_err("malformed Change Unit owner JSON must fail in the Store decoder");
            assert!(matches!(
                error,
                StoreError::CorruptOwnerStateJson {
                    table: "change_units",
                    logical_column,
                    ..
                } if logical_column == column
            ));
        }

        let mut unknown_status = valid();
        unknown_status.status = "legacy".to_owned();
        assert!(matches!(
            validate_decoded_change_unit_record(unknown_status),
            Err(StoreError::CorruptOwnerStateValue {
                table: "change_units",
                logical_column: "status",
                ..
            })
        ));
    }

    #[test]
    fn change_unit_mutation_validates_its_storage_identity_before_sql() {
        let error = with_empty_mutation_context(|context| {
            ChangeUnitMutation::InsertCurrent(ChangeUnitInsert {
                change_unit_id: " ".to_owned(),
                task_id: "task".to_owned(),
                scope_summary: StoredChangeUnitScopeSummary {
                    scope_summary: None,
                    affected_areas: Vec::new(),
                    constraints: Vec::new(),
                },
                bounded_paths: Vec::new(),
                write_basis: StoredChangeUnitWriteBasis {
                    baseline_ref: None,
                    git_workspace_context: None,
                },
                effect_contract: None,
                lifecycle: StoredChangeUnitLifecycle {
                    recovery_required: false,
                },
            })
            .apply(context, 1)
            .expect_err("blank Change Unit id must fail before SQL")
        });

        assert!(matches!(error, StoreError::InvalidInput { .. }));
    }
}
