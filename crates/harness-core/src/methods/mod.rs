use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use chrono::{DateTime, Duration, Utc};
use harness_store::{
    artifacts::{ArtifactStagingInsert, StagedPayloadKind},
    core_pipeline::{
        ArtifactLinkInsert, ArtifactPromotion, ChangeUnitInsert, ChangeUnitRecord,
        CoreProjectStore, CoreStorageMutation, EvidenceSummaryRecord, EvidenceSummaryUpsert,
        ProjectStateHeader, RunInsert, StoredArtifactRecord, StoredArtifactStagingRecord,
        StoredRecordRef, TaskCloseBasisUpdate, TaskCloseUpdate, TaskInsert, TaskRecord,
        TaskScopeRevisionUpdate, TaskScopeUpdate, UserJudgmentInsert, UserJudgmentInvalidation,
        UserJudgmentRecord, UserJudgmentResolutionUpdate, WriteAuthorizationConsumption,
        WriteAuthorizationInsert, WriteAuthorizationRecord,
    },
    StoreError,
};
use harness_types::{
    AccessClass, ActorKind, ArtifactAvailability, ArtifactId, ArtifactInput,
    ArtifactInputSourceKind, ArtifactRef, AuthorizationEffect, AuthorizedAttemptScope, BaselineRef,
    ChangeUnitId, ChangeUnitOperation, CloseIntent, CloseReadinessBlocker,
    CloseReadinessBlockerCategory, CloseReason, CloseState, CloseTaskRequest, CloseTaskResult,
    CompletionPolicy, CurrentCloseBasis, DryRunSummary, DurableIdKind, EffectKind, ErrorCode,
    EvidenceCoverageItem, EvidenceCoverageState, EvidenceStatus, EvidenceSummary, GuaranteeDisplay,
    GuaranteeLevel, JsonObject, JudgmentBasis, JudgmentBasisCompatibilityStatus, JudgmentKind,
    JudgmentResolutionOutcome, MethodAccessClass, MethodName, NextActionKind, NextActionSummary,
    ObservedChanges, PersistedEvidenceMetadata, PersistedJudgmentBasis,
    PersistedUserJudgmentRequest, PersistedUserJudgmentResolution, PlannedEffect,
    PrepareWriteRequest, PrepareWriteResult, ProjectId, RecordId, RecordRunRequest,
    RecordRunResult, RecordUserJudgmentPayload, RecordUserJudgmentRequest, RedactionState,
    RequestedMode, RequiredNullable, ResidualRisk, ResumePolicy, RiskAcceptanceCoverage, RiskId,
    RunId, RunSummary, StageArtifactRequest, StageArtifactResult, StagedArtifactHandle,
    StagedArtifactHandleId, StateRecordKind, StateRecordRef, StatusCloseState, StatusInclude,
    StatusRequest, StorageRef, SurfaceId, SurfaceInstanceId, TaskId, TaskLifecyclePhase,
    TaskLifecycleState, TaskMode, TaskResult, ToolEnvelope, ToolResultBase, UpdateScopeRequest,
    UserJudgment, UserJudgmentContext, UserJudgmentOption, UserJudgmentResolution,
    UserJudgmentStatus, UtcTimestamp, WriteAuthoritySummary, WriteAuthorizationId,
    WriteAuthorizationStatus, WriteAuthorizationSummary, WriteDecisionCategory,
    WriteDecisionReason,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::pipeline::{
    dry_run_response, method_result_base, rejected_response, store_failure_error, tool_error,
    CorePipelineError, CoreResult, CoreService, FreshnessPolicy, InvocationContext,
    MethodEffectPolicy, MethodPolicy, OwnerPipelineBranch, PipelinePreflightOutcome,
    PipelinePreflightRequest, PipelineResponse, PreparedRequest, ReplayPolicy, TaskRequirement,
    VerifiedSurfaceContext,
};
use crate::policy::{
    close_readiness::{
        accepted_risk_ids_within_basis, close_basis_is_current, close_blocker, close_next_action,
        current_acceptance_required_risk_ids, current_final_acceptance,
        current_residual_risk_acceptance_coverage, current_scope_decision,
        final_acceptance_basis_matches_current, final_acceptance_requirement,
        is_terminal_lifecycle, judgment_has_current_basis, residual_risk_basis_matches_current,
        JudgmentAuthority,
    },
    evidence::{evidence_status_for_items, unique_artifact_refs},
    path::{normalize_product_paths, path_is_within, paths_are_authorized, ProductPathError},
    write_authorization::{
        current_sensitive_approval, normalize_sensitive_action_scope, prepare_write_decision,
        prepare_write_dry_run_summary, sensitive_action_scope_matches_requirement,
        surface_supports_prepare_write, write_authorization_expires_at,
        write_authorization_guarantee, write_authorization_is_expired, write_decision_reason,
        SensitiveApprovalRequirement,
    },
};

mod close_task;
mod intake;
mod judgment;
mod prepare_write;
mod record_run;
mod stage_artifact;
mod status;
#[cfg(test)]
mod tests;
mod update_scope;

struct MethodPlan {
    task_id: TaskId,
    change_unit_id: Option<ChangeUnitId>,
    storage_mutations: Vec<CoreStorageMutation>,
    event_payload: JsonObject,
    result_fields: JsonObject,
    next_actions: Vec<NextActionSummary>,
}

struct PrepareWritePlan {
    task_id: TaskId,
    change_unit_id: Option<ChangeUnitId>,
    storage_mutations: Vec<CoreStorageMutation>,
    event_kind: String,
    event_payload: JsonObject,
    result_fields: JsonObject,
    dry_run_summary: DryRunSummary,
}

struct CloseTaskPlan {
    task_id: TaskId,
    change_unit_id: Option<ChangeUnitId>,
    storage_mutations: Vec<CoreStorageMutation>,
    event_kind: String,
    event_payload: JsonObject,
    result_fields: JsonObject,
    close_state: CloseState,
    current_close_basis: Option<CurrentCloseBasis>,
    risk_acceptance_coverage: Vec<RiskAcceptanceCoverage>,
    blockers: Vec<CloseReadinessBlocker>,
}

struct CloseTaskContext {
    task: TaskRecord,
    current_change_unit: Option<ChangeUnitRecord>,
    current_close_basis: Option<CurrentCloseBasis>,
    pending_user_judgment_refs: Vec<StateRecordRef>,
    blocker_refs: Vec<StateRecordRef>,
    evidence_summary: Option<EvidenceSummary>,
    artifact_refs: Vec<ArtifactRef>,
}

struct ValidatedStageArtifactInput {
    safe_bytes: Vec<u8>,
    sha256: String,
    size_bytes: u64,
    payload_kind: StagedPayloadKind,
}

const MAX_STAGED_BODY_BYTES: usize = 10 * 1024 * 1024;

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

fn allocate_user_judgment_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<harness_types::UserJudgmentId> {
    service
        .allocate_generated_id(DurableIdKind::UserJudgment, |candidate| {
            store
                .user_judgment_record(candidate)
                .map(|record| record.is_some())
                .map_err(CorePipelineError::from)
        })
        .map(harness_types::UserJudgmentId::new)
}

fn allocate_write_authorization_id(
    service: &CoreService,
    store: &CoreProjectStore,
) -> CoreResult<WriteAuthorizationId> {
    service
        .allocate_generated_id(DurableIdKind::WriteAuthorization, |candidate| {
            store
                .write_authorization_record(candidate)
                .map(|record| record.is_some())
                .map_err(CorePipelineError::from)
        })
        .map(WriteAuthorizationId::new)
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

fn prepare_or_response(
    service: &CoreService,
    method_name: MethodName,
    envelope: ToolEnvelope,
    request_json: Value,
    invocation: InvocationContext,
    policy: MethodPolicy,
) -> CoreResult<Result<PreparedRequest, PipelineResponse>> {
    match service.prepare_request(PipelinePreflightRequest {
        method_name,
        envelope,
        request_json,
        invocation,
        policy,
    })? {
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

fn decode_optional_json<T>(
    table: &'static str,
    record_ref: impl Into<String>,
    logical_column: &'static str,
    raw: Option<&str>,
) -> CoreResult<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    match raw {
        Some(raw) => decode_required_json(table, record_ref, logical_column, Some(raw)).map(Some),
        None => Ok(None),
    }
}

fn decode_optional_persisted_resolution(
    table: &'static str,
    record_ref: impl Into<String>,
    logical_column: &'static str,
    raw: Option<&str>,
    stored_resolution_outcome: Option<JudgmentResolutionOutcome>,
) -> CoreResult<Option<UserJudgmentResolution>> {
    let record_ref = record_ref.into();
    let resolution = decode_optional_json::<PersistedUserJudgmentResolution>(
        table,
        record_ref.clone(),
        logical_column,
        raw,
    )?;
    let Some(mut resolution) = resolution else {
        return Ok(None);
    };
    if resolution.resolution_outcome.is_some()
        && resolution.resolution_outcome != stored_resolution_outcome
    {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_value(table, record_ref, "resolution_outcome"),
        ));
    }
    resolution.resolution_outcome = stored_resolution_outcome;
    Ok(Some(resolution.into()))
}

fn decode_required_json_object(
    table: &'static str,
    record_ref: impl Into<String>,
    logical_column: &'static str,
    raw: Option<&str>,
) -> CoreResult<JsonObject> {
    decode_required_json(table, record_ref, logical_column, raw)
}

fn user_judgment_authority_from_record(
    record: &UserJudgmentRecord,
) -> CoreResult<JudgmentAuthority> {
    let basis_status = parse_owner_storage_value(
        "user_judgments",
        record.judgment_id.clone(),
        "basis_status",
        &record.basis_status,
    )?;
    let mut basis: Option<JudgmentBasis> = decode_optional_json::<PersistedJudgmentBasis>(
        "user_judgments",
        record.judgment_id.clone(),
        "basis_json",
        record.basis_json.as_deref(),
    )?;
    if let Some(basis) = &mut basis {
        basis.compatibility_status = basis_status;
    }
    let judgment_kind = parse_owner_storage_value(
        "user_judgments",
        record.judgment_id.clone(),
        "judgment_kind",
        &record.judgment_kind,
    )?;
    let status = parse_owner_storage_value(
        "user_judgments",
        record.judgment_id.clone(),
        "status",
        &record.status,
    )?;
    let resolution_outcome = record
        .resolution_outcome
        .as_deref()
        .map(|outcome| {
            parse_owner_storage_value(
                "user_judgments",
                record.judgment_id.clone(),
                "resolution_outcome",
                outcome,
            )
        })
        .transpose()?;
    let resolution = decode_optional_persisted_resolution(
        "user_judgments",
        record.judgment_id.clone(),
        "resolution_json",
        record.resolution_json.as_deref(),
        resolution_outcome,
    )?;
    if resolution.as_ref().is_some_and(|resolution| {
        !stored_answer_branch_matches_kind(judgment_kind, &resolution.answer)
    }) {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json(
                "user_judgments",
                record.judgment_id.clone(),
                "resolution_json",
            ),
        ));
    }
    Ok(JudgmentAuthority {
        judgment_id: record.judgment_id.clone(),
        task_id: TaskId::new(record.task_id.clone()),
        judgment_kind,
        status,
        resolution_outcome,
        basis_status,
        basis,
        resolution,
    })
}

