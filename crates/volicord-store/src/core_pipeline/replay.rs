use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use volicord_types::ids::IdempotencyKey;
use volicord_types::values::MethodName;

use super::{
    validation::{
        nonnegative_i64_to_u64, validate_canonical_replay_identity, ReplayContextFieldKind,
    },
    CoreProjectStore, StoredOperationResult, ToolInvocationRecord, VerifiedReplayContext,
};
use crate::{StoreError, StoreResult};

impl ToolInvocationRecord {
    /// Returns whether this replay row is eligible for the supplied verified context.
    pub fn matches_verified_replay_context(&self, context: &VerifiedReplayContext) -> bool {
        self.actor_source == context.actor_source.as_str()
            && self.operation_category == context.operation_category.as_str()
            && self.verification_basis == context.verification_basis
            && self.git_workspace_context_json == context.git_workspace_context_json
    }
}

impl CoreProjectStore<'_> {
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

    /// Reads immutable response bytes and their integrity metadata without effects.
    pub fn operation_result(
        &self,
        source_method: MethodName,
        source_idempotency_key: &IdempotencyKey,
    ) -> StoreResult<Option<StoredOperationResult>> {
        self.tool_invocation(source_method, source_idempotency_key)?
            .map(stored_operation_result)
            .transpose()
    }
}

fn stored_operation_result(record: ToolInvocationRecord) -> StoreResult<StoredOperationResult> {
    if !matches!(
        serde_json::from_str::<serde_json::Value>(&record.response_json),
        Ok(serde_json::Value::Object(_))
    ) {
        return Err(StoreError::corrupt_stored_json(
            "project_state",
            "tool_invocations.response_json",
        ));
    }
    let response_size_bytes =
        u64::try_from(record.response_json.len()).map_err(|_| StoreError::SchemaInvariant {
            database_kind: "project_state",
            detail: "tool_invocations.response_json byte length does not fit u64".to_owned(),
        })?;
    let response_sha256 = format!(
        "sha256:{:x}",
        Sha256::digest(record.response_json.as_bytes())
    );
    Ok(StoredOperationResult {
        project_id: record.project_id,
        source_method: record.tool_name,
        source_idempotency_key: record.idempotency_key,
        committed_state_version: record.committed_state_version,
        actor_source: record.actor_source,
        operation_category: record.operation_category,
        response_sha256,
        response_size_bytes,
        response_json: record.response_json,
    })
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
    let record_ref = format!(
        "{}/{}/{}",
        record.project_id, record.tool_name, record.idempotency_key
    );
    validate_canonical_replay_identity(
        &record.actor_source,
        &record.operation_category,
        record.verification_basis.as_deref(),
        record.git_workspace_context_json.as_deref(),
    )
    .map_err(|failure| match failure.field_kind {
        ReplayContextFieldKind::Value => StoreError::corrupt_owner_state_value(
            "tool_invocations",
            record_ref.clone(),
            failure.logical_column,
        ),
        ReplayContextFieldKind::Json => StoreError::corrupt_owner_state_json(
            "tool_invocations",
            record_ref.clone(),
            failure.logical_column,
        ),
    })?;
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
