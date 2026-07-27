use super::{
    artifact_ref_from_verified_record, decode_required_json, object_from_value,
    parse_owner_storage_value, persistent_artifact_is_verified_current, state_ref,
};
use crate::pipeline::{CorePipelineError, CoreResult};
use crate::policy::{
    close_readiness_evidence::{CloseEvidenceRunFacts, CloseEvidenceSummaryFacts},
    evidence_binding::{
        authority_ref_matches, exact_artifact_identity_matches, exact_artifact_ref_sets_match,
        producer_output_binding_matches, projected_capture_binding_matches,
    },
    evidence_provenance::{evidence_assurance_matches_source, EvidenceProvenanceFacts},
    evidence_relevance::{capture_outcome_relevance, relevance_supports_claim},
    evidence_target::{
        projected_observation_matches_basis, stored_observation_matches_basis,
        EvidenceObservationBasis,
    },
};
use chrono::Duration;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use volicord_store::core_pipeline::{
    CoreProjectStore, EffectiveUserActionRecord, EvidenceObservationRecord, EvidenceSummaryRecord,
    TaskRecord, UserActionResolutionRecord,
};
use volicord_store::error::StoreError;
use volicord_store::evidence_capture::{
    EvidenceCaptureIntentRecord, EvidenceCaptureReceiptRecord, MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES,
};
use volicord_types::canonical::canonical_json_string;
use volicord_types::ids::{BaselineRef, ChangeUnitId, EvidenceCaptureIntentId, ProjectId, TaskId};
use volicord_types::schema::{
    evidence_capture_input_sha256, evidence_capture_observed_outcome_matches_expected,
    validate_evidence_capture_expected_outcome, validate_evidence_capture_limitations,
    validate_evidence_capture_observed_outcome, ArtifactRef, EvidenceCaptureIntent,
    EvidenceCaptureSpec, EvidenceCoverageItem, EvidenceObservation, EvidenceProducer,
    EvidenceProducerAnchor, EvidenceRelevanceAssessment, EvidenceTarget, JsonObject,
    PersistedEvidenceCaptureReceiptBody, PersistedEvidenceMetadata,
    PersistedEvidenceObservationAuthority, StateRecordRef, UserActionResolutionBody,
    EVIDENCE_CAPTURE_INTENT_TTL_MINUTES,
};
use volicord_types::values::{
    ActorSource, ArtifactAvailability, ArtifactIntegrityStatus, EvidenceAssuranceLevel,
    EvidenceProducerKind, EvidenceRelevanceStatus, EvidenceRequirement, EvidenceSourceKind,
    RedactionState, StateRecordKind, UserActionKind, UserActionStatus, UtcTimestamp,
};

pub(super) struct UserActionObservationResolutionAuthority {
    pub(super) relevance_status: EvidenceRelevanceStatus,
    pub(super) resolved_at: UtcTimestamp,
}

pub(super) fn decode_capture_intent_record(
    record: &EvidenceCaptureIntentRecord,
) -> CoreResult<EvidenceCaptureIntent> {
    let corrupt = |column: &'static str| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_value(
            "evidence_capture_intents",
            record.evidence_capture_intent_id.clone(),
            column,
        ))
    };
    let target = serde_json::from_str::<EvidenceTarget>(&record.target_json)
        .map_err(|_| corrupt("target_json"))?;
    let capture = serde_json::from_str::<EvidenceCaptureSpec>(&record.capture_spec_json)
        .map_err(|_| corrupt("capture_spec_json"))?;
    if evidence_capture_input_sha256(&capture).map_err(|_| corrupt("capture_spec_json"))?
        != record.input_sha256
    {
        return Err(corrupt("input_sha256"));
    }
    let expected_outcome = serde_json::from_str::<JsonObject>(&record.expected_outcome_json)
        .map_err(|_| corrupt("expected_outcome_json"))?;
    validate_evidence_capture_expected_outcome(&capture, &expected_outcome)
        .map_err(|_| corrupt("expected_outcome_json"))?;
    let requested_by_actor_source = record
        .requested_by_actor_source
        .parse::<ActorSource>()
        .map_err(|_| corrupt("requested_by_actor_source"))?;
    let workspace_context = serde_json::from_str::<JsonObject>(&record.workspace_context_json)
        .map_err(|_| corrupt("workspace_context_json"))?;
    let created_at = UtcTimestamp::parse(&record.created_at).map_err(|_| corrupt("created_at"))?;
    let expires_at = UtcTimestamp::parse(&record.expires_at).map_err(|_| corrupt("expires_at"))?;
    created_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| corrupt("created_at"))?;
    expires_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| corrupt("expires_at"))?;
    let expected_expires_at = created_at
        .checked_add(Duration::minutes(EVIDENCE_CAPTURE_INTENT_TTL_MINUTES))
        .map_err(|_| corrupt("expires_at"))?;
    if expires_at != expected_expires_at {
        return Err(corrupt("expires_at"));
    }
    Ok(EvidenceCaptureIntent {
        capture_intent_id: EvidenceCaptureIntentId::new(&record.evidence_capture_intent_id),
        project_id: ProjectId::new(&record.project_id),
        task_id: TaskId::new(&record.task_id),
        change_unit_id: ChangeUnitId::new(&record.change_unit_id),
        scope_revision: record.scope_revision,
        baseline_ref: BaselineRef::new(&record.baseline_ref),
        target,
        capture,
        input_sha256: record.input_sha256.clone(),
        expected_outcome,
        requested_by_actor_source,
        workspace_context,
        created_at,
        expires_at,
    })
}

