use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
};

use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use volicord_store::{
    artifacts::{ArtifactStagingInsert, PersistentArtifactVerificationStatus, StagedPayloadKind},
    core_pipeline::*,
    diagnostics::{
        record_core_rejection_diagnostic, record_workflow_metric_event, CoreRejectionDiagnostic,
        CoreRejectionReason, WorkflowMetricEvent, WorkflowMetricKind,
    },
    evidence_capture::{
        EvidenceCaptureIntentInsert, EvidenceCaptureIntentRecord, EvidenceCaptureReceiptRecord,
        EvidenceProducerInsert, MAX_EVIDENCE_CAPTURE_RECEIPT_BYTES,
    },
    guards::UnrecordedChangeRecord,
    RuntimeHomeMutationContext, StoreError, StoreResult,
};
use volicord_types::*;

#[cfg(test)]
use volicord_types::EVIDENCE_CAPTURE_COMMAND_LIMITATION;

use crate::pipeline::{
    dry_run_response, method_result_base, operation_result_ref, rejected_response,
    store_failure_error, tool_error, CorePipelineError, CoreResult, CoreService, FreshnessPolicy,
    InvocationContext, MethodEffectPolicy, MethodPolicy, OwnerPipelineBranch,
    PipelinePreflightOutcome, PipelinePreflightRequest, PipelineResponse, PreparedRequest,
    ReplayPolicy, TaskRequirement, VerifiedActorContext, VerifiedInvocationContext,
};
use crate::policy::{
    close_readiness::{
        accepted_current_scope_decision_authority, close_basis_is_current, close_basis_run_refs,
        close_blocker, close_next_action, current_acceptance_required_risk_ids,
        current_cancellation_authority, current_final_acceptance,
        current_residual_risk_acceptance_coverage, final_acceptance_requirement,
        is_terminal_lifecycle, run_record_matches_close_basis_context,
        user_action_has_current_basis, verified_user_channel_provenance,
        CancellationAuthorityRequirement, ScopeDecisionAuthorityRequirement, UserActionAuthority,
    },
    continuity::{decision_title_prefix, judgment_continuity_kind},
    effect_contract::{
        product_write_violations, validate_effect_contract, validate_effect_contract_paths,
        EffectContractValidationError, EffectContractViolation,
    },
    evidence::{
        evidence_assurance_matches_source, evidence_item_has_no_support,
        evidence_item_related_refs, evidence_status_for_items, state_record_ref_identity_key,
        unique_artifact_refs, unique_state_record_refs, EvidenceProvenanceClass,
    },
    path::{normalize_product_paths, path_is_within, paths_are_authorized, ProductPathError},
    user_action_relevance::{
        user_action_blocks_operation, user_action_keeps_task_waiting, user_action_required_for,
        UserActionOperation, UserActionOperationContext,
    },
    workflow::{
        acceptance_policy_for_control, effective_control_level, parse_requested_control_level,
        parse_task_control_level, project_workflow_policy, resolve_task_control_authority,
        ProjectWorkflowPolicy,
    },
    write_ticket::{
        current_sensitive_approval, normalize_sensitive_action_scope, normalized_string_set,
        prepare_write_decision, prepare_write_dry_run_summary, run_write_ticket_mismatch,
        write_decision_reason, write_ticket_is_idle_expired, RunWriteTicketAttempt,
        SensitiveApprovalRequirement,
    },
};
use crate::{
    CurrentUserActionProjection, UserChannelInboxProjection, UserChannelInboxProjectionItem,
    UserChannelInboxProjectionRequest, UserChannelInboxResolutionSnapshot,
};

mod close_task;
mod intake;
mod operation_result;
mod prepare_evidence_capture;
mod prepare_write;
mod reconcile_changes;
mod record_run;
mod stage_artifact;
mod status;
#[cfg(test)]
mod tests;
mod update_scope;
mod user_action;

struct MethodPlan<F> {
    task_id: TaskId,
    change_unit_id: Option<ChangeUnitId>,
    storage_mutations: Vec<CoreStorageMutation>,
    event_payload: JsonObject,
    result_fields: F,
    next_actions: Vec<NextActionSummary>,
}

struct PrepareWritePlan {
    task_id: TaskId,
    change_unit_id: ChangeUnitId,
    storage_mutations: Vec<CoreStorageMutation>,
    event_kind: String,
    event_payload: JsonObject,
    result_fields: PrepareWriteResultFields,
    dry_run_summary: DryRunSummary,
}

struct CloseTaskPlan {
    task_id: TaskId,
    change_unit_id: Option<ChangeUnitId>,
    storage_mutations: Vec<CoreStorageMutation>,
    event_kind: String,
    event_payload: JsonObject,
    result_fields: CloseTaskResultFields,
    close_state: CloseState,
    current_close_basis: Option<CurrentCloseBasis>,
    risk_acceptance_coverage: Vec<RiskAcceptanceCoverage>,
    blockers: Vec<CloseReadinessBlocker>,
    evidence_gate: EvidenceGateSummary,
}

struct CloseTaskContext {
    now: UtcTimestamp,
    task: TaskRecord,
    current_change_unit: Option<ChangeUnitRecord>,
    current_close_basis: Option<CurrentCloseBasis>,
    pending_user_action_refs: Vec<StateRecordRef>,
    blocker_refs: Vec<StateRecordRef>,
    evidence_summary: Option<EvidenceSummary>,
    artifact_refs: Vec<ArtifactRef>,
    projected_run_refs: Vec<StateRecordRef>,
    projected_evidence_observations: Vec<EvidenceObservation>,
    projected_artifacts: Vec<ArtifactRef>,
    projected_required_criterion_ids: Option<BTreeSet<String>>,
    projected_resolved_unrecorded_change_ids: BTreeSet<String>,
    pending_user_action_authorities: Option<Vec<UserActionAuthority>>,
    resolved_judgment_authorities: Option<Vec<UserActionAuthority>>,
}

struct ProjectContinuityDraft {
    kind: ProjectContinuityKind,
    title: String,
    summary: String,
    rationale: Option<String>,
    applies_to_paths: Vec<String>,
    applies_to_refs: Vec<StateRecordRef>,
    source_refs: Vec<StateRecordRef>,
    artifact_refs: Vec<ArtifactRef>,
    supersedes_refs: Vec<StateRecordRef>,
    review_triggers: Vec<String>,
    metadata: Value,
}

#[derive(Clone, Copy)]
struct ProjectContinuityPlanContext<'a> {
    service: &'a CoreService,
    store: &'a CoreProjectStore<'a>,
    project_id: &'a ProjectId,
    source_task_id: &'a TaskId,
    source_change_unit_id: Option<&'a ChangeUnitId>,
    planned_state_version: u64,
    now: &'a UtcTimestamp,
}

struct PlannedProjectContinuityRecord {
    record_ref: StateRecordRef,
    summary: ProjectContinuitySummary,
    mutation: CoreStorageMutation,
}

struct ValidatedStageArtifactInput {
    safe_bytes: Vec<u8>,
    sha256: String,
    size_bytes: u64,
    payload_kind: StagedPayloadKind,
}

const MAX_STAGED_BODY_BYTES: usize = 10 * 1024 * 1024;

fn elapsed_micros(start: &UtcTimestamp, end: &UtcTimestamp) -> Option<u64> {
    end.as_datetime()
        .signed_duration_since(start.as_datetime())
        .num_microseconds()
        .and_then(|value| u64::try_from(value).ok())
}

fn first_product_write_duration_micros(
    store: &CoreProjectStore,
    task_id: &TaskId,
    observed_no_later_than: &UtcTimestamp,
) -> Option<u64> {
    let task_created_at = store.task_created_at(task_id).ok()??;
    store
        .product_write_observation_candidates_for_task(task_id)
        .ok()?
        .into_iter()
        .filter_map(|candidate| {
            let paths = serde_json::from_str::<Vec<String>>(&candidate.observed_paths_json).ok()?;
            let normalized =
                normalize_product_paths(&store.project_record().repo_root, &paths).ok()?;
            if normalized.is_empty() {
                return None;
            }
            let observed_at = UtcTimestamp::parse(&candidate.observed_at).ok()?;
            if observed_at.as_datetime() < task_created_at.as_datetime()
                || observed_at.as_datetime() > observed_no_later_than.as_datetime()
            {
                return None;
            }
            elapsed_micros(&task_created_at, &observed_at)
        })
        .min()
}

fn record_core_workflow_metric_best_effort(
    context: &RuntimeHomeMutationContext<'_>,
    session_id: Option<&str>,
    metric_kind: WorkflowMetricKind,
    value: u64,
) {
    let Some(session_id) = session_id else {
        return;
    };
    let _ = record_workflow_metric_event(
        context,
        &WorkflowMetricEvent {
            session_id: session_id.to_owned(),
            metric_kind,
            value,
            method_name: None,
            integration_profile: None,
            decision: None,
            observation_confidence: None,
            outcome: None,
        },
    );
}

fn response_committed_fresh_effect(response: &PipelineResponse) -> bool {
    !response.replayed
        && response
            .response_value
            .pointer("/base/effect_kind")
            .and_then(Value::as_str)
            == Some("core_committed")
}

enum PlanError {
    Core(CorePipelineError),
    Response(Box<PipelineResponse>),
}

impl From<CorePipelineError> for PlanError {
    fn from(error: CorePipelineError) -> Self {
        Self::Core(error)
    }
}

impl From<serde_json::Error> for PlanError {
    fn from(error: serde_json::Error) -> Self {
        Self::Core(CorePipelineError::from(error))
    }
}

fn allocate_task_id(service: &CoreService, store: &CoreProjectStore) -> CoreResult<TaskId> {
    service
        .allocate_generated_id(DurableIdKind::Task, |candidate| {
            store
                .task_exists(&TaskId::new(candidate))
                .map_err(CorePipelineError::from)
        })
        .map(TaskId::new)
}

fn allocate_change_unit_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<ChangeUnitId> {
    service
        .allocate_generated_id(DurableIdKind::ChangeUnit, |candidate| {
            store
                .change_unit_id_exists(candidate)
                .map_err(CorePipelineError::from)
        })
        .map(ChangeUnitId::new)
}

fn allocate_user_action_request_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<UserActionRequestId> {
    service
        .allocate_generated_id(DurableIdKind::UserActionRequest, |candidate| {
            store
                .user_action_request_id_exists(candidate)
                .map_err(CorePipelineError::from)
        })
        .map(UserActionRequestId::new)
}

fn allocate_user_action_resolution_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<UserActionResolutionId> {
    service
        .allocate_generated_id(DurableIdKind::UserActionResolution, |candidate| {
            store
                .user_action_resolution_record(candidate)
                .map(|record| record.is_some())
                .map_err(CorePipelineError::from)
        })
        .map(UserActionResolutionId::new)
}

fn allocate_write_ticket_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<WriteTicketId> {
    service
        .allocate_generated_id(DurableIdKind::WriteTicket, |candidate| {
            store
                .write_ticket_record(candidate)
                .map(|record| record.is_some())
                .map_err(CorePipelineError::from)
        })
        .map(WriteTicketId::new)
}

fn allocate_run_id(service: &CoreService, store: &CoreProjectStore) -> CoreResult<RunId> {
    service
        .allocate_generated_id(DurableIdKind::Run, |candidate| {
            store
                .run_id_exists(candidate)
                .map_err(CorePipelineError::from)
        })
        .map(RunId::new)
}

fn allocate_staged_artifact_handle_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<StagedArtifactHandleId> {
    service
        .allocate_generated_id(DurableIdKind::StagedArtifact, |candidate| {
            store
                .artifact_staging_record(candidate)
                .map(|record| record.is_some())
                .map_err(CorePipelineError::from)
        })
        .map(StagedArtifactHandleId::new)
}

fn allocate_artifact_id(service: &CoreService, store: &CoreProjectStore) -> CoreResult<ArtifactId> {
    service
        .allocate_generated_id(DurableIdKind::Artifact, |candidate| {
            store
                .artifact_record(candidate)
                .map(|record| record.is_some())
                .map_err(CorePipelineError::from)
        })
        .map(ArtifactId::new)
}

fn allocate_evidence_summary_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<String> {
    service.allocate_generated_id(DurableIdKind::Evidence, |candidate| {
        store
            .evidence_summary_exists(candidate)
            .map_err(CorePipelineError::from)
    })
}

fn allocate_acceptance_criterion_id(
    service: &CoreService,
    store: &CoreProjectStore,
    reserved_ids: &BTreeSet<String>,
) -> CoreResult<AcceptanceCriterionId> {
    service
        .allocate_generated_id(DurableIdKind::AcceptanceCriterion, |candidate| {
            if reserved_ids.contains(candidate) {
                return Ok(true);
            }
            store
                .acceptance_criterion_id_exists(candidate)
                .map_err(CorePipelineError::from)
        })
        .map(AcceptanceCriterionId::new)
}

fn allocate_evidence_observation_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<EvidenceObservationId> {
    service
        .allocate_generated_id(DurableIdKind::EvidenceObservation, |candidate| {
            store
                .evidence_observation_exists(candidate)
                .map_err(CorePipelineError::from)
        })
        .map(EvidenceObservationId::new)
}

fn allocate_evidence_capture_intent_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<EvidenceCaptureIntentId> {
    service
        .allocate_generated_id(DurableIdKind::EvidenceCaptureIntent, |candidate| {
            store
                .evidence_capture_intent_record(candidate)
                .map(|record| record.is_some())
                .map_err(CorePipelineError::from)
        })
        .map(EvidenceCaptureIntentId::new)
}

fn allocate_evidence_producer_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<EvidenceProducerId> {
    service
        .allocate_generated_id(DurableIdKind::EvidenceProducer, |candidate| {
            store
                .evidence_producer_record(candidate)
                .map(|record| record.is_some())
                .map_err(CorePipelineError::from)
        })
        .map(EvidenceProducerId::new)
}

fn allocate_risk_id(
    service: &CoreService,
    allocated_in_basis: &BTreeSet<String>,
) -> CoreResult<RiskId> {
    service
        .allocate_generated_id(DurableIdKind::Risk, |candidate| {
            Ok(allocated_in_basis.contains(candidate))
        })
        .map(RiskId::new)
}

fn allocate_project_continuity_record_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<ProjectContinuityRecordId> {
    service
        .allocate_generated_id(DurableIdKind::ProjectContinuityRecord, |candidate| {
            store
                .project_continuity_record_exists(candidate)
                .map_err(CorePipelineError::from)
        })
        .map(ProjectContinuityRecordId::new)
}

