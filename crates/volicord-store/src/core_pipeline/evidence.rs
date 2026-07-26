use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::ids::TaskId;

use super::{
    facade::CoreProjectStore, mutations::MutationContext, record_refs::StoredRecordRef,
    validation::*,
};
use crate::{
    evidence_capture::{
        validate_evidence_capture_intent_window, EvidenceCaptureIntentInsert,
        EvidenceCaptureIntentWindowError, EvidenceProducerInsert,
    },
    StoreError, StoreResult,
};

const EVIDENCE_SUMMARY_COLUMNS: &str = "
    project_id, evidence_summary_id, task_id, change_unit_id,
    produced_at_state_version, status, coverage_json, supporting_refs_json,
    gap_refs_json, metadata_json";

const EVIDENCE_OBSERVATION_COLUMNS: &str = "
    project_id, evidence_observation_id, task_id, change_unit_id, run_id,
    acceptance_criterion_id, evidence_claim_id, source_kind, assurance_level,
    observed_by_actor_source, tool_name, tool_invocation_id, tool_metadata_json,
    input_refs_json, source_refs_json, output_artifact_refs_json,
    limitations_json, observed_at, recorded_at, metadata_json";

/// Evidence mutation applied inside one Core commit transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceMutation {
    EnsureClaim(EvidenceClaimInsert),
    InsertCaptureIntent(EvidenceCaptureIntentInsert),
    UpsertSummary(EvidenceSummaryUpsert),
    InsertObservation(EvidenceObservationInsert),
    InsertProducer(EvidenceProducerInsert),
}

/// Storage input for inserting an immutable Task-scoped supplemental claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceClaimInsert {
    pub evidence_claim_id: String,
    pub task_id: String,
    pub statement: String,
}

/// Storage input for creating or replacing one evidence summary row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSummaryUpsert {
    pub evidence_summary_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub status: String,
    pub coverage_json: String,
    pub supporting_refs_json: String,
    pub gap_refs_json: String,
    pub metadata_json: String,
}

/// Storage input for inserting one durable evidence observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceObservationInsert {
    pub evidence_observation_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub run_id: Option<String>,
    pub acceptance_criterion_id: Option<String>,
    pub evidence_claim_id: Option<String>,
    pub source_kind: String,
    pub assurance_level: String,
    pub observed_by_actor_source: Option<String>,
    pub tool_name: Option<String>,
    pub tool_invocation_id: Option<String>,
    pub tool_metadata_json: String,
    pub input_refs_json: String,
    pub source_refs_json: String,
    pub output_artifact_refs_json: String,
    pub limitations_json: String,
    pub observed_at: String,
    pub recorded_at: String,
    pub metadata_json: String,
}

impl EvidenceMutation {
    pub(super) fn apply(
        &self,
        context: &mut MutationContext<'_>,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        match self {
            Self::EnsureClaim(input) => context.ensure_evidence_claim(input),
            Self::InsertCaptureIntent(input) => context.insert_evidence_capture_intent(input),
            Self::UpsertSummary(input) => {
                context.upsert_evidence_summary(input, committed_state_version)
            }
            Self::InsertObservation(input) => context.insert_evidence_observation(input),
            Self::InsertProducer(input) => context.insert_evidence_producer(input),
        }
    }
}

/// Stored evidence summary facts needed by close-readiness evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSummaryRecord {
    pub project_id: String,
    pub evidence_summary_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub produced_at_state_version: u64,
    pub status: String,
    pub coverage_json: String,
    pub supporting_refs_json: String,
    pub gap_refs_json: String,
    pub metadata_json: String,
}

/// Stored evidence observation facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceObservationRecord {
    pub project_id: String,
    pub evidence_observation_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub run_id: Option<String>,
    pub acceptance_criterion_id: Option<String>,
    pub evidence_claim_id: Option<String>,
    pub source_kind: String,
    pub assurance_level: String,
    pub observed_by_actor_source: Option<String>,
    pub tool_name: Option<String>,
    pub tool_invocation_id: Option<String>,
    pub tool_metadata_json: String,
    pub input_refs_json: String,
    pub source_refs_json: String,
    pub output_artifact_refs_json: String,
    pub limitations_json: String,
    pub observed_at: String,
    pub recorded_at: String,
    pub metadata_json: String,
}

