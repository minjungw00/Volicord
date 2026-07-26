use super::evidence_facts::{
    load_close_evidence_summary_facts, load_required_evidence_criterion_ids,
    projected_evidence_observation_provenance_facts, stored_evidence_observation_capture_relevance,
    stored_evidence_observation_provenance_facts,
};
use super::{
    acceptance_policy_storage, active_acceptance_criteria_for_task,
    agent_safe_pending_user_action_summaries, build_state_summary, change_unit_effect_contract,
    change_unit_ref, changes_summary_text, close_state_text, decode_required_json,
    decode_required_json_object, dry_run_summary, effective_write_ticket_status, elapsed_micros,
    evidence_gate_summary_text, evidence_summary_for_display, guarantee_display_from_profile,
    mutation_method_policy, no_active_task_response, normalize_close_blocker_action_projection,
    object_from_value, parse_acceptance_policy, parse_owner_storage_value, parse_task_mode,
    pending_user_action_authorities_for_plan, persistent_artifact_is_verified_current,
    plan_error_response, plan_project_continuity_record, prepare_or_response, primary_next_action,
    profile_summary_text, projected_write_ticket_summary, record_core_workflow_metric_best_effort,
    resolved_user_action_authorities_for_plan, response_committed_fresh_effect, state_ref,
    state_ref_from_stored, store_error_response, stored_refs_to_state_refs, summary_card_for_core,
    validation_rejected, write_ticket_is_current_for_projection, write_ticket_ref,
    write_ticket_summary_text, CloseTaskContext, CloseTaskPlan, PersistedLifecycleState, PlanError,
    PlannedProjectContinuityRecord, ProjectContinuityDraft, ProjectContinuityPlanContext,
    StoredScope, SummaryBuild, SummaryCardBuild,
};
use crate::pipeline::{
    CorePipelineError, CoreResult, CoreService, FreshnessPolicy, InvocationContext,
    MethodEffectPolicy, MethodPolicy, OwnerPipelineBranch, PipelineResponse, ReplayPolicy,
    TaskRequirement, VerifiedInvocationContext,
};
use crate::policy::close_readiness::{
    close_blocker, close_next_action, current_cancellation_authority, current_final_acceptance,
    current_residual_risk_acceptance_coverage, final_acceptance_requirement, is_terminal_lifecycle,
    user_action_has_current_basis, verified_user_channel_provenance,
    CancellationAuthorityRequirement, UserActionAuthority,
};
use crate::policy::evidence::{
    evidence_item_related_refs, state_record_ref_identity_key, unique_state_record_refs,
};
use crate::policy::path::{path_is_within, paths_are_authorized};
use crate::policy::user_action_relevance::{
    user_action_blocks_operation, user_action_required_for, UserActionOperation,
    UserActionOperationContext,
};
use crate::policy::workflow::{
    acceptance_policy_for_control, parse_task_control_level, project_workflow_policy,
    resolve_task_control_authority, ProjectWorkflowPolicy,
};
use crate::policy::write_ticket::{current_sensitive_approval, SensitiveApprovalRequirement};
use crate::policy::{
    close_readiness_evidence::{
        evaluate_evidence_gate, interpret_close_evidence_item, project_close_evidence_summary,
        CloseEvidenceIssueKind, CloseEvidenceObservationDisposition,
    },
    evidence_provenance::{classify_evidence_provenance, EvidenceProvenanceClass},
    evidence_relevance::capture_relevance_is_unsupported,
    evidence_target::{
        close_basis_is_current, close_basis_run_refs, projected_observation_matches_close_basis,
        run_record_matches_close_basis_context, stored_observation_matches_close_basis,
        EvidenceObservationBasis,
    },
    workflow::ResolvedTaskControlAuthority,
};
use serde_json::json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use volicord_store::core_pipeline::{
    CoreProjectStore, CoreStorageMutation, ProjectStateHeader, TaskCloseUpdate,
    TaskControlLevelUpdate, TaskMutation, TaskRecord, WriteTicketInvalidation, WriteTicketMutation,
};
use volicord_store::diagnostics::WorkflowMetricKind;
use volicord_store::guards::UnrecordedChangeRecord;
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::{BaselineRef, ChangeUnitId, TaskId};
use volicord_types::methods::{CheckCloseRequest, CloseTaskRequest, CloseTaskResultFields};
use volicord_types::schema::{
    AuthorityReceipt, CloseReadinessBlocker, CurrentCloseBasis, DryRunSummary,
    EvidenceCoverageItem, EvidenceGateSummary, EvidenceTarget, JsonObject, NextActionSummary,
    ProjectEnforcementProfile, RequiredNullable, RiskAcceptanceCoverage, StateRecordRef,
    ToolEnvelope, WriteTicketValidityBasis,
};
use volicord_types::values::{
    AcceptancePolicy, ActorSource, ArtifactAvailability, ArtifactIntegrityStatus,
    AuthorityNextActor, ChangeUnitEffectKind, CloseIntent, CloseReadinessBlockerCategory,
    CloseReason, CloseState, EvidenceCoverageState, EvidenceRelevanceStatus, EvidenceRequirement,
    JudgmentKind, JudgmentResolutionOutcome, MethodName, NextActionKind,
    NextActionPresentationRole, OperationCategory, ProjectContinuityKind, RedactionState,
    StateRecordKind, StatusCloseState, TaskControlLevel, TaskMode, UserActionKind,
    UserActionRequiredFor, UtcTimestamp, WriteTicketInvalidationReason, WriteTicketStatus,
};

/// Public close-family input before request-local identity and intent validation.
enum CloseTaskRawRequest {
    Check(CheckCloseRequest),
    Mutating(CloseTaskRequest),
}

impl CloseTaskRawRequest {
    fn request_json(&self) -> CoreResult<Value> {
        match self {
            Self::Check(request) => serde_json::to_value(request).map_err(CorePipelineError::from),
            Self::Mutating(request) => {
                serde_json::to_value(request).map_err(CorePipelineError::from)
            }
        }
    }

    fn normalize(self) -> CoreResult<Result<CloseTaskPlanRequest, PipelineResponse>> {
        match self {
            Self::Check(request) => {
                if let Some(response) = validate_close_task_identity(
                    &request.envelope,
                    &request.task_id,
                    "envelope.task_id must match CheckCloseRequest.task_id",
                    "check_close requires envelope.task_id to identify the Task",
                )? {
                    return Ok(Err(response));
                }
                Ok(Ok(CloseTaskPlanRequest::check(request)))
            }
            Self::Mutating(request) => {
                if let Some(response) = validate_close_task_identity(
                    &request.envelope,
                    &request.task_id,
                    "envelope.task_id must match CloseTaskRequest.task_id",
                    "close_task requires envelope.task_id to identify the Task being closed",
                )? {
                    return Ok(Err(response));
                }
                let request = CloseTaskPlanRequest::mutating(request);
                if let Some(response) = validate_close_intent_fields(&request)? {
                    return Ok(Err(response));
                }
                Ok(Ok(request))
            }
        }
    }
}

impl CoreService {
    /// Executes `volicord.check_close` through read-only close-readiness rules.
    pub fn check_close(
        &self,
        request: CheckCloseRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let raw = CloseTaskRawRequest::Check(request);
        let request_json = raw.request_json()?;
        let request = match raw.normalize()? {
            Ok(request) => request,
            Err(response) => return Ok(response),
        };
        let close_policy = check_close_policy(&request);
        let prepared = match prepare_or_response(
            self,
            None,
            MethodName::CheckClose,
            request.envelope.clone(),
            request_json,
            invocation,
            close_policy,
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan_now = prepared.operation_now.clone();

        let guarantee_profile = match prepared.store.project_enforcement_profile() {
            Ok(record) => record.profile,
            Err(error) => {
                return plan_error_response(
                    &request.envelope,
                    &prepared.context.project_state,
                    PlanError::Core(CorePipelineError::from(error)),
                )
            }
        };
        let plan = match plan_close_task(
            &prepared.store,
            &prepared.context.project_state,
            Some(&prepared.context.verified_invocation),
            Some(&guarantee_profile),
            request.clone(),
            &plan_now,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                return plan_error_response(
                    &request.envelope,
                    &prepared.context.project_state,
                    error,
                )
            }
        };
        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::ReadOnly {
                result_fields: plan.result_fields,
            },
        )
    }

    /// Executes `volicord.close_task` through terminal transition rules.
    pub fn close_task(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        request: CloseTaskRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let raw = CloseTaskRawRequest::Mutating(request);
        let request_json = raw.request_json()?;
        let request = match raw.normalize()? {
            Ok(request) => request,
            Err(response) => return Ok(response),
        };
        let close_policy = close_task_policy(&request);
        let prepared = match prepare_or_response(
            self,
            Some(context),
            MethodName::CloseTask,
            request.envelope.clone(),
            request_json,
            invocation,
            close_policy,
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan_now = prepared.operation_now.clone();

        if request.envelope.dry_run {
            return self.execute_prepared_request::<CloseTaskResultFields>(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: close_task_dry_run_summary(request.intent),
                },
            );
        }

        let guarantee_profile = match prepared.store.project_enforcement_profile() {
            Ok(record) => record.profile,
            Err(error) => {
                return plan_error_response(
                    &request.envelope,
                    &prepared.context.project_state,
                    PlanError::Core(CorePipelineError::from(error)),
                )
            }
        };
        let mut plan = match plan_close_task(
            &prepared.store,
            &prepared.context.project_state,
            Some(&prepared.context.verified_invocation),
            Some(&guarantee_profile),
            request.clone(),
            &plan_now,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                return plan_error_response(
                    &request.envelope,
                    &prepared.context.project_state,
                    error,
                )
            }
        };

        if !plan.blockers.is_empty() {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::NoEffectResult {
                    result_fields: plan.result_fields,
                },
            );
        }

        let continuity_plans = match plan_close_completion_continuity_records(
            self,
            &prepared.store,
            &request,
            plan.current_close_basis.as_ref(),
            prepared.context.project_state.state_version + 1,
            &plan_now,
        ) {
            Ok(records) => records,
            Err(error) => {
                return plan_error_response(
                    &request.envelope,
                    &prepared.context.project_state,
                    error,
                )
            }
        };
        if !continuity_plans.is_empty() {
            let continuity_summary = continuity_plans
                .iter()
                .map(|plan| plan.summary.clone())
                .collect::<Vec<_>>();
            plan.result_fields.continuity_summary = continuity_summary;
            let continuity_record_ids = continuity_plans
                .iter()
                .map(|plan| plan.record_ref.record_id.as_str().to_owned())
                .collect::<Vec<_>>();
            plan.event_payload.insert(
                "continuity_record_ids".to_owned(),
                serde_json::to_value(&continuity_record_ids)?,
            );
            plan.storage_mutations
                .extend(continuity_plans.into_iter().map(|plan| plan.mutation));
        }

        let task_duration = prepared
            .store
            .task_created_at(&request.task_id)
            .ok()
            .flatten()
            .and_then(|created_at| elapsed_micros(&created_at, &plan_now));
        let session_id = prepared.context.verified_invocation.session_id.clone();
        let response = self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: plan.event_kind,
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            },
        )?;
        if response_committed_fresh_effect(&response) {
            if let Some(duration) = task_duration {
                record_core_workflow_metric_best_effort(
                    context,
                    session_id.as_deref(),
                    WorkflowMetricKind::TaskDurationMicros,
                    duration,
                );
            }
        }
        Ok(response)
    }
}

/// Canonical request after the Task identity and intent-field combination are valid.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CloseTaskPlanRequest {
    envelope: ToolEnvelope,
    task_id: TaskId,
    intent: CloseIntent,
    close_reason: RequiredNullable<CloseReason>,
    superseding_task_id: RequiredNullable<TaskId>,
    user_note: RequiredNullable<String>,
}

