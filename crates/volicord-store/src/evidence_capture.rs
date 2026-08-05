use std::fs;

use chrono::Duration;
use rusqlite::{params, OptionalExtension};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use volicord_types::canonical::{canonical_json_bare_sha256, canonical_json_string};
use volicord_types::ids::{AgentConnectionId, BaselineRef};
use volicord_types::schema::{
    evidence_capture_input_sha256, validate_evidence_capture_expected_outcome,
    validate_evidence_capture_limitations, validate_evidence_capture_observed_outcome,
    EvidenceCaptureSpec, EvidenceProducer, EvidenceTarget, JsonObject,
    PersistedEvidenceCaptureReceiptBody, PersistedEvidenceCaptureReceiptSource, StateRecordRef,
    EVIDENCE_CAPTURE_INTENT_TTL_MINUTES, EVIDENCE_CAPTURE_RECEIPT_CONTRACT_ID,
};
use volicord_types::values::{ActorSource, EvidenceProducerKind, RedactionState, UtcTimestamp};

use crate::{
    artifacts::{
        insert_artifact_staging_tx, validate_insert as validate_staging_insert,
        ArtifactStagingInsert, StagedPayloadKind,
    },
    core_pipeline::{
        clock::advance_project_utc_floor_tx, validation::decode_owner_baseline_ref,
        CoreProjectStore,
    },
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
    pub baseline_ref: BaselineRef,
    pub target: EvidenceTarget,
    pub capture: EvidenceCaptureSpec,
    pub input_sha256: String,
    pub expected_outcome: JsonObject,
    pub requested_by_actor_source: ActorSource,
    pub requesting_connection_internal_id: AgentConnectionId,
    pub session_context: StoredEvidenceCaptureIntentSessionContext,
    pub workspace_context: JsonObject,
    pub created_at: UtcTimestamp,
    pub expires_at: UtcTimestamp,
    pub metadata: StoredEvidenceCaptureIntentMetadata,
}

/// Stored immutable evidence-capture intent facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceCaptureIntentRecord {
    pub project_id: String,
    pub evidence_capture_intent_id: String,
    pub task_id: String,
    pub change_unit_id: String,
    pub scope_revision: u64,
    pub baseline_ref: BaselineRef,
    pub target: EvidenceTarget,
    pub capture_kind: EvidenceProducerKind,
    pub capture: EvidenceCaptureSpec,
    pub input_sha256: String,
    pub expected_outcome: JsonObject,
    pub requested_by_actor_source: ActorSource,
    pub requesting_connection_internal_id: AgentConnectionId,
    pub session_context: StoredEvidenceCaptureIntentSessionContext,
    pub workspace_context: JsonObject,
    pub created_at: UtcTimestamp,
    pub expires_at: UtcTimestamp,
    pub metadata: StoredEvidenceCaptureIntentMetadata,
}

/// Persisted session coordinate carried by one evidence-capture intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredEvidenceCaptureIntentSessionContext {
    pub session_id: Option<String>,
}

/// Persisted invocation metadata carried by one evidence-capture intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredEvidenceCaptureIntentMetadata {
    pub verification_basis: String,
}

/// Source-neutral input for atomically staging and recording one complete safe receipt.
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceCaptureReceiptInsert {
    pub evidence_capture_receipt_id: String,
    pub evidence_capture_intent_id: String,
    pub staging_handle_id: String,
    pub task_id: String,
    pub input_sha256: String,
    pub result_sha256: String,
    pub expected_outcome: JsonObject,
    pub observed_outcome: JsonObject,
    pub source_refs: Vec<StateRecordRef>,
    pub observed_by_actor_source: ActorSource,
    pub observed_at: UtcTimestamp,
    pub limitations: Vec<String>,
    pub safe_receipt: PersistedEvidenceCaptureReceiptBody,
    pub created_at: UtcTimestamp,
    pub staging_expires_at: UtcTimestamp,
    pub metadata: StoredEvidenceCaptureReceiptMetadata,
}

/// Stored immutable source receipt facts.
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceCaptureReceiptRecord {
    pub project_id: String,
    pub evidence_capture_receipt_id: String,
    pub evidence_capture_intent_id: String,
    pub staging_handle_id: String,
    pub capture_kind: EvidenceProducerKind,
    pub input_sha256: String,
    pub result_sha256: String,
    pub expected_outcome: JsonObject,
    pub observed_outcome: JsonObject,
    pub source_refs: Vec<StateRecordRef>,
    pub observed_by_actor_source: ActorSource,
    pub observed_at: UtcTimestamp,
    pub completeness: EvidenceCaptureCompleteness,
    pub limitations: Vec<String>,
    pub safe_receipt: PersistedEvidenceCaptureReceiptBody,
    pub safe_receipt_sha256: String,
    pub safe_receipt_size_bytes: u64,
    pub created_at: UtcTimestamp,
    pub metadata: StoredEvidenceCaptureReceiptMetadata,
}

/// Closed completeness value persisted for a source receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceCaptureCompleteness {
    Complete,
}

