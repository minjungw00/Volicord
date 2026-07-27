use super::close_readiness::{
    assess_close_readiness, close_next_action, CloseReadinessAssessment, CloseReadinessFacts,
    CloseReadinessRequest,
};
use super::{
    active_acceptance_criteria_for_task, build_state_summary, changes_summary_text,
    close_state_text, decode_required_json_object, dry_run_summary, elapsed_micros,
    evidence_gate_summary_text, evidence_summary_for_display, guarantee_display_from_profile,
    mutation_method_policy, object_from_value, parse_task_mode, plan_error_response,
    plan_project_continuity_record, prepare_or_response, primary_next_action, profile_summary_text,
    projected_write_ticket_summary, record_core_workflow_metric_best_effort,
    response_committed_fresh_effect, state_ref, summary_card_for_core, validation_rejected,
    write_ticket_summary_text, CloseTaskPlan, PlanError, PlannedProjectContinuityRecord,
    ProjectContinuityDraft, ProjectContinuityPlanContext, SummaryBuild, SummaryCardBuild,
};
use crate::pipeline::{
    CorePipelineError, CoreResult, CoreService, FreshnessPolicy, InvocationContext,
    MethodEffectPolicy, MethodPolicy, OwnerPipelineBranch, PipelineResponse, ReplayPolicy,
    TaskRequirement, VerifiedInvocationContext,
};
use crate::user_action::summary::agent_safe_pending_user_action_summaries;
use serde_json::json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use volicord_store::core_pipeline::{
    CoreProjectStore, CoreStorageMutation, ProjectStateHeader, TaskCloseUpdate, TaskMutation,
    TaskRecord, WriteTicketInvalidation, WriteTicketMutation,
};
use volicord_store::diagnostics::WorkflowMetricKind;
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::{ChangeUnitId, TaskId};
use volicord_types::methods::{CheckCloseRequest, CloseTaskRequest, CloseTaskResultFields};
use volicord_types::schema::{
    AuthorityReceipt, CloseReadinessBlocker, CurrentCloseBasis, DryRunSummary, EvidenceGateSummary,
    JsonObject, NextActionSummary, ProjectEnforcementProfile, RequiredNullable,
    RiskAcceptanceCoverage, ToolEnvelope,
};
use volicord_types::values::{
    AuthorityNextActor, CloseIntent, CloseReason, CloseState, MethodName, OperationCategory,
    ProjectContinuityKind, StateRecordKind, StatusCloseState, TaskMode, UtcTimestamp,
    WriteTicketInvalidationReason,
};

/// Canonical close-method request after request-local identity and intent validation.
#[derive(Debug, Clone, PartialEq)]
struct CloseTaskPlanRequest {
    envelope: ToolEnvelope,
    task_id: TaskId,
    intent: CloseIntent,
    close_reason: RequiredNullable<CloseReason>,
    superseding_task_id: RequiredNullable<TaskId>,
    user_note: RequiredNullable<String>,
}

impl CloseTaskPlanRequest {
    fn check(request: CheckCloseRequest) -> Self {
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

    fn readiness_request(&self) -> CloseReadinessRequest {
        if self.intent == CloseIntent::Check {
            CloseReadinessRequest::check(self.envelope.clone(), self.task_id.clone())
        } else {
            CloseReadinessRequest::terminal(
                self.envelope.clone(),
                self.task_id.clone(),
                self.intent,
                self.superseding_task_id.as_ref().cloned(),
            )
        }
    }
}

struct CloseTaskPlannedMutations {
    request: CloseTaskPlanRequest,
    context: CloseReadinessFacts,
    risk_acceptance_coverage: Vec<RiskAcceptanceCoverage>,
    blockers: Vec<CloseReadinessBlocker>,
    response_state_version: u64,
    close_state: CloseState,
    evidence_gate: EvidenceGateSummary,
    synthetic_task: TaskRecord,
    storage_mutations: Vec<CoreStorageMutation>,
    event_kind: String,
    event_payload: JsonObject,
}

struct CloseTaskResponseProjection {
    task_id: TaskId,
    change_unit_id: Option<ChangeUnitId>,
    storage_mutations: Vec<CoreStorageMutation>,
    event_kind: String,
    event_payload: JsonObject,
    result_fields: CloseTaskResultFields,
    current_close_basis: Option<CurrentCloseBasis>,
    blockers: Vec<CloseReadinessBlocker>,
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
            current_close_basis: self.current_close_basis,
            blockers: self.blockers,
        }
    }
}

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

fn plan_close_task(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: Option<&VerifiedInvocationContext>,
    guarantee_profile: Option<&ProjectEnforcementProfile>,
    request: CloseTaskPlanRequest,
    now: &UtcTimestamp,
) -> Result<CloseTaskPlan, PlanError> {
    let assessment =
        assess_close_readiness(store, project_state, request.readiness_request(), now)?;
    let mutations = plan_close_task_mutations(now, request, assessment)?;
    Ok(project_close_task_response(
        store,
        verified_invocation,
        guarantee_profile,
        now,
        mutations,
    )?
    .into_plan())
}

fn plan_close_task_mutations(
    now: &UtcTimestamp,
    request: CloseTaskPlanRequest,
    decision: CloseReadinessAssessment,
) -> Result<CloseTaskPlannedMutations, PlanError> {
    let CloseReadinessAssessment {
        context,
        control_update,
        risk_acceptance_coverage,
        blockers,
        committed_terminal,
        response_state_version,
        close_state,
        evidence_gate,
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
        evidence_gate,
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
        evidence_gate,
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
        changes: changes_summary_text(true, context.unresolved_unrecorded_changes.len() as u64),
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
        current_close_basis,
        blockers,
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
            applies_to_refs: {
                let mut refs = close_basis.result_refs.clone();
                refs.extend(risk.source_refs.clone());
                refs
            },
            source_refs: {
                let mut refs = vec![close_basis.source_run_ref.clone()];
                refs.extend(risk.source_refs.clone());
                refs
            },
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