fn plan_project_continuity_record(
    context: ProjectContinuityPlanContext<'_>,
    draft: ProjectContinuityDraft,
) -> CoreResult<PlannedProjectContinuityRecord> {
    let continuity_record_id =
        allocate_project_continuity_record_id(context.service, context.store)?;
    let record_ref = state_ref(
        StateRecordKind::ProjectContinuityRecord,
        continuity_record_id.as_str(),
        context.project_id,
        Some(context.source_task_id),
        Some(context.planned_state_version),
    );
    let applies_to_paths = sorted_unique(draft.applies_to_paths);
    let applies_to_refs = unique_state_refs(draft.applies_to_refs);
    let source_refs = unique_state_refs(draft.source_refs);
    let artifact_refs = unique_artifact_refs(draft.artifact_refs);
    let supersedes_refs = unique_state_refs(draft.supersedes_refs);
    let review_triggers = sorted_unique(draft.review_triggers);
    let source_task_ref = state_ref(
        StateRecordKind::Task,
        context.source_task_id.as_str(),
        context.project_id,
        Some(context.source_task_id),
        Some(context.planned_state_version),
    );
    let source_change_unit_ref = context
        .source_change_unit_id
        .map(|change_unit_id| {
            state_ref(
                StateRecordKind::ChangeUnit,
                change_unit_id.as_str(),
                context.project_id,
                Some(context.source_task_id),
                Some(context.planned_state_version),
            )
        })
        .into();
    let summary = ProjectContinuitySummary {
        continuity_record_ref: record_ref.clone(),
        kind: draft.kind,
        status: ProjectContinuityStatus::Active,
        title: draft.title.clone(),
        summary: draft.summary.clone(),
        source_task_ref,
        source_change_unit_ref,
        review_triggers: review_triggers.clone(),
    };
    Ok(PlannedProjectContinuityRecord {
        record_ref,
        summary,
        mutation: CoreStorageMutation::InsertProjectContinuityRecord(
            ProjectContinuityRecordInsert {
                continuity_record_id: continuity_record_id.as_str().to_owned(),
                source_task_id: context.source_task_id.as_str().to_owned(),
                source_change_unit_id: context
                    .source_change_unit_id
                    .map(|change_unit_id| change_unit_id.as_str().to_owned()),
                kind: storage_value(draft.kind)?,
                title: draft.title,
                summary: draft.summary,
                rationale: draft.rationale,
                applies_to_paths_json: serde_json::to_string(&applies_to_paths)?,
                applies_to_refs_json: serde_json::to_string(&applies_to_refs)?,
                source_refs_json: serde_json::to_string(&source_refs)?,
                artifact_refs_json: serde_json::to_string(&artifact_refs)?,
                status: storage_value(ProjectContinuityStatus::Active)?,
                supersedes_refs_json: serde_json::to_string(&supersedes_refs)?,
                review_triggers_json: serde_json::to_string(&review_triggers)?,
                created_at: context.now.to_string(),
                updated_at: context.now.to_string(),
                metadata_json: serde_json::to_string(&draft.metadata)?,
            },
        ),
    })
}

fn prepare_or_response<'mutation>(
    service: &CoreService,
    context: Option<&'mutation RuntimeHomeMutationContext<'mutation>>,
    method_name: MethodName,
    envelope: ToolEnvelope,
    request_json: Value,
    invocation: InvocationContext,
    policy: MethodPolicy,
) -> CoreResult<Result<PreparedRequest<'mutation>, PipelineResponse>> {
    match service.prepare_request(
        context,
        PipelinePreflightRequest {
            method_name,
            envelope,
            request_json,
            invocation,
            policy,
        },
    )? {
        PipelinePreflightOutcome::Prepared(prepared) => Ok(Ok(*prepared)),
        PipelinePreflightOutcome::Response(response) => Ok(Err(*response)),
    }
}

fn parse_storage_value<T>(field: &'static str, value: &str) -> CoreResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(Value::String(value.to_owned())).map_err(|_| {
        CorePipelineError::Store(StoreError::corrupt_stored_value("project_state", field))
    })
}

fn utc_timestamp(timestamp: DateTime<Utc>) -> UtcTimestamp {
    UtcTimestamp::from_datetime(timestamp)
}

fn parse_owner_storage_value<T>(
    table: &'static str,
    record_ref: impl Into<String>,
    logical_column: &'static str,
    value: &str,
) -> CoreResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let record_ref = record_ref.into();
    serde_json::from_value(Value::String(value.to_owned())).map_err(|_| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_value(
            table,
            record_ref,
            logical_column,
        ))
    })
}

fn artifact_ref_from_verified_record(
    store: &CoreProjectStore,
    record: &StoredArtifactRecord,
    display_name: Option<String>,
    created_by_run_state_version: Option<u64>,
) -> CoreResult<ArtifactRef> {
    let verification_status = persistent_artifact_verification_status(store, record)?;
    let task_id = TaskId::new(record.task_id.clone());
    let integrity_status = effective_artifact_integrity_status(record, verification_status)?;
    Ok(ArtifactRef {
        artifact_id: ArtifactId::new(record.artifact_id.clone()),
        project_id: ProjectId::new(record.project_id.clone()),
        task_id: task_id.clone(),
        display_name: display_name
            .or_else(|| record.producer.display_name.clone())
            .unwrap_or_else(|| record.artifact_id.clone()),
        content_type: sanitized_artifact_content_type(record, integrity_status).into(),
        sha256: sanitized_artifact_sha256(record, integrity_status).into(),
        size_bytes: record.size_bytes.into(),
        integrity_status,
        redaction_state: parse_owner_storage_value(
            "artifacts",
            record.artifact_id.clone(),
            "redaction_state",
            &record.redaction_state,
        )?,
        availability: artifact_availability_for_verification_status(record, verification_status)?,
        created_by_run_ref: Some(state_ref(
            StateRecordKind::Run,
            record.provenance.producer_run_id.as_str(),
            &ProjectId::new(record.project_id.clone()),
            Some(&task_id),
            created_by_run_state_version,
        ))
        .into(),
        created_by_actor_source: Some(record.producer.created_by_actor_source.clone()).into(),
        storage_ref: Some(StorageRef::new(record.uri.clone())).into(),
    })
}

fn persistent_artifact_is_verified_current(
    store: &CoreProjectStore,
    record: &StoredArtifactRecord,
) -> CoreResult<bool> {
    Ok(persistent_artifact_verification_status(store, record)?
        == PersistentArtifactVerificationStatus::VerifiedCurrent)
}

fn persistent_artifact_verification_status(
    store: &CoreProjectStore,
    record: &StoredArtifactRecord,
) -> CoreResult<PersistentArtifactVerificationStatus> {
    store
        .verify_persistent_artifact_body(record)
        .map(|verification| verification.status)
        .map_err(CorePipelineError::from)
}

fn artifact_availability_for_verification_status(
    record: &StoredArtifactRecord,
    verification_status: PersistentArtifactVerificationStatus,
) -> CoreResult<ArtifactAvailability> {
    match verification_status {
        PersistentArtifactVerificationStatus::VerifiedCurrent => {
            Ok(ArtifactAvailability::Available)
        }
        PersistentArtifactVerificationStatus::Missing => Ok(ArtifactAvailability::Missing),
        PersistentArtifactVerificationStatus::IntegrityFailed => {
            Ok(ArtifactAvailability::IntegrityFailed)
        }
        PersistentArtifactVerificationStatus::Unavailable => match record.status.as_str() {
            "missing" => Ok(ArtifactAvailability::Missing),
            "integrity_failed" => Ok(ArtifactAvailability::IntegrityFailed),
            "available" | "unavailable" => Ok(ArtifactAvailability::Unavailable),
            _ => Err(CorePipelineError::Store(
                StoreError::corrupt_owner_state_value(
                    "artifacts",
                    record.artifact_id.clone(),
                    "status",
                ),
            )),
        },
        PersistentArtifactVerificationStatus::BoundaryViolation => {
            Ok(ArtifactAvailability::Unusable)
        }
    }
}

fn effective_artifact_integrity_status(
    record: &StoredArtifactRecord,
    verification_status: PersistentArtifactVerificationStatus,
) -> CoreResult<ArtifactIntegrityStatus> {
    match verification_status {
        PersistentArtifactVerificationStatus::VerifiedCurrent => {
            Ok(ArtifactIntegrityStatus::Verified)
        }
        PersistentArtifactVerificationStatus::IntegrityFailed
        | PersistentArtifactVerificationStatus::BoundaryViolation => {
            Ok(ArtifactIntegrityStatus::Corrupt)
        }
        PersistentArtifactVerificationStatus::Missing
        | PersistentArtifactVerificationStatus::Unavailable => parse_owner_storage_value(
            "artifacts",
            record.artifact_id.clone(),
            "integrity_status",
            &record.integrity_status,
        ),
    }
}

fn sanitized_artifact_content_type(
    record: &StoredArtifactRecord,
    integrity_status: ArtifactIntegrityStatus,
) -> Option<String> {
    match integrity_status {
        ArtifactIntegrityStatus::Verified => record.content_type.clone(),
        ArtifactIntegrityStatus::Corrupt => record
            .content_type
            .as_ref()
            .filter(|value| !value.trim().is_empty())
            .cloned(),
    }
}

fn sanitized_artifact_sha256(
    record: &StoredArtifactRecord,
    integrity_status: ArtifactIntegrityStatus,
) -> Option<String> {
    match integrity_status {
        ArtifactIntegrityStatus::Verified => record.sha256.clone(),
        ArtifactIntegrityStatus::Corrupt => record
            .sha256
            .as_ref()
            .filter(|value| artifact_sha256_is_lowercase_hex(value))
            .cloned(),
    }
}

fn artifact_sha256_is_lowercase_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn normalize_source_refs(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    field: &'static str,
    refs: &[SourceRef],
) -> Result<Vec<SourceRef>, PlanError> {
    normalize_source_refs_with_carried_artifact_task(
        store,
        project_state,
        envelope,
        task_id,
        field,
        refs,
        None,
    )
}

fn normalize_source_refs_with_carried_artifact_task(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    field: &'static str,
    refs: &[SourceRef],
    carried_artifact_task_id: Option<&TaskId>,
) -> Result<Vec<SourceRef>, PlanError> {
    refs.iter()
        .cloned()
        .map(|source_ref| {
            normalize_source_ref(
                store,
                project_state,
                envelope,
                task_id,
                field,
                source_ref,
                carried_artifact_task_id,
            )
        })
        .collect()
}

fn normalize_source_ref(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    field: &'static str,
    source_ref: SourceRef,
    carried_artifact_task_id: Option<&TaskId>,
) -> Result<SourceRef, PlanError> {
    match source_ref {
        SourceRef::RepositoryFile(mut source) => {
            source.repository_path = match normalize_source_repository_path(&source.repository_path)
            {
                Some(path) => path,
                None => {
                    return source_ref_error(
                        envelope,
                        project_state,
                        field,
                        "repository_path must be a normalized Product Repository relative path",
                    )
                }
            };
            source.baseline_commit_sha = match canonical_git_object_id(&source.baseline_commit_sha)
            {
                Ok(value) => value,
                Err(_) => {
                    return source_ref_error(
                        envelope,
                        project_state,
                        field,
                        "Git object ids must be exactly 40 or 64 ASCII hexadecimal characters",
                    )
                }
            };
            if !artifact_sha256_is_lowercase_hex(&source.content_sha256) {
                return source_ref_error(
                    envelope,
                    project_state,
                    field,
                    "content_sha256 must be a lowercase 64-character SHA-256 hex string",
                );
            }
            if source
                .line_range
                .as_ref()
                .is_some_and(|range| range.start_line == 0 || range.end_line < range.start_line)
            {
                return source_ref_error(
                    envelope,
                    project_state,
                    field,
                    "line_range must be one-based, inclusive, and ordered",
                );
            }
            Ok(SourceRef::RepositoryFile(source))
        }
        SourceRef::GitCommit(mut source) => {
            source.commit_sha = match canonical_git_object_id(&source.commit_sha) {
                Ok(value) => value,
                Err(_) => {
                    return source_ref_error(
                        envelope,
                        project_state,
                        field,
                        "Git object ids must be exactly 40 or 64 ASCII hexadecimal characters",
                    )
                }
            };
            Ok(SourceRef::GitCommit(source))
        }
        SourceRef::GitDiff(mut source) => {
            source.base_commit_sha = match canonical_git_object_id(&source.base_commit_sha) {
                Ok(value) => value,
                Err(_) => {
                    return source_ref_error(
                        envelope,
                        project_state,
                        field,
                        "Git object ids must be exactly 40 or 64 ASCII hexadecimal characters",
                    )
                }
            };
            source.head_commit_sha = match canonical_git_object_id(&source.head_commit_sha) {
                Ok(value) => value,
                Err(_) => {
                    return source_ref_error(
                        envelope,
                        project_state,
                        field,
                        "Git object ids must be exactly 40 or 64 ASCII hexadecimal characters",
                    )
                }
            };
            if let Some(artifact_ref) = source.diff_artifact_ref.as_ref() {
                source.diff_artifact_ref = Some(canonical_source_artifact_ref(
                    store,
                    project_state,
                    envelope,
                    task_id,
                    field,
                    artifact_ref,
                    carried_artifact_task_id,
                )?)
                .into();
            }
            Ok(SourceRef::GitDiff(source))
        }
        SourceRef::Command(mut source) => {
            if source.invocation_id.trim().is_empty() || source.command_summary.trim().is_empty() {
                return source_ref_error(
                    envelope,
                    project_state,
                    field,
                    "command source identifiers and summaries must not be empty",
                );
            }
            source.command_summary = source
                .command_summary
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if let Some(artifact_ref) = source.output_artifact_ref.as_ref() {
                source.output_artifact_ref = Some(canonical_source_artifact_ref(
                    store,
                    project_state,
                    envelope,
                    task_id,
                    field,
                    artifact_ref,
                    carried_artifact_task_id,
                )?)
                .into();
            }
            Ok(SourceRef::Command(source))
        }
        SourceRef::ExternalUri(source) => {
            if !external_source_uri_is_valid(&source.uri) {
                return source_ref_error(
                    envelope,
                    project_state,
                    field,
                    "external_uri must be an absolute http or https URI without user information",
                );
            }
            if !artifact_sha256_is_lowercase_hex(&source.content_sha256) {
                return source_ref_error(
                    envelope,
                    project_state,
                    field,
                    "content_sha256 must be a lowercase 64-character SHA-256 hex string",
                );
            }
            Ok(SourceRef::ExternalUri(source))
        }
        SourceRef::UserContext(source) => {
            if source.context_id.trim().is_empty() {
                return source_ref_error(
                    envelope,
                    project_state,
                    field,
                    "user context ids must not be empty",
                );
            }
            Ok(SourceRef::UserContext(source))
        }
    }
}

fn source_ref_error<T>(
    envelope: &ToolEnvelope,
    project_state: &ProjectStateHeader,
    field: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    let response = validation_rejected(
        envelope.dry_run,
        Some(project_state.state_version),
        field,
        message,
    )
    .map_err(PlanError::Core)?;
    Err(PlanError::Response(Box::new(response)))
}

