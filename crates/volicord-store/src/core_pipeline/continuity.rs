use rusqlite::{params, Connection};
use volicord_types::schema::{
    ArtifactRef, ContinuityCursor, JsonObject, PersistedProjectContinuityMetadata, StateRecordRef,
    MAX_CONTINUITY_PAGE_SIZE,
};
use volicord_types::values::{
    ActorSource, ProjectContinuityKind, ProjectContinuityStatus, UtcTimestamp,
};

use super::{facade::CoreProjectStore, mutations::MutationContext, validation::*};
use crate::{StoreError, StoreResult};

const PROJECT_CONTINUITY_RECORD_COLUMNS: &str = "
    project_id, continuity_record_id, source_task_id, source_change_unit_id,
    kind, title, summary, rationale, applies_to_paths_json,
    applies_to_refs_json, source_refs_json, artifact_refs_json, status,
    supersedes_refs_json, review_triggers_json, created_at, updated_at,
    metadata_json";

/// Continuity mutation applied inside one Core commit transaction.
#[derive(Debug, Clone, PartialEq)]
pub enum ContinuityMutation {
    ResolveUnrecordedChange(UnrecordedChangeResolutionUpdate),
    InsertRecord(Box<ProjectContinuityRecordInsert>),
}

/// Storage input for resolving one unrecorded Product Repository change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecordedChangeResolutionUpdate {
    pub unrecorded_change_id: String,
    pub resolution: JsonObject,
    pub resolved_at: UtcTimestamp,
    pub resolved_by_actor_source: ActorSource,
}

/// Storage input for inserting one project-level continuity record.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContinuityRecordInsert {
    pub continuity_record_id: String,
    pub source_task_id: String,
    pub source_change_unit_id: Option<String>,
    pub kind: ProjectContinuityKind,
    pub title: String,
    pub summary: String,
    pub rationale: Option<String>,
    pub applies_to_paths: Vec<String>,
    pub applies_to_refs: Vec<StateRecordRef>,
    pub source_refs: Vec<StateRecordRef>,
    pub artifact_refs: Vec<ArtifactRef>,
    pub status: ProjectContinuityStatus,
    pub supersedes_refs: Vec<StateRecordRef>,
    pub review_triggers: Vec<String>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
    pub metadata: PersistedProjectContinuityMetadata,
}

impl ContinuityMutation {
    /// Boxes the largest continuity mutation payload.
    pub fn insert_record(input: ProjectContinuityRecordInsert) -> Self {
        Self::InsertRecord(Box::new(input))
    }

    pub(super) fn apply(&self, context: &mut MutationContext<'_>) -> StoreResult<()> {
        match self {
            Self::ResolveUnrecordedChange(input) => context.resolve_unrecorded_change(input),
            Self::InsertRecord(input) => context.insert_project_continuity_record(input),
        }
    }
}

/// Stored project-continuity row data needed by Core method implementations.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContinuityRecordRecord {
    pub project_id: String,
    pub continuity_record_id: String,
    pub source_task_id: String,
    pub source_change_unit_id: Option<String>,
    pub kind: ProjectContinuityKind,
    pub title: String,
    pub summary: String,
    pub rationale: Option<String>,
    pub applies_to_paths: Vec<String>,
    pub applies_to_refs: Vec<StateRecordRef>,
    pub source_refs: Vec<StateRecordRef>,
    pub artifact_refs: Vec<ArtifactRef>,
    pub status: ProjectContinuityStatus,
    pub supersedes_refs: Vec<StateRecordRef>,
    pub review_triggers: Vec<String>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
    pub metadata: PersistedProjectContinuityMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawProjectContinuityRecord {
    project_id: String,
    continuity_record_id: String,
    source_task_id: String,
    source_change_unit_id: Option<String>,
    kind: String,
    title: String,
    summary: String,
    rationale: Option<String>,
    applies_to_paths_json: String,
    applies_to_refs_json: String,
    source_refs_json: String,
    artifact_refs_json: String,
    status: String,
    supersedes_refs_json: String,
    review_triggers_json: String,
    created_at: String,
    updated_at: String,
    metadata_json: String,
}