impl CloseTaskPlanRequest {
    pub(super) fn check(request: CheckCloseRequest) -> Self {
        let task_id = request.task_id;
        let mut envelope = request.envelope;
        envelope.task_id = Some(task_id.clone()).into();
        Self {
            envelope,
            task_id,
            intent: CloseIntent::Check,
            close_reason: RequiredNullable::null(),
            superseding_task_id: RequiredNullable::null(),
            user_note: RequiredNullable::null(),
        }
    }

    fn mutating(request: CloseTaskRequest) -> Self {
        Self {
            envelope: request.envelope,
            task_id: request.task_id,
            intent: request.intent.into(),
            close_reason: request.close_reason,
            superseding_task_id: request.superseding_task_id,
            user_note: request.user_note,
        }
    }

    fn operation_category(&self) -> OperationCategory {
        match self.intent {
            CloseIntent::Check => OperationCategory::Read,
            CloseIntent::Complete | CloseIntent::Cancel | CloseIntent::Supersede => {
                OperationCategory::AgentWorkflow
            }
        }
    }
}

/// Store-backed close context with typed policy authority and control values.
struct CloseTaskResolvedContext {
    request: CloseTaskPlanRequest,
    context: CloseTaskContext,
    workflow_policy: ProjectWorkflowPolicy,
    current_control: TaskControlLevel,
    resolved_control: ResolvedTaskControlAuthority,
    current_acceptance: AcceptancePolicy,
    sensitive_effect: bool,
}

/// Close-readiness decision with canonical blockers and the selected result state.
struct CloseTaskPolicyDecision {
    request: CloseTaskPlanRequest,
    context: CloseTaskContext,
    control_update: Option<TaskControlLevelUpdate>,
    risk_acceptance_coverage: Vec<RiskAcceptanceCoverage>,
    blockers: Vec<CloseReadinessBlocker>,
    committed_terminal: bool,
    response_state_version: u64,
    close_state: CloseState,
}

/// Exact terminal mutations and immutable event selected by an allowed decision.
struct CloseTaskPlannedMutations {
    request: CloseTaskPlanRequest,
    context: CloseTaskContext,
    risk_acceptance_coverage: Vec<RiskAcceptanceCoverage>,
    blockers: Vec<CloseReadinessBlocker>,
    response_state_version: u64,
    close_state: CloseState,
    synthetic_task: TaskRecord,
    storage_mutations: Vec<CoreStorageMutation>,
    event_kind: String,
    event_payload: JsonObject,
}

