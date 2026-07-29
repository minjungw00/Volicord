use crate::artifact::{artifact_ref_from_verified_record, normalize_source_refs};
use crate::evidence_facts::{
    stored_evidence_observation_provenance_facts, stored_evidence_observation_relevance,
    user_action_observation_resolution_authority,
};
use crate::identity::{allocate_evidence_observation_id, allocate_evidence_producer_id};
use crate::json_object::object_from_value;
use crate::pipeline::{CorePipelineError, CoreResult, CoreService, VerifiedInvocationContext};
use crate::policy::evidence::{
    evidence_status_for_items, state_record_ref_identity_key, unique_artifact_refs,
    unique_state_record_refs,
};
use crate::policy::{
    evidence_provenance::{
        classify_evidence_provenance, evidence_assurance_matches_source, EvidenceProvenanceClass,
    },
    evidence_relevance::relevance_supports_claim,
    evidence_target::{
        acceptance_criterion_target_is_current, run_record_matches_close_basis_context,
        stored_observation_target_matches, supplemental_claim_target_matches,
        EvidenceObservationBasis,
    },
};
use crate::record_refs::state_ref;
use crate::recording::{recording_validation_error, RecordingError};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use volicord_store::core_pipeline::{
    CoreProjectStore, EvidenceClaimInsert, EvidenceMutation, EvidenceObservationInsert,
    ProjectStateHeader, RunStatus,
};
use volicord_store::evidence_capture::{EvidenceProducerInsert, StoredEvidenceProducerMetadata};
use volicord_types::ids::{EvidenceCaptureReceiptId, EvidenceProducerId, RunId};
use volicord_types::methods::RecordRunRequest;
use volicord_types::schema::{
    ArtifactRef, EvidenceCoverageItem, EvidenceCoverageUpdate, EvidenceObservation,
    EvidenceObservationInput, EvidenceProducer, EvidenceProducerAnchor,
    EvidenceRelevanceAssessment, EvidenceTarget, EvidenceUpdateProvenance, JsonObject,
    PersistedEvidenceObservationAuthority, StateRecordRef,
};
use volicord_types::values::{
    ActorSource, ArtifactAvailability, ArtifactIntegrityStatus, EvidenceAssuranceLevel,
    EvidenceCoverageState, EvidenceCoverageUpdateState, EvidenceDisplayState, EvidenceProducerKind,
    EvidenceRelevanceStatus, EvidenceRequirement, EvidenceSourceKind, RedactionState,
    StateRecordKind, UtcTimestamp,
};

use super::{
    authority::{capture_authority_error, capture_authority_rejection},
    model::{
        RecordRunArtifactPlan, RecordRunCaptureAuthority, RecordRunEvidenceTargetPlan,
        RecordRunObservationOrigin, RecordRunObservationPlan,
    },
};
use crate::task_state::{normalize_display_string_list, normalize_display_text};

pub(super) fn normalize_record_run_evidence_targets(request: &mut RecordRunRequest) {
    for update in &mut request.evidence_updates {
        normalize_evidence_target(&mut update.target);
    }
    for observation in &mut request.evidence_observations {
        normalize_evidence_target(&mut observation.target);
    }
    for artifact in &mut request.artifact_inputs {
        if let Some(target) = artifact.evidence_target.as_mut() {
            normalize_evidence_target(target);
        }
    }
}

pub(super) fn normalize_evidence_target(target: &mut EvidenceTarget) {
    if let EvidenceTarget::SupplementalClaim { statement, .. } = target {
        *statement = normalize_display_text(statement);
    }
}

pub(super) fn plan_record_run_evidence_targets(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordRunRequest,
) -> Result<RecordRunEvidenceTargetPlan, RecordingError> {
    let mut supplemental = BTreeMap::<String, String>::new();
    let mut validate_target = |target: &EvidenceTarget, field: &'static str| {
        match target {
            EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id,
            } => {
                if acceptance_criterion_id.as_str().trim().is_empty() {
                    return recording_validation_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        field,
                        "acceptance criterion evidence target ID must not be empty",
                    );
                }
                let record = store
                    .acceptance_criterion_record(acceptance_criterion_id.as_str())
                    .map_err(CorePipelineError::from)?;
                let Some(record) = record else {
                    return recording_validation_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        field,
                        "acceptance criterion evidence target is unknown",
                    );
                };
                if !acceptance_criterion_target_is_current(Some(&record), &request.task_id) {
                    return recording_validation_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        field,
                        "acceptance criterion evidence target must be current for this Task",
                    );
                }
            }
            EvidenceTarget::SupplementalClaim {
                evidence_claim_id,
                statement,
            } => {
                if evidence_claim_id.as_str().trim().is_empty() || statement.is_empty() {
                    return recording_validation_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        field,
                        "supplemental evidence targets require a non-empty ID and statement",
                    );
                }
                if let Some(existing) =
                    supplemental.insert(evidence_claim_id.as_str().to_owned(), statement.clone())
                {
                    if existing != *statement {
                        return recording_validation_error(
                            request.envelope.dry_run,
                            Some(project_state.state_version),
                            field,
                            "one supplemental evidence claim ID cannot use multiple statements",
                        );
                    }
                }
            }
        }
        Ok(())
    };

    for update in &request.evidence_updates {
        validate_target(&update.target, "evidence_updates[].target")?;
        if update.coverage_state == EvidenceCoverageUpdateState::NotApplicable {
            if let EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id,
            } = &update.target
            {
                let record = store
                    .acceptance_criterion_record(acceptance_criterion_id.as_str())
                    .map_err(CorePipelineError::from)?
                    .expect("target validation ensures the criterion exists");
                let requirement = record.evidence_requirement;
                if requirement == EvidenceRequirement::Required {
                    return recording_validation_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        "evidence_updates[].coverage_state",
                        "required acceptance criteria cannot be marked not_applicable",
                    );
                }
            }
        }
    }
    for observation in &request.evidence_observations {
        validate_target(&observation.target, "evidence_observations[].target")?;
    }
    for artifact in &request.artifact_inputs {
        if let Some(target) = artifact.evidence_target.as_ref() {
            validate_target(target, "artifact_inputs[].evidence_target")?;
        }
    }

    let mut mutations = Vec::new();
    for (evidence_claim_id, statement) in supplemental {
        match store
            .evidence_claim_record(&request.task_id, &evidence_claim_id)
            .map_err(CorePipelineError::from)?
        {
            Some(record) if supplemental_claim_target_matches(Some(&record), &statement) => {}
            Some(_) => {
                return recording_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "evidence_target.statement",
                    "supplemental evidence claim statements are immutable within a Task",
                );
            }
            None => mutations.push(EvidenceMutation::EnsureClaim(EvidenceClaimInsert {
                evidence_claim_id,
                task_id: request.task_id.as_str().to_owned(),
                statement,
            })),
        }
    }
    Ok(RecordRunEvidenceTargetPlan {
        claim_mutations: mutations,
    })
}