fn stored_answer_branch_matches_kind(
    judgment_kind: JudgmentKind,
    answer: &RecordUserJudgmentPayload,
) -> bool {
    match judgment_kind {
        JudgmentKind::ProductDecision => answer.product_decision.is_some(),
        JudgmentKind::TechnicalDecision => answer.technical_decision.is_some(),
        JudgmentKind::ScopeDecision => answer.scope_decision.is_some(),
        JudgmentKind::SensitiveApproval => answer.sensitive_action_scope.is_some(),
        JudgmentKind::FinalAcceptance => answer.final_acceptance.is_some(),
        JudgmentKind::ResidualRiskAcceptance => answer.residual_risk_acceptance.is_some(),
        JudgmentKind::Cancellation => answer.cancellation.is_some(),
    }
}

fn resolved_judgment_authorities_for_plan(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    judgment_kind: JudgmentKind,
) -> Result<Vec<JudgmentAuthority>, PlanError> {
    let kind = storage_value(judgment_kind)?;
    store
        .resolved_user_judgment_records(task_id, &kind)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                envelope,
                project_state,
                error,
            )))
        })?
        .iter()
        .map(user_judgment_authority_from_record)
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

fn validation_plan_error(
    dry_run: bool,
    state_version: Option<u64>,
    field: &'static str,
    message: &'static str,
) -> Result<(), PlanError> {
    let response =
        validation_rejected(dry_run, state_version, field, message).map_err(PlanError::Core)?;
    Err(PlanError::Response(Box::new(response)))
}

