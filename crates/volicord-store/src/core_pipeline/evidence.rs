use rusqlite::{params, Connection, OptionalExtension};
use volicord_types::canonical::canonical_json_string;
use volicord_types::ids::TaskId;
use volicord_types::schema::{
    evidence_capture_input_sha256, validate_evidence_capture_expected_outcome, ArtifactRef,
    EvidenceCaptureSpec, EvidenceCoverageItem, JsonObject, PersistedEvidenceMetadata,
    PersistedEvidenceObservationAuthority, SourceRef, StateRecordRef,
};
use volicord_types::values::{
    ActorSource, EvidenceAssuranceLevel, EvidenceProducerKind, EvidenceSourceKind, EvidenceStatus,
    StateRecordKind, UtcTimestamp,
};

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
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceSummaryUpsert {
    pub evidence_summary_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub status: EvidenceStatus,
    pub coverage: Vec<EvidenceCoverageItem>,
    pub supporting_refs: Vec<StateRecordRef>,
    pub gap_refs: Vec<StateRecordRef>,
    pub metadata: PersistedEvidenceMetadata,
}

/// Storage input for inserting one durable evidence observation.
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceObservationInsert {
    pub evidence_observation_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub run_id: Option<String>,
    pub acceptance_criterion_id: Option<String>,
    pub evidence_claim_id: Option<String>,
    pub source_kind: EvidenceSourceKind,
    pub assurance_level: EvidenceAssuranceLevel,
    pub observed_by_actor_source: Option<ActorSource>,
    pub tool_name: Option<String>,
    pub tool_invocation_id: Option<String>,
    pub tool_metadata: JsonObject,
    pub input_refs: Vec<StateRecordRef>,
    pub source_refs: Vec<SourceRef>,
    pub output_artifact_refs: Vec<ArtifactRef>,
    pub limitations: Vec<String>,
    pub observed_at: UtcTimestamp,
    pub recorded_at: UtcTimestamp,
    pub metadata: PersistedEvidenceObservationAuthority,
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
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceSummaryRecord {
    pub project_id: String,
    pub evidence_summary_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub produced_at_state_version: u64,
    pub status: EvidenceStatus,
    pub coverage: Vec<EvidenceCoverageItem>,
    pub supporting_refs: Vec<StateRecordRef>,
    pub gap_refs: Vec<StateRecordRef>,
    pub metadata: PersistedEvidenceMetadata,
}

/// Stored evidence observation facts.
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceObservationRecord {
    pub project_id: String,
    pub evidence_observation_id: String,
    pub task_id: String,
    pub change_unit_id: Option<String>,
    pub run_id: Option<String>,
    pub acceptance_criterion_id: Option<String>,
    pub evidence_claim_id: Option<String>,
    pub source_kind: EvidenceSourceKind,
    pub assurance_level: EvidenceAssuranceLevel,
    pub observed_by_actor_source: Option<ActorSource>,
    pub tool_name: Option<String>,
    pub tool_invocation_id: Option<String>,
    pub tool_metadata: JsonObject,
    pub input_refs: Vec<StateRecordRef>,
    pub source_refs: Vec<SourceRef>,
    pub output_artifact_refs: Vec<ArtifactRef>,
    pub limitations: Vec<String>,
    pub observed_at: UtcTimestamp,
    pub recorded_at: UtcTimestamp,
    pub metadata: PersistedEvidenceObservationAuthority,
}

#[derive(Debug)]
struct EvidenceSummaryRecordRaw {
    project_id: String,
    evidence_summary_id: String,
    task_id: String,
    change_unit_id: Option<String>,
    produced_at_state_version: u64,
    status: String,
    coverage_json: String,
    supporting_refs_json: String,
    gap_refs_json: String,
    metadata_json: String,
}

#[derive(Debug)]
struct EvidenceObservationRecordRaw {
    project_id: String,
    evidence_observation_id: String,
    task_id: String,
    change_unit_id: Option<String>,
    run_id: Option<String>,
    acceptance_criterion_id: Option<String>,
    evidence_claim_id: Option<String>,
    source_kind: String,
    assurance_level: String,
    observed_by_actor_source: Option<String>,
    tool_name: Option<String>,
    tool_invocation_id: Option<String>,
    tool_metadata_json: String,
    input_refs_json: String,
    source_refs_json: String,
    output_artifact_refs_json: String,
    limitations_json: String,
    observed_at: String,
    recorded_at: String,
    metadata_json: String,
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
            record_kind: StateRecordKind::EvidenceObservation,
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
            evidence_summary_record_raw_from_row,
        )
        .optional()?;
    let record = record.map(decode_evidence_summary_record).transpose()?;
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
            evidence_summary_record_raw_from_row,
        )
        .optional()?;
    let record = record.map(decode_evidence_summary_record).transpose()?;
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