pub(super) fn validate_capture_receipt_record(
    intent: &EvidenceCaptureIntent,
    receipt: &EvidenceCaptureReceiptRecord,
) -> CoreResult<PersistedEvidenceCaptureReceiptBody> {
    let corrupt = |column: &'static str| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_value(
            "evidence_capture_receipts",
            receipt.evidence_capture_receipt_id.clone(),
            column,
        ))
    };
    if receipt.safe_receipt_json.len() > MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES
        || receipt.metadata_json.len() > MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES
        || receipt.safe_receipt_json.len() as u64 != receipt.safe_receipt_size_bytes
        || format!("{:x}", Sha256::digest(receipt.safe_receipt_json.as_bytes()))
            != receipt.safe_receipt_sha256
    {
        return Err(corrupt("safe_receipt_json"));
    }
    let safe_value = serde_json::from_str::<Value>(&receipt.safe_receipt_json)
        .map_err(|_| corrupt("safe_receipt_json"))?;
    let body = serde_json::from_value::<PersistedEvidenceCaptureReceiptBody>(safe_value.clone())
        .map_err(|_| corrupt("safe_receipt_json"))?;
    let canonical_body = canonical_json_string(&body).map_err(|_| corrupt("safe_receipt_json"))?;
    let metadata = serde_json::from_str::<Value>(&receipt.metadata_json)
        .map_err(|_| corrupt("metadata_json"))?;
    let stored_expected = serde_json::from_str::<JsonObject>(&receipt.expected_outcome_json)
        .map_err(|_| corrupt("expected_outcome_json"))?;
    let stored_observed = serde_json::from_str::<JsonObject>(&receipt.observed_outcome_json)
        .map_err(|_| corrupt("observed_outcome_json"))?;
    let stored_source_refs = serde_json::from_str::<Vec<StateRecordRef>>(&receipt.source_refs_json)
        .map_err(|_| corrupt("source_refs_json"))?;
    let receipt_created_at =
        UtcTimestamp::parse(&receipt.created_at).map_err(|_| corrupt("created_at"))?;
    receipt_created_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| corrupt("created_at"))?;
    body.observed_at
        .ensure_canonical_rfc3339_representable()
        .map_err(|_| corrupt("observed_at"))?;
    let producer_kind = parse_owner_storage_value::<EvidenceProducerKind>(
        "evidence_capture_receipts",
        receipt.evidence_capture_receipt_id.clone(),
        "capture_kind",
        &receipt.capture_kind,
    )?;
    let intent_producer_kind = capture_spec_producer_kind(&intent.capture);
    validate_evidence_capture_expected_outcome(&intent.capture, &body.expected_outcome)
        .map_err(|_| corrupt("expected_outcome_json"))?;
    validate_evidence_capture_observed_outcome(&intent.capture, &body.observed_outcome)
        .map_err(|_| corrupt("observed_outcome_json"))?;
    validate_evidence_capture_limitations(&intent.capture, &body.limitations)
        .map_err(|_| corrupt("limitations_json"))?;
    let observed_outcome_sha256 =
        volicord_types::canonical::canonical_json_bare_sha256(&body.observed_outcome)?;
    let expected_metadata = serde_json::json!({"source": &body.source});
    if body.contract_id != volicord_types::schema::EVIDENCE_CAPTURE_RECEIPT_CONTRACT_ID
        || canonical_body != receipt.safe_receipt_json
        || !body.complete
        || body.redaction_state != RedactionState::Redacted
        || receipt.completeness != "complete"
        || body.capture_kind != producer_kind
        || body.capture_kind != intent_producer_kind
        || body.capture_intent_id != intent.capture_intent_id
        || body.input_sha256 != intent.input_sha256
        || body.input_sha256 != receipt.input_sha256
        || body.result_sha256 != receipt.result_sha256
        || body.result_sha256 != observed_outcome_sha256
        || body.expected_outcome != intent.expected_outcome
        || body.expected_outcome != stored_expected
        || body.observed_outcome != stored_observed
        || !stored_source_refs.is_empty()
        || receipt.source_refs_json != "[]"
        || body.observed_at.to_canonical_string() != receipt.observed_at
        || body.observed_at < intent.created_at
        || body.observed_at >= intent.expires_at
        || receipt_created_at < body.observed_at
        || receipt_created_at >= intent.expires_at
        || body.observed_by_actor_source.to_canonical_string() != receipt.observed_by_actor_source
        || metadata != expected_metadata
    {
        return Err(corrupt("safe_receipt_json"));
    }
    Ok(body)
}

pub(super) fn capture_outcome_matches_expected(
    receipt_id: &str,
    capture: &EvidenceCaptureSpec,
    expected: &JsonObject,
    observed: &JsonObject,
) -> CoreResult<bool> {
    evidence_capture_observed_outcome_matches_expected(capture, expected, observed).map_err(|_| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_value(
            "evidence_capture_receipts",
            receipt_id,
            "observed_outcome_json",
        ))
    })
}

