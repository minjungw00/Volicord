use crate::acceptance_facts::active_acceptance_criteria;
use crate::close_readiness::{
    facts_from_projection, normalize_close_blockers, open_write_ticket_close_blocker,
    plan_projected_close_readiness,
};
use crate::enforcement_facts::project_enforcement_profile;
use crate::error_boundary::{
    store::{plan_error_response, store_error_plan},
    user_action::user_action_service_plan_error,
};
use crate::evidence_facts::{
    load_current_evidence_summary_facts, load_required_evidence_criterion_ids,
};
use crate::guarantee_projection::guarantee_display;
use crate::identity::allocate_write_ticket_id;
use crate::json_object::object_from_value;
use crate::method_execution::{mutation_method_policy, prepare_or_response, PlanError};
use crate::method_rejection::{
    infallible_rejected_pipeline_response, no_active_task_response, validation_rejected,
};
use crate::pipeline::{
    commit_mutation_branch, dry_run_preview_branch, tool_error, CommitMutationBranch,
    CorePipelineError, CoreResult, CoreService, InvocationContext, MethodPolicy, PipelineResponse,
    TaskRequirement, VerifiedInvocationContext,
};
use crate::policy::close_readiness_evidence::project_close_evidence_summary;
use crate::policy::workflow::project_workflow_policy;
use crate::record_refs::state_ref;
use crate::state_summary::{project_state_header, state_summary, StateSummaryInput};
use crate::task_facts::{active_blocker_refs, current_close_basis};
use crate::workflow_diagnostics::{
    record_core_workflow_metric_best_effort, response_committed_fresh_effect,
};
use crate::write_ticket::current_validity::{
    ReusableStoredWriteTicket, StoredWriteTicketEvaluation,
};
use crate::write_ticket::read_model::WriteTicketEvidenceFacts;
use crate::write_ticket::summary::{
    project_planned_write_ticket_summary, project_stored_write_ticket_summary,
    PlannedWriteTicketSummaryInput, StoredWriteTicketSummaryInput,
};
use crate::write_ticket::{
    materialize_planned_write_ticket, plan_prepare_write as plan_write_ticket,
    MaterializedPrepareWriteTicket, PlannedWriteTicket, PrepareWriteInput,
    PrepareWritePlanningOutcome, PrepareWriteTicketPlan, WriteTicketDecisionReason,
    WriteTicketPlanningError, WriteTicketRelatedRecord,
};
use serde_json::{json, Map, Value};
use volicord_store::core_pipeline::{CoreProjectStore, CoreStorageMutation, ProjectStateHeader};
use volicord_store::diagnostics::WorkflowMetricKind;
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::{ChangeUnitId, ProjectId, TaskId};
use volicord_types::methods::{
    MethodOperationCategory, PrepareWriteRequest, PrepareWriteResultFields,
};
use volicord_types::product_path::WriteTicketPathScope;
use volicord_types::schema::{
    DryRunSummary, GuaranteeDisplay, JsonObject, PlannedBlocker, PlannedEffect, StateRecordRef,
    StateSummary, WriteDecisionReason as PublicWriteDecisionReason, WriteTicket,
    WriteTicketPathPatterns, WriteTicketScope, WriteTicketStateSummary,
};
use volicord_types::values::{
    CloseState, ErrorCode, MethodName, PlannedBlockerSourceKind, PrepareWriteDecision,
    StateRecordKind, UtcTimestamp, WriteDecisionCategory, WriteTicketEffect, WriteTicketState,
};

struct PrepareWritePlan {
    task_id: TaskId,
    change_unit_id: ChangeUnitId,
    storage_mutations: Vec<CoreStorageMutation>,
    event_kind: String,
    event_payload: JsonObject,
    result_fields: PrepareWriteResultFields,
}

struct PrepareWriteProjectionContext<'a, 'store> {
    service: &'a CoreService,
    store: &'a CoreProjectStore<'store>,
    project_state: &'a ProjectStateHeader,
    request: &'a PrepareWriteRequest,
    verified_invocation: &'a VerifiedInvocationContext,
    operation_now: &'a UtcTimestamp,
}

enum PrepareWriteTicketProjection {
    Issued {
        write_ticket: WriteTicket,
        summary: WriteTicketStateSummary,
    },
    Reused {
        write_ticket: WriteTicket,
        summary: WriteTicketStateSummary,
    },
    None {
        path_patterns: WriteTicketPathPatterns,
    },
}

