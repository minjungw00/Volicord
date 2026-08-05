use crate::error_boundary::{
    artifact::artifact_policy_plan_error,
    close_readiness::close_readiness_plan_error,
    store::{plan_error_response, store_error_plan},
    user_action::user_action_service_plan_error,
};
use crate::json_object::object_from_value;
use crate::method_execution::{mutation_method_policy, prepare_or_response, PlanError};
use crate::method_rejection::{
    dry_run_summary, infallible_rejected_pipeline_response, no_active_task_response,
    rejected_pipeline_response, validation_rejected, workflow_rejected_response,
};
use crate::operation_plan::OperationPlan;
use crate::pipeline::{
    commit_mutation_branch, dry_run_preview_branch, tool_error, CoreResult, CoreService,
    InvocationContext, PipelineResponse, TaskRequirement,
};
use crate::recording::{
    plan_record_run, RecordRunInput, RecordRunResultFacts, RecordingError, RecordingRejection,
};
use crate::workflow_diagnostics::{
    first_product_write_duration_micros, record_core_workflow_metric_best_effort,
    response_committed_fresh_effect,
};
use crate::write_ticket::WriteTicketInvalidReason;
use volicord_store::diagnostics::WorkflowMetricKind;
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::methods::{MethodOperationCategory, RecordRunRequest, RecordRunResultFields};
use volicord_types::schema::{RunSummary, ToolEnvelope};
use volicord_types::values::{ErrorCode, MethodName, RunKind, TaskMode, WorkPhase};

fn record_run_error_response(
    store: &volicord_store::core_pipeline::CoreProjectStore,
    request: &RecordRunRequest,
    project_state: &volicord_store::core_pipeline::ProjectStateHeader,
    error: RecordingError,
) -> CoreResult<PipelineResponse> {
    match error {
        RecordingError::Core(error) => {
            plan_error_response(&request.envelope, project_state, PlanError::Core(error))
        }
        RecordingError::Store(error) => plan_error_response(
            &request.envelope,
            project_state,
            store_error_plan(&request.envelope, project_state, error),
        ),
        RecordingError::UserAction(error) => plan_error_response(
            &request.envelope,
            project_state,
            user_action_service_plan_error(&request.envelope, project_state, error),
        ),
        RecordingError::Artifact(error) => plan_error_response(
            &request.envelope,
            project_state,
            artifact_policy_plan_error(&request.envelope, project_state, error),
        ),
        RecordingError::CloseReadiness(error) => plan_error_response(
            &request.envelope,
            project_state,
            close_readiness_plan_error(&request.envelope, project_state, error),
        ),
        RecordingError::Rejected(rejection) => {
            record_run_rejection_response(store, request, project_state, rejection)
        }
    }
}

