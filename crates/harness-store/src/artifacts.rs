use std::{fs, path::PathBuf};

use rusqlite::{params, Transaction};
use serde_json::{json, Value};

use crate::{
    core_pipeline::CoreProjectStore,
    sqlite::{begin_immediate_transaction, ARTIFACTS_DIR, ARTIFACTS_TMP_DIR},
    StoreError, StoreResult,
};

/// Placement marker for future artifact-store plumbing.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactStoreBoundary;

/// Storage representation for a staged payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagedPayloadKind {
    /// Safe textual body bytes may be stored.
    SafeTextBody,
    /// Only a safe textual notice is being stored for the source material.
    SafeNotice,
}

impl StagedPayloadKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::SafeTextBody => "safe_text_body",
            Self::SafeNotice => "safe_notice",
        }
    }
}

/// Input for creating one transient artifact staging row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactStagingInsert {
    pub handle_id: String,
    pub task_id: String,
    pub created_by_surface_id: String,
    pub created_by_surface_instance_id: String,
    pub display_name: String,
    pub content_type: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub redaction_state: String,
    pub relation_hint: Option<String>,
    pub payload_kind: StagedPayloadKind,
    pub safe_bytes_or_notice: Vec<u8>,
}

/// Stored staged-handle facts returned after staging creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactStagingRecord {
    pub handle_id: String,
    pub task_id: String,
    pub created_by_surface_id: String,
    pub created_by_surface_instance_id: String,
    pub content_type: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub redaction_state: String,
    pub expires_at: String,
    pub tmp_path: String,
}

impl CoreProjectStore {
    /// Creates a transient `artifact_staging` row and stores safe staged bytes.
    ///
    /// This operation is storage-owned staging. It does not update
    /// `project_state.state_version`, append `task_events`, create
    /// `tool_invocations`, or insert persistent `artifacts` rows.
    pub fn create_artifact_staging(
        &mut self,
        input: ArtifactStagingInsert,
    ) -> StoreResult<ArtifactStagingRecord> {
        validate_insert(&input)?;

        let tmp_dir = self
            .project
            .project_home
            .join(ARTIFACTS_DIR)
            .join(ARTIFACTS_TMP_DIR);
        fs::create_dir_all(&tmp_dir)?;

        let tx = begin_immediate_transaction(&mut self.conn)?;
        let result = insert_artifact_staging_tx(&tx, &self.project.project_id, &tmp_dir, input);

        match result {
            Ok((record, write_path)) => match tx.commit() {
                Ok(()) => Ok(record),
                Err(error) => {
                    let _ = fs::remove_file(write_path);
                    Err(StoreError::from(error))
                }
            },
            Err(error) => Err(error),
        }
    }
}