/// Transport-independent typed response retained until final plan serialization.
struct CloseTaskResponseProjection {
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

impl CloseTaskResponseProjection {
    fn into_plan(self) -> CloseTaskPlan {
        CloseTaskPlan {
            task_id: self.task_id,
            change_unit_id: self.change_unit_id,
            storage_mutations: self.storage_mutations,
            event_kind: self.event_kind,
            event_payload: self.event_payload,
            result_fields: self.result_fields,
            close_state: self.close_state,
            current_close_basis: self.current_close_basis,
            risk_acceptance_coverage: self.risk_acceptance_coverage,
            blockers: self.blockers,
            evidence_gate: self.evidence_gate,
        }
    }
}

fn validate_close_task_identity(
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    mismatch_message: &'static str,
    missing_message: &'static str,
) -> CoreResult<Option<PipelineResponse>> {
    if let Some(envelope_task_id) = envelope.task_id.as_ref() {
        if envelope_task_id == task_id {
            return Ok(None);
        }
        return validation_rejected(envelope.dry_run, None, "task_id", mismatch_message).map(Some);
    }
    validation_rejected(envelope.dry_run, None, "envelope.task_id", missing_message).map(Some)
}

fn close_acceptance_policy_rank(policy: AcceptancePolicy) -> u8 {
    match policy {
        AcceptancePolicy::NotRequired => 0,
        AcceptancePolicy::PolicyDependent => 1,
        AcceptancePolicy::Required => 2,
    }
}

fn check_close_policy(request: &CloseTaskPlanRequest) -> MethodPolicy {
    MethodPolicy::exact(
        OperationCategory::Read,
        TaskRequirement::Exact(request.task_id.clone()),
        ReplayPolicy::None,
        FreshnessPolicy::None,
        MethodEffectPolicy::ReadOnly,
    )
}

fn close_task_policy(request: &CloseTaskPlanRequest) -> MethodPolicy {
    mutation_method_policy(
        request.operation_category(),
        TaskRequirement::Exact(request.task_id.clone()),
        request.envelope.dry_run,
    )
}

fn validate_close_intent_fields(
    request: &CloseTaskPlanRequest,
) -> CoreResult<Option<PipelineResponse>> {
    let invalid = |field, message| {
        validation_rejected(request.envelope.dry_run, None, field, message).map(Some)
    };
    match request.intent {
        CloseIntent::Check => {
            if request.close_reason.is_some() {
                return invalid(
                    "close_reason",
                    "volicord.check_close must not include close_reason",
                );
            }
            if request.superseding_task_id.is_some() {
                return invalid(
                    "superseding_task_id",
                    "volicord.check_close must not include superseding_task_id",
                );
            }
        }
        CloseIntent::Complete => {
            if !matches!(
                request.close_reason.as_ref(),
                Some(CloseReason::CompletedSelfChecked | CloseReason::CompletedWithRiskAccepted)
            ) {
                return invalid(
                    "close_reason",
                    "intent=complete requires a completion close_reason",
                );
            }
            if request.superseding_task_id.is_some() {
                return invalid(
                    "superseding_task_id",
                    "intent=complete must not include superseding_task_id",
                );
            }
        }
        CloseIntent::Cancel => {
            if request.close_reason.as_ref() != Some(&CloseReason::Cancelled) {
                return invalid(
                    "close_reason",
                    "intent=cancel requires close_reason=cancelled",
                );
            }
            if request.superseding_task_id.is_some() {
                return invalid(
                    "superseding_task_id",
                    "intent=cancel must not include superseding_task_id",
                );
            }
        }
        CloseIntent::Supersede => {
            if request.close_reason.as_ref() != Some(&CloseReason::Superseded) {
                return invalid(
                    "close_reason",
                    "intent=supersede requires close_reason=superseded",
                );
            }
            let Some(superseding_task_id) = request.superseding_task_id.as_ref() else {
                return invalid(
                    "superseding_task_id",
                    "intent=supersede requires superseding_task_id",
                );
            };
            if superseding_task_id == &request.task_id {
                return invalid(
                    "superseding_task_id",
                    "superseding_task_id must identify a different Task",
                );
            }
        }
    }
    Ok(None)
}

fn close_task_dry_run_summary(intent: CloseIntent) -> DryRunSummary {
    let (action, description) = match intent {
        CloseIntent::Check => (
            "would_check",
            "Close readiness check would read the current Task state.",
        ),
        CloseIntent::Complete => (
            "would_complete",
            "Close task would attempt the complete terminal transition.",
        ),
        CloseIntent::Cancel => (
            "would_cancel",
            "Close task would attempt the cancel terminal transition.",
        ),
        CloseIntent::Supersede => (
            "would_supersede",
            "Close task would attempt the supersede terminal transition.",
        ),
    };
    dry_run_summary("task", action, description, Vec::new())
}

pub(super) fn plan_close_task(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: Option<&VerifiedInvocationContext>,
    guarantee_profile: Option<&ProjectEnforcementProfile>,
    request: CloseTaskPlanRequest,
    now: &UtcTimestamp,
) -> Result<CloseTaskPlan, PlanError> {
    let context = load_close_task_context(store, project_state, &request, now)?;
    plan_close_task_with_context(
        store,
        project_state,
        verified_invocation,
        guarantee_profile,
        request,
        now,
        context,
    )
}

pub(super) fn plan_close_task_with_context(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: Option<&VerifiedInvocationContext>,
    guarantee_profile: Option<&ProjectEnforcementProfile>,
    request: CloseTaskPlanRequest,
    now: &UtcTimestamp,
    context: CloseTaskContext,
) -> Result<CloseTaskPlan, PlanError> {
    let resolved = resolve_close_task_context(store, request, context)?;
    let decision = decide_close_task_policy(store, project_state, now, resolved)?;
    let mutations = plan_close_task_mutations(now, decision)?;
    Ok(project_close_task_response(
        store,
        verified_invocation,
        guarantee_profile,
        now,
        mutations,
    )?
    .into_plan())
}

fn resolve_close_task_context(
    store: &CoreProjectStore,
    request: CloseTaskPlanRequest,
    context: CloseTaskContext,
) -> Result<CloseTaskResolvedContext, PlanError> {
    let workflow_policy = project_workflow_policy(store).map_err(CorePipelineError::from)?;
    let current_control = parse_task_control_level(&context.task.effective_control_level)
        .map_err(CorePipelineError::from)?;
    let resolved_control = resolve_task_control_authority(&context.task, &workflow_policy)
        .map_err(CorePipelineError::from)?;
    let sensitive_effect = context
        .current_change_unit
        .as_ref()
        .map(change_unit_effect_contract)
        .transpose()?
        .flatten()
        .is_some_and(|contract| {
            !contract.sensitive_action_expectations.is_empty()
                || contract.allowed_effects.iter().any(|effect| {
                    matches!(
                        effect,
                        ChangeUnitEffectKind::SensitiveAction
                            | ChangeUnitEffectKind::ExternalNetwork
                            | ChangeUnitEffectKind::SecretAccess
                    )
                })
        });
    let current_acceptance = parse_acceptance_policy(&context.task.acceptance_policy)?;

    Ok(CloseTaskResolvedContext {
        request,
        context,
        workflow_policy,
        current_control,
        resolved_control,
        current_acceptance,
        sensitive_effect,
    })
}

fn decide_close_task_policy(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    now: &UtcTimestamp,
    resolved: CloseTaskResolvedContext,
) -> Result<CloseTaskPolicyDecision, PlanError> {
    let CloseTaskResolvedContext {
        request,
        mut context,
        workflow_policy,
        current_control,
        resolved_control,
        current_acceptance,
        sensitive_effect,
    } = resolved;
    let next_control = if sensitive_effect {
        TaskControlLevel::Sensitive
    } else {
        resolved_control.effective_control_level
    };
    let control_acceptance = acceptance_policy_for_control(next_control, &workflow_policy);
    let next_acceptance = if close_acceptance_policy_rank(resolved_control.acceptance_policy)
        >= close_acceptance_policy_rank(control_acceptance)
    {
        resolved_control.acceptance_policy
    } else {
        control_acceptance
    };
    let acceptance_raised = close_acceptance_policy_rank(next_acceptance)
        > close_acceptance_policy_rank(current_acceptance);
    let control_raised = next_control > current_control;
    let control_update = (control_raised || acceptance_raised).then(|| {
        let reason = if sensitive_effect && control_raised {
            "Core raised control to `sensitive` for the current Change Unit effect contract."
                .to_owned()
        } else if control_raised {
            resolved_control.control_level_reason.clone()
        } else {
            context.task.control_level_reason.clone()
        };
        context.task.effective_control_level = next_control.as_str().to_owned();
        context.task.control_level_reason = reason.clone();
        if acceptance_raised {
            context.task.acceptance_policy = acceptance_policy_storage(next_acceptance).to_owned();
            context.task.acceptance_policy_reason = if next_control
                == resolved_control.effective_control_level
                && resolved_control.acceptance_raised
            {
                resolved_control.acceptance_policy_reason.clone()
            } else {
                format!(
                    "Effective control `{}` requires final acceptance for the current close basis.",
                    next_control.as_str()
                )
            };
        }
        TaskControlLevelUpdate {
            task_id: context.task.task_id.clone(),
            effective_control_level: next_control.as_str().to_owned(),
            control_level_reason: reason,
            acceptance_policy: acceptance_raised
                .then(|| acceptance_policy_storage(next_acceptance).to_owned()),
            acceptance_policy_reason: acceptance_raised
                .then(|| context.task.acceptance_policy_reason.clone()),
        }
    });

    let risk_acceptance_coverage =
        risk_acceptance_coverage(store, project_state, &request, &context)?;
    let mut blockers = terminal_close_blockers(store, project_state, &request, &context, now)?;
    if matches!(request.intent, CloseIntent::Check | CloseIntent::Complete) {
        blockers.extend(completion_close_blockers(
            store,
            project_state,
            &request,
            &context,
            &risk_acceptance_coverage,
            now,
        )?);
    }
    blockers.extend(unrecorded_change_close_blockers(
        store,
        project_state,
        &request,
        &context,
    )?);
    normalize_close_blocker_action_projection(&mut blockers, project_state.state_version);

    let committed_terminal = request.intent != CloseIntent::Check && blockers.is_empty();
    let response_state_version = if committed_terminal {
        project_state.state_version + 1
    } else {
        project_state.state_version
    };
    let close_state = close_state_for_policy(request.intent, blockers.is_empty());

    Ok(CloseTaskPolicyDecision {
        request,
        context,
        control_update,
        risk_acceptance_coverage,
        blockers,
        committed_terminal,
        response_state_version,
        close_state,
    })
}

fn close_state_for_policy(intent: CloseIntent, allowed: bool) -> CloseState {
    if !allowed {
        return CloseState::Blocked;
    }
    match intent {
        CloseIntent::Check => CloseState::Ready,
        CloseIntent::Complete => CloseState::Closed,
        CloseIntent::Cancel => CloseState::Cancelled,
        CloseIntent::Supersede => CloseState::Superseded,
    }
}

fn plan_close_task_mutations(
    now: &UtcTimestamp,
    decision: CloseTaskPolicyDecision,
) -> Result<CloseTaskPlannedMutations, PlanError> {
    let CloseTaskPolicyDecision {
        request,
        context,
        control_update,
        risk_acceptance_coverage,
        blockers,
        committed_terminal,
        response_state_version,
        close_state,
    } = decision;
    let mut synthetic_task = context.task.clone();
    let mut storage_mutations = Vec::new();
    if committed_terminal {
        storage_mutations.extend(
            control_update
                .map(|input| CoreStorageMutation::Task(TaskMutation::UpdateControlLevel(input))),
        );
    }
    let mut event_kind = String::new();
    let mut event_payload = Map::new();

    if committed_terminal {
        let terminal = close_terminal_storage(request.intent, parse_task_mode(&context.task.mode)?);
        let close_summary_json = terminal_close_summary_json(&context.task, &request, now)?;
        synthetic_task.lifecycle_phase = terminal.lifecycle_phase.to_owned();
        synthetic_task.result = Some(terminal.result.to_owned());
        synthetic_task.close_summary_json = close_summary_json.clone();
        synthetic_task.closed_at = Some(now.to_string());
        storage_mutations.push(CoreStorageMutation::WriteTicket(
            WriteTicketMutation::InvalidateActive(WriteTicketInvalidation {
                task_id: request.task_id.as_str().to_owned(),
                invalidation_reason: WriteTicketInvalidationReason::TaskClosed
                    .as_str()
                    .to_owned(),
            }),
        ));
        storage_mutations.push(CoreStorageMutation::Task(TaskMutation::Close(
            TaskCloseUpdate {
                task_id: request.task_id.as_str().to_owned(),
                lifecycle_phase: terminal.lifecycle_phase.to_owned(),
                result: terminal.result.to_owned(),
                close_summary_json,
                closed_at: now.to_string(),
            },
        )));
        if request.intent == CloseIntent::Supersede {
            if let Some(superseding_task_id) = request.superseding_task_id.as_ref() {
                storage_mutations.push(CoreStorageMutation::Task(TaskMutation::SetActive {
                    task_id: superseding_task_id.as_str().to_owned(),
                }));
            }
        }
        event_kind = terminal.event_kind.to_owned();
        event_payload = object_from_value(json!({
            "task_id": request.task_id,
            "intent": request.intent,
            "close_reason": request.close_reason,
            "superseding_task_id": request.superseding_task_id,
            "user_note": request.user_note,
            "closed_at": now
        }))?;
    }

    Ok(CloseTaskPlannedMutations {
        request,
        context,
        risk_acceptance_coverage,
        blockers,
        response_state_version,
        close_state,
        synthetic_task,
        storage_mutations,
        event_kind,
        event_payload,
    })
}

fn project_close_task_response(
    store: &CoreProjectStore,
    verified_invocation: Option<&VerifiedInvocationContext>,
    guarantee_profile: Option<&ProjectEnforcementProfile>,
    now: &UtcTimestamp,
    planned: CloseTaskPlannedMutations,
) -> Result<CloseTaskResponseProjection, PlanError> {
    let CloseTaskPlannedMutations {
        request,
        context,
        risk_acceptance_coverage,
        mut blockers,
        response_state_version,
        close_state,
        synthetic_task,
        storage_mutations,
        event_kind,
        event_payload,
    } = planned;
    let guarantee_display = match (verified_invocation, guarantee_profile) {
        (Some(invocation), Some(profile)) => Some(guarantee_display_from_profile(
            profile,
            invocation,
            response_state_version,
        )),
        _ => None,
    };

    let current_close_basis = context.current_close_basis.clone();
    let evidence_summary = context
        .evidence_summary
        .clone()
        .map(|summary| evidence_summary_for_display(summary, current_close_basis.as_ref()));
    let acceptance_criteria = active_acceptance_criteria_for_task(store, &request.task_id)?;
    let evidence_gate =
        evaluate_evidence_gate(&acceptance_criteria, evidence_summary.as_ref(), &blockers);

    let current_close_pending_user_action_ids = blockers
        .iter()
        .filter(|blocker| blocker.code == "pending_user_action")
        .flat_map(|blocker| blocker.related_refs.iter())
        .filter(|record_ref| record_ref.record_kind == StateRecordKind::UserActionRequest)
        .map(|record_ref| record_ref.record_id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    let pending_user_action_summaries = agent_safe_pending_user_action_summaries(
        context
            .pending_user_action_refs
            .iter()
            .filter(|record_ref| {
                current_close_pending_user_action_ids.contains(record_ref.record_id.as_str())
            })
            .cloned(),
    );
    for blocker in &mut blockers {
        if blocker.code != "pending_user_action" {
            continue;
        }
        blocker
            .related_refs
            .retain(|record_ref| record_ref.record_kind != StateRecordKind::UserActionRequest);
        for action in &mut blocker.next_actions {
            action.blocking_question = None;
            action
                .required_refs
                .retain(|record_ref| record_ref.record_kind != StateRecordKind::UserActionRequest);
        }
    }

    let state = build_state_summary(SummaryBuild {
        store,
        project_id: &request.envelope.project_id,
        state_version: response_state_version,
        task: &synthetic_task,
        current_change_unit: context.current_change_unit.as_ref(),
        acceptance_criteria,
        pending_user_action_refs: context.pending_user_action_refs.clone(),
        blocker_refs: context.blocker_refs.clone(),
        write_ticket_summary: projected_write_ticket_summary(
            store,
            &request.task_id,
            response_state_version,
            *now.as_datetime(),
            guarantee_display.clone(),
        )?,
        evidence_summary: evidence_summary.clone(),
        evidence_gate: Some(evidence_gate),
        close_state: Some(close_state),
        close_blockers: blockers.clone(),
        guarantee_display,
    })?;

    let artifact_refs = context.artifact_refs.clone();
    let no_next_actions: &[NextActionSummary] = &[];
    let summary_card = summary_card_for_core(SummaryCardBuild {
        task: Some(&synthetic_task),
        recording: if storage_mutations.is_empty() {
            "read_only"
        } else {
            "core_committed"
        },
        profile: profile_summary_text(state.guarantee_display.as_ref()),
        write_ticket: write_ticket_summary_text(true, state.write_ticket_summary.as_ref()),
        evidence: evidence_gate_summary_text(true, state.evidence_gate.as_ref()),
        pending_user_actions: state.pending_user_action_summaries.len(),
        changes: changes_summary_text(
            true,
            unresolved_unrecorded_changes(store, &request, &context)?.len() as u64,
        ),
        close_status: close_state_text(close_state).to_owned(),
        verified_invocation: verified_invocation
            .expect("close task result planning requires verified invocation context"),
        next_action: primary_next_action(no_next_actions, &blockers),
    });
    let task_ref = state_ref(
        StateRecordKind::Task,
        request.task_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(response_state_version),
    );
    let change_unit_ref = context.current_change_unit.as_ref().map(|record| {
        state_ref(
            StateRecordKind::ChangeUnit,
            &record.change_unit_id,
            &request.envelope.project_id,
            Some(&request.task_id),
            Some(response_state_version),
        )
    });
    let latest_run = store
        .run_observed_changes_for_task(&request.task_id)
        .map_err(CorePipelineError::from)?
        .into_iter()
        .find(|record| record.status == "recorded");
    let latest_run_ref = latest_run.as_ref().map(|record| {
        state_ref(
            StateRecordKind::Run,
            &record.run_id,
            &request.envelope.project_id,
            Some(&request.task_id),
            Some(response_state_version),
        )
    });
    let product_file_write_observed = latest_run
        .as_ref()
        .is_some_and(|record| record.observed_changes.product_file_write_observed);
    let next_action = if blockers.is_empty() && close_state == CloseState::Ready {
        Some(close_next_action(
            "Complete the current Task.",
            vec![task_ref.clone()],
        ))
    } else {
        primary_next_action(no_next_actions, &blockers).cloned()
    };
    let next_actor = next_action
        .as_ref()
        .map(|action| {
            if action
                .allowed_operation_categories
                .contains(&OperationCategory::UserOnly)
            {
                AuthorityNextActor::User
            } else if action
                .allowed_operation_categories
                .contains(&OperationCategory::AgentWorkflow)
            {
                AuthorityNextActor::Agent
            } else {
                AuthorityNextActor::None
            }
        })
        .unwrap_or(AuthorityNextActor::None);
    let authority_receipt = AuthorityReceipt {
        project_id: request.envelope.project_id.clone(),
        state_version: response_state_version,
        task_ref,
        change_unit_ref,
        scope_revision: synthetic_task.scope_revision,
        latest_run_ref,
        product_file_write_observed,
        evidence_gate: Some(evidence_gate),
        close_state: match close_state {
            CloseState::Ready => StatusCloseState::Ready,
            CloseState::Blocked => StatusCloseState::Blocked,
            CloseState::Closed => StatusCloseState::Closed,
            CloseState::Cancelled => StatusCloseState::Cancelled,
            CloseState::Superseded => StatusCloseState::Superseded,
        },
        close_blockers: blockers.clone(),
        completion_claim_allowed: current_close_basis.is_some()
            && blockers.is_empty()
            && matches!(close_state, CloseState::Ready | CloseState::Closed),
        next_actor,
        next_action,
    };
    let result_fields = CloseTaskResultFields {
        summary_card,
        close_state,
        current_close_basis: current_close_basis.clone(),
        risk_acceptance_coverage: risk_acceptance_coverage.clone(),
        continuity_summary: Vec::new(),
        state,
        blockers: blockers.clone(),
        pending_user_action_summaries,
        evidence_summary: evidence_summary.clone(),
        evidence_gate,
        artifact_refs,
        authority_receipt,
    };
    let change_unit_id = context
        .current_change_unit
        .as_ref()
        .map(|record| ChangeUnitId::new(record.change_unit_id.clone()));

    Ok(CloseTaskResponseProjection {
        task_id: request.task_id,
        change_unit_id,
        storage_mutations,
        event_kind,
        event_payload,
        result_fields,
        close_state,
        current_close_basis,
        risk_acceptance_coverage,
        blockers,
        evidence_gate,
    })
}

fn plan_close_completion_continuity_records(
    service: &CoreService,
    store: &CoreProjectStore,
    request: &CloseTaskPlanRequest,
    close_basis: Option<&CurrentCloseBasis>,
    planned_state_version: u64,
    now: &UtcTimestamp,
) -> Result<Vec<PlannedProjectContinuityRecord>, PlanError> {
    if request.intent != CloseIntent::Complete {
        return Ok(Vec::new());
    }
    let Some(close_basis) = close_basis else {
        return Ok(Vec::new());
    };
    let source_change_unit_id = Some(close_basis.change_unit_id.clone());
    let continuity_context = ProjectContinuityPlanContext {
        service,
        store,
        project_id: &request.envelope.project_id,
        source_task_id: &request.task_id,
        source_change_unit_id: source_change_unit_id.as_ref(),
        planned_state_version,
        now,
    };
    let mut records = Vec::new();
    for risk in close_basis
        .residual_risks
        .iter()
        .filter(|risk| !risk.acceptance_required)
    {
        let draft = ProjectContinuityDraft {
            kind: ProjectContinuityKind::KnownLimit,
            title: format!("Known limit: {}", risk.summary.trim().to_owned()),
            summary: risk.summary.clone(),
            rationale: Some(format!(
                "{} Consequence: {}",
                close_basis.result_summary, risk.consequence
            )),
            applies_to_paths: Vec::new(),
            applies_to_refs: refs_with_context(
                close_basis.result_refs.clone(),
                risk.source_refs.clone(),
            ),
            source_refs: refs_with_context(
                vec![close_basis.source_run_ref.clone()],
                risk.source_refs.clone(),
            ),
            artifact_refs: Vec::new(),
            supersedes_refs: Vec::new(),
            review_triggers: Vec::new(),
            metadata: json!({
                "source": "close_task",
                "risk_id": risk.risk_id,
                "close_basis_revision": close_basis.close_basis_revision
            }),
        };
        records.push(
            plan_project_continuity_record(continuity_context, draft).map_err(PlanError::Core)?,
        );
    }
    Ok(records)
}

struct CloseTerminalStorage {
    lifecycle_phase: &'static str,
    result: &'static str,
    event_kind: &'static str,
}

fn close_terminal_storage(intent: CloseIntent, task_mode: TaskMode) -> CloseTerminalStorage {
    match intent {
        CloseIntent::Complete => CloseTerminalStorage {
            lifecycle_phase: "completed",
            result: if task_mode == TaskMode::Advisor {
                "advice_only"
            } else {
                "completed"
            },
            event_kind: "task_completed",
        },
        CloseIntent::Cancel => CloseTerminalStorage {
            lifecycle_phase: "cancelled",
            result: "cancelled",
            event_kind: "task_cancelled",
        },
        CloseIntent::Supersede => CloseTerminalStorage {
            lifecycle_phase: "superseded",
            result: "superseded",
            event_kind: "task_superseded",
        },
        CloseIntent::Check => CloseTerminalStorage {
            lifecycle_phase: "ready",
            result: "none",
            event_kind: "task_close_checked",
        },
    }
}

fn terminal_close_summary_json(
    task: &TaskRecord,
    request: &CloseTaskPlanRequest,
    closed_at: &UtcTimestamp,
) -> CoreResult<String> {
    let mut close_summary = decode_required_json_object(
        "tasks",
        task.task_id.clone(),
        "close_summary_json",
        Some(&task.close_summary_json),
    )?;
    close_summary.insert(
        "close_reason".to_owned(),
        serde_json::to_value(
            request
                .close_reason
                .as_ref()
                .expect("validated terminal close_reason is present"),
        )?,
    );
    close_summary.insert("closed_at".to_owned(), serde_json::to_value(closed_at)?);
    close_summary.insert("intent".to_owned(), serde_json::to_value(request.intent)?);
    close_summary.insert(
        "user_note".to_owned(),
        request
            .user_note
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    close_summary.insert(
        "superseding_task_id".to_owned(),
        request
            .superseding_task_id
            .as_ref()
            .map(|id| Value::String(id.as_str().to_owned()))
            .unwrap_or(Value::Null),
    );
    serde_json::to_string(&Value::Object(close_summary)).map_err(CorePipelineError::from)
}

fn load_close_task_context(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    now: &UtcTimestamp,
) -> Result<CloseTaskContext, PlanError> {
    let task = store
        .task_record(&request.task_id)
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
    let current_change_unit = store
        .current_change_unit(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let task_revision = store
        .task_revision_record(&request.task_id)
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
    let current_close_basis = task_revision.current_close_basis;
    let pending_user_action_refs = store
        .pending_user_action_refs(&request.task_id, project_state.state_version, now)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    let blocker_refs = store
        .active_blocker_refs(&request.task_id, project_state.state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    let evidence_record = current_close_basis
        .as_ref()
        .and_then(|basis| basis.evidence_summary_ref.as_ref())
        .map(|evidence_ref| {
            store
                .evidence_summary_record(evidence_ref.record_id.as_str())
                .map_err(|error| {
                    PlanError::Response(Box::new(store_error_response(
                        &request.envelope,
                        project_state,
                        error,
                    )))
                })
        })
        .transpose()?
        .flatten();
    let required_criterion_ids = load_required_evidence_criterion_ids(store, &request.task_id)?;
    let evidence_facts = load_close_evidence_summary_facts(
        store,
        evidence_record.as_ref(),
        &task,
        &request.envelope.project_id,
        &request.task_id,
        project_state.state_version,
    )?;
    let evidence_summary = project_close_evidence_summary(evidence_facts, &required_criterion_ids);
    let artifact_refs = evidence_summary
        .as_ref()
        .map(|summary| summary.artifact_refs.clone())
        .unwrap_or_default();

    Ok(CloseTaskContext {
        now: now.clone(),
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
    })
}

fn unrecorded_change_close_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
) -> Result<Vec<CloseReadinessBlocker>, PlanError> {
    let unresolved = unresolved_unrecorded_changes(store, request, context)?;
    if unresolved.is_empty() {
        return Ok(Vec::new());
    }

    let task_ref = task_ref_for_close(request, project_state.state_version);
    Ok(vec![close_blocker(
        CloseReadinessBlockerCategory::ConnectionCapability,
        "unresolved_unrecorded_changes",
        "Observed Product Repository changes still need reconciliation.",
        vec![task_ref.clone()],
        vec![NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::ReconcileChanges,
            owner_method: Some(MethodName::ReconcileChanges),
            allowed_operation_categories: vec![
                OperationCategory::AgentWorkflow,
                OperationCategory::LocalRecovery,
            ],
            label: "Run reconciliation for observed Product Repository changes before close."
                .to_owned(),
            blocking_question: Some(
                "Does the user accept any remaining observed Product Repository change as intentional?"
                    .to_owned(),
            ),
            expected_state_version: RequiredNullable::null(),
            required_refs: vec![task_ref],
        }],
    )])
}

fn unresolved_unrecorded_changes(
    store: &CoreProjectStore,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
) -> Result<Vec<UnrecordedChangeRecord>, PlanError> {
    let unresolved = volicord_store::guards::list_unresolved_unrecorded_changes(
        store.runtime_home(),
        request.envelope.project_id.as_str(),
        None,
    )
    .map_err(CorePipelineError::from)
    .map_err(PlanError::Core)?;
    Ok(unresolved
        .into_iter()
        .filter(|record| {
            !context
                .projected_resolved_unrecorded_change_ids
                .contains(&record.unrecorded_change_id)
        })
        .collect())
}

pub(super) fn user_channel_pending_action_instruction() -> String {
    "Use `volicord inbox` through the User Channel to list and resolve pending actions.".to_owned()
}

fn terminal_close_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
    now: &UtcTimestamp,
) -> Result<Vec<CloseReadinessBlocker>, PlanError> {
    let mut blockers = Vec::new();
    let task_ref = task_ref_for_close(request, project_state.state_version);
    if is_terminal_lifecycle(&context.task.lifecycle_phase)
        || project_state
            .active_task_id
            .as_deref()
            .is_some_and(|active_task_id| active_task_id != request.task_id.as_str())
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Task,
            "task_not_closeable",
            "The addressed Task is not the current non-terminal Task.",
            vec![task_ref.clone()],
            vec![close_next_action(
                "Review the current Task before closing.",
                vec![task_ref.clone()],
            )],
        ));
    }

    if request.intent == CloseIntent::Supersede {
        let superseding_ref = request.superseding_task_id.as_ref().map(|task_id| {
            state_ref(
                StateRecordKind::Task,
                task_id.as_str(),
                &request.envelope.project_id,
                Some(task_id),
                Some(project_state.state_version),
            )
        });
        let replacement = request
            .superseding_task_id
            .as_ref()
            .map(|task_id| {
                store.task_record(task_id).map_err(|error| {
                    PlanError::Response(Box::new(store_error_response(
                        &request.envelope,
                        project_state,
                        error,
                    )))
                })
            })
            .transpose()?
            .flatten();
        if replacement
            .as_ref()
            .map(|task| is_terminal_lifecycle(&task.lifecycle_phase))
            .unwrap_or(true)
        {
            blockers.push(close_blocker(
                CloseReadinessBlockerCategory::Task,
                "task_not_closeable",
                "superseding_task_id must identify a non-terminal Task in this project.",
                superseding_ref.into_iter().collect(),
                Vec::new(),
            ));
        }
    }

    if recovery_required(context)? {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Recovery,
            "recovery_required",
            "A recovery constraint or active blocker must be resolved before this terminal transition.",
            context.blocker_refs.clone(),
            vec![close_next_action(
                "Resolve recovery blockers before closing the Task.",
                context.blocker_refs.clone(),
            )],
        ));
    }

    if matches!(request.intent, CloseIntent::Check | CloseIntent::Complete) {
        blockers.extend(unresolved_write_ticket_close_blockers(
            store,
            project_state,
            request,
            now,
        )?);
    }

    match request.intent {
        CloseIntent::Cancel => {
            if let Some(blocker) =
                cancellation_authority_blocker(store, project_state, request, context)?
            {
                blockers.push(blocker);
            }
        }
        CloseIntent::Supersede => {
            let pending_refs = pending_user_action_refs_for_close_operation(
                store,
                project_state,
                request,
                context,
                UserActionOperation::CloseSupersede,
                now,
            )?;
            if !pending_refs.is_empty() {
                blockers.push(close_blocker(
                    CloseReadinessBlockerCategory::PendingUserAction,
                    "pending_user_action",
                    "A user action required before superseding this Task is still pending.",
                    pending_refs.clone(),
                    vec![NextActionSummary {
                        presentation_role: NextActionPresentationRole::Primary,
                        action_kind: NextActionKind::ResolveUserAction,
                        owner_method: Some(MethodName::ResolveUserAction),
                        allowed_operation_categories: vec![OperationCategory::UserOnly],
                        label: "Resolve pending user actions through the User Channel.".to_owned(),
                        blocking_question: Some(user_channel_pending_action_instruction()),
                        expected_state_version: RequiredNullable::null(),
                        required_refs: pending_refs,
                    }],
                ));
            }
        }
        CloseIntent::Check | CloseIntent::Complete => {}
    }

    Ok(blockers)
}