impl CoreProjectStore<'_> {
    /// Returns whether an evidence summary id already exists in this project.
    pub fn evidence_summary_exists(&self, evidence_summary_id: &str) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT COUNT(*)
                   FROM evidence_summaries
                  WHERE project_id = ?1
                    AND evidence_summary_id = ?2",
                params![self.project.project_id, evidence_summary_id],
                |row| Ok(row.get::<_, i64>(0)? > 0),
            )
            .map_err(StoreError::from)
    }

    /// Returns whether an evidence observation id already exists in this project.
    pub fn evidence_observation_exists(&self, evidence_observation_id: &str) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT COUNT(*)
                   FROM evidence_observations
                  WHERE project_id = ?1
                    AND evidence_observation_id = ?2",
                params![self.project.project_id, evidence_observation_id],
                |row| Ok(row.get::<_, i64>(0)? > 0),
            )
            .map_err(StoreError::from)
    }

    /// Reads one evidence observation row by exact project-local observation identity.
    pub fn evidence_observation_record(
        &self,
        evidence_observation_id: &str,
    ) -> StoreResult<Option<EvidenceObservationRecord>> {
        evidence_observation_record(
            &self.conn,
            &self.project.project_id,
            evidence_observation_id,
        )
    }

    /// Lists evidence observation refs created by a committed Run.
    pub fn evidence_observation_refs_for_run(
        &self,
        task_id: &TaskId,
        run_id: &str,
        state_version: u64,
    ) -> StoreResult<Vec<StoredRecordRef>> {
        evidence_observation_refs_for_run(
            &self.conn,
            &self.project.project_id,
            task_id.as_str(),
            run_id,
            state_version,
        )
    }

    /// Reads the latest evidence summary row for a Task, when one exists.
    pub fn latest_evidence_summary(
        &self,
        task_id: &TaskId,
    ) -> StoreResult<Option<EvidenceSummaryRecord>> {
        latest_evidence_summary(&self.conn, &self.project.project_id, task_id.as_str())
    }

    /// Reads one evidence summary row by exact project-local evidence identity.
    pub fn evidence_summary_record(
        &self,
        evidence_summary_id: &str,
    ) -> StoreResult<Option<EvidenceSummaryRecord>> {
        evidence_summary_record(&self.conn, &self.project.project_id, evidence_summary_id)
    }
}

fn evidence_observation_refs_for_run(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    run_id: &str,
    state_version: u64,
) -> StoreResult<Vec<StoredRecordRef>> {
    let mut statement = conn.prepare(
        "SELECT evidence_observation_id
           FROM evidence_observations
          WHERE project_id = ?1
            AND task_id = ?2
            AND run_id = ?3
          ORDER BY evidence_observation_id",
    )?;
    let rows = statement.query_map(params![project_id, task_id, run_id], |row| {
        Ok(StoredRecordRef {
            record_kind: "evidence_observation".to_owned(),
            record_id: row.get(0)?,
            project_id: project_id.to_owned(),
            task_id: Some(task_id.to_owned()),
            state_version: Some(state_version),
        })
    })?;
    let mut refs = Vec::new();
    for row in rows {
        refs.push(row?);
    }
    Ok(refs)
}

