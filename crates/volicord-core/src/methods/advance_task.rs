use std::collections::BTreeSet;

use serde_json::json;
use volicord_store::core_pipeline::{
    CoreProjectStore, CoreStorageMutation, ProjectStateHeader, ShapingAdvanceApplication,
    ShapingCheckpointMutation, ShapingGapApplication,
};
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::shaping_decision_application_id;
use volicord_types::methods::{
    AdvanceTaskRequest, AdvanceTaskResultFields, MethodOperationCategory,
};
use volicord_types::schema::{PersistedUserActionRequestMetadata, StateRecordRef};
use volicord_types::values::{
    ErrorCode, MethodName, ShapingCheckpointReadiness, ShapingDecisionApplicationOwner,
    ShapingGapStatus, StateRecordKind, TaskLifecyclePhase, TaskMode, UserActionBasisStatus,
    UserActionStatus, WorkPhase,
};
use volicord_user_action_service::{
    accepted_current_user_authority, user_action_authority_from_record,
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
            &prepared.operation_now,
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
    operation_now: &volicord_types::values::UtcTimestamp,
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
    let current_change_unit = store
        .current_change_unit(&request.task_id)
        .map_err(CorePipelineError::from)?;
    let current_checkpoint = store
        .current_shaping_checkpoint(&request.task_id)
        .map_err(CorePipelineError::from)?;
    let task_wide_authority = crate::workflow_projection::task_wide_shaping_authority(
        store,
        &request.envelope.project_id,
        project_state.state_version,
        &task,
        current_change_unit.as_ref(),
        current_checkpoint.as_ref(),
        operation_now,
    )?;
    if task_wide_authority.blocks_advance_application() {
        let recovery_owner = if !task_wide_authority.recovery_required.is_empty() {
            MethodName::RecordShapingCheckpoint
        } else if !task_wide_authority.awaiting_user.is_empty() {
            MethodName::ResolveUserAction
        } else {
            MethodName::Status
        };
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::UserDecisionUnresolved,
            "task-wide UserAction authority required for advance_task is not accepted",
            MethodName::AdvanceTask,
            None,
            Vec::new(),
            false,
            recovery_owner,
        );
    }
    let checkpoint = match current_checkpoint {
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
                MethodName::RecordShapingCheckpoint,
            )
        }
    };
    if checkpoint
        .gaps
        .iter()
        .any(|gap| gap.status == ShapingGapStatus::Current && gap.gap_kind.is_user_owned())
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
        || checkpoint.gaps.iter().any(|gap| {
            gap.gap_kind.decision_policy().is_some_and(|policy| {
                policy.application_owner == ShapingDecisionApplicationOwner::UpdateScope
                    && gap.status != ShapingGapStatus::Applied
            }) || (!gap.gap_kind.is_user_owned() && gap.status == ShapingGapStatus::Current)
        })
    {
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::ShapingCheckpointStale,
            "shaping checkpoint is stale, structurally blocked, or has unapplied scope decisions",
            MethodName::AdvanceTask,
            None,
            Vec::new(),
            true,
            MethodName::Status,
        );
    }
    let change_unit = match current_change_unit {
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
    let mut applications = Vec::new();
    let mut expected_resolution_ids = BTreeSet::new();
    for gap in checkpoint.gaps.iter().filter(|gap| {
        gap.gap_kind.decision_policy().is_some_and(|policy| {
            policy.application_owner == ShapingDecisionApplicationOwner::AdvanceTask
        })
    }) {
        if gap.status != ShapingGapStatus::Accepted {
            return workflow_rejection_plan_error(
                store,
                project_state,
                &request.envelope,
                &request.task_id,
                ErrorCode::UserDecisionUnresolved,
                "every advance-owned shaping gap must be accepted and supplied exactly once",
                MethodName::AdvanceTask,
                None,
                Vec::new(),
                true,
                MethodName::Status,
            );
        }
        let Some(link) = gap.user_action.as_ref() else {
            return Err(CorePipelineError::Invariant {
                detail: "an advance-owned shaping gap has no UserAction link".to_owned(),
            }
            .into());
        };
        let Some(resolution_id) = link.user_action_resolution_id.as_ref() else {
            return workflow_rejection_plan_error(
                store,
                project_state,
                &request.envelope,
                &request.task_id,
                ErrorCode::UserDecisionUnresolved,
                "an advance-owned shaping gap has no exact resolution",
                MethodName::AdvanceTask,
                None,
                Vec::new(),
                true,
                MethodName::ResolveUserAction,
            );
        };
        let record = store
            .user_action_record(&link.user_action_request_id, operation_now)
            .map_err(CorePipelineError::from)?
            .ok_or_else(|| CorePipelineError::Invariant {
                detail: "an advance-owned shaping gap references a missing UserAction request"
                    .to_owned(),
            })?;
        let policy =
            gap.gap_kind
                .decision_policy()
                .ok_or_else(|| CorePipelineError::Invariant {
                    detail: "advance application policy is missing".to_owned(),
                })?;
        let metadata_matches = matches!(
            record.request().metadata(),
            PersistedUserActionRequestMetadata::Shaping(metadata)
                if metadata.shaping_checkpoint_id.as_str() == checkpoint.shaping_checkpoint_id
                    && metadata.shaping_gap_id.as_str() == gap.shaping_gap_id
        );
        let coordinates = record.request().basis().coordinates();
        let authority = user_action_authority_from_record(&record).map_err(|error| {
            CorePipelineError::Invariant {
                detail: format!("advance-owned UserAction authority is invalid: {error}"),
            }
        })?;
        let exact_basis = record.status() == UserActionStatus::Resolved
            && record.request().basis_status() == UserActionBasisStatus::Current
            && coordinates.compatibility_status == UserActionBasisStatus::Current
            && coordinates.task_id == request.task_id
            && coordinates.scope_revision == request.scope_revision
            && coordinates.change_unit_id.as_ref() == Some(&request.change_unit_id)
            && coordinates.baseline_ref.as_ref() == Some(&request.baseline_ref)
            && record.request().required_for() == policy.required_for
            && record
                .resolution()
                .is_some_and(|resolution| resolution.user_action_resolution_id() == resolution_id)
            && metadata_matches
            && accepted_current_user_authority(&authority, policy.user_action_kind);
        if !exact_basis {
            return workflow_rejection_plan_error(
                store,
                project_state,
                &request.envelope,
                &request.task_id,
                ErrorCode::UserDecisionUnresolved,
                "an advance-owned resolution does not match the exact current checkpoint basis",
                MethodName::AdvanceTask,
                None,
                Vec::new(),
                false,
                MethodName::Status,
            );
        }
        expected_resolution_ids.insert(resolution_id.clone());
        applications.push(ShapingGapApplication {
            shaping_decision_application_id: shaping_decision_application_id(
                &volicord_types::ids::UserActionResolutionId::new(resolution_id),
                ShapingDecisionApplicationOwner::AdvanceTask,
            )
            .map_err(CorePipelineError::from)?
            .into_inner(),
            shaping_gap_id: gap.shaping_gap_id.clone(),
            user_action_resolution_id: resolution_id.clone(),
        });
    }
    let supplied_resolution_ids = request
        .user_action_resolution_ids
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    if supplied_resolution_ids != expected_resolution_ids
        || supplied_resolution_ids.len() != request.user_action_resolution_ids.len()
    {
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
    let resolution_refs = applications
        .iter()
        .map(|application| {
            state_ref(
                StateRecordKind::UserActionResolution,
                &application.user_action_resolution_id,
                &request.envelope.project_id,
                Some(&request.task_id),
                Some(planned_state_version),
            )
        })
        .collect::<Vec<StateRecordRef>>();
    let gap_refs = applications
        .iter()
        .map(|application| {
            state_ref(
                StateRecordKind::ShapingGap,
                &application.shaping_gap_id,
                &request.envelope.project_id,
                Some(&request.task_id),
                Some(planned_state_version),
            )
        })
        .collect::<Vec<_>>();
    let mut projected_checkpoint = checkpoint.clone();
    crate::workflow_projection::apply_projected_shaping_applications(
        &mut projected_checkpoint,
        &applications,
        ShapingDecisionApplicationOwner::AdvanceTask,
        request.scope_revision,
        &request.baseline_ref,
        Some(&request.change_unit_id),
        operation_now,
    )?;
    let mut projected_task = task.clone();
    projected_task.work_phase = WorkPhase::Implementation;
    projected_task.lifecycle_phase = TaskLifecyclePhase::Executing;
    let task_wide_authority = crate::workflow_projection::task_wide_shaping_authority(
        store,
        &request.envelope.project_id,
        planned_state_version,
        &projected_task,
        Some(&change_unit),
        Some(&projected_checkpoint),
        operation_now,
    )?;
    let workflow = crate::workflow_projection::workflow_projection(
        &request.envelope.project_id,
        planned_state_version,
        &projected_task,
        Some(&change_unit),
        Some(&projected_checkpoint),
        &task_wide_authority,
    );
    let state = state_summary(StateSummaryInput {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &projected_task,
        current_change_unit: Some(&change_unit),
        shaping_checkpoint: Some(&projected_checkpoint),
        task_wide_shaping_authority: &task_wide_authority,
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
    let application_refs = applications
        .iter()
        .map(|application| {
            state_ref(
                StateRecordKind::ShapingDecisionApplication,
                &application.shaping_decision_application_id,
                &request.envelope.project_id,
                Some(&request.task_id),
                Some(planned_state_version),
            )
        })
        .collect::<Vec<_>>();
    Ok(AdvanceTaskPlan {
        operation: OperationPlan::new(
            request.task_id.clone(),
            Some(request.change_unit_id.clone()),
            vec![CoreStorageMutation::Shaping(
                ShapingCheckpointMutation::ApplyAdvanceAndTransition(ShapingAdvanceApplication {
                    task_id: request.task_id.as_str().to_owned(),
                    shaping_checkpoint_id: request.shaping_checkpoint_id.as_str().to_owned(),
                    change_unit_id: request.change_unit_id.as_str().to_owned(),
                    scope_revision: request.scope_revision,
                    baseline_ref: request.baseline_ref.clone(),
                    applications: applications.clone(),
                }),
            )],
            object_from_value(json!({
                "task_id": request.task_id,
                "shaping_checkpoint_id": request.shaping_checkpoint_id,
                "change_unit_id": request.change_unit_id,
                "target_work_phase": "implementation",
                "applied_shaping_gap_ids": applications
                    .iter()
                    .map(|application| application.shaping_gap_id.clone())
                    .collect::<Vec<_>>(),
                "applied_user_action_resolution_ids": applications
                    .iter()
                    .map(|application| application.user_action_resolution_id.clone())
                    .collect::<Vec<_>>(),
                "applied_shaping_decision_application_ids": applications
                    .iter()
                    .map(|application| application.shaping_decision_application_id.clone())
                    .collect::<Vec<_>>(),
                "applied_shaping_decision_application_refs": application_refs.clone(),
            }))?,
        ),
        result_fields: AdvanceTaskResultFields {
            task_ref,
            shaping_checkpoint_ref: checkpoint_ref,
            change_unit_ref,
            applied_shaping_gap_refs: gap_refs,
            applied_user_action_resolution_refs: resolution_refs,
            applied_shaping_decision_application_refs: application_refs,
            workflow,
            state,
        },
    })
}