fn insert_artifact_staging_tx(
    tx: &Transaction<'_>,
    project_id: &str,
    tmp_dir: &std::path::Path,
    input: ArtifactStagingInsert,
) -> StoreResult<(ArtifactStagingRecord, PathBuf)> {
    let handle_id = input.handle_id;
    let file_name = format!("{}.txt", path_component(&handle_id));
    let relative_tmp_path = format!("{ARTIFACTS_DIR}/{ARTIFACTS_TMP_DIR}/{file_name}");
    let write_path = tmp_dir.join(&file_name);
    let (created_at, expires_at) = staging_timestamps(tx)?;
    let artifact_json = json_text(json!({
        "display_name": input.display_name,
        "relation_hint": input.relation_hint
    }))?;
    let safe_metadata_json = json_text(json!({
        "payload_kind": input.payload_kind.as_str(),
        "stored_representation": input.payload_kind.as_str()
    }))?;
    let metadata_json = json_text(json!({
        "staging_ttl_hours": 24
    }))?;
    let size_bytes = u64_to_i64("artifact_staging.size_bytes", input.size_bytes)?;

    fs::write(&write_path, &input.safe_bytes_or_notice)?;

    let insert_result = tx.execute(
        "INSERT INTO artifact_staging (
            project_id,
            handle_id,
            task_id,
            created_by_surface_id,
            created_by_surface_instance_id,
            artifact_json,
            safe_metadata_json,
            tmp_path,
            sha256,
            size_bytes,
            content_type,
            redaction_state,
            status,
            expires_at,
            consumed_by_run_id,
            promoted_artifact_id,
            consumed_at,
            created_at,
            metadata_json
        )
        VALUES (
            ?1,
            ?2,
            ?3,
            ?4,
            ?5,
            ?6,
            ?7,
            ?8,
            ?9,
            ?10,
            ?11,
            ?12,
            'staged',
            ?13,
            NULL,
            NULL,
            NULL,
            ?14,
            ?15
        )",
        params![
            project_id,
            handle_id,
            input.task_id,
            input.created_by_surface_id,
            input.created_by_surface_instance_id,
            artifact_json,
            safe_metadata_json,
            relative_tmp_path,
            input.sha256,
            size_bytes,
            input.content_type,
            input.redaction_state,
            expires_at,
            created_at,
            metadata_json
        ],
    );

    if let Err(error) = insert_result {
        let _ = fs::remove_file(&write_path);
        return Err(StoreError::from(error));
    }

    Ok((
        ArtifactStagingRecord {
            handle_id,
            task_id: input.task_id,
            created_by_surface_id: input.created_by_surface_id,
            created_by_surface_instance_id: input.created_by_surface_instance_id,
            content_type: input.content_type,
            sha256: input.sha256,
            size_bytes: input.size_bytes,
            redaction_state: input.redaction_state,
            expires_at,
            tmp_path: relative_tmp_path,
        },
        write_path,
    ))
}

fn validate_insert(input: &ArtifactStagingInsert) -> StoreResult<()> {
    validate_identifier("handle_id", &input.handle_id)?;
    validate_identifier("task_id", &input.task_id)?;
    validate_identifier("created_by_surface_id", &input.created_by_surface_id)?;
    validate_identifier(
        "created_by_surface_instance_id",
        &input.created_by_surface_instance_id,
    )?;
    validate_nonempty_text("display_name", &input.display_name)?;
    validate_nonempty_text("content_type", &input.content_type)?;
    validate_identifier("sha256", &input.sha256)?;
    validate_identifier("redaction_state", &input.redaction_state)?;
    if input.safe_bytes_or_notice.is_empty() {
        return Err(StoreError::InvalidInput {
            detail: "safe_bytes_or_notice must not be empty".to_owned(),
        });
    }
    Ok(())
}

fn staging_timestamps(tx: &Transaction<'_>) -> StoreResult<(String, String)> {
    tx.query_row(
        "SELECT
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '+24 hours')",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(StoreError::from)
}

fn path_component(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    let trimmed = output.trim_matches(['_', '.', '-']);
    if trimmed.is_empty() {
        "staged_artifact".to_owned()
    } else {
        trimmed.chars().take(120).collect()
    }
}

fn validate_identifier(field: &'static str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}

fn validate_nonempty_text(field: &'static str, value: &str) -> StoreResult<()> {
    if value.trim().is_empty() {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must not be empty"),
        });
    }
    if value.chars().any(char::is_control) {
        return Err(StoreError::InvalidInput {
            detail: format!("{field} must not contain control characters"),
        });
    }
    Ok(())
}

fn json_text(value: Value) -> StoreResult<String> {
    serde_json::to_string(&value).map_err(|error| StoreError::InvalidInput {
        detail: format!("artifact staging JSON could not be encoded: {error}"),
    })
}

fn u64_to_i64(field: &'static str, value: u64) -> StoreResult<i64> {
    i64::try_from(value).map_err(|_| StoreError::InvalidInput {
        detail: format!("{field} does not fit in SQLite integer"),
    })
}