pub(super) fn capture_spec_producer_kind(capture: &EvidenceCaptureSpec) -> EvidenceProducerKind {
    match capture {
        EvidenceCaptureSpec::VerifiedCommandExecution { .. } => {
            EvidenceProducerKind::VerifiedCommandExecution
        }
        EvidenceCaptureSpec::VerifiedToolInvocation { .. } => {
            EvidenceProducerKind::VerifiedToolInvocation
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn user_action_observation_resolution_authority(
    action_record: &EffectiveUserActionRecord,
    resolution_record: &UserActionResolutionRecord,
    project_id: &ProjectId,
    task_id: &TaskId,
    change_unit_id: &str,
    scope_revision: u64,
    baseline_ref: Option<&str>,
    target: &EvidenceTarget,
    output_artifact_refs: &[ArtifactRef],
) -> CoreResult<Option<UserActionObservationResolutionAuthority>> {
    let request = &action_record.request.request;
    let basis = &action_record.request.basis;
    let resolution = &resolution_record.resolution;
    let observed_by_actor_source = &resolution_record.resolved_by_actor_source;
    let UserActionResolutionBody::EvidenceObservation { observation } = resolution else {
        return Ok(None);
    };
    let coordinates = basis.coordinates();
    if action_record.status != UserActionStatus::Resolved
        || action_record.request.action_kind != UserActionKind::EvidenceObservation
        || request.body.action_kind() != UserActionKind::EvidenceObservation
        || action_record.request.project_id != project_id.as_str()
        || action_record.request.task_id != task_id.as_str()
        || coordinates
            .change_unit_id
            .as_ref()
            .map(ChangeUnitId::as_str)
            != Some(change_unit_id)
        || coordinates.scope_revision != scope_revision
        || coordinates.baseline_ref.as_ref().map(BaselineRef::as_str) != baseline_ref
        || !matches!(
            observation.relevance_status,
            EvidenceRelevanceStatus::Supported | EvidenceRelevanceStatus::Contradicted
        )
        || observed_by_actor_source != &ActorSource::LocalUser
        || observation.target != *target
        || !exact_artifact_ref_sets_match(&observation.output_artifact_refs, output_artifact_refs)
    {
        return Ok(None);
    }
    Ok(Some(UserActionObservationResolutionAuthority {
        relevance_status: observation.relevance_status,
        resolved_at: resolution_record.resolved_at.clone(),
    }))
}

pub(super) fn stored_evidence_observation_provenance_facts(
    store: &CoreProjectStore,
    record: &EvidenceObservationRecord,
    basis: &EvidenceObservationBasis<'_>,
) -> CoreResult<EvidenceProvenanceFacts> {
    let basis_matches = stored_evidence_observation_matches_basis(store, record, basis)?;
    let source_kind: EvidenceSourceKind = parse_owner_storage_value(
        "evidence_observations",
        record.evidence_observation_id.clone(),
        "source_kind",
        &record.source_kind,
    )?;
    let assurance_level: EvidenceAssuranceLevel = parse_owner_storage_value(
        "evidence_observations",
        record.evidence_observation_id.clone(),
        "assurance_level",
        &record.assurance_level,
    )?;
    let binding_matches = if basis_matches
        && evidence_assurance_matches_source(source_kind, assurance_level)
        && !(source_kind == EvidenceSourceKind::AgentReport
            && assurance_level == EvidenceAssuranceLevel::CooperativeReport)
    {
        let mut visited = BTreeSet::new();
        stored_evidence_observation_anchored_assurance(store, record, basis, &mut visited)?
            .is_some()
    } else {
        false
    };
    Ok(EvidenceProvenanceFacts {
        basis_matches,
        source_kind,
        assurance_level,
        artifact_binding_matches: binding_matches,
        producer_binding_matches: binding_matches,
    })
}

pub(super) fn stored_evidence_observation_relevance(
    record: &EvidenceObservationRecord,
) -> CoreResult<EvidenceRelevanceAssessment> {
    let authority: PersistedEvidenceObservationAuthority = decode_required_json(
        "evidence_observations",
        record.evidence_observation_id.clone(),
        "metadata_json",
        Some(&record.metadata_json),
    )?;
    Ok(authority.relevance_assessment)
}

pub(super) fn stored_evidence_observation_capture_relevance(
    record: &EvidenceObservationRecord,
) -> CoreResult<Option<EvidenceRelevanceStatus>> {
    let authority: PersistedEvidenceObservationAuthority = decode_required_json(
        "evidence_observations",
        record.evidence_observation_id.clone(),
        "metadata_json",
        Some(&record.metadata_json),
    )?;
    Ok(matches!(
        authority.producer_anchor.producer_kind,
        EvidenceProducerKind::VerifiedToolInvocation
            | EvidenceProducerKind::VerifiedCommandExecution
    )
    .then_some(authority.relevance_assessment.status))
}

pub(super) fn projected_evidence_observation_provenance_facts(
    store: &CoreProjectStore,
    observation: &EvidenceObservation,
    basis: &EvidenceObservationBasis<'_>,
    projected_artifacts: &[ArtifactRef],
) -> CoreResult<EvidenceProvenanceFacts> {
    let basis_matches = projected_observation_matches_basis(observation, basis);
    let bindings_can_match = basis_matches
        && evidence_assurance_matches_source(observation.source_kind, observation.assurance_level)
        && !(observation.source_kind == EvidenceSourceKind::AgentReport
            && observation.assurance_level == EvidenceAssuranceLevel::CooperativeReport);
    if !bindings_can_match {
        return Ok(EvidenceProvenanceFacts {
            basis_matches,
            source_kind: observation.source_kind,
            assurance_level: observation.assurance_level,
            artifact_binding_matches: false,
            producer_binding_matches: false,
        });
    }
    let artifact_binding_matches = projected_observation_artifacts_are_current(
        store,
        basis,
        &observation.output_artifact_refs,
        projected_artifacts,
    )? && producer_output_binding_matches(
        &observation.producer_anchor,
        &observation.output_artifact_refs,
    );
    let producer_binding_matches = match (observation.source_kind, observation.assurance_level) {
        (EvidenceSourceKind::UserObservation, EvidenceAssuranceLevel::UserObserved) => {
            user_channel_authority_is_current(
                store,
                basis,
                &UserChannelObservationAuthorityView {
                    input_refs: &observation.input_refs,
                    output_artifact_refs: &observation.output_artifact_refs,
                    observed_by_actor_source: observation.observed_by_actor_source.as_ref(),
                    producer_anchor: &observation.producer_anchor,
                    relevance_assessment: &observation.relevance_assessment,
                    observed_at: &observation.observed_at,
                },
            )?
        }
        (EvidenceSourceKind::ReusedEvidence, assurance_level) => {
            let mut visited = BTreeSet::new();
            projected_reuse_authority_is_current(
                store,
                basis,
                observation,
                assurance_level,
                &mut visited,
            )?
        }
        (EvidenceSourceKind::ExternalTool, EvidenceAssuranceLevel::ExternalToolResult) => {
            projected_capture_binding_matches(
                observation,
                basis,
                capture_verification_basis(observation.producer_anchor.producer_kind),
            )
        }
        _ => false,
    };
    Ok(EvidenceProvenanceFacts {
        basis_matches,
        source_kind: observation.source_kind,
        assurance_level: observation.assurance_level,
        artifact_binding_matches,
        producer_binding_matches,
    })
}

fn stored_evidence_observation_anchored_assurance(
    store: &CoreProjectStore,
    record: &EvidenceObservationRecord,
    basis: &EvidenceObservationBasis<'_>,
    visited: &mut BTreeSet<String>,
) -> CoreResult<Option<EvidenceAssuranceLevel>> {
    if !visited.insert(record.evidence_observation_id.clone())
        || !stored_evidence_observation_matches_basis(store, record, basis)?
    {
        return Ok(None);
    }

    let source_kind: EvidenceSourceKind = parse_owner_storage_value(
        "evidence_observations",
        record.evidence_observation_id.clone(),
        "source_kind",
        &record.source_kind,
    )?;
    let assurance_level: EvidenceAssuranceLevel = parse_owner_storage_value(
        "evidence_observations",
        record.evidence_observation_id.clone(),
        "assurance_level",
        &record.assurance_level,
    )?;
    if !evidence_assurance_matches_source(source_kind, assurance_level) {
        return Ok(None);
    }

    let authority: PersistedEvidenceObservationAuthority = decode_required_json(
        "evidence_observations",
        record.evidence_observation_id.clone(),
        "metadata_json",
        Some(&record.metadata_json),
    )?;
    if record.run_id.as_deref() != Some(authority.recorded_by_run_id.as_str())
        || authority.invocation_verification_basis.trim().is_empty()
    {
        return Ok(None);
    }
    let input_refs: Vec<StateRecordRef> = decode_required_json(
        "evidence_observations",
        record.evidence_observation_id.clone(),
        "input_refs_json",
        Some(&record.input_refs_json),
    )?;
    let output_artifact_refs: Vec<ArtifactRef> = decode_required_json(
        "evidence_observations",
        record.evidence_observation_id.clone(),
        "output_artifact_refs_json",
        Some(&record.output_artifact_refs_json),
    )?;
    let observed_by_actor_source = record
        .observed_by_actor_source
        .as_deref()
        .map(|value| {
            parse_owner_storage_value(
                "evidence_observations",
                record.evidence_observation_id.clone(),
                "observed_by_actor_source",
                value,
            )
        })
        .transpose()?;
    let observed_at = UtcTimestamp::parse(&record.observed_at).map_err(|_| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_value(
            "evidence_observations",
            record.evidence_observation_id.clone(),
            "observed_at",
        ))
    })?;
    if !stored_observation_artifacts_are_current(store, record, basis, &output_artifact_refs)?
        || !producer_output_binding_matches(&authority.producer_anchor, &output_artifact_refs)
    {
        return Ok(None);
    }

    match (source_kind, assurance_level) {
        (EvidenceSourceKind::UserObservation, EvidenceAssuranceLevel::UserObserved) => {
            Ok(user_channel_authority_is_current(
                store,
                basis,
                &UserChannelObservationAuthorityView {
                    input_refs: &input_refs,
                    output_artifact_refs: &output_artifact_refs,
                    observed_by_actor_source: observed_by_actor_source.as_ref(),
                    producer_anchor: &authority.producer_anchor,
                    relevance_assessment: &authority.relevance_assessment,
                    observed_at: &observed_at,
                },
            )?
            .then_some(assurance_level))
        }
        (EvidenceSourceKind::ReusedEvidence, inherited_assurance) => {
            let [source_ref] = input_refs.as_slice() else {
                return Ok(None);
            };
            if source_ref.record_kind != StateRecordKind::EvidenceObservation
                || source_ref.project_id != *basis.project_id
                || source_ref.task_id.as_ref() != Some(basis.task_id)
                || source_ref.record_id.as_str() == record.evidence_observation_id
            {
                return Ok(None);
            }
            if authority.producer_anchor.producer_kind != EvidenceProducerKind::ReusedEvidence
                || authority.producer_anchor.verification_basis.as_deref()
                    != Some("core_validated_evidence_reuse")
                || authority.relevance_assessment.status != EvidenceRelevanceStatus::Supported
                || !authority_ref_matches(
                    authority.producer_anchor.producer_ref.as_ref(),
                    source_ref,
                )
                || !authority_ref_matches(
                    authority.relevance_assessment.assessment_ref.as_ref(),
                    source_ref,
                )
                || authority
                    .relevance_assessment
                    .assessed_by_actor_source
                    .is_some()
            {
                return Ok(None);
            }
            let Some(source_record) = store
                .evidence_observation_record(source_ref.record_id.as_str())
                .map_err(CorePipelineError::from)?
            else {
                return Ok(None);
            };
            let source_outputs: Vec<ArtifactRef> = decode_required_json(
                "evidence_observations",
                source_record.evidence_observation_id.clone(),
                "output_artifact_refs_json",
                Some(&source_record.output_artifact_refs_json),
            )?;
            if !exact_artifact_ref_sets_match(&source_outputs, &output_artifact_refs)
                || !relevance_supports_claim(&stored_evidence_observation_relevance(
                    &source_record,
                )?)
            {
                return Ok(None);
            }
            let inherited = stored_evidence_observation_anchored_assurance(
                store,
                &source_record,
                basis,
                visited,
            )?;
            Ok((inherited == Some(inherited_assurance)).then_some(inherited_assurance))
        }
        (EvidenceSourceKind::ExternalTool, EvidenceAssuranceLevel::ExternalToolResult) => {
            Ok(stored_capture_authority_is_current(
                store,
                record,
                basis,
                &input_refs,
                &output_artifact_refs,
                observed_by_actor_source.as_ref(),
                &authority.producer_anchor,
                &authority.relevance_assessment,
            )?
            .then_some(assurance_level))
        }
        _ => Ok(None),
    }
}

#[allow(clippy::too_many_arguments)]
fn stored_capture_authority_is_current(
    store: &CoreProjectStore,
    observation_record: &EvidenceObservationRecord,
    basis: &EvidenceObservationBasis<'_>,
    input_refs: &[StateRecordRef],
    output_artifact_refs: &[ArtifactRef],
    observed_by_actor_source: Option<&ActorSource>,
    producer_anchor: &EvidenceProducerAnchor,
    relevance_assessment: &EvidenceRelevanceAssessment,
) -> CoreResult<bool> {
    let Some(producer_ref) = producer_anchor.producer_ref.as_ref() else {
        return Ok(false);
    };
    let Some(intent_ref) = relevance_assessment.assessment_ref.as_ref() else {
        return Ok(false);
    };
    let capture_refs = input_refs
        .iter()
        .filter(|record_ref| record_ref.record_kind == StateRecordKind::EvidenceCaptureIntent)
        .collect::<Vec<_>>();
    if producer_ref.record_kind != StateRecordKind::EvidenceProducer
        || producer_ref.project_id != *basis.project_id
        || producer_ref.task_id.as_ref() != Some(basis.task_id)
        || intent_ref.record_kind != StateRecordKind::EvidenceCaptureIntent
        || intent_ref.project_id != *basis.project_id
        || intent_ref.task_id.as_ref() != Some(basis.task_id)
        || capture_refs.as_slice() != [intent_ref]
        || relevance_assessment.assessed_by_actor_source.is_some()
        || observed_by_actor_source
            .and_then(ActorSource::agent_connection_id)
            .is_none()
    {
        return Ok(false);
    }
    let Some(record) = store
        .evidence_producer_record(producer_ref.record_id.as_str())
        .map_err(CorePipelineError::from)?
    else {
        return Ok(false);
    };
    let producer: EvidenceProducer = serde_json::from_str(&record.canonical_producer_json)
        .map_err(|_| {
            CorePipelineError::Store(StoreError::corrupt_owner_state_value(
                "evidence_producers",
                record.evidence_producer_id.clone(),
                "canonical_producer_json",
            ))
        })?;
    let producer_metadata = serde_json::from_str::<Value>(&record.metadata_json).map_err(|_| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_value(
            "evidence_producers",
            record.evidence_producer_id.clone(),
            "metadata_json",
        ))
    })?;
    let record_producer_kind = parse_owner_storage_value::<EvidenceProducerKind>(
        "evidence_producers",
        record.evidence_producer_id.clone(),
        "producer_kind",
        &record.producer_kind,
    )?;
    let canonical_producer_json = canonical_json_string(&producer).map_err(|_| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_value(
            "evidence_producers",
            record.evidence_producer_id.clone(),
            "canonical_producer_json",
        ))
    })?;
    if canonical_producer_json != record.canonical_producer_json
        || record.project_id != basis.project_id.as_str()
        || producer.evidence_producer_id.as_str() != record.evidence_producer_id
        || producer.capture_intent_id.as_str() != record.evidence_capture_intent_id
        || producer.capture_receipt_id.as_str() != record.evidence_capture_receipt_id
        || producer.observation_ref.record_id.as_str() != record.evidence_observation_id
        || producer.run_ref.record_id.as_str() != record.run_id
        || observation_record.run_id.as_deref() != Some(record.run_id.as_str())
        || observation_record.project_id != record.project_id
        || observation_record.task_id != record.task_id
        || producer.task_id.as_str() != record.task_id
        || producer.change_unit_id.as_str() != record.change_unit_id
        || producer.scope_revision != record.scope_revision
        || producer.baseline_ref.as_str() != record.baseline_ref
        || producer.producer_kind != record_producer_kind
        || producer.finalized_at.to_canonical_string() != record.created_at
        || producer.project_id != *basis.project_id
        || producer.task_id != *basis.task_id
        || producer.change_unit_id.as_str() != basis.change_unit_id
        || producer.scope_revision != basis.scope_revision
        || basis
            .baseline_ref
            .is_some_and(|baseline| producer.baseline_ref.as_str() != baseline)
        || producer.target != *basis.target
        || producer.observation_ref.record_kind != StateRecordKind::EvidenceObservation
        || producer.observation_ref.record_id.as_str() != observation_record.evidence_observation_id
        || producer.observation_ref.project_id != *basis.project_id
        || producer.observation_ref.task_id.as_ref() != Some(basis.task_id)
        || producer.observation_ref.produced_at_state_version
            != producer_ref.produced_at_state_version
        || producer.run_ref.record_kind != StateRecordKind::Run
        || producer.run_ref.project_id != *basis.project_id
        || producer.run_ref.task_id.as_ref() != Some(basis.task_id)
        || producer.run_ref.produced_at_state_version != producer_ref.produced_at_state_version
        || producer.capture_intent_ref != *intent_ref
        || producer.receipt_artifact_refs.as_slice() != output_artifact_refs
        || producer.receipt_artifact_refs.len() != 1
        || producer.receipt_artifact_refs[0].artifact_id.as_str() != record.artifact_id
        || Some(&producer.observed_by_actor_source) != observed_by_actor_source
        || !producer.complete
        || producer.redaction_state != RedactionState::Redacted
        || producer.producer_kind != producer_anchor.producer_kind
        || producer_anchor.verification_basis.as_deref()
            != capture_verification_basis(producer.producer_kind)
        || producer_metadata
            != serde_json::json!({
                "verification_basis": capture_verification_basis(producer.producer_kind)
            })
    {
        return Ok(false);
    }
    let Some(intent_record) = store
        .evidence_capture_intent_record(&record.evidence_capture_intent_id)
        .map_err(CorePipelineError::from)?
    else {
        return Ok(false);
    };
    let intent = decode_capture_intent_record(&intent_record)?;
    let Some(receipt) = store
        .evidence_capture_receipt_for_intent(intent.capture_intent_id.as_str())
        .map_err(CorePipelineError::from)?
    else {
        return Ok(false);
    };
    let receipt_body = validate_capture_receipt_record(&intent, &receipt)?;
    store
        .validate_evidence_capture_source_claims_for_receipt(
            &intent_record,
            &receipt,
            &intent.capture,
            &receipt_body,
        )
        .map_err(CorePipelineError::from)?;
    let outcome_matches_expected = capture_outcome_matches_expected(
        &receipt.evidence_capture_receipt_id,
        &intent.capture,
        &receipt_body.expected_outcome,
        &receipt_body.observed_outcome,
    )?;
    let expected_relevance = capture_outcome_relevance(outcome_matches_expected);
    let receipt_source_refs =
        serde_json::from_str::<Vec<StateRecordRef>>(&receipt.source_refs_json).map_err(|_| {
            CorePipelineError::Store(StoreError::corrupt_owner_state_value(
                "evidence_capture_receipts",
                receipt.evidence_capture_receipt_id.clone(),
                "source_refs_json",
            ))
        })?;
    let expected_tool_name = match &intent.capture {
        EvidenceCaptureSpec::VerifiedCommandExecution { .. } => {
            Some("volicord.command_runner".to_owned())
        }
        EvidenceCaptureSpec::VerifiedToolInvocation { tool_name, .. } => Some(tool_name.clone()),
    };
    let expected_tool_invocation_id = receipt_body.source.host_invocation_id.as_ref().cloned();
    let expected_tool_metadata = object_from_value(serde_json::json!({
        "capture_intent_id": intent.capture_intent_id,
        "capture_receipt_id": receipt.evidence_capture_receipt_id,
        "result_sha256": receipt.result_sha256,
        "connection_id": receipt_body.source.connection_id,
        "host_invocation_id": receipt_body.source.host_invocation_id
    }))?;
    let expected_tool_metadata_json = canonical_json_string(&expected_tool_metadata)?;
    let expected_source_refs_json = canonical_json_string(&receipt_source_refs)?;
    let expected_limitations_json = canonical_json_string(&receipt_body.limitations)?;
    Ok(intent.capture_intent_id == producer.capture_intent_id
        && intent.project_id == *basis.project_id
        && intent.task_id == *basis.task_id
        && intent.change_unit_id.as_str() == basis.change_unit_id
        && intent.scope_revision == basis.scope_revision
        && intent.baseline_ref == producer.baseline_ref
        && intent.target == *basis.target
        && receipt.evidence_capture_receipt_id == producer.capture_receipt_id.as_str()
        && producer.input_sha256 == intent.input_sha256
        && producer.input_sha256 == receipt.input_sha256
        && receipt.result_sha256 == producer.result_sha256
        && producer.producer_kind == receipt_body.capture_kind
        && producer.producer_kind == capture_spec_producer_kind(&intent.capture)
        && producer.expected_outcome == intent.expected_outcome
        && producer.expected_outcome == receipt_body.expected_outcome
        && receipt_body.observed_outcome == producer.observed_outcome
        && producer.source_refs == receipt_source_refs
        && producer.connection_id == receipt_body.source.connection_id
        && producer.host_invocation_id.as_ref() == receipt_body.source.host_invocation_id.as_ref()
        && producer.limitations == receipt_body.limitations
        && producer.observed_at == receipt_body.observed_at
        && observation_record.tool_name == expected_tool_name
        && observation_record.tool_invocation_id == expected_tool_invocation_id
        && observation_record.tool_metadata_json == expected_tool_metadata_json
        && observation_record.source_refs_json == expected_source_refs_json
        && observation_record.limitations_json == expected_limitations_json
        && observation_record.observed_at == receipt_body.observed_at.to_canonical_string()
        && observation_record.recorded_at == producer.finalized_at.to_canonical_string()
        && relevance_assessment.status == expected_relevance
        && receipt_body.observed_by_actor_source == producer.observed_by_actor_source)
}