pub(super) fn capture_authority_for_input<'a>(
    context: &'a RecordRunObservationContext<'_>,
    input: &EvidenceObservationInput,
) -> Result<Option<&'a RecordRunCaptureAuthority>, RecordingError> {
    let refs = input
        .input_refs
        .iter()
        .filter(|record_ref| record_ref.record_kind == StateRecordKind::EvidenceCaptureIntent)
        .collect::<Vec<_>>();
    let Some(intent_ref) = refs.first() else {
        return Ok(None);
    };
    if refs.len() != 1
        || intent_ref.project_id != context.request.envelope.project_id
        || intent_ref.task_id.as_ref() != Some(&context.request.task_id)
    {
        return capture_authority_error(
            context.request,
            context.project_state,
            "evidence-capture intent ref does not match the request project and Task",
        );
    }
    let capture = context
        .capture_authorities
        .get(intent_ref.record_id.as_str())
        .ok_or_else(|| {
            capture_authority_rejection(
                context.request,
                context.project_state,
                "evidence-capture intent authority was not prepared for this observation",
            )
        })?;
    let claimed_pair_matches = match capture.producer_kind {
        EvidenceProducerKind::VerifiedCommandExecution
        | EvidenceProducerKind::VerifiedToolInvocation => {
            input.source_kind == EvidenceSourceKind::ExternalTool
                && input.assurance_level == EvidenceAssuranceLevel::ExternalToolResult
        }
        EvidenceProducerKind::UnverifiedCaller
        | EvidenceProducerKind::UserChannelObservation
        | EvidenceProducerKind::ReusedEvidence => false,
    };
    if input.target != capture.intent.target {
        return capture_authority_error(
            context.request,
            context.project_state,
            "evidence-capture observation target does not match the immutable intent",
        );
    }
    if !claimed_pair_matches {
        return capture_authority_error(
            context.request,
            context.project_state,
            "evidence-capture observation source and assurance do not match the producer kind",
        );
    }
    let populated = if input.observed_by_actor_source.is_some() {
        Some("observed_by_actor_source")
    } else if input.tool_name.is_some() {
        Some("tool_name")
    } else if input.tool_invocation_id.is_some() {
        Some("tool_invocation_id")
    } else if !input.tool_metadata.is_empty() {
        Some("tool_metadata")
    } else if !input.limitations.is_empty() {
        Some("limitations")
    } else {
        None
    };
    if let Some(populated) = populated {
        return capture_authority_error(
            context.request,
            context.project_state,
            match populated {
                "observed_by_actor_source" => {
                    "evidence-capture observation must leave observed_by_actor_source null"
                }
                "tool_name" => "evidence-capture observation must leave tool_name null",
                "tool_invocation_id" => {
                    "evidence-capture observation must leave tool_invocation_id null"
                }
                "tool_metadata" => "evidence-capture observation must leave tool_metadata empty",
                _ => "evidence-capture observation must leave limitations empty",
            },
        );
    }
    Ok(Some(capture))
}

pub(super) struct RecordRunObservationContext<'a> {
    pub(super) service: &'a CoreService,
    pub(super) store: &'a CoreProjectStore<'a>,
    pub(super) project_state: &'a ProjectStateHeader,
    pub(super) request: &'a RecordRunRequest,
    pub(super) verified_invocation: &'a VerifiedInvocationContext,
    pub(super) run_id: &'a RunId,
    pub(super) run_ref: &'a StateRecordRef,
    pub(super) registered_artifacts: &'a [ArtifactRef],
    pub(super) artifact_plans: &'a [RecordRunArtifactPlan],
    pub(super) capture_authorities: &'a BTreeMap<String, RecordRunCaptureAuthority>,
    pub(super) current_scope_revision: u64,
    pub(super) planned_state_version: u64,
    pub(super) now: &'a UtcTimestamp,
}

pub(super) fn plan_record_run_observations(
    context: &RecordRunObservationContext<'_>,
) -> Result<Vec<RecordRunObservationPlan>, RecordingError> {
    let mut plans = Vec::new();
    for input in &context.request.evidence_observations {
        plans.push(plan_record_run_observation(
            context,
            input,
            RecordRunObservationOrigin::Caller,
        )?);
    }
    let explicit_observation_targets = plans
        .iter()
        .map(|plan| plan.observation.target.clone())
        .collect::<BTreeSet<_>>();
    for update in &context.request.evidence_updates {
        validate_record_run_evidence_update(context, update, &explicit_observation_targets)?;
        if update.coverage_state == EvidenceCoverageUpdateState::Supported
            && !explicit_observation_targets.contains(&update.target)
        {
            if let Some(provenance) = update.provenance.as_ref() {
                plans.push(plan_record_run_observation(
                    context,
                    &observation_input_from_evidence_update(context, update, provenance),
                    RecordRunObservationOrigin::Caller,
                )?);
            } else {
                for input in reused_observation_inputs_for_update(context, update)? {
                    plans.push(plan_record_run_observation(
                        context,
                        &input,
                        RecordRunObservationOrigin::ValidatedReuse,
                    )?);
                }
            }
        }
    }
    Ok(plans)
}

