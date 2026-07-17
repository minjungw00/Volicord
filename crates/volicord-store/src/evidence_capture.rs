use std::fs;

use chrono::Duration;
use rusqlite::{params, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};
use volicord_types::{
    canonical_json_bare_sha256, canonical_json_string, evidence_capture_input_sha256,
    validate_evidence_capture_expected_outcome, validate_evidence_capture_limitations,
    validate_evidence_capture_observed_outcome, EvidenceCaptureSpec, EvidenceProducerKind,
    JsonObject, PersistedEvidenceCaptureReceiptBody, RedactionState, UtcTimestamp,
    EVIDENCE_CAPTURE_INTENT_TTL_MINUTES, EVIDENCE_CAPTURE_RECEIPT_CONTRACT_ID,
};

use crate::{
    artifacts::{
        insert_artifact_staging_tx, validate_insert as validate_staging_insert,
        ArtifactStagingInsert, StagedPayloadKind,
    },
    core_pipeline::{advance_project_utc_floor_tx, CoreProjectStore},
    sqlite::{begin_immediate_transaction, ARTIFACTS_DIR, ARTIFACTS_TMP_DIR},
    StoreError, StoreResult,
};

/// Maximum serialized safe receipt body accepted by the source-fulfillment path.
pub const MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES: usize = 24 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvidenceCaptureIntentWindowError {
    CreatedAt,
    ExpiresAt,
}

pub(crate) fn validate_evidence_capture_intent_window(
    created_at: &str,
    expires_at: &str,
) -> Result<(UtcTimestamp, UtcTimestamp), EvidenceCaptureIntentWindowError> {
    let created_at =
        UtcTimestamp::parse(created_at).map_err(|_| EvidenceCaptureIntentWindowError::CreatedAt)?;
    created_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| EvidenceCaptureIntentWindowError::CreatedAt)?;
    let expires_at =
        UtcTimestamp::parse(expires_at).map_err(|_| EvidenceCaptureIntentWindowError::ExpiresAt)?;
    expires_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| EvidenceCaptureIntentWindowError::ExpiresAt)?;
    let expected_expires_at = created_at
        .checked_add(Duration::minutes(EVIDENCE_CAPTURE_INTENT_TTL_MINUTES))
        .map_err(|_| EvidenceCaptureIntentWindowError::ExpiresAt)?;
    if expires_at != expected_expires_at {
        return Err(EvidenceCaptureIntentWindowError::ExpiresAt);
    }
    Ok((created_at, expires_at))
}

/// Storage input for inserting one immutable evidence-capture intent in a Core commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceCaptureIntentInsert {
    pub evidence_capture_intent_id: String,
    pub task_id: String,
    pub change_unit_id: String,
    pub scope_revision: u64,
    pub baseline_ref: String,
    pub target_json: String,
    pub capture_kind: String,
    pub capture_spec_json: String,
    pub input_sha256: String,
    pub expected_outcome_json: String,
    pub requested_by_actor_source: String,
    pub requesting_connection_internal_id: String,
    pub session_context_json: String,
    pub workspace_context_json: String,
    pub created_at: String,
    pub expires_at: String,
    pub metadata_json: String,
}

/// Stored immutable evidence-capture intent facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceCaptureIntentRecord {
    pub project_id: String,
    pub evidence_capture_intent_id: String,
    pub task_id: String,
    pub change_unit_id: String,
    pub scope_revision: u64,
    pub baseline_ref: String,
    pub target_json: String,
    pub capture_kind: String,
    pub capture_spec_json: String,
    pub input_sha256: String,
    pub expected_outcome_json: String,
    pub requested_by_actor_source: String,
    pub requesting_connection_internal_id: String,
    pub session_context_json: String,
    pub workspace_context_json: String,
    pub created_at: String,
    pub expires_at: String,
    pub metadata_json: String,
}

/// Source-neutral input for atomically staging and recording one complete safe receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceCaptureReceiptInsert {
    pub evidence_capture_receipt_id: String,
    pub evidence_capture_intent_id: String,
    pub staging_handle_id: String,
    pub task_id: String,
    pub capture_kind: String,
    pub input_sha256: String,
    pub result_sha256: String,
    pub expected_outcome_json: String,
    pub observed_outcome_json: String,
    pub source_refs_json: String,
    pub observed_by_actor_source: String,
    pub observed_at: String,
    pub limitations_json: String,
    pub safe_receipt_json: String,
    pub created_at: String,
    pub staging_expires_at: String,
    pub metadata_json: String,
}

/// Stored immutable source receipt facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceCaptureReceiptRecord {
    pub project_id: String,
    pub evidence_capture_receipt_id: String,
    pub evidence_capture_intent_id: String,
    pub staging_handle_id: String,
    pub capture_kind: String,
    pub input_sha256: String,
    pub result_sha256: String,
    pub expected_outcome_json: String,
    pub observed_outcome_json: String,
    pub source_refs_json: String,
    pub observed_by_actor_source: String,
    pub observed_at: String,
    pub completeness: String,
    pub limitations_json: String,
    pub safe_receipt_json: String,
    pub safe_receipt_sha256: String,
    pub safe_receipt_size_bytes: u64,
    pub created_at: String,
    pub metadata_json: String,
}