fn mutation_method_policy(
    access_class: AccessClass,
    task: TaskRequirement,
    dry_run: bool,
) -> MethodPolicy {
    if dry_run {
        MethodPolicy::exact(
            access_class,
            task,
            ReplayPolicy::None,
            FreshnessPolicy::IfPresent,
            MethodEffectPolicy::DryRunPreview,
        )
    } else {
        MethodPolicy::exact(
            access_class,
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

fn resolve_prepare_write_change_unit<'a>(
    request: &PrepareWriteRequest,
    task_id: &TaskId,
    current_change_unit: Option<&'a ChangeUnitRecord>,
    reasons: &mut Vec<WriteDecisionReason>,
) -> Option<&'a ChangeUnitRecord> {
    let Some(current_change_unit) = current_change_unit else {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Scope,
            "no_current_change_unit",
            "No current Change Unit can be resolved for write preparation.",
            Vec::new(),
        ));
        return None;
    };

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
                current_change_unit.basis_state_version.unwrap_or_default(),
            )],
        ));
    }

    Some(current_change_unit)
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
    let baseline = match write_basis.baseline_ref {
        Some(value) => Some(value.as_str().to_owned()),
        None => StoredScope::from_task(task)?.baseline_ref,
    };
    Ok(baseline.as_deref() == Some(baseline_ref.as_str()))
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

