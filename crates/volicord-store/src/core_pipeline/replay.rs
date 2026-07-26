use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use volicord_types::ids::IdempotencyKey;
use volicord_types::values::MethodName;

use super::{
    facade::CoreProjectStore,
    validation::{
        nonnegative_i64_to_u64, validate_canonical_replay_identity, ReplayContextFieldKind,
    },
};
use crate::{StoreError, StoreResult};

const TOOL_INVOCATION_COLUMNS: &str = "
    project_id, tool_name, idempotency_key, request_hash,
    basis_state_version, committed_state_version, actor_source,
    operation_category, verification_basis, git_workspace_context_json,
    response_json";

/// Stored idempotency replay row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolInvocationRecord {
    pub project_id: String,
    pub tool_name: String,
    pub idempotency_key: String,
    pub request_hash: String,
    pub basis_state_version: u64,
    pub committed_state_version: u64,
    pub actor_source: String,
    pub operation_category: String,
    pub verification_basis: Option<String>,
    pub git_workspace_context_json: Option<String>,
    pub response_json: String,
}

/// Immutable replay response facts used by exact historical result retrieval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredOperationResult {
    pub project_id: String,
    pub source_method: String,
    pub source_idempotency_key: String,
    pub committed_state_version: u64,
    pub actor_source: String,
    pub operation_category: String,
    pub response_sha256: String,
    pub response_size_bytes: u64,
    pub response_json: String,
}

/// Verified replay identity derived from current invocation context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedReplayContext {
    pub actor_source: String,
    pub operation_category: String,
    pub verification_basis: Option<String>,
    pub git_workspace_context_json: Option<String>,
}

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
    let sql = format!(
        "SELECT {TOOL_INVOCATION_COLUMNS}
           FROM tool_invocations
          WHERE project_id = ?1
            AND tool_name = ?2
            AND idempotency_key = ?3"
    );
    let record = tx
        .query_row(
            &sql,
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
    let sql = format!(
        "SELECT {TOOL_INVOCATION_COLUMNS}
           FROM tool_invocations
          WHERE project_id = ?1
            AND tool_name = ?2
            AND idempotency_key = ?3"
    );
    let record = conn
        .query_row(
            &sql,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_result_preserves_exact_response_bytes_and_derives_integrity_facts() {
        let response_json = "{\"result\":\"ok\"}".to_owned();
        let result = stored_operation_result(ToolInvocationRecord {
            project_id: "project".to_owned(),
            tool_name: "volicord.status".to_owned(),
            idempotency_key: "idempotency".to_owned(),
            request_hash: "sha256:request".to_owned(),
            basis_state_version: 4,
            committed_state_version: 5,
            actor_source: "local_user".to_owned(),
            operation_category: "read_only".to_owned(),
            verification_basis: None,
            git_workspace_context_json: None,
            response_json: response_json.clone(),
        })
        .expect("object response must project");

        assert_eq!(result.response_json, response_json);
        assert_eq!(result.response_size_bytes, 15);
        assert_eq!(
            result.response_sha256,
            format!("sha256:{:x}", Sha256::digest(response_json.as_bytes()))
        );
    }
}
