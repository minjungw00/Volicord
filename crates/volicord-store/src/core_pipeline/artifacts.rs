use rusqlite::{params, Connection, OptionalExtension, Transaction};
use volicord_types::{
    ids::{RunId, StagedArtifactHandleId, TaskId},
    schema::{
        PersistedArtifactProducer, PersistedArtifactProvenance, PersistedArtifactProvenanceMetadata,
    },
    values::UtcTimestamp,
};

use super::{facade::CoreProjectStore, mutations::MutationContext, validation::*};
use crate::{
    artifacts::{
        persistent_body_path_from_staging_tmp_path,
        verify_persistent_artifact_body as verify_persistent_artifact_body_in_store,
        PersistentArtifactBodySpec, PersistentArtifactVerification,
    },
    sqlite::ARTIFACTS_DIR,
    StoreError, StoreResult,
};

const ARTIFACT_STAGING_RECORD_COLUMNS: &str = "
    project_id, handle_id, task_id, created_by_actor_source, artifact_json,
    tmp_path, sha256, size_bytes, content_type, redaction_state, status,
    created_at, expires_at";

const ARTIFACT_RECORD_COLUMNS: &str = "
    project_id, artifact_id, task_id, producer_run_id,
    source_staging_handle_id, uri, body_path, sha256, size_bytes,
    content_type, integrity_status, redaction_state, status, producer_json,
    metadata_json";

/// Artifact mutation applied inside one Core commit transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactMutation {
    PromoteStaged(ArtifactPromotion),
    Link(ArtifactLinkInsert),
}

/// Storage input for promoting one staged artifact to a persistent artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactPromotion {
    pub handle_id: String,
    pub artifact_id: String,
    pub task_id: String,
    pub run_id: String,
    pub expected_created_by_actor_source: String,
    pub expected_sha256: String,
    pub expected_size_bytes: u64,
    pub expected_redaction_state: String,
    pub expected_created_at: String,
    pub expected_expires_at: String,
    pub uri: String,
    pub retention_json: String,
    pub producer_json: String,
    pub metadata_json: String,
}

/// Storage input for linking a persistent artifact to an owner relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactLinkInsert {
    pub artifact_id: String,
    pub task_id: String,
    pub owner_record_kind: String,
    pub owner_record_id: String,
    pub created_by_run_id: String,
    pub metadata_json: String,
}

impl ArtifactMutation {
    pub(super) fn apply(&self, context: &mut MutationContext<'_>) -> StoreResult<()> {
        match self {
            Self::PromoteStaged(input) => context.promote_staged_artifact(input),
            Self::Link(input) => context.link_artifact(input),
        }
    }
}

/// Stored staged artifact facts needed by `volicord.record_run`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredArtifactStagingRecord {
    pub project_id: String,
    pub handle_id: String,
    pub task_id: String,
    pub created_by_actor_source: String,
    pub artifact_json: String,
    pub tmp_path: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
    pub content_type: Option<String>,
    pub redaction_state: String,
    pub status: String,
    pub created_at: String,
    pub expires_at: String,
}

/// Stored persistent artifact facts needed by `volicord.record_run`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredArtifactRecord {
    pub project_id: String,
    pub artifact_id: String,
    pub task_id: String,
    pub producer_run_id: Option<String>,
    pub source_staging_handle_id: Option<String>,
    pub uri: String,
    pub body_path: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
    pub content_type: Option<String>,
    pub integrity_status: String,
    pub redaction_state: String,
    pub status: String,
    pub producer: PersistedArtifactProducer,
    pub provenance: PersistedArtifactProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredArtifactRecordRaw {
    project_id: String,
    artifact_id: String,
    task_id: String,
    producer_run_id: Option<String>,
    source_staging_handle_id: Option<String>,
    uri: String,
    body_path: Option<String>,
    sha256: Option<String>,
    size_bytes: Option<u64>,
    content_type: Option<String>,
    integrity_status: String,
    redaction_state: String,
    status: String,
    producer_json: String,
    metadata_json: String,
}