fn normalize_source_repository_path(raw: &str) -> Option<String> {
    if raw.trim().is_empty() || raw.contains('\\') || has_windows_drive_prefix(raw) {
        return None;
    }
    let path = Path::new(raw);
    if path.is_absolute() {
        return None;
    }
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::Normal(value) => parts.push(value.to_str()?.to_owned()),
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

fn has_windows_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn external_source_uri_is_valid(value: &str) -> bool {
    if value
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return false;
    }
    let Some(rest) = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
    else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    !authority.is_empty() && !authority.contains('@')
}

fn canonical_source_artifact_ref(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    field: &'static str,
    submitted: &ArtifactRef,
    carried_artifact_task_id: Option<&TaskId>,
) -> Result<ArtifactRef, PlanError> {
    let artifact_task_id = if submitted.task_id == *task_id {
        task_id
    } else if carried_artifact_task_id == Some(&submitted.task_id) {
        carried_artifact_task_id.expect("matched carried artifact Task")
    } else {
        return source_ref_error(
            envelope,
            project_state,
            field,
            "source artifact refs must belong to the request Task or the explicitly carried predecessor Task",
        );
    };
    if submitted.project_id != envelope.project_id {
        return source_ref_error(
            envelope,
            project_state,
            field,
            "source artifact refs must belong to the request project",
        );
    }
    let record = store
        .artifact_record(submitted.artifact_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                envelope,
                project_state,
                error,
            )))
        })?;
    let Some(record) = record else {
        return source_ref_error(
            envelope,
            project_state,
            field,
            "source artifact refs must identify an existing artifact",
        );
    };
    let owner_link = store
        .artifact_has_task_owner_link(submitted.artifact_id.as_str(), artifact_task_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                envelope,
                project_state,
                error,
            )))
        })?;
    if record.project_id != envelope.project_id.as_str()
        || record.task_id != artifact_task_id.as_str()
        || !owner_link
    {
        return source_ref_error(
            envelope,
            project_state,
            field,
            "source artifact refs must identify an artifact owned by the request project and Task",
        );
    }
    let integrity_status = parse_owner_storage_value(
        "artifacts",
        record.artifact_id.clone(),
        "integrity_status",
        &record.integrity_status,
    )?;
    let availability = match record.status.as_str() {
        "available" => ArtifactAvailability::Available,
        "missing" => ArtifactAvailability::Missing,
        "integrity_failed" => ArtifactAvailability::IntegrityFailed,
        "unavailable" => ArtifactAvailability::Unavailable,
        _ => {
            return Err(PlanError::Core(CorePipelineError::Store(
                StoreError::corrupt_owner_state_value(
                    "artifacts",
                    record.artifact_id.clone(),
                    "status",
                ),
            )))
        }
    };
    Ok(ArtifactRef {
        artifact_id: ArtifactId::new(record.artifact_id.clone()),
        project_id: envelope.project_id.clone(),
        task_id: artifact_task_id.clone(),
        display_name: record
            .producer
            .display_name
            .clone()
            .unwrap_or_else(|| record.artifact_id.clone()),
        content_type: record.content_type.clone().into(),
        sha256: record.sha256.clone().into(),
        size_bytes: record.size_bytes.into(),
        integrity_status,
        redaction_state: parse_owner_storage_value(
            "artifacts",
            record.artifact_id.clone(),
            "redaction_state",
            &record.redaction_state,
        )?,
        availability,
        created_by_run_ref: Some(state_ref(
            StateRecordKind::Run,
            record.provenance.producer_run_id.as_str(),
            &envelope.project_id,
            Some(artifact_task_id),
            Some(project_state.state_version),
        ))
        .into(),
        created_by_actor_source: Some(record.producer.created_by_actor_source.clone()).into(),
        storage_ref: Some(StorageRef::new(record.uri)).into(),
    })
}

fn decode_required_json<T>(
    table: &'static str,
    record_ref: impl Into<String>,
    logical_column: &'static str,
    raw: Option<&str>,
) -> CoreResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let record_ref = record_ref.into();
    let Some(raw) = raw else {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json(table, record_ref, logical_column),
        ));
    };
    if raw.trim().is_empty() {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json(table, record_ref, logical_column),
        ));
    }
    serde_json::from_str(raw).map_err(|_| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_json(
            table,
            record_ref,
            logical_column,
        ))
    })
}

fn decode_required_json_object(
    table: &'static str,
    record_ref: impl Into<String>,
    logical_column: &'static str,
    raw: Option<&str>,
) -> CoreResult<JsonObject> {
    decode_required_json(table, record_ref, logical_column, raw)
}

fn user_action_authority_from_record(
    record: &EffectiveUserActionRecord,
) -> CoreResult<UserActionAuthority> {
    let request: PersistedUserActionRequest = decode_required_json(
        "user_action_requests",
        record.request.user_action_request_id.clone(),
        "request_json",
        Some(&record.request.request_json),
    )?;
    let basis: UserActionBasis = decode_required_json(
        "user_action_requests",
        record.request.user_action_request_id.clone(),
        "basis_json",
        Some(&record.request.basis_json),
    )?;
    if request.body.action_kind() != record.request.action_kind
        || basis.compatibility_status() != record.request.basis_status
    {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json(
                "user_action_requests",
                record.request.user_action_request_id.clone(),
                "request_json",
            ),
        ));
    }
    let resolution = record
        .resolution
        .as_ref()
        .map(|resolution| {
            let body: PersistedUserActionResolution = decode_required_json(
                "user_action_resolutions",
                resolution.user_action_resolution_id.clone(),
                "resolution_json",
                Some(&resolution.resolution_json),
            )?;
            body.validate().map_err(|_| {
                CorePipelineError::Store(StoreError::corrupt_owner_state_json(
                    "user_action_resolutions",
                    resolution.user_action_resolution_id.clone(),
                    "resolution_json",
                ))
            })?;
            if resolution.action_kind != record.request.action_kind {
                return Err(CorePipelineError::Store(
                    StoreError::corrupt_owner_state_value(
                        "user_action_resolutions",
                        resolution.user_action_resolution_id.clone(),
                        "action_kind",
                    ),
                ));
            }
            Ok(body)
        })
        .transpose()?;
    if record.status == UserActionStatus::Resolved && record.resolution.is_none() {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_value(
                "user_action_requests",
                record.request.user_action_request_id.clone(),
                "resolution",
            ),
        ));
    }
    let (machine_action, resolution_outcome) = match resolution.as_ref() {
        Some(UserActionResolutionBody::Choice {
            machine_action,
            resolution_outcome,
            ..
        }) => (Some(*machine_action), Some(*resolution_outcome)),
        _ => (None, None),
    };
    let affected_refs = request.body.affected_refs().to_vec();
    let expires_at = request.expires_at.into_option();
    let resolution_id = record
        .resolution
        .as_ref()
        .map(|resolution| resolution.user_action_resolution_id.clone());
    let resolved_by_actor_source = record
        .resolution
        .as_ref()
        .map(|resolution| {
            parse_owner_storage_value(
                "user_action_resolutions",
                resolution.user_action_resolution_id.clone(),
                "resolved_by_actor_source",
                &resolution.resolved_by_actor_source,
            )
        })
        .transpose()?;
    Ok(UserActionAuthority {
        user_action_request_id: record.request.user_action_request_id.clone(),
        user_action_resolution_id: resolution_id,
        task_id: TaskId::new(record.request.task_id.clone()),
        action_kind: record.request.action_kind,
        status: record.status,
        required_for: request.required_for,
        affected_refs,
        machine_action,
        resolution_outcome,
        resolved_by_actor_source,
        resolved_verification_basis: record
            .resolution
            .as_ref()
            .map(|resolution| resolution.resolved_verification_basis.clone()),
        resolved_assurance_level: record
            .resolution
            .as_ref()
            .map(|resolution| resolution.resolved_assurance_level.clone()),
        basis_status: record.request.basis_status,
        basis: Some(basis),
        resolution,
        expires_at,
    })
}

fn user_action_authority_from_state(request: &UserActionRequest) -> UserActionAuthority {
    UserActionAuthority {
        user_action_request_id: request.user_action_request_id.as_str().to_owned(),
        user_action_resolution_id: None,
        task_id: request.task_id.clone(),
        action_kind: request.action_kind,
        status: request.status,
        required_for: request.required_for.clone(),
        affected_refs: request.body.affected_refs().to_vec(),
        machine_action: None,
        resolution_outcome: None,
        resolved_by_actor_source: None,
        resolved_verification_basis: None,
        resolved_assurance_level: None,
        basis_status: request.basis.compatibility_status(),
        basis: Some(request.basis.clone()),
        resolution: None,
        expires_at: request.expires_at.as_ref().cloned(),
    }
}

fn resolved_user_action_authorities_for_plan(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    judgment_kind: JudgmentKind,
    now: &UtcTimestamp,
) -> Result<Vec<UserActionAuthority>, PlanError> {
    store
        .resolved_user_action_records(task_id, judgment_kind.into(), now)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                envelope,
                project_state,
                error,
            )))
        })?
        .iter()
        .map(user_action_authority_from_record)
        .collect::<CoreResult<Vec<_>>>()
        .map_err(PlanError::Core)
}

fn user_action_from_record(
    record: &EffectiveUserActionRecord,
    state_version: u64,
) -> CoreResult<UserActionRequest> {
    let persisted: PersistedUserActionRequest = decode_required_json(
        "user_action_requests",
        record.request.user_action_request_id.clone(),
        "request_json",
        Some(&record.request.request_json),
    )?;
    let basis: UserActionBasis = decode_required_json(
        "user_action_requests",
        record.request.user_action_request_id.clone(),
        "basis_json",
        Some(&record.request.basis_json),
    )?;
    if persisted.body.action_kind() != record.request.action_kind
        || basis.compatibility_status() != record.request.basis_status
    {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json(
                "user_action_requests",
                record.request.user_action_request_id.clone(),
                "request_json",
            ),
        ));
    }
    let project_id = ProjectId::new(record.request.project_id.clone());
    let task_id = TaskId::new(record.request.task_id.clone());
    let resolution_ref = record.resolution.as_ref().map(|resolution| {
        state_ref(
            StateRecordKind::UserActionResolution,
            &resolution.user_action_resolution_id,
            &project_id,
            Some(&task_id),
            Some(state_version),
        )
    });
    Ok(UserActionRequest {
        user_action_request_id: UserActionRequestId::new(
            record.request.user_action_request_id.clone(),
        ),
        project_id,
        task_id,
        change_unit_id: record
            .request
            .change_unit_id
            .clone()
            .map(ChangeUnitId::new)
            .into(),
        action_kind: record.request.action_kind,
        status: record.status,
        body: persisted.body,
        basis,
        required_for: persisted.required_for,
        user_action_resolution_ref: resolution_ref.into(),
        expires_at: persisted.expires_at,
        created_at: parse_owner_storage_value(
            "user_action_requests",
            record.request.user_action_request_id.clone(),
            "requested_at",
            &record.request.requested_at,
        )?,
    })
}

fn user_action_inbox_item_from_request(
    record: &EffectiveUserActionRecord,
    request: UserActionRequest,
    state_version: u64,
) -> CoreResult<UserActionInboxItem> {
    let form = request.body.capture_form().map_err(|_| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_json(
            "user_action_requests",
            record.request.user_action_request_id.clone(),
            "request_json",
        ))
    })?;
    let answer_path_availability = user_channel_availability();
    let (preferred_capture_path, fallbacks) =
        user_action_capture_paths(&request.user_action_request_id, request.action_kind);
    let required = request
        .required_for
        .iter()
        .any(|target| *target != UserActionRequiredFor::Informational);
    Ok(UserActionInboxItem {
        user_action_request_id: request.user_action_request_id.clone(),
        request_ref: state_ref(
            StateRecordKind::UserActionRequest,
            request.user_action_request_id.as_str(),
            &request.project_id,
            Some(&request.task_id),
            Some(state_version),
        ),
        project_id: request.project_id,
        task_id: request.task_id,
        change_unit_id: request.change_unit_id,
        action_kind: request.action_kind,
        question: request.body.question().to_owned(),
        context_summary: request.body.context_summary().to_owned(),
        form,
        required,
        requirement_status: if required { "required" } else { "optional" }.to_owned(),
        required_for: request.required_for,
        status: request.status,
        answer_path_availability,
        preferred_capture_path: preferred_capture_path.into(),
        fallbacks,
        expires_at: request.expires_at,
    })
}

fn user_action_capture_paths(
    request_id: &UserActionRequestId,
    action_kind: UserActionKind,
) -> (Option<UserActionCapturePath>, Vec<UserActionCapturePath>) {
    let cli_command = if action_kind == UserActionKind::EvidenceObservation {
        format!(
            "volicord inbox resolve {} (--criterion ID | --claim ID) --artifact ID --summary TEXT",
            request_id.as_str()
        )
    } else {
        format!(
            "volicord inbox resolve {} --choice <choice>",
            request_id.as_str()
        )
    };
    (
        Some(UserActionCapturePath {
            kind: "cli".to_owned(),
            label: "CLI inbox".to_owned(),
            available: true,
            command: Some(cli_command).into(),
            url: RequiredNullable::null(),
            capture_basis: Some(VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL.to_owned()).into(),
            expires_at: RequiredNullable::null(),
            detail: RequiredNullable::null(),
        }),
        Vec::new(),
    )
}

fn user_channel_availability() -> UserChannelAvailability {
    let path = UserChannelPathAvailability {
        kind: "cli".to_owned(),
        label: "CLI inbox".to_owned(),
        available: true,
        status: "available".to_owned(),
        capture_basis: Some(VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL.to_owned()).into(),
        detail: RequiredNullable::null(),
    };
    UserChannelAvailability {
        paths: vec![path.clone()],
        recommended_path_kind: Some(path.kind).into(),
        recommended_path_label: Some(path.label.clone()).into(),
        recommendation: Some(format!(
            "Use {} to resolve pending user actions.",
            path.label
        ))
        .into(),
    }
}

fn pending_user_action_authorities_for_plan(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    now: &UtcTimestamp,
) -> Result<Vec<UserActionAuthority>, PlanError> {
    store
        .pending_user_action_records(task_id, now)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                envelope,
                project_state,
                error,
            )))
        })?
        .iter()
        .map(user_action_authority_from_record)
        .collect::<CoreResult<Vec<_>>>()
        .map_err(PlanError::Core)
}

fn pending_user_action_refs_for_operation(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    now: &UtcTimestamp,
    context: &UserActionOperationContext<'_>,
) -> Result<Vec<StateRecordRef>, PlanError> {
    Ok(pending_user_action_authorities_for_plan(
        store,
        project_state,
        envelope,
        context.task_id,
        now,
    )?
    .iter()
    .filter(|authority| user_action_blocks_operation(authority, context))
    .map(|authority| {
        state_ref(
            StateRecordKind::UserActionRequest,
            &authority.user_action_request_id,
            &envelope.project_id,
            Some(context.task_id),
            Some(project_state.state_version),
        )
    })
    .collect())
}