pub(super) fn capture_verification_basis(kind: EvidenceProducerKind) -> Option<&'static str> {
    match kind {
        EvidenceProducerKind::VerifiedCommandExecution => {
            Some(volicord_types::schema::EVIDENCE_CAPTURE_COMMAND_VERIFICATION_BASIS)
        }
        EvidenceProducerKind::VerifiedToolInvocation => {
            Some(volicord_types::schema::EVIDENCE_CAPTURE_TOOL_VERIFICATION_BASIS)
        }
        EvidenceProducerKind::UnverifiedCaller
        | EvidenceProducerKind::UserChannelObservation
        | EvidenceProducerKind::ReusedEvidence => None,
    }
}

fn stored_evidence_observation_matches_basis(
    store: &CoreProjectStore,
    record: &EvidenceObservationRecord,
    basis: &EvidenceObservationBasis<'_>,
) -> CoreResult<bool> {
    let source_run = record
        .run_id
        .as_deref()
        .map(|run_id| store.run_record(run_id))
        .transpose()
        .map_err(CorePipelineError::from)?
        .flatten();
    Ok(stored_observation_matches_basis(
        record,
        source_run.as_ref(),
        basis,
    ))
}

struct UserChannelObservationAuthorityView<'a> {
    input_refs: &'a [StateRecordRef],
    output_artifact_refs: &'a [ArtifactRef],
    observed_by_actor_source: Option<&'a ActorSource>,
    producer_anchor: &'a EvidenceProducerAnchor,
    relevance_assessment: &'a EvidenceRelevanceAssessment,
    observed_at: &'a UtcTimestamp,
}