/// Persisted source metadata carried by one evidence-capture receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredEvidenceCaptureReceiptMetadata {
    pub source: PersistedEvidenceCaptureReceiptSource,
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
    pub source_claim_kind: EvidenceCaptureSourceClaimKind,
    pub source_claim_id: String,
    pub evidence_capture_intent_id: String,
    pub evidence_capture_receipt_id: String,
    pub capture_kind: EvidenceProducerKind,
    pub claimed_at: UtcTimestamp,
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
#[derive(Debug, Clone, PartialEq)]
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
    pub baseline_ref: BaselineRef,
    pub producer_kind: EvidenceProducerKind,
    pub canonical_producer: EvidenceProducer,
    pub created_at: UtcTimestamp,
    pub metadata: StoredEvidenceProducerMetadata,
}

/// Stored canonical evidence-producer facts.
#[derive(Debug, Clone, PartialEq)]
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
    pub baseline_ref: BaselineRef,
    pub producer_kind: EvidenceProducerKind,
    pub canonical_producer: EvidenceProducer,
    pub created_at: UtcTimestamp,
    pub metadata: StoredEvidenceProducerMetadata,
}

/// Persisted producer metadata selected by Core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredEvidenceProducerMetadata {
    pub verification_basis: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawEvidenceCaptureIntentRecord {
    project_id: String,
    evidence_capture_intent_id: String,
    task_id: String,
    change_unit_id: String,
    scope_revision: i64,
    baseline_ref: String,
    target_json: String,
    capture_kind: String,
    capture_spec_json: String,
    input_sha256: String,
    expected_outcome_json: String,
    requested_by_actor_source: String,
    requesting_connection_internal_id: String,
    session_context_json: String,
    workspace_context_json: String,
    created_at: String,
    expires_at: String,
    metadata_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawEvidenceCaptureReceiptRecord {
    project_id: String,
    evidence_capture_receipt_id: String,
    evidence_capture_intent_id: String,
    staging_handle_id: String,
    capture_kind: String,
    input_sha256: String,
    result_sha256: String,
    expected_outcome_json: String,
    observed_outcome_json: String,
    source_refs_json: String,
    observed_by_actor_source: String,
    observed_at: String,
    completeness: String,
    limitations_json: String,
    safe_receipt_json: String,
    safe_receipt_sha256: String,
    safe_receipt_size_bytes: i64,
    created_at: String,
    metadata_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawEvidenceProducerRecord {
    project_id: String,
    evidence_producer_id: String,
    evidence_capture_intent_id: String,
    evidence_capture_receipt_id: String,
    evidence_observation_id: String,
    artifact_id: String,
    run_id: String,
    task_id: String,
    change_unit_id: String,
    scope_revision: i64,
    baseline_ref: String,
    producer_kind: String,
    canonical_producer_json: String,
    created_at: String,
    metadata_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawEvidenceCaptureSourceClaimRecord {
    project_id: String,
    source_claim_kind: String,
    source_claim_id: String,
    evidence_capture_intent_id: String,
    evidence_capture_receipt_id: String,
    capture_kind: String,
    claimed_at: String,
}

impl CoreProjectStore<'_> {
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
            || body.source.connection_id != intent.requesting_connection_internal_id
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
                        && record.source_claim_kind == identity.source_claim_kind
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
        self.require_mutation_context()?;
        validate_receipt_input(&input)?;
        let intent = self
            .evidence_capture_intent_record(&input.evidence_capture_intent_id)?
            .ok_or_else(|| StoreError::NotFound {
                entity: "evidence_capture_intent",
                id: input.evidence_capture_intent_id.clone(),
            })?;
        let validated_source = validate_receipt_against_intent(&input, &intent)?;
        let created_at = input.created_at.clone();
        let safe_receipt_json = canonical_json_string(&input.safe_receipt).map_err(|error| {
            StoreError::InvalidInput {
                detail: format!("safe receipt could not be serialized: {error}"),
            }
        })?;
        let expected_outcome_json = encode_input_json("expected_outcome", &input.expected_outcome)?;
        let observed_outcome_json = encode_input_json("observed_outcome", &input.observed_outcome)?;
        let source_refs_json = encode_input_json("source_refs", &input.source_refs)?;
        let limitations_json = encode_input_json("limitations", &input.limitations)?;
        let metadata_json = encode_input_json("metadata", &input.metadata)?;
        let safe_bytes = safe_receipt_json.as_bytes().to_vec();
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
            redaction_state: RedactionState::Redacted,
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
                evidence_producer_kind_storage_value(input.safe_receipt.capture_kind),
                input.input_sha256,
                input.result_sha256,
                expected_outcome_json,
                observed_outcome_json,
                source_refs_json,
                input.observed_by_actor_source.to_canonical_string(),
                input.observed_at.to_canonical_string(),
                limitations_json,
                safe_receipt_json,
                safe_receipt_sha256,
                i64::try_from(safe_receipt_size_bytes).map_err(|_| StoreError::InvalidInput {
                    detail: "safe evidence-capture receipt size does not fit in SQLite integer"
                        .to_owned(),
                })?,
                input.created_at.to_canonical_string(),
                metadata_json
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
                    evidence_producer_kind_storage_value(input.safe_receipt.capture_kind),
                    input.created_at.to_canonical_string(),
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
    let raw = conn
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
                Ok(RawEvidenceCaptureIntentRecord {
                    project_id: row.get(0)?,
                    evidence_capture_intent_id: row.get(1)?,
                    task_id: row.get(2)?,
                    change_unit_id: row.get(3)?,
                    scope_revision: row.get(4)?,
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
    raw.map(decode_intent_record).transpose()
}

fn decode_intent_record(
    raw: RawEvidenceCaptureIntentRecord,
) -> StoreResult<EvidenceCaptureIntentRecord> {
    let record_id = raw.evidence_capture_intent_id.clone();
    let created_at = decode_owner_timestamp(
        "evidence_capture_intents",
        &record_id,
        "created_at",
        &raw.created_at,
    )?;
    let expires_at = decode_owner_timestamp(
        "evidence_capture_intents",
        &record_id,
        "expires_at",
        &raw.expires_at,
    )?;
    validate_evidence_capture_intent_window(&raw.created_at, &raw.expires_at).map_err(|field| {
        StoreError::corrupt_owner_state_value(
            "evidence_capture_intents",
            record_id.clone(),
            match field {
                EvidenceCaptureIntentWindowError::CreatedAt => "created_at",
                EvidenceCaptureIntentWindowError::ExpiresAt => "expires_at",
            },
        )
    })?;
    let capture = decode_owner_json::<EvidenceCaptureSpec>(
        "evidence_capture_intents",
        &record_id,
        "capture_spec_json",
        &raw.capture_spec_json,
    )?;
    let capture_kind = decode_owner_value::<EvidenceProducerKind>(
        "evidence_capture_intents",
        &record_id,
        "capture_kind",
        &raw.capture_kind,
    )?;
    if capture_kind != producer_kind_for_capture(&capture) {
        return Err(StoreError::corrupt_owner_state_value(
            "evidence_capture_intents",
            record_id,
            "capture_kind",
        ));
    }
    let input_sha256 = evidence_capture_input_sha256(&capture).map_err(|_| {
        StoreError::corrupt_owner_state_json(
            "evidence_capture_intents",
            raw.evidence_capture_intent_id.clone(),
            "capture_spec_json",
        )
    })?;
    if input_sha256 != raw.input_sha256 {
        return Err(StoreError::corrupt_owner_state_value(
            "evidence_capture_intents",
            raw.evidence_capture_intent_id,
            "input_sha256",
        ));
    }
    let expected_outcome = decode_owner_json::<JsonObject>(
        "evidence_capture_intents",
        &record_id,
        "expected_outcome_json",
        &raw.expected_outcome_json,
    )?;
    validate_evidence_capture_expected_outcome(&capture, &expected_outcome).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "evidence_capture_intents",
            record_id.clone(),
            "expected_outcome_json",
        )
    })?;
    Ok(EvidenceCaptureIntentRecord {
        project_id: raw.project_id,
        evidence_capture_intent_id: record_id.clone(),
        task_id: raw.task_id,
        change_unit_id: raw.change_unit_id,
        scope_revision: decode_owner_nonnegative(
            "evidence_capture_intents",
            &record_id,
            "scope_revision",
            raw.scope_revision,
        )?,
        baseline_ref: decode_owner_baseline_ref(
            "evidence_capture_intents",
            &record_id,
            "baseline_ref",
            raw.baseline_ref,
        )?,
        target: decode_owner_json(
            "evidence_capture_intents",
            &record_id,
            "target_json",
            &raw.target_json,
        )?,
        capture_kind,
        capture,
        input_sha256: raw.input_sha256,
        expected_outcome,
        requested_by_actor_source: decode_owner_actor_source(
            "evidence_capture_intents",
            &record_id,
            "requested_by_actor_source",
            &raw.requested_by_actor_source,
        )?,
        requesting_connection_internal_id: AgentConnectionId::new(
            raw.requesting_connection_internal_id,
        ),
        session_context: decode_owner_json(
            "evidence_capture_intents",
            &record_id,
            "session_context_json",
            &raw.session_context_json,
        )?,
        workspace_context: decode_owner_json(
            "evidence_capture_intents",
            &record_id,
            "workspace_context_json",
            &raw.workspace_context_json,
        )?,
        created_at,
        expires_at,
        metadata: decode_owner_json(
            "evidence_capture_intents",
            &record_id,
            "metadata_json",
            &raw.metadata_json,
        )?,
    })
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
        raw_receipt_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|raw| raw.map(decode_receipt_record).transpose())
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
        raw_receipt_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|raw| raw.map(decode_receipt_record).transpose())
}