fn resolved_user_action_authorities_for_all_kinds(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    now: &UtcTimestamp,
) -> Result<Vec<UserActionAuthority>, PlanError> {
    store
        .user_action_records_for_task(task_id, now)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .filter(|record| record.status == UserActionStatus::Resolved)
        .map(|record| user_action_authority_from_record(&record))
        .collect::<CoreResult<Vec<_>>>()
        .map_err(PlanError::Core)
}

fn storage_value<T>(value: T) -> CoreResult<String>
where
    T: serde::Serialize,
{
    match serde_json::to_value(value)? {
        Value::String(value) => Ok(value),
        _ => Err(CorePipelineError::InvalidDispatch {
            detail: "storage value must serialize to a string".to_owned(),
        }),
    }
}

fn validation_plan_error<T>(
    dry_run: bool,
    state_version: Option<u64>,
    field: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    let response =
        validation_rejected(dry_run, state_version, field, message).map_err(PlanError::Core)?;
    Err(PlanError::Response(Box::new(response)))
}

fn checked_derived_expiration(
    created_at: &UtcTimestamp,
    duration: Duration,
    dry_run: bool,
    state_version: Option<u64>,
    field: &'static str,
) -> Result<UtcTimestamp, PlanError> {
    match created_at.checked_add(duration) {
        Ok(expires_at) => Ok(expires_at),
        Err(_) => validation_plan_error(
            dry_run,
            state_version,
            field,
            "derived expiration exceeds the supported canonical RFC 3339 range",
        ),
    }
}

fn mutation_method_policy(
    operation_category: volicord_types::OperationCategory,
    task: TaskRequirement,
    dry_run: bool,
) -> MethodPolicy {
    if dry_run {
        MethodPolicy::exact(
            operation_category,
            task,
            ReplayPolicy::None,
            FreshnessPolicy::IfPresent,
            MethodEffectPolicy::DryRunPreview,
        )
    } else {
        MethodPolicy::exact(
            operation_category,
            task,
            ReplayPolicy::Committed,
            FreshnessPolicy::IfPresent,
            MethodEffectPolicy::CoreMutation,
        )
    }
}

fn redaction_state_value(redaction_state: RedactionState) -> &'static str {
    match redaction_state {
        RedactionState::None => "none",
        RedactionState::Redacted => "redacted",
        RedactionState::SecretOmitted => "secret_omitted",
        RedactionState::Blocked => "blocked",
    }
}

fn resolve_prepare_write_task(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &PrepareWriteRequest,
) -> Result<(TaskId, TaskRecord, Vec<WriteDecisionReason>), PlanError> {
    let task_id = request
        .task_id
        .clone()
        .or_else(|| request.envelope.task_id.as_ref().cloned())
        .or_else(|| project_state.active_task_id.clone().map(TaskId::new))
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    let task = store
        .task_record(&task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;

    let mut reasons = Vec::new();
    if project_state
        .active_task_id
        .as_deref()
        .is_some_and(|active_task_id| active_task_id != task_id.as_str())
    {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Scope,
            "scope_not_current",
            "The addressed Task is not the current Task.",
            vec![state_ref(
                StateRecordKind::Task,
                task_id.as_str(),
                &request.envelope.project_id,
                Some(&task_id),
                Some(project_state.state_version),
            )],
        ));
    }

    Ok((task_id, task, reasons))
}

fn validate_prepare_write_change_unit(
    request: &PrepareWriteRequest,
    task_id: &TaskId,
    current_change_unit: &ChangeUnitRecord,
    reasons: &mut Vec<WriteDecisionReason>,
) {
    if request
        .change_unit_id
        .as_ref()
        .is_some_and(|change_unit_id| change_unit_id.as_str() != current_change_unit.change_unit_id)
    {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Scope,
            "scope_not_current",
            "The addressed Change Unit is not the current Change Unit.",
            vec![change_unit_ref(
                &request.envelope.project_id,
                task_id,
                current_change_unit,
                current_change_unit.basis_state_version,
            )],
        ));
    }
}

fn baseline_matches(
    change_unit: &ChangeUnitRecord,
    task: &TaskRecord,
    baseline_ref: &BaselineRef,
) -> CoreResult<bool> {
    let write_basis: PersistedWriteBasis = decode_required_json(
        "change_units",
        change_unit.change_unit_id.clone(),
        "write_basis_json",
        Some(&change_unit.write_basis_json),
    )?;
    let task_baseline = StoredScope::from_task(task)?.baseline_ref;
    Ok(
        write_basis.baseline_ref.as_ref().map(BaselineRef::as_str) == Some(baseline_ref.as_str())
            && task_baseline.as_deref() == Some(baseline_ref.as_str()),
    )
}

fn workspace_context_matches(
    change_unit: &ChangeUnitRecord,
    verified_invocation: &VerifiedInvocationContext,
) -> CoreResult<bool> {
    let basis: PersistedWriteBasis = decode_required_json(
        "change_units",
        change_unit.change_unit_id.clone(),
        "write_basis_json",
        Some(&change_unit.write_basis_json),
    )?;
    Ok(basis.git_workspace_context == verified_invocation.git_workspace_context)
}

fn paths_match_current_change_unit(
    repo_root: &Path,
    intended_paths: &[String],
    change_unit: &ChangeUnitRecord,
) -> CoreResult<bool> {
    if intended_paths.is_empty() {
        return Ok(true);
    }
    let raw_bounded_paths: Vec<String> = decode_required_json(
        "change_units",
        change_unit.change_unit_id.clone(),
        "bounded_paths_json",
        Some(&change_unit.bounded_paths_json),
    )?;
    if raw_bounded_paths.is_empty() {
        return Ok(false);
    }
    let bounded_paths = normalize_product_paths(repo_root, &raw_bounded_paths).map_err(|_| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_json(
            "change_units",
            change_unit.change_unit_id.clone(),
            "bounded_paths_json",
        ))
    })?;
    Ok(!bounded_paths.is_empty()
        && intended_paths.iter().all(|path| {
            bounded_paths
                .iter()
                .any(|scope| path_is_within(path, scope))
        }))
}

fn change_unit_effect_contract(
    change_unit: &ChangeUnitRecord,
) -> CoreResult<Option<ChangeUnitEffectContract>> {
    decode_required_json(
        "change_units",
        change_unit.change_unit_id.clone(),
        "effect_contract_json",
        Some(&change_unit.effect_contract_json),
    )
}

struct SensitiveApprovalSearch<'a> {
    store: &'a CoreProjectStore<'a>,
    project_state: &'a ProjectStateHeader,
    request: &'a PrepareWriteRequest,
    task_id: &'a TaskId,
    task: &'a TaskRecord,
    change_unit: &'a ChangeUnitRecord,
    intended_operation: &'a str,
    normalized_paths: &'a [String],
    sensitive_categories: &'a [String],
    now: &'a UtcTimestamp,
}

fn matching_sensitive_approval(
    search: SensitiveApprovalSearch<'_>,
) -> Result<Option<EffectiveUserActionRecord>, PlanError> {
    let SensitiveApprovalSearch {
        store,
        project_state,
        request,
        task_id,
        task,
        change_unit,
        intended_operation,
        normalized_paths,
        sensitive_categories,
        now,
    } = search;
    let records = store
        .resolved_user_action_records(task_id, UserActionKind::SensitiveApproval, now)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let change_unit_id = ChangeUnitId::new(change_unit.change_unit_id.clone());
    let requirement = SensitiveApprovalRequirement {
        task_id,
        change_unit_id: &change_unit_id,
        scope_revision: task.scope_revision,
        operation: intended_operation,
        normalized_paths,
        sensitive_categories,
        baseline_ref: Some(&request.baseline_ref),
        required_for: UserActionRequiredFor::PrepareWrite,
        now,
        repo_root: &store.project_record().repo_root,
    };

    for record in records {
        let authority = user_action_authority_from_record(&record)?;
        if current_sensitive_approval(&authority, &requirement) {
            return Ok(Some(record));
        }
    }

    Ok(None)
}

fn change_unit_ref(
    project_id: &ProjectId,
    task_id: &TaskId,
    change_unit: &ChangeUnitRecord,
    state_version: u64,
) -> StateRecordRef {
    state_ref(
        StateRecordKind::ChangeUnit,
        &change_unit.change_unit_id,
        project_id,
        Some(task_id),
        Some(state_version),
    )
}

fn sorted_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn unique_state_refs(values: Vec<StateRecordRef>) -> Vec<StateRecordRef> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for value in values {
        let key = state_record_ref_identity_key(&value);
        if seen.insert(key) {
            unique.push(value);
        }
    }
    unique
}

fn artifact_input_validation_plan_error<T>(
    request: &RecordRunRequest,
    project_state: &ProjectStateHeader,
    input: &ArtifactInput,
    reason: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    Err(PlanError::Response(Box::new(
        artifact_input_validation_response(request, project_state, input, reason, message),
    )))
}

fn artifact_input_validation_response(
    request: &RecordRunRequest,
    project_state: &ProjectStateHeader,
    input: &ArtifactInput,
    reason: &'static str,
    message: &'static str,
) -> PipelineResponse {
    let details = object_from_value(json!({
        "artifact_input_error": {
            "artifact_input_id": input.artifact_input_id.as_str(),
            "reason": reason
        }
    }))
    .expect("artifact input error details should be an object");
    infallible_rejected_pipeline_response(
        request.envelope.dry_run,
        Some(project_state.state_version),
        vec![tool_error(
            ErrorCode::ValidationFailed,
            message,
            false,
            Some(details),
        )],
    )
}

fn artifact_missing_response(
    request: &RecordRunRequest,
    project_state: &ProjectStateHeader,
    message: &'static str,
) -> PipelineResponse {
    infallible_rejected_pipeline_response(
        request.envelope.dry_run,
        Some(project_state.state_version),
        vec![tool_error(ErrorCode::ArtifactMissing, message, false, None)],
    )
}

fn write_ticket_required_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
) -> PipelineResponse {
    let details = object_from_value(json!({
        "write_ticket_reason": "missing"
    }))
    .expect("write ticket details should be an object");
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::WriteTicketRequired,
            "product-file write observations require a compatible active write ticket",
            false,
            Some(details),
        )],
    )
}

fn write_ticket_invalid_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    reason: &'static str,
    message: &'static str,
) -> PipelineResponse {
    let details = object_from_value(json!({
        "write_ticket_reason": reason
    }))
    .expect("write ticket details should be an object");
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::WriteTicketInvalid,
            message,
            false,
            Some(details),
        )],
    )
}

fn baseline_stale_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    baseline_ref: &BaselineRef,
) -> PipelineResponse {
    let details = object_from_value(json!({
        "baseline_ref": baseline_ref.as_str()
    }))
    .expect("baseline details should be an object");
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::BaselineStale,
            "baseline_ref does not match the current Change Unit basis",
            true,
            Some(details),
        )],
    )
}

fn workspace_stale_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
) -> PipelineResponse {
    let details = object_from_value(json!({
        "workspace_reason": "workspace_context_mismatch"
    }))
    .expect("workspace details should be an object");
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::BaselineStale,
            "current Git workspace context does not match the current Change Unit basis",
            true,
            Some(details),
        )],
    )
}

fn no_active_change_unit_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    message: &'static str,
) -> PipelineResponse {
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::NoActiveChangeUnit,
            message,
            false,
            None,
        )],
    )
}