fn unresolved_write_ticket_close_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    now: &UtcTimestamp,
) -> Result<Vec<CloseReadinessBlocker>, PlanError> {
    let mut blockers = Vec::new();
    let task_ref = task_ref_for_close(request, project_state.state_version);
    for record in store
        .write_tickets_for_task(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
    {
        let mut status = effective_write_ticket_status(
            &record,
            project_state.state_version,
            Some(*now.as_datetime()),
        )
        .map_err(PlanError::Core)?;
        if status == WriteTicketStatus::Active
            && !write_ticket_is_current_for_projection(store, &record, *now.as_datetime())?
        {
            status = WriteTicketStatus::Invalidated;
        }
        match status {
            WriteTicketStatus::Active => blockers.push(open_write_ticket_close_blocker(
                task_ref.clone(),
                write_ticket_ref(&record, project_state.state_version),
            )),
            WriteTicketStatus::Invalidated
            | WriteTicketStatus::Revoked
            | WriteTicketStatus::Consumed => {}
        }
    }
    Ok(blockers)
}

pub(super) fn open_write_ticket_close_blocker(
    task_ref: StateRecordRef,
    write_ticket_ref: StateRecordRef,
) -> CloseReadinessBlocker {
    close_blocker(
        CloseReadinessBlockerCategory::WriteCompatibility,
        "open_write_ticket",
        "An open write ticket remains unresolved for this Task.",
        vec![write_ticket_ref],
        vec![NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::RecordRun,
            owner_method: Some(MethodName::RecordRun),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label: "Record the ticket-backed run or reconcile observed changes before close."
                .to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::null(),
            required_refs: vec![task_ref],
        }],
    )
}