fn latest_evidence_summary(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> StoreResult<Option<EvidenceSummaryRecord>> {
    let sql = format!(
        "SELECT {EVIDENCE_SUMMARY_COLUMNS}
           FROM evidence_summaries
          WHERE project_id = ?1
            AND task_id = ?2
          ORDER BY produced_at_state_version DESC
          LIMIT 1"
    );
    let record = conn
        .query_row(
            &sql,
            params![project_id, task_id],
            evidence_summary_record_from_row,
        )
        .optional()?;
    validate_evidence_summary_state_version(conn, project_id, record)
}

fn evidence_summary_record(
    conn: &Connection,
    project_id: &str,
    evidence_summary_id: &str,
) -> StoreResult<Option<EvidenceSummaryRecord>> {
    let sql = format!(
        "SELECT {EVIDENCE_SUMMARY_COLUMNS}
           FROM evidence_summaries
          WHERE project_id = ?1
            AND evidence_summary_id = ?2"
    );
    let record = conn
        .query_row(
            &sql,
            params![project_id, evidence_summary_id],
            evidence_summary_record_from_row,
        )
        .optional()?;
    validate_evidence_summary_state_version(conn, project_id, record)
}

fn validate_evidence_summary_state_version(
    conn: &Connection,
    project_id: &str,
    record: Option<EvidenceSummaryRecord>,
) -> StoreResult<Option<EvidenceSummaryRecord>> {
    let Some(record) = record else {
        return Ok(None);
    };
    let current_state_version = conn
        .query_row(
            "SELECT state_version FROM project_state WHERE project_id = ?1",
            [project_id],
            |row| nonnegative_i64_to_u64("project_state.state_version", row.get(0)?),
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: "project_state",
            id: project_id.to_owned(),
        })?;
    if record.produced_at_state_version > current_state_version {
        return Err(StoreError::corrupt_owner_state_value(
            "evidence_summaries",
            &record.evidence_summary_id,
            "produced_at_state_version",
        ));
    }
    Ok(Some(record))
}

fn evidence_summary_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<EvidenceSummaryRecord> {
    Ok(EvidenceSummaryRecord {
        project_id: row.get(0)?,
        evidence_summary_id: row.get(1)?,
        task_id: row.get(2)?,
        change_unit_id: row.get(3)?,
        produced_at_state_version: nonnegative_i64_to_u64(
            "evidence_summaries.produced_at_state_version",
            row.get(4)?,
        )?,
        status: row.get(5)?,
        coverage_json: row.get(6)?,
        supporting_refs_json: row.get(7)?,
        gap_refs_json: row.get(8)?,
        metadata_json: row.get(9)?,
    })
}