/// One strictly bounded active project-continuity page read from a single snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveProjectContinuityPage {
    pub records: Vec<ProjectContinuityRecordRecord>,
    pub total_count: u64,
    pub truncated: bool,
}

impl CoreProjectStore<'_> {
    /// Returns whether a project-continuity record id already exists in this project.
    pub fn project_continuity_record_exists(
        &self,
        continuity_record_id: &str,
    ) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT COUNT(*)
                   FROM project_continuity_records
                  WHERE project_id = ?1
                    AND continuity_record_id = ?2",
                params![self.project.project_id, continuity_record_id],
                |row| Ok(row.get::<_, i64>(0)? > 0),
            )
            .map_err(StoreError::from)
    }

    /// Reads one active project-continuity page in canonical status order.
    pub fn active_project_continuity_page(
        &self,
        page_size: u64,
        cursor: Option<&ContinuityCursor>,
    ) -> StoreResult<ActiveProjectContinuityPage> {
        active_project_continuity_page(&self.conn, &self.project.project_id, page_size, cursor)
    }

    /// Lists project-continuity rows that originated from one Task.
    pub fn project_continuity_records_for_task(
        &self,
        task_id: &str,
    ) -> StoreResult<Vec<ProjectContinuityRecordRecord>> {
        project_continuity_records_for_task(&self.conn, &self.project.project_id, task_id)
    }
}

fn active_project_continuity_page(
    conn: &Connection,
    project_id: &str,
    page_size: u64,
    cursor: Option<&ContinuityCursor>,
) -> StoreResult<ActiveProjectContinuityPage> {
    if !(1..=MAX_CONTINUITY_PAGE_SIZE).contains(&page_size) {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "project_continuity_records page_size must be between 1 and {MAX_CONTINUITY_PAGE_SIZE}"
            ),
        });
    }

    let (cursor_updated_at, cursor_record_id) = match cursor {
        Some(cursor) => {
            cursor
                .updated_at
                .ensure_canonical_rfc3339_representable()
                .map_err(|_| StoreError::InvalidInput {
                    detail: "project_continuity_records cursor.updated_at is not representable as canonical RFC 3339 UTC"
                        .to_owned(),
                })?;
            validate_identifier(
                "project_continuity_records cursor.continuity_record_id",
                cursor.continuity_record_id.as_str(),
            )?;
            (
                Some(cursor.updated_at.to_canonical_string()),
                Some(cursor.continuity_record_id.as_str()),
            )
        }
        None => (None, None),
    };
    let fetch_limit = i64::try_from(page_size + 1).map_err(|_| StoreError::InvalidInput {
        detail: "project_continuity_records page_size cannot be represented by SQLite".to_owned(),
    })?;
    let page_size = usize::try_from(page_size).map_err(|_| StoreError::InvalidInput {
        detail: "project_continuity_records page_size cannot be represented by this platform"
            .to_owned(),
    })?;

    let transaction = conn.unchecked_transaction()?;
    let total_count: i64 = transaction.query_row(
        "SELECT COUNT(*)
           FROM project_continuity_records
          WHERE project_id = ?1
            AND status = 'active'",
        [project_id],
        |row| row.get(0),
    )?;
    let total_count = u64::try_from(total_count).map_err(|_| StoreError::CorruptStoredValue {
        database_kind: crate::schema::PROJECT_STATE_DATABASE_KIND,
        field: "project_continuity_records.total_count",
    })?;
    let mut records = {
        let sql = format!(
            "SELECT {PROJECT_CONTINUITY_RECORD_COLUMNS}
               FROM project_continuity_records
              WHERE project_id = ?1
                AND status = 'active'
                AND (
                     ?2 IS NULL
                     OR volicord_utc_seconds(updated_at) < volicord_utc_seconds(?2)
                     OR (
                         volicord_utc_seconds(updated_at) = volicord_utc_seconds(?2)
                         AND volicord_utc_subsec_nanos(updated_at)
                             < volicord_utc_subsec_nanos(?2)
                     )
                     OR (
                         volicord_utc_seconds(updated_at) = volicord_utc_seconds(?2)
                         AND volicord_utc_subsec_nanos(updated_at)
                             = volicord_utc_subsec_nanos(?2)
                         AND continuity_record_id < ?3
                     )
                )
              ORDER BY volicord_utc_seconds(updated_at) DESC,
                       volicord_utc_subsec_nanos(updated_at) DESC,
                       continuity_record_id DESC
              LIMIT ?4"
        );
        let mut statement = transaction.prepare(&sql)?;
        let rows = statement.query_map(
            params![
                project_id,
                cursor_updated_at.as_deref(),
                cursor_record_id,
                fetch_limit
            ],
            raw_project_continuity_record_from_row,
        )?;
        let mut records = Vec::new();
        for row in rows {
            records.push(decode_project_continuity_record(row?)?);
        }
        records
    };
    transaction.commit()?;

    let truncated = records.len() > page_size;
    if truncated {
        records.truncate(page_size);
    }
    Ok(ActiveProjectContinuityPage {
        records,
        total_count,
        truncated,
    })
}