fn raw_receipt_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RawEvidenceCaptureReceiptRecord> {
    Ok(RawEvidenceCaptureReceiptRecord {
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
        safe_receipt_size_bytes: row.get(16)?,
        created_at: row.get(17)?,
        metadata_json: row.get(18)?,
    })
}

fn decode_receipt_record(
    raw: RawEvidenceCaptureReceiptRecord,
) -> StoreResult<EvidenceCaptureReceiptRecord> {
    let record_id = raw.evidence_capture_receipt_id.clone();
    let safe_receipt = decode_owner_json::<PersistedEvidenceCaptureReceiptBody>(
        "evidence_capture_receipts",
        &record_id,
        "safe_receipt_json",
        &raw.safe_receipt_json,
    )?;
    let canonical_safe_receipt = canonical_json_string(&safe_receipt).map_err(|_| {
        StoreError::corrupt_owner_state_json(
            "evidence_capture_receipts",
            record_id.clone(),
            "safe_receipt_json",
        )
    })?;
    let safe_receipt_size_bytes = decode_owner_nonnegative(
        "evidence_capture_receipts",
        &record_id,
        "safe_receipt_size_bytes",
        raw.safe_receipt_size_bytes,
    )?;
    if raw.safe_receipt_json.len() > MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES
        || raw.metadata_json.len() > MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES
        || canonical_safe_receipt != raw.safe_receipt_json
        || safe_receipt_size_bytes != raw.safe_receipt_json.len() as u64
        || sha256_hex(raw.safe_receipt_json.as_bytes()) != raw.safe_receipt_sha256
    {
        return Err(StoreError::corrupt_owner_state_value(
            "evidence_capture_receipts",
            record_id,
            "safe_receipt_json",
        ));
    }
    let capture_kind = decode_owner_value(
        "evidence_capture_receipts",
        &record_id,
        "capture_kind",
        &raw.capture_kind,
    )?;
    let expected_outcome = decode_owner_json(
        "evidence_capture_receipts",
        &record_id,
        "expected_outcome_json",
        &raw.expected_outcome_json,
    )?;
    let observed_outcome = decode_owner_json(
        "evidence_capture_receipts",
        &record_id,
        "observed_outcome_json",
        &raw.observed_outcome_json,
    )?;
    let source_refs = decode_owner_json(
        "evidence_capture_receipts",
        &record_id,
        "source_refs_json",
        &raw.source_refs_json,
    )?;
    let limitations = decode_owner_json(
        "evidence_capture_receipts",
        &record_id,
        "limitations_json",
        &raw.limitations_json,
    )?;
    let metadata = decode_owner_json(
        "evidence_capture_receipts",
        &record_id,
        "metadata_json",
        &raw.metadata_json,
    )?;
    let record = EvidenceCaptureReceiptRecord {
        project_id: raw.project_id,
        evidence_capture_receipt_id: record_id.clone(),
        evidence_capture_intent_id: raw.evidence_capture_intent_id,
        staging_handle_id: raw.staging_handle_id,
        capture_kind,
        input_sha256: raw.input_sha256,
        result_sha256: raw.result_sha256,
        expected_outcome,
        observed_outcome,
        source_refs,
        observed_by_actor_source: decode_owner_actor_source(
            "evidence_capture_receipts",
            &record_id,
            "observed_by_actor_source",
            &raw.observed_by_actor_source,
        )?,
        observed_at: decode_owner_timestamp(
            "evidence_capture_receipts",
            &record_id,
            "observed_at",
            &raw.observed_at,
        )?,
        completeness: decode_owner_value(
            "evidence_capture_receipts",
            &record_id,
            "completeness",
            &raw.completeness,
        )?,
        limitations,
        safe_receipt,
        safe_receipt_sha256: raw.safe_receipt_sha256,
        safe_receipt_size_bytes,
        created_at: decode_owner_timestamp(
            "evidence_capture_receipts",
            &record_id,
            "created_at",
            &raw.created_at,
        )?,
        metadata,
    };
    validate_decoded_receipt_owner_fields(&record)?;
    Ok(record)
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
        raw_source_claim_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|raw| raw.map(decode_source_claim_record).transpose())
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
    let rows = statement.query_map(params![project_id, receipt_id], raw_source_claim_from_row)?;
    rows.map(|raw| {
        raw.map_err(StoreError::from)
            .and_then(decode_source_claim_record)
    })
    .collect()
}