fn evidence_observation_record(
    conn: &Connection,
    project_id: &str,
    evidence_observation_id: &str,
) -> StoreResult<Option<EvidenceObservationRecord>> {
    let sql = format!(
        "SELECT {EVIDENCE_OBSERVATION_COLUMNS}
           FROM evidence_observations
          WHERE project_id = ?1
            AND evidence_observation_id = ?2"
    );
    conn.query_row(
        &sql,
        params![project_id, evidence_observation_id],
        evidence_observation_record_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn evidence_observation_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<EvidenceObservationRecord> {
    Ok(EvidenceObservationRecord {
        project_id: row.get(0)?,
        evidence_observation_id: row.get(1)?,
        task_id: row.get(2)?,
        change_unit_id: row.get(3)?,
        run_id: row.get(4)?,
        acceptance_criterion_id: row.get(5)?,
        evidence_claim_id: row.get(6)?,
        source_kind: row.get(7)?,
        assurance_level: row.get(8)?,
        observed_by_actor_source: row.get(9)?,
        tool_name: row.get(10)?,
        tool_invocation_id: row.get(11)?,
        tool_metadata_json: row.get(12)?,
        input_refs_json: row.get(13)?,
        source_refs_json: row.get(14)?,
        output_artifact_refs_json: row.get(15)?,
        limitations_json: row.get(16)?,
        observed_at: row.get(17)?,
        recorded_at: row.get(18)?,
        metadata_json: row.get(19)?,
    })
}

fn validate_evidence_capture_kind(field: &'static str, value: &str) -> StoreResult<()> {
    if matches!(
        value,
        "verified_command_execution"
            | "verified_tool_invocation"
            | "registered_connection_observation"
    ) {
        Ok(())
    } else {
        Err(StoreError::schema_invariant(
            "project_state",
            format!("{field} is outside the evidence-capture value set"),
        ))
    }
}

impl MutationContext<'_> {
    fn ensure_evidence_claim(&mut self, input: &EvidenceClaimInsert) -> StoreResult<()> {
        validate_identifier("evidence_claim_id", &input.evidence_claim_id)?;
        validate_identifier("task_id", &input.task_id)?;
        if input.statement.trim().is_empty() {
            return Err(StoreError::schema_invariant(
                "project_state",
                "supplemental evidence claim statement must not be empty",
            ));
        }
        self.tx.execute(
            "INSERT OR IGNORE INTO evidence_claims (
                project_id, evidence_claim_id, task_id, statement, created_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5
            )",
            params![
                self.project_id,
                input.evidence_claim_id,
                input.task_id,
                input.statement,
                self.committed_at,
            ],
        )?;
        Ok(())
    }

    fn insert_evidence_capture_intent(
        &mut self,
        input: &EvidenceCaptureIntentInsert,
    ) -> StoreResult<()> {
        validate_identifier(
            "evidence_capture_intent_id",
            &input.evidence_capture_intent_id,
        )?;
        validate_identifier("task_id", &input.task_id)?;
        validate_identifier("change_unit_id", &input.change_unit_id)?;
        validate_identifier("baseline_ref", &input.baseline_ref)?;
        validate_evidence_capture_kind("capture_kind", &input.capture_kind)?;
        validate_artifact_sha256("input_sha256", &input.input_sha256)?;
        validate_identifier(
            "requested_by_actor_source",
            &input.requested_by_actor_source,
        )?;
        validate_identifier(
            "requesting_connection_internal_id",
            &input.requesting_connection_internal_id,
        )?;
        for (field, value) in [
            ("target_json", input.target_json.as_str()),
            ("capture_spec_json", input.capture_spec_json.as_str()),
            (
                "expected_outcome_json",
                input.expected_outcome_json.as_str(),
            ),
            ("session_context_json", input.session_context_json.as_str()),
            (
                "workspace_context_json",
                input.workspace_context_json.as_str(),
            ),
            ("metadata_json", input.metadata_json.as_str()),
        ] {
            validate_json_text(field, value)?;
        }
        validate_evidence_capture_intent_window(&input.created_at, &input.expires_at).map_err(
            |field| {
                StoreError::schema_invariant(
                    "project_state",
                    match field {
                        EvidenceCaptureIntentWindowError::CreatedAt => {
                            "invalid capture-intent created_at"
                        }
                        EvidenceCaptureIntentWindowError::ExpiresAt => {
                            "capture-intent expires_at must be exactly 15 minutes after created_at"
                        }
                    },
                )
            },
        )?;

        self.tx.execute(
            "INSERT INTO evidence_capture_intents (
                project_id,
                evidence_capture_intent_id,
                task_id,
                change_unit_id,
                scope_revision,
                baseline_ref,
                target_json,
                capture_kind,
                capture_spec_json,
                input_sha256,
                expected_outcome_json,
                requested_by_actor_source,
                requesting_connection_internal_id,
                session_context_json,
                workspace_context_json,
                created_at,
                expires_at,
                metadata_json
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18
            )",
            params![
                self.project_id,
                input.evidence_capture_intent_id,
                input.task_id,
                input.change_unit_id,
                u64_to_i64(
                    "evidence_capture_intents.scope_revision",
                    input.scope_revision
                )?,
                input.baseline_ref,
                input.target_json,
                input.capture_kind,
                input.capture_spec_json,
                input.input_sha256,
                input.expected_outcome_json,
                input.requested_by_actor_source,
                input.requesting_connection_internal_id,
                input.session_context_json,
                input.workspace_context_json,
                input.created_at,
                input.expires_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn upsert_evidence_summary(
        &mut self,
        input: &EvidenceSummaryUpsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        validate_identifier("evidence_summary_id", &input.evidence_summary_id)?;
        validate_identifier("task_id", &input.task_id)?;
        if let Some(change_unit_id) = &input.change_unit_id {
            validate_identifier("change_unit_id", change_unit_id)?;
        }
        validate_identifier("evidence_summaries.status", &input.status)?;
        validate_evidence_coverage_json("evidence_summaries.coverage_json", &input.coverage_json)?;
        validate_state_refs_json(
            "evidence_summaries.supporting_refs_json",
            &input.supporting_refs_json,
        )?;
        validate_state_refs_json("evidence_summaries.gap_refs_json", &input.gap_refs_json)?;
        validate_evidence_metadata_json("evidence_summaries.metadata_json", &input.metadata_json)?;
        let produced_at_state_version = u64_to_i64(
            "evidence_summaries.produced_at_state_version",
            committed_state_version,
        )?;

        self.tx.execute(
            "INSERT INTO evidence_summaries (
                project_id,
                evidence_summary_id,
                task_id,
                change_unit_id,
                produced_at_state_version,
                status,
                coverage_json,
                supporting_refs_json,
                gap_refs_json,
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
                ?10,
                ?11
            )
            ON CONFLICT(project_id, evidence_summary_id) DO UPDATE SET
                task_id = excluded.task_id,
                change_unit_id = excluded.change_unit_id,
                produced_at_state_version = excluded.produced_at_state_version,
                status = excluded.status,
                coverage_json = excluded.coverage_json,
                supporting_refs_json = excluded.supporting_refs_json,
                gap_refs_json = excluded.gap_refs_json,
                updated_at = excluded.updated_at,
                metadata_json = excluded.metadata_json",
            params![
                self.project_id,
                input.evidence_summary_id,
                input.task_id,
                input.change_unit_id,
                produced_at_state_version,
                input.status,
                input.coverage_json,
                input.supporting_refs_json,
                input.gap_refs_json,
                self.committed_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn insert_evidence_observation(
        &mut self,
        input: &EvidenceObservationInsert,
    ) -> StoreResult<()> {
        validate_identifier("evidence_observation_id", &input.evidence_observation_id)?;
        validate_identifier("task_id", &input.task_id)?;
        if let Some(change_unit_id) = &input.change_unit_id {
            validate_identifier("change_unit_id", change_unit_id)?;
        }
        if let Some(run_id) = &input.run_id {
            validate_identifier("run_id", run_id)?;
        }
        if input.acceptance_criterion_id.is_some() == input.evidence_claim_id.is_some() {
            return Err(StoreError::schema_invariant(
                "project_state",
                "evidence observation must select exactly one target identity",
            ));
        }
        validate_evidence_source_kind("evidence_observations.source_kind", &input.source_kind)?;
        validate_evidence_assurance_level(
            "evidence_observations.assurance_level",
            &input.assurance_level,
        )?;
        if let Some(actor_source) = &input.observed_by_actor_source {
            validate_identifier("observed_by_actor_source", actor_source)?;
        }
        if let Some(tool_name) = &input.tool_name {
            validate_identifier("tool_name", tool_name)?;
        }
        if let Some(tool_invocation_id) = &input.tool_invocation_id {
            validate_identifier("tool_invocation_id", tool_invocation_id)?;
        }
        validate_evidence_observation_tool_metadata_json(
            "evidence_observations.tool_metadata_json",
            &input.tool_metadata_json,
        )?;
        validate_state_refs_json(
            "evidence_observations.input_refs_json",
            &input.input_refs_json,
        )?;
        validate_source_refs_json(
            "evidence_observations.source_refs_json",
            &input.source_refs_json,
        )?;
        validate_artifact_refs_json(
            "evidence_observations.output_artifact_refs_json",
            &input.output_artifact_refs_json,
        )?;
        validate_string_list_json(
            "evidence_observations.limitations_json",
            &input.limitations_json,
        )?;
        validate_timestamp("observed_at", &input.observed_at)?;
        validate_timestamp("recorded_at", &input.recorded_at)?;
        validate_evidence_observation_metadata_json(
            "evidence_observations.metadata_json",
            &input.metadata_json,
        )?;

        self.tx.execute(
            "INSERT INTO evidence_observations (
                project_id,
                evidence_observation_id,
                task_id,
                change_unit_id,
                run_id,
                acceptance_criterion_id,
                evidence_claim_id,
                source_kind,
                assurance_level,
                observed_by_actor_source,
                tool_name,
                tool_invocation_id,
                tool_metadata_json,
                input_refs_json,
                source_refs_json,
                output_artifact_refs_json,
                limitations_json,
                observed_at,
                recorded_at,
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
                ?18,
                ?19,
                ?20
            )",
            params![
                self.project_id,
                input.evidence_observation_id,
                input.task_id,
                input.change_unit_id,
                input.run_id,
                input.acceptance_criterion_id,
                input.evidence_claim_id,
                input.source_kind,
                input.assurance_level,
                input.observed_by_actor_source,
                input.tool_name,
                input.tool_invocation_id,
                input.tool_metadata_json,
                input.input_refs_json,
                input.source_refs_json,
                input.output_artifact_refs_json,
                input.limitations_json,
                input.observed_at,
                input.recorded_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }

    fn insert_evidence_producer(&mut self, input: &EvidenceProducerInsert) -> StoreResult<()> {
        for (field, value) in [
            ("evidence_producer_id", input.evidence_producer_id.as_str()),
            (
                "evidence_capture_intent_id",
                input.evidence_capture_intent_id.as_str(),
            ),
            (
                "evidence_capture_receipt_id",
                input.evidence_capture_receipt_id.as_str(),
            ),
            (
                "evidence_observation_id",
                input.evidence_observation_id.as_str(),
            ),
            ("artifact_id", input.artifact_id.as_str()),
            ("run_id", input.run_id.as_str()),
            ("task_id", input.task_id.as_str()),
            ("change_unit_id", input.change_unit_id.as_str()),
            ("baseline_ref", input.baseline_ref.as_str()),
        ] {
            validate_identifier(field, value)?;
        }
        validate_evidence_capture_kind("producer_kind", &input.producer_kind)?;
        validate_json_text(
            "evidence_producers.canonical_producer_json",
            &input.canonical_producer_json,
        )?;
        validate_timestamp("created_at", &input.created_at)?;
        validate_json_text("evidence_producers.metadata_json", &input.metadata_json)?;

        self.tx.execute(
            "INSERT INTO evidence_producers (
                project_id,
                evidence_producer_id,
                evidence_capture_intent_id,
                evidence_capture_receipt_id,
                evidence_observation_id,
                artifact_id,
                run_id,
                task_id,
                change_unit_id,
                scope_revision,
                baseline_ref,
                producer_kind,
                canonical_producer_json,
                created_at,
                metadata_json
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                ?9, ?10, ?11, ?12, ?13, ?14, ?15
            )",
            params![
                self.project_id,
                input.evidence_producer_id,
                input.evidence_capture_intent_id,
                input.evidence_capture_receipt_id,
                input.evidence_observation_id,
                input.artifact_id,
                input.run_id,
                input.task_id,
                input.change_unit_id,
                u64_to_i64("evidence_producers.scope_revision", input.scope_revision)?,
                input.baseline_ref,
                input.producer_kind,
                input.canonical_producer_json,
                input.created_at,
                input.metadata_json
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_pipeline::mutations::with_empty_mutation_context;

    #[test]
    fn evidence_summary_decoder_rejects_a_negative_state_version() {
        let connection = Connection::open_in_memory().expect("in-memory database must open");
        let error = connection
            .query_row(
                "SELECT 'project', 'summary', 'task', NULL, -1, 'current',
                        '{}', '[]', '[]', '{}'",
                [],
                evidence_summary_record_from_row,
            )
            .expect_err("negative authority order must fail closed");

        assert!(matches!(
            error,
            rusqlite::Error::FromSqlConversionFailure(..)
        ));
    }

    #[test]
    fn evidence_mutation_validates_its_storage_identity_before_sql() {
        let error = with_empty_mutation_context(|context| {
            EvidenceMutation::EnsureClaim(EvidenceClaimInsert {
                evidence_claim_id: " ".to_owned(),
                task_id: "task".to_owned(),
                statement: "claim".to_owned(),
            })
            .apply(context, 1)
            .expect_err("blank evidence claim id must fail before SQL")
        });

        assert!(matches!(error, StoreError::InvalidInput { .. }));
    }
}