fn pending_user_action_refs_for_close_operation(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
    operation: UserActionOperation,
    now: &UtcTimestamp,
) -> Result<Vec<StateRecordRef>, PlanError> {
    let authorities =
        pending_user_action_authorities_for_context(store, project_state, request, context)?;
    let current_change_unit_id = context
        .current_change_unit
        .as_ref()
        .map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let operation_refs = close_operation_refs(request, project_state, context);
    let mut refs = Vec::new();
    for authority in &authorities {
        let blocks = if operation == UserActionOperation::CloseComplete
            && authority.action_kind == UserActionKind::SensitiveApproval
        {
            pending_sensitive_judgment_blocks_close(
                store,
                request,
                context,
                authority,
                current_change_unit_id.as_ref(),
                &operation_refs,
                now,
            )
        } else {
            let operation_context = UserActionOperationContext {
                operation,
                task_id: &request.task_id,
                change_unit_id: current_change_unit_id.as_ref(),
                scope_revision: context.task.scope_revision,
                close_basis: context.current_close_basis.as_ref(),
                operation_refs: &operation_refs,
                sensitive_approval: None,
            };
            user_action_blocks_operation(authority, &operation_context)
        };
        if blocks {
            refs.push(state_ref(
                StateRecordKind::UserActionRequest,
                &authority.user_action_request_id,
                &request.envelope.project_id,
                Some(&request.task_id),
                Some(project_state.state_version),
            ));
        }
    }
    Ok(refs)
}

fn pending_user_action_authorities_for_context(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
) -> Result<Vec<UserActionAuthority>, PlanError> {
    if let Some(authorities) = &context.pending_user_action_authorities {
        return Ok(authorities.clone());
    }
    pending_user_action_authorities_for_plan(
        store,
        project_state,
        &request.envelope,
        &request.task_id,
        &context.now,
    )
}

fn resolved_judgment_authorities_for_context(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
    judgment_kind: JudgmentKind,
) -> Result<Vec<UserActionAuthority>, PlanError> {
    if let Some(authorities) = &context.resolved_judgment_authorities {
        return Ok(authorities
            .iter()
            .filter(|authority| authority.action_kind == judgment_kind.into())
            .cloned()
            .collect());
    }
    resolved_user_action_authorities_for_plan(
        store,
        project_state,
        &request.envelope,
        &request.task_id,
        judgment_kind,
        &context.now,
    )
}

fn pending_sensitive_judgment_blocks_close(
    store: &CoreProjectStore,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
    authority: &UserActionAuthority,
    current_change_unit_id: Option<&ChangeUnitId>,
    operation_refs: &[StateRecordRef],
    now: &UtcTimestamp,
) -> bool {
    let Some(close_basis) = context.current_close_basis.as_ref() else {
        return false;
    };
    close_basis
        .sensitive_action_requirements
        .iter()
        .any(|close_requirement| {
            let requirement = SensitiveApprovalRequirement {
                task_id: &request.task_id,
                change_unit_id: &close_requirement.change_unit_id,
                scope_revision: context.task.scope_revision,
                operation: &close_requirement.action_kind,
                normalized_paths: &close_requirement.normalized_paths,
                sensitive_categories: &close_requirement.sensitive_categories,
                baseline_ref: close_requirement.baseline_ref.as_ref(),
                required_for: UserActionRequiredFor::CloseComplete,
                now,
                repo_root: &store.project_record().repo_root,
            };
            let operation_context = UserActionOperationContext {
                operation: UserActionOperation::CloseComplete,
                task_id: &request.task_id,
                change_unit_id: current_change_unit_id,
                scope_revision: context.task.scope_revision,
                close_basis: Some(close_basis),
                operation_refs,
                sensitive_approval: Some(&requirement),
            };
            user_action_blocks_operation(authority, &operation_context)
        })
}

fn close_operation_refs(
    request: &CloseTaskPlanRequest,
    project_state: &ProjectStateHeader,
    context: &CloseTaskContext,
) -> Vec<StateRecordRef> {
    let mut refs = vec![task_ref_for_close(request, project_state.state_version)];
    if let Some(change_unit) = context.current_change_unit.as_ref() {
        refs.push(change_unit_ref(
            &request.envelope.project_id,
            &request.task_id,
            change_unit,
            project_state.state_version,
        ));
    }
    if let Some(close_basis) = context.current_close_basis.as_ref() {
        refs.extend(close_basis.result_refs.clone());
        if let Some(evidence_ref) = close_basis.evidence_summary_ref.as_ref() {
            refs.push(evidence_ref.clone());
        }
        for risk in &close_basis.residual_risks {
            refs.extend(risk.source_refs.clone());
        }
    }
    refs
}

fn cancellation_authority_blocker(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
) -> Result<Option<CloseReadinessBlocker>, PlanError> {
    let current_change_unit_id = context
        .current_change_unit
        .as_ref()
        .map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let requirement = CancellationAuthorityRequirement {
        task_id: &request.task_id,
        change_unit_id: current_change_unit_id.as_ref(),
        scope_revision: context.task.scope_revision,
    };
    let authorities = resolved_judgment_authorities_for_context(
        store,
        project_state,
        request,
        context,
        JudgmentKind::Cancellation,
    )?;
    if authorities.iter().any(|authority| {
        user_action_required_for(authority, UserActionRequiredFor::CloseCancel)
            && current_cancellation_authority(authority, &requirement)
    }) {
        return Ok(None);
    }

    let mut stale_refs = Vec::new();
    let mut rejected_refs = Vec::new();
    for authority in &authorities {
        if !user_action_required_for(authority, UserActionRequiredFor::CloseCancel) {
            continue;
        }
        let user_action_request_ref = state_ref(
            StateRecordKind::UserActionRequest,
            &authority.user_action_request_id,
            &request.envelope.project_id,
            Some(&request.task_id),
            Some(project_state.state_version),
        );
        let current_basis_matches = authority.basis.as_ref().is_some_and(|basis| {
            let coordinates = basis.coordinates();
            coordinates.task_id == request.task_id
                && coordinates.scope_revision == context.task.scope_revision
                && coordinates.change_unit_id.as_ref() == current_change_unit_id.as_ref()
        });
        if !user_action_has_current_basis(authority) || !current_basis_matches {
            stale_refs.push(user_action_request_ref);
        } else if authority.resolution_outcome == Some(JudgmentResolutionOutcome::Rejected)
            && authority.resolved_by_actor_source == Some(ActorSource::LocalUser)
            && verified_user_channel_provenance(authority)
        {
            rejected_refs.push(user_action_request_ref);
        }
    }
    if stale_refs.is_empty() {
        stale_refs.extend(non_current_judgment_refs_for_plan(
            store,
            project_state,
            request,
            JudgmentKind::Cancellation,
            &context.now,
        )?);
    }

    let task_ref = task_ref_for_close(request, project_state.state_version);
    let (code, message, related_refs) = if !rejected_refs.is_empty() {
        (
            "rejected_cancellation_authority",
            "The current user cancellation resolution rejected cancellation.",
            refs_with_context(vec![task_ref.clone()], rejected_refs),
        )
    } else if !stale_refs.is_empty() {
        (
            "stale_cancellation_authority",
            "The available cancellation resolution is stale or incompatible with the current Task scope.",
            refs_with_context(vec![task_ref.clone()], stale_refs),
        )
    } else {
        (
            "missing_cancellation_authority",
            "Cancelling the Task requires a current accepted user cancellation resolution.",
            vec![task_ref.clone()],
        )
    };
    Ok(Some(close_blocker(
        CloseReadinessBlockerCategory::UserAction,
        code,
        message,
        related_refs,
        vec![NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::RequestUserAction,
            owner_method: Some(MethodName::RequestUserAction),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label: "Request current user cancellation authority.".to_owned(),
            blocking_question: None,
            expected_state_version: RequiredNullable::null(),
            required_refs: vec![task_ref],
        }],
    )))
}

