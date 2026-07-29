use super::{
    artifact_ref_from_verified_record, object_from_value, persistent_artifact_is_verified_current,
    state_ref,
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
use std::collections::BTreeSet;
use volicord_store::core_pipeline::{
    CoreProjectStore, EvidenceObservationRecord, EvidenceSummaryRecord, StoredUserActionRecordSet,
    StoredUserActionResolution, TaskRecord,
};
use volicord_store::evidence_capture::{
    EvidenceCaptureCompleteness, EvidenceCaptureIntentRecord, EvidenceCaptureReceiptRecord,
};
use volicord_types::ids::{BaselineRef, ChangeUnitId, EvidenceCaptureIntentId, ProjectId, TaskId};
use volicord_types::schema::{
    evidence_capture_observed_outcome_matches_expected, validate_evidence_capture_expected_outcome,
    validate_evidence_capture_limitations, validate_evidence_capture_observed_outcome, ArtifactRef,
    EvidenceCaptureIntent, EvidenceCaptureSpec, EvidenceObservation, EvidenceProducerAnchor,
    EvidenceRelevanceAssessment, EvidenceTarget, JsonObject, PersistedEvidenceCaptureReceiptBody,
    StateRecordRef, UserActionResolutionBody,
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

pub(super) fn capture_intent_from_record(
    record: &EvidenceCaptureIntentRecord,
) -> CoreResult<EvidenceCaptureIntent> {
    Ok(EvidenceCaptureIntent {
        capture_intent_id: EvidenceCaptureIntentId::new(&record.evidence_capture_intent_id),
        project_id: ProjectId::new(&record.project_id),
        task_id: TaskId::new(&record.task_id),
        change_unit_id: ChangeUnitId::new(&record.change_unit_id),
        scope_revision: record.scope_revision,
        baseline_ref: record.baseline_ref.clone(),
        target: record.target.clone(),
        capture: record.capture.clone(),
        input_sha256: record.input_sha256.clone(),
        expected_outcome: record.expected_outcome.clone(),
        requested_by_actor_source: record.requested_by_actor_source.clone(),
        workspace_context: record.workspace_context.clone(),
        created_at: record.created_at.clone(),
        expires_at: record.expires_at.clone(),
    })
}

pub(super) fn validate_capture_receipt_record(
    intent: &EvidenceCaptureIntent,
    receipt: &EvidenceCaptureReceiptRecord,
) -> CoreResult<PersistedEvidenceCaptureReceiptBody> {
    let contradiction = |field: &'static str| CorePipelineError::Invariant {
        detail: format!(
            "typed evidence receipt `{}` contradicts its capture intent at `{field}`",
            receipt.evidence_capture_receipt_id
        ),
    };
    let body = &receipt.safe_receipt;
    let intent_producer_kind = capture_spec_producer_kind(&intent.capture);
    validate_evidence_capture_expected_outcome(&intent.capture, &body.expected_outcome)
        .map_err(|_| contradiction("expected_outcome"))?;
    validate_evidence_capture_observed_outcome(&intent.capture, &body.observed_outcome)
        .map_err(|_| contradiction("observed_outcome"))?;
    validate_evidence_capture_limitations(&intent.capture, &body.limitations)
        .map_err(|_| contradiction("limitations"))?;
    let observed_outcome_sha256 =
        volicord_types::canonical::canonical_json_bare_sha256(&body.observed_outcome)?;
    if body.contract_id != volicord_types::schema::EVIDENCE_CAPTURE_RECEIPT_CONTRACT_ID
        || !body.complete
        || body.redaction_state != RedactionState::Redacted
        || receipt.completeness != EvidenceCaptureCompleteness::Complete
        || body.capture_kind != receipt.capture_kind
        || body.capture_kind != intent_producer_kind
        || body.capture_intent_id != intent.capture_intent_id
        || body.input_sha256 != intent.input_sha256
        || body.input_sha256 != receipt.input_sha256
        || body.result_sha256 != receipt.result_sha256
        || body.result_sha256 != observed_outcome_sha256
        || body.expected_outcome != intent.expected_outcome
        || body.expected_outcome != receipt.expected_outcome
        || body.observed_outcome != receipt.observed_outcome
        || !receipt.source_refs.is_empty()
        || body.observed_at != receipt.observed_at
        || body.observed_at < intent.created_at
        || body.observed_at >= intent.expires_at
        || receipt.created_at < body.observed_at
        || receipt.created_at >= intent.expires_at
        || body.observed_by_actor_source != receipt.observed_by_actor_source
        || receipt.metadata.source != body.source
    {
        return Err(contradiction("safe_receipt"));
    }
    Ok(body.clone())
}

pub(super) fn capture_outcome_matches_expected(
    receipt_id: &str,
    capture: &EvidenceCaptureSpec,
    expected: &JsonObject,
    observed: &JsonObject,
) -> CoreResult<bool> {
    evidence_capture_observed_outcome_matches_expected(capture, expected, observed).map_err(|_| {
        CorePipelineError::Invariant {
            detail: format!(
                "typed evidence receipt `{receipt_id}` has an invalid observed-outcome shape"
            ),
        }
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
    action_record: &StoredUserActionRecordSet,
    resolution_record: &StoredUserActionResolution,
    project_id: &ProjectId,
    task_id: &TaskId,
    change_unit_id: &str,
    scope_revision: u64,
    baseline_ref: Option<&str>,
    target: &EvidenceTarget,
    output_artifact_refs: &[ArtifactRef],
) -> CoreResult<Option<UserActionObservationResolutionAuthority>> {
    let request = action_record.request().request();
    let basis = action_record.request().basis();
    let resolution = resolution_record.resolution();
    let observed_by_actor_source = resolution_record.resolved_by_actor_source();
    let UserActionResolutionBody::EvidenceObservation { observation } = resolution else {
        return Ok(None);
    };
    let coordinates = basis.coordinates();
    if action_record.status() != UserActionStatus::Resolved
        || action_record.request().action_kind() != UserActionKind::EvidenceObservation
        || request.body.action_kind() != UserActionKind::EvidenceObservation
        || action_record.request().project_id() != project_id.as_str()
        || action_record.request().task_id() != task_id.as_str()
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
        resolved_at: resolution_record.resolved_at().clone(),
    }))
}