fn decision_rejected_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    message: &'static str,
) -> PipelineResponse {
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::DecisionUnresolved,
            message,
            false,
            None,
        )],
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredScope {
    goal_summary: Option<String>,
    scope_summary: Option<String>,
    non_goals: Vec<String>,
    autonomy_boundary: Option<String>,
    baseline_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PersistedTaskShaping {
    #[serde(default)]
    goal_summary: Option<String>,
    #[serde(default)]
    scope_summary: Option<String>,
    #[serde(default)]
    non_goals: Vec<String>,
    #[serde(default)]
    baseline_ref: Option<String>,
    #[serde(default)]
    autonomy_boundary: Option<String>,
    #[serde(default)]
    initial_context_refs: Option<Value>,
    #[serde(default)]
    initial_source_refs: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedAutonomyBoundary {
    #[serde(default)]
    autonomy_boundary: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PersistedScopeSummary {
    #[serde(default)]
    scope_summary: Option<String>,
    #[serde(default)]
    affected_areas: Vec<String>,
    #[serde(default)]
    constraints: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedWriteBasis {
    #[serde(default)]
    baseline_ref: Option<BaselineRef>,
    #[serde(default)]
    git_workspace_context: Option<crate::pipeline::GitWorkspaceContext>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedLifecycleState {
    #[serde(default)]
    recovery_required: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedWriteTicketAttemptScope {
    task_id: TaskId,
    change_unit_id: ChangeUnitId,
    intended_operation: String,
    intended_paths: Vec<String>,
    product_file_write_intended: bool,
    sensitive_categories: Vec<String>,
    baseline_ref: Option<BaselineRef>,
}

impl From<PersistedWriteTicketAttemptScope> for WriteTicketAttemptScope {
    fn from(scope: PersistedWriteTicketAttemptScope) -> Self {
        Self {
            task_id: scope.task_id,
            change_unit_id: scope.change_unit_id,
            intended_operation: scope.intended_operation,
            intended_paths: scope.intended_paths,
            product_file_write_intended: scope.product_file_write_intended,
            sensitive_categories: scope.sensitive_categories,
            baseline_ref: scope.baseline_ref,
        }
    }
}

impl StoredScope {
    fn from_task(task: &TaskRecord) -> CoreResult<Self> {
        let shaping: PersistedTaskShaping = decode_required_json(
            "tasks",
            task.task_id.clone(),
            "shaping_summary_json",
            Some(&task.shaping_summary_json),
        )?;
        let autonomy: PersistedAutonomyBoundary = decode_required_json(
            "tasks",
            task.task_id.clone(),
            "autonomy_boundary_json",
            Some(&task.autonomy_boundary_json),
        )?;
        Ok(Self::normalized(Self {
            goal_summary: shaping.goal_summary.or_else(|| task.summary.clone()),
            scope_summary: shaping.scope_summary,
            non_goals: shaping.non_goals,
            autonomy_boundary: autonomy.autonomy_boundary.or(shaping.autonomy_boundary),
            baseline_ref: shaping.baseline_ref,
        }))
    }

    fn apply_request(&self, request: &UpdateScopeRequest) -> Self {
        Self {
            goal_summary: request
                .goal_summary
                .clone()
                .or_else(|| self.goal_summary.clone()),
            scope_summary: request
                .scope_boundary
                .clone()
                .or_else(|| self.scope_summary.clone()),
            non_goals: request
                .non_goals
                .clone()
                .unwrap_or_else(|| self.non_goals.clone()),
            autonomy_boundary: request
                .autonomy_boundary
                .clone()
                .or_else(|| self.autonomy_boundary.clone()),
            baseline_ref: request
                .baseline_ref
                .as_ref()
                .map(|value| value.as_str().to_owned())
                .or_else(|| self.baseline_ref.clone()),
        }
        .normalized()
    }

    fn normalized(mut self) -> Self {
        self.goal_summary = normalize_scope_text_option(self.goal_summary);
        self.scope_summary = normalize_scope_text_option(self.scope_summary);
        self.non_goals = normalize_scope_string_list(self.non_goals);
        self.autonomy_boundary = normalize_scope_text_option(self.autonomy_boundary);
        self.baseline_ref = normalize_scope_text_option(self.baseline_ref);
        self
    }

    fn to_json(&self) -> Value {
        task_shaping_json(
            self.goal_summary.clone(),
            self.scope_summary.clone(),
            self.non_goals.clone(),
            self.baseline_ref.clone(),
            self.autonomy_boundary.clone(),
            None,
        )
    }
}

fn normalize_scope_text_option(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn normalize_display_text(value: &str) -> String {
    value.trim().to_owned()
}

fn normalize_scope_string_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .filter_map(|value| normalize_scope_text_option(Some(value)))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn acceptance_criterion_from_record(
    record: &AcceptanceCriterionRecord,
) -> CoreResult<AcceptanceCriterion> {
    Ok(AcceptanceCriterion {
        acceptance_criterion_id: AcceptanceCriterionId::new(record.acceptance_criterion_id.clone()),
        statement: record.statement.clone(),
        evidence_requirement: parse_owner_storage_value(
            "acceptance_criteria",
            record.acceptance_criterion_id.clone(),
            "evidence_requirement",
            &record.evidence_requirement,
        )?,
    })
}

fn active_acceptance_criteria_for_task(
    store: &CoreProjectStore,
    task_id: &TaskId,
) -> CoreResult<Vec<AcceptanceCriterion>> {
    store
        .active_acceptance_criteria(task_id)
        .map_err(CorePipelineError::from)?
        .iter()
        .map(acceptance_criterion_from_record)
        .collect()
}

fn agent_safe_pending_user_action_summaries(
    refs: impl IntoIterator<Item = StateRecordRef>,
) -> Vec<AgentSafeUserActionRequestSummary> {
    refs.into_iter()
        .map(|record_ref| {
            AgentSafeUserActionRequestSummary::pending(UserActionRequestId::new(
                record_ref.record_id.as_str(),
            ))
        })
        .collect()
}

struct SummaryBuild<'a> {
    store: &'a CoreProjectStore<'a>,
    project_id: &'a ProjectId,
    state_version: u64,
    task: &'a TaskRecord,
    current_change_unit: Option<&'a ChangeUnitRecord>,
    acceptance_criteria: Vec<AcceptanceCriterion>,
    pending_user_action_refs: Vec<StateRecordRef>,
    blocker_refs: Vec<StateRecordRef>,
    write_ticket_summary: Option<WriteTicketStateSummary>,
    evidence_summary: Option<EvidenceSummary>,
    evidence_gate: Option<EvidenceGateSummary>,
    close_state: Option<CloseState>,
    close_blockers: Vec<CloseReadinessBlocker>,
    guarantee_display: Option<GuaranteeDisplay>,
}

fn build_state_summary(input: SummaryBuild<'_>) -> CoreResult<volicord_types::StateSummary> {
    let SummaryBuild {
        store,
        project_id,
        state_version,
        task,
        current_change_unit,
        acceptance_criteria,
        pending_user_action_refs,
        blocker_refs,
        write_ticket_summary,
        evidence_summary,
        evidence_gate,
        close_state,
        close_blockers,
        guarantee_display,
    } = input;
    let workflow_policy = project_workflow_policy(store).map_err(CorePipelineError::from)?;
    let task_id = TaskId::new(task.task_id.clone());
    let task_ref = state_ref(
        StateRecordKind::Task,
        &task.task_id,
        project_id,
        Some(&task_id),
        Some(state_version),
    );
    let active_change_unit_ref = current_change_unit.map(|record| {
        state_ref(
            StateRecordKind::ChangeUnit,
            &record.change_unit_id,
            project_id,
            Some(&task_id),
            Some(record.basis_state_version),
        )
    });
    let effect_contract = current_change_unit
        .map(change_unit_effect_contract)
        .transpose()?
        .flatten();
    let workspace_context = current_change_unit
        .map(|record| {
            decode_required_json::<PersistedWriteBasis>(
                "change_units",
                record.change_unit_id.clone(),
                "write_basis_json",
                Some(&record.write_basis_json),
            )
            .map(|basis| {
                basis
                    .git_workspace_context
                    .map(|workspace| WorkspaceContext {
                        vcs: WorkspaceVcs::Git,
                        git_common_dir: workspace.git_common_dir,
                        worktree_id: workspace.worktree_id,
                        branch_ref: workspace.branch_ref,
                        head_sha: workspace.head_sha,
                        workspace_fingerprint: workspace.workspace_fingerprint,
                    })
            })
        })
        .transpose()?
        .flatten();
    let lineage = match (
        task.predecessor_task_id.as_ref(),
        task.lineage_relation.as_deref(),
        task.lineage_reason.as_ref(),
    ) {
        (Some(predecessor_task_id), Some(relation), Some(creation_reason)) => {
            let dispositions: Vec<CarryForwardDisposition> = decode_required_json(
                "tasks",
                task.task_id.clone(),
                "carry_forward_json",
                Some(&task.carry_forward_json),
            )?;
            Some(TaskLineageSummary {
                predecessor_task_ref: state_ref(
                    StateRecordKind::Task,
                    predecessor_task_id,
                    project_id,
                    Some(&TaskId::new(predecessor_task_id.clone())),
                    Some(state_version),
                ),
                relation: parse_task_lineage_relation(relation)?,
                creation_reason: creation_reason.clone(),
                carry_forward: dispositions,
            })
        }
        (None, None, None) => None,
        _ => return invalid_storage("tasks.lineage"),
    };
    let scope = StoredScope::from_task(task)?;
    let change_unit_scope = current_change_unit
        .map(|record| {
            decode_required_json::<PersistedScopeSummary>(
                "change_units",
                record.change_unit_id.clone(),
                "scope_summary_json",
                Some(&record.scope_summary_json),
            )
            .map(|summary| summary.scope_summary)
        })
        .transpose()?
        .flatten();
    Ok(volicord_types::StateSummary {
        project_id: project_id.clone(),
        state_version,
        task_ref: Some(task_ref),
        mode: Some(parse_task_mode(&task.mode)?),
        requested_control_level: Some(
            parse_requested_control_level(&task.requested_control_level)
                .map_err(CorePipelineError::from)?,
        ),
        effective_control_level: Some(
            parse_task_control_level(&task.effective_control_level)
                .map_err(CorePipelineError::from)?,
        ),
        control_level_reason: Some(task.control_level_reason.clone()),
        project_policy: workflow_policy.summary,
        work_phase: Some(parse_work_phase(&task.work_phase)?),
        acceptance_policy: Some(parse_acceptance_policy(&task.acceptance_policy)?),
        acceptance_policy_reason: Some(task.acceptance_policy_reason.clone()),
        lineage,
        lifecycle: Some(TaskLifecycleState {
            lifecycle_phase: parse_lifecycle_phase(&task.lifecycle_phase)?,
            close_reason: parse_close_reason(task)?,
            result: parse_task_result(task.result.as_deref().unwrap_or("none"))?,
            closed_at: task
                .closed_at
                .as_ref()
                .map(|closed_at| {
                    parse_owner_storage_value("tasks", task.task_id.clone(), "closed_at", closed_at)
                })
                .transpose()?,
        }),
        scope_revision: task.scope_revision,
        goal_summary: scope.goal_summary,
        scope_summary: change_unit_scope.or(scope.scope_summary),
        non_goals: scope.non_goals,
        acceptance_criteria,
        autonomy_boundary: scope.autonomy_boundary,
        active_change_unit_ref,
        effect_contract,
        baseline_ref: scope.baseline_ref.map(BaselineRef::new),
        workspace_context,
        shaping_readiness: None,
        pending_user_action_summaries: agent_safe_pending_user_action_summaries(
            pending_user_action_refs,
        ),
        blocker_refs,
        write_ticket_summary,
        evidence_summary,
        evidence_gate,
        close_state,
        close_blockers,
        guarantee_display,
    })
}

fn write_ticket_summary_for_record(
    store: Option<&CoreProjectStore>,
    record: &WriteTicketRecord,
    state_version: u64,
    now: Option<DateTime<Utc>>,
    observation_refs: Option<Vec<StateRecordRef>>,
    guarantee_display: Option<GuaranteeDisplay>,
) -> CoreResult<WriteTicketStateSummary> {
    let attempt_scope = decode_required_json::<PersistedWriteTicketAttemptScope>(
        "write_tickets",
        record.write_ticket_id.clone(),
        "attempt_scope_json",
        Some(&record.attempt_scope_json),
    )?;
    let consumed_by_run_ref = record.consumed_by_run_id.as_ref().map(|run_id| {
        state_ref(
            StateRecordKind::Run,
            run_id,
            &ProjectId::new(record.project_id.clone()),
            Some(&TaskId::new(record.task_id.clone())),
            Some(state_version),
        )
    });
    let observation_refs = match (observation_refs, record.consumed_by_run_id.as_ref(), store) {
        (Some(refs), _, _) => refs,
        (None, Some(run_id), Some(store)) => stored_refs_to_state_refs(
            store
                .evidence_observation_refs_for_run(
                    &TaskId::new(record.task_id.clone()),
                    run_id,
                    state_version,
                )
                .map_err(CorePipelineError::from)?,
        ),
        _ => Vec::new(),
    };
    let mut effective_status = effective_write_ticket_status(record, state_version, now)?;
    let mut effective_invalidation_reason =
        effective_write_ticket_invalidation_reason(record, now)?;
    if effective_status == WriteTicketStatus::Active {
        if let (Some(store), Some(now)) = (store, now) {
            if let Some(reason) = write_ticket_projection_invalidation_reason(store, record, now)? {
                effective_status = WriteTicketStatus::Invalidated;
                effective_invalidation_reason = Some(reason);
            }
        }
    }
    Ok(WriteTicketStateSummary {
        status: effective_status,
        write_ticket_ref: Some(write_ticket_ref(record, state_version)),
        basis_state_version: Some(record.basis_state_version),
        validity_basis: Some(decode_required_json(
            "write_tickets",
            record.write_ticket_id.clone(),
            "validity_basis_json",
            Some(&record.validity_basis_json),
        )?),
        invalidation_reason: effective_invalidation_reason,
        idle_expires_at: record
            .idle_expires_at
            .as_ref()
            .map(|value| {
                parse_owner_storage_value(
                    "write_tickets",
                    record.write_ticket_id.clone(),
                    "idle_expires_at",
                    value,
                )
            })
            .transpose()?,
        intended_paths: attempt_scope.intended_paths,
        consumed_by_run_ref,
        observation_refs,
        guarantee_display,
    })
}

fn effective_write_ticket_status(
    record: &WriteTicketRecord,
    _state_version: u64,
    now: Option<DateTime<Utc>>,
) -> CoreResult<WriteTicketStatus> {
    let stored_status = parse_storage_value("write_tickets.status", &record.status)?;
    if stored_status != WriteTicketStatus::Active {
        return Ok(stored_status);
    }
    if now
        .map(|now| write_ticket_is_idle_expired(record, now))
        .transpose()
        .map_err(CorePipelineError::from)?
        .unwrap_or(false)
    {
        Ok(WriteTicketStatus::Invalidated)
    } else {
        Ok(WriteTicketStatus::Active)
    }
}

fn effective_write_ticket_invalidation_reason(
    record: &WriteTicketRecord,
    now: Option<DateTime<Utc>>,
) -> CoreResult<Option<WriteTicketInvalidationReason>> {
    if record.status == "active"
        && now
            .map(|now| write_ticket_is_idle_expired(record, now))
            .transpose()
            .map_err(CorePipelineError::from)?
            .unwrap_or(false)
    {
        return Ok(Some(WriteTicketInvalidationReason::IdleTimeout));
    }
    record
        .invalidation_reason
        .as_deref()
        .map(|value| parse_storage_value("write_tickets.invalidation_reason", value))
        .transpose()
}

fn write_ticket_projection_invalidation_reason(
    store: &CoreProjectStore,
    record: &WriteTicketRecord,
    now: DateTime<Utc>,
) -> CoreResult<Option<WriteTicketInvalidationReason>> {
    let validity_basis: WriteTicketValidityBasis = decode_required_json(
        "write_tickets",
        record.write_ticket_id.clone(),
        "validity_basis_json",
        Some(&record.validity_basis_json),
    )?;
    let scope: WriteTicketAttemptScope = decode_required_json::<PersistedWriteTicketAttemptScope>(
        "write_tickets",
        record.write_ticket_id.clone(),
        "attempt_scope_json",
        Some(&record.attempt_scope_json),
    )?
    .into();
    let task = store
        .task_record(&validity_basis.task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            CorePipelineError::Store(StoreError::NotFound {
                entity: "task",
                id: validity_basis.task_id.as_str().to_owned(),
            })
        })?;
    let workflow_policy = project_workflow_policy(store).map_err(CorePipelineError::from)?;
    if validity_basis.write_authority_fingerprint != workflow_policy.write_authority_fingerprint {
        return Ok(Some(WriteTicketInvalidationReason::ExplicitRevoke));
    }
    let resolved_control =
        resolve_task_control_authority(&task, &workflow_policy).map_err(CorePipelineError::from)?;
    if resolved_control.pending_policy_reevaluation {
        return Ok(Some(WriteTicketInvalidationReason::ExplicitRevoke));
    }
    if validity_basis.approval_basis_refs.is_empty() {
        return Ok((!scope.sensitive_categories.is_empty()
            || resolved_control.effective_control_level == TaskControlLevel::Sensitive)
            .then_some(WriteTicketInvalidationReason::ApprovalBasisChanged));
    }

    let now = UtcTimestamp::from_datetime(now);
    let requirement = SensitiveApprovalRequirement {
        task_id: &validity_basis.task_id,
        change_unit_id: &validity_basis.change_unit_id,
        scope_revision: task.scope_revision,
        operation: &scope.intended_operation,
        normalized_paths: &scope.intended_paths,
        sensitive_categories: &scope.sensitive_categories,
        baseline_ref: scope.baseline_ref.as_ref(),
        required_for: UserActionRequiredFor::PrepareWrite,
        now: &now,
        repo_root: &store.project_record().repo_root,
    };
    let records = store
        .resolved_user_action_records(
            &validity_basis.task_id,
            UserActionKind::SensitiveApproval,
            &now,
        )
        .map_err(CorePipelineError::from)?;
    let mut current_resolution_ids = BTreeSet::new();
    for record in records {
        let authority = user_action_authority_from_record(&record)?;
        if current_sensitive_approval(&authority, &requirement) {
            if let Some(resolution_id) = authority.user_action_resolution_id {
                current_resolution_ids.insert(resolution_id);
            }
        }
    }
    let approval_basis_is_current = !current_resolution_ids.is_empty()
        && validity_basis.approval_basis_refs.iter().all(|stored| {
            stored.record_kind == StateRecordKind::UserActionResolution
                && current_resolution_ids.contains(stored.record_id.as_str())
        });
    Ok((!approval_basis_is_current).then_some(WriteTicketInvalidationReason::ApprovalBasisChanged))
}

fn write_ticket_is_current_for_projection(
    store: &CoreProjectStore,
    record: &WriteTicketRecord,
    now: DateTime<Utc>,
) -> CoreResult<bool> {
    Ok(write_ticket_projection_invalidation_reason(store, record, now)?.is_none())
}

fn guarantee_display_for_invocation(
    store: &CoreProjectStore,
    verified_invocation: &VerifiedInvocationContext,
    state_version: u64,
) -> Result<GuaranteeDisplay, PlanError> {
    let profile = store
        .project_enforcement_profile()
        .map_err(CorePipelineError::from)?
        .profile;
    Ok(guarantee_display_from_profile(
        &profile,
        verified_invocation,
        state_version,
    ))
}

fn guarantee_display_from_profile(
    profile: &ProjectEnforcementProfile,
    verified_invocation: &VerifiedInvocationContext,
    state_version: u64,
) -> GuaranteeDisplay {
    GuaranteeDisplay {
        level: profile.guarantee_level,
        basis: format!(
            "Project enforcement profile `{}` is active for actor source `{}` operation category `{}` verified by `{}`; enabled mechanisms: none; no stronger enforcement is active.",
            profile.profile_id,
            verified_invocation.actor_source,
            verified_invocation.operation_category.as_str(),
            verified_invocation.verification_basis
        ),
        capability_refs: vec![invocation_binding_ref(verified_invocation, state_version)],
    }
}

fn invocation_binding_ref(
    verified_invocation: &VerifiedInvocationContext,
    state_version: u64,
) -> StateRecordRef {
    match &verified_invocation.actor_source {
        ActorSource::AgentConnection(connection_id) => state_ref(
            StateRecordKind::AgentConnection,
            connection_id.as_str(),
            &verified_invocation.project_id,
            None,
            Some(state_version),
        ),
        ActorSource::LocalUser | ActorSource::System => state_ref(
            StateRecordKind::ProjectState,
            verified_invocation
                .actor_source
                .to_canonical_string()
                .as_str(),
            &verified_invocation.project_id,
            None,
            Some(state_version),
        ),
    }
}

fn selected_write_ticket_for_projection(
    store: &CoreProjectStore,
    task_id: &TaskId,
    state_version: u64,
    now: DateTime<Utc>,
) -> Result<Option<WriteTicketRecord>, PlanError> {
    let records = store
        .write_tickets_for_task(task_id)
        .map_err(CorePipelineError::from)?;
    let mut selected = None;
    let mut selected_priority = u8::MAX;
    for record in records {
        let mut status = effective_write_ticket_status(&record, state_version, Some(now))?;
        if status == WriteTicketStatus::Active
            && !write_ticket_is_current_for_projection(store, &record, now)?
        {
            status = WriteTicketStatus::Invalidated;
        }
        let priority = match status {
            WriteTicketStatus::Active => 0,
            WriteTicketStatus::Invalidated => 1,
            WriteTicketStatus::Consumed => 2,
            WriteTicketStatus::Revoked => 3,
        };
        if priority < selected_priority {
            selected_priority = priority;
            selected = Some(record);
        }
    }
    Ok(selected)
}

fn projected_write_ticket_summary(
    store: &CoreProjectStore,
    task_id: &TaskId,
    state_version: u64,
    now: DateTime<Utc>,
    guarantee_display: Option<GuaranteeDisplay>,
) -> Result<Option<WriteTicketStateSummary>, PlanError> {
    Ok(
        selected_write_ticket_for_projection(store, task_id, state_version, now)?
            .as_ref()
            .map(|record| {
                write_ticket_summary_for_record(
                    Some(store),
                    record,
                    state_version,
                    Some(now),
                    None,
                    guarantee_display,
                )
            })
            .transpose()?,
    )
}

fn projected_evidence_summary(
    store: &CoreProjectStore,
    project_id: &ProjectId,
    state_version: u64,
    task: &TaskRecord,
) -> Result<Option<EvidenceSummary>, PlanError> {
    let task_id = TaskId::new(task.task_id.clone());
    let record = store
        .latest_evidence_summary(&task_id)
        .map_err(CorePipelineError::from)?;
    Ok(close_task::close_evidence_summary(
        store,
        record.as_ref(),
        task,
        project_id,
        &task_id,
        state_version,
    )?)
}

fn projected_evidence_summary_for_criteria(
    store: &CoreProjectStore,
    project_id: &ProjectId,
    state_version: u64,
    task: &TaskRecord,
    acceptance_criteria: &[AcceptanceCriterion],
) -> Result<Option<EvidenceSummary>, PlanError> {
    let task_id = TaskId::new(task.task_id.clone());
    let record = store
        .latest_evidence_summary(&task_id)
        .map_err(CorePipelineError::from)?;
    let required = required_acceptance_criterion_ids(acceptance_criteria);
    Ok(close_task::close_evidence_summary_with_required(
        store,
        record.as_ref(),
        task,
        project_id,
        &task_id,
        state_version,
        &required,
    )?)
}

fn projected_pending_user_action_refs(
    store: &CoreProjectStore,
    task_id: &TaskId,
    state_version: u64,
    now: &UtcTimestamp,
) -> Result<Vec<StateRecordRef>, PlanError> {
    Ok(stored_refs_to_state_refs(
        store
            .pending_user_action_refs(task_id, state_version, now)
            .map_err(CorePipelineError::from)?,
    ))
}

fn projected_blocker_refs(
    store: &CoreProjectStore,
    task_id: &TaskId,
    state_version: u64,
) -> Result<Vec<StateRecordRef>, PlanError> {
    Ok(stored_refs_to_state_refs(
        store
            .active_blocker_refs(task_id, state_version)
            .map_err(CorePipelineError::from)?,
    ))
}

fn projected_close_basis(
    store: &CoreProjectStore,
    task_id: &TaskId,
) -> Result<Option<CurrentCloseBasis>, PlanError> {
    Ok(store
        .task_revision_record(task_id)
        .map_err(CorePipelineError::from)?
        .and_then(|record| record.current_close_basis))
}

fn project_state_projection(
    project_state: &ProjectStateHeader,
    state_version: u64,
    active_task_id: Option<String>,
) -> ProjectStateHeader {
    ProjectStateHeader {
        project_id: project_state.project_id.clone(),
        state_version,
        active_task_id,
        updated_at: project_state.updated_at.clone(),
    }
}

fn close_context_from_projection(
    task: TaskRecord,
    current_change_unit: Option<ChangeUnitRecord>,
    current_close_basis: Option<CurrentCloseBasis>,
    pending_user_action_refs: Vec<StateRecordRef>,
    blocker_refs: Vec<StateRecordRef>,
    evidence_summary: Option<EvidenceSummary>,
    now: UtcTimestamp,
) -> CloseTaskContext {
    let artifact_refs = evidence_summary
        .as_ref()
        .map(|summary| summary.artifact_refs.clone())
        .unwrap_or_default();
    CloseTaskContext {
        now,
        task,
        current_change_unit,
        current_close_basis,
        pending_user_action_refs,
        blocker_refs,
        evidence_summary,
        artifact_refs,
        projected_run_refs: Vec::new(),
        projected_evidence_observations: Vec::new(),
        projected_artifacts: Vec::new(),
        projected_required_criterion_ids: None,
        projected_resolved_unrecorded_change_ids: BTreeSet::new(),
        pending_user_action_authorities: None,
        resolved_judgment_authorities: None,
    }
}

fn close_context_with_projected_acceptance_criteria(
    mut context: CloseTaskContext,
    acceptance_criteria: &[AcceptanceCriterion],
) -> CloseTaskContext {
    context.projected_required_criterion_ids =
        Some(required_acceptance_criterion_ids(acceptance_criteria));
    context
}

fn required_acceptance_criterion_ids(
    acceptance_criteria: &[AcceptanceCriterion],
) -> BTreeSet<String> {
    acceptance_criteria
        .iter()
        .filter(|criterion| criterion.evidence_requirement == EvidenceRequirement::Required)
        .map(|criterion| criterion.acceptance_criterion_id.as_str().to_owned())
        .collect()
}

fn evidence_summary_with_required_criteria(
    summary: Option<EvidenceSummary>,
    acceptance_criteria: &[AcceptanceCriterion],
) -> Option<EvidenceSummary> {
    let required = required_acceptance_criterion_ids(acceptance_criteria);
    if summary.is_none() && required.is_empty() {
        return None;
    }
    let mut summary = summary.unwrap_or(EvidenceSummary {
        evidence_state: None,
        status: EvidenceStatus::Unknown,
        coverage_items: Vec::new(),
        artifact_refs: Vec::new(),
        observation_refs: Vec::new(),
        updated_by_run_ref: None,
    });
    for acceptance_criterion_id in required {
        if !summary.coverage_items.iter().any(|item| {
            matches!(
                &item.target,
                EvidenceTarget::AcceptanceCriterion {
                    acceptance_criterion_id: existing
                } if existing.as_str() == acceptance_criterion_id
            )
        }) {
            summary.coverage_items.push(EvidenceCoverageItem {
                target: EvidenceTarget::AcceptanceCriterion {
                    acceptance_criterion_id: AcceptanceCriterionId::new(acceptance_criterion_id),
                },
                coverage_state: EvidenceCoverageState::Unsupported,
                supporting_run_refs: Vec::new(),
                observation_refs: Vec::new(),
                supporting_artifact_refs: Vec::new(),
                gap_refs: Vec::new(),
            });
        }
    }
    summary.status = evidence_status_for_items(&summary.coverage_items);
    Some(summary)
}

fn evaluate_evidence_gate(
    acceptance_criteria: &[AcceptanceCriterion],
    evidence_summary: Option<&EvidenceSummary>,
    close_blockers: &[CloseReadinessBlocker],
) -> EvidenceGateSummary {
    let required_ids = acceptance_criteria
        .iter()
        .filter(|criterion| criterion.evidence_requirement == EvidenceRequirement::Required)
        .map(|criterion| criterion.acceptance_criterion_id.as_str())
        .collect::<BTreeSet<_>>();
    let optional_ids = acceptance_criteria
        .iter()
        .filter(|criterion| criterion.evidence_requirement == EvidenceRequirement::Optional)
        .map(|criterion| criterion.acceptance_criterion_id.as_str())
        .collect::<BTreeSet<_>>();

    if required_ids.is_empty() && optional_ids.is_empty() {
        return EvidenceGateSummary {
            state: EvidenceGateState::NotRequired,
        };
    }

    let coverage_items = evidence_summary
        .map(|summary| summary.coverage_items.as_slice())
        .unwrap_or_default();
    let criterion_item = |criterion_id: &str| {
        coverage_items.iter().find(|item| {
            matches!(
                &item.target,
                EvidenceTarget::AcceptanceCriterion {
                    acceptance_criterion_id
                } if acceptance_criterion_id.as_str() == criterion_id
            )
        })
    };
    let required_items = coverage_items.iter().filter(|item| {
        matches!(
            &item.target,
            EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id
            } if required_ids.contains(acceptance_criterion_id.as_str())
        )
    });
    let required_artifact_ids = required_items
        .clone()
        .flat_map(|item| item.supporting_artifact_refs.iter())
        .map(|artifact_ref| artifact_ref.artifact_id.as_str())
        .collect::<BTreeSet<_>>();

    let has_blocking_evidence_condition = close_blockers.iter().any(|blocker| {
        blocker.category == CloseReadinessBlockerCategory::Evidence
            || (blocker.category == CloseReadinessBlockerCategory::ArtifactAvailability
                && blocker.related_refs.iter().any(|record_ref| {
                    record_ref.record_kind == StateRecordKind::Artifact
                        && required_artifact_ids.contains(record_ref.record_id.as_str())
                }))
            || (blocker.category == CloseReadinessBlockerCategory::EvidenceProvenance
                && blocker.code != "evidence_provenance_stale")
    }) || required_items
        .clone()
        .any(|item| item.coverage_state == EvidenceCoverageState::Contradicted);
    if has_blocking_evidence_condition {
        return EvidenceGateSummary {
            state: EvidenceGateState::Blocked,
        };
    }

    let has_stale_evidence = close_blockers.iter().any(|blocker| {
        blocker.category == CloseReadinessBlockerCategory::EvidenceProvenance
            && blocker.code == "evidence_provenance_stale"
    }) || required_items
        .clone()
        .any(|item| item.coverage_state == EvidenceCoverageState::Stale);
    if has_stale_evidence {
        return EvidenceGateSummary {
            state: EvidenceGateState::Stale,
        };
    }

    let item_is_sufficient =
        |item: &EvidenceCoverageItem| item.coverage_state == EvidenceCoverageState::Supported;
    let item_has_recorded_evidence =
        |item: &EvidenceCoverageItem| !evidence_item_has_no_support(item);
    let has_evidence_claim_blocker = close_blockers
        .iter()
        .any(|blocker| blocker.category == CloseReadinessBlockerCategory::EvidenceClaim);

    if !required_ids.is_empty() {
        if !has_evidence_claim_blocker
            && required_ids
                .iter()
                .all(|criterion_id| criterion_item(criterion_id).is_some_and(item_is_sufficient))
        {
            return EvidenceGateSummary {
                state: EvidenceGateState::Sufficient,
            };
        }
        let any_required_evidence = required_ids.iter().any(|criterion_id| {
            criterion_item(criterion_id).is_some_and(item_has_recorded_evidence)
        });
        return EvidenceGateSummary {
            state: if any_required_evidence {
                EvidenceGateState::Partial
            } else {
                EvidenceGateState::RequiredMissing
            },
        };
    }

    let optional_items = optional_ids
        .iter()
        .filter_map(|criterion_id| criterion_item(criterion_id))
        .filter(|item| item_has_recorded_evidence(item))
        .collect::<Vec<_>>();
    if optional_items.is_empty() {
        return EvidenceGateSummary {
            state: EvidenceGateState::OptionalNone,
        };
    }
    EvidenceGateSummary {
        state: if optional_items.iter().all(|item| item_is_sufficient(item)) {
            EvidenceGateState::Sufficient
        } else {
            EvidenceGateState::Partial
        },
    }
}

fn close_context_with_record_run_projection(
    mut context: CloseTaskContext,
    run_ref: StateRecordRef,
    evidence_observations: Vec<EvidenceObservation>,
    registered_artifacts: Vec<ArtifactRef>,
) -> CloseTaskContext {
    context.projected_run_refs.push(run_ref);
    context.projected_evidence_observations = evidence_observations;
    context.projected_artifacts = registered_artifacts;
    context
}

fn close_context_with_pending_authorities(
    mut context: CloseTaskContext,
    authorities: Vec<UserActionAuthority>,
) -> CloseTaskContext {
    context.pending_user_action_authorities = Some(authorities);
    context
}

fn close_context_with_resolved_authorities(
    mut context: CloseTaskContext,
    authorities: Vec<UserActionAuthority>,
) -> CloseTaskContext {
    context.resolved_judgment_authorities = Some(authorities);
    context
}

fn projected_close_check(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    context: CloseTaskContext,
    now: DateTime<Utc>,
) -> Result<CloseTaskPlan, PlanError> {
    close_task::plan_close_task_with_context(
        store,
        project_state,
        Some(verified_invocation),
        None,
        close_task::CloseTaskPlanRequest::check(CheckCloseRequest {
            envelope: ToolEnvelope {
                task_id: Some(task_id.clone()).into(),
                ..envelope.clone()
            },
            task_id: task_id.clone(),
        }),
        &utc_timestamp(now),
        context,
    )
}

fn change_unit_insert(
    request: &UpdateScopeRequest,
    change_unit_id: &ChangeUnitId,
    verified_invocation: &VerifiedInvocationContext,
) -> CoreResult<ChangeUnitInsert> {
    let fields = &request.change_unit.fields;
    let scope_summary = string_member(fields, "scope_summary")
        .or_else(|| request.scope_boundary.as_ref().cloned())
        .unwrap_or_else(|| "Current Change Unit".to_owned());
    let affected_areas = string_array_member(fields, "affected_areas");
    let affected_paths = string_array_member(fields, "affected_paths");
    let constraints = string_array_member(fields, "constraints");
    Ok(ChangeUnitInsert {
        change_unit_id: change_unit_id.as_str().to_owned(),
        task_id: request.task_id.as_str().to_owned(),
        scope_summary_json: serde_json::to_string(&json!({
            "scope_summary": scope_summary,
            "affected_areas": affected_areas,
            "constraints": constraints
        }))?,
        bounded_paths_json: serde_json::to_string(&affected_paths)?,
        write_basis_json: serde_json::to_string(&json!({
            "baseline_ref": request.baseline_ref,
            "git_workspace_context": verified_invocation.git_workspace_context
        }))?,
        effect_contract_json: serde_json::to_string(&request.change_unit.effect_contract)?,
        lifecycle_json: "{}".to_owned(),
    })
}

fn synthetic_change_unit_record(
    project_id: &ProjectId,
    task_id: &TaskId,
    insert: &ChangeUnitInsert,
    planned_state_version: u64,
) -> ChangeUnitRecord {
    ChangeUnitRecord {
        project_id: project_id.as_str().to_owned(),
        change_unit_id: insert.change_unit_id.clone(),
        task_id: task_id.as_str().to_owned(),
        status: "active".to_owned(),
        is_current: true,
        basis_state_version: planned_state_version,
        scope_summary_json: insert.scope_summary_json.clone(),
        bounded_paths_json: insert.bounded_paths_json.clone(),
        write_basis_json: insert.write_basis_json.clone(),
        effect_contract_json: insert.effect_contract_json.clone(),
        lifecycle_json: insert.lifecycle_json.clone(),
    }
}

fn task_shaping_json(
    goal_summary: Option<String>,
    scope_summary: Option<String>,
    non_goals: Vec<String>,
    baseline_ref: Option<String>,
    autonomy_boundary: Option<String>,
    initial_context_refs: Option<Value>,
) -> Value {
    json!({
        "goal_summary": goal_summary,
        "scope_summary": scope_summary,
        "non_goals": non_goals,
        "baseline_ref": baseline_ref,
        "autonomy_boundary": autonomy_boundary,
        "initial_context_refs": initial_context_refs.unwrap_or(Value::Array(Vec::new()))
    })
}

fn next_actions_for_state(
    task_mode: TaskMode,
    task_ref: &StateRecordRef,
    change_unit_ref: Option<&StateRecordRef>,
    expected_state_version: u64,
) -> Vec<NextActionSummary> {
    match (task_mode, change_unit_ref) {
        (TaskMode::Advisor, Some(change_unit_ref)) => vec![NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::RecordRun,
            owner_method: Some(MethodName::RecordRun),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label: "Record an advisory shaping update for the current Change Unit.".to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::some(expected_state_version),
            required_refs: vec![task_ref.clone(), change_unit_ref.clone()],
        }],
        (_, Some(change_unit_ref)) => vec![NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::PrepareWrite,
            owner_method: Some(MethodName::PrepareWrite),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label: "Check the current change against current scope.".to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::some(expected_state_version),
            required_refs: vec![task_ref.clone(), change_unit_ref.clone()],
        }],
        (TaskMode::Advisor, None) => vec![NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::UpdateScope,
            owner_method: Some(MethodName::UpdateScope),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label:
                "Create the first currently applied Change Unit before recording advisory shaping."
                    .to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::some(expected_state_version),
            required_refs: vec![task_ref.clone()],
        }],
        (_, None) => vec![NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::UpdateScope,
            owner_method: Some(MethodName::UpdateScope),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label:
                "Create the first currently applied Change Unit before write-ticket preparation."
                    .to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::some(expected_state_version),
            required_refs: vec![task_ref.clone()],
        }],
    }
}

