use std::fs;

use chrono::Duration;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};
use volicord_types::{
    canonical_json_bare_sha256, canonical_json_string, evidence_capture_input_sha256,
    validate_evidence_capture_expected_outcome, validate_evidence_capture_limitations,
    validate_evidence_capture_observed_outcome, AgentSessionId,
    ConnectionObservationSourceSelector, EvidenceCaptureSpec, EvidenceProducerKind, JsonObject,
    PersistedEvidenceCaptureReceiptBody, RedactionState, RequiredNullable, UtcTimestamp,
    EVIDENCE_CAPTURE_INTENT_TTL_MINUTES,
};

use crate::{
    artifacts::{
        insert_artifact_staging_tx, validate_insert as validate_staging_insert,
        ArtifactStagingInsert, StagedPayloadKind,
    },
    core_pipeline::{advance_project_utc_floor_tx, CoreProjectStore},
    session_watch::validate_current_complete_watch_observation_from_conn,
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
    GuardEvent,
    SessionWatchObservation,
}

impl EvidenceCaptureSourceClaimKind {
    /// Returns the stable storage value for this claim class.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HostInvocation => "host_invocation",
            Self::GuardEvent => "guard_event",
            Self::SessionWatchObservation => "session_watch_observation",
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
    watcher: Option<ValidatedWatcherSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedWatcherSource {
    observation_id: String,
    session_id: String,
    connection_internal_id: String,
    observed_at: UtcTimestamp,
    snapshot_algorithm: String,
    snapshot_digest: String,
    observation_sha256: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedCaptureSessionContext {
    session_id: RequiredNullable<AgentSessionId>,
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
        intent_session_id: Option<&AgentSessionId>,
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
        let mut expected =
            derive_evidence_capture_source_claims(capture_spec, intent_session_id, body)?;
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
        validate_transactional_source_freshness(
            &tx,
            &self.project.project_id,
            &intent.evidence_capture_intent_id,
            validated_source.watcher.as_ref(),
        )?;
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
        "registered_connection_observation" => {
            EvidenceProducerKind::RegisteredConnectionObservation
        }
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
        || body.schema_version != "volicord.evidence_capture_receipt.v1"
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
    let session_context =
        serde_json::from_str::<PersistedCaptureSessionContext>(&intent.session_context_json)
            .map_err(|_| {
                StoreError::corrupt_owner_state_json(
                    "evidence_capture_intents",
                    intent.evidence_capture_intent_id.clone(),
                    "session_context_json",
                )
            })?;
    let source_claims = derive_evidence_capture_source_claims(
        &capture_spec,
        session_context.session_id.as_ref(),
        &body,
    )
    .map_err(|_| StoreError::Conflict {
        entity: "evidence_capture_intent",
        id: intent.evidence_capture_intent_id.clone(),
        detail: "receipt source shape does not match the immutable capture class".to_owned(),
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
    let watcher = match &capture_spec {
        EvidenceCaptureSpec::RegisteredConnectionObservation {
            source_selector: ConnectionObservationSourceSelector::SessionWatcher {},
            ..
        } => {
            let observation_id = body
                .source
                .watch_observation_refs
                .first()
                .cloned()
                .ok_or_else(|| StoreError::InvalidInput {
                    detail: "validated watcher receipt has no selected observation".to_owned(),
                })?;
            let session_id = body
                .source
                .session_id
                .as_ref()
                .ok_or_else(|| StoreError::InvalidInput {
                    detail: "validated watcher receipt has no exact session".to_owned(),
                })?
                .as_str()
                .to_owned();
            let outcome_string = |field: &'static str| {
                body.observed_outcome
                    .get(field)
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| StoreError::Conflict {
                        entity: "evidence_capture_intent",
                        id: intent.evidence_capture_intent_id.clone(),
                        detail: format!(
                            "watcher receipt observed outcome has no canonical {field}"
                        ),
                    })
            };
            Some(ValidatedWatcherSource {
                observation_id,
                session_id,
                connection_internal_id: body.source.connection_id.as_str().to_owned(),
                observed_at: body.observed_at.clone(),
                snapshot_algorithm: outcome_string("snapshot_algorithm")?,
                snapshot_digest: outcome_string("snapshot_digest")?,
                observation_sha256: outcome_string("observation_sha256")?,
            })
        }
        _ => None,
    };
    Ok(ValidatedEvidenceCaptureSource {
        claims: source_claims,
        watcher,
    })
}

fn validate_transactional_source_freshness(
    conn: &Connection,
    project_id: &str,
    intent_id: &str,
    watcher: Option<&ValidatedWatcherSource>,
) -> StoreResult<()> {
    let Some(watcher) = watcher else {
        return Ok(());
    };
    let conflict = |detail: &'static str| StoreError::Conflict {
        entity: "evidence_capture_intent",
        id: intent_id.to_owned(),
        detail: detail.to_owned(),
    };
    let validated = validate_current_complete_watch_observation_from_conn(
        conn,
        project_id,
        &watcher.connection_internal_id,
        &watcher.session_id,
        &watcher.observation_id,
    )
    .map_err(|error| match error {
        StoreError::NotFound { .. }
        | StoreError::Conflict { .. }
        | StoreError::InvalidInput { .. } => conflict(
            "selected session-watch observation is not eligible for the immutable capture intent",
        ),
        other => other,
    })?;
    let actual_observed_at =
        UtcTimestamp::parse(&validated.observation.observed_at).map_err(|_| {
            StoreError::corrupt_owner_state_value(
                "session_watch_observations",
                validated.observation.watch_observation_id.clone(),
                "observed_at",
            )
        })?;
    if actual_observed_at != watcher.observed_at
        || validated.observation.snapshot_algorithm != watcher.snapshot_algorithm
        || validated.observation.snapshot_digest != watcher.snapshot_digest
        || validated.selection_sha256 != watcher.observation_sha256
    {
        return Err(conflict(
            "watcher receipt facts do not match the selected current complete observation",
        ));
    }
    Ok(())
}

/// Derives the exact normalized source-fact claims required by a strict receipt class.
pub fn derive_evidence_capture_source_claims(
    capture_spec: &EvidenceCaptureSpec,
    intent_session_id: Option<&AgentSessionId>,
    body: &PersistedEvidenceCaptureReceiptBody,
) -> StoreResult<Vec<EvidenceCaptureSourceClaimIdentity>> {
    let source = &body.source;
    let source_session_matches_intent = source.session_id.as_ref() == intent_session_id;
    let malformed = || StoreError::InvalidInput {
        detail: "receipt source shape does not match the immutable capture class".to_owned(),
    };
    let host_invocation = || {
        source
            .host_invocation_id
            .as_ref()
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .ok_or_else(malformed)
    };
    let claim = |source_claim_kind, source_claim_id: String| {
        if source_claim_id.trim().is_empty() {
            Err(malformed())
        } else {
            Ok(EvidenceCaptureSourceClaimIdentity {
                source_claim_kind,
                source_claim_id,
            })
        }
    };

    match capture_spec {
        EvidenceCaptureSpec::VerifiedCommandExecution { .. }
            if body.capture_kind == EvidenceProducerKind::VerifiedCommandExecution
                && source_session_matches_intent
                && source.guard_installation_id.is_none()
                && source.guard_event_ids.is_empty()
                && source.watch_observation_refs.is_empty() =>
        {
            Ok(vec![claim(
                EvidenceCaptureSourceClaimKind::HostInvocation,
                normalized_host_invocation_claim_id(source, &host_invocation()?)?,
            )?])
        }
        EvidenceCaptureSpec::VerifiedToolInvocation { .. }
            if body.capture_kind == EvidenceProducerKind::VerifiedToolInvocation
                && source_session_matches_intent
                && source.session_id.is_some()
                && source.guard_installation_id.is_some()
                && source.guard_event_ids.len() == 2
                && source.guard_event_ids[0] != source.guard_event_ids[1]
                && source.watch_observation_refs.is_empty() =>
        {
            let mut claims = vec![claim(
                EvidenceCaptureSourceClaimKind::HostInvocation,
                normalized_host_invocation_claim_id(source, &host_invocation()?)?,
            )?];
            claims.extend(
                source
                    .guard_event_ids
                    .iter()
                    .map(|event_id| {
                        claim(
                            EvidenceCaptureSourceClaimKind::GuardEvent,
                            event_id.as_str().to_owned(),
                        )
                    })
                    .collect::<StoreResult<Vec<_>>>()?,
            );
            Ok(claims)
        }
        EvidenceCaptureSpec::RegisteredConnectionObservation {
            source_selector: ConnectionObservationSourceSelector::GuardEvent { .. },
            ..
        } if body.capture_kind == EvidenceProducerKind::RegisteredConnectionObservation
            && source_session_matches_intent
            && source.session_id.is_some()
            && source.guard_installation_id.is_some()
            && source.guard_event_ids.len() == 1
            && source.watch_observation_refs.is_empty()
            && source.host_invocation_id.is_none() =>
        {
            Ok(vec![claim(
                EvidenceCaptureSourceClaimKind::GuardEvent,
                source.guard_event_ids[0].as_str().to_owned(),
            )?])
        }
        EvidenceCaptureSpec::RegisteredConnectionObservation {
            source_selector: ConnectionObservationSourceSelector::SessionWatcher {},
            ..
        } if body.capture_kind == EvidenceProducerKind::RegisteredConnectionObservation
            && source_session_matches_intent
            && source.session_id.is_some()
            && source.guard_installation_id.is_none()
            && source.guard_event_ids.is_empty()
            && source.watch_observation_refs.len() == 1
            && source.host_invocation_id.is_none() =>
        {
            Ok(vec![claim(
                EvidenceCaptureSourceClaimKind::SessionWatchObservation,
                source.watch_observation_refs[0].clone(),
            )?])
        }
        _ => Err(malformed()),
    }
}

fn normalized_host_invocation_claim_id(
    source: &volicord_types::PersistedEvidenceCaptureReceiptSource,
    host_invocation_id: &str,
) -> StoreResult<String> {
    canonical_json_bare_sha256(&serde_json::json!({
        "connection_id": source.connection_id,
        "session_id": source.session_id,
        "guard_installation_id": source.guard_installation_id,
        "host_invocation_id": host_invocation_id,
    }))
    .map_err(|error| StoreError::InvalidInput {
        detail: format!("host invocation source coordinates could not be normalized: {error}"),
    })
}

fn validate_capture_kind(value: &str) -> StoreResult<()> {
    if matches!(
        value,
        "verified_command_execution"
            | "verified_tool_invocation"
            | "registered_connection_observation"
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

#[cfg(test)]
mod tests {
    use std::{error::Error, path::PathBuf};

    use volicord_test_support::TempRuntimeHome;
    use volicord_types::{
        ConnectionObservationGuardEventKind, ProjectId, EVIDENCE_CAPTURE_COMMAND_LIMITATION,
        EVIDENCE_CAPTURE_GUARD_LIMITATION, EVIDENCE_CAPTURE_WATCHER_LIMITATION,
        WATCH_SNAPSHOT_ALGORITHM,
    };

    use super::*;
    use crate::agent_connections::{
        add_connection_project, ensure_agent_connection, AgentConnectionRegistration,
        ConnectionProjectRegistration, CONNECTION_INTENT_PERSONAL, CONNECTION_MODE_WORKFLOW,
        HOST_KIND_CODEX, HOST_SCOPE_USER, VERIFIED_STATUS_COMPLETE,
    };
    use crate::bootstrap::{
        initialize_runtime_home, register_project, ProjectRegistration, ACTIVE_PROJECT_STATUS,
    };
    use crate::session_watch::{
        canonical_watch_observation_selection, compare_watch_snapshots, create_watch_baseline,
        record_watch_observation, snapshot_product_repository, SessionWatchStatus,
        WatchBaselineCreate, WatchBaselineRecord, WatchObservationInsert, WatchObservationRecord,
    };

    struct CaptureHarness {
        _runtime_home: TempRuntimeHome,
        runtime_home_path: PathBuf,
    }

    impl CaptureHarness {
        fn new() -> Result<Self, Box<dyn Error>> {
            let runtime_home = TempRuntimeHome::new("evidence-capture-store")?;
            initialize_runtime_home(runtime_home.path(), "runtime_home_capture", "{}")?;
            register_project(
                runtime_home.path(),
                ProjectRegistration {
                    project_id: "project_capture".to_owned(),
                    repo_root: runtime_home.create_product_repo("repo")?,
                    project_home: None,
                    status: ACTIVE_PROJECT_STATUS.to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            ensure_agent_connection(
                runtime_home.path(),
                AgentConnectionRegistration {
                    connection_internal_id: "conn_capture".to_owned(),
                    host_kind: HOST_KIND_CODEX.to_owned(),
                    intent: CONNECTION_INTENT_PERSONAL.to_owned(),
                    host_scope: HOST_SCOPE_USER.to_owned(),
                    server_name: "volicord".to_owned(),
                    config_target: "/tmp/volicord-evidence-capture-test.toml".to_owned(),
                    mode: CONNECTION_MODE_WORKFLOW.to_owned(),
                    enabled: true,
                    managed_fingerprint: "evidence-capture-test".to_owned(),
                    last_verification_status: VERIFIED_STATUS_COMPLETE.to_owned(),
                    last_verification_report_json: "{}".to_owned(),
                    last_user_actions_json: "[]".to_owned(),
                    metadata_json: "{}".to_owned(),
                },
            )?;
            add_connection_project(
                runtime_home.path(),
                ConnectionProjectRegistration {
                    connection_internal_id: "conn_capture".to_owned(),
                    project_id: "project_capture".to_owned(),
                },
            )?;
            Ok(Self {
                runtime_home_path: runtime_home.path().to_path_buf(),
                _runtime_home: runtime_home,
            })
        }

        fn store(&self) -> StoreResult<CoreProjectStore> {
            CoreProjectStore::open(&self.runtime_home_path, &ProjectId::new("project_capture"))
        }
    }

    #[test]
    fn intent_read_rejects_noncanonical_or_nonfixed_time_windows_without_effects(
    ) -> Result<(), Box<dyn Error>> {
        for (variant, created_at, expires_at, expected_column) in [
            (
                "unrepresentable_created_at",
                "9999-12-31T23:59:59-23:59",
                "2026-07-13T00:15:00Z",
                "created_at",
            ),
            (
                "unrepresentable_expires_at",
                "2026-07-13T00:00:00Z",
                "9999-12-31T23:59:59-23:59",
                "expires_at",
            ),
            (
                "reversed",
                "2026-07-13T00:15:00Z",
                "2026-07-13T00:00:00Z",
                "expires_at",
            ),
            (
                "extended_ttl",
                "2026-07-13T00:00:00Z",
                "2026-07-13T00:16:00Z",
                "expires_at",
            ),
        ] {
            let harness = CaptureHarness::new()?;
            let store = harness.store()?;
            seed_intent(&store)?;
            store.conn.execute(
                "UPDATE evidence_capture_intents
                    SET created_at = ?3,
                        expires_at = ?4
                  WHERE project_id = ?1
                    AND evidence_capture_intent_id = ?2",
                rusqlite::params!["project_capture", "intent_capture", created_at, expires_at],
            )?;
            let before = (store.effect_counts()?, store.project_state()?);
            let error = store
                .evidence_capture_intent_record("intent_capture")
                .expect_err("corrupt intent window should fail closed");
            assert!(
                matches!(
                    &error,
                    StoreError::CorruptOwnerStateValue {
                        table: "evidence_capture_intents",
                        logical_column,
                        ..
                    } if *logical_column == expected_column
                ),
                "variant {variant} returned {error}"
            );
            assert_eq!(
                (store.effect_counts()?, store.project_state()?),
                before,
                "variant {variant}"
            );
        }
        Ok(())
    }

    #[test]
    fn selector_input_digest_corruption_fails_closed_without_source_effects(
    ) -> Result<(), Box<dyn Error>> {
        let harness = CaptureHarness::new()?;
        let mut store = harness.store()?;
        seed_graph(&store)?;
        seed_capture_intent(
            &store,
            "intent_selector_digest_corrupt",
            "registered_connection_observation",
            connection_spec(ConnectionObservationSourceSelector::SessionWatcher {}),
            "a",
            Some("session_capture"),
        )?;

        let corrupt_digest = "f".repeat(64);
        store.conn.execute(
            "UPDATE evidence_capture_intents
                SET input_sha256 = ?3
              WHERE project_id = ?1
                AND evidence_capture_intent_id = ?2",
            rusqlite::params![
                "project_capture",
                "intent_selector_digest_corrupt",
                &corrupt_digest
            ],
        )?;
        let before = store.effect_counts()?;
        let mut receipt = receipt_for(
            "intent_selector_digest_corrupt",
            "receipt_selector_digest_corrupt",
            "handle_selector_digest_corrupt",
            "registered_connection_observation",
            "a",
            watcher_source("watch_selector_digest_corrupt"),
        );
        receipt.input_sha256 = corrupt_digest;

        let error = store
            .fulfill_evidence_capture_source(receipt)
            .expect_err("a stored selector digest mismatch should fail closed");
        assert!(
            matches!(
                &error,
                StoreError::CorruptOwnerStateValue {
                    table: "evidence_capture_intents",
                    logical_column: "input_sha256",
                    ..
                }
            ),
            "unexpected selector digest corruption error: {error}"
        );
        assert_eq!(store.effect_counts()?, before);
        assert!(!staging_path(&store, "handle_selector_digest_corrupt").exists());
        Ok(())
    }

    #[test]
    fn watcher_source_is_revalidated_against_current_active_baseline_without_effects(
    ) -> Result<(), Box<dyn Error>> {
        for variant in ["stale_latest", "co_latest", "inactive_latest"] {
            let harness = CaptureHarness::new()?;
            let mut store = harness.store()?;
            seed_graph(&store)?;
            seed_capture_intent(
                &store,
                "intent_watcher_freshness",
                "registered_connection_observation",
                connection_spec(ConnectionObservationSourceSelector::SessionWatcher {}),
                "a",
                Some("session_capture"),
            )?;
            let selected_observation = seed_watch_observation(
                &store,
                "watch_baseline_selected",
                "watch_observation_selected",
                SessionWatchStatus::Active,
                "2026-07-13T00:00:01Z",
            )?;
            let receipt = watcher_receipt_for(
                "intent_watcher_freshness",
                "receipt_watcher_freshness",
                "handle_watcher_freshness",
                "a",
                &selected_observation,
            )?;

            match variant {
                "stale_latest" => {
                    seed_watch_baseline(
                        &store,
                        "watch_baseline_newer",
                        SessionWatchStatus::Active,
                        "2026-07-13T00:00:02Z",
                    )?;
                }
                "co_latest" => {
                    seed_watch_baseline(
                        &store,
                        "watch_baseline_co_latest",
                        SessionWatchStatus::Active,
                        "2026-07-13T00:00:01Z",
                    )?;
                }
                "inactive_latest" => {
                    store.conn.execute(
                        "UPDATE session_watch_baselines
                            SET status = 'degraded'
                          WHERE project_id = 'project_capture'
                            AND watch_baseline_id = 'watch_baseline_selected'",
                        [],
                    )?;
                }
                _ => unreachable!(),
            }

            let before = (store.effect_counts()?, store.project_state()?);
            let error = store
                .fulfill_evidence_capture_source(receipt)
                .expect_err("non-current watcher source must fail before receipt effects");
            match variant {
                "co_latest" => assert!(
                    matches!(&error, StoreError::SchemaInvariant { .. }),
                    "variant {variant} returned {error}"
                ),
                _ => assert!(
                    matches!(&error, StoreError::Conflict { .. }),
                    "variant {variant} returned {error}"
                ),
            }
            assert_eq!(
                (store.effect_counts()?, store.project_state()?),
                before,
                "variant {variant}"
            );
            assert!(
                store
                    .evidence_capture_receipt_for_intent("intent_watcher_freshness")?
                    .is_none(),
                "variant {variant} created a receipt"
            );
            assert!(
                !staging_path(&store, "handle_watcher_freshness").exists(),
                "variant {variant} created staging bytes"
            );
        }
        Ok(())
    }

    #[test]
    fn watcher_receipt_actual_digest_mismatch_fails_transactionally_without_effects(
    ) -> Result<(), Box<dyn Error>> {
        let harness = CaptureHarness::new()?;
        let mut store = harness.store()?;
        seed_graph(&store)?;
        seed_capture_intent(
            &store,
            "intent_watcher_actual_mismatch",
            "registered_connection_observation",
            connection_spec(ConnectionObservationSourceSelector::SessionWatcher {}),
            "a",
            Some("session_capture"),
        )?;
        let observation = seed_watch_observation(
            &store,
            "watch_baseline_actual_mismatch",
            "watch_observation_actual_mismatch",
            SessionWatchStatus::Active,
            "2026-07-13T00:00:01Z",
        )?;
        let mut receipt = watcher_receipt_for(
            "intent_watcher_actual_mismatch",
            "receipt_watcher_actual_mismatch",
            "handle_watcher_actual_mismatch",
            "a",
            &observation,
        )?;
        let mut tampered_outcome = serde_json::from_str::<Value>(&receipt.observed_outcome_json)?;
        tampered_outcome["observation_sha256"] = Value::String("f".repeat(64));
        set_receipt_observed_outcome(&mut receipt, tampered_outcome)?;

        let before = (store.effect_counts()?, store.project_state()?);
        let error = store
            .fulfill_evidence_capture_source(receipt)
            .expect_err("self-consistent receipt digest drift must fail inside the transaction");
        assert!(
            matches!(&error, StoreError::Conflict { .. }),
            "unexpected actual watcher digest mismatch error: {error}"
        );
        assert_eq!((store.effect_counts()?, store.project_state()?), before);
        assert!(store
            .evidence_capture_receipt_for_intent("intent_watcher_actual_mismatch")?
            .is_none());
        assert!(!staging_path(&store, "handle_watcher_actual_mismatch").exists());
        Ok(())
    }

    #[test]
    fn source_fulfillment_is_atomic_bounded_and_one_per_intent() -> Result<(), Box<dyn Error>> {
        let harness = CaptureHarness::new()?;
        let mut store = harness.store()?;
        seed_intent(&store)?;
        store.conn.execute(
            "UPDATE project_state SET updated_at = '2026-01-01T00:00:00Z'
              WHERE project_id = 'project_capture'",
            [],
        )?;
        let before = store.effect_counts()?;

        let first = store
            .fulfill_evidence_capture_source(receipt_input("receipt_capture", "handle_capture"))?;
        assert_eq!(first.evidence_capture_intent_id, "intent_capture");
        assert_eq!(first.completeness, "complete");
        assert_eq!(
            first.safe_receipt_size_bytes,
            first.safe_receipt_json.len() as u64
        );
        let after_first = store.effect_counts()?;
        assert_eq!(after_first.state_version, before.state_version);
        assert_eq!(after_first.task_events, before.task_events);
        assert_eq!(after_first.tool_invocations, before.tool_invocations);
        assert_eq!(after_first.artifact_staging, before.artifact_staging + 1);
        assert_eq!(
            after_first.evidence_capture_receipts,
            before.evidence_capture_receipts + 1
        );
        assert_eq!(
            after_first.evidence_capture_source_claims,
            before.evidence_capture_source_claims + 1
        );
        let state = store.project_state()?;
        assert_eq!(state.updated_at, first.created_at);
        assert!(
            UtcTimestamp::parse(&store.current_timestamp()?)?
                >= UtcTimestamp::parse(&first.created_at)?
        );
        let claims = store.evidence_capture_source_claims_for_receipt("receipt_capture")?;
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].source_claim_kind, "host_invocation");
        assert_eq!(claims[0].source_claim_id.len(), 64);
        assert_eq!(
            store
                .evidence_capture_source_claim_record(
                    EvidenceCaptureSourceClaimKind::HostInvocation,
                    &claims[0].source_claim_id,
                )?
                .expect("command source claim should be readable")
                .evidence_capture_intent_id,
            "intent_capture"
        );

        let duplicate = store.fulfill_evidence_capture_source(receipt_input(
            "receipt_duplicate",
            "handle_duplicate",
        ));
        assert!(duplicate.is_err());
        assert_eq!(store.effect_counts()?, after_first);
        assert!(!store
            .project
            .project_home
            .join("artifacts/tmp/handle_duplicate.txt")
            .exists());

        let oversized = "x".repeat(MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES + 1);
        let mut oversized_input = receipt_input("receipt_oversized", "handle_oversized");
        oversized_input.safe_receipt_json = format!("{{\"payload\":\"{oversized}\"}}");
        assert!(store
            .fulfill_evidence_capture_source(oversized_input)
            .is_err());
        assert_eq!(store.effect_counts()?, after_first);
        Ok(())
    }

    #[test]
    fn source_fulfillment_enforces_intent_time_window_and_receipt_body_binding(
    ) -> Result<(), Box<dyn Error>> {
        let harness = CaptureHarness::new()?;
        let mut store = harness.store()?;
        seed_intent(&store)?;
        let before = store.effect_counts()?;

        let mut before_intent = receipt_input("receipt_before", "handle_before");
        set_receipt_times(
            &mut before_intent,
            "2026-07-12T23:59:59Z",
            "2026-07-13T00:01:00Z",
        )?;
        assert!(store
            .fulfill_evidence_capture_source(before_intent)
            .is_err());

        let mut at_expiry = receipt_input("receipt_expiry", "handle_expiry");
        set_receipt_times(
            &mut at_expiry,
            "2026-07-13T00:15:00Z",
            "2026-07-13T00:15:00Z",
        )?;
        assert!(store.fulfill_evidence_capture_source(at_expiry).is_err());

        let mut mismatched_body = receipt_input("receipt_mismatch", "handle_mismatch");
        mismatched_body.result_sha256 = "c".repeat(64);
        assert!(store
            .fulfill_evidence_capture_source(mismatched_body)
            .is_err());
        assert_eq!(store.effect_counts()?, before);

        let mut lower_boundary = receipt_input("receipt_boundary", "handle_boundary");
        set_receipt_times(
            &mut lower_boundary,
            "2026-07-13T00:00:00Z",
            "2026-07-13T00:00:00Z",
        )?;
        let committed = store.fulfill_evidence_capture_source(lower_boundary)?;
        assert_eq!(committed.observed_at, "2026-07-13T00:00:00Z");
        Ok(())
    }

    #[test]
    fn source_claims_prevent_cross_intent_and_cross_class_reuse_atomically(
    ) -> Result<(), Box<dyn Error>> {
        let harness = CaptureHarness::new()?;
        let mut store = harness.store()?;
        seed_graph(&store)?;
        seed_capture_intent(
            &store,
            "intent_tool",
            "verified_tool_invocation",
            serde_json::json!({
                "capture_kind": "verified_tool_invocation",
                "tool_name": "example.tool",
                "tool_input_sha256": "b".repeat(64),
                "expected_success": true
            }),
            "b",
            Some("session_capture"),
        )?;
        seed_capture_intent(
            &store,
            "intent_tool_reuse",
            "verified_tool_invocation",
            serde_json::json!({
                "capture_kind": "verified_tool_invocation",
                "tool_name": "example.tool",
                "tool_input_sha256": "c".repeat(64),
                "expected_success": true
            }),
            "c",
            Some("session_capture"),
        )?;
        seed_capture_intent(
            &store,
            "intent_tool_other_session",
            "verified_tool_invocation",
            serde_json::json!({
                "capture_kind": "verified_tool_invocation",
                "tool_name": "example.tool",
                "tool_input_sha256": "1".repeat(64),
                "expected_success": true
            }),
            "1",
            Some("session_other"),
        )?;
        seed_capture_intent(
            &store,
            "intent_guard_reuse",
            "registered_connection_observation",
            connection_spec(ConnectionObservationSourceSelector::GuardEvent {
                event_kind: ConnectionObservationGuardEventKind::Stop,
            }),
            "d",
            Some("session_capture"),
        )?;
        seed_capture_intent(
            &store,
            "intent_watch_first",
            "registered_connection_observation",
            connection_spec(ConnectionObservationSourceSelector::SessionWatcher {}),
            "e",
            Some("session_capture"),
        )?;
        seed_capture_intent(
            &store,
            "intent_watch_reuse",
            "registered_connection_observation",
            connection_spec(ConnectionObservationSourceSelector::SessionWatcher {}),
            "f",
            Some("session_capture"),
        )?;
        let shared_watch_observation = seed_watch_observation(
            &store,
            "watch_baseline_shared",
            "watch_shared",
            SessionWatchStatus::Active,
            "2026-07-13T00:00:01Z",
        )?;

        store.fulfill_evidence_capture_source(receipt_for(
            "intent_tool",
            "receipt_tool",
            "handle_tool",
            "verified_tool_invocation",
            "b",
            tool_source("host_shared", "guard_shared", "guard_post"),
        ))?;
        let after_tool = store.effect_counts()?;
        assert_eq!(
            store
                .evidence_capture_source_claims_for_receipt("receipt_tool")?
                .len(),
            3,
            "tool fulfillment must claim its host invocation and both guard events"
        );

        let host_reuse = store.fulfill_evidence_capture_source(receipt_for(
            "intent_tool_reuse",
            "receipt_tool_reuse",
            "handle_tool_reuse",
            "verified_tool_invocation",
            "c",
            tool_source("host_shared", "guard_other_pre", "guard_other_post"),
        ));
        assert!(host_reuse.is_err());
        assert_eq!(store.effect_counts()?, after_tool);
        assert!(!staging_path(&store, "handle_tool_reuse").exists());

        store.fulfill_evidence_capture_source(receipt_for(
            "intent_tool_other_session",
            "receipt_tool_other_session",
            "handle_tool_other_session",
            "verified_tool_invocation",
            "1",
            tool_source_with_context(
                "session_other",
                "installation_capture",
                "host_shared",
                "guard_session_other_pre",
                "guard_session_other_post",
            ),
        ))?;
        let after_other_session = store.effect_counts()?;
        assert_eq!(
            after_other_session.evidence_capture_source_claims,
            after_tool.evidence_capture_source_claims + 3,
            "the same host-local ID in another exact source context is a distinct fact"
        );

        let guard_reuse = store.fulfill_evidence_capture_source(receipt_for(
            "intent_guard_reuse",
            "receipt_guard_reuse",
            "handle_guard_reuse",
            "registered_connection_observation",
            "d",
            guard_source("guard_shared"),
        ));
        assert!(guard_reuse.is_err());
        assert_eq!(store.effect_counts()?, after_other_session);
        assert!(!staging_path(&store, "handle_guard_reuse").exists());

        store.fulfill_evidence_capture_source(watcher_receipt_for(
            "intent_watch_first",
            "receipt_watch_first",
            "handle_watch_first",
            "e",
            &shared_watch_observation,
        )?)?;
        let after_watch = store.effect_counts()?;
        let watcher_reuse = store.fulfill_evidence_capture_source(watcher_receipt_for(
            "intent_watch_reuse",
            "receipt_watch_reuse",
            "handle_watch_reuse",
            "f",
            &shared_watch_observation,
        )?);
        assert!(watcher_reuse.is_err());
        assert_eq!(store.effect_counts()?, after_watch);
        assert!(!staging_path(&store, "handle_watch_reuse").exists());
        Ok(())
    }

    #[test]
    fn source_class_shapes_reject_missing_extra_and_ambiguous_coordinates(
    ) -> Result<(), Box<dyn Error>> {
        let harness = CaptureHarness::new()?;
        let mut store = harness.store()?;
        seed_graph(&store)?;
        seed_capture_intent(
            &store,
            "intent_command_shape",
            "verified_command_execution",
            command_spec("a"),
            "a",
            Some("session_capture"),
        )?;
        seed_capture_intent(
            &store,
            "intent_tool_shape",
            "verified_tool_invocation",
            serde_json::json!({
                "capture_kind": "verified_tool_invocation",
                "tool_name": "example.tool",
                "tool_input_sha256": "b".repeat(64),
                "expected_success": true
            }),
            "b",
            Some("session_capture"),
        )?;
        seed_capture_intent(
            &store,
            "intent_guard_shape",
            "registered_connection_observation",
            connection_spec(ConnectionObservationSourceSelector::GuardEvent {
                event_kind: ConnectionObservationGuardEventKind::Stop,
            }),
            "c",
            Some("session_capture"),
        )?;
        seed_capture_intent(
            &store,
            "intent_watcher_shape",
            "registered_connection_observation",
            connection_spec(ConnectionObservationSourceSelector::SessionWatcher {}),
            "d",
            Some("session_capture"),
        )?;
        let before = store.effect_counts()?;

        let mut command_missing_host = command_source(Some("session_capture"), "host_required");
        command_missing_host["host_invocation_id"] = Value::Null;
        assert!(store
            .fulfill_evidence_capture_source(receipt_for(
                "intent_command_shape",
                "receipt_command_missing",
                "handle_command_missing",
                "verified_command_execution",
                "a",
                command_missing_host,
            ))
            .is_err());

        let mut command_extra_event = command_source(Some("session_capture"), "host_command");
        command_extra_event["guard_event_ids"] = serde_json::json!(["guard_extra"]);
        assert!(store
            .fulfill_evidence_capture_source(receipt_for(
                "intent_command_shape",
                "receipt_command_extra",
                "handle_command_extra",
                "verified_command_execution",
                "a",
                command_extra_event,
            ))
            .is_err());

        let mut tool_duplicate_events = tool_source("host_tool", "guard_same", "guard_same");
        assert!(store
            .fulfill_evidence_capture_source(receipt_for(
                "intent_tool_shape",
                "receipt_tool_duplicate",
                "handle_tool_duplicate",
                "verified_tool_invocation",
                "b",
                tool_duplicate_events.clone(),
            ))
            .is_err());
        tool_duplicate_events["session_id"] = Value::Null;
        assert!(store
            .fulfill_evidence_capture_source(receipt_for(
                "intent_tool_shape",
                "receipt_tool_session",
                "handle_tool_session",
                "verified_tool_invocation",
                "b",
                tool_duplicate_events,
            ))
            .is_err());

        let mut guard_extra_host = guard_source("guard_connection");
        guard_extra_host["host_invocation_id"] = Value::String("host_extra".to_owned());
        assert!(store
            .fulfill_evidence_capture_source(receipt_for(
                "intent_guard_shape",
                "receipt_guard_extra",
                "handle_guard_extra",
                "registered_connection_observation",
                "c",
                guard_extra_host,
            ))
            .is_err());

        let mut watcher_extra_guard = watcher_source("watch_shape");
        watcher_extra_guard["guard_event_ids"] = serde_json::json!(["guard_extra"]);
        assert!(store
            .fulfill_evidence_capture_source(receipt_for(
                "intent_watcher_shape",
                "receipt_watcher_extra",
                "handle_watcher_extra",
                "registered_connection_observation",
                "d",
                watcher_extra_guard,
            ))
            .is_err());

        assert_eq!(store.effect_counts()?, before);
        Ok(())
    }

    #[test]
    fn source_fulfillment_rejects_noncanonical_outcomes_and_raw_metadata_atomically(
    ) -> Result<(), Box<dyn Error>> {
        let harness = CaptureHarness::new()?;
        let mut store = harness.store()?;
        seed_graph(&store)?;
        seed_capture_intent(
            &store,
            "intent_strict_command",
            "verified_command_execution",
            command_spec("a"),
            "a",
            None,
        )?;
        seed_capture_intent(
            &store,
            "intent_strict_tool",
            "verified_tool_invocation",
            serde_json::json!({
                "capture_kind": "verified_tool_invocation",
                "tool_name": "example.tool",
                "tool_input_sha256": "b".repeat(64),
                "expected_success": true
            }),
            "b",
            Some("session_capture"),
        )?;
        seed_capture_intent(
            &store,
            "intent_strict_watcher",
            "registered_connection_observation",
            connection_spec(ConnectionObservationSourceSelector::SessionWatcher {}),
            "c",
            Some("session_capture"),
        )?;
        let before = store.effect_counts()?;

        let mut missing_digest = receipt_for(
            "intent_strict_command",
            "receipt_missing_digest",
            "handle_missing_digest",
            "verified_command_execution",
            "a",
            command_source(None, "host_missing_digest"),
        );
        let mut outcome = serde_json::from_str::<Value>(&missing_digest.observed_outcome_json)?;
        outcome
            .as_object_mut()
            .expect("command outcome should be an object")
            .remove("stdout_sha256");
        set_receipt_observed_outcome(&mut missing_digest, outcome)?;

        let mut missing_count = receipt_for(
            "intent_strict_tool",
            "receipt_missing_count",
            "handle_missing_count",
            "verified_tool_invocation",
            "b",
            tool_source("host_missing_count", "guard_count_pre", "guard_count_post"),
        );
        let mut outcome = serde_json::from_str::<Value>(&missing_count.observed_outcome_json)?;
        outcome
            .as_object_mut()
            .expect("tool outcome should be an object")
            .remove("tool_result_size_bytes");
        set_receipt_observed_outcome(&mut missing_count, outcome)?;

        let mut wrong_type = receipt_for(
            "intent_strict_command",
            "receipt_wrong_type",
            "handle_wrong_type",
            "verified_command_execution",
            "a",
            command_source(None, "host_wrong_type"),
        );
        let mut outcome = serde_json::from_str::<Value>(&wrong_type.observed_outcome_json)?;
        outcome["stderr_size_bytes"] = Value::String("0".to_owned());
        set_receipt_observed_outcome(&mut wrong_type, outcome)?;

        let mut raw_extra = receipt_for(
            "intent_strict_watcher",
            "receipt_raw_extra",
            "handle_raw_extra",
            "registered_connection_observation",
            "c",
            watcher_source("watch_raw_extra"),
        );
        let mut outcome = serde_json::from_str::<Value>(&raw_extra.observed_outcome_json)?;
        outcome["raw_snapshot"] = serde_json::json!({"secret": true});
        set_receipt_observed_outcome(&mut raw_extra, outcome)?;

        let mut raw_metadata = receipt_for(
            "intent_strict_command",
            "receipt_raw_metadata",
            "handle_raw_metadata",
            "verified_command_execution",
            "a",
            command_source(None, "host_raw_metadata"),
        );
        let source = serde_json::from_str::<Value>(&raw_metadata.metadata_json)?["source"].clone();
        raw_metadata.metadata_json = canonical_json_string(&serde_json::json!({
            "source": source,
            "raw_output": "must not persist"
        }))?;

        let mut incomplete_watcher = receipt_for(
            "intent_strict_watcher",
            "receipt_incomplete_watcher",
            "handle_incomplete_watcher",
            "registered_connection_observation",
            "c",
            watcher_source("watch_incomplete"),
        );
        let mut outcome = serde_json::from_str::<Value>(&incomplete_watcher.observed_outcome_json)?;
        outcome["complete"] = Value::Bool(false);
        set_receipt_observed_outcome(&mut incomplete_watcher, outcome)?;

        let mut nonempty_source_refs = receipt_for(
            "intent_strict_command",
            "receipt_source_refs",
            "handle_source_refs",
            "verified_command_execution",
            "a",
            command_source(None, "host_source_refs"),
        );
        nonempty_source_refs.source_refs_json = canonical_json_string(&serde_json::json!([{
            "record_kind": "task",
            "record_id": "task_capture",
            "project_id": "project_capture",
            "task_id": "task_capture",
            "produced_at_state_version": 0
        }]))?;

        for (input, handle) in [
            (missing_digest, "handle_missing_digest"),
            (missing_count, "handle_missing_count"),
            (wrong_type, "handle_wrong_type"),
            (raw_extra, "handle_raw_extra"),
            (raw_metadata, "handle_raw_metadata"),
            (incomplete_watcher, "handle_incomplete_watcher"),
            (nonempty_source_refs, "handle_source_refs"),
        ] {
            assert!(store.fulfill_evidence_capture_source(input).is_err());
            assert_eq!(store.effect_counts()?, before);
            assert!(!staging_path(&store, handle).exists());
        }
        Ok(())
    }

    #[test]
    fn source_claim_revalidation_fails_closed_after_claim_deletion() -> Result<(), Box<dyn Error>> {
        let harness = CaptureHarness::new()?;
        let mut store = harness.store()?;
        seed_intent(&store)?;
        let receipt = store.fulfill_evidence_capture_source(receipt_input(
            "receipt_claim_check",
            "handle_claim_check",
        ))?;
        let intent = store
            .evidence_capture_intent_record("intent_capture")?
            .expect("intent should exist");
        let capture_spec = serde_json::from_str::<EvidenceCaptureSpec>(&intent.capture_spec_json)?;
        let body = serde_json::from_str::<PersistedEvidenceCaptureReceiptBody>(
            &receipt.safe_receipt_json,
        )?;
        store.validate_evidence_capture_source_claims_for_receipt(
            &intent,
            &receipt,
            &capture_spec,
            None,
            &body,
        )?;

        store.conn.execute(
            "DELETE FROM evidence_capture_source_claims
              WHERE project_id = 'project_capture'
                AND evidence_capture_receipt_id = 'receipt_claim_check'",
            [],
        )?;
        assert!(store
            .validate_evidence_capture_source_claims_for_receipt(
                &intent,
                &receipt,
                &capture_spec,
                None,
                &body,
            )
            .is_err());
        Ok(())
    }

    fn seed_intent(store: &CoreProjectStore) -> StoreResult<()> {
        seed_graph(store)?;
        seed_capture_intent(
            store,
            "intent_capture",
            "verified_command_execution",
            command_spec("a"),
            "a",
            None,
        )
    }

    fn seed_graph(store: &CoreProjectStore) -> StoreResult<()> {
        store.conn.execute(
            "INSERT INTO tasks (
                project_id, task_id, created_by_actor_source, mode, work_phase,
                acceptance_policy, acceptance_policy_reason, lifecycle_phase,
                created_at, updated_at
            ) VALUES (
                'project_capture', 'task_capture', 'agent_connection:conn_capture',
                'work', 'implementation', 'required', 'capture fixture',
                'implementation', '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'
            )",
            [],
        )?;
        store.conn.execute(
            "INSERT INTO change_units (
                project_id, change_unit_id, task_id, status, is_current,
                basis_state_version, created_at, updated_at
            ) VALUES (
                'project_capture', 'cu_capture', 'task_capture', 'active', 1, 0,
                '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'
            )",
            [],
        )?;
        Ok(())
    }

    fn seed_watch_baseline(
        store: &CoreProjectStore,
        baseline_id: &str,
        status: SessionWatchStatus,
        updated_at: &str,
    ) -> StoreResult<WatchBaselineRecord> {
        store.conn.execute(
            "INSERT OR IGNORE INTO agent_sessions (
                project_id, session_id, connection_internal_id,
                guard_installation_id, host_kind, guard_mode, started_at,
                ended_at, metadata_json
            ) VALUES (
                'project_capture', 'session_capture', 'conn_capture', NULL,
                'codex', 'detective', '2026-07-13T00:00:00Z', NULL, '{}'
            )",
            [],
        )?;
        let snapshot = snapshot_product_repository(
            &store.runtime_home,
            &store.project.repo_root,
            Default::default(),
        )?;
        create_watch_baseline(
            &store.runtime_home,
            &store.project.project_id,
            WatchBaselineCreate {
                watch_baseline_id: baseline_id.to_owned(),
                session_id: "session_capture".to_owned(),
                connection_internal_id: "conn_capture".to_owned(),
                guard_installation_id: None,
                status,
                snapshot,
                created_at: updated_at.to_owned(),
                metadata_json: "{}".to_owned(),
            },
        )
    }