struct SensitiveApprovalSearch<'a> {
    store: &'a CoreProjectStore,
    project_state: &'a ProjectStateHeader,
    request: &'a PrepareWriteRequest,
    task_id: &'a TaskId,
    task: &'a TaskRecord,
    change_unit: Option<&'a ChangeUnitRecord>,
    normalized_paths: &'a [String],
    now: &'a UtcTimestamp,
}

fn matching_sensitive_approval(
    search: SensitiveApprovalSearch<'_>,
) -> Result<Option<UserJudgmentRecord>, PlanError> {
    let SensitiveApprovalSearch {
        store,
        project_state,
        request,
        task_id,
        task,
        change_unit,
        normalized_paths,
        now,
    } = search;
    let records = store
        .resolved_user_judgment_records(task_id, "sensitive_approval")
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let Some(change_unit) = change_unit else {
        return Ok(None);
    };
    let change_unit_id = ChangeUnitId::new(change_unit.change_unit_id.clone());
    let requirement = SensitiveApprovalRequirement {
        task_id,
        change_unit_id: &change_unit_id,
        scope_revision: task.scope_revision,
        operation: &request.intended_operation,
        normalized_paths,
        sensitive_categories: &request.sensitive_categories,
        baseline_ref: Some(&request.baseline_ref),
        now,
        repo_root: &store.project_record().repo_root,
    };

    for record in records {
        let authority = user_judgment_authority_from_record(&record)?;
        if current_sensitive_approval(&authority, &requirement) {
            return Ok(Some(record));
        }
    }

    Ok(None)
}

fn string_set(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
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

fn write_authorization_required_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
) -> PipelineResponse {
    let details = object_from_value(json!({
        "authorization_reason": "missing"
    }))
    .expect("authorization details should be an object");
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::WriteAuthorizationRequired,
            "product-file write observations require a compatible active Write Authorization",
            false,
            Some(details),
        )],
    )
}

fn write_authorization_invalid_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    reason: &'static str,
    message: &'static str,
) -> PipelineResponse {
    let details = object_from_value(json!({
        "authorization_reason": reason
    }))
    .expect("authorization details should be an object");
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::WriteAuthorizationInvalid,
            message,
            false,
            Some(details),
        )],
    )
}