fn record_run_rejection_response(
    store: &volicord_store::core_pipeline::CoreProjectStore,
    request: &RecordRunRequest,
    project_state: &volicord_store::core_pipeline::ProjectStateHeader,
    rejection: RecordingRejection,
) -> CoreResult<PipelineResponse> {
    let state_version = Some(project_state.state_version);
    match rejection {
        RecordingRejection::Validation { field, message } => {
            validation_rejected(request.envelope.dry_run, state_version, field, message)
        }
        RecordingRejection::NoActiveTask => {
            Ok(no_active_task_response(&request.envelope, project_state))
        }
        RecordingRejection::RunKindIncompatible => {
            let task = store.task_record(&request.task_id)?.ok_or_else(|| {
                crate::pipeline::CorePipelineError::Invariant {
                    detail: "run-kind rejection requires an existing Task".to_owned(),
                }
            })?;
            let allowed_run_kinds = match (task.mode, task.work_phase) {
                (TaskMode::Direct, _) => vec![RunKind::Direct],
                (TaskMode::Work, WorkPhase::Implementation) => vec![RunKind::Implementation],
                _ => Vec::new(),
            };
            workflow_rejected_response(
                store,
                project_state,
                &request.envelope,
                &request.task_id,
                ErrorCode::RunKindIncompatible,
                "kind is not compatible with the current Task mode and work phase",
                MethodName::RecordRun,
                Some(request.kind),
                allowed_run_kinds,
                true,
            )
        }
        RecordingRejection::TaskPhaseTransitionRequired => workflow_rejected_response(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::TaskPhaseTransitionRequired,
            "record_run requires the Task to enter implementation",
            MethodName::RecordRun,
            Some(request.kind),
            vec![RunKind::Implementation],
            true,
        ),
        RecordingRejection::ChangeUnitRequired => workflow_rejected_response(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::ChangeUnitRequired,
            "record_run requires a current Change Unit",
            MethodName::RecordRun,
            Some(request.kind),
            Vec::new(),
            true,
        ),
        RecordingRejection::ChangeUnitStale | RecordingRejection::BaselineStale => {
            workflow_rejected_response(
                store,
                project_state,
                &request.envelope,
                &request.task_id,
                ErrorCode::ChangeUnitStale,
                "change_unit_id or baseline_ref does not match the current Change Unit",
                MethodName::RecordRun,
                Some(request.kind),
                Vec::new(),
                true,
            )
        }
        RecordingRejection::WorkspaceStale => workflow_rejected_response(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::WorkspaceBasisStale,
            "current Git workspace context does not match the current Change Unit basis",
            MethodName::RecordRun,
            Some(request.kind),
            Vec::new(),
            true,
        ),
        RecordingRejection::ProductPathContainment { message } => rejected_pipeline_response(
            request.envelope.dry_run,
            state_version,
            vec![tool_error(
                ErrorCode::InvocationContextMismatch,
                message,
                false,
                None,
            )],
        ),
        RecordingRejection::DecisionRejected { message } => workflow_rejected_response(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::UserDecisionUnresolved,
            message,
            MethodName::RecordRun,
            Some(request.kind),
            Vec::new(),
            true,
        ),
        RecordingRejection::WriteTicketRequired => Ok(write_ticket_required_response(
            &request.envelope,
            state_version,
        )),
        RecordingRejection::WriteTicketInvalid { reason, message } => Ok(
            write_ticket_invalid_response(&request.envelope, state_version, reason, message),
        ),
        RecordingRejection::EvidenceInsufficient { message } => rejected_pipeline_response(
            request.envelope.dry_run,
            state_version,
            vec![tool_error(
                ErrorCode::EvidenceInsufficient,
                message,
                false,
                None,
            )],
        ),
        RecordingRejection::ArtifactInput {
            artifact_input_id,
            reason,
            message,
        } => {
            let details = object_from_value(serde_json::json!({
                "artifact_input_error": {
                    "artifact_input_id": artifact_input_id,
                    "reason": reason
                }
            }))?;
            Ok(infallible_rejected_pipeline_response(
                request.envelope.dry_run,
                state_version,
                vec![tool_error(
                    ErrorCode::ValidationFailed,
                    message,
                    false,
                    Some(details),
                )],
            ))
        }
        RecordingRejection::ArtifactMissing { message } => {
            Ok(infallible_rejected_pipeline_response(
                request.envelope.dry_run,
                state_version,
                vec![tool_error(ErrorCode::ArtifactMissing, message, false, None)],
            ))
        }
    }
}

fn write_ticket_required_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
) -> PipelineResponse {
    write_ticket_invalid_or_required_response(
        envelope,
        state_version,
        ErrorCode::WriteTicketRequired,
        WriteTicketInvalidReason::Missing,
        "product-file write observations require a compatible active write ticket",
    )
}

fn write_ticket_invalid_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    reason: WriteTicketInvalidReason,
    message: &'static str,
) -> PipelineResponse {
    write_ticket_invalid_or_required_response(
        envelope,
        state_version,
        ErrorCode::WriteTicketInvalid,
        reason,
        message,
    )
}

fn write_ticket_invalid_or_required_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    code: ErrorCode,
    reason: WriteTicketInvalidReason,
    message: &'static str,
) -> PipelineResponse {
    let details = [(
        "write_ticket_reason".to_owned(),
        serde_json::Value::String(reason.as_str().to_owned()),
    )]
    .into_iter()
    .collect();
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(code, message, false, Some(details))],
    )
}