impl PrepareWriteTicketProjection {
    fn write_ticket_effect(&self) -> WriteTicketEffect {
        match self {
            Self::Issued { .. } => WriteTicketEffect::Issued,
            Self::Reused { .. } => WriteTicketEffect::Reused,
            Self::None { .. } => WriteTicketEffect::None,
        }
    }

    fn write_ticket_ref(&self) -> Option<&StateRecordRef> {
        match self {
            Self::Issued { write_ticket, .. } | Self::Reused { write_ticket, .. } => {
                Some(&write_ticket.write_ticket_ref)
            }
            Self::None { .. } => None,
        }
    }

    fn summary(&self) -> Option<WriteTicketStateSummary> {
        match self {
            Self::Issued { summary, .. } | Self::Reused { summary, .. } => Some(summary.clone()),
            Self::None { .. } => None,
        }
    }

    fn into_result_fields(
        self,
        decision: PrepareWriteDecision,
        state: StateSummary,
        active_user_action_refs: Vec<StateRecordRef>,
        write_decision_reasons: Vec<PublicWriteDecisionReason>,
        guarantee_display: Option<GuaranteeDisplay>,
    ) -> PrepareWriteResultFields {
        match self {
            Self::Issued { write_ticket, .. } => {
                let path_patterns = write_ticket.path_patterns.clone();
                PrepareWriteResultFields {
                    decision,
                    state: Some(state),
                    write_ticket_id: Some(write_ticket.write_ticket_id.clone()),
                    write_ticket_ref: Some(write_ticket.write_ticket_ref.clone()),
                    write_ticket: Some(write_ticket),
                    write_ticket_effect: WriteTicketEffect::Issued,
                    allowed_path_patterns: path_patterns.allowed,
                    denied_path_patterns: path_patterns.denied,
                    active_user_action_refs,
                    write_decision_reasons,
                    user_action_draft: None,
                    guarantee_display,
                }
            }
            Self::Reused { write_ticket, .. } => {
                let path_patterns = write_ticket.path_patterns.clone();
                PrepareWriteResultFields {
                    decision,
                    state: Some(state),
                    write_ticket_id: Some(write_ticket.write_ticket_id.clone()),
                    write_ticket_ref: Some(write_ticket.write_ticket_ref.clone()),
                    write_ticket: Some(write_ticket),
                    write_ticket_effect: WriteTicketEffect::Reused,
                    allowed_path_patterns: path_patterns.allowed,
                    denied_path_patterns: path_patterns.denied,
                    active_user_action_refs,
                    write_decision_reasons,
                    user_action_draft: None,
                    guarantee_display,
                }
            }
            Self::None { path_patterns } => PrepareWriteResultFields {
                decision,
                state: Some(state),
                write_ticket_id: None,
                write_ticket_ref: None,
                write_ticket: None,
                write_ticket_effect: WriteTicketEffect::None,
                allowed_path_patterns: path_patterns.allowed,
                denied_path_patterns: path_patterns.denied,
                active_user_action_refs,
                write_decision_reasons,
                user_action_draft: None,
                guarantee_display,
            },
        }
    }
}