fn stale_write_authorization_basis_response(
    envelope: &ToolEnvelope,
    record: &WriteAuthorizationRecord,
    current_state_version: u64,
) -> PipelineResponse {
    let mut details = Map::new();
    details.insert(
        "state_clock".to_owned(),
        Value::String("project_state.state_version".to_owned()),
    );
    details.insert(
        "current_state_version".to_owned(),
        Value::from(current_state_version),
    );
    details.insert(
        "write_authorization_basis_state_version".to_owned(),
        Value::from(record.basis_state_version),
    );
    details.insert(
        "write_authorization_id".to_owned(),
        Value::String(record.write_authorization_id.clone()),
    );
    details.insert(
        "project_id".to_owned(),
        Value::String(envelope.project_id.as_str().to_owned()),
    );
    if let Some(task_id) = envelope.task_id.as_ref() {
        details.insert(
            "task_id".to_owned(),
            Value::String(task_id.as_str().to_owned()),
        );
    }
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        Some(current_state_version),
        vec![tool_error(
            ErrorCode::StateVersionConflict,
            "Write Authorization basis_state_version is stale",
            true,
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
    acceptance_criteria: Vec<String>,
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
    acceptance_criteria: Vec<String>,
    #[serde(default)]
    baseline_ref: Option<String>,
    #[serde(default)]
    autonomy_boundary: Option<String>,
    #[serde(default)]
    initial_context_refs: Option<Value>,
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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedLifecycleState {
    #[serde(default)]
    recovery_required: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PersistedCloseSummary {
    #[serde(default)]
    close_reason: Option<CloseReason>,
    #[serde(default)]
    closed_at: Option<UtcTimestamp>,
    #[serde(default)]
    intent: Option<CloseIntent>,
    #[serde(default)]
    user_note: Option<String>,
    #[serde(default)]
    superseding_task_id: Option<TaskId>,
    #[serde(default)]
    required_sensitive_categories: Vec<String>,
    #[serde(default)]
    sensitive_categories: Vec<String>,
    #[serde(default)]
    baseline_stale: bool,
    #[serde(default)]
    baseline_status: Option<String>,
    #[serde(default)]
    recovery_required: bool,
    #[serde(default)]
    visible_risks: Vec<Value>,
    #[serde(default)]
    residual_risk_visible: bool,
    #[serde(default)]
    residual_risks: Vec<Value>,
    #[serde(default)]
    residual_risk_present: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedCompletionPolicy {
    #[serde(default)]
    evidence_required: bool,
    #[serde(default)]
    required_claims: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedAuthorizedAttemptScope {
    task_id: TaskId,
    change_unit_id: ChangeUnitId,
    intended_operation: String,
    intended_paths: Vec<String>,
    product_file_write_intended: bool,
    sensitive_categories: Vec<String>,
    baseline_ref: Option<BaselineRef>,
}

impl From<PersistedAuthorizedAttemptScope> for AuthorizedAttemptScope {
    fn from(scope: PersistedAuthorizedAttemptScope) -> Self {
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
            acceptance_criteria: shaping.acceptance_criteria,
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
            acceptance_criteria: request
                .acceptance_criteria
                .clone()
                .unwrap_or_else(|| self.acceptance_criteria.clone()),
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
        self.acceptance_criteria = normalize_scope_string_list(self.acceptance_criteria);
        self.autonomy_boundary = normalize_scope_text_option(self.autonomy_boundary);
        self.baseline_ref = normalize_scope_text_option(self.baseline_ref);
        self
    }

    fn to_json(&self) -> Value {
        task_shaping_json(
            self.goal_summary.clone(),
            self.scope_summary.clone(),
            self.non_goals.clone(),
            self.acceptance_criteria.clone(),
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

fn normalize_scope_string_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .filter_map(|value| normalize_scope_text_option(Some(value)))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct SummaryOptions {
    pending_user_judgments: bool,
    blockers: bool,
    write_authority: bool,
}

impl SummaryOptions {
    fn mutation() -> Self {
        Self {
            pending_user_judgments: true,
            blockers: true,
            write_authority: false,
        }
    }

    fn prepare_write() -> Self {
        Self {
            pending_user_judgments: true,
            blockers: true,
            write_authority: true,
        }
    }

    fn status(include: &StatusInclude) -> Self {
        Self {
            pending_user_judgments: include.pending_user_judgments,
            blockers: true,
            write_authority: include.write_authority,
        }
    }
}

struct SummaryBuild<'a> {
    project_id: &'a ProjectId,
    state_version: u64,
    task: &'a TaskRecord,
    current_change_unit: Option<&'a ChangeUnitRecord>,
    pending_user_judgment_refs: Vec<StateRecordRef>,
    blocker_refs: Vec<StateRecordRef>,
    active_write_authorization: Option<&'a WriteAuthorizationRecord>,
    effective_authorization_now: Option<DateTime<Utc>>,
    options: SummaryOptions,
}

fn build_state_summary(input: SummaryBuild<'_>) -> CoreResult<harness_types::StateSummary> {
    let SummaryBuild {
        project_id,
        state_version,
        task,
        current_change_unit,
        pending_user_judgment_refs,
        blocker_refs,
        active_write_authorization,
        effective_authorization_now,
        options,
    } = input;
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
            Some(record.basis_state_version.unwrap_or(state_version)),
        )
    });
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
    let write_authority_summary = if options.write_authority {
        active_write_authorization
            .map(|record| {
                write_authority_summary_for_record(
                    record,
                    state_version,
                    effective_authorization_now,
                )
            })
            .transpose()?
    } else {
        None
    };

    Ok(harness_types::StateSummary {
        project_id: project_id.clone(),
        state_version,
        task_ref: Some(task_ref),
        mode: parse_task_mode(&task.mode)?,
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
        goal_summary: scope.goal_summary,
        scope_summary: change_unit_scope.or(scope.scope_summary),
        non_goals: scope.non_goals,
        acceptance_criteria: scope.acceptance_criteria,
        autonomy_boundary: scope.autonomy_boundary,
        active_change_unit_ref,
        baseline_ref: scope.baseline_ref.map(BaselineRef::new),
        shaping_readiness: None,
        pending_user_judgment_refs: if options.pending_user_judgments {
            pending_user_judgment_refs
        } else {
            Vec::new()
        },
        blocker_refs: if options.blockers {
            blocker_refs
        } else {
            Vec::new()
        },
        write_authority_summary,
        evidence_summary: None,
        close_state: None,
        close_blockers: Vec::new(),
        guarantee_display: None,
    })
}

fn write_authority_summary_for_record(
    record: &WriteAuthorizationRecord,
    state_version: u64,
    now: Option<DateTime<Utc>>,
) -> CoreResult<WriteAuthoritySummary> {
    let attempt_scope = decode_required_json::<PersistedAuthorizedAttemptScope>(
        "write_authorizations",
        record.write_authorization_id.clone(),
        "attempt_scope_json",
        Some(&record.attempt_scope_json),
    )?;
    Ok(WriteAuthoritySummary {
        status: effective_write_authorization_status(record, state_version, now)?,
        write_authorization_ref: Some(write_authorization_ref(record, state_version)),
        basis_state_version: Some(record.basis_state_version),
        intended_paths: attempt_scope.intended_paths,
        guarantee_display: None,
    })
}

fn effective_write_authorization_status(
    record: &WriteAuthorizationRecord,
    state_version: u64,
    now: Option<DateTime<Utc>>,
) -> CoreResult<WriteAuthorizationStatus> {
    let stored_status = parse_storage_value("write_authorizations.status", &record.status)?;
    if stored_status != WriteAuthorizationStatus::Active {
        return Ok(stored_status);
    }
    if record.basis_state_version != state_version {
        return Ok(WriteAuthorizationStatus::Stale);
    }
    if now
        .map(|now| write_authorization_is_expired(record, now))
        .transpose()
        .map_err(CorePipelineError::from)?
        .unwrap_or(false)
    {
        Ok(WriteAuthorizationStatus::Expired)
    } else {
        Ok(WriteAuthorizationStatus::Active)
    }
}

fn status_guarantee_display() -> GuaranteeDisplay {
    GuaranteeDisplay {
        level: GuaranteeLevel::Cooperative,
        basis: "No stronger local guarantee is currently applied.".to_owned(),
        capability_refs: Vec::new(),
    }
}

fn change_unit_insert(
    request: &UpdateScopeRequest,
    change_unit_id: &ChangeUnitId,
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
            "baseline_ref": request.baseline_ref
        }))?,
        close_basis_json: "{}".to_owned(),
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
        basis_state_version: Some(planned_state_version),
        scope_summary_json: insert.scope_summary_json.clone(),
        bounded_paths_json: insert.bounded_paths_json.clone(),
        write_basis_json: insert.write_basis_json.clone(),
        close_basis_json: insert.close_basis_json.clone(),
        lifecycle_json: insert.lifecycle_json.clone(),
    }
}

fn task_shaping_json(
    goal_summary: Option<String>,
    scope_summary: Option<String>,
    non_goals: Vec<String>,
    acceptance_criteria: Vec<String>,
    baseline_ref: Option<String>,
    autonomy_boundary: Option<String>,
    initial_context_refs: Option<Value>,
) -> Value {
    json!({
        "goal_summary": goal_summary,
        "scope_summary": scope_summary,
        "non_goals": non_goals,
        "acceptance_criteria": acceptance_criteria,
        "baseline_ref": baseline_ref,
        "autonomy_boundary": autonomy_boundary,
        "initial_context_refs": initial_context_refs.unwrap_or(Value::Array(Vec::new()))
    })
}

fn next_actions_for_state(
    task_ref: &StateRecordRef,
    change_unit_ref: Option<&StateRecordRef>,
) -> Vec<NextActionSummary> {
    match change_unit_ref {
        Some(change_unit_ref) => vec![NextActionSummary {
            action_kind: NextActionKind::PrepareWrite,
            owner_method: Some(MethodName::PrepareWrite),
            label: "Check the current change against current scope.".to_owned(),
            blocking_question: None,
            required_refs: vec![task_ref.clone(), change_unit_ref.clone()],
        }],
        None => vec![NextActionSummary {
            action_kind: NextActionKind::UpdateScope,
            owner_method: Some(MethodName::UpdateScope),
            label: "Create the first currently applied Change Unit before write checking."
                .to_owned(),
            blocking_question: None,
            required_refs: vec![task_ref.clone()],
        }],
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
        state_version: state_version.into(),
    }
}

fn write_authorization_ref(
    record: &WriteAuthorizationRecord,
    state_version: u64,
) -> StateRecordRef {
    state_ref(
        StateRecordKind::WriteAuthorization,
        &record.write_authorization_id,
        &ProjectId::new(record.project_id.clone()),
        Some(&TaskId::new(record.task_id.clone())),
        Some(state_version),
    )
}

fn state_ref_from_stored(record: StoredRecordRef) -> StateRecordRef {
    let kind = match record.record_kind.as_str() {
        "user_judgment" => StateRecordKind::UserJudgment,
        "blocker" => StateRecordKind::Blocker,
        "write_authorization" => StateRecordKind::WriteAuthorization,
        "change_unit" => StateRecordKind::ChangeUnit,
        "task" => StateRecordKind::Task,
        _ => StateRecordKind::ProjectState,
    };
    StateRecordRef {
        record_kind: kind,
        record_id: RecordId::new(record.record_id),
        project_id: ProjectId::new(record.project_id),
        task_id: record.task_id.map(TaskId::new).into(),
        state_version: record.state_version.into(),
    }
}

fn stored_refs_to_state_refs(records: Vec<StoredRecordRef>) -> Vec<StateRecordRef> {
    records.into_iter().map(state_ref_from_stored).collect()
}

fn strip_base(value: Value) -> CoreResult<JsonObject> {
    let mut object = object_from_value(value)?;
    object.remove("base");
    Ok(object)
}

fn object_from_value(value: Value) -> CoreResult<JsonObject> {
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(CorePipelineError::InvalidDispatch {
            detail: "expected JSON object".to_owned(),
        }),
    }
}

