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
    ErrorCode, MethodName, ShapingCheckpointReadiness, ShapingGapStatus, StateRecordKind,
    TaskLifecyclePhase, TaskMode, WorkPhase,
};

use crate::acceptance_facts::active_acceptance_criteria;
use crate::error_boundary::store::plan_error_response;
use crate::json_object::object_from_value;
use crate::method_execution::{mutation_method_policy, prepare_or_response, PlanError};
use crate::method_rejection::{
    dry_run_summary, no_active_task_response, validation_rejected, workflow_rejection_plan_error,
};
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
                let response =
                    plan_error_response(&request.envelope, &prepared.context.project_state, error)?;
                return Ok(response.with_prepared_context(&prepared));
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
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::WorkflowActionNotAllowed,
            "advance_task is not allowed for the current Task mode and work phase",
            MethodName::AdvanceTask,
            None,
            Vec::new(),
            false,
            MethodName::Status,
        );
    }
    if task.scope_revision != request.scope_revision {
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::ShapingCheckpointStale,
            "scope_revision does not match the current shaping checkpoint basis",
            MethodName::AdvanceTask,
            None,
            Vec::new(),
            true,
            MethodName::Status,
        );
    }
    if task.shaping.baseline_ref.as_ref() != Some(&request.baseline_ref) {
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::ShapingCheckpointStale,
            "baseline_ref does not match the current shaping checkpoint basis",
            MethodName::AdvanceTask,
            None,
            Vec::new(),
            true,
            MethodName::Status,
        );
    }
    let checkpoint = match store
        .current_shaping_checkpoint(&request.task_id)
        .map_err(CorePipelineError::from)?
    {
        Some(checkpoint) => checkpoint,
        None => {
            return workflow_rejection_plan_error(
                store,
                project_state,
                &request.envelope,
                &request.task_id,
                ErrorCode::ShapingCheckpointRequired,
                "a current shaping checkpoint is required before implementation",
                MethodName::AdvanceTask,
                None,
                Vec::new(),
                true,
                MethodName::RecordShaping,
            )
        }
    };
    if checkpoint
        .gaps
        .iter()
        .any(|gap| gap.status == ShapingGapStatus::Current && gap.user_action.is_some())
    {
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::UserDecisionUnresolved,
            "the current shaping checkpoint has an unresolved User Channel decision",
            MethodName::AdvanceTask,
            None,
            Vec::new(),
            true,
            MethodName::ResolveUserAction,
        );
    }
    if checkpoint.shaping_checkpoint_id != request.shaping_checkpoint_id.as_str()
        || checkpoint.readiness != ShapingCheckpointReadiness::Ready
        || checkpoint.scope_revision != request.scope_revision
        || checkpoint.baseline_ref.as_ref() != Some(&request.baseline_ref)
        || checkpoint
            .gaps
            .iter()
            .any(|gap| gap.status != ShapingGapStatus::Applied)
    {
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::ShapingCheckpointStale,
            "shaping checkpoint is stale, blocked, or has unapplied gaps",
            MethodName::AdvanceTask,
            None,
            Vec::new(),
            true,
            MethodName::Status,
        );
    }
    let change_unit = match store
        .current_change_unit(&request.task_id)
        .map_err(CorePipelineError::from)?
    {
        Some(change_unit) => change_unit,
        None => {
            return workflow_rejection_plan_error(
                store,
                project_state,
                &request.envelope,
                &request.task_id,
                ErrorCode::ChangeUnitRequired,
                "a current Change Unit is required before implementation",
                MethodName::AdvanceTask,
                None,
                Vec::new(),
                true,
                MethodName::UpdateScope,
            )
        }
    };
    if change_unit.change_unit_id != request.change_unit_id.as_str()
        || change_unit.write_basis.baseline_ref.as_ref() != Some(&request.baseline_ref)
        || change_unit.lifecycle.recovery_required
    {
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::ChangeUnitStale,
            "Change Unit is stale, baseline-incompatible, or recovery-constrained",
            MethodName::AdvanceTask,
            None,
            Vec::new(),
            true,
            MethodName::Status,
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
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::UserDecisionUnresolved,
            "the exact current User Channel resolution set is required",
            MethodName::AdvanceTask,
            None,
            Vec::new(),
            true,
            MethodName::Status,
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
