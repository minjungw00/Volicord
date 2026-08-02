use std::collections::BTreeSet;

use serde_json::json;
use volicord_store::core_pipeline::{
    CoreProjectStore, CoreStorageMutation, ProjectStateHeader, TaskMutation,
};
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::methods::{
    AdvanceTaskRequest, AdvanceTaskResultFields, MethodOperationCategory,
};
use volicord_types::schema::StateRecordRef;
use volicord_types::values::{
    MethodName, ShapingCheckpointReadiness, ShapingGapStatus, StateRecordKind, TaskLifecyclePhase,
    TaskMode, WorkPhase,
};

use crate::acceptance_facts::active_acceptance_criteria;
use crate::error_boundary::store::plan_error_response;
use crate::json_object::object_from_value;
use crate::method_execution::{mutation_method_policy, prepare_or_response, PlanError};
use crate::method_rejection::{dry_run_summary, no_active_task_response, validation_rejected};
use crate::operation_plan::OperationPlan;
use crate::pipeline::{
    commit_mutation_branch, dry_run_preview_branch, CorePipelineError, CoreResult, CoreService,
    InvocationContext, PipelineResponse, TaskRequirement,
};
use crate::policy::workflow::project_workflow_policy;
use crate::record_refs::state_ref;
use crate::state_summary::{state_summary, StateSummaryInput};