fn raw_source_claim_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RawEvidenceCaptureSourceClaimRecord> {
    Ok(RawEvidenceCaptureSourceClaimRecord {
        project_id: row.get(0)?,
        source_claim_kind: row.get(1)?,
        source_claim_id: row.get(2)?,
        evidence_capture_intent_id: row.get(3)?,
        evidence_capture_receipt_id: row.get(4)?,
        capture_kind: row.get(5)?,
        claimed_at: row.get(6)?,
    })
}

fn decode_source_claim_record(
    raw: RawEvidenceCaptureSourceClaimRecord,
) -> StoreResult<EvidenceCaptureSourceClaimRecord> {
    let record_id = raw.source_claim_id.clone();
    let source_claim_kind = match raw.source_claim_kind.as_str() {
        "host_invocation" => EvidenceCaptureSourceClaimKind::HostInvocation,
        _ => {
            return Err(StoreError::corrupt_owner_state_value(
                "evidence_capture_source_claims",
                record_id,
                "source_claim_kind",
            ))
        }
    };
    Ok(EvidenceCaptureSourceClaimRecord {
        project_id: raw.project_id,
        source_claim_kind,
        source_claim_id: raw.source_claim_id,
        evidence_capture_intent_id: raw.evidence_capture_intent_id,
        evidence_capture_receipt_id: raw.evidence_capture_receipt_id,
        capture_kind: decode_owner_value(
            "evidence_capture_source_claims",
            &record_id,
            "capture_kind",
            &raw.capture_kind,
        )?,
        claimed_at: decode_owner_timestamp(
            "evidence_capture_source_claims",
            &record_id,
            "claimed_at",
            &raw.claimed_at,
        )?,
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
        raw_producer_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|raw| raw.map(decode_producer_record).transpose())
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
        raw_producer_from_row,
    )
    .optional()
    .map_err(StoreError::from)
    .and_then(|raw| raw.map(decode_producer_record).transpose())
}