fn project_continuity_records_for_task(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Vec<ProjectContinuityRecordRecord>> {
    let sql = format!(
        "SELECT {PROJECT_CONTINUITY_RECORD_COLUMNS}
           FROM project_continuity_records
          WHERE project_id = ?1
            AND source_task_id = ?2
          ORDER BY volicord_utc_seconds(created_at),
                   volicord_utc_subsec_nanos(created_at),
                   continuity_record_id"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(
        params![project_id, task_id],
        raw_project_continuity_record_from_row,
    )?;
    let mut records = Vec::new();
    for row in rows {
        records.push(decode_project_continuity_record(row?)?);
    }
    Ok(records)
}

fn raw_project_continuity_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RawProjectContinuityRecord> {
    Ok(RawProjectContinuityRecord {
        project_id: row.get(0)?,
        continuity_record_id: row.get(1)?,
        source_task_id: row.get(2)?,
        source_change_unit_id: row.get(3)?,
        kind: row.get(4)?,
        title: row.get(5)?,
        summary: row.get(6)?,
        rationale: row.get(7)?,
        applies_to_paths_json: row.get(8)?,
        applies_to_refs_json: row.get(9)?,
        source_refs_json: row.get(10)?,
        artifact_refs_json: row.get(11)?,
        status: row.get(12)?,
        supersedes_refs_json: row.get(13)?,
        review_triggers_json: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
        metadata_json: row.get(17)?,
    })
}

fn decode_project_continuity_record(
    raw: RawProjectContinuityRecord,
) -> StoreResult<ProjectContinuityRecordRecord> {
    let record_id = raw.continuity_record_id.clone();
    let corrupt_value = |field| {
        StoreError::corrupt_owner_state_value(
            "project_continuity_records",
            record_id.clone(),
            field,
        )
    };
    let decode_timestamp = |field, value: &str| {
        let timestamp = UtcTimestamp::parse(value).map_err(|_| corrupt_value(field))?;
        timestamp
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| corrupt_value(field))?;
        if timestamp.to_canonical_string() == value {
            Ok(timestamp)
        } else {
            Err(corrupt_value(field))
        }
    };
    Ok(ProjectContinuityRecordRecord {
        project_id: raw.project_id,
        continuity_record_id: raw.continuity_record_id,
        source_task_id: raw.source_task_id,
        source_change_unit_id: raw.source_change_unit_id,
        kind: decode_owner_closed_value(
            "project_continuity_records",
            &record_id,
            "kind",
            &raw.kind,
        )?,
        title: raw.title,
        summary: raw.summary,
        rationale: raw.rationale,
        applies_to_paths: decode_owner_json_text(
            "project_continuity_records",
            &record_id,
            "applies_to_paths_json",
            &raw.applies_to_paths_json,
        )?,
        applies_to_refs: decode_owner_json_text(
            "project_continuity_records",
            &record_id,
            "applies_to_refs_json",
            &raw.applies_to_refs_json,
        )?,
        source_refs: decode_owner_json_text(
            "project_continuity_records",
            &record_id,
            "source_refs_json",
            &raw.source_refs_json,
        )?,
        artifact_refs: decode_owner_json_text(
            "project_continuity_records",
            &record_id,
            "artifact_refs_json",
            &raw.artifact_refs_json,
        )?,
        status: decode_owner_closed_value(
            "project_continuity_records",
            &record_id,
            "status",
            &raw.status,
        )?,
        supersedes_refs: decode_owner_json_text(
            "project_continuity_records",
            &record_id,
            "supersedes_refs_json",
            &raw.supersedes_refs_json,
        )?,
        review_triggers: decode_owner_json_text(
            "project_continuity_records",
            &record_id,
            "review_triggers_json",
            &raw.review_triggers_json,
        )?,
        created_at: decode_timestamp("created_at", &raw.created_at)?,
        updated_at: decode_timestamp("updated_at", &raw.updated_at)?,
        metadata: decode_owner_json_text(
            "project_continuity_records",
            &record_id,
            "metadata_json",
            &raw.metadata_json,
        )?,
    })
}