/// Project-scoped identity class for one claimed underlying source fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceCaptureSourceClaimKind {
    HostInvocation,
}

impl EvidenceCaptureSourceClaimKind {
    /// Returns the stable storage value for this claim class.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HostInvocation => "host_invocation",
        }
    }
}

/// Immutable claim that one underlying source fact fulfilled one capture receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceCaptureSourceClaimRecord {
    pub project_id: String,
    pub source_claim_kind: String,
    pub source_claim_id: String,
    pub evidence_capture_intent_id: String,
    pub evidence_capture_receipt_id: String,
    pub capture_kind: String,
    pub claimed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceCaptureSourceClaimIdentity {
    pub source_claim_kind: EvidenceCaptureSourceClaimKind,
    pub source_claim_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedEvidenceCaptureSource {
    claims: Vec<EvidenceCaptureSourceClaimIdentity>,
}

/// Storage input for inserting one canonical evidence producer in a Core commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceProducerInsert {
    pub evidence_producer_id: String,
    pub evidence_capture_intent_id: String,
    pub evidence_capture_receipt_id: String,
    pub evidence_observation_id: String,
    pub artifact_id: String,
    pub run_id: String,
    pub task_id: String,
    pub change_unit_id: String,
    pub scope_revision: u64,
    pub baseline_ref: String,
    pub producer_kind: String,
    pub canonical_producer_json: String,
    pub created_at: String,
    pub metadata_json: String,
}

/// Stored canonical evidence-producer facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceProducerRecord {
    pub project_id: String,
    pub evidence_producer_id: String,
    pub evidence_capture_intent_id: String,
    pub evidence_capture_receipt_id: String,
    pub evidence_observation_id: String,
    pub artifact_id: String,
    pub run_id: String,
    pub task_id: String,
    pub change_unit_id: String,
    pub scope_revision: u64,
    pub baseline_ref: String,
    pub producer_kind: String,
    pub canonical_producer_json: String,
    pub created_at: String,
    pub metadata_json: String,
}

impl CoreProjectStore {
    /// Reads one immutable capture intent by exact project-local identity.
    pub fn evidence_capture_intent_record(
        &self,
        intent_id: &str,
    ) -> StoreResult<Option<EvidenceCaptureIntentRecord>> {
        read_intent(&self.conn, &self.project.project_id, intent_id)
    }

    /// Reads one immutable source receipt by exact project-local identity.
    pub fn evidence_capture_receipt_record(
        &self,
        receipt_id: &str,
    ) -> StoreResult<Option<EvidenceCaptureReceiptRecord>> {
        read_receipt(&self.conn, &self.project.project_id, receipt_id)
    }

    /// Reads the unique immutable source receipt for one capture intent.
    pub fn evidence_capture_receipt_for_intent(
        &self,
        intent_id: &str,
    ) -> StoreResult<Option<EvidenceCaptureReceiptRecord>> {
        read_receipt_for_intent(&self.conn, &self.project.project_id, intent_id)
    }

    /// Reads one source-fact claim by exact project-local source identity.
    pub fn evidence_capture_source_claim_record(
        &self,
        claim_kind: EvidenceCaptureSourceClaimKind,
        claim_id: &str,
    ) -> StoreResult<Option<EvidenceCaptureSourceClaimRecord>> {
        read_source_claim(&self.conn, &self.project.project_id, claim_kind, claim_id)
    }

    /// Reads the complete source-fact claim set owned by one capture receipt.
    pub fn evidence_capture_source_claims_for_receipt(
        &self,
        receipt_id: &str,
    ) -> StoreResult<Vec<EvidenceCaptureSourceClaimRecord>> {
        read_source_claims_for_receipt(&self.conn, &self.project.project_id, receipt_id)
    }