    fn seed_watch_observation(
        store: &CoreProjectStore,
        baseline_id: &str,
        observation_id: &str,
        baseline_status: SessionWatchStatus,
        baseline_updated_at: &str,
    ) -> StoreResult<WatchObservationRecord> {
        seed_watch_baseline(store, baseline_id, baseline_status, baseline_updated_at)?;
        let baseline_snapshot = snapshot_product_repository(
            &store.runtime_home,
            &store.project.repo_root,
            Default::default(),
        )?;
        let snapshot = baseline_snapshot.clone();
        let diff = compare_watch_snapshots(&baseline_snapshot, &snapshot);
        let metadata_json = canonical_json_string(&serde_json::json!({
            "scan_summary": &snapshot.scan_summary
        }))
        .map_err(|error| StoreError::InvalidInput {
            detail: error.to_string(),
        })?;
        record_watch_observation(
            &store.runtime_home,
            &store.project.project_id,
            WatchObservationInsert {
                watch_observation_id: observation_id.to_owned(),
                watch_baseline_id: baseline_id.to_owned(),
                expected_write_id: None,
                snapshot,
                diff,
                observed_at: "2026-07-13T00:01:00Z".to_owned(),
                metadata_json,
            },
        )
    }

    fn seed_capture_intent(
        store: &CoreProjectStore,
        intent_id: &str,
        capture_kind: &str,
        capture_spec: Value,
        _input_sha_char: &str,
        session_id: Option<&str>,
    ) -> StoreResult<()> {
        let decoded_capture = serde_json::from_value::<EvidenceCaptureSpec>(capture_spec.clone())
            .map_err(|error| StoreError::InvalidInput {
            detail: error.to_string(),
        })?;
        let input_sha256 = evidence_capture_input_sha256(&decoded_capture).map_err(|error| {
            StoreError::InvalidInput {
                detail: error.to_string(),
            }
        })?;
        let capture_spec_json =
            canonical_json_string(&capture_spec).map_err(|error| StoreError::InvalidInput {
                detail: error.to_string(),
            })?;
        let session_context_json = canonical_json_string(&serde_json::json!({
            "session_id": session_id
        }))
        .map_err(|error| StoreError::InvalidInput {
            detail: error.to_string(),
        })?;
        let expected_outcome_json = canonical_json_string(&expected_outcome(capture_kind))
            .map_err(|error| StoreError::InvalidInput {
                detail: error.to_string(),
            })?;
        store.conn.execute(
            "INSERT INTO evidence_capture_intents (
                project_id, evidence_capture_intent_id, task_id, change_unit_id,
                scope_revision, baseline_ref, target_json, capture_kind,
                capture_spec_json, input_sha256, expected_outcome_json,
                requested_by_actor_source, requesting_connection_internal_id,
                session_context_json, workspace_context_json, created_at, expires_at,
                metadata_json
            ) VALUES (
                'project_capture', ?1, 'task_capture', 'cu_capture', 0,
                'baseline_capture', '{}', ?2, ?3, ?4,
                ?6, 'agent_connection:conn_capture', 'conn_capture',
                ?5, '{}', '2026-07-13T00:00:00Z', '2026-07-13T00:15:00Z', '{}'
            )",
            rusqlite::params![
                intent_id,
                capture_kind,
                capture_spec_json,
                input_sha256,
                session_context_json,
                expected_outcome_json
            ],
        )?;
        Ok(())
    }

    fn receipt_input(receipt_id: &str, handle_id: &str) -> EvidenceCaptureReceiptInsert {
        receipt_for(
            "intent_capture",
            receipt_id,
            handle_id,
            "verified_command_execution",
            "a",
            command_source(None, "host_command_capture"),
        )
    }

    fn receipt_for(
        intent_id: &str,
        receipt_id: &str,
        handle_id: &str,
        capture_kind: &str,
        input_sha_char: &str,
        source: Value,
    ) -> EvidenceCaptureReceiptInsert {
        let input_sha256 = if capture_kind == "registered_connection_observation" {
            let source_selector = if source["guard_event_ids"]
                .as_array()
                .is_some_and(|events| !events.is_empty())
            {
                ConnectionObservationSourceSelector::GuardEvent {
                    event_kind: ConnectionObservationGuardEventKind::Stop,
                }
            } else {
                ConnectionObservationSourceSelector::SessionWatcher {}
            };
            canonical_json_bare_sha256(&source_selector).expect("fixed source selector should hash")
        } else {
            input_sha_char.repeat(64)
        };
        let observed_outcome = match capture_kind {
            "verified_command_execution" => serde_json::json!({
                "exit_code": 0,
                "stdout_sha256": "1".repeat(64),
                "stdout_size_bytes": 12,
                "stderr_sha256": "2".repeat(64),
                "stderr_size_bytes": 0
            }),
            "verified_tool_invocation" => serde_json::json!({
                "success": true,
                "exit_code": null,
                "tool_result_sha256": "3".repeat(64),
                "tool_result_size_bytes": 18
            }),
            "registered_connection_observation"
                if source["guard_event_ids"]
                    .as_array()
                    .is_some_and(|events| !events.is_empty()) =>
            {
                serde_json::json!({
                    "complete": true,
                    "guard_event_kind": "stop",
                    "guard_decision": "allow",
                    "observation_sha256": input_sha_char.repeat(64)
                })
            }
            "registered_connection_observation" => serde_json::json!({
                "complete": true,
                "snapshot_algorithm": WATCH_SNAPSHOT_ALGORITHM,
                "snapshot_digest": "4".repeat(64),
                "observation_sha256": input_sha_char.repeat(64)
            }),
            _ => unreachable!("test helper received an unsupported capture kind"),
        };
        let expected_outcome = expected_outcome(capture_kind);
        let limitations = match capture_kind {
            "verified_command_execution" => {
                serde_json::json!([EVIDENCE_CAPTURE_COMMAND_LIMITATION])
            }
            "verified_tool_invocation" => {
                serde_json::json!([EVIDENCE_CAPTURE_GUARD_LIMITATION])
            }
            "registered_connection_observation"
                if source["guard_event_ids"]
                    .as_array()
                    .is_some_and(|events| !events.is_empty()) =>
            {
                serde_json::json!([EVIDENCE_CAPTURE_GUARD_LIMITATION])
            }
            "registered_connection_observation" => {
                serde_json::json!([EVIDENCE_CAPTURE_WATCHER_LIMITATION])
            }
            _ => unreachable!("test helper received an unsupported capture kind"),
        };
        let result_sha256 = canonical_json_bare_sha256(&observed_outcome)
            .expect("fixed receipt outcome should hash");
        let safe_receipt = serde_json::json!({
            "schema_version": "volicord.evidence_capture_receipt.v1",
            "capture_kind": capture_kind,
            "capture_intent_id": intent_id,
            "input_sha256": input_sha256.clone(),
            "result_sha256": result_sha256,
            "expected_outcome": expected_outcome,
            "observed_outcome": observed_outcome,
            "source": source,
            "complete": true,
            "limitations": limitations,
            "redaction_state": "redacted",
            "observed_by_actor_source": "agent_connection:conn_capture",
            "observed_at": "2026-07-13T00:01:00Z"
        });
        EvidenceCaptureReceiptInsert {
            evidence_capture_receipt_id: receipt_id.to_owned(),
            evidence_capture_intent_id: intent_id.to_owned(),
            staging_handle_id: handle_id.to_owned(),
            task_id: "task_capture".to_owned(),
            capture_kind: capture_kind.to_owned(),
            input_sha256: input_sha256.clone(),
            result_sha256: result_sha256.clone(),
            expected_outcome_json: canonical_json_string(&safe_receipt["expected_outcome"])
                .expect("fixed expected outcome should serialize"),
            observed_outcome_json: canonical_json_string(&safe_receipt["observed_outcome"])
                .expect("fixed observed outcome should serialize"),
            source_refs_json: "[]".to_owned(),
            observed_by_actor_source: "agent_connection:conn_capture".to_owned(),
            observed_at: "2026-07-13T00:01:00Z".to_owned(),
            limitations_json: canonical_json_string(&safe_receipt["limitations"])
                .expect("fixed limitations should serialize"),
            safe_receipt_json: canonical_json_string(&safe_receipt)
                .expect("fixed receipt should serialize"),
            created_at: "2026-07-13T00:01:00Z".to_owned(),
            staging_expires_at: "2026-07-13T00:15:00Z".to_owned(),
            metadata_json: canonical_json_string(&serde_json::json!({
                "source": safe_receipt["source"].clone()
            }))
            .expect("fixed metadata should serialize"),
        }
    }

    fn watcher_receipt_for(
        intent_id: &str,
        receipt_id: &str,
        handle_id: &str,
        input_sha_char: &str,
        observation: &WatchObservationRecord,
    ) -> Result<EvidenceCaptureReceiptInsert, Box<dyn Error>> {
        let mut receipt = receipt_for(
            intent_id,
            receipt_id,
            handle_id,
            "registered_connection_observation",
            input_sha_char,
            watcher_source(&observation.watch_observation_id),
        );
        let selection = canonical_watch_observation_selection(observation)?;
        set_receipt_observed_outcome(
            &mut receipt,
            serde_json::json!({
                "complete": true,
                "snapshot_algorithm": observation.snapshot_algorithm,
                "snapshot_digest": observation.snapshot_digest,
                "observation_sha256": canonical_json_bare_sha256(&selection)?,
            }),
        )?;
        set_receipt_times(
            &mut receipt,
            &observation.observed_at,
            &observation.observed_at,
        )?;
        Ok(receipt)
    }

    fn expected_outcome(capture_kind: &str) -> Value {
        match capture_kind {
            "verified_command_execution" => serde_json::json!({"expected_exit_code": 0}),
            "verified_tool_invocation" => serde_json::json!({"expected_success": true}),
            "registered_connection_observation" => {
                serde_json::json!({"expected_complete": true})
            }
            _ => unreachable!("test helper received an unsupported capture kind"),
        }
    }

    fn command_spec(sha_char: &str) -> Value {
        serde_json::json!({
            "capture_kind": "verified_command_execution",
            "command_sha256": sha_char.repeat(64),
            "command_label": "capture fixture",
            "expected_exit_code": 0
        })
    }

    fn connection_spec(source_selector: ConnectionObservationSourceSelector) -> Value {
        serde_json::json!({
            "capture_kind": "registered_connection_observation",
            "source_selector": source_selector,
            "expected_complete": true
        })
    }

    fn source(
        session_id: Option<&str>,
        guard_installation_id: Option<&str>,
        guard_event_ids: Vec<&str>,
        watch_observation_refs: Vec<&str>,
        host_invocation_id: Option<&str>,
    ) -> Value {
        serde_json::json!({
            "connection_id": "conn_capture",
            "session_id": session_id,
            "guard_installation_id": guard_installation_id,
            "guard_event_ids": guard_event_ids,
            "watch_observation_refs": watch_observation_refs,
            "host_invocation_id": host_invocation_id
        })
    }

    fn command_source(session_id: Option<&str>, host_invocation_id: &str) -> Value {
        source(session_id, None, vec![], vec![], Some(host_invocation_id))
    }

    fn tool_source(host_invocation_id: &str, pre_event_id: &str, post_event_id: &str) -> Value {
        tool_source_with_context(
            "session_capture",
            "installation_capture",
            host_invocation_id,
            pre_event_id,
            post_event_id,
        )
    }

    fn tool_source_with_context(
        session_id: &str,
        installation_id: &str,
        host_invocation_id: &str,
        pre_event_id: &str,
        post_event_id: &str,
    ) -> Value {
        source(
            Some(session_id),
            Some(installation_id),
            vec![pre_event_id, post_event_id],
            vec![],
            Some(host_invocation_id),
        )
    }

    fn guard_source(event_id: &str) -> Value {
        source(
            Some("session_capture"),
            Some("installation_capture"),
            vec![event_id],
            vec![],
            None,
        )
    }

    fn watcher_source(observation_id: &str) -> Value {
        source(
            Some("session_capture"),
            None,
            vec![],
            vec![observation_id],
            None,
        )
    }

    fn staging_path(store: &CoreProjectStore, handle_id: &str) -> PathBuf {
        store
            .project
            .project_home
            .join(format!("artifacts/tmp/{handle_id}.txt"))
    }

    fn set_receipt_times(
        input: &mut EvidenceCaptureReceiptInsert,
        observed_at: &str,
        created_at: &str,
    ) -> Result<(), Box<dyn Error>> {
        input.observed_at = observed_at.to_owned();
        input.created_at = created_at.to_owned();
        let mut body = serde_json::from_str::<Value>(&input.safe_receipt_json)?;
        body["observed_at"] = Value::String(observed_at.to_owned());
        input.safe_receipt_json = canonical_json_string(&body)?;
        Ok(())
    }

    fn set_receipt_observed_outcome(
        input: &mut EvidenceCaptureReceiptInsert,
        observed_outcome: Value,
    ) -> Result<(), Box<dyn Error>> {
        let result_sha256 = canonical_json_bare_sha256(&observed_outcome)?;
        input.result_sha256 = result_sha256.clone();
        input.observed_outcome_json = canonical_json_string(&observed_outcome)?;
        let mut body = serde_json::from_str::<Value>(&input.safe_receipt_json)?;
        body["result_sha256"] = Value::String(result_sha256);
        body["observed_outcome"] = observed_outcome;
        input.safe_receipt_json = canonical_json_string(&body)?;
        Ok(())
    }
}
