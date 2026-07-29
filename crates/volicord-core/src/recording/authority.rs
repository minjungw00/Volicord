use crate::evidence_facts::{
    capture_intent_from_record, capture_outcome_matches_expected, capture_verification_basis,
    validate_capture_receipt_record,
};
use crate::json_object::object_from_value;
use crate::pipeline::{CorePipelineError, CoreService};
use crate::policy::evidence_relevance::capture_outcome_relevance;
use crate::record_refs::state_ref;
use crate::recording::{RecordingError, RecordingRejection};
use std::collections::{BTreeMap, BTreeSet};
use volicord_store::core_pipeline::ArtifactStagingStatus;
use volicord_store::evidence_capture::EvidenceCaptureIntentRecord;
use volicord_types::ids::{ArtifactInputId, StagedArtifactHandleId};
use volicord_types::schema::{
    ArtifactInput, EvidenceCaptureIntent, EvidenceCaptureSpec, StagedArtifactHandle,
};
use volicord_types::values::{
    ArtifactInputSourceKind, EvidenceAssuranceLevel, EvidenceProducerKind, EvidenceSourceKind,
    RedactionState, StateRecordKind,
};

use super::{
    artifact::plan_staged_artifact_input,
    model::{RecordRunArtifactContext, RecordRunArtifactPlan, RecordRunCaptureAuthority},
};
use crate::task_state::normalize_display_string_list;

pub(super) fn plan_record_run_capture_authorities(
    service: &CoreService,
    artifact_context: &RecordRunArtifactContext<'_>,
    current_scope_revision: u64,
) -> Result<
    (
        Vec<RecordRunArtifactPlan>,
        BTreeMap<String, RecordRunCaptureAuthority>,
    ),
    RecordingError,
> {
    let mut intent_ids = BTreeSet::new();
    for observation in &artifact_context.request.evidence_observations {
        let matching = observation
            .input_refs
            .iter()
            .filter(|record_ref| record_ref.record_kind == StateRecordKind::EvidenceCaptureIntent)
            .collect::<Vec<_>>();
        if matching.len() > 1 {
            return capture_authority_error(
                "one evidence observation may cite at most one evidence-capture intent",
            );
        }
        if let Some(record_ref) = matching.first() {
            if !intent_ids.insert(record_ref.record_id.as_str().to_owned()) {
                return capture_authority_error(
                    "one evidence-capture intent cannot be consumed by multiple observations",
                );
            }
        }
    }

    let mut artifact_plans = Vec::new();
    let mut authorities = BTreeMap::new();
    for intent_id in intent_ids {
        let (artifact_plan, authority) = plan_record_run_capture_authority(
            service,
            artifact_context,
            current_scope_revision,
            &intent_id,
        )?;
        authorities.insert(intent_id, authority);
        artifact_plans.push(artifact_plan);
    }
    Ok((artifact_plans, authorities))
}