    /// Revalidates the exact persisted claim set for one intent/receipt source chain.
    pub fn validate_evidence_capture_source_claims_for_receipt(
        &self,
        intent: &EvidenceCaptureIntentRecord,
        receipt: &EvidenceCaptureReceiptRecord,
        capture_spec: &EvidenceCaptureSpec,
        body: &PersistedEvidenceCaptureReceiptBody,
    ) -> StoreResult<()> {
        if intent.project_id != self.project.project_id
            || receipt.project_id != self.project.project_id
            || receipt.evidence_capture_intent_id != intent.evidence_capture_intent_id
            || receipt.capture_kind != intent.capture_kind
            || body.capture_intent_id.as_str() != intent.evidence_capture_intent_id
            || body.source.connection_id.as_str() != intent.requesting_connection_internal_id
        {
            return Err(StoreError::corrupt_owner_state_value(
                "evidence_capture_source_claims",
                receipt.evidence_capture_receipt_id.clone(),
                "source_chain",
            ));
        }
        let mut expected = derive_evidence_capture_source_claims(capture_spec, body)?;
        expected.sort_by(|left, right| {
            (
                left.source_claim_kind.as_str(),
                left.source_claim_id.as_str(),
            )
                .cmp(&(
                    right.source_claim_kind.as_str(),
                    right.source_claim_id.as_str(),
                ))
        });
        let actual =
            self.evidence_capture_source_claims_for_receipt(&receipt.evidence_capture_receipt_id)?;
        let exact = actual.len() == expected.len()
            && actual
                .iter()
                .zip(expected.iter())
                .all(|(record, identity)| {
                    record.project_id == self.project.project_id
                        && record.source_claim_kind == identity.source_claim_kind.as_str()
                        && record.source_claim_id == identity.source_claim_id
                        && record.evidence_capture_intent_id == intent.evidence_capture_intent_id
                        && record.evidence_capture_receipt_id == receipt.evidence_capture_receipt_id
                        && record.capture_kind == intent.capture_kind
                        && record.claimed_at == receipt.created_at
                });
        if exact {
            Ok(())
        } else {
            Err(StoreError::corrupt_owner_state_value(
                "evidence_capture_source_claims",
                receipt.evidence_capture_receipt_id.clone(),
                "claim_set",
            ))
        }
    }

    /// Reads one canonical evidence producer by exact project-local identity.
    pub fn evidence_producer_record(
        &self,
        producer_id: &str,
    ) -> StoreResult<Option<EvidenceProducerRecord>> {
        read_producer(&self.conn, &self.project.project_id, producer_id)
    }

    /// Reads the unique finalized producer for one capture intent.
    pub fn evidence_producer_for_intent(
        &self,
        intent_id: &str,
    ) -> StoreResult<Option<EvidenceProducerRecord>> {
        read_producer_for_intent(&self.conn, &self.project.project_id, intent_id)
    }

    /// Atomically stages bounded safe receipt bytes and inserts one source receipt.
    ///
    /// This storage-owned source transition does not advance project state, append an
    /// authority event, or create a replay row. A duplicate intent is rejected by the
    /// receipt uniqueness constraint, and any newly written staging body is removed if
    /// the SQLite transaction does not commit.
    pub fn fulfill_evidence_capture_source(
        &mut self,
        input: EvidenceCaptureReceiptInsert,
    ) -> StoreResult<EvidenceCaptureReceiptRecord> {
        validate_receipt_input(&input)?;
        let intent = self
            .evidence_capture_intent_record(&input.evidence_capture_intent_id)?
            .ok_or_else(|| StoreError::NotFound {
                entity: "evidence_capture_intent",
                id: input.evidence_capture_intent_id.clone(),
            })?;
        let validated_source = validate_receipt_against_intent(&input, &intent)?;
        let created_at =
            UtcTimestamp::parse(&input.created_at).map_err(|_| StoreError::InvalidInput {
                detail: "created_at must be a valid RFC 3339 timestamp".to_owned(),
            })?;

        let safe_bytes = input.safe_receipt_json.as_bytes().to_vec();
        let safe_receipt_sha256 = sha256_hex(&safe_bytes);
        let safe_receipt_size_bytes =
            u64::try_from(safe_bytes.len()).map_err(|_| StoreError::InvalidInput {
                detail: "safe evidence-capture receipt size does not fit in u64".to_owned(),
            })?;
        let staging = ArtifactStagingInsert {
            handle_id: input.staging_handle_id.clone(),
            task_id: input.task_id.clone(),
            created_by_actor_source: input.observed_by_actor_source.clone(),
            display_name: "evidence-capture-receipt.json".to_owned(),
            content_type: "application/json".to_owned(),
            sha256: safe_receipt_sha256.clone(),
            size_bytes: safe_receipt_size_bytes,
            redaction_state: "redacted".to_owned(),
            relation_hint: Some("evidence capture receipt".to_owned()),
            payload_kind: StagedPayloadKind::SafeTextBody,
            safe_bytes_or_notice: safe_bytes,
            created_at: input.created_at.clone(),
            expires_at: input.staging_expires_at.clone(),
        };
        validate_staging_insert(&staging)?;

        let tmp_dir = self
            .project
            .project_home
            .join(ARTIFACTS_DIR)
            .join(ARTIFACTS_TMP_DIR);
        fs::create_dir_all(&tmp_dir)?;
        let tx = begin_immediate_transaction(&mut self.conn)?;
        let clock_floor = advance_project_utc_floor_tx(&tx, &self.project.project_id, &created_at)?;
        let (_, write_path) =
            insert_artifact_staging_tx(&tx, &self.project.project_id, &tmp_dir, staging)?;

        let insert_result = tx.execute(
            "INSERT INTO evidence_capture_receipts (
                project_id,
                evidence_capture_receipt_id,
                evidence_capture_intent_id,
                staging_handle_id,
                capture_kind,
                input_sha256,
                result_sha256,
                expected_outcome_json,
                observed_outcome_json,
                source_refs_json,
                observed_by_actor_source,
                observed_at,
                completeness,
                limitations_json,
                safe_receipt_json,
                safe_receipt_sha256,
                safe_receipt_size_bytes,
                created_at,
                metadata_json
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, 'complete', ?13, ?14, ?15, ?16, ?17, ?18
            )",
            params![
                self.project.project_id,
                input.evidence_capture_receipt_id,
                input.evidence_capture_intent_id,
                input.staging_handle_id,
                input.capture_kind,
                input.input_sha256,
                input.result_sha256,
                input.expected_outcome_json,
                input.observed_outcome_json,
                input.source_refs_json,
                input.observed_by_actor_source,
                input.observed_at,
                input.limitations_json,
                input.safe_receipt_json,
                safe_receipt_sha256,
                i64::try_from(safe_receipt_size_bytes).map_err(|_| StoreError::InvalidInput {
                    detail: "safe evidence-capture receipt size does not fit in SQLite integer"
                        .to_owned(),
                })?,
                input.created_at,
                input.metadata_json
            ],
        );
        if let Err(error) = insert_result {
            let _ = fs::remove_file(&write_path);
            return Err(StoreError::from(error));
        }