pub(super) fn plan_record_run_observation(
    context: &RecordRunObservationContext<'_>,
    input: &EvidenceObservationInput,
    origin: RecordRunObservationOrigin,
) -> Result<RecordRunObservationPlan, RecordingError> {
    validate_evidence_source_assurance(
        context.request.envelope.dry_run,
        Some(context.project_state.state_version),
        "evidence_observations[]",
        input.source_kind,
        input.assurance_level,
    )?;
    validate_evidence_observation_state_refs(
        context,
        "evidence_observations[].input_refs",
        &input.input_refs,
    )?;
    let capture_authority = capture_authority_for_input(context, input)?;
    let source_refs = if capture_authority.is_some() {
        if !input.source_refs.is_empty() || !input.output_artifact_refs.is_empty() {
            return capture_authority_error(
                context.request,
                context.project_state,
                "caller source or output refs cannot replace an evidence-capture receipt",
            );
        }
        Vec::new()
    } else {
        normalize_source_refs(
            context.store,
            context.project_state,
            &context.request.envelope,
            &context.request.task_id,
            "evidence_observations[].source_refs",
            &input.source_refs,
        )
        .map_err(RecordingError::Artifact)?
    };
    let canonical_output_artifact_refs = if let Some(capture) = capture_authority {
        vec![capture.receipt_artifact_ref.clone()]
    } else {
        canonical_evidence_artifact_refs(
            context,
            "evidence_observations[].output_artifact_refs",
            &input.output_artifact_refs,
        )?
    };
    let mut canonical_input = input.clone();
    canonical_input.output_artifact_refs = canonical_output_artifact_refs;
    if let Some(capture) = capture_authority {
        canonical_input.source_kind = capture.source_kind;
        canonical_input.assurance_level = capture.assurance_level;
        canonical_input.observed_by_actor_source =
            Some(capture.observed_by_actor_source.clone()).into();
        canonical_input.tool_name = capture.tool_name.clone().into();
        canonical_input.tool_invocation_id = capture
            .host_invocation_id
            .clone()
            .or_else(|| {
                (capture.producer_kind == EvidenceProducerKind::VerifiedCommandExecution)
                    .then(|| capture.receipt.evidence_capture_receipt_id.clone())
            })
            .into();
        canonical_input.tool_metadata = object_from_value(json!({
            "capture_intent_id": capture.intent.capture_intent_id,
            "capture_receipt_id": capture.receipt.evidence_capture_receipt_id,
            "result_sha256": capture.receipt.result_sha256,
            "connection_id": capture.connection_id,
            "host_invocation_id": capture.host_invocation_id
        }))?;
        canonical_input.source_refs.clear();
        canonical_input.limitations = capture.limitations.clone();
        canonical_input.observed_at = capture.observed_at.clone();
    }
    let input = &canonical_input;
    if input
        .tool_name
        .as_ref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return recording_validation_error(
            context.request.envelope.dry_run,
            Some(context.project_state.state_version),
            "evidence_observations[].tool_name",
            "tool_name must be null or a non-empty string",
        );
    }
    if input
        .tool_invocation_id
        .as_ref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return recording_validation_error(
            context.request.envelope.dry_run,
            Some(context.project_state.state_version),
            "evidence_observations[].tool_invocation_id",
            "tool_invocation_id must be null or a non-empty string",
        );
    }

    let observation_id =
        allocate_evidence_observation_id(context.service.durable_id_generator(), context.store)
            .map_err(RecordingError::Core)?;
    let observation_ref = state_ref(
        StateRecordKind::EvidenceObservation,
        observation_id.as_str(),
        &context.request.envelope.project_id,
        Some(&context.request.task_id),
        Some(context.planned_state_version),
    );
    let authority_bound_outputs = matches!(
        input.source_kind,
        EvidenceSourceKind::UserObservation | EvidenceSourceKind::ReusedEvidence
    ) || capture_authority.is_some();
    let output_artifact_refs =
        if origin == RecordRunObservationOrigin::ValidatedReuse || authority_bound_outputs {
            input.output_artifact_refs.clone()
        } else {
            output_artifact_refs_for_observation(context, input)
        };
    let output_artifact_refs = unique_artifact_refs(output_artifact_refs);
    let authority =
        derive_record_run_observation_authority(context, input, &output_artifact_refs, origin)?;
    let limitations = normalize_display_string_list(&input.limitations);
    let observation = EvidenceObservation {
        observation_id,
        project_id: context.request.envelope.project_id.clone(),
        task_id: context.request.task_id.clone(),
        change_unit_id: Some(context.request.change_unit_id.clone()).into(),
        run_ref: Some(context.run_ref.clone()).into(),
        target: input.target.clone(),
        source_kind: authority.source_kind,
        assurance_level: authority.assurance_level,
        producer_anchor: authority.producer_anchor.clone(),
        relevance_assessment: authority.relevance_assessment.clone(),
        observed_by_actor_source: authority.observed_by_actor_source.clone().into(),
        tool_name: input.tool_name.clone(),
        tool_invocation_id: input.tool_invocation_id.clone(),
        tool_metadata: input.tool_metadata.clone(),
        input_refs: input.input_refs.clone(),
        source_refs,
        output_artifact_refs,
        limitations,
        observed_at: authority
            .observed_at
            .clone()
            .unwrap_or_else(|| input.observed_at.clone()),
        recorded_at: context.now.clone(),
    };
    let mutation = EvidenceMutation::InsertObservation(EvidenceObservationInsert {
        evidence_observation_id: observation.observation_id.as_str().to_owned(),
        task_id: observation.task_id.as_str().to_owned(),
        change_unit_id: observation
            .change_unit_id
            .as_ref()
            .map(|id| id.as_str().to_owned()),
        run_id: Some(context.run_id.as_str().to_owned()),
        acceptance_criterion_id: match &observation.target {
            EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id,
            } => Some(acceptance_criterion_id.as_str().to_owned()),
            EvidenceTarget::SupplementalClaim { .. } => None,
        },
        evidence_claim_id: match &observation.target {
            EvidenceTarget::SupplementalClaim {
                evidence_claim_id, ..
            } => Some(evidence_claim_id.as_str().to_owned()),
            EvidenceTarget::AcceptanceCriterion { .. } => None,
        },
        source_kind: observation.source_kind,
        assurance_level: observation.assurance_level,
        observed_by_actor_source: observation.observed_by_actor_source.clone().into_option(),
        tool_name: observation.tool_name.as_ref().cloned(),
        tool_invocation_id: observation.tool_invocation_id.as_ref().cloned(),
        tool_metadata: observation.tool_metadata.clone(),
        input_refs: observation.input_refs.clone(),
        source_refs: observation.source_refs.clone(),
        output_artifact_refs: observation.output_artifact_refs.clone(),
        limitations: observation.limitations.clone(),
        observed_at: observation.observed_at.clone(),
        recorded_at: observation.recorded_at.clone(),
        metadata: PersistedEvidenceObservationAuthority {
            recorded_by_run_id: context.run_id.clone(),
            invocation_verification_basis: context.verified_invocation.verification_basis.clone(),
            producer_anchor: authority.producer_anchor.clone(),
            relevance_assessment: authority.relevance_assessment.clone(),
        },
    });
    let (producer, producer_mutation) = if let (Some(capture), Some(producer_id)) =
        (capture_authority, authority.producer_id.clone())
    {
        let producer = EvidenceProducer {
            evidence_producer_id: producer_id.clone(),
            capture_receipt_id: EvidenceCaptureReceiptId::new(
                &capture.receipt.evidence_capture_receipt_id,
            ),
            capture_intent_id: capture.intent.capture_intent_id.clone(),
            capture_intent_ref: capture.intent_ref.clone(),
            producer_kind: capture.producer_kind,
            project_id: context.request.envelope.project_id.clone(),
            task_id: context.request.task_id.clone(),
            change_unit_id: context.request.change_unit_id.clone(),
            scope_revision: context.current_scope_revision,
            baseline_ref: context.request.baseline_ref.clone(),
            target: capture.intent.target.clone(),
            input_sha256: capture.intent.input_sha256.clone(),
            result_sha256: capture.receipt.result_sha256.clone(),
            expected_outcome: capture.intent.expected_outcome.clone(),
            observed_outcome: capture.observed_outcome.clone(),
            source_refs: capture.source_refs.clone(),
            connection_id: capture.connection_id.clone(),
            host_invocation_id: capture.host_invocation_id.clone().into(),
            receipt_artifact_refs: vec![capture.receipt_artifact_ref.clone()],
            complete: true,
            limitations: capture.limitations.clone(),
            redaction_state: RedactionState::Redacted,
            observed_by_actor_source: capture.observed_by_actor_source.clone(),
            observed_at: capture.observed_at.clone(),
            finalized_at: context.now.clone(),
            run_ref: context.run_ref.clone(),
            observation_ref: observation_ref.clone(),
        };
        let mutation = EvidenceMutation::InsertProducer(EvidenceProducerInsert {
            evidence_producer_id: producer_id.as_str().to_owned(),
            evidence_capture_intent_id: capture.intent.capture_intent_id.as_str().to_owned(),
            evidence_capture_receipt_id: capture.receipt.evidence_capture_receipt_id.clone(),
            evidence_observation_id: observation.observation_id.as_str().to_owned(),
            artifact_id: capture.receipt_artifact_ref.artifact_id.as_str().to_owned(),
            run_id: context.run_id.as_str().to_owned(),
            task_id: context.request.task_id.as_str().to_owned(),
            change_unit_id: context.request.change_unit_id.as_str().to_owned(),
            scope_revision: context.current_scope_revision,
            baseline_ref: context.request.baseline_ref.clone(),
            producer_kind: capture.producer_kind,
            canonical_producer: producer.clone(),
            created_at: context.now.clone(),
            metadata: StoredEvidenceProducerMetadata {
                verification_basis: capture.verification_basis.clone(),
            },
        });
        (Some(producer), Some(mutation))
    } else {
        (None, None)
    };
    Ok(RecordRunObservationPlan {
        observation,
        observation_ref,
        mutation,
        producer,
        producer_mutation,
    })
}