impl MutationContext<'_> {
    fn resolve_unrecorded_change(
        &mut self,
        input: &UnrecordedChangeResolutionUpdate,
    ) -> StoreResult<()> {
        validate_identifier("unrecorded_change_id", &input.unrecorded_change_id)?;
        let resolution_json =
            encode_json_column("unrecorded_changes.resolution_json", &input.resolution)?;
        let resolved_at = input.resolved_at.to_canonical_string();
        validate_timestamp("resolved_at", &resolved_at)?;

        let changed = self.tx.execute(
            "UPDATE unrecorded_changes
                SET status = 'resolved',
                    resolution_json = ?3,
                    resolved_at = ?4,
                    resolved_by_actor_source = ?5
              WHERE project_id = ?1
                AND unrecorded_change_id = ?2
                AND status = 'unresolved'",
            params![
                self.project_id,
                input.unrecorded_change_id,
                resolution_json,
                resolved_at,
                input.resolved_by_actor_source.to_canonical_string(),
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::SchemaInvariant {
                database_kind: "project_state",
                detail: "unresolved unrecorded-change resolution changed no rows".to_owned(),
            })
        }
    }

    fn insert_project_continuity_record(
        &mut self,
        input: &ProjectContinuityRecordInsert,
    ) -> StoreResult<()> {
        validate_identifier("continuity_record_id", &input.continuity_record_id)?;
        validate_identifier("source_task_id", &input.source_task_id)?;
        if let Some(source_change_unit_id) = &input.source_change_unit_id {
            validate_identifier("source_change_unit_id", source_change_unit_id)?;
        }
        validate_nonempty_text("project_continuity_records.title", &input.title)?;
        validate_nonempty_text("project_continuity_records.summary", &input.summary)?;
        if let Some(rationale) = &input.rationale {
            validate_nonempty_text("project_continuity_records.rationale", rationale)?;
        }
        let kind = encode_closed_value("project_continuity_records.kind", &input.kind)?;
        let applies_to_paths_json = encode_json_column(
            "project_continuity_records.applies_to_paths_json",
            &input.applies_to_paths,
        )?;
        let applies_to_refs_json = encode_json_column(
            "project_continuity_records.applies_to_refs_json",
            &input.applies_to_refs,
        )?;
        let source_refs_json = encode_json_column(
            "project_continuity_records.source_refs_json",
            &input.source_refs,
        )?;
        let artifact_refs_json = encode_json_column(
            "project_continuity_records.artifact_refs_json",
            &input.artifact_refs,
        )?;
        let status = encode_closed_value("project_continuity_records.status", &input.status)?;
        let supersedes_refs_json = encode_json_column(
            "project_continuity_records.supersedes_refs_json",
            &input.supersedes_refs,
        )?;
        let review_triggers_json = encode_json_column(
            "project_continuity_records.review_triggers_json",
            &input.review_triggers,
        )?;
        let created_at = input.created_at.to_canonical_string();
        let updated_at = input.updated_at.to_canonical_string();
        validate_timestamp("project_continuity_records.created_at", &created_at)?;
        validate_timestamp("project_continuity_records.updated_at", &updated_at)?;
        let metadata_json =
            encode_json_column("project_continuity_records.metadata_json", &input.metadata)?;

        self.tx.execute(
            "INSERT INTO project_continuity_records (
                project_id,
                continuity_record_id,
                source_task_id,
                source_change_unit_id,
                kind,
                title,
                summary,
                rationale,
                applies_to_paths_json,
                applies_to_refs_json,
                source_refs_json,
                artifact_refs_json,
                status,
                supersedes_refs_json,
                review_triggers_json,
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
                ?11,
                ?12,
                ?13,
                ?14,
                ?15,
                ?16,
                ?17,
                ?18
            )",
            params![
                self.project_id,
                input.continuity_record_id,
                input.source_task_id,
                input.source_change_unit_id,
                kind,
                input.title,
                input.summary,
                input.rationale,
                applies_to_paths_json,
                applies_to_refs_json,
                source_refs_json,
                artifact_refs_json,
                status,
                supersedes_refs_json,
                review_triggers_json,
                created_at,
                updated_at,
                metadata_json
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
    fn continuity_row_decoder_preserves_typed_owner_coordinates() {
        let connection = Connection::open_in_memory().expect("in-memory database must open");
        let raw = connection
            .query_row(
                "SELECT 'project', 'continuity', 'task', 'change', 'decision',
                        'title', 'summary', NULL, '[]', '[]', '[]', '[]',
                        'active', '[]', '[]', '2026-01-01T00:00:00Z',
                        '2026-01-01T00:00:00Z',
                        '{\"source\":\"resolve_user_action\",\"action_kind\":\"product_decision\",\"resolution_outcome\":\"accepted\",\"selected_option_id\":\"option\"}'",
                [],
                raw_project_continuity_record_from_row,
            )
            .expect("physical row must load");
        let record = decode_project_continuity_record(raw.clone()).expect("typed row must decode");

        assert_eq!(record.project_id, "project");
        assert_eq!(record.continuity_record_id, "continuity");
        assert_eq!(record.source_change_unit_id.as_deref(), Some("change"));
        assert_eq!(record.kind, ProjectContinuityKind::Decision);
        assert_eq!(record.status, ProjectContinuityStatus::Active);
        assert!(record.source_refs.is_empty());

        let mut unknown_kind = raw.clone();
        unknown_kind.kind = "legacy".to_owned();
        assert!(matches!(
            decode_project_continuity_record(unknown_kind),
            Err(StoreError::CorruptOwnerStateValue {
                table: "project_continuity_records",
                logical_column: "kind",
                ..
            })
        ));

        let mut missing_metadata_fields = raw;
        missing_metadata_fields.metadata_json = "{}".to_owned();
        assert!(matches!(
            decode_project_continuity_record(missing_metadata_fields),
            Err(StoreError::CorruptOwnerStateJson {
                table: "project_continuity_records",
                logical_column: "metadata_json",
                ..
            })
        ));
    }

    #[test]
    fn continuity_mutation_validates_its_storage_identity_before_sql() {
        let error = with_empty_mutation_context(|context| {
            ContinuityMutation::ResolveUnrecordedChange(UnrecordedChangeResolutionUpdate {
                unrecorded_change_id: " ".to_owned(),
                resolution: JsonObject::new(),
                resolved_at: UtcTimestamp::parse("2026-01-01T00:00:00Z")
                    .expect("timestamp must parse"),
                resolved_by_actor_source: ActorSource::LocalUser,
            })
            .apply(context)
            .expect_err("blank unrecorded-change id must fail before SQL")
        });

        assert!(matches!(error, StoreError::InvalidInput { .. }));
    }
}
