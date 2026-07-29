use crate::acceptance_facts::active_acceptance_criteria;
use crate::close_readiness::{
    facts_from_projection, normalize_close_blockers, open_write_ticket_close_blocker,
    plan_projected_close_readiness,
};
use crate::error_boundary::{
    store::{plan_error_response, store_error_plan},
    user_action::user_action_service_plan_error,
};
use crate::evidence_facts::{
    load_current_evidence_summary_facts, load_required_evidence_criterion_ids,
};
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
    evaluate_planned_write_ticket, evaluate_reused_write_ticket,
};
use crate::write_ticket::read_model::{stored_write_ticket_facts, WriteTicketEvidenceFacts};
use crate::write_ticket::summary::{project_write_ticket_summary, WriteTicketSummaryInput};
use crate::write_ticket::{
    plan_prepare_write as plan_write_ticket, PrepareWritePlannedMutations, WriteTicketPlanningError,
};
use serde_json::{json, Map, Value};
use volicord_store::core_pipeline::{CoreProjectStore, CoreStorageMutation, ProjectStateHeader};
use volicord_store::diagnostics::WorkflowMetricKind;
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::{ChangeUnitId, TaskId};
use volicord_types::methods::{
    MethodOperationCategory, PrepareWriteRequest, PrepareWriteResultFields,
};
use volicord_types::schema::{
    DryRunSummary, JsonObject, PlannedBlocker, PlannedEffect, WriteDecisionReason, WriteTicket,
    WriteTicketPathPatterns, WriteTicketScope,
};
use volicord_types::values::{
    CloseState, ErrorCode, MethodName, PlannedBlockerSourceKind, StateRecordKind, UtcTimestamp,
    WriteDecisionCategory, WriteTicketEffect, WriteTicketState,
};