struct DerivedObservationAuthority {
    source_kind: EvidenceSourceKind,
    assurance_level: EvidenceAssuranceLevel,
    observed_by_actor_source: Option<ActorSource>,
    producer_anchor: EvidenceProducerAnchor,
    relevance_assessment: EvidenceRelevanceAssessment,
    observed_at: Option<UtcTimestamp>,
    producer_id: Option<EvidenceProducerId>,
}

fn derive_record_run_observation_authority(
    context: &RecordRunObservationContext<'_>,
    input: &EvidenceObservationInput,
    output_artifact_refs: &[ArtifactRef],
    origin: RecordRunObservationOrigin,
) -> Result<DerivedObservationAuthority, RecordingError> {
    if origin == RecordRunObservationOrigin::ValidatedReuse
        && input.source_kind == EvidenceSourceKind::ReusedEvidence
    {
        let producer_ref = input.input_refs.first().cloned();
        return Ok(DerivedObservationAuthority {
            source_kind: input.source_kind,
            assurance_level: input.assurance_level,
            observed_by_actor_source: None,
            producer_anchor: EvidenceProducerAnchor {
                producer_kind: EvidenceProducerKind::ReusedEvidence,
                producer_ref: producer_ref.clone().into(),
                output_artifact_refs: output_artifact_refs.to_vec(),
                verification_basis: Some("core_validated_evidence_reuse".to_owned()).into(),
            },
            relevance_assessment: EvidenceRelevanceAssessment {
                status: EvidenceRelevanceStatus::Supported,
                assessment_ref: producer_ref.into(),
                assessed_by_actor_source: None.into(),
            },
            observed_at: None,
            producer_id: None,
        });
    }

    let canonical_capture = input
        .input_refs
        .iter()
        .find(|record_ref| record_ref.record_kind == StateRecordKind::EvidenceCaptureIntent)
        .and_then(|record_ref| {
            context
                .capture_authorities
                .get(record_ref.record_id.as_str())
        });
    if let Some(capture) = canonical_capture {
        let producer_id =
            allocate_evidence_producer_id(context.service.durable_id_generator(), context.store)
                .map_err(RecordingError::Core)?;
        let producer_ref = state_ref(
            StateRecordKind::EvidenceProducer,
            producer_id.as_str(),
            &context.request.envelope.project_id,
            Some(&context.request.task_id),
            Some(context.planned_state_version),
        );
        return Ok(DerivedObservationAuthority {
            source_kind: capture.source_kind,
            assurance_level: capture.assurance_level,
            observed_by_actor_source: Some(capture.observed_by_actor_source.clone()),
            producer_anchor: EvidenceProducerAnchor {
                producer_kind: capture.producer_kind,
                producer_ref: Some(producer_ref).into(),
                output_artifact_refs: output_artifact_refs.to_vec(),
                verification_basis: Some(capture.verification_basis.clone()).into(),
            },
            relevance_assessment: EvidenceRelevanceAssessment {
                status: capture.relevance_status,
                assessment_ref: Some(capture.intent_ref.clone()).into(),
                assessed_by_actor_source: None.into(),
            },
            observed_at: None,
            producer_id: Some(producer_id),
        });
    }

    let anchored = match (input.source_kind, input.assurance_level) {
        (EvidenceSourceKind::UserObservation, EvidenceAssuranceLevel::UserObserved) => {
            derive_user_observation_authority(context, input, output_artifact_refs)?
        }
        _ => None,
    };
    if let Some(authority) = anchored {
        return Ok(authority);
    }

    let (source_kind, assurance_level) = match (input.source_kind, input.assurance_level) {
        (EvidenceSourceKind::AgentReport, EvidenceAssuranceLevel::CooperativeReport) => {
            (input.source_kind, input.assurance_level)
        }
        (EvidenceSourceKind::UnverifiedClaim, EvidenceAssuranceLevel::Unverified) => {
            (input.source_kind, input.assurance_level)
        }
        _ => (
            EvidenceSourceKind::AgentReport,
            EvidenceAssuranceLevel::CooperativeReport,
        ),
    };
    Ok(DerivedObservationAuthority {
        source_kind,
        assurance_level,
        observed_by_actor_source: Some(context.verified_invocation.actor_source.clone()),
        producer_anchor: EvidenceProducerAnchor {
            producer_kind: EvidenceProducerKind::UnverifiedCaller,
            producer_ref: None.into(),
            output_artifact_refs: output_artifact_refs.to_vec(),
            verification_basis: None.into(),
        },
        relevance_assessment: EvidenceRelevanceAssessment {
            status: EvidenceRelevanceStatus::Unassessed,
            assessment_ref: None.into(),
            assessed_by_actor_source: None.into(),
        },
        observed_at: None,
        producer_id: None,
    })
}