fn completion_close_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
    risk_acceptance_coverage: &[RiskAcceptanceCoverage],
    now: &UtcTimestamp,
) -> Result<Vec<CloseReadinessBlocker>, PlanError> {
    let mut blockers = Vec::new();
    let task_ref = task_ref_for_close(request, project_state.state_version);
    let change_unit_ref = context.current_change_unit.as_ref().map(|record| {
        change_unit_ref(
            &request.envelope.project_id,
            &request.task_id,
            record,
            project_state.state_version,
        )
    });

    if context
        .current_change_unit
        .as_ref()
        .map(|record| record.status != "active" || !record.is_current)
        .unwrap_or(true)
    {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Scope,
            "missing_active_change_unit",
            "Completion requires a current active Change Unit.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                presentation_role: NextActionPresentationRole::Primary,
                action_kind: NextActionKind::UpdateScope,
                owner_method: Some(MethodName::UpdateScope),
                allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
                label: "Create or restore the current active Change Unit.".to_owned(),
                blocking_question: None,
                expected_state_version: RequiredNullable::null(),
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    if let Some(blocker) = current_close_basis_blocker(store, request, project_state, context)? {
        blockers.push(blocker);
    }

    let close_complete_pending_refs = pending_user_action_refs_for_close_operation(
        store,
        project_state,
        request,
        context,
        UserActionOperation::CloseComplete,
        now,
    )?;
    if !close_complete_pending_refs.is_empty() {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::PendingUserAction,
            "pending_user_action",
            "A user action required before close is still pending.",
            close_complete_pending_refs.clone(),
            vec![NextActionSummary {
                presentation_role: NextActionPresentationRole::Primary,
                action_kind: NextActionKind::ResolveUserAction,
                owner_method: Some(MethodName::ResolveUserAction),
                allowed_operation_categories: vec![OperationCategory::UserOnly],
                label: "Resolve pending user actions through the User Channel.".to_owned(),
                blocking_question: Some(user_channel_pending_action_instruction()),
                expected_state_version: RequiredNullable::null(),
                required_refs: close_complete_pending_refs,
            }],
        ));
    }

    if sensitive_action_basis_missing(context)? {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::SensitiveApproval,
            "missing_sensitive_action_basis",
            "The effective sensitive Task has no ticket-backed sensitive-action basis for close.",
            change_unit_ref
                .clone()
                .into_iter()
                .chain(std::iter::once(task_ref.clone()))
                .collect(),
            vec![NextActionSummary {
                presentation_role: NextActionPresentationRole::Primary,
                action_kind: NextActionKind::PrepareWrite,
                owner_method: Some(MethodName::PrepareWrite),
                allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
                label: "Prepare the exact sensitive action with user-owned approval, then record its ticket-backed Run."
                    .to_owned(),
                blocking_question: None,
                expected_state_version: RequiredNullable::null(),
                required_refs: vec![task_ref.clone()],
            }],
        ));
    } else if sensitive_approval_required(context)?
        && !has_current_sensitive_approval_for_close(store, project_state, request, context, now)?
    {
        let related_refs = refs_with_context(
            change_unit_ref.clone().into_iter().collect(),
            non_current_judgment_refs_for_plan(
                store,
                project_state,
                request,
                JudgmentKind::SensitiveApproval,
                &context.now,
            )?,
        );
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::SensitiveApproval,
            "missing_sensitive_approval",
            "A documented sensitive-action approval required for close is missing.",
            related_refs,
            vec![NextActionSummary {
                presentation_role: NextActionPresentationRole::Primary,
                action_kind: NextActionKind::RequestUserAction,
                owner_method: Some(MethodName::RequestUserAction),
                allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
                label: "Request the user-owned sensitive-action approval.".to_owned(),
                blocking_question: None,
                expected_state_version: RequiredNullable::null(),
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    if baseline_stale_for_close(context)? {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::Baseline,
            "baseline_stale",
            "The current close basis is stale against the current baseline.",
            change_unit_ref.clone().into_iter().collect(),
            vec![NextActionSummary {
                presentation_role: NextActionPresentationRole::Primary,
                action_kind: NextActionKind::UpdateScope,
                owner_method: Some(MethodName::UpdateScope),
                allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
                label: "Refresh the current scope or close basis before completing the Task."
                    .to_owned(),
                blocking_question: None,
                expected_state_version: RequiredNullable::null(),
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    if let Some(basis) = context.current_close_basis.as_ref() {
        if !basis.recovery_constraints.is_empty() {
            blockers.push(close_blocker(
                CloseReadinessBlockerCategory::Recovery,
                "recovery_required",
                "The current close basis records recovery constraints that must be resolved.",
                vec![task_ref.clone()],
                vec![close_next_action(
                    "Resolve recovery constraints before completing the Task.",
                    vec![task_ref.clone()],
                )],
            ));
        }
    }

    blockers.extend(close_evidence_blockers(
        store,
        project_state,
        request,
        context,
        change_unit_ref.clone(),
    )?);

    let unavailable_artifacts =
        unavailable_close_artifact_refs(store, project_state, request, context)?;
    if !unavailable_artifacts.is_empty() {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ArtifactAvailability,
            "artifact_unavailable",
            "A required close artifact is missing, unavailable, or incompatible with storage.",
            unavailable_artifacts,
            vec![NextActionSummary {
                presentation_role: NextActionPresentationRole::Primary,
                action_kind: NextActionKind::RecordRun,
                owner_method: Some(MethodName::RecordRun),
                allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
                label: "Record or repair the artifact supporting close evidence.".to_owned(),
                blocking_question: None,
                expected_state_version: RequiredNullable::null(),
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }

    if let Some(blocker) = final_acceptance_blocker(store, project_state, request, context)? {
        blockers.push(blocker);
    }

    let residual_risk = residual_risk_state(context);
    if residual_risk.known && !residual_risk.visible {
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ResidualRiskVisibility,
            "residual_risk_not_visible",
            "Residual risk exists but is not visible in the close basis.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                presentation_role: NextActionPresentationRole::Primary,
                action_kind: NextActionKind::RequestUserAction,
                owner_method: Some(MethodName::RequestUserAction),
                allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
                label: "Make residual risk visible before requesting acceptance.".to_owned(),
                blocking_question: None,
                expected_state_version: RequiredNullable::null(),
                required_refs: vec![task_ref.clone()],
            }],
        ));
    }
    if residual_risk.known
        && residual_risk.visible
        && risk_acceptance_coverage
            .iter()
            .any(|coverage| !coverage.accepted)
    {
        let stale_refs = non_current_judgment_refs_for_plan(
            store,
            project_state,
            request,
            JudgmentKind::ResidualRiskAcceptance,
            &context.now,
        )?;
        let (code, message) = if stale_refs.is_empty() {
            (
                "missing_residual_risk_acceptance",
                "Visible residual risk requires distinct residual-risk acceptance.",
            )
        } else {
            (
                "stale_residual_risk_acceptance",
                "The available residual-risk acceptance is stale or incompatible with the current close basis.",
            )
        };
        let related_refs = refs_with_context(vec![task_ref.clone()], stale_refs);
        blockers.push(close_blocker(
            CloseReadinessBlockerCategory::ResidualRiskAcceptance,
            code,
            message,
            related_refs,
            vec![NextActionSummary {
                presentation_role: NextActionPresentationRole::Primary,
                action_kind: NextActionKind::RequestUserAction,
                owner_method: Some(MethodName::RequestUserAction),
                allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
                label: "Request current residual-risk acceptance from the user.".to_owned(),
                blocking_question: None,
                expected_state_version: RequiredNullable::null(),
                required_refs: vec![task_ref],
            }],
        ));
    }

    Ok(blockers)
}

fn evidence_target_required_by(target: &EvidenceTarget, required: &BTreeSet<String>) -> bool {
    matches!(
        target,
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id
        } if required.contains(acceptance_criterion_id.as_str())
    )
}

fn required_criteria_for_close_context(
    store: &CoreProjectStore,
    task_id: &TaskId,
    context: &CloseTaskContext,
) -> CoreResult<BTreeSet<String>> {
    if let Some(required) = context.projected_required_criterion_ids.as_ref() {
        return Ok(required.clone());
    }
    store
        .active_acceptance_criteria(task_id)
        .map_err(CorePipelineError::from)?
        .into_iter()
        .map(|criterion| {
            let requirement = parse_owner_storage_value::<EvidenceRequirement>(
                "acceptance_criteria",
                criterion.acceptance_criterion_id.clone(),
                "evidence_requirement",
                &criterion.evidence_requirement,
            )?;
            Ok::<_, CorePipelineError>((criterion.acceptance_criterion_id, requirement))
        })
        .collect::<CoreResult<Vec<_>>>()
        .map(|criteria| {
            criteria
                .into_iter()
                .filter_map(|(id, requirement)| {
                    (requirement == EvidenceRequirement::Required).then_some(id)
                })
                .collect()
        })
}

fn current_close_basis_blocker(
    store: &CoreProjectStore,
    request: &CloseTaskPlanRequest,
    project_state: &ProjectStateHeader,
    context: &CloseTaskContext,
) -> Result<Option<CloseReadinessBlocker>, PlanError> {
    let task_ref = task_ref_for_close(request, project_state.state_version);
    let Some(basis) = context.current_close_basis.as_ref() else {
        return Ok(Some(close_blocker(
            CloseReadinessBlockerCategory::Task,
            "missing_current_close_basis",
            "Completion requires a current close basis recorded by volicord.record_run.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                presentation_role: NextActionPresentationRole::Primary,
                action_kind: NextActionKind::RecordRun,
                owner_method: Some(MethodName::RecordRun),
                allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
                label: "Record the current result and close basis.".to_owned(),
                blocking_question: None,
                expected_state_version: RequiredNullable::null(),
                required_refs: vec![task_ref],
            }],
        )));
    };
    let current_change_unit_id = context
        .current_change_unit
        .as_ref()
        .map(|record| record.change_unit_id.as_str());
    let current_baseline = StoredScope::from_task(&context.task)?.baseline_ref;
    if !close_basis_is_current(
        basis,
        &request.task_id,
        current_change_unit_id,
        context.task.scope_revision,
        context.task.close_basis_revision,
        current_baseline.as_deref(),
    ) {
        Ok(Some(close_blocker(
            CloseReadinessBlockerCategory::Scope,
            "stale_current_close_basis",
            "The current close basis is stale against current Task scope.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                presentation_role: NextActionPresentationRole::Primary,
                action_kind: NextActionKind::RecordRun,
                owner_method: Some(MethodName::RecordRun),
                allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
                label: "Record a fresh close basis for the current scope.".to_owned(),
                blocking_question: None,
                expected_state_version: RequiredNullable::null(),
                required_refs: vec![task_ref],
            }],
        )))
    } else if let Some(blocker) = incompatible_close_basis_run_refs_blocker(
        store,
        request,
        project_state,
        context,
        basis,
        current_baseline.as_deref(),
    )? {
        Ok(Some(blocker))
    } else {
        Ok(None)
    }
}

fn incompatible_close_basis_run_refs_blocker(
    store: &CoreProjectStore,
    request: &CloseTaskPlanRequest,
    project_state: &ProjectStateHeader,
    context: &CloseTaskContext,
    basis: &CurrentCloseBasis,
    current_baseline: Option<&str>,
) -> Result<Option<CloseReadinessBlocker>, PlanError> {
    let Some(current_change_unit) = context.current_change_unit.as_ref() else {
        return Ok(None);
    };
    let current_change_unit_id = current_change_unit.change_unit_id.as_str();
    let mut seen = BTreeSet::new();
    let mut incompatible_refs = Vec::new();
    for record_ref in close_basis_run_refs(basis) {
        let record_id = record_ref.record_id.as_str();
        if !seen.insert(state_record_ref_identity_key(record_ref)) {
            continue;
        }
        if record_ref.project_id != request.envelope.project_id
            || record_ref.task_id.as_ref() != Some(&request.task_id)
        {
            incompatible_refs.push(record_ref.clone());
            continue;
        }
        if context.projected_run_refs.iter().any(|projected_ref| {
            state_record_ref_identity_key(projected_ref)
                == state_record_ref_identity_key(record_ref)
        }) {
            continue;
        }
        let record = store.run_record(record_id).map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
        if record.as_ref().is_none_or(|record| {
            !run_record_matches_close_basis_context(
                record,
                &request.envelope.project_id,
                &request.task_id,
                current_change_unit_id,
                context.task.scope_revision,
                current_baseline,
            )
        }) {
            incompatible_refs.push(record_ref.clone());
        }
    }

    if incompatible_refs.is_empty() {
        Ok(None)
    } else {
        let task_ref = task_ref_for_close(request, project_state.state_version);
        Ok(Some(close_blocker(
            CloseReadinessBlockerCategory::Scope,
            "stale_current_close_basis",
            "The current close basis contains Run refs that are not current for the Task scope.",
            incompatible_refs,
            vec![NextActionSummary {
                presentation_role: NextActionPresentationRole::Primary,
                action_kind: NextActionKind::RecordRun,
                owner_method: Some(MethodName::RecordRun),
                allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
                label: "Record a fresh close basis for the current Run context.".to_owned(),
                blocking_question: None,
                expected_state_version: RequiredNullable::null(),
                required_refs: vec![task_ref],
            }],
        )))
    }
}

struct CloseEvidenceIssue {
    kind: CloseEvidenceIssueKind,
    related_refs: Vec<StateRecordRef>,
}

fn close_evidence_blockers(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
    change_unit_ref: Option<StateRecordRef>,
) -> Result<Vec<CloseReadinessBlocker>, PlanError> {
    let Some(summary) = context.evidence_summary.as_ref() else {
        return Ok(Vec::new());
    };
    let mut grouped: BTreeMap<CloseEvidenceIssueKind, Vec<StateRecordRef>> = BTreeMap::new();
    for item in &summary.coverage_items {
        if let Some(issue) =
            close_evidence_issue_for_item(store, project_state, request, context, item)?
        {
            grouped
                .entry(issue.kind)
                .or_default()
                .extend(issue.related_refs);
        }
    }

    let required_refs = change_unit_ref.into_iter().collect::<Vec<_>>();
    let mut blockers = Vec::new();
    for kind in [
        CloseEvidenceIssueKind::Missing,
        CloseEvidenceIssueKind::Unsupported,
        CloseEvidenceIssueKind::Stale,
        CloseEvidenceIssueKind::AgentReportOnly,
        CloseEvidenceIssueKind::InsufficientProvenance,
    ] {
        let Some(related_refs) = grouped.remove(&kind) else {
            continue;
        };
        let category = match kind {
            CloseEvidenceIssueKind::Missing | CloseEvidenceIssueKind::Unsupported => {
                CloseReadinessBlockerCategory::EvidenceClaim
            }
            CloseEvidenceIssueKind::Stale
            | CloseEvidenceIssueKind::AgentReportOnly
            | CloseEvidenceIssueKind::InsufficientProvenance => {
                CloseReadinessBlockerCategory::EvidenceProvenance
            }
        };
        let (code, message) = match kind {
            CloseEvidenceIssueKind::Missing => (
                "evidence_claim_missing",
                "One or more required close evidence claims are missing.",
            ),
            CloseEvidenceIssueKind::Unsupported => (
                "evidence_claim_unsupported",
                "One or more required close evidence claims are unsupported.",
            ),
            CloseEvidenceIssueKind::Stale => (
                "evidence_provenance_stale",
                "Evidence provenance exists but is stale against the current close basis.",
            ),
            CloseEvidenceIssueKind::AgentReportOnly => (
                "evidence_agent_report_only",
                "Required close evidence is supported only by cooperative agent reports.",
            ),
            CloseEvidenceIssueKind::InsufficientProvenance => (
                "evidence_provenance_insufficient",
                "Required close evidence lacks sufficient source provenance.",
            ),
        };
        blockers.push(close_blocker(
            category,
            code,
            message,
            unique_state_record_refs(related_refs),
            vec![NextActionSummary {
                presentation_role: NextActionPresentationRole::Primary,
                action_kind: NextActionKind::RecordRun,
                owner_method: Some(MethodName::RecordRun),
                allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
                label: "Record evidence that supports the required close claims.".to_owned(),
                blocking_question: None,
                expected_state_version: RequiredNullable::null(),
                required_refs: required_refs.clone(),
            }],
        ));
    }
    Ok(blockers)
}