fn placeholder_base() -> ToolResultBase {
    method_result_base(EffectKind::NoEffect, false, None, Vec::new())
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
    errors: Vec<harness_types::ToolError>,
) -> CoreResult<PipelineResponse> {
    let response = rejected_response(dry_run, state_version, errors);
    let response_value = serde_json::to_value(response)?;
    let response_json = serde_json::to_string(&response_value)?;
    Ok(PipelineResponse {
        response_json,
        response_value,
        verified_surface: None,
        resolved_task_id: None,
        replayed: false,
    })
}

fn infallible_rejected_pipeline_response(
    dry_run: bool,
    state_version: Option<u64>,
    errors: Vec<harness_types::ToolError>,
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

fn parse_task_mode(value: &str) -> CoreResult<Option<TaskMode>> {
    match value {
        "advisor" => Ok(Some(TaskMode::Advisor)),
        "direct" => Ok(Some(TaskMode::Direct)),
        "work" => Ok(Some(TaskMode::Work)),
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
    Ok(value.close_reason.unwrap_or(CloseReason::None))
}

fn invalid_storage<T>(field: &'static str) -> CoreResult<T> {
    Err(CorePipelineError::Store(StoreError::corrupt_stored_value(
        "project_state",
        field,
    )))
}

fn display_only_json_object_lossy(text: &str) -> JsonObject {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|value| match value {
            Value::Object(object) => Some(object),
            _ => None,
        })
        .unwrap_or_default()
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