fn derive_user_observation_authority(
    context: &RecordRunObservationContext<'_>,
    input: &EvidenceObservationInput,
    output_artifact_refs: &[ArtifactRef],
) -> CoreResult<Option<DerivedObservationAuthority>> {
    for input_ref in &input.input_refs {
        if input_ref.record_kind != StateRecordKind::UserActionResolution
            || input_ref.project_id != context.request.envelope.project_id
            || input_ref.task_id.as_ref() != Some(&context.request.task_id)
        {
            continue;
        }
        let Some(resolution_record) = context
            .store
            .user_action_resolution_record(input_ref.record_id.as_str())
            .map_err(CorePipelineError::from)?
        else {
            continue;
        };
        let Some(action_record) = context
            .store
            .user_action_record(resolution_record.user_action_request_id(), context.now)
            .map_err(CorePipelineError::from)?
        else {
            continue;
        };
        let Some(resolution_authority) = user_action_observation_resolution_authority(
            &action_record,
            &resolution_record,
            &context.request.envelope.project_id,
            &context.request.task_id,
            context.request.change_unit_id.as_str(),
            context.current_scope_revision,
            Some(context.request.baseline_ref.as_str()),
            &input.target,
            output_artifact_refs,
        )?
        else {
            continue;
        };
        let producer_ref = state_ref(
            StateRecordKind::UserActionResolution,
            resolution_record.user_action_resolution_id(),
            &context.request.envelope.project_id,
            Some(&context.request.task_id),
            Some(context.project_state.state_version),
        );
        return Ok(Some(DerivedObservationAuthority {
            source_kind: input.source_kind,
            assurance_level: input.assurance_level,
            observed_by_actor_source: Some(ActorSource::LocalUser),
            producer_anchor: EvidenceProducerAnchor {
                producer_kind: EvidenceProducerKind::UserChannelObservation,
                producer_ref: Some(producer_ref.clone()).into(),
                output_artifact_refs: output_artifact_refs.to_vec(),
                verification_basis: Some(
                    resolution_record
                        .resolved_verification_basis()
                        .as_str()
                        .to_owned(),
                )
                .into(),
            },
            relevance_assessment: EvidenceRelevanceAssessment {
                status: resolution_authority.relevance_status,
                assessment_ref: Some(producer_ref).into(),
                assessed_by_actor_source: Some(ActorSource::LocalUser).into(),
            },
            observed_at: Some(resolution_authority.resolved_at),
            producer_id: None,
        }));
    }
    Ok(None)
}