fn close_evidence_issue_for_item(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
    item: &EvidenceCoverageItem,
) -> Result<Option<CloseEvidenceIssue>, PlanError> {
    let EvidenceTarget::AcceptanceCriterion {
        acceptance_criterion_id,
    } = &item.target
    else {
        return Ok(None);
    };
    let required_criteria = required_criteria_for_close_context(store, &request.task_id, context)?;
    if !required_criteria.contains(acceptance_criterion_id.as_str()) {
        return Ok(None);
    }
    let Some(basis) = context.current_close_basis.as_ref() else {
        return Ok(
            interpret_close_evidence_item(item, &required_criteria, false, &[]).map(|kind| {
                CloseEvidenceIssue {
                    kind,
                    related_refs: evidence_item_related_refs(item),
                }
            }),
        );
    };
    if item.coverage_state != EvidenceCoverageState::Supported || item.observation_refs.is_empty() {
        return Ok(
            interpret_close_evidence_item(item, &required_criteria, true, &[]).map(|kind| {
                CloseEvidenceIssue {
                    kind,
                    related_refs: evidence_item_related_refs(item),
                }
            }),
        );
    }

    let mut dispositions = Vec::new();
    let evidence_state_version = basis
        .evidence_summary_ref
        .as_ref()
        .and_then(|record_ref| record_ref.produced_at_state_version.as_ref().copied());
    for observation_ref in &item.observation_refs {
        if observation_ref.record_kind != StateRecordKind::EvidenceObservation
            || observation_ref.project_id != request.envelope.project_id
            || observation_ref.task_id.as_ref() != Some(&request.task_id)
        {
            dispositions.push(CloseEvidenceObservationDisposition::Weak);
            continue;
        }
        if evidence_state_version.is_some_and(|state_version| {
            observation_ref.produced_at_state_version.as_ref() != Some(&state_version)
        }) {
            dispositions.push(CloseEvidenceObservationDisposition::Stale);
            continue;
        }
        if let Some(observation) =
            context
                .projected_evidence_observations
                .iter()
                .find(|observation| {
                    observation.observation_id.as_str() == observation_ref.record_id.as_str()
                })
        {
            if observation.project_id != request.envelope.project_id
                || observation.task_id != request.task_id
                || !projected_observation_matches_close_basis(observation, basis, &item.target)
            {
                dispositions.push(CloseEvidenceObservationDisposition::Stale);
                continue;
            }
            if capture_relevance_is_unsupported(
                observation.producer_anchor.producer_kind,
                &observation.relevance_assessment,
            ) {
                dispositions.push(CloseEvidenceObservationDisposition::UnsupportedRelevance);
                continue;
            }
            let facts = projected_evidence_observation_provenance_facts(
                store,
                observation,
                &EvidenceObservationBasis {
                    project_id: &request.envelope.project_id,
                    task_id: &request.task_id,
                    change_unit_id: basis.change_unit_id.as_str(),
                    scope_revision: basis.scope_revision,
                    baseline_ref: basis.baseline_ref.as_ref().map(BaselineRef::as_str),
                    target: &item.target,
                    now: &context.now,
                },
                &context.projected_artifacts,
            )?;
            dispositions.push(match classify_evidence_provenance(&facts) {
                EvidenceProvenanceClass::Strong => {
                    CloseEvidenceObservationDisposition::StrongSupported
                }
                EvidenceProvenanceClass::CooperativeAgentReport => {
                    CloseEvidenceObservationDisposition::CooperativeAgentReport
                }
                EvidenceProvenanceClass::Weak => CloseEvidenceObservationDisposition::Weak,
            });
            continue;
        }
        let record = store
            .evidence_observation_record(observation_ref.record_id.as_str())
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?;
        let Some(record) = record else {
            dispositions.push(CloseEvidenceObservationDisposition::Weak);
            continue;
        };
        if record.project_id != request.envelope.project_id.as_str()
            || record.task_id != request.task_id.as_str()
            || !stored_observation_matches_close_basis(&record, basis, &item.target)
        {
            dispositions.push(CloseEvidenceObservationDisposition::Stale);
            continue;
        }
        if stored_evidence_observation_capture_relevance(&record)?
            .is_some_and(|status| status != EvidenceRelevanceStatus::Supported)
        {
            dispositions.push(CloseEvidenceObservationDisposition::UnsupportedRelevance);
            continue;
        }
        let facts = stored_evidence_observation_provenance_facts(
            store,
            &record,
            &EvidenceObservationBasis {
                project_id: &request.envelope.project_id,
                task_id: &request.task_id,
                change_unit_id: basis.change_unit_id.as_str(),
                scope_revision: basis.scope_revision,
                baseline_ref: basis.baseline_ref.as_ref().map(BaselineRef::as_str),
                target: &item.target,
                now: &context.now,
            },
        )?;
        dispositions.push(match classify_evidence_provenance(&facts) {
            EvidenceProvenanceClass::Strong => CloseEvidenceObservationDisposition::StrongSupported,
            EvidenceProvenanceClass::CooperativeAgentReport => {
                CloseEvidenceObservationDisposition::CooperativeAgentReport
            }
            EvidenceProvenanceClass::Weak => CloseEvidenceObservationDisposition::Weak,
        });
    }

    Ok(
        interpret_close_evidence_item(item, &required_criteria, true, &dispositions).map(|kind| {
            CloseEvidenceIssue {
                kind,
                related_refs: evidence_item_related_refs(item),
            }
        }),
    )
}

fn unavailable_close_artifact_refs(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
) -> Result<Vec<StateRecordRef>, PlanError> {
    let mut seen = BTreeSet::new();
    let mut unavailable = Vec::new();
    let required_criteria = required_criteria_for_close_context(store, &request.task_id, context)?;
    if let Some(evidence_summary) = context.evidence_summary.as_ref() {
        for artifact_ref in evidence_summary
            .coverage_items
            .iter()
            .filter(|item| evidence_target_required_by(&item.target, &required_criteria))
            .flat_map(|item| item.supporting_artifact_refs.iter())
        {
            let state_ref = state_ref(
                StateRecordKind::Artifact,
                artifact_ref.artifact_id.as_str(),
                &request.envelope.project_id,
                Some(&request.task_id),
                Some(project_state.state_version),
            );
            if !seen.insert(state_record_ref_identity_key(&state_ref)) {
                continue;
            }
            if artifact_ref.availability != ArtifactAvailability::Available {
                unavailable.push(state_ref);
                continue;
            }
            if context.projected_artifacts.iter().any(|projected| {
                projected == artifact_ref
                    && projected.integrity_status == ArtifactIntegrityStatus::Verified
            }) {
                continue;
            }
            let stored = store
                .artifact_record(artifact_ref.artifact_id.as_str())
                .map_err(|error| {
                    PlanError::Response(Box::new(store_error_response(
                        &request.envelope,
                        project_state,
                        error,
                    )))
                })?;
            let Some(stored) = stored else {
                unavailable.push(state_ref);
                continue;
            };
            let owner_link_exists = store
                .artifact_has_task_owner_link(
                    artifact_ref.artifact_id.as_str(),
                    request.task_id.as_str(),
                )
                .map_err(|error| {
                    PlanError::Response(Box::new(store_error_response(
                        &request.envelope,
                        project_state,
                        error,
                    )))
                })?;
            let stored_available = persistent_artifact_is_verified_current(store, &stored)?;
            let stored_redaction_state: RedactionState = parse_owner_storage_value(
                "artifacts",
                stored.artifact_id.clone(),
                "redaction_state",
                &stored.redaction_state,
            )?;
            let artifact_sha256 = artifact_ref.sha256.as_ref();
            let artifact_size_bytes = artifact_ref.size_bytes.as_ref().copied();
            if stored.project_id != request.envelope.project_id.as_str()
                || stored.task_id != request.task_id.as_str()
                || !stored_available
                || artifact_ref.integrity_status != ArtifactIntegrityStatus::Verified
                || stored.sha256.as_deref() != artifact_sha256.map(String::as_str)
                || stored.size_bytes != artifact_size_bytes
                || stored_redaction_state != artifact_ref.redaction_state
                || !owner_link_exists
            {
                unavailable.push(state_ref);
            }
        }
    }
    if let Some(basis) = context.current_close_basis.as_ref() {
        for record_ref in basis
            .result_refs
            .iter()
            .chain(
                basis
                    .residual_risks
                    .iter()
                    .flat_map(|risk| risk.source_refs.iter()),
            )
            .filter(|record_ref| record_ref.record_kind == StateRecordKind::Artifact)
        {
            if !seen.insert(state_record_ref_identity_key(record_ref)) {
                continue;
            }
            if close_basis_artifact_ref_unavailable(
                store,
                request,
                record_ref,
                project_state,
                context,
            )? {
                unavailable.push(record_ref.clone());
            }
        }
    }
    Ok(unavailable)
}