struct PrepareWritePlan {
    task_id: TaskId,
    change_unit_id: ChangeUnitId,
    storage_mutations: Vec<CoreStorageMutation>,
    event_kind: String,
    event_payload: JsonObject,
    result_fields: PrepareWriteResultFields,
    dry_run_summary: DryRunSummary,
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
        let had_prior_write_ticket = !prepared
            .store
            .write_tickets_for_task(&ticket_task_id)?
            .is_empty();
        let plan = match plan_prepare_write(
            self,
            &prepared.store,
            &prepared.context.project_state,
            request.clone(),
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

        if request.envelope.dry_run.is_requested() {
            return self.execute_prepared_request(
                prepared,
                dry_run_preview_branch::<PrepareWriteRequest>(plan.dry_run_summary),
            );
        }

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

struct PrepareWriteResponseProjection {
    task_id: TaskId,
    change_unit_id: ChangeUnitId,
    storage_mutations: Vec<CoreStorageMutation>,
    event_kind: String,
    event_payload: JsonObject,
    result_fields: PrepareWriteResultFields,
    dry_run_summary: DryRunSummary,
}

impl PrepareWriteResponseProjection {
    fn into_plan(self) -> PrepareWritePlan {
        PrepareWritePlan {
            task_id: self.task_id,
            change_unit_id: self.change_unit_id,
            storage_mutations: self.storage_mutations,
            event_kind: self.event_kind,
            event_payload: self.event_payload,
            result_fields: self.result_fields,
            dry_run_summary: self.dry_run_summary,
        }
    }
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
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: PrepareWriteRequest,
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &UtcTimestamp,
) -> Result<PrepareWritePlan, PlanError> {
    let mutations = plan_write_ticket(
        service.durable_id_generator(),
        store,
        project_state,
        request.clone(),
        verified_invocation,
        operation_now,
    )
    .map_err(|error| prepare_write_planning_error(&request, project_state, error))?;
    Ok(project_prepare_write_response(store, project_state, mutations)?.into_plan())
}

fn prepare_write_planning_error(
    request: &PrepareWriteRequest,
    project_state: &ProjectStateHeader,
    error: WriteTicketPlanningError,
) -> PlanError {
    match error {
        WriteTicketPlanningError::Core(CorePipelineError::Store(error)) => {
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
                field,
                message,
            ) {
                Ok(response) => PlanError::Response(Box::new(response)),
                Err(error) => PlanError::Core(error),
            }
        }
        WriteTicketPlanningError::ProductPathContainment { field, message } => {
            let mut details = Map::new();
            details.insert("field".to_owned(), Value::String(field.to_owned()));
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
    }
}

fn project_prepare_write_response(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    planned: PrepareWritePlannedMutations,
) -> Result<PrepareWriteResponseProjection, PlanError> {
    let PrepareWritePlannedMutations {
        request,
        planned_state_version,
        plan_now,
        task_id,
        task,
        change_unit,
        reasons,
        decision,
        allowed,
        pending_user_action_refs,
        active_user_action_refs,
        guarantee_display,
        write_ticket_id,
        write_ticket_ref,
        planned_write_ticket,
        reused_write_ticket,
        write_ticket_effect,
        allowed_path_patterns,
        denied_path_patterns,
        storage_mutations,
    } = planned;
    let would_reuse_write_ticket = reused_write_ticket.is_some();
    let change_unit_id = ChangeUnitId::new(change_unit.change_unit_id.clone());
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
            plan_now.clone(),
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
    if write_ticket_effect == WriteTicketEffect::Issued {
        if let Some(write_ticket_ref) = write_ticket_ref.as_ref() {
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
    let write_ticket = match (
        write_ticket_id.as_ref(),
        write_ticket_ref.as_ref(),
        planned_write_ticket.as_ref(),
        reused_write_ticket.as_ref(),
    ) {
        (Some(write_ticket_id), Some(write_ticket_ref), Some(plan), None) => {
            let selected_scope = plan.attempt_scope();
            Some(WriteTicket {
                write_ticket_id: write_ticket_id.clone(),
                write_ticket_ref: write_ticket_ref.clone(),
                state: WriteTicketState::Open,
                scope: WriteTicketScope {
                    task_id: task_id.clone(),
                    change_unit_id: change_unit_id.clone(),
                    intended_operation: selected_scope.intended_operation.clone(),
                    product_file_write_intended: selected_scope.product_file_write_intended,
                    sensitive_categories: selected_scope.sensitive_categories.clone(),
                    baseline_ref: selected_scope.baseline_ref.clone(),
                },
                path_patterns: WriteTicketPathPatterns {
                    allowed: allowed_path_patterns.clone(),
                    denied: denied_path_patterns.clone(),
                },
                observed_paths: Vec::new(),
                basis_state_version: plan.basis_state_version(),
                validity_basis: plan.validity_basis().clone(),
                invalidation_reason: None,
                idle_expires_at: plan.idle_expires_at().cloned(),
                guarantee_display: guarantee_display.clone(),
            })
        }
        (Some(write_ticket_id), Some(write_ticket_ref), None, Some(record)) => {
            let selected_scope = record.attempt_scope();
            Some(WriteTicket {
                write_ticket_id: write_ticket_id.clone(),
                write_ticket_ref: write_ticket_ref.clone(),
                state: WriteTicketState::Open,
                scope: WriteTicketScope {
                    task_id: task_id.clone(),
                    change_unit_id: change_unit_id.clone(),
                    intended_operation: selected_scope.intended_operation.clone(),
                    product_file_write_intended: selected_scope.product_file_write_intended,
                    sensitive_categories: selected_scope.sensitive_categories.clone(),
                    baseline_ref: selected_scope.baseline_ref.clone(),
                },
                path_patterns: WriteTicketPathPatterns {
                    allowed: allowed_path_patterns.clone(),
                    denied: denied_path_patterns.clone(),
                },
                observed_paths: Vec::new(),
                basis_state_version: record.basis_state_version(),
                validity_basis: record.validity_basis().clone(),
                invalidation_reason: None,
                idle_expires_at: record.idle_expires_at().cloned(),
                guarantee_display: guarantee_display.clone(),
            })
        }
        _ => None,
    };
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
        write_ticket_summary: if let Some(plan) = planned_write_ticket.as_ref() {
            let evaluated = evaluate_planned_write_ticket(plan);
            Some(project_write_ticket_summary(WriteTicketSummaryInput {
                evaluated: &evaluated,
                state_version: planned_state_version,
                evidence: &WriteTicketEvidenceFacts::default(),
                guarantee_display: guarantee_display.clone(),
            }))
        } else {
            reused_write_ticket.as_ref().map(|record| {
                let evaluated = evaluate_reused_write_ticket(stored_write_ticket_facts(record));
                project_write_ticket_summary(WriteTicketSummaryInput {
                    evaluated: &evaluated,
                    state_version: planned_state_version,
                    evidence: &WriteTicketEvidenceFacts::default(),
                    guarantee_display: guarantee_display.clone(),
                })
            })
        },
        evidence_summary,
        evidence_gate: Some(close_plan.evidence_gate),
        close_state: Some(close_state),
        close_blockers,
        guarantee_display: guarantee_display.clone(),
    })?;
    let result_fields = PrepareWriteResultFields {
        decision,
        state: Some(state),
        write_ticket_id: write_ticket_id.clone(),
        write_ticket_ref: write_ticket_ref.clone(),
        write_ticket,
        write_ticket_effect,
        allowed_path_patterns: allowed_path_patterns.clone(),
        denied_path_patterns: denied_path_patterns.clone(),
        active_user_action_refs,
        write_decision_reasons: reasons.clone(),
        user_action_draft: None,
        guarantee_display: guarantee_display.clone(),
    };

    let event_kind = if write_ticket_effect == WriteTicketEffect::Reused {
        "write_ticket_reused"
    } else if allowed {
        "write_ticket_issued"
    } else {
        "write_decision_recorded"
    }
    .to_owned();
    let mut event_payload = object_from_value(json!({
        "task_id": task_id.clone(),
        "change_unit_id": change_unit_id.clone(),
        "decision": decision,
        "write_ticket_id": write_ticket_id
            .as_ref()
            .map(|id| id.as_str().to_owned())
    }))?;
    if !allowed {
        event_payload.insert(
            "write_decision_reasons".to_owned(),
            serde_json::to_value(&reasons)?,
        );
    }

    Ok(PrepareWriteResponseProjection {
        task_id,
        change_unit_id,
        storage_mutations,
        event_kind,
        event_payload,
        result_fields,
        dry_run_summary: prepare_write_dry_run_summary(allowed, would_reuse_write_ticket, &reasons),
    })
}

fn prepare_write_dry_run_summary(
    allowed: bool,
    would_reuse_write_ticket: bool,
    reasons: &[WriteDecisionReason],
) -> DryRunSummary {
    DryRunSummary {
        planned_effects: if allowed {
            vec![PlannedEffect {
                target_kind: "write_ticket".to_owned(),
                action: if would_reuse_write_ticket {
                    "would_reuse"
                } else {
                    "would_issue"
                }
                .to_owned(),
                description: if would_reuse_write_ticket {
                    "Prepare write would reuse the compatible open write ticket."
                } else {
                    "Prepare write would issue one open write ticket."
                }
                .to_owned(),
            }]
        } else {
            Vec::new()
        },
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