impl CoreProjectStore<'_> {
    /// Reads one staged artifact row by exact project-local handle identity.
    pub fn artifact_staging_record(
        &self,
        handle_id: &str,
    ) -> StoreResult<Option<StoredArtifactStagingRecord>> {
        artifact_staging_record(&self.conn, &self.project.project_id, handle_id)
    }

    /// Returns whether a Task has prepared artifact input that has not been consumed.
    pub fn has_prepared_artifact_input(
        &self,
        task_id: &TaskId,
        now: &UtcTimestamp,
    ) -> StoreResult<bool> {
        has_prepared_artifact_input(&self.conn, &self.project.project_id, task_id.as_str(), now)
    }

    /// Reads one persistent artifact row by exact project-local artifact identity.
    pub fn artifact_record(&self, artifact_id: &str) -> StoreResult<Option<StoredArtifactRecord>> {
        artifact_record(&self.conn, &self.project.project_id, artifact_id)
    }

    /// Verifies the current persistent body bytes for an artifact row.
    pub fn verify_persistent_artifact_body(
        &self,
        record: &StoredArtifactRecord,
    ) -> StoreResult<PersistentArtifactVerification> {
        let artifact_store_root = self.project.project_home.join(ARTIFACTS_DIR);
        verify_persistent_artifact_body_in_store(
            &artifact_store_root,
            &PersistentArtifactBodySpec {
                body_path: record.body_path.as_deref(),
                sha256: record.sha256.as_deref(),
                size_bytes: record.size_bytes,
                content_type: record.content_type.as_deref(),
                integrity_status: &record.integrity_status,
                availability_status: &record.status,
            },
        )
    }

    /// Returns whether a persistent artifact already has an owner link for a Task.
    pub fn artifact_has_task_owner_link(
        &self,
        artifact_id: &str,
        task_id: &str,
    ) -> StoreResult<bool> {
        artifact_has_task_owner_link(&self.conn, &self.project.project_id, artifact_id, task_id)
    }

    /// Returns whether a persistent artifact has one exact owner relation.
    pub fn artifact_has_owner_link(
        &self,
        artifact_id: &str,
        task_id: &str,
        owner_record_kind: &str,
        owner_record_id: &str,
    ) -> StoreResult<bool> {
        artifact_has_owner_link(
            &self.conn,
            &self.project.project_id,
            artifact_id,
            task_id,
            owner_record_kind,
            owner_record_id,
        )
    }
}

fn artifact_staging_record(
    conn: &Connection,
    project_id: &str,
    handle_id: &str,
) -> StoreResult<Option<StoredArtifactStagingRecord>> {
    let sql = format!(
        "SELECT {ARTIFACT_STAGING_RECORD_COLUMNS}
           FROM artifact_staging
          WHERE project_id = ?1
            AND handle_id = ?2"
    );
    let record = conn
        .query_row(
            &sql,
            params![project_id, handle_id],
            artifact_staging_record_from_row,
        )
        .optional()?;
    record
        .map(validate_stored_artifact_staging_record)
        .transpose()
}