pub(super) fn stored_evidence_observation_provenance_facts(
    store: &CoreProjectStore,
    record: &EvidenceObservationRecord,
    basis: &EvidenceObservationBasis<'_>,
) -> CoreResult<EvidenceProvenanceFacts> {
    let basis_matches = stored_evidence_observation_matches_basis(store, record, basis)?;
    let source_kind = record.source_kind;
    let assurance_level = record.assurance_level;
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
    Ok(record.metadata.relevance_assessment.clone())
}

pub(super) fn stored_evidence_observation_capture_relevance(
    record: &EvidenceObservationRecord,
) -> CoreResult<Option<EvidenceRelevanceStatus>> {
    Ok(matches!(
        record.metadata.producer_anchor.producer_kind,
        EvidenceProducerKind::VerifiedToolInvocation
            | EvidenceProducerKind::VerifiedCommandExecution
    )
    .then_some(record.metadata.relevance_assessment.status))
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

    let source_kind = record.source_kind;
    let assurance_level = record.assurance_level;
    if !evidence_assurance_matches_source(source_kind, assurance_level) {
        return Ok(None);
    }

    let authority = &record.metadata;
    if record.run_id.as_deref() != Some(authority.recorded_by_run_id.as_str())
        || authority.invocation_verification_basis.trim().is_empty()
    {
        return Ok(None);
    }
    let input_refs = &record.input_refs;
    let output_artifact_refs = &record.output_artifact_refs;
    let observed_by_actor_source = record.observed_by_actor_source.as_ref();
    let observed_at = &record.observed_at;
    if !stored_observation_artifacts_are_current(store, record, basis, output_artifact_refs)?
        || !producer_output_binding_matches(&authority.producer_anchor, output_artifact_refs)
    {
        return Ok(None);
    }

    match (source_kind, assurance_level) {
        (EvidenceSourceKind::UserObservation, EvidenceAssuranceLevel::UserObserved) => {
            Ok(user_channel_authority_is_current(
                store,
                basis,
                &UserChannelObservationAuthorityView {
                    input_refs,
                    output_artifact_refs,
                    observed_by_actor_source,
                    producer_anchor: &authority.producer_anchor,
                    relevance_assessment: &authority.relevance_assessment,
                    observed_at,
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
            if !exact_artifact_ref_sets_match(
                &source_record.output_artifact_refs,
                output_artifact_refs,
            ) || !relevance_supports_claim(&stored_evidence_observation_relevance(
                &source_record,
            )?) {
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
                input_refs,
                output_artifact_refs,
                observed_by_actor_source,
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
    let producer = &record.canonical_producer;
    if record.project_id != basis.project_id.as_str()
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
        || producer.baseline_ref != record.baseline_ref
        || producer.producer_kind != record.producer_kind
        || producer.finalized_at != record.created_at
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
        || Some(record.metadata.verification_basis.as_str())
            != capture_verification_basis(producer.producer_kind)
    {
        return Ok(false);
    }
    let Some(intent_record) = store
        .evidence_capture_intent_record(&record.evidence_capture_intent_id)
        .map_err(CorePipelineError::from)?
    else {
        return Ok(false);
    };
    let intent = capture_intent_from_record(&intent_record)?;
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
    let receipt_source_refs = receipt.source_refs.clone();
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
        && observation_record.tool_metadata == expected_tool_metadata
        && observation_record.source_refs.is_empty()
        && observation_record.limitations == receipt_body.limitations
        && observation_record.observed_at == receipt_body.observed_at
        && observation_record.recorded_at == producer.finalized_at
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
        .user_action_record(resolution_record.user_action_request_id(), basis.now)
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
        == Some(resolution_record.resolved_verification_basis().as_str())
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
                    StateRecordKind::EvidenceObservation,
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
    if !exact_artifact_ref_sets_match(
        &source_record.output_artifact_refs,
        &observation.output_artifact_refs,
    ) || !relevance_supports_claim(&stored_evidence_observation_relevance(&source_record)?)
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
            let requirement = criterion.evidence_requirement;
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
    let updated_by_run_id = record.map(|record| record.metadata.updated_by_run_id.clone());
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
        .map(|record| record.coverage.clone())
        .unwrap_or_default();
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