fn raw_producer_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawEvidenceProducerRecord> {
    Ok(RawEvidenceProducerRecord {
        project_id: row.get(0)?,
        evidence_producer_id: row.get(1)?,
        evidence_capture_intent_id: row.get(2)?,
        evidence_capture_receipt_id: row.get(3)?,
        evidence_observation_id: row.get(4)?,
        artifact_id: row.get(5)?,
        run_id: row.get(6)?,
        task_id: row.get(7)?,
        change_unit_id: row.get(8)?,
        scope_revision: row.get(9)?,
        baseline_ref: row.get(10)?,
        producer_kind: row.get(11)?,
        canonical_producer_json: row.get(12)?,
        created_at: row.get(13)?,
        metadata_json: row.get(14)?,
    })
}

fn decode_producer_record(raw: RawEvidenceProducerRecord) -> StoreResult<EvidenceProducerRecord> {
    let record_id = raw.evidence_producer_id.clone();
    let canonical_producer = decode_owner_json(
        "evidence_producers",
        &record_id,
        "canonical_producer_json",
        &raw.canonical_producer_json,
    )?;
    if canonical_json_string(&canonical_producer).map_err(|_| {
        StoreError::corrupt_owner_state_json(
            "evidence_producers",
            record_id.clone(),
            "canonical_producer_json",
        )
    })? != raw.canonical_producer_json
    {
        return Err(StoreError::corrupt_owner_state_value(
            "evidence_producers",
            record_id,
            "canonical_producer_json",
        ));
    }
    let record = EvidenceProducerRecord {
        project_id: raw.project_id,
        evidence_producer_id: record_id.clone(),
        evidence_capture_intent_id: raw.evidence_capture_intent_id,
        evidence_capture_receipt_id: raw.evidence_capture_receipt_id,
        evidence_observation_id: raw.evidence_observation_id,
        artifact_id: raw.artifact_id,
        run_id: raw.run_id,
        task_id: raw.task_id,
        change_unit_id: raw.change_unit_id,
        scope_revision: decode_owner_nonnegative(
            "evidence_producers",
            &record_id,
            "scope_revision",
            raw.scope_revision,
        )?,
        baseline_ref: decode_owner_baseline_ref(
            "evidence_producers",
            &record_id,
            "baseline_ref",
            raw.baseline_ref,
        )?,
        producer_kind: decode_owner_value(
            "evidence_producers",
            &record_id,
            "producer_kind",
            &raw.producer_kind,
        )?,
        canonical_producer,
        created_at: decode_owner_timestamp(
            "evidence_producers",
            &record_id,
            "created_at",
            &raw.created_at,
        )?,
        metadata: decode_owner_json(
            "evidence_producers",
            &record_id,
            "metadata_json",
            &raw.metadata_json,
        )?,
    };
    validate_decoded_producer_owner_fields(&record)?;
    Ok(record)
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
    ] {
        if value.trim().is_empty() {
            return Err(StoreError::InvalidInput {
                detail: format!("{field} must not be empty"),
            });
        }
    }
    validate_sha256("input_sha256", &input.input_sha256)?;
    validate_sha256("result_sha256", &input.result_sha256)?;
    for (field, value) in [
        ("observed_at", &input.observed_at),
        ("created_at", &input.created_at),
        ("staging_expires_at", &input.staging_expires_at),
    ] {
        value
            .ensure_canonical_rfc3339_representable()
            .map_err(|_| StoreError::InvalidInput {
                detail: format!("{field} must be a canonical four-digit RFC 3339 timestamp"),
            })?;
    }
    let safe_receipt_json = encode_input_json("safe_receipt", &input.safe_receipt)?;
    if safe_receipt_json.len() > MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES {
        return Err(StoreError::InvalidInput {
            detail: format!(
                "safe_receipt_json exceeds the {MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES}-byte bound"
            ),
        });
    }
    let metadata_json = encode_input_json("metadata", &input.metadata)?;
    if metadata_json.len() > MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES {
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
        || input.safe_receipt.capture_kind != intent.capture_kind
        || input.input_sha256 != intent.input_sha256
        || input.expected_outcome != intent.expected_outcome
    {
        return Err(StoreError::Conflict {
            entity: "evidence_capture_intent",
            id: intent.evidence_capture_intent_id.clone(),
            detail: "source receipt does not match the immutable capture intent".to_owned(),
        });
    }
    if !input.source_refs.is_empty() {
        return Err(StoreError::InvalidInput {
            detail: "source_refs must be empty for source fulfillment".to_owned(),
        });
    }
    let body = &input.safe_receipt;
    let capture_spec = &intent.capture;
    if evidence_capture_input_sha256(capture_spec).map_err(|_| {
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
    validate_evidence_capture_expected_outcome(capture_spec, &input.expected_outcome).map_err(
        |detail| StoreError::InvalidInput {
            detail: format!("expected_outcome does not match its capture class: {detail}"),
        },
    )?;
    validate_evidence_capture_observed_outcome(capture_spec, &input.observed_outcome).map_err(
        |detail| StoreError::InvalidInput {
            detail: format!("observed_outcome does not match its capture class: {detail}"),
        },
    )?;
    validate_evidence_capture_limitations(capture_spec, &input.limitations).map_err(|detail| {
        StoreError::InvalidInput {
            detail: format!("limitations do not match their capture class: {detail}"),
        }
    })?;
    let result_sha256 = canonical_json_bare_sha256(&body.observed_outcome).map_err(|error| {
        StoreError::InvalidInput {
            detail: format!("observed outcome could not be hashed: {error}"),
        }
    })?;
    let expected_metadata = StoredEvidenceCaptureReceiptMetadata {
        source: body.source.clone(),
    };
    if body.contract_id != EVIDENCE_CAPTURE_RECEIPT_CONTRACT_ID
        || !body.complete
        || body.redaction_state != RedactionState::Redacted
        || body.capture_kind != intent.capture_kind
        || body.capture_intent_id.as_str() != intent.evidence_capture_intent_id
        || body.input_sha256 != input.input_sha256
        || body.result_sha256 != input.result_sha256
        || body.result_sha256 != result_sha256
        || body.expected_outcome != input.expected_outcome
        || body.observed_outcome != input.observed_outcome
        || body.limitations != input.limitations
        || body.observed_by_actor_source != input.observed_by_actor_source
        || body.observed_by_actor_source != intent.requested_by_actor_source
        || body.source.connection_id != intent.requesting_connection_internal_id
        || body.observed_at != input.observed_at
        || input.metadata != expected_metadata
    {
        return Err(StoreError::Conflict {
            entity: "evidence_capture_intent",
            id: intent.evidence_capture_intent_id.clone(),
            detail: "source receipt body does not match the immutable capture intent and receipt columns"
                .to_owned(),
        });
    }
    let source_claims =
        derive_evidence_capture_source_claims(capture_spec, body).map_err(|_| {
            StoreError::Conflict {
                entity: "evidence_capture_intent",
                id: intent.evidence_capture_intent_id.clone(),
                detail: "receipt source shape does not match the immutable capture class"
                    .to_owned(),
            }
        })?;
    if input.observed_at < intent.created_at || input.observed_at >= intent.expires_at {
        return Err(StoreError::Conflict {
            entity: "evidence_capture_intent",
            id: intent.evidence_capture_intent_id.clone(),
            detail: "source receipt observation is outside the capture intent validity window"
                .to_owned(),
        });
    }
    if input.created_at < input.observed_at || input.created_at >= intent.expires_at {
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

fn evidence_producer_kind_storage_value(value: EvidenceProducerKind) -> &'static str {
    match value {
        EvidenceProducerKind::UnverifiedCaller => "unverified_caller",
        EvidenceProducerKind::UserChannelObservation => "user_channel_observation",
        EvidenceProducerKind::VerifiedToolInvocation => "verified_tool_invocation",
        EvidenceProducerKind::VerifiedCommandExecution => "verified_command_execution",
        EvidenceProducerKind::ReusedEvidence => "reused_evidence",
    }
}

fn encode_input_json<T: Serialize>(field: &'static str, value: &T) -> StoreResult<String> {
    canonical_json_string(value).map_err(|error| StoreError::InvalidInput {
        detail: format!("{field} could not be serialized: {error}"),
    })
}

fn decode_owner_json<T: DeserializeOwned>(
    table: &'static str,
    record_id: &str,
    column: &'static str,
    text: &str,
) -> StoreResult<T> {
    serde_json::from_str(text)
        .map_err(|_| StoreError::corrupt_owner_state_json(table, record_id.to_owned(), column))
}

fn decode_owner_value<T: DeserializeOwned>(
    table: &'static str,
    record_id: &str,
    column: &'static str,
    value: &str,
) -> StoreResult<T> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| StoreError::corrupt_owner_state_value(table, record_id.to_owned(), column))
}

fn decode_owner_actor_source(
    table: &'static str,
    record_id: &str,
    column: &'static str,
    value: &str,
) -> StoreResult<ActorSource> {
    let actor = value
        .parse::<ActorSource>()
        .map_err(|_| StoreError::corrupt_owner_state_value(table, record_id.to_owned(), column))?;
    if actor.to_canonical_string() == value {
        Ok(actor)
    } else {
        Err(StoreError::corrupt_owner_state_value(
            table,
            record_id.to_owned(),
            column,
        ))
    }
}

fn decode_owner_timestamp(
    table: &'static str,
    record_id: &str,
    column: &'static str,
    value: &str,
) -> StoreResult<UtcTimestamp> {
    let timestamp = UtcTimestamp::parse(value)
        .map_err(|_| StoreError::corrupt_owner_state_value(table, record_id.to_owned(), column))?;
    timestamp
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| StoreError::corrupt_owner_state_value(table, record_id.to_owned(), column))?;
    if timestamp.to_canonical_string() == value {
        Ok(timestamp)
    } else {
        Err(StoreError::corrupt_owner_state_value(
            table,
            record_id.to_owned(),
            column,
        ))
    }
}