impl CoreService {
    /// Executes `volicord.prepare_write` through the shared Core mutation pipeline.
    pub fn prepare_write(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        request: PrepareWriteRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = request.envelope.task_id.as_ref() {
            if request
                .task_id
                .as_ref()
                .is_some_and(|task_id| task_id != envelope_task_id)
            {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match PrepareWriteRequest.task_id",
                );
            }
        }
        let policy = prepare_write_policy(&request);
        let prepared = match prepare_or_response(
            self,
            Some(context),
            MethodName::PrepareWrite,
            request.envelope.clone(),
            request_json,
            invocation,
            policy,
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let ticket_task_id = request.task_id.clone().unwrap_or_else(|| {
            prepared
                .context
                .resolved_task_id
                .clone()
                .expect("prepare_write preflight resolves an exact Task")
        });
        let planned = match plan_prepare_write(
            &prepared.store,
            &prepared.context.project_state,
            &request,
            &ticket_task_id,
            &prepared.context.verified_invocation,
            &prepared.operation_now,
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
        let write_decision_reasons = project_write_decision_reasons(
            &request.envelope.project_id,
            prepared.context.project_state.state_version,
            &planned.common.reasons,
        );

        if request.envelope.dry_run.is_requested() {
            let dry_run_summary =
                prepare_write_dry_run_summary(&planned.ticket, &write_decision_reasons);
            return self.execute_prepared_request(
                prepared,
                dry_run_preview_branch::<PrepareWriteRequest>(dry_run_summary),
            );
        }
        let had_prior_write_ticket = !prepared
            .store
            .write_tickets_for_task(&ticket_task_id)?
            .is_empty();
        let plan = match project_prepare_write_response(
            PrepareWriteProjectionContext {
                service: self,
                store: &prepared.store,
                project_state: &prepared.context.project_state,
                request: &request,
                verified_invocation: &prepared.context.verified_invocation,
                operation_now: &prepared.operation_now,
            },
            planned,
            write_decision_reasons,
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

        let metric_kind = match plan.result_fields.write_ticket_effect {
            WriteTicketEffect::Reused => Some(WorkflowMetricKind::WriteTicketReused),
            WriteTicketEffect::Issued if had_prior_write_ticket => {
                Some(WorkflowMetricKind::WriteTicketReissued)
            }
            WriteTicketEffect::Issued => Some(WorkflowMetricKind::WriteTicketIssued),
            _ => None,
        };
        let sensitive_approval_missing = plan
            .result_fields
            .write_decision_reasons
            .iter()
            .any(|reason| reason.code == "sensitive_approval_missing");
        let session_id = prepared.context.verified_invocation.session_id.clone();
        let response = self.execute_prepared_request(
            prepared,
            commit_mutation_branch::<PrepareWriteRequest>(CommitMutationBranch {
                result_fields: plan.result_fields,
                event_kind: plan.event_kind,
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: Some(plan.change_unit_id),
                storage_mutations: plan.storage_mutations,
            }),
        )?;
        if response_committed_fresh_effect(&response) {
            if let Some(metric_kind) = metric_kind {
                record_core_workflow_metric_best_effort(
                    context,
                    session_id.as_deref(),
                    metric_kind,
                    1,
                );
            }
            if sensitive_approval_missing {
                record_core_workflow_metric_best_effort(
                    context,
                    session_id.as_deref(),
                    WorkflowMetricKind::SensitiveApprovalMissingBlock,
                    1,
                );
            }
        }
        Ok(response)
    }
}

fn prepare_write_policy(request: &PrepareWriteRequest) -> MethodPolicy {
    let task = request
        .task_id
        .clone()
        .or_else(|| request.envelope.task_id.as_ref().cloned())
        .map(TaskRequirement::Exact)
        .unwrap_or(TaskRequirement::Required);

    mutation_method_policy(
        MethodName::PrepareWrite,
        request.operation_category(),
        task,
        request.envelope.dry_run,
    )
}

fn prepare_write_change_unit_required_response(
    request: &PrepareWriteRequest,
    project_state: &ProjectStateHeader,
    task_id: &TaskId,
) -> PipelineResponse {
    let mut details = Map::new();
    details.insert(
        "reason".to_owned(),
        Value::String("current_change_unit_required".to_owned()),
    );
    details.insert(
        "method".to_owned(),
        Value::String(MethodName::PrepareWrite.as_str().to_owned()),
    );
    details.insert(
        "project_id".to_owned(),
        Value::String(request.envelope.project_id.as_str().to_owned()),
    );
    details.insert(
        "task_id".to_owned(),
        Value::String(task_id.as_str().to_owned()),
    );
    infallible_rejected_pipeline_response(
        request.envelope.dry_run,
        Some(project_state.state_version),
        vec![tool_error(
            ErrorCode::NoActiveChangeUnit,
            "write preparation requires a current Change Unit",
            false,
            Some(details),
        )],
    )
}

fn plan_prepare_write(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &PrepareWriteRequest,
    task_id: &TaskId,
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &UtcTimestamp,
) -> Result<PrepareWritePlanningOutcome, PlanError> {
    let task_is_current = !project_state
        .active_task_id
        .as_deref()
        .is_some_and(|active_task_id| active_task_id != task_id.as_str());
    plan_write_ticket(
        store,
        PrepareWriteInput::new(
            request.envelope.project_id.clone(),
            task_id.clone(),
            task_is_current,
            request.change_unit_id.as_ref().cloned(),
            request.intended_operation.clone(),
            request.intended_paths.clone(),
            request.product_file_write_intended,
            request.sensitive_categories.clone(),
            request.baseline_ref.clone(),
            verified_invocation.actor_source.clone(),
            verified_invocation.git_workspace_context.clone(),
            verified_invocation.verification_basis.clone(),
        ),
        operation_now,
    )
    .map_err(|error| prepare_write_planning_error(request, project_state, error))
}

fn prepare_write_planning_error(
    request: &PrepareWriteRequest,
    project_state: &ProjectStateHeader,
    error: WriteTicketPlanningError,
) -> PlanError {
    match error {
        WriteTicketPlanningError::Store(error) => {
            store_error_plan(&request.envelope, project_state, error)
        }
        WriteTicketPlanningError::Core(error) => PlanError::Core(error),
        WriteTicketPlanningError::UserAction(error) => {
            user_action_service_plan_error(&request.envelope, project_state, error)
        }
        WriteTicketPlanningError::NoActiveTask => PlanError::Response(Box::new(
            no_active_task_response(&request.envelope, project_state),
        )),
        WriteTicketPlanningError::CurrentChangeUnitRequired { task_id } => {
            PlanError::Response(Box::new(prepare_write_change_unit_required_response(
                request,
                project_state,
                &task_id,
            )))
        }
        WriteTicketPlanningError::Validation { field, message } => {
            match validation_rejected(
                request.envelope.dry_run,
                Some(project_state.state_version),
                field.as_str(),
                message,
            ) {
                Ok(response) => PlanError::Response(Box::new(response)),
                Err(error) => PlanError::Core(error),
            }
        }
        WriteTicketPlanningError::ProductPathContainment { message } => {
            let mut details = Map::new();
            details.insert(
                "field".to_owned(),
                Value::String("intended_paths".to_owned()),
            );
            PlanError::Response(Box::new(infallible_rejected_pipeline_response(
                request.envelope.dry_run,
                Some(project_state.state_version),
                vec![tool_error(
                    ErrorCode::InvocationContextMismatch,
                    message,
                    false,
                    Some(details),
                )],
            )))
        }
        WriteTicketPlanningError::Invariant { detail } => {
            PlanError::Core(CorePipelineError::Invariant { detail })
        }
    }
}

fn materialize_prepare_write_ticket(
    service: &CoreService,
    store: &CoreProjectStore<'_>,
    project_state: &ProjectStateHeader,
    request: &PrepareWriteRequest,
    ticket: PrepareWriteTicketPlan,
    planned_state_version: u64,
) -> Result<MaterializedPrepareWriteTicket, PlanError> {
    match ticket {
        PrepareWriteTicketPlan::Issue(draft) => {
            let write_ticket_id = allocate_write_ticket_id(service.durable_id_generator(), store)?;
            let planned = materialize_planned_write_ticket(
                draft,
                write_ticket_id,
                planned_state_version,
                project_state.state_version,
            )
            .map_err(|error| prepare_write_planning_error(request, project_state, error))?;
            Ok(MaterializedPrepareWriteTicket::Issued(planned))
        }
        PrepareWriteTicketPlan::Reuse(ticket) => Ok(MaterializedPrepareWriteTicket::Reused(ticket)),
        PrepareWriteTicketPlan::NoTicket(facts) => Ok(MaterializedPrepareWriteTicket::None(facts)),
    }
}

fn project_materialized_prepare_write_ticket(
    materialized: &MaterializedPrepareWriteTicket,
    state_version: u64,
    guarantee_display: Option<GuaranteeDisplay>,
) -> PrepareWriteTicketProjection {
    match materialized {
        MaterializedPrepareWriteTicket::Issued(planned) => {
            let write_ticket =
                project_issued_write_ticket(planned, state_version, guarantee_display.clone());
            let summary = project_planned_write_ticket_summary(PlannedWriteTicketSummaryInput {
                planned,
                state_version,
                guarantee_display: guarantee_display.clone(),
            });
            PrepareWriteTicketProjection::Issued {
                write_ticket,
                summary,
            }
        }
        MaterializedPrepareWriteTicket::Reused(reusable) => {
            let write_ticket =
                project_reused_write_ticket(reusable, state_version, guarantee_display.clone());
            let evaluated = StoredWriteTicketEvaluation::Reusable(reusable.clone());
            let summary = project_stored_write_ticket_summary(StoredWriteTicketSummaryInput {
                evaluated: &evaluated,
                state_version,
                evidence: &WriteTicketEvidenceFacts::default(),
                guarantee_display: guarantee_display.clone(),
            });
            PrepareWriteTicketProjection::Reused {
                write_ticket,
                summary,
            }
        }
        MaterializedPrepareWriteTicket::None(_) => PrepareWriteTicketProjection::None {
            path_patterns: project_write_ticket_path_scope(materialized.path_scope()),
        },
    }
}

fn project_issued_write_ticket(
    planned: &PlannedWriteTicket,
    state_version: u64,
    guarantee_display: Option<GuaranteeDisplay>,
) -> WriteTicket {
    let write_ticket_ref = state_ref(
        StateRecordKind::WriteTicket,
        planned.write_ticket_id().as_str(),
        planned.project_id(),
        Some(&planned.validity_basis().task_id),
        Some(state_version),
    );
    let attempt_scope = planned.attempt_scope();
    WriteTicket {
        write_ticket_id: planned.write_ticket_id().clone(),
        write_ticket_ref,
        state: WriteTicketState::Open,
        scope: WriteTicketScope {
            task_id: attempt_scope.task_id.clone(),
            change_unit_id: attempt_scope.change_unit_id.clone(),
            intended_operation: attempt_scope.intended_operation.clone(),
            product_file_write_intended: attempt_scope.product_file_write_intended,
            sensitive_categories: attempt_scope.sensitive_categories.clone(),
            baseline_ref: attempt_scope.baseline_ref.clone(),
        },
        path_patterns: project_write_ticket_path_scope(planned.path_scope()),
        observed_paths: Vec::new(),
        basis_state_version: planned.basis_state_version(),
        validity_basis: planned.validity_basis().clone(),
        invalidation_reason: None,
        idle_expires_at: planned.idle_expires_at().cloned(),
        guarantee_display,
    }
}

fn project_reused_write_ticket(
    reusable: &ReusableStoredWriteTicket,
    state_version: u64,
    guarantee_display: Option<GuaranteeDisplay>,
) -> WriteTicket {
    let semantic = reusable.semantic_facts();
    let write_ticket_ref = state_ref(
        StateRecordKind::WriteTicket,
        reusable.write_ticket_id().as_str(),
        &semantic.project_id,
        Some(&semantic.validity_basis.task_id),
        Some(state_version),
    );
    let attempt_scope = &semantic.attempt_scope;
    WriteTicket {
        write_ticket_id: reusable.write_ticket_id().clone(),
        write_ticket_ref,
        state: WriteTicketState::Open,
        scope: WriteTicketScope {
            task_id: attempt_scope.task_id.clone(),
            change_unit_id: attempt_scope.change_unit_id.clone(),
            intended_operation: attempt_scope.intended_operation.clone(),
            product_file_write_intended: attempt_scope.product_file_write_intended,
            sensitive_categories: attempt_scope.sensitive_categories.clone(),
            baseline_ref: attempt_scope.baseline_ref.clone(),
        },
        path_patterns: project_write_ticket_path_scope(reusable.path_scope()),
        observed_paths: Vec::new(),
        basis_state_version: semantic.basis_state_version,
        validity_basis: semantic.validity_basis.clone(),
        invalidation_reason: None,
        idle_expires_at: semantic.idle_expires_at.clone(),
        guarantee_display,
    }
}

fn project_write_ticket_path_scope(path_scope: &WriteTicketPathScope) -> WriteTicketPathPatterns {
    WriteTicketPathPatterns {
        allowed: path_scope
            .allowed()
            .iter()
            .map(|path| path.as_str().to_owned())
            .collect(),
        denied: path_scope
            .denied()
            .iter()
            .map(|path| path.as_str().to_owned())
            .collect(),
    }
}

fn project_prepare_write_response(
    context: PrepareWriteProjectionContext<'_, '_>,
    planned: PrepareWritePlanningOutcome,
    reasons: Vec<PublicWriteDecisionReason>,
) -> Result<PrepareWritePlan, PlanError> {
    let PrepareWriteProjectionContext {
        service,
        store,
        project_state,
        request,
        verified_invocation,
        operation_now,
    } = context;
    let PrepareWritePlanningOutcome {
        common,
        ticket,
        mutations,
    } = planned;
    let task_id = common.task_id;
    let task = common.task;
    let change_unit = common.change_unit;
    let decision = common.decision;
    let pending_user_action_request_ids = common.pending_user_action_request_ids;
    let approval_basis = common.approval_basis;
    let mut storage_mutations = mutations.storage_mutations;
    let planned_state_version = project_state.state_version + 1;
    let pending_user_action_refs = pending_user_action_request_ids
        .iter()
        .map(|request_id| {
            state_ref(
                StateRecordKind::UserActionRequest,
                request_id.as_str(),
                &request.envelope.project_id,
                Some(&task_id),
                Some(project_state.state_version),
            )
        })
        .collect::<Vec<_>>();
    let active_user_action_refs = approval_basis
        .as_ref()
        .map(|basis| basis.state_refs(project_state.state_version))
        .unwrap_or_default();
    let materialized_ticket = materialize_prepare_write_ticket(
        service,
        store,
        project_state,
        request,
        ticket,
        planned_state_version,
    )?;
    if let Some(mutation) = materialized_ticket.persistence_mutation() {
        storage_mutations.push(mutation);
    }
    let enforcement_profile = project_enforcement_profile(store)?;
    let guarantee_display = Some(guarantee_display(
        &enforcement_profile,
        verified_invocation,
        planned_state_version,
    ));
    let change_unit_id = ChangeUnitId::new(change_unit.change_unit_id.clone());
    let ticket_projection = project_materialized_prepare_write_ticket(
        &materialized_ticket,
        planned_state_version,
        guarantee_display.clone(),
    );
    let blocker_refs = active_blocker_refs(store, &task_id, planned_state_version)?;
    let evidence_facts = load_current_evidence_summary_facts(
        store,
        &task,
        &request.envelope.project_id,
        &task_id,
        planned_state_version,
    )?;
    let required_criteria = load_required_evidence_criterion_ids(store, &task_id)?;
    let evidence_summary = project_close_evidence_summary(evidence_facts, &required_criteria);
    let projected_project_state = project_state_header(
        project_state,
        planned_state_version,
        project_state
            .active_task_id
            .clone()
            .or_else(|| Some(task_id.as_str().to_owned())),
    );
    let close_plan = plan_projected_close_readiness(
        store,
        &projected_project_state,
        &request.envelope.project_id,
        &task_id,
        facts_from_projection(
            task.clone(),
            Some(change_unit.clone()),
            current_close_basis(store, &task_id)?,
            pending_user_action_refs.clone(),
            blocker_refs.clone(),
            evidence_summary.clone(),
            operation_now.clone(),
        ),
    )
    .map_err(|error| {
        crate::error_boundary::close_readiness::close_readiness_plan_error(
            &request.envelope,
            &projected_project_state,
            error,
        )
    })?;
    let mut close_state = close_plan.close_state;
    let mut close_blockers = close_plan.blockers;
    if ticket_projection.write_ticket_effect() == WriteTicketEffect::Issued {
        if let Some(write_ticket_ref) = ticket_projection.write_ticket_ref() {
            let planned_task_ref = state_ref(
                StateRecordKind::Task,
                task_id.as_str(),
                &request.envelope.project_id,
                Some(&task_id),
                Some(planned_state_version),
            );
            close_blockers.insert(
                0,
                open_write_ticket_close_blocker(planned_task_ref, write_ticket_ref.clone()),
            );
            close_state = CloseState::Blocked;
        }
    }
    normalize_close_blockers(&mut close_blockers, planned_state_version);
    let project_policy = project_workflow_policy(store)
        .map_err(CorePipelineError::from)?
        .summary;
    let state = state_summary(StateSummaryInput {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task,
        current_change_unit: Some(&change_unit),
        project_policy,
        acceptance_criteria: active_acceptance_criteria(store, &task_id)?,
        pending_user_action_refs,
        blocker_refs,
        write_ticket_summary: ticket_projection.summary(),
        evidence_summary,
        evidence_gate: Some(close_plan.evidence_gate),
        close_state: Some(close_state),
        close_blockers,
        guarantee_display: guarantee_display.clone(),
    })?;
    let materialized_effect = materialized_ticket.write_ticket_effect();
    let event_kind = if materialized_effect == WriteTicketEffect::Issued {
        "write_ticket_issued"
    } else if materialized_effect == WriteTicketEffect::Reused {
        "write_ticket_reused"
    } else {
        "write_decision_recorded"
    }
    .to_owned();
    let mut event_payload = object_from_value(json!({
        "task_id": task_id.clone(),
        "change_unit_id": change_unit_id.clone(),
        "decision": decision,
        "write_ticket_id": materialized_ticket
            .write_ticket_id()
            .map(|id| id.as_str().to_owned())
    }))?;
    if materialized_effect == WriteTicketEffect::None {
        event_payload.insert(
            "write_decision_reasons".to_owned(),
            serde_json::to_value(&reasons)?,
        );
    }
    let result_fields = ticket_projection.into_result_fields(
        decision,
        state,
        active_user_action_refs,
        reasons.clone(),
        guarantee_display,
    );

    Ok(PrepareWritePlan {
        task_id,
        change_unit_id,
        storage_mutations,
        event_kind,
        event_payload,
        result_fields,
    })
}

fn project_write_decision_reasons(
    project_id: &ProjectId,
    current_state_version: u64,
    reasons: &[WriteTicketDecisionReason],
) -> Vec<PublicWriteDecisionReason> {
    reasons
        .iter()
        .map(|reason| PublicWriteDecisionReason {
            category: reason.category,
            code: reason.code.as_str().to_owned(),
            message: reason.message.to_owned(),
            related_refs: reason
                .related_records
                .iter()
                .map(|record| match record {
                    WriteTicketRelatedRecord::Task(task_id) => state_ref(
                        StateRecordKind::Task,
                        task_id.as_str(),
                        project_id,
                        Some(task_id),
                        Some(current_state_version),
                    ),
                    WriteTicketRelatedRecord::CurrentChangeUnit {
                        task_id,
                        change_unit_id,
                    } => state_ref(
                        StateRecordKind::ChangeUnit,
                        change_unit_id.as_str(),
                        project_id,
                        Some(task_id),
                        Some(current_state_version),
                    ),
                    WriteTicketRelatedRecord::UserActionRequest {
                        task_id,
                        request_id,
                    } => state_ref(
                        StateRecordKind::UserActionRequest,
                        request_id.as_str(),
                        project_id,
                        Some(task_id),
                        Some(current_state_version),
                    ),
                })
                .collect(),
        })
        .collect()
}

fn prepare_write_dry_run_summary(
    ticket: &PrepareWriteTicketPlan,
    reasons: &[PublicWriteDecisionReason],
) -> DryRunSummary {
    let planned_effect = match ticket {
        PrepareWriteTicketPlan::Issue(_) => Some(PlannedEffect {
            target_kind: "write_ticket".to_owned(),
            action: "would_issue".to_owned(),
            description: "Prepare write would issue one open write ticket.".to_owned(),
        }),
        PrepareWriteTicketPlan::Reuse(_) => Some(PlannedEffect {
            target_kind: "write_ticket".to_owned(),
            action: "would_reuse".to_owned(),
            description: "Prepare write would reuse the compatible open write ticket.".to_owned(),
        }),
        PrepareWriteTicketPlan::NoTicket(_) => None,
    };
    DryRunSummary {
        planned_effects: planned_effect.into_iter().collect(),
        would_blockers: reasons
            .iter()
            .map(|reason| PlannedBlocker {
                source_kind: PlannedBlockerSourceKind::WriteDecision,
                category: write_decision_category_value(reason.category).to_owned(),
                code: reason.code.clone(),
                message: reason.message.clone(),
                related_refs: reason.related_refs.clone(),
            })
            .collect(),
        would_errors: Vec::new(),
        next_actions: Vec::new(),
        diagnostics: Vec::new(),
    }
}

fn write_decision_category_value(category: WriteDecisionCategory) -> &'static str {
    match category {
        WriteDecisionCategory::Scope => "scope",
        WriteDecisionCategory::Workspace => "workspace",
        WriteDecisionCategory::UserAction => "user_action",
        WriteDecisionCategory::SensitiveApproval => "sensitive_approval",
        WriteDecisionCategory::WriteCompatibility => "write_compatibility",
        WriteDecisionCategory::Baseline => "baseline",
        WriteDecisionCategory::EffectContract => "effect_contract",
        WriteDecisionCategory::ConnectionCapability => "connection_capability",
    }
}