impl CoreService {
    /// Executes the explicit `work/shaping -> work/implementation` transition.
    pub fn advance_task(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        request: AdvanceTaskRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        if request
            .envelope
            .task_id
            .as_ref()
            .is_some_and(|id| id != &request.task_id)
        {
            return validation_rejected(
                request.envelope.dry_run,
                None,
                "task_id",
                "envelope.task_id must match AdvanceTaskRequest.task_id",
            );
        }
        let request_json = serde_json::to_value(&request)?;
        let prepared = match prepare_or_response(
            self,
            Some(context),
            MethodName::AdvanceTask,
            request.envelope.clone(),
            request_json,
            invocation,
            mutation_method_policy(
                MethodName::AdvanceTask,
                request.operation_category(),
                TaskRequirement::Exact(request.task_id.clone()),
                request.envelope.dry_run,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan = match plan_advance_task(
            &prepared.store,
            &prepared.context.project_state,
            request.clone(),
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
                dry_run_preview_branch::<AdvanceTaskRequest>(dry_run_summary(
                    "task_transition",
                    "commit",
                    "Task would advance from shaping to implementation.",
                    Vec::new(),
                )),
            );
        }
        self.execute_prepared_request(
            prepared,
            commit_mutation_branch::<AdvanceTaskRequest>(
                plan.operation.into_commit_branch::<AdvanceTaskRequest>(
                    plan.result_fields,
                    "task_advanced_to_implementation",
                ),
            ),
        )
    }
}

struct AdvanceTaskPlan {
    operation: OperationPlan,
    result_fields: AdvanceTaskResultFields,
}

fn plan_advance_task(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: AdvanceTaskRequest,
) -> Result<AdvanceTaskPlan, PlanError> {
    let task = store
        .task_record(&request.task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    if task.mode != TaskMode::Work || task.work_phase != WorkPhase::Shaping {
        return advance_validation(
            &request,
            project_state,
            "task_id",
            "advance_task supports only a work Task in shaping",
        );
    }
    if task.scope_revision != request.scope_revision {
        return advance_validation(
            &request,
            project_state,
            "scope_revision",
            "scope_revision is stale",
        );
    }
    if task.shaping.baseline_ref.as_ref() != Some(&request.baseline_ref) {
        return advance_validation(
            &request,
            project_state,
            "baseline_ref",
            "baseline_ref does not match the current Task",
        );
    }
    let checkpoint = store
        .current_shaping_checkpoint(&request.task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(
                validation_rejected(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "shaping_checkpoint_id",
                    "a current shaping checkpoint is required",
                )
                .expect("validation response serializes"),
            ))
        })?;
    if checkpoint.shaping_checkpoint_id != request.shaping_checkpoint_id.as_str()
        || checkpoint.readiness != ShapingCheckpointReadiness::Ready
        || checkpoint.scope_revision != request.scope_revision
        || checkpoint.baseline_ref.as_ref() != Some(&request.baseline_ref)
        || checkpoint
            .gaps
            .iter()
            .any(|gap| gap.status != ShapingGapStatus::Applied)
    {
        return advance_validation(
            &request,
            project_state,
            "shaping_checkpoint_id",
            "checkpoint is stale, blocked, or has unapplied gaps",
        );
    }
    let change_unit = store
        .current_change_unit(&request.task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(
                validation_rejected(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "change_unit_id",
                    "a current active Change Unit is required",
                )
                .expect("validation response serializes"),
            ))
        })?;
    if change_unit.change_unit_id != request.change_unit_id.as_str()
        || change_unit.write_basis.baseline_ref.as_ref() != Some(&request.baseline_ref)
    {
        return advance_validation(
            &request,
            project_state,
            "change_unit_id",
            "Change Unit is stale or baseline-incompatible",
        );
    }
    if change_unit.lifecycle.recovery_required {
        return advance_validation(
            &request,
            project_state,
            "change_unit_id",
            "a current recovery constraint blocks implementation",
        );
    }
    let expected_resolution_ids = checkpoint
        .gaps
        .iter()
        .filter_map(|gap| gap.user_action.as_ref())
        .filter_map(|link| link.user_action_resolution_id.clone())
        .collect::<BTreeSet<_>>();
    let supplied_resolution_ids = request
        .user_action_resolution_ids
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    if supplied_resolution_ids != expected_resolution_ids {
        return advance_validation(
            &request,
            project_state,
            "user_action_resolution_ids",
            "the exact current checkpoint resolution set is required",
        );
    }
    let planned_state_version = project_state.state_version + 1;
    let resolution_refs = request
        .user_action_resolution_ids
        .iter()
        .map(|id| {
            state_ref(
                StateRecordKind::UserActionResolution,
                id.as_str(),
                &request.envelope.project_id,
                Some(&request.task_id),
                Some(planned_state_version),
            )
        })
        .collect::<Vec<StateRecordRef>>();
    let mut projected_task = task.clone();
    projected_task.work_phase = WorkPhase::Implementation;
    projected_task.lifecycle_phase = TaskLifecyclePhase::Executing;
    let workflow = crate::workflow_projection::workflow_projection(
        &request.envelope.project_id,
        planned_state_version,
        &projected_task,
        Some(&change_unit),
        Some(&checkpoint),
    );
    let state = state_summary(StateSummaryInput {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &projected_task,
        current_change_unit: Some(&change_unit),
        shaping_checkpoint: Some(&checkpoint),
        project_policy: project_workflow_policy(store)
            .map_err(CorePipelineError::from)?
            .summary,
        acceptance_criteria: active_acceptance_criteria(store, &request.task_id)?,
        pending_user_action_refs: Vec::new(),
        blocker_refs: Vec::new(),
        write_ticket_summary: None,
        evidence_summary: None,
        evidence_gate: None,
        close_state: None,
        close_blockers: Vec::new(),
        guarantee_display: None,
    })?;
    let task_ref = state_ref(
        StateRecordKind::Task,
        request.task_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let checkpoint_ref = state_ref(
        StateRecordKind::ShapingCheckpoint,
        request.shaping_checkpoint_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let change_unit_ref = state_ref(
        StateRecordKind::ChangeUnit,
        request.change_unit_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(change_unit.basis_state_version),
    );
    Ok(AdvanceTaskPlan {
        operation: OperationPlan::new(
            request.task_id.clone(),
            Some(request.change_unit_id.clone()),
            vec![CoreStorageMutation::Task(
                TaskMutation::AdvanceToImplementation {
                    task_id: request.task_id.as_str().to_owned(),
                },
            )],
            object_from_value(json!({
                "task_id": request.task_id,
                "shaping_checkpoint_id": request.shaping_checkpoint_id,
                "change_unit_id": request.change_unit_id,
                "target_work_phase": "implementation",
            }))?,
        ),
        result_fields: AdvanceTaskResultFields {
            task_ref,
            shaping_checkpoint_ref: checkpoint_ref,
            change_unit_ref,
            user_action_resolution_refs: resolution_refs,
            workflow,
            state,
        },
    })
}

fn advance_validation<T>(
    request: &AdvanceTaskRequest,
    project_state: &ProjectStateHeader,
    field: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    Err(PlanError::Response(Box::new(
        validation_rejected(
            request.envelope.dry_run,
            Some(project_state.state_version),
            field,
            message,
        )
        .map_err(PlanError::Core)?,
    )))
}