fn decode_owner_nonnegative(
    table: &'static str,
    record_id: &str,
    column: &'static str,
    value: i64,
) -> StoreResult<u64> {
    u64::try_from(value)
        .map_err(|_| StoreError::corrupt_owner_state_value(table, record_id.to_owned(), column))
}

fn validate_decoded_receipt_owner_fields(record: &EvidenceCaptureReceiptRecord) -> StoreResult<()> {
    let body = &record.safe_receipt;
    let result_sha256 = canonical_json_bare_sha256(&body.observed_outcome).map_err(|_| {
        StoreError::corrupt_owner_state_value(
            "evidence_capture_receipts",
            record.evidence_capture_receipt_id.clone(),
            "observed_outcome_json",
        )
    })?;
    if body.contract_id != EVIDENCE_CAPTURE_RECEIPT_CONTRACT_ID
        || !body.complete
        || body.redaction_state != RedactionState::Redacted
        || record.completeness != EvidenceCaptureCompleteness::Complete
        || body.capture_kind != record.capture_kind
        || body.capture_intent_id.as_str() != record.evidence_capture_intent_id
        || body.input_sha256 != record.input_sha256
        || body.result_sha256 != record.result_sha256
        || body.result_sha256 != result_sha256
        || body.expected_outcome != record.expected_outcome
        || body.observed_outcome != record.observed_outcome
        || !record.source_refs.is_empty()
        || body.limitations != record.limitations
        || body.observed_by_actor_source != record.observed_by_actor_source
        || body.observed_at != record.observed_at
        || record.metadata.source != body.source
    {
        return Err(StoreError::corrupt_owner_state_value(
            "evidence_capture_receipts",
            record.evidence_capture_receipt_id.clone(),
            "safe_receipt_json",
        ));
    }
    Ok(())
}