pub(super) fn plan_record_run_capture_authority(
    service: &CoreService,
    artifact_context: &RecordRunArtifactContext<'_>,
    current_scope_revision: u64,
    intent_id: &str,
) -> Result<(RecordRunArtifactPlan, RecordRunCaptureAuthority), RecordingError> {
    let request = artifact_context.request;
    let project_state = artifact_context.project_state;
    let store = artifact_context.store;
    if store
        .evidence_producer_for_intent(intent_id)
        .map_err(CorePipelineError::from)?
        .is_some()
    {
        return capture_authority_error(
            "evidence-capture intent is already finalized and must be reused through its observation",
        );
    }
    let intent_record = store
        .evidence_capture_intent_record(intent_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| capture_authority_rejection("evidence-capture intent was not found"))?;
    let intent = capture_intent_from_record(&intent_record)?;
    let intent_ref = state_ref(
        StateRecordKind::EvidenceCaptureIntent,
        intent_id,
        &request.project_id,
        Some(&request.task_id),
        Some(project_state.state_version),
    );
    validate_capture_intent_current(
        artifact_context,
        current_scope_revision,
        &intent_record,
        &intent,
    )?;

    let receipt = store
        .evidence_capture_receipt_for_intent(intent_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            capture_authority_rejection("evidence-capture source receipt is not available")
        })?;
    let body = validate_capture_receipt_record(&intent, &receipt)?;
    store
        .validate_evidence_capture_source_claims_for_receipt(
            &intent_record,
            &receipt,
            &intent.capture,
            &body,
        )
        .map_err(CorePipelineError::from)?;
    let receipt_created_at = receipt.created_at.clone();
    if body.observed_at > *artifact_context.now
        || receipt_created_at > *artifact_context.now
        || body.observed_at >= intent.expires_at
        || artifact_context.now >= &intent.expires_at
    {
        return capture_authority_error(
            "evidence-capture intent or receipt is outside its current time window",
        );
    }
    if body.observed_by_actor_source != intent.requested_by_actor_source
        || body.source.connection_id != intent_record.requesting_connection_internal_id
        || artifact_context.verified_invocation.actor_source != intent.requested_by_actor_source
    {
        return capture_authority_error(
            "evidence-capture source connection does not match the immutable intent",
        );
    }

    let staging = store
        .artifact_staging_record(&receipt.staging_handle_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            capture_authority_rejection("evidence-capture receipt staging handle was not found")
        })?;
    if staging.sha256.as_deref() != Some(receipt.safe_receipt_sha256.as_str())
        || staging.size_bytes != Some(receipt.safe_receipt_size_bytes)
        || staging.content_type.as_deref() != Some("application/json")
        || staging.redaction_state != RedactionState::Redacted
        || staging.expires_at != intent.expires_at
    {
        return capture_authority_error(
            "evidence-capture receipt staging facts do not match the immutable receipt",
        );
    }
    let staged_handle = StagedArtifactHandle {
        handle_id: StagedArtifactHandleId::new(receipt.staging_handle_id.clone()),
        project_id: request.project_id.clone(),
        task_id: request.task_id.clone(),
        created_by_actor_source: body.observed_by_actor_source.clone(),
        content_type: "application/json".to_owned(),
        sha256: receipt.safe_receipt_sha256.clone(),
        size_bytes: receipt.safe_receipt_size_bytes,
        redaction_state: RedactionState::Redacted,
        expires_at: staging.expires_at.clone(),
        consumed: staging.status == ArtifactStagingStatus::Consumed,
    };
    let artifact_input = ArtifactInput {
        artifact_input_id: ArtifactInputId::new(format!("capture_receipt_{intent_id}")),
        source_kind: ArtifactInputSourceKind::StagedArtifact,
        staged_artifact_handle: Some(staged_handle.clone()).into(),
        existing_artifact_ref: None.into(),
        relation_hint: Some("evidence_capture_receipt".to_owned()).into(),
        evidence_target: Some(intent.target.clone()).into(),
        expected_sha256: Some(receipt.safe_receipt_sha256.clone()).into(),
        expected_size_bytes: Some(receipt.safe_receipt_size_bytes).into(),
        redaction_state: Some(RedactionState::Redacted).into(),
    };
    let artifact_plan =
        plan_staged_artifact_input(service, artifact_context, &artifact_input, &staged_handle)?;
    let (source_kind, assurance_level, tool_name) = match body.capture_kind {
        EvidenceProducerKind::VerifiedCommandExecution => (
            EvidenceSourceKind::ExternalTool,
            EvidenceAssuranceLevel::ExternalToolResult,
            Some("volicord.command_runner".to_owned()),
        ),
        EvidenceProducerKind::VerifiedToolInvocation => {
            let tool_name = match &intent.capture {
                EvidenceCaptureSpec::VerifiedToolInvocation { tool_name, .. } => {
                    Some(tool_name.clone())
                }
                _ => None,
            };
            (
                EvidenceSourceKind::ExternalTool,
                EvidenceAssuranceLevel::ExternalToolResult,
                tool_name,
            )
        }
        EvidenceProducerKind::UnverifiedCaller
        | EvidenceProducerKind::UserChannelObservation
        | EvidenceProducerKind::ReusedEvidence => {
            return Err(RecordingError::Core(CorePipelineError::Invariant {
                detail: format!(
                    "typed evidence receipt `{}` has a capture kind that cannot produce strict evidence",
                    receipt.evidence_capture_receipt_id
                ),
            }))
        }
    };
    let verification_basis = capture_verification_basis(body.capture_kind)
        .expect("strict capture producer kinds have a verification basis")
        .to_owned();
    let outcome_matches_expected = capture_outcome_matches_expected(
        &receipt.evidence_capture_receipt_id,
        &intent.capture,
        &body.expected_outcome,
        &body.observed_outcome,
    )?;
    let relevance_status = capture_outcome_relevance(outcome_matches_expected);
    let source_refs = receipt.source_refs.clone();
    for source_ref in &source_refs {
        if source_ref.project_id != request.project_id
            || source_ref
                .task_id
                .as_ref()
                .is_some_and(|task_id| task_id != &request.task_id)
        {
            return capture_authority_error(
                "evidence-capture receipt source refs cross the request scope",
            );
        }
    }
    let authority = RecordRunCaptureAuthority {
        intent,
        intent_ref,
        receipt,
        producer_kind: body.capture_kind,
        source_kind,
        assurance_level,
        relevance_status,
        receipt_artifact_ref: artifact_plan.artifact_ref.clone(),
        source_refs,
        connection_id: body.source.connection_id,
        host_invocation_id: body.source.host_invocation_id.into_option(),
        observed_by_actor_source: body.observed_by_actor_source,
        observed_outcome: body.observed_outcome,
        limitations: normalize_display_string_list(&body.limitations),
        observed_at: body.observed_at,
        tool_name,
        verification_basis,
    };
    Ok((artifact_plan, authority))
}

pub(super) fn validate_capture_intent_current(
    context: &RecordRunArtifactContext<'_>,
    current_scope_revision: u64,
    record: &EvidenceCaptureIntentRecord,
    intent: &EvidenceCaptureIntent,
) -> Result<(), RecordingError> {
    let request = context.request;
    let current_workspace = context
        .verified_invocation
        .git_workspace_context
        .as_ref()
        .map(|workspace| {
            serde_json::to_value(workspace)
                .map_err(RecordingError::from)
                .and_then(|value| object_from_value(value).map_err(RecordingError::from))
        })
        .transpose()?
        .unwrap_or_default();
    if intent.project_id != request.project_id
        || intent.task_id != request.task_id
        || intent.change_unit_id != request.change_unit_id
        || intent.scope_revision != current_scope_revision
        || intent.baseline_ref != request.baseline_ref
        || intent.requested_by_actor_source != context.verified_invocation.actor_source
        || current_workspace != record.workspace_context
    {
        return capture_authority_error(
            "evidence-capture intent is stale or belongs to another current basis",
        );
    }
    Ok(())
}

pub(super) fn capture_authority_rejection(message: &'static str) -> RecordingError {
    RecordingError::Rejected(RecordingRejection::EvidenceInsufficient { message })
}

pub(super) fn capture_authority_error<T>(message: &'static str) -> Result<T, RecordingError> {
    Err(capture_authority_rejection(message))
}