        for claim in validated_source.claims {
            if let Err(error) = tx.execute(
                "INSERT INTO evidence_capture_source_claims (
                    project_id,
                    source_claim_kind,
                    source_claim_id,
                    evidence_capture_intent_id,
                    evidence_capture_receipt_id,
                    capture_kind,
                    claimed_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    self.project.project_id,
                    claim.source_claim_kind.as_str(),
                    claim.source_claim_id,
                    input.evidence_capture_intent_id,
                    input.evidence_capture_receipt_id,
                    input.capture_kind,
                    input.created_at,
                ],
            ) {
                let _ = fs::remove_file(&write_path);
                return Err(StoreError::from(error));
            }
        }

        if let Err(error) = tx.commit() {
            let _ = fs::remove_file(&write_path);
            return Err(StoreError::from(error));
        }
        self.remember_clock_sample(&clock_floor);

        self.evidence_capture_receipt_record(&input.evidence_capture_receipt_id)?
            .ok_or_else(|| {
                StoreError::schema_invariant(
                    "project_state",
                    "committed evidence-capture receipt could not be read",
                )
            })
    }
}

fn read_intent(
    conn: &rusqlite::Connection,
    project_id: &str,
    intent_id: &str,
) -> StoreResult<Option<EvidenceCaptureIntentRecord>> {
    let record = conn
        .query_row(
            "SELECT project_id, evidence_capture_intent_id, task_id, change_unit_id,
                scope_revision, baseline_ref, target_json, capture_kind,
                capture_spec_json, input_sha256, expected_outcome_json,
                requested_by_actor_source, requesting_connection_internal_id,
                session_context_json, workspace_context_json, created_at, expires_at,
                metadata_json
           FROM evidence_capture_intents
          WHERE project_id = ?1 AND evidence_capture_intent_id = ?2",
            params![project_id, intent_id],
            |row| {
                let scope_revision = row.get::<_, i64>(4)?;
                Ok(EvidenceCaptureIntentRecord {
                    project_id: row.get(0)?,
                    evidence_capture_intent_id: row.get(1)?,
                    task_id: row.get(2)?,
                    change_unit_id: row.get(3)?,
                    scope_revision: nonnegative(scope_revision, 4)?,
                    baseline_ref: row.get(5)?,
                    target_json: row.get(6)?,
                    capture_kind: row.get(7)?,
                    capture_spec_json: row.get(8)?,
                    input_sha256: row.get(9)?,
                    expected_outcome_json: row.get(10)?,
                    requested_by_actor_source: row.get(11)?,
                    requesting_connection_internal_id: row.get(12)?,
                    session_context_json: row.get(13)?,
                    workspace_context_json: row.get(14)?,
                    created_at: row.get(15)?,
                    expires_at: row.get(16)?,
                    metadata_json: row.get(17)?,
                })
            },
        )
        .optional()
        .map_err(StoreError::from)?;
    record.map(validate_intent_record).transpose()
}

fn validate_intent_record(
    record: EvidenceCaptureIntentRecord,
) -> StoreResult<EvidenceCaptureIntentRecord> {
    validate_evidence_capture_intent_window(&record.created_at, &record.expires_at).map_err(
        |field| {
            StoreError::corrupt_owner_state_value(
                "evidence_capture_intents",
                record.evidence_capture_intent_id.clone(),
                match field {
                    EvidenceCaptureIntentWindowError::CreatedAt => "created_at",
                    EvidenceCaptureIntentWindowError::ExpiresAt => "expires_at",
                },
            )
        },
    )?;
    Ok(record)
}