fn close_basis_artifact_ref_unavailable(
    store: &CoreProjectStore,
    request: &CloseTaskPlanRequest,
    record_ref: &StateRecordRef,
    project_state: &ProjectStateHeader,
    context: &CloseTaskContext,
) -> Result<bool, PlanError> {
    if let Some(artifact_ref) = context
        .projected_artifacts
        .iter()
        .find(|artifact_ref| artifact_ref.artifact_id.as_str() == record_ref.record_id.as_str())
    {
        return Ok(record_ref.project_id != request.envelope.project_id
            || record_ref.task_id.as_ref() != Some(&request.task_id)
            || artifact_ref.project_id != request.envelope.project_id
            || artifact_ref.task_id != request.task_id
            || artifact_ref.availability != ArtifactAvailability::Available
            || artifact_ref.integrity_status != ArtifactIntegrityStatus::Verified);
    }
    let stored = store
        .artifact_record(record_ref.record_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let owner_link_exists = store
        .artifact_has_task_owner_link(record_ref.record_id.as_str(), request.task_id.as_str())
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    Ok(stored
        .as_ref()
        .map(|record| {
            let available = persistent_artifact_is_verified_current(store, record)?;
            let unavailable = record.project_id != request.envelope.project_id.as_str()
                || record.task_id != request.task_id.as_str()
                || !available
                || !owner_link_exists;
            Ok::<_, CorePipelineError>(unavailable)
        })
        .transpose()?
        .unwrap_or(true))
}

fn final_acceptance_blocker(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
) -> Result<Option<CloseReadinessBlocker>, PlanError> {
    let acceptance_policy = parse_acceptance_policy(&context.task.acceptance_policy)?;
    let control = parse_task_control_level(&context.task.effective_control_level)
        .map_err(CorePipelineError::from)?;
    let acceptance_required = match control {
        TaskControlLevel::Observe => false,
        TaskControlLevel::Tracked | TaskControlLevel::Sensitive => true,
        TaskControlLevel::Light => match acceptance_policy {
            AcceptancePolicy::Required => true,
            AcceptancePolicy::NotRequired | AcceptancePolicy::PolicyDependent => {
                !light_completion_without_acceptance_allowed(
                    store,
                    project_state,
                    request,
                    context,
                )?
            }
        },
    };
    if !acceptance_required {
        return Ok(None);
    }
    let task_ref = task_ref_for_close(request, project_state.state_version);
    let Some(close_basis) = context.current_close_basis.as_ref() else {
        return Ok(Some(close_blocker(
            CloseReadinessBlockerCategory::FinalAcceptance,
            "missing_final_acceptance",
            "Final acceptance is required before completing the Task.",
            vec![task_ref.clone()],
            vec![NextActionSummary {
                presentation_role: NextActionPresentationRole::Primary,
                action_kind: NextActionKind::RequestUserAction,
                owner_method: Some(MethodName::RequestUserAction),
                allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
                label:
                    "The Agent Connection must create a current final-acceptance request for the user."
                        .to_owned(),
                blocking_question: Some(
                    "Does the user accept the current Task result and close basis as complete?"
                        .to_owned(),
                ),
                expected_state_version: RequiredNullable::null(),
                required_refs: vec![task_ref],
            }],
        )));
    };
    let requirement = final_acceptance_requirement(close_basis);
    let authorities = resolved_judgment_authorities_for_context(
        store,
        project_state,
        request,
        context,
        JudgmentKind::FinalAcceptance,
    )?;
    if authorities
        .iter()
        .any(|authority| current_final_acceptance(authority, &requirement))
    {
        return Ok(None);
    }

    let stale_refs = non_current_judgment_refs_for_plan(
        store,
        project_state,
        request,
        JudgmentKind::FinalAcceptance,
        &context.now,
    )?;
    let (code, message, related_refs) = if stale_refs.is_empty() {
        (
            "missing_final_acceptance",
            "Final acceptance is required before completing the Task.",
            vec![task_ref.clone()],
        )
    } else {
        (
            "stale_final_acceptance",
            "The available final acceptance is stale or incompatible with the current close basis.",
            refs_with_context(vec![task_ref.clone()], stale_refs),
        )
    };
    Ok(Some(close_blocker(
        CloseReadinessBlockerCategory::FinalAcceptance,
        code,
        message,
        related_refs,
        vec![NextActionSummary {
            presentation_role: NextActionPresentationRole::Primary,
            action_kind: NextActionKind::RequestUserAction,
            owner_method: Some(MethodName::RequestUserAction),
            allowed_operation_categories: vec![OperationCategory::AgentWorkflow],
            label:
                "The Agent Connection must create a current final-acceptance request for the user."
                    .to_owned(),
            blocking_question: Some(
                "Does the user accept the current Task result and close basis as complete?"
                    .to_owned(),
            ),
            expected_state_version: RequiredNullable::null(),
            required_refs: vec![task_ref],
        }],
    )))
}

fn light_completion_without_acceptance_allowed(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
) -> Result<bool, PlanError> {
    if parse_task_control_level(&context.task.effective_control_level)
        .map_err(CorePipelineError::from)?
        != TaskControlLevel::Light
    {
        return Ok(false);
    }
    let workflow_policy = project_workflow_policy(store).map_err(CorePipelineError::from)?;
    if !workflow_policy.light.enabled
        || workflow_policy.light.final_acceptance == AcceptancePolicy::Required
        || !context.pending_user_action_refs.is_empty()
    {
        return Ok(false);
    }
    let Some(close_basis) = context.current_close_basis.as_ref() else {
        return Ok(false);
    };
    if close_basis
        .residual_risks
        .iter()
        .any(|risk| risk.acceptance_required)
        || !close_basis.sensitive_categories.is_empty()
        || !close_basis.sensitive_action_requirements.is_empty()
        || !unresolved_unrecorded_changes(store, request, context)?.is_empty()
    {
        return Ok(false);
    }
    let change_unit_ref = context.current_change_unit.as_ref().map(|record| {
        state_ref(
            StateRecordKind::ChangeUnit,
            &record.change_unit_id,
            &request.envelope.project_id,
            Some(&request.task_id),
            Some(project_state.state_version),
        )
    });
    if !close_evidence_blockers(store, project_state, request, context, change_unit_ref)?.is_empty()
    {
        return Ok(false);
    }

    let tickets = store
        .write_tickets_for_task(&request.task_id)
        .map_err(CorePipelineError::from)?;
    for observed in store
        .run_observed_changes_for_task(&request.task_id)
        .map_err(CorePipelineError::from)?
    {
        if observed.status != "recorded" {
            return Ok(false);
        }
        if !observed.observed_changes.sensitive_categories.is_empty() {
            return Ok(false);
        }
        if !observed.observed_changes.product_file_write_observed {
            continue;
        }
        if !workflow_policy.light_paths_are_allowed(&observed.observed_changes.changed_paths) {
            return Ok(false);
        }
        let Some(run) = store
            .run_record(&observed.run_id)
            .map_err(CorePipelineError::from)?
        else {
            return Ok(false);
        };
        if run.scope_revision != context.task.scope_revision
            || run.change_unit_id.as_deref() != Some(close_basis.change_unit_id.as_str())
            || run.baseline_ref.as_deref()
                != close_basis.baseline_ref.as_ref().map(BaselineRef::as_str)
        {
            return Ok(false);
        }
        let Some(ticket) = tickets.iter().find(|ticket| {
            ticket.status == "consumed"
                && ticket.consumed_by_run_id.as_deref() == Some(observed.run_id.as_str())
        }) else {
            return Ok(false);
        };
        let validity_basis: WriteTicketValidityBasis = decode_required_json(
            "write_tickets",
            ticket.write_ticket_id.clone(),
            "validity_basis_json",
            Some(&ticket.validity_basis_json),
        )?;
        if validity_basis.task_id != request.task_id
            || validity_basis.change_unit_id != close_basis.change_unit_id
            || validity_basis.scope_revision != context.task.scope_revision
            || validity_basis.baseline_ref.as_ref() != close_basis.baseline_ref.as_ref()
        {
            return Ok(false);
        }
        let allowed: Vec<String> = decode_required_json(
            "write_tickets",
            ticket.write_ticket_id.clone(),
            "allowed_path_prefixes_json",
            Some(&ticket.allowed_path_prefixes_json),
        )?;
        let denied: Vec<String> = decode_required_json(
            "write_tickets",
            ticket.write_ticket_id.clone(),
            "denied_path_prefixes_json",
            Some(&ticket.denied_path_prefixes_json),
        )?;
        if !paths_are_authorized(&observed.observed_changes.changed_paths, &allowed)
            || observed.observed_changes.changed_paths.iter().any(|path| {
                denied
                    .iter()
                    .any(|denied_prefix| path_is_within(path, denied_prefix))
            })
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn has_current_sensitive_approval_for_close(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
    now: &UtcTimestamp,
) -> Result<bool, PlanError> {
    let Some(close_basis) = context.current_close_basis.as_ref() else {
        return Ok(false);
    };
    if close_basis.sensitive_action_requirements.is_empty() {
        return Ok(false);
    }
    let authorities = resolved_judgment_authorities_for_context(
        store,
        project_state,
        request,
        context,
        JudgmentKind::SensitiveApproval,
    )?;
    Ok(close_basis
        .sensitive_action_requirements
        .iter()
        .all(|close_requirement| {
            if close_requirement.change_unit_id != close_basis.change_unit_id {
                return false;
            }
            let requirement = SensitiveApprovalRequirement {
                task_id: &request.task_id,
                change_unit_id: &close_requirement.change_unit_id,
                scope_revision: context.task.scope_revision,
                operation: &close_requirement.action_kind,
                normalized_paths: &close_requirement.normalized_paths,
                sensitive_categories: &close_requirement.sensitive_categories,
                baseline_ref: close_requirement.baseline_ref.as_ref(),
                required_for: UserActionRequiredFor::CloseComplete,
                now,
                repo_root: &store.project_record().repo_root,
            };
            authorities
                .iter()
                .any(|authority| current_sensitive_approval(authority, &requirement))
        }))
}

fn risk_acceptance_coverage(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    context: &CloseTaskContext,
) -> Result<Vec<RiskAcceptanceCoverage>, PlanError> {
    let Some(basis) = context.current_close_basis.as_ref() else {
        return Ok(Vec::new());
    };
    let authorities = resolved_judgment_authorities_for_context(
        store,
        project_state,
        request,
        context,
        JudgmentKind::ResidualRiskAcceptance,
    )?;
    let mut coverage = current_residual_risk_acceptance_coverage(
        &request.envelope.project_id,
        &request.task_id,
        project_state.state_version,
        basis,
        &authorities,
    );
    let stale_refs = non_current_judgment_refs_for_plan(
        store,
        project_state,
        request,
        JudgmentKind::ResidualRiskAcceptance,
        &context.now,
    )?;
    if !stale_refs.is_empty() {
        for item in coverage.iter_mut().filter(|item| !item.accepted) {
            item.missing_reason = Some("stale_acceptance".to_owned()).into();
        }
    }
    Ok(coverage)
}

fn non_current_judgment_refs_for_plan(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &CloseTaskPlanRequest,
    judgment_kind: JudgmentKind,
    now: &UtcTimestamp,
) -> Result<Vec<StateRecordRef>, PlanError> {
    store
        .non_current_user_action_refs(
            &request.task_id,
            judgment_kind.into(),
            project_state.state_version,
            now,
        )
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })
        .map(stored_refs_to_state_refs)
}

fn refs_with_context(
    mut refs: Vec<StateRecordRef>,
    context_refs: Vec<StateRecordRef>,
) -> Vec<StateRecordRef> {
    refs.extend(context_refs);
    refs
}

fn sensitive_approval_required(context: &CloseTaskContext) -> CoreResult<bool> {
    Ok(
        parse_task_control_level(&context.task.effective_control_level)?
            == TaskControlLevel::Sensitive
            || context
                .current_close_basis
                .as_ref()
                .map(|basis| !basis.sensitive_action_requirements.is_empty())
                .unwrap_or(false),
    )
}

fn sensitive_action_basis_missing(context: &CloseTaskContext) -> CoreResult<bool> {
    Ok(
        parse_task_control_level(&context.task.effective_control_level)?
            == TaskControlLevel::Sensitive
            && context
                .current_close_basis
                .as_ref()
                .map(|basis| basis.sensitive_action_requirements.is_empty())
                .unwrap_or(true),
    )
}

fn baseline_stale_for_close(context: &CloseTaskContext) -> CoreResult<bool> {
    let Some(basis) = context.current_close_basis.as_ref() else {
        return Ok(false);
    };
    let current_baseline = StoredScope::from_task(&context.task)?.baseline_ref;
    Ok(basis.baseline_ref.as_ref().map(BaselineRef::as_str) != current_baseline.as_deref())
}

fn recovery_required(context: &CloseTaskContext) -> CoreResult<bool> {
    if !context.blocker_refs.is_empty() {
        return Ok(true);
    }
    context
        .current_change_unit
        .as_ref()
        .map(|record| {
            let lifecycle: PersistedLifecycleState = decode_required_json(
                "change_units",
                record.change_unit_id.clone(),
                "lifecycle_json",
                Some(&record.lifecycle_json),
            )?;
            Ok(lifecycle.recovery_required)
        })
        .transpose()
        .map(|value| value.unwrap_or(false))
}

#[derive(Debug, Clone, Copy)]
struct ResidualRiskState {
    known: bool,
    visible: bool,
}

fn residual_risk_state(context: &CloseTaskContext) -> ResidualRiskState {
    let known = context
        .current_close_basis
        .as_ref()
        .map(|basis| !basis.residual_risks.is_empty())
        .unwrap_or(false);
    ResidualRiskState {
        known,
        visible: known,
    }
}

fn task_ref_for_close(request: &CloseTaskPlanRequest, state_version: u64) -> StateRecordRef {
    state_ref(
        StateRecordKind::Task,
        request.task_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(state_version),
    )
}