fn user_channel_authority_is_current(
    store: &CoreProjectStore,
    basis: &EvidenceObservationBasis<'_>,
    authority: &UserChannelObservationAuthorityView<'_>,
) -> CoreResult<bool> {
    let UserChannelObservationAuthorityView {
        input_refs,
        output_artifact_refs,
        observed_by_actor_source,
        producer_anchor,
        relevance_assessment,
        observed_at,
    } = authority;
    let Some(producer_ref) = producer_anchor.producer_ref.as_ref() else {
        return Ok(false);
    };
    if producer_anchor.producer_kind != EvidenceProducerKind::UserChannelObservation
        || producer_ref.record_kind != StateRecordKind::UserActionResolution
        || producer_ref.project_id != *basis.project_id
        || producer_ref.task_id.as_ref() != Some(basis.task_id)
        || *observed_by_actor_source != Some(&ActorSource::LocalUser)
        || !matches!(
            relevance_assessment.status,
            EvidenceRelevanceStatus::Supported | EvidenceRelevanceStatus::Contradicted
        )
        || relevance_assessment.assessed_by_actor_source.as_ref() != Some(&ActorSource::LocalUser)
        || !authority_ref_matches(relevance_assessment.assessment_ref.as_ref(), producer_ref)
        || !input_refs
            .iter()
            .any(|input_ref| authority_ref_matches(Some(input_ref), producer_ref))
    {
        return Ok(false);
    }
    let Some(resolution_record) = store
        .user_action_resolution_record(producer_ref.record_id.as_str())
        .map_err(CorePipelineError::from)?
    else {
        return Ok(false);
    };
    let Some(action_record) = store
        .user_action_record(&resolution_record.user_action_request_id, basis.now)
        .map_err(CorePipelineError::from)?
    else {
        return Ok(false);
    };
    let Some(resolution_authority) = user_action_observation_resolution_authority(
        &action_record,
        &resolution_record,
        basis.project_id,
        basis.task_id,
        basis.change_unit_id,
        basis.scope_revision,
        basis.baseline_ref,
        basis.target,
        output_artifact_refs,
    )?
    else {
        return Ok(false);
    };
    Ok(producer_anchor.verification_basis.as_deref()
        == Some(resolution_record.resolved_verification_basis.as_str())
        && relevance_assessment.status == resolution_authority.relevance_status
        && **observed_at == resolution_authority.resolved_at)
}