fn read_receipt(
    conn: &rusqlite::Connection,
    project_id: &str,
    receipt_id: &str,
) -> StoreResult<Option<EvidenceCaptureReceiptRecord>> {
    conn.query_row(
        "SELECT project_id, evidence_capture_receipt_id, evidence_capture_intent_id,
                staging_handle_id, capture_kind, input_sha256, result_sha256,
                expected_outcome_json, observed_outcome_json, source_refs_json,
                observed_by_actor_source, observed_at, completeness, limitations_json,
                safe_receipt_json, safe_receipt_sha256, safe_receipt_size_bytes,
                created_at, metadata_json
           FROM evidence_capture_receipts
          WHERE project_id = ?1 AND evidence_capture_receipt_id = ?2",
        params![project_id, receipt_id],
        receipt_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn read_receipt_for_intent(
    conn: &rusqlite::Connection,
    project_id: &str,
    intent_id: &str,
) -> StoreResult<Option<EvidenceCaptureReceiptRecord>> {
    conn.query_row(
        "SELECT project_id, evidence_capture_receipt_id, evidence_capture_intent_id,
                staging_handle_id, capture_kind, input_sha256, result_sha256,
                expected_outcome_json, observed_outcome_json, source_refs_json,
                observed_by_actor_source, observed_at, completeness, limitations_json,
                safe_receipt_json, safe_receipt_sha256, safe_receipt_size_bytes,
                created_at, metadata_json
           FROM evidence_capture_receipts
          WHERE project_id = ?1 AND evidence_capture_intent_id = ?2",
        params![project_id, intent_id],
        receipt_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn receipt_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvidenceCaptureReceiptRecord> {
    let size = row.get::<_, i64>(16)?;
    Ok(EvidenceCaptureReceiptRecord {
        project_id: row.get(0)?,
        evidence_capture_receipt_id: row.get(1)?,
        evidence_capture_intent_id: row.get(2)?,
        staging_handle_id: row.get(3)?,
        capture_kind: row.get(4)?,
        input_sha256: row.get(5)?,
        result_sha256: row.get(6)?,
        expected_outcome_json: row.get(7)?,
        observed_outcome_json: row.get(8)?,
        source_refs_json: row.get(9)?,
        observed_by_actor_source: row.get(10)?,
        observed_at: row.get(11)?,
        completeness: row.get(12)?,
        limitations_json: row.get(13)?,
        safe_receipt_json: row.get(14)?,
        safe_receipt_sha256: row.get(15)?,
        safe_receipt_size_bytes: nonnegative(size, 16)?,
        created_at: row.get(17)?,
        metadata_json: row.get(18)?,
    })
}

fn read_source_claim(
    conn: &rusqlite::Connection,
    project_id: &str,
    claim_kind: EvidenceCaptureSourceClaimKind,
    claim_id: &str,
) -> StoreResult<Option<EvidenceCaptureSourceClaimRecord>> {
    conn.query_row(
        "SELECT project_id, source_claim_kind, source_claim_id,
                evidence_capture_intent_id, evidence_capture_receipt_id,
                capture_kind, claimed_at
           FROM evidence_capture_source_claims
          WHERE project_id = ?1 AND source_claim_kind = ?2 AND source_claim_id = ?3",
        params![project_id, claim_kind.as_str(), claim_id],
        source_claim_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

fn read_source_claims_for_receipt(
    conn: &rusqlite::Connection,
    project_id: &str,
    receipt_id: &str,
) -> StoreResult<Vec<EvidenceCaptureSourceClaimRecord>> {
    let mut statement = conn.prepare(
        "SELECT project_id, source_claim_kind, source_claim_id,
                evidence_capture_intent_id, evidence_capture_receipt_id,
                capture_kind, claimed_at
           FROM evidence_capture_source_claims
          WHERE project_id = ?1 AND evidence_capture_receipt_id = ?2
          ORDER BY source_claim_kind, source_claim_id",
    )?;
    let rows = statement.query_map(params![project_id, receipt_id], source_claim_from_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StoreError::from)
}

fn source_claim_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<EvidenceCaptureSourceClaimRecord> {
    Ok(EvidenceCaptureSourceClaimRecord {
        project_id: row.get(0)?,
        source_claim_kind: row.get(1)?,
        source_claim_id: row.get(2)?,
        evidence_capture_intent_id: row.get(3)?,
        evidence_capture_receipt_id: row.get(4)?,
        capture_kind: row.get(5)?,
        claimed_at: row.get(6)?,
    })
}

fn read_producer(
    conn: &rusqlite::Connection,
    project_id: &str,
    producer_id: &str,
) -> StoreResult<Option<EvidenceProducerRecord>> {
    conn.query_row(
        "SELECT project_id, evidence_producer_id, evidence_capture_intent_id,
                evidence_capture_receipt_id, evidence_observation_id, artifact_id,
                run_id, task_id, change_unit_id, scope_revision, baseline_ref,
                producer_kind, canonical_producer_json, created_at, metadata_json
           FROM evidence_producers
          WHERE project_id = ?1 AND evidence_producer_id = ?2",
        params![project_id, producer_id],
        |row| {
            let scope_revision = row.get::<_, i64>(9)?;
            Ok(EvidenceProducerRecord {
                project_id: row.get(0)?,
                evidence_producer_id: row.get(1)?,
                evidence_capture_intent_id: row.get(2)?,
                evidence_capture_receipt_id: row.get(3)?,
                evidence_observation_id: row.get(4)?,
                artifact_id: row.get(5)?,
                run_id: row.get(6)?,
                task_id: row.get(7)?,
                change_unit_id: row.get(8)?,
                scope_revision: nonnegative(scope_revision, 9)?,
                baseline_ref: row.get(10)?,
                producer_kind: row.get(11)?,
                canonical_producer_json: row.get(12)?,
                created_at: row.get(13)?,
                metadata_json: row.get(14)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

fn read_producer_for_intent(
    conn: &rusqlite::Connection,
    project_id: &str,
    intent_id: &str,
) -> StoreResult<Option<EvidenceProducerRecord>> {
    conn.query_row(
        "SELECT project_id, evidence_producer_id, evidence_capture_intent_id,
                evidence_capture_receipt_id, evidence_observation_id, artifact_id,
                run_id, task_id, change_unit_id, scope_revision, baseline_ref,
                producer_kind, canonical_producer_json, created_at, metadata_json
           FROM evidence_producers
          WHERE project_id = ?1 AND evidence_capture_intent_id = ?2",
        params![project_id, intent_id],
        |row| {
            let scope_revision = row.get::<_, i64>(9)?;
            Ok(EvidenceProducerRecord {
                project_id: row.get(0)?,
                evidence_producer_id: row.get(1)?,
                evidence_capture_intent_id: row.get(2)?,
                evidence_capture_receipt_id: row.get(3)?,
                evidence_observation_id: row.get(4)?,
                artifact_id: row.get(5)?,
                run_id: row.get(6)?,
                task_id: row.get(7)?,
                change_unit_id: row.get(8)?,
                scope_revision: nonnegative(scope_revision, 9)?,
                baseline_ref: row.get(10)?,
                producer_kind: row.get(11)?,
                canonical_producer_json: row.get(12)?,
                created_at: row.get(13)?,
                metadata_json: row.get(14)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

fn validate_receipt_input(input: &EvidenceCaptureReceiptInsert) -> StoreResult<()> {
    for (field, value) in [
        (
            "evidence_capture_receipt_id",
            input.evidence_capture_receipt_id.as_str(),
        ),
        (
            "evidence_capture_intent_id",
            input.evidence_capture_intent_id.as_str(),
        ),
        ("staging_handle_id", input.staging_handle_id.as_str()),
        ("task_id", input.task_id.as_str()),
        (
            "observed_by_actor_source",
            input.observed_by_actor_source.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(StoreError::InvalidInput {
                detail: format!("{field} must not be empty"),
            });
        }
    }
    validate_capture_kind(&input.capture_kind)?;
    validate_sha256("input_sha256", &input.input_sha256)?;
    validate_sha256("result_sha256", &input.result_sha256)?;
    validate_json_object("expected_outcome_json", &input.expected_outcome_json)?;
    validate_json_object("observed_outcome_json", &input.observed_outcome_json)?;
    validate_json_array("source_refs_json", &input.source_refs_json)?;
    validate_json_array("limitations_json", &input.limitations_json)?;
    validate_json_object("safe_receipt_json", &input.safe_receipt_json)?;
    validate_json_object("metadata_json", &input.metadata_json)?;
    for (field, value) in [
        ("observed_at", input.observed_at.as_str()),
        ("created_at", input.created_at.as_str()),
        ("staging_expires_at", input.staging_expires_at.as_str()),
    ] {
        UtcTimestamp::parse(value)
            .and_then(|timestamp| {
                timestamp
                    .ensure_canonical_rfc3339_representable()
                    .map_err(|_| volicord_types::UtcTimestampParseError)
            })
            .map_err(|_| StoreError::InvalidInput {
                detail: format!("{field} must be a canonical four-digit RFC 3339 timestamp"),
            })?;
    }
    if input.safe_receipt_json.len() > MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "safe_receipt_json exceeds the {MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES}-byte bound"
            ),
        });
    }
    if input.metadata_json.len() > MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "metadata_json exceeds the {MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES}-byte bound"
            ),
        });
    }
    Ok(())
}

fn validate_receipt_against_intent(
    input: &EvidenceCaptureReceiptInsert,
    intent: &EvidenceCaptureIntentRecord,
) -> StoreResult<ValidatedEvidenceCaptureSource> {
    if input.task_id != intent.task_id
        || input.capture_kind != intent.capture_kind
        || input.input_sha256 != intent.input_sha256
        || input.expected_outcome_json != intent.expected_outcome_json
    {
        return Err(StoreError::Conflict {
            entity: "evidence_capture_intent",
            id: intent.evidence_capture_intent_id.clone(),
            detail: "source receipt does not match the immutable capture intent".to_owned(),
        });
    }
    if input.source_refs_json != "[]" {
        return Err(StoreError::InvalidInput {
            detail: "source_refs_json must be the canonical empty array for source fulfillment"
                .to_owned(),
        });
    }
    let body =
        serde_json::from_str::<PersistedEvidenceCaptureReceiptBody>(&input.safe_receipt_json)
            .map_err(|error| StoreError::InvalidInput {
                detail: format!(
                    "safe_receipt_json is not a valid evidence-capture receipt: {error}"
                ),
            })?;
    let canonical_body =
        canonical_json_string(&body).map_err(|error| StoreError::InvalidInput {
            detail: format!("safe_receipt_json could not be canonicalized: {error}"),
        })?;
    let expected_outcome = serde_json::from_str::<JsonObject>(&input.expected_outcome_json)
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("expected_outcome_json is invalid: {error}"),
        })?;
    let observed_outcome = serde_json::from_str::<JsonObject>(&input.observed_outcome_json)
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("observed_outcome_json is invalid: {error}"),
        })?;
    let capture_spec = serde_json::from_str::<EvidenceCaptureSpec>(&intent.capture_spec_json)
        .map_err(|_| {
            StoreError::corrupt_owner_state_json(
                "evidence_capture_intents",
                intent.evidence_capture_intent_id.clone(),
                "capture_spec_json",
            )
        })?;
    if evidence_capture_input_sha256(&capture_spec).map_err(|_| {
        StoreError::corrupt_owner_state_json(
            "evidence_capture_intents",
            intent.evidence_capture_intent_id.clone(),
            "capture_spec_json",
        )
    })? != intent.input_sha256
    {
        return Err(StoreError::corrupt_owner_state_value(
            "evidence_capture_intents",
            intent.evidence_capture_intent_id.clone(),
            "input_sha256",
        ));
    }
    validate_evidence_capture_expected_outcome(&capture_spec, &expected_outcome).map_err(
        |detail| StoreError::InvalidInput {
            detail: format!("expected_outcome_json does not match its capture class: {detail}"),
        },
    )?;
    validate_evidence_capture_observed_outcome(&capture_spec, &observed_outcome).map_err(
        |detail| StoreError::InvalidInput {
            detail: format!("observed_outcome_json does not match its capture class: {detail}"),
        },
    )?;
    let limitations =
        serde_json::from_str::<Vec<String>>(&input.limitations_json).map_err(|error| {
            StoreError::InvalidInput {
                detail: format!("limitations_json is invalid: {error}"),
            }
        })?;
    validate_evidence_capture_limitations(&capture_spec, &limitations).map_err(|detail| {
        StoreError::InvalidInput {
            detail: format!("limitations_json does not match its capture class: {detail}"),
        }
    })?;
    let metadata = serde_json::from_str::<Value>(&input.metadata_json).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("metadata_json is invalid: {error}"),
        }
    })?;
    let capture_kind = match input.capture_kind.as_str() {
        "verified_command_execution" => EvidenceProducerKind::VerifiedCommandExecution,
        "verified_tool_invocation" => EvidenceProducerKind::VerifiedToolInvocation,
        _ => {
            return Err(StoreError::InvalidInput {
                detail: "capture_kind is not supported for evidence capture".to_owned(),
            })
        }
    };
    let result_sha256 = canonical_json_bare_sha256(&body.observed_outcome).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("observed outcome could not be hashed: {error}"),
        }
    })?;
    let source_value =
        serde_json::to_value(&body.source).map_err(|error| StoreError::InvalidInput {
            detail: format!("receipt source could not be serialized: {error}"),
        })?;
    let expected_metadata = serde_json::json!({"source": source_value});
    let expected_metadata_json =
        canonical_json_string(&expected_metadata).map_err(|error| StoreError::InvalidInput {
            detail: format!("receipt metadata could not be canonicalized: {error}"),
        })?;
    if canonical_body != input.safe_receipt_json
        || body.contract_id != EVIDENCE_CAPTURE_RECEIPT_CONTRACT_ID
        || !body.complete
        || body.redaction_state != RedactionState::Redacted
        || body.capture_kind != capture_kind
        || body.capture_intent_id.as_str() != intent.evidence_capture_intent_id
        || body.input_sha256 != input.input_sha256
        || body.result_sha256 != input.result_sha256
        || body.result_sha256 != result_sha256
        || body.expected_outcome != expected_outcome
        || body.observed_outcome != observed_outcome
        || body.limitations != limitations
        || body.observed_by_actor_source.to_canonical_string() != input.observed_by_actor_source
        || body.observed_by_actor_source.to_canonical_string() != intent.requested_by_actor_source
        || body.source.connection_id.as_str() != intent.requesting_connection_internal_id
        || body.observed_at.to_canonical_string() != input.observed_at
        || metadata != expected_metadata
        || input.metadata_json != expected_metadata_json
    {
        return Err(StoreError::Conflict {
            entity: "evidence_capture_intent",
            id: intent.evidence_capture_intent_id.clone(),
            detail: "source receipt body does not match the immutable capture intent and receipt columns"
                .to_owned(),
        });
    }
    let source_claims =
        derive_evidence_capture_source_claims(&capture_spec, &body).map_err(|_| {
            StoreError::Conflict {
                entity: "evidence_capture_intent",
                id: intent.evidence_capture_intent_id.clone(),
                detail: "receipt source shape does not match the immutable capture class"
                    .to_owned(),
            }
        })?;
    let observed_at =
        UtcTimestamp::parse(&input.observed_at).map_err(|_| StoreError::InvalidInput {
            detail: "observed_at must be a valid RFC 3339 timestamp".to_owned(),
        })?;
    let created_at =
        UtcTimestamp::parse(&input.created_at).map_err(|_| StoreError::InvalidInput {
            detail: "created_at must be a valid RFC 3339 timestamp".to_owned(),
        })?;
    observed_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::InvalidInput {
            detail: "observed_at must be a canonical four-digit RFC 3339 timestamp".to_owned(),
        })?;
    created_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::InvalidInput {
            detail: "created_at must be a canonical four-digit RFC 3339 timestamp".to_owned(),
        })?;
    let (intent_created_at, expires_at) =
        validate_evidence_capture_intent_window(&intent.created_at, &intent.expires_at).map_err(
            |field| {
                StoreError::corrupt_owner_state_value(
                    "evidence_capture_intents",
                    intent.evidence_capture_intent_id.clone(),
                    match field {
                        EvidenceCaptureIntentWindowError::CreatedAt => "created_at",
                        EvidenceCaptureIntentWindowError::ExpiresAt => "expires_at",
                    },
                )
            },
        )?;
    if observed_at < intent_created_at || observed_at >= expires_at {
        return Err(StoreError::Conflict {
            entity: "evidence_capture_intent",
            id: intent.evidence_capture_intent_id.clone(),
            detail: "source receipt observation is outside the capture intent validity window"
                .to_owned(),
        });
    }
    if created_at < observed_at || created_at >= expires_at {
        return Err(StoreError::Conflict {
            entity: "evidence_capture_intent",
            id: intent.evidence_capture_intent_id.clone(),
            detail: "source receipt creation is outside its observation and expiry bounds"
                .to_owned(),
        });
    }
    if input.staging_expires_at != intent.expires_at {
        return Err(StoreError::Conflict {
            entity: "evidence_capture_intent",
            id: intent.evidence_capture_intent_id.clone(),
            detail: "source receipt staging expiry does not match the capture intent expiry"
                .to_owned(),
        });
    }
    Ok(ValidatedEvidenceCaptureSource {
        claims: source_claims,
    })
}

