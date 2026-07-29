use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use volicord_types::ids::IdempotencyKey;
use volicord_types::schema::JsonObject;
use volicord_types::values::{ActorSource, MethodName, OperationCategory};

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
    pub tool_name: MethodName,
    pub idempotency_key: IdempotencyKey,
    pub request_hash: String,
    pub basis_state_version: u64,
    pub committed_state_version: u64,
    pub actor_source: ActorSource,
    pub operation_category: OperationCategory,
    pub verification_basis: Option<String>,
    pub git_workspace_context: Option<JsonObject>,
    pub response_json: String,
}

/// Immutable replay response facts used by exact historical result retrieval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredOperationResult {
    pub project_id: String,
    pub source_method: MethodName,
    pub source_idempotency_key: IdempotencyKey,
    pub committed_state_version: u64,
    pub actor_source: ActorSource,
    pub operation_category: OperationCategory,
    pub response_sha256: String,
    pub response_size_bytes: u64,
    pub response_json: String,
}

/// Verified replay identity derived from current invocation context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedReplayContext {
    pub actor_source: ActorSource,
    pub operation_category: OperationCategory,
    pub verification_basis: Option<String>,
    pub git_workspace_context: Option<JsonObject>,
}

#[derive(Debug)]
struct ToolInvocationRaw {
    project_id: String,
    tool_name: String,
    idempotency_key: String,
    request_hash: String,
    basis_state_version: u64,
    committed_state_version: u64,
    actor_source: String,
    operation_category: String,
    verification_basis: Option<String>,
    git_workspace_context_json: Option<String>,
    response_json: String,
}

impl ToolInvocationRecord {
    /// Returns whether this replay row is eligible for the supplied verified context.
    pub fn matches_verified_replay_context(&self, context: &VerifiedReplayContext) -> bool {
        self.actor_source == context.actor_source
            && self.operation_category == context.operation_category
            && self.verification_basis == context.verification_basis
            && self.git_workspace_context == context.git_workspace_context
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
            tool_invocation_raw_from_row,
        )
        .optional()
        .map_err(StoreError::from)?;
    record.map(decode_tool_invocation).transpose()
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
            tool_invocation_raw_from_row,
        )
        .optional()
        .map_err(StoreError::from)?;
    record.map(decode_tool_invocation).transpose()
}

fn decode_tool_invocation(raw: ToolInvocationRaw) -> StoreResult<ToolInvocationRecord> {
    let record_ref = format!(
        "{}/{}/{}",
        raw.project_id, raw.tool_name, raw.idempotency_key
    );
    validate_canonical_replay_identity(
        &raw.actor_source,
        &raw.operation_category,
        raw.verification_basis.as_deref(),
        raw.git_workspace_context_json.as_deref(),
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
    let tool_name = serde_json::from_value::<MethodName>(serde_json::Value::String(raw.tool_name))
        .map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "tool_invocations",
                record_ref.clone(),
                "tool_name",
            )
        })?;
    let actor_source = raw.actor_source.parse::<ActorSource>().map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "tool_invocations",
            record_ref.clone(),
            "actor_source",
        )
    })?;
    let operation_category = serde_json::from_value::<OperationCategory>(
        serde_json::Value::String(raw.operation_category),
    )
    .map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "tool_invocations",
            record_ref.clone(),
            "operation_category",
        )
    })?;
    let git_workspace_context = raw
        .git_workspace_context_json
        .as_deref()
        .map(|value| {
            serde_json::from_str::<JsonObject>(value).map_err(|_| {
                StoreError::corrupt_owner_state_json(
                    "tool_invocations",
                    record_ref.clone(),
                    "git_workspace_context_json",
                )
            })
        })
        .transpose()?;
    if !matches!(
        serde_json::from_str::<serde_json::Value>(&raw.response_json),
        Ok(serde_json::Value::Object(_))
    ) {
        return Err(StoreError::corrupt_owner_state_json(
            "tool_invocations",
            record_ref,
            "response_json",
        ));
    }
    Ok(ToolInvocationRecord {
        project_id: raw.project_id,
        tool_name,
        idempotency_key: IdempotencyKey::new(raw.idempotency_key),
        request_hash: raw.request_hash,
        basis_state_version: raw.basis_state_version,
        committed_state_version: raw.committed_state_version,
        actor_source,
        operation_category,
        verification_basis: raw.verification_basis,
        git_workspace_context,
        response_json: raw.response_json,
    })
}

fn tool_invocation_raw_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ToolInvocationRaw> {
    let basis_state_version = row.get::<_, i64>(4)?;
    let committed_state_version = row.get::<_, i64>(5)?;
    Ok(ToolInvocationRaw {
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
mod behavior_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_result_preserves_exact_response_bytes_and_derives_integrity_facts() {
        let response_json = "{\"result\":\"ok\"}".to_owned();
        let result = stored_operation_result(ToolInvocationRecord {
            project_id: "project".to_owned(),
            tool_name: MethodName::Status,
            idempotency_key: IdempotencyKey::new("idempotency"),
            request_hash: "sha256:request".to_owned(),
            basis_state_version: 4,
            committed_state_version: 5,
            actor_source: ActorSource::LocalUser,
            operation_category: OperationCategory::Read,
            verification_basis: None,
            git_workspace_context: None,
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