fn record_run_result_fields(facts: &RecordRunResultFacts) -> RecordRunResultFields {
    RecordRunResultFields {
        run_summary: RunSummary {
            run_ref: facts.run_ref().clone(),
            kind: facts.kind(),
            summary: facts.summary().to_owned(),
            observed_changes: facts.observed_changes().clone(),
            artifact_refs: facts.registered_artifacts().to_vec(),
        },
        registered_artifacts: facts.registered_artifacts().to_vec(),
        evidence_summary: facts.evidence_summary().cloned(),
        evidence_observations: facts.evidence_observations().to_vec(),
        evidence_producers: facts.evidence_producers().to_vec(),
        current_close_basis: facts.current_close_basis().cloned(),
        blocker_refs: facts.blocker_refs().to_vec(),
        state: facts.state().clone(),
    }
}

fn recording_input(request: &RecordRunRequest) -> RecordRunInput {
    RecordRunInput::new(
        request.envelope.project_id.clone(),
        request.envelope.dry_run,
        request.task_id.clone(),
        request.change_unit_id.clone(),
        request.kind,
        request.run_id.as_ref().cloned(),
        request.baseline_ref.clone(),
        request.write_ticket_id.as_ref().cloned(),
        request.performed_operation.as_ref().cloned(),
        request.summary.clone(),
        request.observed_changes.clone(),
        request.artifact_inputs.clone(),
        request.evidence_updates.clone(),
        request.evidence_observations.clone(),
        request.close_assessment.as_ref().cloned(),
    )
}

impl CoreService {
    /// Executes `volicord.record_run` through the shared Core mutation pipeline.
    pub fn record_run(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        request: RecordRunRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = request.envelope.task_id.as_ref() {
            if envelope_task_id != &request.task_id {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match RecordRunRequest.task_id",
                );
            }
        }
        let prepared = match prepare_or_response(
            self,
            Some(context),
            MethodName::RecordRun,
            Some(volicord_types::values::WorkflowActionSemanticVariant::RecordRun),
            request.envelope.clone(),
            request_json,
            invocation,
            mutation_method_policy(
                MethodName::RecordRun,
                request.operation_category(),
                TaskRequirement::Exact(request.task_id.clone()),
                request.envelope.dry_run,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let first_product_write_duration = request
            .observed_changes
            .product_file_write_observed
            .then(|| {
                let prior_runs = prepared
                    .store
                    .run_observed_changes_for_task(&request.task_id)
                    .ok()?;
                if prior_runs
                    .iter()
                    .any(|run| run.observed_changes.product_file_write_observed)
                {
                    return None;
                }
                first_product_write_duration_micros(
                    &prepared.store,
                    &request.task_id,
                    &prepared.operation_now,
                )
            })
            .flatten();
        let plan = match plan_record_run(
            self,
            &prepared.store,
            &prepared.context.project_state,
            recording_input(&request),
            &prepared.context.verified_invocation,
            &prepared.operation_now,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                let response = record_run_error_response(
                    &prepared.store,
                    &request,
                    &prepared.context.project_state,
                    error,
                )?;
                return Ok(response.with_prepared_context(&prepared));
            }
        };

        if request.envelope.dry_run.is_requested() {
            return self.execute_prepared_request(
                prepared,
                dry_run_preview_branch::<RecordRunRequest>(dry_run_summary(
                        "run",
                        "would_record",
                        "Record run would create one Run and any compatible evidence or artifact links.",
                        Vec::new(),
                    )),
            );
        }

        let session_id = prepared.context.verified_invocation.session_id.clone();
        let (effect, result_facts) = plan.into_parts();
        let task_id = effect.task_id().clone();
        let change_unit_id = effect.change_unit_id().clone();
        let event_payload = effect.event_payload().clone();
        let operation = OperationPlan::new(
            task_id,
            Some(change_unit_id),
            effect.into_storage_mutations(),
            event_payload,
        );
        let response = self.execute_prepared_request(
            prepared,
            commit_mutation_branch::<RecordRunRequest>(
                operation.into_commit_branch::<RecordRunRequest>(
                    record_run_result_fields(&result_facts),
                    "run_recorded",
                ),
            ),
        )?;
        if response_committed_fresh_effect(&response) {
            if let Some(duration) = first_product_write_duration {
                record_core_workflow_metric_best_effort(
                    context,
                    session_id.as_deref(),
                    WorkflowMetricKind::FirstProductWriteDurationMicros,
                    duration,
                );
            }
        }
        Ok(response)
    }
}