fn projected_user_action_lifecycle_phase(
    project_state: &ProjectStateHeader,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    pending_authorities: &[UserActionAuthority],
) -> Option<&'static str> {
    if project_state.active_task_id.as_deref() != Some(task.task_id.as_str())
        || is_terminal_lifecycle(&task.lifecycle_phase)
    {
        return None;
    }

    let task_id = TaskId::new(task.task_id.clone());
    let current_change_unit_id =
        current_change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let waits_for_user = pending_authorities.iter().any(|authority| {
        user_action_keeps_task_waiting(
            authority,
            &task_id,
            current_change_unit_id.as_ref(),
            task.scope_revision,
        )
    });
    let next_phase = if waits_for_user {
        "waiting_user"
    } else if task.lifecycle_phase == "waiting_user" {
        if current_change_unit.is_some() {
            "ready"
        } else {
            "shaping"
        }
    } else {
        return None;
    };

    (task.lifecycle_phase != next_phase).then_some(next_phase)
}

fn task_lifecycle_mutation(task_id: &TaskId, lifecycle_phase: &str) -> CoreStorageMutation {
    CoreStorageMutation::UpdateTaskScope(TaskScopeUpdate {
        task_id: task_id.as_str().to_owned(),
        work_phase: None,
        lifecycle_phase: Some(lifecycle_phase.to_owned()),
        result: None,
        title: None,
        summary: None,
        shaping_summary_json: None,
        bounded_context_json: None,
        autonomy_boundary_json: None,
        close_summary_json: None,
    })
}

