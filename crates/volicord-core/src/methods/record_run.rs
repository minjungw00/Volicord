use crate::error_boundary::{
    artifact::artifact_policy_plan_error, close_readiness::close_readiness_plan_error,
    store::plan_error_response, user_action::user_action_service_plan_error,
};
use crate::json_object::object_from_value;
use crate::method_execution::{mutation_method_policy, prepare_or_response, PlanError};
use crate::method_rejection::{
    baseline_stale_response, decision_rejected_response, dry_run_summary,
    infallible_rejected_pipeline_response, no_active_change_unit_response, no_active_task_response,
    rejected_pipeline_response, validation_rejected, workspace_stale_response,
};
use crate::pipeline::{
    commit_mutation_branch, dry_run_preview_branch, tool_error, CommitMutationBranch, CoreResult,
    CoreService, InvocationContext, PipelineResponse, TaskRequirement,
};
use crate::recording::{plan_record_run, RecordingError, RecordingRejection};
use crate::workflow_diagnostics::{
    first_product_write_duration_micros, record_core_workflow_metric_best_effort,
    response_committed_fresh_effect,
};
use volicord_store::diagnostics::WorkflowMetricKind;
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::methods::{MethodOperationCategory, RecordRunRequest};
use volicord_types::schema::ToolEnvelope;
use volicord_types::values::{ErrorCode, MethodName};

fn record_run_error_response(
    request: &RecordRunRequest,
    project_state: &volicord_store::core_pipeline::ProjectStateHeader,
    error: RecordingError,
) -> CoreResult<PipelineResponse> {
    match error {
        RecordingError::Core(error) => {
            plan_error_response(&request.envelope, project_state, PlanError::Core(error))
        }
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
            record_run_rejection_response(request, project_state, rejection)
        }
    }
}

fn record_run_rejection_response(
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
        RecordingRejection::NoActiveChangeUnit { message } => Ok(no_active_change_unit_response(
            &request.envelope,
            state_version,
            message,
        )),
        RecordingRejection::BaselineStale => Ok(baseline_stale_response(
            &request.envelope,
            state_version,
            &request.baseline_ref,
        )),
        RecordingRejection::WorkspaceStale => {
            Ok(workspace_stale_response(&request.envelope, state_version))
        }
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
        RecordingRejection::DecisionRejected { message } => Ok(decision_rejected_response(
            &request.envelope,
            state_version,
            message,
        )),
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
        "missing",
        "product-file write observations require a compatible active write ticket",
    )
}

fn write_ticket_invalid_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    reason: &'static str,
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
    reason: &'static str,
    message: &'static str,
) -> PipelineResponse {
    let details = object_from_value(serde_json::json!({
        "write_ticket_reason": reason
    }))
    .expect("fixed write ticket error details should be an object");
    infallible_rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(code, message, false, Some(details))],
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
            request.clone(),
            &prepared.context.verified_invocation,
            &prepared.operation_now,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                return record_run_error_response(&request, &prepared.context.project_state, error)
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
        let response = self.execute_prepared_request(
            prepared,
            commit_mutation_branch::<RecordRunRequest>(CommitMutationBranch {
                result_fields: plan.result_fields,
                event_kind: "run_recorded".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            }),
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
