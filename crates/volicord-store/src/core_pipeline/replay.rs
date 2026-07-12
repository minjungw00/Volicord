use rusqlite::{params, Connection, OptionalExtension, Transaction};
use volicord_types::{IdempotencyKey, MethodName};

use super::{
    validation::{nonnegative_i64_to_u64, validate_stored_git_workspace_context_json},
    CoreProjectStore, ToolInvocationRecord, VerifiedReplayContext,
};
use crate::{StoreError, StoreResult};

impl ToolInvocationRecord {
    /// Returns whether this replay row is eligible for the supplied verified context.
    pub fn matches_verified_replay_context(&self, context: &VerifiedReplayContext) -> bool {
        self.actor_source == context.actor_source.as_str()
            && self.operation_category == context.operation_category.as_str()
            && self.git_workspace_context_json == context.git_workspace_context_json
    }
}

impl CoreProjectStore {
    /// Reads a committed replay row without creating storage effects.
    pub fn tool_invocation(
        &self,
        method_name: MethodName,
        idempotency_key: &IdempotencyKey,
    ) -> StoreResult<Option<ToolInvocationRecord>> {
        tool_invocation(
            &self.conn,
            &self.project.project_id,
            method_name.as_str(),
            idempotency_key.as_str(),
        )
    }
}

pub(super) fn tool_invocation_tx(
    tx: &Transaction<'_>,
    project_id: &str,
    tool_name: &str,
    idempotency_key: &str,
) -> StoreResult<Option<ToolInvocationRecord>> {
    let record = tx
        .query_row(
            "SELECT
            project_id,
            tool_name,
            idempotency_key,
            request_hash,
            basis_state_version,
            committed_state_version,
            actor_source,
            operation_category,
            verification_basis,
            git_workspace_context_json,
            response_json
         FROM tool_invocations
         WHERE project_id = ?1
           AND tool_name = ?2
           AND idempotency_key = ?3",
            params![project_id, tool_name, idempotency_key],
            tool_invocation_from_row,
        )
        .optional()
        .map_err(StoreError::from)?;
    record.map(validate_loaded_replay_context).transpose()
}

fn tool_invocation(
    conn: &Connection,
    project_id: &str,
    tool_name: &str,
    idempotency_key: &str,
) -> StoreResult<Option<ToolInvocationRecord>> {
    let record = conn
        .query_row(
            "SELECT
            project_id,
            tool_name,
            idempotency_key,
            request_hash,
            basis_state_version,
            committed_state_version,
            actor_source,
            operation_category,
            verification_basis,
            git_workspace_context_json,
            response_json
         FROM tool_invocations
         WHERE project_id = ?1
           AND tool_name = ?2
           AND idempotency_key = ?3",
            params![project_id, tool_name, idempotency_key],
            tool_invocation_from_row,
        )
        .optional()
        .map_err(StoreError::from)?;
    record.map(validate_loaded_replay_context).transpose()
}

fn validate_loaded_replay_context(
    record: ToolInvocationRecord,
) -> StoreResult<ToolInvocationRecord> {
    if let Some(context) = record.git_workspace_context_json.as_deref() {
        let record_ref = format!(
            "{}/{}/{}",
            record.project_id, record.tool_name, record.idempotency_key
        );
        validate_stored_git_workspace_context_json(&record_ref, context)?;
    }
    Ok(record)
}

fn tool_invocation_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ToolInvocationRecord> {
    let basis_state_version = row.get::<_, i64>(4)?;
    let committed_state_version = row.get::<_, i64>(5)?;
    Ok(ToolInvocationRecord {
        project_id: row.get(0)?,
        tool_name: row.get(1)?,
        idempotency_key: row.get(2)?,
        request_hash: row.get(3)?,
        basis_state_version: nonnegative_i64_to_u64(
            "tool_invocations.basis_state_version",
            basis_state_version,
        )?,
        committed_state_version: nonnegative_i64_to_u64(
            "tool_invocations.committed_state_version",
            committed_state_version,
        )?,
        actor_source: row.get(6)?,
        operation_category: row.get(7)?,
        verification_basis: row.get(8)?,
        git_workspace_context_json: row.get(9)?,
        response_json: row.get(10)?,
    })
}