pub(super) fn validate_record_run_evidence_update(
    context: &RecordRunObservationContext<'_>,
    update: &EvidenceCoverageUpdate,
    explicit_observation_targets: &BTreeSet<EvidenceTarget>,
) -> Result<(), RecordingError> {
    validate_evidence_update_observation_refs(
        context,
        &update.target,
        &update.observation_refs,
        update.coverage_state == EvidenceCoverageUpdateState::Supported
            && !explicit_observation_targets.contains(&update.target)
            && update.provenance.is_none(),
    )?;
    validate_supporting_run_refs(context, &update.supporting_run_refs)?;
    canonical_evidence_artifact_refs(
        context,
        "evidence_updates[].supporting_artifact_refs",
        &update.supporting_artifact_refs,
    )?;
    validate_evidence_gap_refs(context, &update.gap_refs)?;
    if let Some(provenance) = update.provenance.as_ref() {
        validate_evidence_source_assurance(
            context.request.envelope.dry_run,
            Some(context.project_state.state_version),
            "evidence_updates[].provenance",
            provenance.source_kind,
            provenance.assurance_level,
        )?;
        if provenance
            .tool_name
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return recording_validation_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].provenance.tool_name",
                "tool_name must be null or a non-empty string",
            );
        }
        normalize_source_refs(
            context.store,
            context.project_state,
            &context.request.envelope,
            &context.request.task_id,
            "evidence_updates[].provenance.source_refs",
            &provenance.source_refs,
        )
        .map_err(RecordingError::Artifact)?;
        if provenance
            .tool_invocation_id
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return recording_validation_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].provenance.tool_invocation_id",
                "tool_invocation_id must be null or a non-empty string",
            );
        }
    }
    if update.coverage_state == EvidenceCoverageUpdateState::Supported
        && !explicit_observation_targets.contains(&update.target)
        && update.provenance.is_none()
        && update.observation_refs.is_empty()
    {
        return recording_validation_error(
            context.request.envelope.dry_run,
            Some(context.project_state.state_version),
            "evidence_updates[].provenance",
            "supported evidence updates require provenance or a target-matching evidence observation",
        );
    }
    Ok(())
}

pub(super) fn observation_input_from_evidence_update(
    context: &RecordRunObservationContext<'_>,
    update: &EvidenceCoverageUpdate,
    provenance: &EvidenceUpdateProvenance,
) -> EvidenceObservationInput {
    EvidenceObservationInput {
        target: update.target.clone(),
        source_kind: provenance.source_kind,
        assurance_level: provenance.assurance_level,
        observed_by_actor_source: None.into(),
        tool_name: provenance.tool_name.clone(),
        tool_invocation_id: provenance.tool_invocation_id.clone(),
        tool_metadata: provenance.tool_metadata.clone(),
        input_refs: update.supporting_run_refs.clone(),
        source_refs: provenance.source_refs.clone(),
        output_artifact_refs: update.supporting_artifact_refs.clone(),
        limitations: provenance.limitations.clone(),
        observed_at: provenance
            .observed_at
            .clone()
            .unwrap_or_else(|| context.now.clone()),
    }
}

pub(super) fn validate_evidence_source_assurance(
    dry_run: volicord_types::schema::DryRunIntent,
    state_version: Option<u64>,
    field: &'static str,
    source_kind: EvidenceSourceKind,
    assurance_level: EvidenceAssuranceLevel,
) -> Result<(), RecordingError> {
    if evidence_assurance_matches_source(source_kind, assurance_level) {
        Ok(())
    } else {
        recording_validation_error(
            dry_run,
            state_version,
            field,
            "evidence source_kind and assurance_level must describe the same provenance class",
        )
    }
}

pub(super) fn validate_evidence_observation_state_refs(
    context: &RecordRunObservationContext<'_>,
    field: &'static str,
    refs: &[StateRecordRef],
) -> Result<(), RecordingError> {
    for record_ref in refs {
        if record_ref.record_id.as_str().trim().is_empty() {
            return recording_validation_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                field,
                "evidence observation refs must use non-empty record_id values",
            );
        }
        if field == "evidence_updates[].observation_refs"
            && record_ref.record_kind != StateRecordKind::EvidenceObservation
        {
            return recording_validation_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                field,
                "evidence update observation_refs must identify evidence_observation records",
            );
        }
        if record_ref.project_id != context.request.envelope.project_id {
            return recording_validation_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                field,
                "evidence observation refs must belong to the request project",
            );
        }
        if record_ref
            .task_id
            .as_ref()
            .is_some_and(|task_id| task_id != &context.request.task_id)
        {
            return recording_validation_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                field,
                "evidence observation refs must not belong to another Task",
            );
        }
    }
    Ok(())
}