fn stored_observation_artifacts_are_current(
    store: &CoreProjectStore,
    record: &EvidenceObservationRecord,
    basis: &EvidenceObservationBasis<'_>,
    artifact_refs: &[ArtifactRef],
) -> CoreResult<bool> {
    if artifact_refs.is_empty() {
        return Ok(false);
    }
    for artifact_ref in artifact_refs {
        if !persistent_artifact_ref_is_current(store, basis, artifact_ref)?
            || !store
                .artifact_has_owner_link(
                    artifact_ref.artifact_id.as_str(),
                    basis.task_id.as_str(),
                    "evidence_observation",
                    &record.evidence_observation_id,
                )
                .map_err(CorePipelineError::from)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn projected_observation_artifacts_are_current(
    store: &CoreProjectStore,
    basis: &EvidenceObservationBasis<'_>,
    artifact_refs: &[ArtifactRef],
    projected_artifacts: &[ArtifactRef],
) -> CoreResult<bool> {
    if artifact_refs.is_empty() {
        return Ok(false);
    }
    for artifact_ref in artifact_refs {
        if projected_artifacts
            .iter()
            .any(|projected| exact_artifact_identity_matches(projected, artifact_ref))
        {
            continue;
        }
        if !persistent_artifact_ref_is_current(store, basis, artifact_ref)?
            || !store
                .artifact_has_task_owner_link(
                    artifact_ref.artifact_id.as_str(),
                    basis.task_id.as_str(),
                )
                .map_err(CorePipelineError::from)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn persistent_artifact_ref_is_current(
    store: &CoreProjectStore,
    basis: &EvidenceObservationBasis<'_>,
    artifact_ref: &ArtifactRef,
) -> CoreResult<bool> {
    if artifact_ref.project_id != *basis.project_id
        || artifact_ref.task_id != *basis.task_id
        || artifact_ref.availability != ArtifactAvailability::Available
        || artifact_ref.integrity_status != ArtifactIntegrityStatus::Verified
    {
        return Ok(false);
    }
    let Some(record) = store
        .artifact_record(artifact_ref.artifact_id.as_str())
        .map_err(CorePipelineError::from)?
    else {
        return Ok(false);
    };
    if record.project_id != basis.project_id.as_str()
        || record.task_id != basis.task_id.as_str()
        || !persistent_artifact_is_verified_current(store, &record)?
    {
        return Ok(false);
    }
    let canonical = artifact_ref_from_verified_record(store, &record, None, None)?;
    Ok(exact_artifact_identity_matches(&canonical, artifact_ref))
}

fn projected_reuse_authority_is_current(
    store: &CoreProjectStore,
    basis: &EvidenceObservationBasis<'_>,
    observation: &EvidenceObservation,
    inherited_assurance: EvidenceAssuranceLevel,
    visited: &mut BTreeSet<String>,
) -> CoreResult<bool> {
    let [source_ref] = observation.input_refs.as_slice() else {
        return Ok(false);
    };
    if source_ref.record_kind != StateRecordKind::EvidenceObservation
        || source_ref.project_id != *basis.project_id
        || source_ref.task_id.as_ref() != Some(basis.task_id)
        || observation.producer_anchor.producer_kind != EvidenceProducerKind::ReusedEvidence
        || observation.producer_anchor.verification_basis.as_deref()
            != Some("core_validated_evidence_reuse")
        || observation.relevance_assessment.status != EvidenceRelevanceStatus::Supported
        || observation
            .relevance_assessment
            .assessed_by_actor_source
            .is_some()
        || !authority_ref_matches(
            observation.producer_anchor.producer_ref.as_ref(),
            source_ref,
        )
        || !authority_ref_matches(
            observation.relevance_assessment.assessment_ref.as_ref(),
            source_ref,
        )
    {
        return Ok(false);
    }
    let Some(source_record) = store
        .evidence_observation_record(source_ref.record_id.as_str())
        .map_err(CorePipelineError::from)?
    else {
        return Ok(false);
    };
    let source_outputs: Vec<ArtifactRef> = decode_required_json(
        "evidence_observations",
        source_record.evidence_observation_id.clone(),
        "output_artifact_refs_json",
        Some(&source_record.output_artifact_refs_json),
    )?;
    if !exact_artifact_ref_sets_match(&source_outputs, &observation.output_artifact_refs)
        || !relevance_supports_claim(&stored_evidence_observation_relevance(&source_record)?)
    {
        return Ok(false);
    }
    Ok(
        stored_evidence_observation_anchored_assurance(store, &source_record, basis, visited)?
            == Some(inherited_assurance),
    )
}

pub(super) fn load_required_evidence_criterion_ids(
    store: &CoreProjectStore,
    task_id: &TaskId,
) -> CoreResult<BTreeSet<String>> {
    Ok(store
        .active_acceptance_criteria(task_id)
        .map_err(CorePipelineError::from)?
        .into_iter()
        .map(|criterion| {
            let requirement: EvidenceRequirement = parse_owner_storage_value(
                "acceptance_criteria",
                criterion.acceptance_criterion_id.clone(),
                "evidence_requirement",
                &criterion.evidence_requirement,
            )?;
            Ok::<_, CorePipelineError>((criterion.acceptance_criterion_id, requirement))
        })
        .collect::<CoreResult<Vec<_>>>()?
        .into_iter()
        .filter_map(|(id, requirement)| {
            (requirement == EvidenceRequirement::Required).then_some(id)
        })
        .collect())
}

pub(super) fn load_close_evidence_summary_facts(
    store: &CoreProjectStore,
    record: Option<&EvidenceSummaryRecord>,
    task: &TaskRecord,
    project_id: &ProjectId,
    task_id: &TaskId,
    state_version: u64,
) -> CoreResult<CloseEvidenceSummaryFacts> {
    let updated_by_run_id = record
        .map(|record| {
            decode_required_json::<PersistedEvidenceMetadata>(
                "evidence_summaries",
                record.evidence_summary_id.clone(),
                "metadata_json",
                Some(&record.metadata_json),
            )
            .map(|metadata| metadata.updated_by_run_id)
        })
        .transpose()?;
    let updated_by_run = updated_by_run_id
        .as_ref()
        .map(|run_id| store.run_record(run_id.as_str()))
        .transpose()?
        .flatten()
        .map(|run| CloseEvidenceRunFacts {
            project_id: run.project_id,
            task_id: run.task_id,
            change_unit_id: run.change_unit_id,
            scope_revision: run.scope_revision,
        });
    let mut coverage_items = record
        .map(|record| {
            decode_required_json::<Vec<EvidenceCoverageItem>>(
                "evidence_summaries",
                record.evidence_summary_id.clone(),
                "coverage_json",
                Some(&record.coverage_json),
            )
        })
        .transpose()?
        .unwrap_or_default();
    if let Some(record) = record {
        let _supporting_refs: Vec<StateRecordRef> = decode_required_json(
            "evidence_summaries",
            record.evidence_summary_id.clone(),
            "supporting_refs_json",
            Some(&record.supporting_refs_json),
        )?;
        let _gap_refs: Vec<StateRecordRef> = decode_required_json(
            "evidence_summaries",
            record.evidence_summary_id.clone(),
            "gap_refs_json",
            Some(&record.gap_refs_json),
        )?;
    }
    for item in &mut coverage_items {
        item.supporting_artifact_refs = item
            .supporting_artifact_refs
            .iter()
            .map(|artifact_ref| {
                sanitize_evidence_artifact_ref(
                    store,
                    artifact_ref,
                    project_id,
                    task_id,
                    state_version,
                )
            })
            .collect::<CoreResult<Vec<_>>>()?;
    }
    let updated_by_run_ref = updated_by_run_id.as_ref().map(|updated_by_run_id| {
        state_ref(
            StateRecordKind::Run,
            updated_by_run_id.as_str(),
            project_id,
            Some(task_id),
            Some(state_version),
        )
    });
    Ok(CloseEvidenceSummaryFacts {
        task_project_id: task.project_id.clone(),
        task_id: task.task_id.clone(),
        task_change_unit_id: task.current_change_unit_id.clone(),
        task_scope_revision: task.scope_revision,
        summary_change_unit_id: record.and_then(|record| record.change_unit_id.clone()),
        updated_by_run_declared: updated_by_run_id.is_some(),
        updated_by_run,
        updated_by_run_ref,
        coverage_items,
    })
}

fn sanitize_evidence_artifact_ref(
    store: &CoreProjectStore,
    artifact_ref: &ArtifactRef,
    project_id: &ProjectId,
    task_id: &TaskId,
    state_version: u64,
) -> CoreResult<ArtifactRef> {
    if artifact_ref.project_id != *project_id || artifact_ref.task_id != *task_id {
        return Ok(unavailable_artifact_ref_from_raw(
            artifact_ref,
            ArtifactAvailability::Unusable,
        ));
    }
    let Some(record) = store.artifact_record(artifact_ref.artifact_id.as_str())? else {
        return Ok(unavailable_artifact_ref_from_raw(
            artifact_ref,
            ArtifactAvailability::Missing,
        ));
    };
    artifact_ref_from_verified_record(
        store,
        &record,
        Some(artifact_ref.display_name.clone()),
        Some(state_version),
    )
}

fn unavailable_artifact_ref_from_raw(
    artifact_ref: &ArtifactRef,
    availability: ArtifactAvailability,
) -> ArtifactRef {
    ArtifactRef {
        artifact_id: artifact_ref.artifact_id.clone(),
        project_id: artifact_ref.project_id.clone(),
        task_id: artifact_ref.task_id.clone(),
        display_name: artifact_ref.display_name.clone(),
        content_type: artifact_ref.content_type.clone(),
        sha256: artifact_ref.sha256.clone(),
        size_bytes: artifact_ref.size_bytes.clone(),
        integrity_status: artifact_ref.integrity_status,
        redaction_state: artifact_ref.redaction_state,
        availability,
        created_by_run_ref: artifact_ref.created_by_run_ref.clone(),
        created_by_actor_source: artifact_ref.created_by_actor_source.clone(),
        storage_ref: artifact_ref.storage_ref.clone(),
    }
}