/// Derives the exact normalized source-fact claim required by a strict receipt class.
pub fn derive_evidence_capture_source_claims(
    capture_spec: &EvidenceCaptureSpec,
    body: &PersistedEvidenceCaptureReceiptBody,
) -> StoreResult<Vec<EvidenceCaptureSourceClaimIdentity>> {
    let expected_kind = match capture_spec {
        EvidenceCaptureSpec::VerifiedCommandExecution { .. } => {
            EvidenceProducerKind::VerifiedCommandExecution
        }
        EvidenceCaptureSpec::VerifiedToolInvocation { .. } => {
            EvidenceProducerKind::VerifiedToolInvocation
        }
    };
    let source = &body.source;
    let Some(host_invocation_id) = source
        .host_invocation_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    else {
        return Err(StoreError::InvalidInput {
            detail: "receipt source requires a non-empty host invocation identifier".to_owned(),
        });
    };
    if body.capture_kind != expected_kind {
        return Err(StoreError::InvalidInput {
            detail: "receipt source shape does not match the immutable capture class".to_owned(),
        });
    }
    Ok(vec![EvidenceCaptureSourceClaimIdentity {
        source_claim_kind: EvidenceCaptureSourceClaimKind::HostInvocation,
        source_claim_id: canonical_json_bare_sha256(&serde_json::json!({
            "connection_id": source.connection_id,
            "host_invocation_id": host_invocation_id,
        }))
        .map_err(|error| StoreError::InvalidInput {
            detail: format!("host invocation source coordinates could not be normalized: {error}"),
        })?,
    }])
}
fn validate_capture_kind(value: &str) -> StoreResult<()> {
    if matches!(
        value,
        "verified_command_execution" | "verified_tool_invocation"
    ) {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: "capture_kind is outside the evidence-capture value set".to_owned(),
        })
    }
}

fn validate_sha256(field: &'static str, value: &str) -> StoreResult<()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(StoreError::InvalidInput {
            detail: format!("{field} must be lowercase 64-character SHA-256 hex"),
        })
    }
}

fn validate_json_object(field: &'static str, text: &str) -> StoreResult<()> {
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(_)) => Ok(()),
        _ => Err(StoreError::InvalidInput {
            detail: format!("{field} must be a JSON object"),
        }),
    }
}

fn validate_json_array(field: &'static str, text: &str) -> StoreResult<()> {
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Array(_)) => Ok(()),
        _ => Err(StoreError::InvalidInput {
            detail: format!("{field} must be a JSON array"),
        }),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn nonnegative(value: i64, column: usize) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}
