use rusqlite::{params, Connection};
use volicord_types::schema::{ContinuityCursor, MAX_CONTINUITY_PAGE_SIZE};

use super::{facade::CoreProjectStore, mutations::MutationContext, validation::*};
use crate::{StoreError, StoreResult};

const PROJECT_CONTINUITY_RECORD_COLUMNS: &str = "
    project_id, continuity_record_id, source_task_id, source_change_unit_id,
    kind, title, summary, rationale, applies_to_paths_json,
    applies_to_refs_json, source_refs_json, artifact_refs_json, status,
    supersedes_refs_json, review_triggers_json, created_at, updated_at,
    metadata_json";

/// Continuity mutation applied inside one Core commit transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinuityMutation {
    ResolveUnrecordedChange(UnrecordedChangeResolutionUpdate),
    InsertRecord(Box<ProjectContinuityRecordInsert>),
}

/// Storage input for resolving one unrecorded Product Repository change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecordedChangeResolutionUpdate {
    pub unrecorded_change_id: String,
    pub resolution_json: String,
    pub resolved_at: String,
    pub resolved_by_actor_source: String,
}

/// Storage input for inserting one project-level continuity record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContinuityRecordInsert {
    pub continuity_record_id: String,
    pub source_task_id: String,
    pub source_change_unit_id: Option<String>,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub rationale: Option<String>,
    pub applies_to_paths_json: String,
    pub applies_to_refs_json: String,
    pub source_refs_json: String,
    pub artifact_refs_json: String,
    pub status: String,
    pub supersedes_refs_json: String,
    pub review_triggers_json: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContinuityRecordRecord {
    pub project_id: String,
    pub continuity_record_id: String,
    pub source_task_id: String,
    pub source_change_unit_id: Option<String>,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub rationale: Option<String>,
    pub applies_to_paths_json: String,
    pub applies_to_refs_json: String,
    pub source_refs_json: String,
    pub artifact_refs_json: String,
    pub status: String,
    pub supersedes_refs_json: String,
    pub review_triggers_json: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata_json: String,
}

/// One strictly bounded active project-continuity page read from a single snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
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
            project_continuity_record_from_row,
        )?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
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
        project_continuity_record_from_row,
    )?;
    let mut records = Vec::new();
    for row in rows {
        records.push(row?);
    }
    Ok(records)
}

fn project_continuity_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProjectContinuityRecordRecord> {
    Ok(ProjectContinuityRecordRecord {
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

impl MutationContext<'_> {
    fn resolve_unrecorded_change(
        &mut self,
        input: &UnrecordedChangeResolutionUpdate,
    ) -> StoreResult<()> {
        validate_identifier("unrecorded_change_id", &input.unrecorded_change_id)?;
        validate_json_text("unrecorded_changes.resolution_json", &input.resolution_json)?;
        validate_timestamp("resolved_at", &input.resolved_at)?;
        validate_identifier("resolved_by_actor_source", &input.resolved_by_actor_source)?;

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
                input.resolution_json,
                input.resolved_at,
                input.resolved_by_actor_source,
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
        validate_project_continuity_kind("project_continuity_records.kind", &input.kind)?;
        validate_nonempty_text("project_continuity_records.title", &input.title)?;
        validate_nonempty_text("project_continuity_records.summary", &input.summary)?;
        if let Some(rationale) = &input.rationale {
            validate_nonempty_text("project_continuity_records.rationale", rationale)?;
        }
        validate_string_list_json(
            "project_continuity_records.applies_to_paths_json",
            &input.applies_to_paths_json,
        )?;
        validate_state_refs_json(
            "project_continuity_records.applies_to_refs_json",
            &input.applies_to_refs_json,
        )?;
        validate_state_refs_json(
            "project_continuity_records.source_refs_json",
            &input.source_refs_json,
        )?;
        validate_artifact_refs_json(
            "project_continuity_records.artifact_refs_json",
            &input.artifact_refs_json,
        )?;
        validate_project_continuity_status("project_continuity_records.status", &input.status)?;
        validate_state_refs_json(
            "project_continuity_records.supersedes_refs_json",
            &input.supersedes_refs_json,
        )?;
        validate_string_list_json(
            "project_continuity_records.review_triggers_json",
            &input.review_triggers_json,
        )?;
        validate_timestamp("project_continuity_records.created_at", &input.created_at)?;
        validate_timestamp("project_continuity_records.updated_at", &input.updated_at)?;
        validate_json_text(
            "project_continuity_records.metadata_json",
            &input.metadata_json,
        )?;

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
                input.kind,
                input.title,
                input.summary,
                input.rationale,
                input.applies_to_paths_json,
                input.applies_to_refs_json,
                input.source_refs_json,
                input.artifact_refs_json,
                input.status,
                input.supersedes_refs_json,
                input.review_triggers_json,
                input.created_at,
                input.updated_at,
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
    fn continuity_row_decoder_preserves_typed_owner_coordinates() {
        let connection = Connection::open_in_memory().expect("in-memory database must open");
        let record = connection
            .query_row(
                "SELECT 'project', 'continuity', 'task', 'change', 'decision',
                        'title', 'summary', NULL, '[]', '[]', '[]', '[]',
                        'active', '[]', '[]', '2026-01-01T00:00:00Z',
                        '2026-01-01T00:00:00Z', '{}'",
                [],
                project_continuity_record_from_row,
            )
            .expect("typed row must decode");

        assert_eq!(record.project_id, "project");
        assert_eq!(record.continuity_record_id, "continuity");
        assert_eq!(record.source_change_unit_id.as_deref(), Some("change"));
    }

    #[test]
    fn continuity_mutation_validates_its_storage_identity_before_sql() {
        let error = with_empty_mutation_context(|context| {
            ContinuityMutation::ResolveUnrecordedChange(UnrecordedChangeResolutionUpdate {
                unrecorded_change_id: " ".to_owned(),
                resolution_json: "{}".to_owned(),
                resolved_at: "2026-01-01T00:00:00Z".to_owned(),
                resolved_by_actor_source: "actor".to_owned(),
            })
            .apply(context)
            .expect_err("blank unrecorded-change id must fail before SQL")
        });

        assert!(matches!(error, StoreError::InvalidInput { .. }));
    }
}