fn validate_decoded_producer_owner_fields(record: &EvidenceProducerRecord) -> StoreResult<()> {
    let producer = &record.canonical_producer;
    let mismatch = if producer.evidence_producer_id.as_str() != record.evidence_producer_id {
        Some("canonical_producer_json.evidence_producer_id")
    } else if producer.capture_intent_id.as_str() != record.evidence_capture_intent_id {
        Some("canonical_producer_json.capture_intent_id")
    } else if producer.capture_receipt_id.as_str() != record.evidence_capture_receipt_id {
        Some("canonical_producer_json.capture_receipt_id")
    } else if producer.observation_ref.record_id.as_str() != record.evidence_observation_id {
        Some("canonical_producer_json.observation_ref")
    } else if producer.receipt_artifact_refs.len() != 1
        || producer.receipt_artifact_refs[0].artifact_id.as_str() != record.artifact_id
    {
        Some("canonical_producer_json.receipt_artifact_refs")
    } else if producer.run_ref.record_id.as_str() != record.run_id {
        Some("canonical_producer_json.run_ref")
    } else if producer.project_id.as_str() != record.project_id {
        Some("canonical_producer_json.project_id")
    } else if producer.task_id.as_str() != record.task_id {
        Some("canonical_producer_json.task_id")
    } else if producer.change_unit_id.as_str() != record.change_unit_id {
        Some("canonical_producer_json.change_unit_id")
    } else if producer.scope_revision != record.scope_revision {
        Some("canonical_producer_json.scope_revision")
    } else if producer.baseline_ref != record.baseline_ref {
        Some("canonical_producer_json.baseline_ref")
    } else if producer.producer_kind != record.producer_kind {
        Some("canonical_producer_json.producer_kind")
    } else if producer.finalized_at != record.created_at {
        Some("canonical_producer_json.finalized_at")
    } else {
        None
    };
    if let Some(logical_column) = mismatch {
        return Err(StoreError::corrupt_owner_state_value(
            "evidence_producers",
            record.evidence_producer_id.clone(),
            logical_column,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::{
        ids::EvidenceClaimId,
        schema::{evidence_capture_expected_outcome, RequiredNullable},
    };

    fn valid_raw_intent() -> RawEvidenceCaptureIntentRecord {
        let capture = EvidenceCaptureSpec::VerifiedCommandExecution {
            command_sha256: "a".repeat(64),
            command_label: "Run bounded verification.".to_owned(),
            expected_exit_code: RequiredNullable::some(0),
        };
        let target = EvidenceTarget::SupplementalClaim {
            evidence_claim_id: EvidenceClaimId::new("claim"),
            statement: "The bounded verification succeeds.".to_owned(),
        };
        RawEvidenceCaptureIntentRecord {
            project_id: "project".to_owned(),
            evidence_capture_intent_id: "intent".to_owned(),
            task_id: "task".to_owned(),
            change_unit_id: "change".to_owned(),
            scope_revision: 1,
            baseline_ref: "baseline".to_owned(),
            target_json: serde_json::to_string(&target).expect("target must serialize"),
            capture_kind: "verified_command_execution".to_owned(),
            capture_spec_json: serde_json::to_string(&capture).expect("capture must serialize"),
            input_sha256: "a".repeat(64),
            expected_outcome_json: serde_json::to_string(&evidence_capture_expected_outcome(
                &capture,
            ))
            .expect("expected outcome must serialize"),
            requested_by_actor_source: ActorSource::System.to_canonical_string(),
            requesting_connection_internal_id: "connection".to_owned(),
            session_context_json: r#"{"session_id":null}"#.to_owned(),
            workspace_context_json: "{}".to_owned(),
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            expires_at: "2026-01-01T00:15:00Z".to_owned(),
            metadata_json: r#"{"verification_basis":"test"}"#.to_owned(),
        }
    }

    #[test]
    fn capture_intent_decoder_owns_structured_and_closed_value_corruption() {
        let decoded =
            decode_intent_record(valid_raw_intent()).expect("valid physical row must decode");
        assert_eq!(
            decoded.capture_kind,
            EvidenceProducerKind::VerifiedCommandExecution
        );

        let mut unknown_kind = valid_raw_intent();
        unknown_kind.capture_kind = "legacy".to_owned();
        assert!(matches!(
            decode_intent_record(unknown_kind),
            Err(StoreError::CorruptOwnerStateValue {
                table: "evidence_capture_intents",
                logical_column: "capture_kind",
                ..
            })
        ));

        for malformed in [
            "{",
            r#"{"capture_kind":"verified_command_execution"}"#,
            r#"{"capture_kind":"verified_command_execution","command_sha256":0,"command_label":"verify","expected_exit_code":0}"#,
        ] {
            let mut row = valid_raw_intent();
            row.capture_spec_json = malformed.to_owned();
            assert!(matches!(
                decode_intent_record(row),
                Err(StoreError::CorruptOwnerStateJson {
                    table: "evidence_capture_intents",
                    logical_column: "capture_spec_json",
                    ..
                })
            ));
        }
    }

    #[test]
    fn capture_intent_decoder_rejects_every_noncanonical_scalar_baseline() {
        for invalid in BaselineRef::spec().generated_invalid_corpus() {
            let mut row = valid_raw_intent();
            row.baseline_ref = invalid;
            assert!(matches!(
                decode_intent_record(row),
                Err(StoreError::CorruptOwnerStateValue {
                    table: "evidence_capture_intents",
                    record_ref,
                    logical_column: "baseline_ref",
                    ..
                }) if record_ref == "intent"
            ));
        }
    }

    #[test]
    fn source_claim_decoder_owns_closed_values_and_timestamps() {
        let valid = || RawEvidenceCaptureSourceClaimRecord {
            project_id: "project".to_owned(),
            source_claim_kind: "host_invocation".to_owned(),
            source_claim_id: "claim".to_owned(),
            evidence_capture_intent_id: "intent".to_owned(),
            evidence_capture_receipt_id: "receipt".to_owned(),
            capture_kind: "verified_command_execution".to_owned(),
            claimed_at: "2026-01-01T00:00:00Z".to_owned(),
        };
        let decoded =
            decode_source_claim_record(valid()).expect("valid physical source claim must decode");
        assert_eq!(
            decoded.source_claim_kind,
            EvidenceCaptureSourceClaimKind::HostInvocation
        );
        assert_eq!(
            decoded.capture_kind,
            EvidenceProducerKind::VerifiedCommandExecution
        );

        let mut unknown_kind = valid();
        unknown_kind.source_claim_kind = "legacy".to_owned();
        assert!(matches!(
            decode_source_claim_record(unknown_kind),
            Err(StoreError::CorruptOwnerStateValue {
                table: "evidence_capture_source_claims",
                logical_column: "source_claim_kind",
                ..
            })
        ));

        let mut invalid_time = valid();
        invalid_time.claimed_at = "not-a-time".to_owned();
        assert!(matches!(
            decode_source_claim_record(invalid_time),
            Err(StoreError::CorruptOwnerStateValue {
                table: "evidence_capture_source_claims",
                logical_column: "claimed_at",
                ..
            })
        ));
    }
}