pub(super) fn validate_evidence_update_observation_refs(
    context: &RecordRunObservationContext<'_>,
    target: &EvidenceTarget,
    refs: &[StateRecordRef],
    require_strong_reuse: bool,
) -> Result<(), RecordingError> {
    for record_ref in refs {
        if record_ref.record_kind != StateRecordKind::EvidenceObservation
            || record_ref.project_id != context.request.envelope.project_id
            || record_ref.task_id.as_ref() != Some(&context.request.task_id)
            || record_ref.record_id.as_str().trim().is_empty()
        {
            return recording_validation_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].observation_refs",
                "evidence update observation refs must identify same-Task evidence observations",
            );
        }
        let record = context
            .store
            .evidence_observation_record(record_ref.record_id.as_str())
            .map_err(CorePipelineError::from)?;
        let Some(record) = record else {
            return recording_validation_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].observation_refs",
                "evidence update observation refs must identify existing observations",
            );
        };
        if record.task_id != context.request.task_id.as_str()
            || record.change_unit_id.as_deref() != Some(context.request.change_unit_id.as_str())
            || !stored_observation_target_matches(&record, target)
        {
            return recording_validation_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].observation_refs",
                "evidence update observation refs must match the current Task, Change Unit, and evidence target",
            );
        }
        let source_run = record
            .run_id
            .as_deref()
            .map(|run_id| context.store.run_record(run_id))
            .transpose()
            .map_err(CorePipelineError::from)?
            .flatten();
        if source_run.as_ref().is_none_or(|run| {
            !run_record_matches_close_basis_context(
                run,
                &context.request.envelope.project_id,
                &context.request.task_id,
                context.request.change_unit_id.as_str(),
                context.current_scope_revision,
                Some(context.request.baseline_ref.as_str()),
            )
        }) {
            return recording_validation_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].observation_refs",
                "evidence update observation refs must have current same-scope Run provenance",
            );
        }
        if require_strong_reuse
            && (classify_evidence_provenance(&stored_evidence_observation_provenance_facts(
                context.store,
                &record,
                &EvidenceObservationBasis {
                    project_id: &context.request.envelope.project_id,
                    task_id: &context.request.task_id,
                    change_unit_id: context.request.change_unit_id.as_str(),
                    scope_revision: context.current_scope_revision,
                    baseline_ref: Some(context.request.baseline_ref.as_str()),
                    target,
                    now: context.now,
                },
            )?) != EvidenceProvenanceClass::Strong
                || !relevance_supports_claim(&stored_evidence_observation_relevance(&record)?))
        {
            return recording_validation_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].observation_refs",
                "supported evidence may only reuse target-matching observations with sufficient provenance and supported relevance",
            );
        }
    }
    Ok(())
}

pub(super) fn reused_observation_inputs_for_update(
    context: &RecordRunObservationContext<'_>,
    update: &EvidenceCoverageUpdate,
) -> Result<Vec<EvidenceObservationInput>, RecordingError> {
    let mut inputs = Vec::with_capacity(update.observation_refs.len());
    for observation_ref in &update.observation_refs {
        let record = context
            .store
            .evidence_observation_record(observation_ref.record_id.as_str())
            .map_err(CorePipelineError::from)?
            .expect("validated reused observation exists");
        inputs.push(EvidenceObservationInput {
            target: update.target.clone(),
            source_kind: EvidenceSourceKind::ReusedEvidence,
            assurance_level: record.assurance_level,
            observed_by_actor_source: None.into(),
            tool_name: None.into(),
            tool_invocation_id: None.into(),
            tool_metadata: JsonObject::new(),
            input_refs: vec![state_ref(
                StateRecordKind::EvidenceObservation,
                &record.evidence_observation_id,
                &context.request.envelope.project_id,
                Some(&context.request.task_id),
                Some(context.project_state.state_version),
            )],
            source_refs: Vec::new(),
            output_artifact_refs: record.output_artifact_refs,
            limitations: vec![
                "Reuses target-matching observation provenance from the current scope.".to_owned(),
            ],
            observed_at: context.now.clone(),
        });
    }
    Ok(inputs)
}

pub(super) fn validate_supporting_run_refs(
    context: &RecordRunObservationContext<'_>,
    refs: &[StateRecordRef],
) -> Result<(), RecordingError> {
    for record_ref in refs {
        let is_current_run = record_ref.record_id == context.run_ref.record_id;
        let stored_run = if is_current_run {
            None
        } else {
            context
                .store
                .run_record(record_ref.record_id.as_str())
                .map_err(CorePipelineError::from)?
        };
        if record_ref.record_kind != StateRecordKind::Run
            || record_ref.project_id != context.request.envelope.project_id
            || record_ref.task_id.as_ref() != Some(&context.request.task_id)
            || record_ref.record_id.as_str().trim().is_empty()
            || (!is_current_run
                && stored_run.as_ref().is_none_or(|run| {
                    run.task_id != context.request.task_id.as_str()
                        || run.project_id != context.request.envelope.project_id.as_str()
                        || run.status != RunStatus::Recorded
                }))
        {
            return recording_validation_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].supporting_run_refs",
                "supporting_run_refs must identify existing Runs for the request Task",
            );
        }
    }
    Ok(())
}

pub(super) fn validate_evidence_gap_refs(
    context: &RecordRunObservationContext<'_>,
    refs: &[StateRecordRef],
) -> Result<(), RecordingError> {
    let active = context
        .store
        .active_blocker_refs(
            &context.request.task_id,
            context.project_state.state_version,
        )
        .map_err(CorePipelineError::from)?;
    for record_ref in refs {
        if record_ref.record_kind != StateRecordKind::Blocker
            || record_ref.project_id != context.request.envelope.project_id
            || record_ref.task_id.as_ref() != Some(&context.request.task_id)
            || !active
                .iter()
                .any(|stored| stored.record_id == record_ref.record_id.as_str())
        {
            return recording_validation_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                "evidence_updates[].gap_refs",
                "gap_refs must identify active blockers for the request Task",
            );
        }
    }
    Ok(())
}