fn summary_card_for_core(input: SummaryCardBuild<'_>) -> SummaryCard {
    let next = input
        .next_action
        .map(next_action_label)
        .unwrap_or_else(|| "none".to_owned());
    SummaryCard {
        task: task_summary_text(input.task),
        recording: input.recording.to_owned(),
        profile: input.profile.unwrap_or_else(|| "not_selected".to_owned()),
        write_ticket: input.write_ticket,
        evidence: input.evidence,
        user_action: count_state_text("pending", input.pending_user_actions),
        changes: input.changes,
        close_status: input.close_status,
        transport: transport_summary(input.verified_invocation),
        next,
        next_action: input.next_action.cloned(),
        guarantee: AUTHORITY_RECORD_SUMMARY_GUARANTEE.to_owned(),
    }
}

struct SummaryCardBuild<'a> {
    task: Option<&'a TaskRecord>,
    recording: &'a str,
    profile: Option<String>,
    write_ticket: String,
    evidence: String,
    pending_user_actions: usize,
    changes: String,
    close_status: String,
    verified_invocation: &'a VerifiedInvocationContext,
    next_action: Option<&'a NextActionSummary>,
}

const AUTHORITY_RECORD_SUMMARY_GUARANTEE: &str =
    "Local authority record; not OS enforcement, correctness proof, test sufficiency proof, or review completion.";

fn task_summary_text(task: Option<&TaskRecord>) -> String {
    task.map(|task| format!("selected ({})", task.lifecycle_phase))
        .unwrap_or_else(|| "none".to_owned())
}

fn profile_summary_text(guarantee_display: Option<&GuaranteeDisplay>) -> Option<String> {
    guarantee_display.map(|display| match display.level {
        GuaranteeLevel::Cooperative => "record".to_owned(),
    })
}

fn write_ticket_summary_text(selected: bool, summary: Option<&WriteTicketStateSummary>) -> String {
    if !selected {
        return "not_selected".to_owned();
    }
    summary
        .map(|summary| match summary.status {
            WriteTicketStatus::Active => "active",
            WriteTicketStatus::Consumed => "consumed",
            WriteTicketStatus::Invalidated => "invalidated",
            WriteTicketStatus::Revoked => "revoked",
        })
        .unwrap_or("none")
        .to_owned()
}

fn evidence_summary_for_display(
    mut summary: EvidenceSummary,
    close_basis: Option<&CurrentCloseBasis>,
) -> EvidenceSummary {
    summary.evidence_state = if close_basis
        .and_then(|basis| basis.evidence_summary_ref.as_ref())
        .is_some()
    {
        Some(EvidenceDisplayState::AcceptedForClose)
    } else if evidence_summary_has_attached_evidence(&summary) {
        Some(EvidenceDisplayState::Attached)
    } else {
        None
    };
    summary
}

fn evidence_summary_has_attached_evidence(summary: &EvidenceSummary) -> bool {
    summary.updated_by_run_ref.is_some()
        || !summary.artifact_refs.is_empty()
        || !summary.observation_refs.is_empty()
        || summary.coverage_items.iter().any(|item| {
            !item.supporting_run_refs.is_empty()
                || !item.observation_refs.is_empty()
                || !item.supporting_artifact_refs.is_empty()
        })
}

fn evidence_gate_summary_text(selected: bool, summary: Option<&EvidenceGateSummary>) -> String {
    if !selected {
        return "not_selected".to_owned();
    }
    summary
        .map(|summary| evidence_gate_state_text(summary.state))
        .unwrap_or("none")
        .to_owned()
}

fn evidence_gate_state_text(state: EvidenceGateState) -> &'static str {
    match state {
        EvidenceGateState::NotRequired => "not_required",
        EvidenceGateState::OptionalNone => "optional_none",
        EvidenceGateState::RequiredMissing => "required_missing",
        EvidenceGateState::Partial => "partial",
        EvidenceGateState::Sufficient => "sufficient",
        EvidenceGateState::Stale => "stale",
        EvidenceGateState::Blocked => "blocked",
    }
}

fn close_state_summary_text(selected: bool, close_state: Option<StatusCloseState>) -> String {
    if !selected {
        return "not_selected".to_owned();
    }
    close_state
        .map(status_close_state_text)
        .unwrap_or("none")
        .to_owned()
}

fn status_close_state_text(close_state: StatusCloseState) -> &'static str {
    match close_state {
        StatusCloseState::Ready => "ready",
        StatusCloseState::Blocked => "blocked",
        StatusCloseState::Closed => "closed",
        StatusCloseState::Cancelled => "cancelled",
        StatusCloseState::Superseded => "superseded",
        StatusCloseState::None => "none",
    }
}

fn close_state_text(close_state: CloseState) -> &'static str {
    match close_state {
        CloseState::Ready => "ready",
        CloseState::Blocked => "blocked",
        CloseState::Closed => "closed",
        CloseState::Cancelled => "cancelled",
        CloseState::Superseded => "superseded",
    }
}

fn changes_summary_text(selected: bool, unresolved_count: u64) -> String {
    if !selected {
        return "not_selected".to_owned();
    }
    count_state_text("unresolved", unresolved_count as usize)
}

fn count_state_text(label: &str, count: usize) -> String {
    if count == 0 {
        "none".to_owned()
    } else {
        format!("{label} ({count})")
    }
}

fn next_action_label(action: &NextActionSummary) -> String {
    if !action.label.trim().is_empty() {
        action.label.clone()
    } else {
        action
            .blocking_question
            .clone()
            .unwrap_or_else(|| "none".to_owned())
    }
}

fn normalize_next_action_collection(
    actions: &mut [NextActionSummary],
    expected_state_version: u64,
) {
    for (index, action) in actions.iter_mut().enumerate() {
        action.presentation_role = if index == 0 {
            NextActionPresentationRole::Primary
        } else {
            NextActionPresentationRole::Additional
        };
        action.allowed_operation_categories = allowed_operation_categories(action.owner_method);
        action.expected_state_version = next_action_expected_state_version(
            &action.allowed_operation_categories,
            expected_state_version,
        );
    }
}

fn normalize_close_blocker_action_projection(
    blockers: &mut [CloseReadinessBlocker],
    expected_state_version: u64,
) {
    for (action_index, action) in blockers
        .iter_mut()
        .flat_map(|blocker| blocker.next_actions.iter_mut())
        .enumerate()
    {
        action.presentation_role = if action_index == 0 {
            NextActionPresentationRole::Primary
        } else {
            NextActionPresentationRole::Additional
        };
        action.allowed_operation_categories = allowed_operation_categories(action.owner_method);
        action.expected_state_version = next_action_expected_state_version(
            &action.allowed_operation_categories,
            expected_state_version,
        );
    }
}

fn next_action_expected_state_version(
    allowed_operation_categories: &[OperationCategory],
    expected_state_version: u64,
) -> RequiredNullable<u64> {
    if allowed_operation_categories.contains(&OperationCategory::AgentWorkflow) {
        RequiredNullable::some(expected_state_version)
    } else {
        RequiredNullable::null()
    }
}

fn allowed_operation_categories(owner_method: Option<MethodName>) -> Vec<OperationCategory> {
    match owner_method {
        Some(MethodName::ResolveUserAction) => {
            vec![OperationCategory::UserOnly]
        }
        Some(MethodName::ReconcileChanges) => vec![
            OperationCategory::AgentWorkflow,
            OperationCategory::LocalRecovery,
        ],
        Some(
            MethodName::UpdateScope
            | MethodName::PrepareEvidenceCapture
            | MethodName::PrepareWrite
            | MethodName::StageArtifact
            | MethodName::RecordRun
            | MethodName::RequestUserAction
            | MethodName::CloseTask,
        ) => vec![OperationCategory::AgentWorkflow],
        Some(
            MethodName::Intake
            | MethodName::Status
            | MethodName::GetOperationResult
            | MethodName::CheckClose,
        )
        | None => Vec::new(),
    }
}