fn evidence_summary_record_raw_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<EvidenceSummaryRecordRaw> {
    Ok(EvidenceSummaryRecordRaw {
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

fn decode_evidence_summary_record(
    raw: EvidenceSummaryRecordRaw,
) -> StoreResult<EvidenceSummaryRecord> {
    let record_ref = raw.evidence_summary_id.clone();
    let coverage = decode_owner_json_text(
        "evidence_summaries",
        record_ref.clone(),
        "coverage_json",
        &raw.coverage_json,
    )?;
    let supporting_refs = decode_owner_json_text(
        "evidence_summaries",
        record_ref.clone(),
        "supporting_refs_json",
        &raw.supporting_refs_json,
    )?;
    let gap_refs = decode_owner_json_text(
        "evidence_summaries",
        record_ref.clone(),
        "gap_refs_json",
        &raw.gap_refs_json,
    )?;
    let metadata = decode_owner_json_text(
        "evidence_summaries",
        record_ref,
        "metadata_json",
        &raw.metadata_json,
    )?;
    let status = decode_owner_closed_value(
        "evidence_summaries",
        raw.evidence_summary_id.as_str(),
        "status",
        &raw.status,
    )?;
    Ok(EvidenceSummaryRecord {
        project_id: raw.project_id,
        evidence_summary_id: raw.evidence_summary_id,
        task_id: raw.task_id,
        change_unit_id: raw.change_unit_id,
        produced_at_state_version: raw.produced_at_state_version,
        status,
        coverage,
        supporting_refs,
        gap_refs,
        metadata,
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
        evidence_observation_record_raw_from_row,
    )
    .optional()
    .map_err(StoreError::from)?
    .map(decode_evidence_observation_record)
    .transpose()
}

fn evidence_observation_record_raw_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<EvidenceObservationRecordRaw> {
    Ok(EvidenceObservationRecordRaw {
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

fn decode_evidence_observation_record(
    raw: EvidenceObservationRecordRaw,
) -> StoreResult<EvidenceObservationRecord> {
    let corrupt_value = |column| {
        StoreError::corrupt_owner_state_value(
            "evidence_observations",
            raw.evidence_observation_id.clone(),
            column,
        )
    };
    let source_kind = decode_owner_closed_value(
        "evidence_observations",
        raw.evidence_observation_id.as_str(),
        "source_kind",
        &raw.source_kind,
    )?;
    let assurance_level = decode_owner_closed_value(
        "evidence_observations",
        raw.evidence_observation_id.as_str(),
        "assurance_level",
        &raw.assurance_level,
    )?;
    let observed_by_actor_source = raw
        .observed_by_actor_source
        .as_deref()
        .map(str::parse)
        .transpose()
        .map_err(|_| corrupt_value("observed_by_actor_source"))?;
    let record_id = raw.evidence_observation_id.clone();
    let tool_metadata = decode_owner_json_text(
        "evidence_observations",
        record_id.clone(),
        "tool_metadata_json",
        &raw.tool_metadata_json,
    )?;
    let input_refs = decode_owner_json_text(
        "evidence_observations",
        record_id.clone(),
        "input_refs_json",
        &raw.input_refs_json,
    )?;
    let source_refs = decode_owner_json_text(
        "evidence_observations",
        record_id.clone(),
        "source_refs_json",
        &raw.source_refs_json,
    )?;
    let output_artifact_refs = decode_owner_json_text(
        "evidence_observations",
        record_id.clone(),
        "output_artifact_refs_json",
        &raw.output_artifact_refs_json,
    )?;
    let limitations = decode_owner_json_text(
        "evidence_observations",
        record_id.clone(),
        "limitations_json",
        &raw.limitations_json,
    )?;
    let metadata = decode_owner_json_text(
        "evidence_observations",
        record_id,
        "metadata_json",
        &raw.metadata_json,
    )?;
    let observed_at =
        UtcTimestamp::parse(&raw.observed_at).map_err(|_| corrupt_value("observed_at"))?;
    let recorded_at =
        UtcTimestamp::parse(&raw.recorded_at).map_err(|_| corrupt_value("recorded_at"))?;
    Ok(EvidenceObservationRecord {
        project_id: raw.project_id,
        evidence_observation_id: raw.evidence_observation_id,
        task_id: raw.task_id,
        change_unit_id: raw.change_unit_id,
        run_id: raw.run_id,
        acceptance_criterion_id: raw.acceptance_criterion_id,
        evidence_claim_id: raw.evidence_claim_id,
        source_kind,
        assurance_level,
        observed_by_actor_source,
        tool_name: raw.tool_name,
        tool_invocation_id: raw.tool_invocation_id,
        tool_metadata,
        input_refs,
        source_refs,
        output_artifact_refs,
        limitations,
        observed_at,
        recorded_at,
        metadata,
    })
}

fn producer_kind_for_capture(capture: &EvidenceCaptureSpec) -> EvidenceProducerKind {
    match capture {
        EvidenceCaptureSpec::VerifiedCommandExecution { .. } => {
            EvidenceProducerKind::VerifiedCommandExecution
        }
        EvidenceCaptureSpec::VerifiedToolInvocation { .. } => {
            EvidenceProducerKind::VerifiedToolInvocation
        }
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
        validate_identifier("baseline_ref", input.baseline_ref.as_str())?;
        validate_artifact_sha256("input_sha256", &input.input_sha256)?;
        validate_identifier(
            "requesting_connection_internal_id",
            input.requesting_connection_internal_id.as_str(),
        )?;
        if evidence_capture_input_sha256(&input.capture).map_err(|_| StoreError::InvalidInput {
            detail: "capture could not provide its immutable input digest".to_owned(),
        })? != input.input_sha256
        {
            return Err(StoreError::InvalidInput {
                detail: "input_sha256 does not match the evidence capture".to_owned(),
            });
        }
        validate_evidence_capture_expected_outcome(&input.capture, &input.expected_outcome)
            .map_err(|detail| StoreError::InvalidInput {
                detail: format!("expected_outcome does not match the evidence capture: {detail}"),
            })?;
        let created_at = input.created_at.to_canonical_string();
        let expires_at = input.expires_at.to_canonical_string();
        validate_evidence_capture_intent_window(&created_at, &expires_at).map_err(|field| {
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
        })?;
        let target_json =
            encode_json_column("evidence_capture_intents.target_json", &input.target)?;
        let capture_kind = encode_closed_value(
            "evidence_capture_intents.capture_kind",
            &producer_kind_for_capture(&input.capture),
        )?;
        let capture_spec_json =
            encode_json_column("evidence_capture_intents.capture_spec_json", &input.capture)?;
        let expected_outcome_json = encode_json_column(
            "evidence_capture_intents.expected_outcome_json",
            &input.expected_outcome,
        )?;
        let session_context_json = encode_json_column(
            "evidence_capture_intents.session_context_json",
            &input.session_context,
        )?;
        let workspace_context_json = encode_json_column(
            "evidence_capture_intents.workspace_context_json",
            &input.workspace_context,
        )?;
        let metadata_json =
            encode_json_column("evidence_capture_intents.metadata_json", &input.metadata)?;

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
                input.baseline_ref.as_str(),
                target_json,
                capture_kind,
                capture_spec_json,
                input.input_sha256,
                expected_outcome_json,
                input.requested_by_actor_source.to_canonical_string(),
                input.requesting_connection_internal_id.as_str(),
                session_context_json,
                workspace_context_json,
                created_at,
                expires_at,
                metadata_json
            ],
        )?;
        Ok(())
    }

    fn upsert_evidence_summary(
        &mut self,
        input: &EvidenceSummaryUpsert,
        committed_state_version: u64,
    ) -> StoreResult<()> {
        let status = encode_closed_value("evidence_summaries.status", &input.status)?;
        let coverage_json =
            encode_json_column("evidence_summaries.coverage_json", &input.coverage)?;
        let supporting_refs_json = encode_json_column(
            "evidence_summaries.supporting_refs_json",
            &input.supporting_refs,
        )?;
        let gap_refs_json =
            encode_json_column("evidence_summaries.gap_refs_json", &input.gap_refs)?;
        let metadata_json =
            encode_json_column("evidence_summaries.metadata_json", &input.metadata)?;
        validate_identifier("evidence_summary_id", &input.evidence_summary_id)?;
        validate_identifier("task_id", &input.task_id)?;
        if let Some(change_unit_id) = &input.change_unit_id {
            validate_identifier("change_unit_id", change_unit_id)?;
        }
        validate_identifier("evidence_summaries.status", &status)?;
        validate_evidence_coverage_json("evidence_summaries.coverage_json", &coverage_json)?;
        validate_state_refs_json(
            "evidence_summaries.supporting_refs_json",
            &supporting_refs_json,
        )?;
        validate_state_refs_json("evidence_summaries.gap_refs_json", &gap_refs_json)?;
        validate_evidence_metadata_json("evidence_summaries.metadata_json", &metadata_json)?;
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
                status,
                coverage_json,
                supporting_refs_json,
                gap_refs_json,
                self.committed_at,
                metadata_json
            ],
        )?;
        Ok(())
    }

    fn insert_evidence_observation(
        &mut self,
        input: &EvidenceObservationInsert,
    ) -> StoreResult<()> {
        let source_kind =
            encode_closed_value("evidence_observations.source_kind", &input.source_kind)?;
        let assurance_level = encode_closed_value(
            "evidence_observations.assurance_level",
            &input.assurance_level,
        )?;
        let observed_by_actor_source = input
            .observed_by_actor_source
            .as_ref()
            .map(ActorSource::to_canonical_string);
        let tool_metadata_json = encode_json_column(
            "evidence_observations.tool_metadata_json",
            &input.tool_metadata,
        )?;
        let input_refs_json =
            encode_json_column("evidence_observations.input_refs_json", &input.input_refs)?;
        let source_refs_json =
            encode_json_column("evidence_observations.source_refs_json", &input.source_refs)?;
        let output_artifact_refs_json = encode_json_column(
            "evidence_observations.output_artifact_refs_json",
            &input.output_artifact_refs,
        )?;
        let limitations_json =
            encode_json_column("evidence_observations.limitations_json", &input.limitations)?;
        let observed_at = input.observed_at.to_string();
        let recorded_at = input.recorded_at.to_string();
        let metadata_json =
            encode_json_column("evidence_observations.metadata_json", &input.metadata)?;
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
        validate_evidence_source_kind("evidence_observations.source_kind", &source_kind)?;
        validate_evidence_assurance_level(
            "evidence_observations.assurance_level",
            &assurance_level,
        )?;
        if let Some(actor_source) = &observed_by_actor_source {
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
            &tool_metadata_json,
        )?;
        validate_state_refs_json("evidence_observations.input_refs_json", &input_refs_json)?;
        validate_source_refs_json("evidence_observations.source_refs_json", &source_refs_json)?;
        validate_artifact_refs_json(
            "evidence_observations.output_artifact_refs_json",
            &output_artifact_refs_json,
        )?;
        validate_string_list_json("evidence_observations.limitations_json", &limitations_json)?;
        validate_timestamp("observed_at", &observed_at)?;
        validate_timestamp("recorded_at", &recorded_at)?;
        validate_evidence_observation_metadata_json(
            "evidence_observations.metadata_json",
            &metadata_json,
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
                source_kind,
                assurance_level,
                observed_by_actor_source,
                input.tool_name,
                input.tool_invocation_id,
                tool_metadata_json,
                input_refs_json,
                source_refs_json,
                output_artifact_refs_json,
                limitations_json,
                observed_at,
                recorded_at,
                metadata_json
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
        let producer_kind =
            encode_closed_value("evidence_producers.producer_kind", &input.producer_kind)?;
        let canonical_producer_json =
            canonical_json_string(&input.canonical_producer).map_err(|_| {
                StoreError::InvalidInput {
                    detail: "evidence_producers.canonical_producer_json could not be serialized"
                        .to_owned(),
                }
            })?;
        let created_at = input.created_at.to_canonical_string();
        validate_timestamp("created_at", &created_at)?;
        let metadata_json =
            encode_json_column("evidence_producers.metadata_json", &input.metadata)?;

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
                input.baseline_ref.as_str(),
                producer_kind,
                canonical_producer_json,
                created_at,
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
    fn evidence_summary_decoder_rejects_a_negative_state_version() {
        let connection = Connection::open_in_memory().expect("in-memory database must open");
        let error = connection
            .query_row(
                "SELECT 'project', 'summary', 'task', NULL, -1, 'current',
                        '{}', '[]', '[]', '{}'",
                [],
                evidence_summary_record_raw_from_row,
            )
            .expect_err("negative authority order must fail closed");

        assert!(matches!(
            error,
            rusqlite::Error::FromSqlConversionFailure(..)
        ));
    }

    #[test]
    fn evidence_summary_decoder_owns_json_corruption_errors() {
        let valid_metadata = r#"{"updated_by_run_id":"run-test"}"#;
        let cases = [
            (
                "coverage_json",
                r#"{"target":"not-an-array"}"#,
                "[]",
                "[]",
                valid_metadata,
            ),
            (
                "supporting_refs_json",
                "[]",
                r#"{"record_kind":"run"}"#,
                "[]",
                valid_metadata,
            ),
            (
                "gap_refs_json",
                "[]",
                "[]",
                r#"{"record_kind":"run"}"#,
                valid_metadata,
            ),
            (
                "metadata_json",
                "[]",
                "[]",
                "[]",
                r#"{"updated_by_run_id":123}"#,
            ),
        ];

        for (column, coverage_json, supporting_refs_json, gap_refs_json, metadata_json) in cases {
            let error = decode_evidence_summary_record(EvidenceSummaryRecordRaw {
                project_id: "project".to_owned(),
                evidence_summary_id: "summary".to_owned(),
                task_id: "task".to_owned(),
                change_unit_id: Some("change".to_owned()),
                produced_at_state_version: 2,
                status: "current".to_owned(),
                coverage_json: coverage_json.to_owned(),
                supporting_refs_json: supporting_refs_json.to_owned(),
                gap_refs_json: gap_refs_json.to_owned(),
                metadata_json: metadata_json.to_owned(),
            })
            .expect_err("malformed evidence owner JSON must fail in the Store decoder");

            assert!(matches!(
                error,
                StoreError::CorruptOwnerStateJson {
                    table: "evidence_summaries",
                    logical_column,
                    ..
                } if logical_column == column
            ));
        }
    }

    #[test]
    fn evidence_observation_decoder_owns_source_and_metadata_corruption() {
        let raw = || EvidenceObservationRecordRaw {
            project_id: "project".to_owned(),
            evidence_observation_id: "observation".to_owned(),
            task_id: "task".to_owned(),
            change_unit_id: Some("change".to_owned()),
            run_id: Some("run-test".to_owned()),
            acceptance_criterion_id: Some("criterion".to_owned()),
            evidence_claim_id: None,
            source_kind: "user_observation".to_owned(),
            assurance_level: "user_observed".to_owned(),
            observed_by_actor_source: Some("local_user".to_owned()),
            tool_name: None,
            tool_invocation_id: None,
            tool_metadata_json: "{}".to_owned(),
            input_refs_json: "[]".to_owned(),
            source_refs_json: "[]".to_owned(),
            output_artifact_refs_json: "[]".to_owned(),
            limitations_json: "[]".to_owned(),
            observed_at: "2026-01-01T00:00:00Z".to_owned(),
            recorded_at: "2026-01-01T00:00:00Z".to_owned(),
            metadata_json: r#"{
                "recorded_by_run_id":"run-test",
                "invocation_verification_basis":"verified",
                "producer_anchor":{
                    "producer_kind":"user_observation",
                    "producer_identity":"local_user",
                    "output_artifact_ids":[]
                },
                "relevance_assessment":{
                    "status":"supported",
                    "summary":"supported",
                    "assessed_by_actor_source":"local_user"
                }
            }"#
            .to_owned(),
        };

        let mut bad_source_refs = raw();
        bad_source_refs.source_refs_json = r#"{"context_id":"message"}"#.to_owned();
        assert!(matches!(
            decode_evidence_observation_record(bad_source_refs),
            Err(StoreError::CorruptOwnerStateJson {
                table: "evidence_observations",
                logical_column: "source_refs_json",
                ..
            })
        ));

        let mut bad_metadata = raw();
        bad_metadata.metadata_json = r#"{"recorded_by_run_id":123}"#.to_owned();
        assert!(matches!(
            decode_evidence_observation_record(bad_metadata),
            Err(StoreError::CorruptOwnerStateJson {
                table: "evidence_observations",
                logical_column: "metadata_json",
                ..
            })
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