fn has_prepared_artifact_input(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    now: &UtcTimestamp,
) -> StoreResult<bool> {
    let mut statement = conn.prepare(
        "SELECT handle_id, created_at, expires_at
           FROM artifact_staging
          WHERE project_id = ?1
            AND task_id = ?2
            AND status = 'staged'",
    )?;
    let rows = statement.query_map(params![project_id, task_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut windows = Vec::new();
    for row in rows {
        let (handle_id, created_at, expires_at) = row?;
        windows.push(stored_artifact_staging_window(
            &handle_id,
            &created_at,
            &expires_at,
        )?);
    }
    Ok(windows
        .iter()
        .any(|(created_at, expires_at)| created_at <= now && now < expires_at))
}

pub(super) fn artifact_staging_record_tx(
    tx: &Transaction<'_>,
    project_id: &str,
    handle_id: &str,
) -> StoreResult<Option<StoredArtifactStagingRecord>> {
    let sql = format!(
        "SELECT {ARTIFACT_STAGING_RECORD_COLUMNS}
           FROM artifact_staging
          WHERE project_id = ?1
            AND handle_id = ?2"
    );
    let record = tx
        .query_row(
            &sql,
            params![project_id, handle_id],
            artifact_staging_record_from_row,
        )
        .optional()?;
    record
        .map(validate_stored_artifact_staging_record)
        .transpose()
}

fn artifact_staging_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StoredArtifactStagingRecord> {
    let size_bytes = row
        .get::<_, Option<i64>>(7)?
        .map(|value| nonnegative_i64_to_u64("artifact_staging.size_bytes", value))
        .transpose()?;
    Ok(StoredArtifactStagingRecord {
        project_id: row.get(0)?,
        handle_id: row.get(1)?,
        task_id: row.get(2)?,
        created_by_actor_source: row.get(3)?,
        artifact_json: row.get(4)?,
        tmp_path: row.get(5)?,
        sha256: row.get(6)?,
        size_bytes,
        content_type: row.get(8)?,
        redaction_state: row.get(9)?,
        status: row.get(10)?,
        created_at: row.get(11)?,
        expires_at: row.get(12)?,
    })
}

fn validate_stored_artifact_staging_record(
    record: StoredArtifactStagingRecord,
) -> StoreResult<StoredArtifactStagingRecord> {
    stored_artifact_staging_window(&record.handle_id, &record.created_at, &record.expires_at)?;
    Ok(record)
}

fn stored_artifact_staging_window(
    handle_id: &str,
    created_at: &str,
    expires_at: &str,
) -> StoreResult<(UtcTimestamp, UtcTimestamp)> {
    let parse = |field, value: &str| {
        let timestamp = UtcTimestamp::parse(value).map_err(|_| {
            StoreError::corrupt_owner_state_value("artifact_staging", handle_id, field)
        })?;
        timestamp
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| {
                StoreError::corrupt_owner_state_value("artifact_staging", handle_id, field)
            })?;
        Ok::<_, StoreError>(timestamp)
    };
    let created_at = parse("created_at", created_at)?;
    let expires_at = parse("expires_at", expires_at)?;
    if expires_at <= created_at {
        return Err(StoreError::corrupt_owner_state_value(
            "artifact_staging",
            handle_id,
            "expires_at",
        ));
    }
    Ok((created_at, expires_at))
}

fn artifact_record(
    conn: &Connection,
    project_id: &str,
    artifact_id: &str,
) -> StoreResult<Option<StoredArtifactRecord>> {
    let sql = format!(
        "SELECT {ARTIFACT_RECORD_COLUMNS}
           FROM artifacts
          WHERE project_id = ?1
            AND artifact_id = ?2"
    );
    let row = conn
        .query_row(
            &sql,
            params![project_id, artifact_id],
            artifact_record_raw_from_row,
        )
        .optional()?;
    row.map(stored_artifact_record_from_raw).transpose()
}

fn stored_artifact_record_from_raw(
    raw: StoredArtifactRecordRaw,
) -> StoreResult<StoredArtifactRecord> {
    let producer = decode_owner_json_text::<PersistedArtifactProducer>(
        "artifacts",
        raw.artifact_id.clone(),
        "producer_json",
        &raw.producer_json,
    )?;
    let provenance_metadata = decode_owner_json_text::<PersistedArtifactProvenanceMetadata>(
        "artifacts",
        raw.artifact_id.clone(),
        "metadata_json",
        &raw.metadata_json,
    )?;
    let producer_run_id = raw.producer_run_id.as_ref().ok_or_else(|| {
        StoreError::corrupt_owner_state_value(
            "artifacts",
            raw.artifact_id.clone(),
            "producer_run_id",
        )
    })?;
    let source_staging_handle_id = raw.source_staging_handle_id.as_ref().ok_or_else(|| {
        StoreError::corrupt_owner_state_value(
            "artifacts",
            raw.artifact_id.clone(),
            "source_staging_handle_id",
        )
    })?;
    let provenance = PersistedArtifactProvenance {
        source_kind: provenance_metadata.source_kind,
        producer_run_id: RunId::new(producer_run_id.clone()),
        source_staging_handle_id: StagedArtifactHandleId::new(source_staging_handle_id.clone()),
    };
    Ok(StoredArtifactRecord {
        project_id: raw.project_id,
        artifact_id: raw.artifact_id,
        task_id: raw.task_id,
        producer_run_id: raw.producer_run_id,
        source_staging_handle_id: raw.source_staging_handle_id,
        uri: raw.uri,
        body_path: raw.body_path,
        sha256: raw.sha256,
        size_bytes: raw.size_bytes,
        content_type: raw.content_type,
        integrity_status: raw.integrity_status,
        redaction_state: raw.redaction_state,
        status: raw.status,
        producer,
        provenance,
    })
}

fn artifact_record_raw_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StoredArtifactRecordRaw> {
    let size_bytes = row
        .get::<_, Option<i64>>(8)?
        .map(|value| nonnegative_i64_to_u64("artifacts.size_bytes", value))
        .transpose()?;
    Ok(StoredArtifactRecordRaw {
        project_id: row.get(0)?,
        artifact_id: row.get(1)?,
        task_id: row.get(2)?,
        producer_run_id: row.get(3)?,
        source_staging_handle_id: row.get(4)?,
        uri: row.get(5)?,
        body_path: row.get(6)?,
        sha256: row.get(7)?,
        size_bytes,
        content_type: row.get(9)?,
        integrity_status: row.get(10)?,
        redaction_state: row.get(11)?,
        status: row.get(12)?,
        producer_json: row.get(13)?,
        metadata_json: row.get(14)?,
    })
}

fn artifact_has_task_owner_link(
    conn: &Connection,
    project_id: &str,
    artifact_id: &str,
    task_id: &str,
) -> StoreResult<bool> {
    conn.query_row(
        "SELECT COUNT(*)
           FROM artifact_links
          WHERE project_id = ?1
            AND artifact_id = ?2
            AND task_id = ?3",
        params![project_id, artifact_id, task_id],
        |row| Ok(row.get::<_, i64>(0)? > 0),
    )
    .map_err(StoreError::from)
}

fn artifact_has_owner_link(
    conn: &Connection,
    project_id: &str,
    artifact_id: &str,
    task_id: &str,
    owner_record_kind: &str,
    owner_record_id: &str,
) -> StoreResult<bool> {
    conn.query_row(
        "SELECT COUNT(*)
           FROM artifact_links
          WHERE project_id = ?1
            AND artifact_id = ?2
            AND task_id = ?3
            AND owner_record_kind = ?4
            AND owner_record_id = ?5",
        params![
            project_id,
            artifact_id,
            task_id,
            owner_record_kind,
            owner_record_id
        ],
        |row| Ok(row.get::<_, i64>(0)? > 0),
    )
    .map_err(StoreError::from)
}

impl MutationContext<'_> {
    fn promote_staged_artifact(&mut self, input: &ArtifactPromotion) -> StoreResult<()> {
        validate_identifier("artifact_staging.handle_id", &input.handle_id)?;
        validate_identifier("artifact_id", &input.artifact_id)?;
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("run_id", &input.run_id)?;
        validate_identifier(
            "expected_created_by_actor_source",
            &input.expected_created_by_actor_source,
        )?;
        validate_artifact_sha256("expected_sha256", &input.expected_sha256)?;
        validate_identifier("expected_redaction_state", &input.expected_redaction_state)?;
        validate_timestamp("expected_created_at", &input.expected_created_at)?;
        validate_timestamp("expected_expires_at", &input.expected_expires_at)?;
        validate_identifier("artifacts.uri", &input.uri)?;
        validate_json_text("artifacts.retention_json", &input.retention_json)?;
        validate_artifact_producer_json("artifacts.producer_json", &input.producer_json)?;
        validate_artifact_provenance_metadata_json(
            "artifacts.metadata_json",
            &input.metadata_json,
        )?;

        let staging = artifact_staging_record_tx(self.tx, self.project_id, &input.handle_id)?
            .ok_or_else(|| StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "staged artifact disappeared before promotion".to_owned(),
            })?;
        if staging.task_id != input.task_id
            || staging.created_by_actor_source != input.expected_created_by_actor_source
            || staging.status != "staged"
            || staging.sha256.as_deref() != Some(input.expected_sha256.as_str())
            || staging.size_bytes != Some(input.expected_size_bytes)
            || staging.redaction_state != input.expected_redaction_state
            || staging.created_at != input.expected_created_at
            || staging.expires_at != input.expected_expires_at
        {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "staged artifact changed before promotion".to_owned(),
            });
        }
        let created_at = UtcTimestamp::parse(&staging.created_at).map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "artifact_staging",
                &staging.handle_id,
                "created_at",
            )
        })?;
        let expires_at = UtcTimestamp::parse(&staging.expires_at).map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "artifact_staging",
                &staging.handle_id,
                "expires_at",
            )
        })?;
        let committed_at = UtcTimestamp::parse(self.committed_at).map_err(|_| {
            StoreError::corrupt_owner_state_value("project_state", self.project_id, "updated_at")
        })?;
        if committed_at < created_at || committed_at >= expires_at {
            return Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "staged artifact is outside its exact eligibility window".to_owned(),
            });
        }
        let staging_tmp_path =
            staging
                .tmp_path
                .as_deref()
                .ok_or_else(|| StoreError::SchemaInvariant {
                    database_kind: "project_state",
                    detail: "staged artifact body path is missing before promotion".to_owned(),
                })?;
        let body_path = persistent_body_path_from_staging_tmp_path(staging_tmp_path)?;
        verify_staged_artifact_body(
            self.project_home,
            Some(staging_tmp_path),
            &input.expected_sha256,
            input.expected_size_bytes,
        )?;

        let size_bytes = u64_to_i64("artifacts.size_bytes", input.expected_size_bytes)?;
        self.tx.execute(
            "INSERT INTO artifacts (
                project_id,
                artifact_id,
                task_id,
                producer_run_id,
                source_staging_handle_id,
                uri,
                body_path,
                sha256,
                size_bytes,
                content_type,
                integrity_status,
                redaction_state,
                status,
                retention_json,
                producer_json,
                created_at,
                updated_at,
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
                'verified',
                ?11,
                'available',
                ?12,
                ?13,
                ?14,
                ?14,
                ?15
            )",
            params![
                self.project_id,
                input.artifact_id,
                input.task_id,
                input.run_id,
                input.handle_id,
                input.uri,
                body_path,
                input.expected_sha256,
                size_bytes,
                staging.content_type,
                input.expected_redaction_state,
                input.retention_json,
                input.producer_json,
                self.committed_at,
                input.metadata_json
            ],
        )?;

        let changed = self.tx.execute(
            "UPDATE artifact_staging
                SET status = 'consumed',
                    consumed_by_run_id = ?3,
                    promoted_artifact_id = ?4,
                    consumed_at = ?5
              WHERE project_id = ?1
                AND handle_id = ?2
                AND status = 'staged'",
            params![
                self.project_id,
                input.handle_id,
                input.run_id,
                input.artifact_id,
                self.committed_at
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "staged artifact consumption changed no rows".to_owned(),
            })
        }
    }

    fn link_artifact(&mut self, input: &ArtifactLinkInsert) -> StoreResult<()> {
        validate_identifier("artifact_id", &input.artifact_id)?;
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("owner_record_kind", &input.owner_record_kind)?;
        validate_identifier("owner_record_id", &input.owner_record_id)?;
        validate_identifier("created_by_run_id", &input.created_by_run_id)?;
        validate_json_text("artifact_links.metadata_json", &input.metadata_json)?;

        self.tx.execute(
            "INSERT OR IGNORE INTO artifact_links (
                project_id,
                artifact_id,
                task_id,
                owner_record_kind,
                owner_record_id,
                created_by_run_id,
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
                ?8
            )",
            params![
                self.project_id,
                input.artifact_id,
                input.task_id,
                input.owner_record_kind,
                input.owner_record_id,
                input.created_by_run_id,
                self.committed_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod behavior_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_pipeline::mutations::with_empty_mutation_context;

    #[test]
    fn staged_artifact_window_uses_a_half_open_interval() {
        let created_at =
            UtcTimestamp::parse("2026-01-01T00:00:00.000000001Z").expect("created_at must parse");
        let expires_at =
            UtcTimestamp::parse("2026-01-01T00:00:00.000000002Z").expect("expires_at must parse");
        let decoded = stored_artifact_staging_window(
            "handle",
            &created_at.to_string(),
            &expires_at.to_string(),
        )
        .expect("ordered window must decode");

        assert_eq!(decoded, (created_at, expires_at));
    }

    fn artifact_record_raw() -> StoredArtifactRecordRaw {
        StoredArtifactRecordRaw {
            project_id: "project".to_owned(),
            artifact_id: "artifact".to_owned(),
            task_id: "task".to_owned(),
            producer_run_id: Some("run".to_owned()),
            source_staging_handle_id: Some("handle".to_owned()),
            uri: "artifact://artifact".to_owned(),
            body_path: None,
            sha256: Some("a".repeat(64)),
            size_bytes: Some(1),
            content_type: Some("text/plain".to_owned()),
            integrity_status: "verified".to_owned(),
            redaction_state: "redacted".to_owned(),
            status: "available".to_owned(),
            producer_json: serde_json::to_string(&PersistedArtifactProducer {
                display_name: None,
                content_type: None,
                created_by_actor_source: volicord_types::values::ActorSource::System,
                artifact_input_id: volicord_types::ids::ArtifactInputId::new("artifact-input"),
                relation_hint: volicord_types::schema::RequiredNullable::null(),
                evidence_target: volicord_types::schema::RequiredNullable::null(),
            })
            .expect("typed artifact producer must serialize"),
            metadata_json: r#"{"source_kind":"staged_artifact"}"#.to_owned(),
        }
    }

    #[test]
    fn artifact_decoder_owns_producer_and_provenance_corruption() {
        let mut malformed_producer = artifact_record_raw();
        malformed_producer.producer_json = "{".to_owned();
        assert!(matches!(
            stored_artifact_record_from_raw(malformed_producer),
            Err(StoreError::CorruptOwnerStateJson {
                table: "artifacts",
                logical_column: "producer_json",
                ..
            })
        ));

        let mut missing_source = artifact_record_raw();
        missing_source.source_staging_handle_id = None;
        assert!(matches!(
            stored_artifact_record_from_raw(missing_source),
            Err(StoreError::CorruptOwnerStateValue {
                table: "artifacts",
                logical_column: "source_staging_handle_id",
                ..
            })
        ));
    }

    #[test]
    fn artifact_mutation_validates_its_storage_identity_before_sql() {
        let error = with_empty_mutation_context(|context| {
            ArtifactMutation::Link(ArtifactLinkInsert {
                artifact_id: " ".to_owned(),
                task_id: "task".to_owned(),
                owner_record_kind: "run".to_owned(),
                owner_record_id: "run".to_owned(),
                created_by_run_id: "run".to_owned(),
                metadata_json: "{}".to_owned(),
            })
            .apply(context)
            .expect_err("blank artifact id must fail before SQL")
        });

        assert!(matches!(error, StoreError::InvalidInput { .. }));
    }
}