pub(super) fn canonical_evidence_artifact_refs(
    context: &RecordRunObservationContext<'_>,
    field: &'static str,
    refs: &[ArtifactRef],
) -> Result<Vec<ArtifactRef>, RecordingError> {
    let mut canonical = BTreeMap::new();
    for artifact_ref in refs {
        let newly_registered = context
            .registered_artifacts
            .iter()
            .find(|registered| registered.artifact_id == artifact_ref.artifact_id);
        if artifact_ref.project_id != context.request.envelope.project_id
            || artifact_ref.task_id != context.request.task_id
        {
            return recording_validation_error(
                context.request.envelope.dry_run,
                Some(context.project_state.state_version),
                field,
                "evidence artifact refs must identify existing artifacts owned by the request project and Task",
            );
        }
        let canonical_ref = if let Some(registered) = newly_registered {
            registered.clone()
        } else {
            let stored = context
                .store
                .artifact_record(artifact_ref.artifact_id.as_str())
                .map_err(CorePipelineError::from)?;
            let owner_link = context
                .store
                .artifact_has_task_owner_link(
                    artifact_ref.artifact_id.as_str(),
                    context.request.task_id.as_str(),
                )
                .map_err(CorePipelineError::from)?;
            let Some(stored) = stored else {
                return recording_validation_error(
                    context.request.envelope.dry_run,
                    Some(context.project_state.state_version),
                    field,
                    "evidence artifact refs must identify existing artifacts owned by the request project and Task",
                );
            };
            if stored.project_id != context.request.envelope.project_id.as_str()
                || stored.task_id != context.request.task_id.as_str()
                || !owner_link
            {
                return recording_validation_error(
                    context.request.envelope.dry_run,
                    Some(context.project_state.state_version),
                    field,
                    "evidence artifact refs must identify existing artifacts owned by the request project and Task",
                );
            }
            artifact_ref_from_verified_record(
                context.store,
                &stored,
                None,
                Some(context.planned_state_version),
            )?
        };
        canonical
            .entry(canonical_ref.artifact_id.as_str().to_owned())
            .or_insert(canonical_ref);
    }
    Ok(canonical.into_values().collect())
}

pub(super) fn output_artifact_refs_for_observation(
    context: &RecordRunObservationContext<'_>,
    input: &EvidenceObservationInput,
) -> Vec<ArtifactRef> {
    input
        .output_artifact_refs
        .iter()
        .cloned()
        .chain(
            context
                .artifact_plans
                .iter()
                .filter(|plan| plan.evidence_target.as_ref() == Some(&input.target))
                .map(|plan| plan.artifact_ref.clone()),
        )
        .chain(
            context
                .registered_artifacts
                .iter()
                .filter(|artifact| {
                    input.output_artifact_refs.iter().any(|existing| {
                        existing.artifact_id == artifact.artifact_id
                            && existing.project_id == artifact.project_id
                    })
                })
                .cloned(),
        )
        .collect()
}

pub(super) fn observation_refs_by_target(
    plans: &[RecordRunObservationPlan],
) -> BTreeMap<EvidenceTarget, Vec<StateRecordRef>> {
    let mut refs_by_target: BTreeMap<EvidenceTarget, Vec<StateRecordRef>> = BTreeMap::new();
    for plan in plans {
        refs_by_target
            .entry(plan.observation.target.clone())
            .or_default()
            .push(plan.observation_ref.clone());
    }
    refs_by_target
}

pub(super) fn build_record_run_evidence_summary(
    context: &RecordRunObservationContext<'_>,
    request: &RecordRunRequest,
    run_ref: &StateRecordRef,
    registered_artifacts: &[ArtifactRef],
    artifact_plans: &[RecordRunArtifactPlan],
    observation_refs_by_target: &BTreeMap<EvidenceTarget, Vec<StateRecordRef>>,
) -> Result<Option<volicord_types::schema::EvidenceSummary>, RecordingError> {
    if request.evidence_updates.is_empty() {
        return Ok(None);
    }
    let mut coverage_items = Vec::new();
    for update in &request.evidence_updates {
        let mut item = EvidenceCoverageItem {
            target: update.target.clone(),
            coverage_state: update.coverage_state.into(),
            supporting_run_refs: update.supporting_run_refs.clone(),
            observation_refs: update.observation_refs.clone(),
            supporting_artifact_refs: canonical_evidence_artifact_refs(
                context,
                "evidence_updates[].supporting_artifact_refs",
                &update.supporting_artifact_refs,
            )?,
            gap_refs: update.gap_refs.clone(),
        };
        if !item.supporting_run_refs.iter().any(|record_ref| {
            state_record_ref_identity_key(record_ref) == state_record_ref_identity_key(run_ref)
        }) {
            item.supporting_run_refs.push(run_ref.clone());
        }
        for plan in artifact_plans {
            if plan.evidence_target.as_ref() == Some(&item.target)
                && !item
                    .supporting_artifact_refs
                    .iter()
                    .any(|artifact| artifact.artifact_id == plan.artifact_ref.artifact_id)
            {
                item.supporting_artifact_refs
                    .push(plan.artifact_ref.clone());
            }
        }
        if let Some(observation_refs) = observation_refs_by_target.get(&item.target) {
            for observation_ref in observation_refs {
                if !item.observation_refs.iter().any(|existing| {
                    state_record_ref_identity_key(existing)
                        == state_record_ref_identity_key(observation_ref)
                }) {
                    item.observation_refs.push(observation_ref.clone());
                }
            }
        }
        if item.coverage_state == EvidenceCoverageState::Supported
            && item.supporting_artifact_refs.iter().any(|artifact_ref| {
                artifact_ref.availability != ArtifactAvailability::Available
                    || artifact_ref.integrity_status != ArtifactIntegrityStatus::Verified
            })
        {
            item.coverage_state = EvidenceCoverageState::Stale;
        }
        coverage_items.push(item);
    }
    let artifact_refs = unique_artifact_refs(
        registered_artifacts
            .iter()
            .cloned()
            .chain(
                coverage_items
                    .iter()
                    .flat_map(|item| item.supporting_artifact_refs.clone()),
            )
            .collect(),
    );
    let observation_refs = unique_state_record_refs(
        coverage_items
            .iter()
            .flat_map(|item| item.observation_refs.clone())
            .collect(),
    );
    let status = evidence_status_for_items(&coverage_items);
    Ok(Some(volicord_types::schema::EvidenceSummary {
        evidence_state: Some(EvidenceDisplayState::Attached),
        status,
        coverage_items,
        artifact_refs,
        observation_refs,
        updated_by_run_ref: Some(run_ref.clone()),
    }))
}