fn primary_next_action<'a>(
    next_actions: &'a [NextActionSummary],
    close_blockers: &'a [CloseReadinessBlocker],
) -> Option<&'a NextActionSummary> {
    next_actions
        .iter()
        .find(|action| action.presentation_role == NextActionPresentationRole::Primary)
        .or_else(|| {
            close_blockers
                .iter()
                .flat_map(|blocker| blocker.next_actions.iter())
                .find(|action| action.presentation_role == NextActionPresentationRole::Primary)
        })
}

fn transport_summary(verified_invocation: &VerifiedInvocationContext) -> String {
    match &verified_invocation.actor_source {
        ActorSource::AgentConnection(_) => "Agent Connection".to_owned(),
        ActorSource::LocalUser => "User Channel".to_owned(),
        ActorSource::System => "system".to_owned(),
    }
}

fn dry_run_summary(
    target_kind: &str,
    action: &str,
    description: &str,
    next_actions: Vec<NextActionSummary>,
) -> DryRunSummary {
    DryRunSummary {
        planned_effects: vec![PlannedEffect {
            target_kind: target_kind.to_owned(),
            action: action.to_owned(),
            description: description.to_owned(),
        }],
        would_blockers: Vec::new(),
        would_errors: Vec::new(),
        next_actions,
        diagnostics: Vec::new(),
    }
}

fn state_ref(
    record_kind: StateRecordKind,
    record_id: &str,
    project_id: &ProjectId,
    task_id: Option<&TaskId>,
    state_version: Option<u64>,
) -> StateRecordRef {
    StateRecordRef {
        record_kind,
        record_id: RecordId::new(record_id),
        project_id: project_id.clone(),
        task_id: task_id.cloned().into(),
        produced_at_state_version: state_version.into(),
    }
}

fn write_ticket_ref(record: &WriteTicketRecord, state_version: u64) -> StateRecordRef {
    state_ref(
        StateRecordKind::WriteTicket,
        &record.write_ticket_id,
        &ProjectId::new(record.project_id.clone()),
        Some(&TaskId::new(record.task_id.clone())),
        Some(state_version),
    )
}

fn project_continuity_ref(
    record: &ProjectContinuityRecordRecord,
    state_version: u64,
) -> StateRecordRef {
    state_ref(
        StateRecordKind::ProjectContinuityRecord,
        &record.continuity_record_id,
        &ProjectId::new(record.project_id.clone()),
        Some(&TaskId::new(record.source_task_id.clone())),
        Some(state_version),
    )
}

fn project_continuity_record_from_storage(
    record: &ProjectContinuityRecordRecord,
) -> CoreResult<ProjectContinuityRecord> {
    let record_id = record.continuity_record_id.clone();
    Ok(ProjectContinuityRecord {
        continuity_record_id: ProjectContinuityRecordId::new(record.continuity_record_id.clone()),
        project_id: ProjectId::new(record.project_id.clone()),
        source_task_id: TaskId::new(record.source_task_id.clone()),
        source_change_unit_id: record
            .source_change_unit_id
            .clone()
            .map(ChangeUnitId::new)
            .into(),
        kind: parse_owner_storage_value(
            "project_continuity_records",
            record_id.clone(),
            "kind",
            &record.kind,
        )?,
        title: record.title.clone(),
        summary: record.summary.clone(),
        rationale: record.rationale.clone().into(),
        applies_to_paths: decode_required_json(
            "project_continuity_records",
            record_id.clone(),
            "applies_to_paths_json",
            Some(&record.applies_to_paths_json),
        )?,
        applies_to_refs: decode_required_json(
            "project_continuity_records",
            record_id.clone(),
            "applies_to_refs_json",
            Some(&record.applies_to_refs_json),
        )?,
        source_refs: decode_required_json(
            "project_continuity_records",
            record_id.clone(),
            "source_refs_json",
            Some(&record.source_refs_json),
        )?,
        artifact_refs: decode_required_json(
            "project_continuity_records",
            record_id.clone(),
            "artifact_refs_json",
            Some(&record.artifact_refs_json),
        )?,
        status: parse_owner_storage_value(
            "project_continuity_records",
            record_id.clone(),
            "status",
            &record.status,
        )?,
        supersedes_refs: decode_required_json(
            "project_continuity_records",
            record_id.clone(),
            "supersedes_refs_json",
            Some(&record.supersedes_refs_json),
        )?,
        review_triggers: decode_required_json(
            "project_continuity_records",
            record_id.clone(),
            "review_triggers_json",
            Some(&record.review_triggers_json),
        )?,
        created_at: parse_owner_storage_value(
            "project_continuity_records",
            record_id.clone(),
            "created_at",
            &record.created_at,
        )?,
        updated_at: parse_owner_storage_value(
            "project_continuity_records",
            record_id,
            "updated_at",
            &record.updated_at,
        )?,
    })
}

fn project_continuity_summary_from_record(
    record: &ProjectContinuityRecordRecord,
    state_version: u64,
) -> CoreResult<ProjectContinuitySummary> {
    let continuity = project_continuity_record_from_storage(record)?;
    let project_id = continuity.project_id.clone();
    let source_task_id = continuity.source_task_id.clone();
    let source_change_unit_ref = continuity
        .source_change_unit_id
        .as_ref()
        .map(|change_unit_id| {
            state_ref(
                StateRecordKind::ChangeUnit,
                change_unit_id.as_str(),
                &project_id,
                Some(&source_task_id),
                Some(state_version),
            )
        })
        .into();
    Ok(ProjectContinuitySummary {
        continuity_record_ref: project_continuity_ref(record, state_version),
        kind: continuity.kind,
        status: continuity.status,
        title: continuity.title,
        summary: continuity.summary,
        source_task_ref: state_ref(
            StateRecordKind::Task,
            source_task_id.as_str(),
            &project_id,
            Some(&source_task_id),
            Some(state_version),
        ),
        source_change_unit_ref,
        review_triggers: continuity.review_triggers,
    })
}

fn state_ref_from_stored(record: StoredRecordRef) -> StateRecordRef {
    let kind = match record.record_kind.as_str() {
        "user_action_request" => StateRecordKind::UserActionRequest,
        "user_action_resolution" => StateRecordKind::UserActionResolution,
        "blocker" => StateRecordKind::Blocker,
        "write_ticket" => StateRecordKind::WriteTicket,
        "change_unit" => StateRecordKind::ChangeUnit,
        "task" => StateRecordKind::Task,
        "evidence_observation" => StateRecordKind::EvidenceObservation,
        "unrecorded_change" => StateRecordKind::UnrecordedChange,
        "project_continuity_record" => StateRecordKind::ProjectContinuityRecord,
        _ => StateRecordKind::ProjectState,
    };
    StateRecordRef {
        record_kind: kind,
        record_id: RecordId::new(record.record_id),
        project_id: ProjectId::new(record.project_id),
        task_id: record.task_id.map(TaskId::new).into(),
        produced_at_state_version: record.state_version.into(),
    }
}

fn stored_refs_to_state_refs(records: Vec<StoredRecordRef>) -> Vec<StateRecordRef> {
    records.into_iter().map(state_ref_from_stored).collect()
}

fn object_from_value(value: Value) -> CoreResult<JsonObject> {
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(CorePipelineError::InvalidDispatch {
            detail: "expected JSON object".to_owned(),
        }),
    }
}

fn validation_rejected(
    dry_run: bool,
    state_version: Option<u64>,
    field: &'static str,
    message: &'static str,
) -> CoreResult<PipelineResponse> {
    let mut details = Map::new();
    details.insert("field".to_owned(), Value::String(field.to_owned()));
    rejected_pipeline_response(
        dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::ValidationFailed,
            message,
            false,
            Some(details),
        )],
    )
}

fn rejected_pipeline_response(
    dry_run: bool,
    state_version: Option<u64>,
    errors: Vec<volicord_types::ToolError>,
) -> CoreResult<PipelineResponse> {
    let response = rejected_response(dry_run, state_version, errors);
    let response_value = serde_json::to_value(response)?;
    let response_json = serde_json::to_string(&response_value)?;
    Ok(PipelineResponse {
        response_json,
        response_value,
        operation_result_ref: None,
        verified_invocation: None,
        resolved_task_id: None,
        replayed: false,
    })
}

fn infallible_rejected_pipeline_response(
    dry_run: bool,
    state_version: Option<u64>,
    errors: Vec<volicord_types::ToolError>,
) -> PipelineResponse {
    rejected_pipeline_response(dry_run, state_version, errors)
        .expect("rejected response serialization should succeed")
}

fn store_error_response(
    envelope: &ToolEnvelope,
    project_state: &ProjectStateHeader,
    error: StoreError,
) -> PipelineResponse {
    rejected_pipeline_response(
        envelope.dry_run,
        Some(project_state.state_version),
        vec![store_failure_error(error)],
    )
    .expect("rejected response serialization should succeed")
}

fn core_error_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    error: CorePipelineError,
) -> CoreResult<PipelineResponse> {
    match error {
        CorePipelineError::Store(error) => rejected_pipeline_response(
            envelope.dry_run,
            state_version,
            vec![store_failure_error(error)],
        ),
        error => Err(error),
    }
}

fn plan_error_response(
    envelope: &ToolEnvelope,
    project_state: &ProjectStateHeader,
    error: PlanError,
) -> CoreResult<PipelineResponse> {
    match error {
        PlanError::Response(response) => Ok(*response),
        PlanError::Core(error) => {
            core_error_response(envelope, Some(project_state.state_version), error)
        }
    }
}

fn no_active_task_response(
    envelope: &ToolEnvelope,
    project_state: &ProjectStateHeader,
) -> PipelineResponse {
    rejected_pipeline_response(
        envelope.dry_run,
        Some(project_state.state_version),
        vec![tool_error(
            ErrorCode::NoActiveTask,
            "a Task is required but no addressed or current Task is available",
            false,
            None,
        )],
    )
    .expect("rejected response serialization should succeed")
}

fn resolve_requested_mode(requested_mode: RequestedMode) -> TaskMode {
    match requested_mode {
        RequestedMode::Advisor => TaskMode::Advisor,
        RequestedMode::Direct => TaskMode::Direct,
        RequestedMode::Work | RequestedMode::Auto => TaskMode::Work,
    }
}

fn task_mode_storage(mode: TaskMode) -> &'static str {
    match mode {
        TaskMode::Advisor => "advisor",
        TaskMode::Direct => "direct",
        TaskMode::Work => "work",
    }
}

fn initial_work_phase(mode: TaskMode) -> WorkPhase {
    match mode {
        TaskMode::Direct => WorkPhase::Implementation,
        TaskMode::Advisor | TaskMode::Work => WorkPhase::Shaping,
    }
}

fn work_phase_storage(phase: WorkPhase) -> &'static str {
    match phase {
        WorkPhase::Shaping => "shaping",
        WorkPhase::Implementation => "implementation",
    }
}

fn parse_work_phase(value: &str) -> CoreResult<WorkPhase> {
    match value {
        "shaping" => Ok(WorkPhase::Shaping),
        "implementation" => Ok(WorkPhase::Implementation),
        _ => invalid_storage("tasks.work_phase"),
    }
}

fn acceptance_policy_storage(policy: AcceptancePolicy) -> &'static str {
    match policy {
        AcceptancePolicy::Required => "required",
        AcceptancePolicy::NotRequired => "not_required",
        AcceptancePolicy::PolicyDependent => "policy_dependent",
    }
}

fn parse_acceptance_policy(value: &str) -> CoreResult<AcceptancePolicy> {
    match value {
        "required" => Ok(AcceptancePolicy::Required),
        "not_required" => Ok(AcceptancePolicy::NotRequired),
        "policy_dependent" => Ok(AcceptancePolicy::PolicyDependent),
        _ => invalid_storage("tasks.acceptance_policy"),
    }
}

fn task_lineage_relation_storage(relation: TaskLineageRelation) -> &'static str {
    match relation {
        TaskLineageRelation::Continues => "continues",
        TaskLineageRelation::DerivedFrom => "derived_from",
        TaskLineageRelation::SplitFrom => "split_from",
        TaskLineageRelation::Replaces => "replaces",
        TaskLineageRelation::ImplementsAdviceFrom => "implements_advice_from",
    }
}

fn parse_task_lineage_relation(value: &str) -> CoreResult<TaskLineageRelation> {
    match value {
        "continues" => Ok(TaskLineageRelation::Continues),
        "derived_from" => Ok(TaskLineageRelation::DerivedFrom),
        "split_from" => Ok(TaskLineageRelation::SplitFrom),
        "replaces" => Ok(TaskLineageRelation::Replaces),
        "implements_advice_from" => Ok(TaskLineageRelation::ImplementsAdviceFrom),
        _ => invalid_storage("tasks.lineage_relation"),
    }
}

fn parse_task_mode(value: &str) -> CoreResult<TaskMode> {
    match value {
        "advisor" => Ok(TaskMode::Advisor),
        "direct" => Ok(TaskMode::Direct),
        "work" => Ok(TaskMode::Work),
        _ => invalid_storage("tasks.mode"),
    }
}

fn parse_lifecycle_phase(value: &str) -> CoreResult<TaskLifecyclePhase> {
    match value {
        "shaping" => Ok(TaskLifecyclePhase::Shaping),
        "ready" => Ok(TaskLifecyclePhase::Ready),
        "executing" => Ok(TaskLifecyclePhase::Executing),
        "waiting_user" => Ok(TaskLifecyclePhase::WaitingUser),
        "blocked" => Ok(TaskLifecyclePhase::Blocked),
        "completed" => Ok(TaskLifecyclePhase::Completed),
        "cancelled" => Ok(TaskLifecyclePhase::Cancelled),
        "superseded" => Ok(TaskLifecyclePhase::Superseded),
        _ => invalid_storage("tasks.lifecycle_phase"),
    }
}

fn parse_task_result(value: &str) -> CoreResult<TaskResult> {
    match value {
        "none" => Ok(TaskResult::None),
        "advice_only" => Ok(TaskResult::AdviceOnly),
        "completed" => Ok(TaskResult::Completed),
        "cancelled" => Ok(TaskResult::Cancelled),
        "superseded" => Ok(TaskResult::Superseded),
        _ => invalid_storage("tasks.result"),
    }
}

fn parse_close_reason(task: &TaskRecord) -> CoreResult<CloseReason> {
    let value: PersistedCloseSummary = decode_required_json(
        "tasks",
        task.task_id.clone(),
        "close_summary_json",
        Some(&task.close_summary_json),
    )?;
    Ok(value.close_reason)
}

fn invalid_storage<T>(field: &'static str) -> CoreResult<T> {
    Err(CorePipelineError::Store(StoreError::corrupt_stored_value(
        "project_state",
        field,
    )))
}

fn string_member(object: &JsonObject, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn string_array_member(object: &JsonObject, key: &str) -> Vec<String> {
    object
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
