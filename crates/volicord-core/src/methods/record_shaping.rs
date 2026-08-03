use std::collections::BTreeSet;

use serde_json::json;
use volicord_store::core_pipeline::{
    CoreProjectStore, CoreStorageMutation, ProjectStateHeader, ShapingAdvanceApplication,
    ShapingCheckpointGapInsert, ShapingCheckpointInsert, ShapingCheckpointMutation,
    ShapingCheckpointRecord, ShapingCheckpointUserActionInsert, ShapingGapApplication,
    TaskCloseBasisUpdate, TaskMutation, TaskScopeUpdate,
};
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::methods::{
    MethodOperationCategory, RecordShapingOperation, RecordShapingRequest,
    RecordShapingResultFields,
};
use volicord_types::schema::{
    advisor_compatible_change_unit, CurrentCloseBasis, PersistedUserActionRequestMetadata,
    RequiredNullable, ResidualRisk, ShapingCheckpoint, ShapingCheckpointOperation, StateRecordRef,
    WorkflowRejectionUserAction,
};
use volicord_types::values::{
    ErrorCode, MethodName, ShapingCheckpointReadiness, ShapingDecisionApplicationOwner,
    ShapingGapStatus, StateRecordKind, TaskLifecyclePhase, TaskMode, UserActionBasisStatus,
    UserActionStatus, WorkPhase,
};
use volicord_user_action_service::{
    accepted_current_user_authority, construct_user_action, materialize_user_action_request,
    user_action_authority_from_record, UserActionConstructionContext, UserActionConstructionInput,
    UserActionIntent, UserActionMaterializationInput, UserActionOrigin,
    UserActionPersistenceContext,
};

use crate::acceptance_facts::active_acceptance_criteria;
use crate::error_boundary::{
    store::plan_error_response, user_action::user_action_service_plan_error,
};
use crate::identity::{
    allocate_risk_id, allocate_shaping_checkpoint_id, allocate_shaping_gap_id,
    allocate_user_action_request_id,
};
use crate::json_object::object_from_value;
use crate::method_execution::{mutation_method_policy, prepare_or_response, PlanError};
use crate::method_rejection::{
    dry_run_summary, no_active_task_response, validation_rejected, workflow_rejection_plan_error,
    workflow_rejection_plan_error_with_user_actions,
};
use crate::operation_plan::OperationPlan;
use crate::pipeline::{
    commit_mutation_branch, dry_run_preview_branch, CorePipelineError, CoreResult, CoreService,
    InvocationContext, PipelineResponse, TaskRequirement, VerifiedInvocationContext,
};
use crate::policy::workflow::project_workflow_policy;
use crate::state_summary::{project_state_header, state_summary, StateSummaryInput};

impl CoreService {
    /// Executes `volicord.record_shaping` as one authority-bearing aggregate mutation.
    pub fn record_shaping(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        request: RecordShapingRequest,
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
                "envelope.task_id must match RecordShapingRequest.task_id",
            );
        }
        let request_json = serde_json::to_value(&request)?;
        let prepared = match prepare_or_response(
            self,
            Some(context),
            MethodName::RecordShaping,
            request.envelope.clone(),
            request_json,
            invocation,
            mutation_method_policy(
                MethodName::RecordShaping,
                request.operation_category(),
                TaskRequirement::Exact(request.task_id.clone()),
                request.envelope.dry_run,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan = match plan_record_shaping(
            self,
            &prepared.store,
            &prepared.context.project_state,
            request.clone(),
            &prepared.context.verified_invocation,
            &prepared.operation_now,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                let response =
                    plan_error_response(&request.envelope, &prepared.context.project_state, error)?;
                return Ok(response.with_prepared_context(&prepared));
            }
        };
        let (entity_kind, description, event_kind) = match &request.operation {
            RecordShapingOperation::RecordCheckpoint { .. } => (
                "shaping_checkpoint",
                "Shaping checkpoint and linked UserAction requests would be recorded atomically.",
                "shaping_recorded",
            ),
            RecordShapingOperation::FinalizeAdvice { .. } => (
                "advisor_advice",
                "Advisor decisions, result, and checkpoint-backed close basis would be recorded atomically.",
                "advisor_advice_finalized",
            ),
        };
        if request.envelope.dry_run.is_requested() {
            return self.execute_prepared_request(
                prepared,
                dry_run_preview_branch::<RecordShapingRequest>(dry_run_summary(
                    entity_kind,
                    "commit",
                    description,
                    Vec::new(),
                )),
            );
        }
        self.execute_prepared_request(
            prepared,
            commit_mutation_branch::<RecordShapingRequest>(
                plan.operation
                    .into_commit_branch::<RecordShapingRequest>(plan.result_fields, event_kind),
            ),
        )
    }
}

struct RecordShapingPlan {
    operation: OperationPlan,
    result_fields: RecordShapingResultFields,
}

fn plan_record_shaping(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: RecordShapingRequest,
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &volicord_types::values::UtcTimestamp,
) -> Result<RecordShapingPlan, PlanError> {
    let RecordShapingOperation::RecordCheckpoint {
        checkpoint_operation,
        scope_revision,
        baseline_ref,
        summary,
        implementation_boundary,
        gaps,
        source_refs,
        evidence_refs,
    } = &request.operation
    else {
        return plan_finalize_advice(service, store, project_state, &request, operation_now);
    };
    let task = store
        .task_record(&request.task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    if !matches!(task.mode, TaskMode::Advisor | TaskMode::Work)
        || task.work_phase != WorkPhase::Shaping
    {
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::WorkflowActionNotAllowed,
            "record_shaping is not allowed for the current Task mode and work phase",
            MethodName::RecordShaping,
            None,
            Vec::new(),
            false,
            MethodName::Status,
        );
    }
    if task.scope_revision != *scope_revision {
        return shaping_validation(
            &request,
            project_state,
            "scope_revision",
            "scope_revision must equal the current Task scope revision",
        );
    }
    let current_checkpoint = store
        .current_shaping_checkpoint(&request.task_id)
        .map_err(CorePipelineError::from)?;
    let predecessor_checkpoint_id =
        match checkpoint_operation {
            ShapingCheckpointOperation::CreateInitial => {
                if current_checkpoint.is_some() {
                    return workflow_rejection_plan_error(
                        store,
                        project_state,
                        &request.envelope,
                        &request.task_id,
                        ErrorCode::ShapingCheckpointStale,
                        "create_initial requires that the Task have no current shaping checkpoint",
                        MethodName::RecordShaping,
                        None,
                        Vec::new(),
                        true,
                        MethodName::RecordShaping,
                    );
                }
                None
            }
            ShapingCheckpointOperation::ReplaceCurrent {
                expected_current_checkpoint_id,
            } => {
                let Some(current) = current_checkpoint.as_ref() else {
                    return workflow_rejection_plan_error(
                        store,
                        project_state,
                        &request.envelope,
                        &request.task_id,
                        ErrorCode::ShapingCheckpointStale,
                        "replace_current requires an exact current shaping checkpoint",
                        MethodName::RecordShaping,
                        None,
                        Vec::new(),
                        true,
                        MethodName::RecordShaping,
                    );
                };
                if current.shaping_checkpoint_id != expected_current_checkpoint_id.as_str() {
                    return workflow_rejection_plan_error(
                        store,
                        project_state,
                        &request.envelope,
                        &request.task_id,
                        ErrorCode::ShapingCheckpointStale,
                        "expected_current_checkpoint_id is not the exact current checkpoint",
                        MethodName::RecordShaping,
                        None,
                        Vec::new(),
                        true,
                        MethodName::RecordShaping,
                    );
                }
                let mut live_linked_decisions = Vec::new();
                for gap in current.gaps.iter().filter(|gap| {
                    gap.status != ShapingGapStatus::Applied && gap.user_action.is_some()
                }) {
                    let Some(link) = gap.user_action.as_ref() else {
                        continue;
                    };
                    let record = store
                        .user_action_record(&link.user_action_request_id, operation_now)
                        .map_err(CorePipelineError::from)?
                        .ok_or_else(|| CorePipelineError::Invariant {
                            detail: "a shaping gap link references a missing UserAction request"
                                .to_owned(),
                        })?;
                    if record.request().basis_status() == UserActionBasisStatus::Current {
                        live_linked_decisions.push(WorkflowRejectionUserAction {
                            user_action_request_ref: crate::record_refs::state_ref(
                                StateRecordKind::UserActionRequest,
                                &link.user_action_request_id,
                                &request.envelope.project_id,
                                Some(&request.task_id),
                                Some(project_state.state_version),
                            ),
                            effective_status: record.status(),
                            required_owner_method: match record.status() {
                                UserActionStatus::Pending | UserActionStatus::Expired => {
                                    MethodName::ResolveUserAction
                                }
                                UserActionStatus::Resolved => gap
                                    .gap_kind
                                    .decision_policy_for_mode(task.mode)
                                    .map_or(MethodName::Status, |policy| {
                                        policy.application_owner.method()
                                    }),
                                UserActionStatus::Stale | UserActionStatus::Superseded => {
                                    MethodName::Status
                                }
                            },
                        });
                    }
                }
                if !live_linked_decisions.is_empty() {
                    return workflow_rejection_plan_error_with_user_actions(
                        store,
                        project_state,
                        &request.envelope,
                        &request.task_id,
                        ErrorCode::UserDecisionUnresolved,
                        "the current shaping checkpoint has live linked UserAction authority",
                        MethodName::RecordShaping,
                        None,
                        Vec::new(),
                        false,
                        MethodName::ResolveUserAction,
                        live_linked_decisions,
                    );
                }
                Some(expected_current_checkpoint_id.clone())
            }
        };
    if summary.trim().is_empty() {
        return shaping_validation(
            &request,
            project_state,
            "summary",
            "summary must not be empty",
        );
    }
    let task_baseline = task.shaping.baseline_ref.as_ref();
    if baseline_ref.as_ref() != task_baseline {
        return shaping_validation(
            &request,
            project_state,
            "baseline_ref",
            "baseline_ref must match the current Task baseline",
        );
    }
    let current_change_unit = store
        .current_change_unit(&request.task_id)
        .map_err(CorePipelineError::from)?;
    let planned_state_version = project_state.state_version + 1;
    let checkpoint_id = allocate_shaping_checkpoint_id(service.durable_id_generator(), store)
        .map_err(PlanError::Core)?;
    let operation_identity = request
        .envelope
        .idempotency_key
        .as_ref()
        .cloned()
        .ok_or_else(|| {
            PlanError::Response(Box::new(
                validation_rejected(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "envelope.idempotency_key",
                    "record_shaping requires an idempotency key",
                )
                .expect("validation response serializes"),
            ))
        })?;

    let mut reserved_gap_ids = BTreeSet::new();
    let mut mutations = Vec::new();
    let mut gap_inserts = Vec::new();
    let mut projected_gaps = Vec::new();
    let mut created_request_refs = Vec::new();
    for gap in gaps {
        if gap.summary.trim().is_empty() {
            return shaping_validation(
                &request,
                project_state,
                "gaps",
                "gap summaries must not be empty",
            );
        }
        let gap_id =
            allocate_shaping_gap_id(service.durable_id_generator(), store, &reserved_gap_ids)
                .map_err(PlanError::Core)?;
        reserved_gap_ids.insert(gap_id.as_str().to_owned());
        let decision_policy = gap.gap_kind.decision_policy_for_mode(task.mode);
        let user_action_draft = gap.user_action.as_ref();
        if gap.gap_kind.is_user_owned() != user_action_draft.is_some() {
            return shaping_validation(
                &request,
                project_state,
                "gaps",
                "user-owned shaping gaps require one UserAction draft and other gaps forbid it",
            );
        }
        let mut user_action_insert = None;
        let mut projected_user_action = None;
        if let Some(draft) = user_action_draft {
            let Some(decision_policy) = decision_policy else {
                return shaping_validation(
                    &request,
                    project_state,
                    "gaps",
                    "a UserAction draft requires a user-owned shaping gap",
                );
            };
            if draft.action.action_kind() != decision_policy.user_action_kind {
                return shaping_validation(
                    &request,
                    project_state,
                    "gaps",
                    "shaping gap and UserAction kinds are incompatible",
                );
            }
            let constructed = construct_user_action(UserActionConstructionInput {
                store,
                task: &task,
                current_change_unit: current_change_unit.as_ref(),
                context: UserActionConstructionContext {
                    project_id: request.envelope.project_id.clone(),
                    observed_state_version: project_state.state_version,
                    observed_at: operation_now.clone(),
                    locale: request.envelope.locale.as_ref().cloned(),
                },
                intent: UserActionIntent {
                    task_id: request.task_id.clone(),
                    change_unit_id: current_change_unit.as_ref().map(|cu| {
                        volicord_types::ids::ChangeUnitId::new(cu.change_unit_id.clone())
                    }),
                    action: draft.action.clone(),
                    required_for: decision_policy.required_for.to_vec(),
                    expires_at: draft.expires_at.clone(),
                },
            })
            .map_err(|error| {
                user_action_service_plan_error(&request.envelope, project_state, error)
            })?;
            let request_id = allocate_user_action_request_id(service.durable_id_generator(), store)
                .map_err(PlanError::Core)?;
            let materialized = materialize_user_action_request(UserActionMaterializationInput {
                context: UserActionPersistenceContext {
                    project_id: request.envelope.project_id.clone(),
                    actor_source: verified_invocation.actor_source.clone(),
                    operation_identity: operation_identity.clone(),
                    planned_state_version,
                    user_action_request_id: request_id,
                },
                origin: UserActionOrigin::Shaping {
                    shaping_checkpoint_id: checkpoint_id.clone(),
                    shaping_gap_id: gap_id.clone(),
                },
                constructed,
            })
            .map_err(|error| {
                user_action_service_plan_error(&request.envelope, project_state, error)
            })?;
            created_request_refs.push(materialized.request_ref.clone());
            user_action_insert = Some(ShapingCheckpointUserActionInsert {
                user_action_request_id: materialized
                    .public_request
                    .user_action_request_id
                    .as_str()
                    .to_owned(),
                action_kind: materialized.public_request.action_kind,
            });
            projected_user_action = user_action_insert.as_ref().map(|link| {
                volicord_store::core_pipeline::ShapingCheckpointUserActionRecord {
                    user_action_request_id: link.user_action_request_id.clone(),
                    action_kind: link.action_kind,
                    user_action_resolution_id: None,
                    linked_at: operation_now.clone(),
                    resolved_at: None,
                }
            });
            mutations.push(materialized.mutation);
        }
        gap_inserts.push(ShapingCheckpointGapInsert {
            shaping_gap_id: gap_id.as_str().to_owned(),
            gap_kind: gap.gap_kind,
            summary: gap.summary.clone(),
            affected_refs: gap.affected_refs.clone(),
            user_action: user_action_insert,
        });
        projected_gaps.push(volicord_store::core_pipeline::ShapingCheckpointGapRecord {
            shaping_gap_id: gap_id.as_str().to_owned(),
            gap_kind: gap.gap_kind,
            summary: gap.summary.clone(),
            affected_refs: gap.affected_refs.clone(),
            status: ShapingGapStatus::Current,
            user_action: projected_user_action,
        });
    }
    let readiness = if gaps.is_empty()
        && baseline_ref.is_some()
        && implementation_boundary
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
    {
        ShapingCheckpointReadiness::Ready
    } else {
        ShapingCheckpointReadiness::Blocked
    };
    let checkpoint_insert = ShapingCheckpointInsert {
        shaping_checkpoint_id: checkpoint_id.as_str().to_owned(),
        checkpoint_operation: checkpoint_operation.clone(),
        task_id: request.task_id.as_str().to_owned(),
        scope_revision: *scope_revision,
        baseline_ref: baseline_ref.as_ref().cloned(),
        summary: summary.clone(),
        implementation_boundary: implementation_boundary.as_ref().cloned(),
        readiness,
        source_refs: source_refs.clone(),
        evidence_refs: evidence_refs.clone(),
        created_at: operation_now.clone(),
        gaps: gap_inserts,
    };
    mutations.push(CoreStorageMutation::Shaping(
        ShapingCheckpointMutation::Record(checkpoint_insert),
    ));
    if task.close_basis.is_some() {
        mutations.push(CoreStorageMutation::Task(TaskMutation::UpdateCloseBasis(
            TaskCloseBasisUpdate {
                task_id: task.task_id.clone(),
                close_basis_revision: task.close_basis_revision + 1,
                close_basis: None,
            },
        )));
    }
    let lifecycle = if !created_request_refs.is_empty() {
        TaskLifecyclePhase::WaitingUser
    } else if task.mode == TaskMode::Advisor && readiness == ShapingCheckpointReadiness::Ready {
        TaskLifecyclePhase::Ready
    } else {
        TaskLifecyclePhase::Shaping
    };
    if task.lifecycle_phase != lifecycle {
        mutations.push(CoreStorageMutation::Task(TaskMutation::UpdateScope(
            TaskScopeUpdate {
                task_id: task.task_id.clone(),
                work_phase: None,
                lifecycle_phase: Some(lifecycle),
                result: None,
                title: None,
                summary: None,
                shaping: None,
                bounded_context: None,
                autonomy_boundary: None,
                close_summary: None,
            },
        )));
    }
    let mut projected_task = task.clone();
    projected_task.lifecycle_phase = lifecycle;
    if projected_task.close_basis.is_some() {
        projected_task.close_basis_revision += 1;
        projected_task.close_basis = None;
    }
    let projected_checkpoint = ShapingCheckpointRecord {
        project_id: request.envelope.project_id.as_str().to_owned(),
        shaping_checkpoint_id: checkpoint_id.as_str().to_owned(),
        predecessor_shaping_checkpoint_id: predecessor_checkpoint_id
            .as_ref()
            .map(|id| id.as_str().to_owned()),
        task_id: request.task_id.as_str().to_owned(),
        scope_revision: *scope_revision,
        baseline_ref: baseline_ref.as_ref().cloned(),
        summary: summary.clone(),
        implementation_boundary: implementation_boundary.as_ref().cloned(),
        readiness,
        source_refs: source_refs.clone(),
        evidence_refs: evidence_refs.clone(),
        created_at: operation_now.clone(),
        superseded_at: None,
        gaps: projected_gaps,
    };
    let project_policy = project_workflow_policy(store)
        .map_err(CorePipelineError::from)?
        .summary;
    let task_wide_shaping_authority = crate::workflow_projection::task_wide_shaping_authority(
        store,
        &request.envelope.project_id,
        planned_state_version,
        &projected_task,
        current_change_unit.as_ref(),
        Some(&projected_checkpoint),
        operation_now,
    )?;
    let workflow = crate::workflow_projection::workflow_projection(
        &request.envelope.project_id,
        planned_state_version,
        &projected_task,
        current_change_unit.as_ref(),
        Some(&projected_checkpoint),
        &task_wide_shaping_authority,
    );
    let state = state_summary(StateSummaryInput {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &projected_task,
        current_change_unit: current_change_unit.as_ref(),
        shaping_checkpoint: Some(&projected_checkpoint),
        task_wide_shaping_authority: &task_wide_shaping_authority,
        project_policy,
        acceptance_criteria: active_acceptance_criteria(store, &request.task_id)?,
        pending_user_action_refs: created_request_refs.clone(),
        blocker_refs: Vec::new(),
        write_ticket_summary: None,
        evidence_summary: None,
        evidence_gate: None,
        close_state: None,
        close_blockers: Vec::new(),
        guarantee_display: None,
    })?;
    let shaping_checkpoint = ShapingCheckpoint {
        shaping_checkpoint_id: checkpoint_id.clone(),
        predecessor_checkpoint_id: RequiredNullable::new(predecessor_checkpoint_id),
        project_id: request.envelope.project_id.clone(),
        task_id: request.task_id.clone(),
        scope_revision: *scope_revision,
        baseline_ref: baseline_ref.clone(),
        summary: summary.clone(),
        implementation_boundary: implementation_boundary.clone(),
        readiness,
        source_refs: source_refs.clone(),
        evidence_refs: evidence_refs.clone(),
        created_at: operation_now.clone(),
        superseded_at: RequiredNullable::null(),
    };
    let result_fields = RecordShapingResultFields {
        shaping_checkpoint,
        created_user_action_request_refs: created_request_refs,
        workflow,
        state,
    };
    Ok(RecordShapingPlan {
        operation: OperationPlan::new(
            request.task_id,
            current_change_unit
                .as_ref()
                .map(|cu| volicord_types::ids::ChangeUnitId::new(cu.change_unit_id.clone())),
            mutations,
            object_from_value(json!({
                "shaping_checkpoint_id": checkpoint_id,
                "readiness": readiness,
            }))?,
        ),
        result_fields,
    })
}

fn shaping_validation<T>(
    request: &RecordShapingRequest,
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

fn plan_finalize_advice(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordShapingRequest,
    operation_now: &volicord_types::values::UtcTimestamp,
) -> Result<RecordShapingPlan, PlanError> {
    let RecordShapingOperation::FinalizeAdvice {
        shaping_checkpoint_id,
        change_unit_id,
        scope_revision,
        baseline_ref,
        user_action_resolution_ids,
        result_summary,
        result_refs,
        evidence_refs,
        residual_risks: risk_inputs,
        recovery_constraints,
    } = &request.operation
    else {
        return Err(CorePipelineError::Invariant {
            detail: "advisor finalization planner received a checkpoint operation".to_owned(),
        }
        .into());
    };
    let task = store
        .task_record(&request.task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    if task.mode != TaskMode::Advisor || task.work_phase != WorkPhase::Shaping {
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::WorkflowActionNotAllowed,
            "finalize_advice requires an advisor Task in shaping",
            MethodName::RecordShaping,
            None,
            Vec::new(),
            false,
            MethodName::Status,
        );
    }
    if task.scope_revision != *scope_revision
        || task.shaping.baseline_ref.as_ref() != Some(baseline_ref)
    {
        return shaping_validation(
            request,
            project_state,
            "operation",
            "finalize_advice scope_revision and baseline_ref must be current",
        );
    }
    if result_summary.trim().is_empty() {
        return shaping_validation(
            request,
            project_state,
            "operation.result_summary",
            "result_summary must not be empty",
        );
    }
    if recovery_constraints
        .iter()
        .any(|constraint| constraint.trim().is_empty())
    {
        return shaping_validation(
            request,
            project_state,
            "operation.recovery_constraints",
            "recovery constraints must not be empty strings",
        );
    }
    let checkpoint = store
        .current_shaping_checkpoint(&request.task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| CorePipelineError::Invariant {
            detail: "advisor finalization requires a current shaping checkpoint".to_owned(),
        })?;
    if checkpoint.shaping_checkpoint_id != shaping_checkpoint_id.as_str()
        || checkpoint.readiness != ShapingCheckpointReadiness::Ready
        || checkpoint.scope_revision != *scope_revision
        || checkpoint.baseline_ref.as_ref() != Some(baseline_ref)
        || checkpoint
            .gaps
            .iter()
            .any(|gap| gap.status == ShapingGapStatus::Current)
    {
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::ShapingCheckpointStale,
            "finalize_advice requires the exact structurally ready current checkpoint with no unresolved gap",
            MethodName::RecordShaping,
            None,
            Vec::new(),
            true,
            MethodName::Status,
        );
    }
    let change_unit = store
        .current_change_unit(&request.task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| CorePipelineError::Invariant {
            detail: "advisor finalization requires a current non-write Change Unit".to_owned(),
        })?;
    if change_unit.change_unit_id != change_unit_id.as_str()
        || change_unit.write_basis.baseline_ref.as_ref() != Some(baseline_ref)
        || change_unit.lifecycle.recovery_required
        || !advisor_compatible_change_unit(
            &change_unit.bounded_paths,
            change_unit.effect_contract.as_ref(),
        )
    {
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::ChangeUnitStale,
            "finalize_advice requires the exact current observe-only Change Unit",
            MethodName::RecordShaping,
            None,
            Vec::new(),
            true,
            MethodName::UpdateScope,
        );
    }
    let task_wide_authority = crate::workflow_projection::task_wide_shaping_authority(
        store,
        &request.envelope.project_id,
        project_state.state_version,
        &task,
        Some(&change_unit),
        Some(&checkpoint),
        operation_now,
    )?;
    if !task_wide_authority.inconsistent.is_empty() {
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::UserDecisionUnresolved,
            "task-wide UserAction authority required for advisor finalization is inconsistent",
            MethodName::RecordShaping,
            None,
            Vec::new(),
            false,
            MethodName::Status,
        );
    }

    let mut applications = Vec::new();
    let mut expected_resolution_ids = BTreeSet::new();
    let mut applied_resolution_refs = Vec::new();
    for gap in checkpoint
        .gaps
        .iter()
        .filter(|gap| gap.gap_kind.is_user_owned())
    {
        let policy = gap
            .gap_kind
            .decision_policy_for_mode(TaskMode::Advisor)
            .ok_or_else(|| CorePipelineError::Invariant {
                detail: "advisor decision application policy is missing".to_owned(),
            })?;
        if policy.application_owner == ShapingDecisionApplicationOwner::UpdateScope
            && gap.status != ShapingGapStatus::Applied
        {
            return workflow_rejection_plan_error(
                store,
                project_state,
                &request.envelope,
                &request.task_id,
                ErrorCode::UserDecisionUnresolved,
                "an advisor scope decision must be applied by update_scope before finalization",
                MethodName::RecordShaping,
                None,
                Vec::new(),
                true,
                MethodName::UpdateScope,
            );
        }
        if policy.application_owner == ShapingDecisionApplicationOwner::RecordShaping
            && !matches!(
                gap.status,
                ShapingGapStatus::Resolved | ShapingGapStatus::Applied
            )
        {
            return workflow_rejection_plan_error(
                store,
                project_state,
                &request.envelope,
                &request.task_id,
                ErrorCode::UserDecisionUnresolved,
                "every advisor-owned decision must be resolved before finalization",
                MethodName::RecordShaping,
                None,
                Vec::new(),
                true,
                MethodName::ResolveUserAction,
            );
        }
        let link = gap
            .user_action
            .as_ref()
            .ok_or_else(|| CorePipelineError::Invariant {
                detail: "an advisor shaping decision has no UserAction link".to_owned(),
            })?;
        let resolution_id = link.user_action_resolution_id.as_ref().ok_or_else(|| {
            CorePipelineError::Invariant {
                detail: "an advisor shaping decision has no resolution link".to_owned(),
            }
        })?;
        let record = store
            .user_action_record(&link.user_action_request_id, operation_now)
            .map_err(CorePipelineError::from)?
            .ok_or_else(|| CorePipelineError::Invariant {
                detail: "an advisor shaping decision references a missing UserAction request"
                    .to_owned(),
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
                detail: format!("advisor UserAction authority is invalid: {error}"),
            }
        })?;
        let exact_basis = record.status() == UserActionStatus::Resolved
            && record.request().basis_status() == UserActionBasisStatus::Current
            && coordinates.compatibility_status == UserActionBasisStatus::Current
            && coordinates.task_id == request.task_id
            && coordinates.scope_revision == *scope_revision
            && coordinates.change_unit_id.as_ref() == Some(change_unit_id)
            && coordinates.baseline_ref.as_ref() == Some(baseline_ref)
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
                "an advisor resolution is stale or does not match its exact current gap basis",
                MethodName::RecordShaping,
                None,
                Vec::new(),
                false,
                MethodName::Status,
            );
        }
        expected_resolution_ids.insert(resolution_id.clone());
        applied_resolution_refs.push(crate::record_refs::state_ref(
            StateRecordKind::UserActionResolution,
            resolution_id,
            &request.envelope.project_id,
            Some(&request.task_id),
            Some(project_state.state_version + 1),
        ));
        if policy.application_owner == ShapingDecisionApplicationOwner::RecordShaping
            && gap.status == ShapingGapStatus::Resolved
        {
            applications.push(ShapingGapApplication {
                shaping_gap_id: gap.shaping_gap_id.clone(),
                user_action_resolution_id: resolution_id.clone(),
            });
        }
    }
    let supplied_resolution_ids = user_action_resolution_ids
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    if supplied_resolution_ids != expected_resolution_ids
        || supplied_resolution_ids.len() != user_action_resolution_ids.len()
    {
        return shaping_validation(
            request,
            project_state,
            "operation.user_action_resolution_ids",
            "finalize_advice requires the exact current resolution set",
        );
    }
    validate_advisor_refs(
        store,
        project_state,
        request,
        change_unit_id,
        result_refs,
        false,
    )?;
    validate_advisor_refs(
        store,
        project_state,
        request,
        change_unit_id,
        evidence_refs,
        true,
    )?;

    let mut allocated_risk_ids = BTreeSet::new();
    let mut residual_risks = Vec::new();
    for risk in risk_inputs {
        if risk.summary.trim().is_empty() || risk.consequence.trim().is_empty() {
            return shaping_validation(
                request,
                project_state,
                "operation.residual_risks",
                "residual risk summary and consequence must not be empty",
            );
        }
        validate_advisor_refs(
            store,
            project_state,
            request,
            change_unit_id,
            &risk.source_refs,
            false,
        )?;
        let risk_id = allocate_risk_id(service.durable_id_generator(), &allocated_risk_ids)
            .map_err(PlanError::Core)?;
        allocated_risk_ids.insert(risk_id.as_str().to_owned());
        residual_risks.push(ResidualRisk {
            risk_id,
            summary: risk.summary.trim().to_owned(),
            consequence: risk.consequence.trim().to_owned(),
            acceptance_required: risk.acceptance_required,
            source_refs: risk.source_refs.clone(),
        });
    }
    let planned_state_version = project_state.state_version + 1;
    let checkpoint_ref = crate::record_refs::state_ref(
        StateRecordKind::ShapingCheckpoint,
        shaping_checkpoint_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let evidence_summary_ref = evidence_refs
        .iter()
        .find(|reference| reference.record_kind == StateRecordKind::EvidenceSummary)
        .cloned();
    let close_basis = CurrentCloseBasis {
        close_basis_revision: task.close_basis_revision + 1,
        scope_revision: *scope_revision,
        task_id: request.task_id.clone(),
        change_unit_id: change_unit_id.clone(),
        baseline_ref: RequiredNullable::some(baseline_ref.clone()),
        result_summary: result_summary.trim().to_owned(),
        result_refs: result_refs.clone(),
        evidence_refs: evidence_refs.clone(),
        evidence_summary_ref: RequiredNullable::new(evidence_summary_ref),
        residual_risks,
        sensitive_categories: Vec::new(),
        sensitive_action_requirements: Vec::new(),
        recovery_constraints: recovery_constraints.clone(),
        source_run_ref: RequiredNullable::null(),
        shaping_checkpoint_ref: RequiredNullable::some(checkpoint_ref.clone()),
        applied_user_action_resolution_refs: applied_resolution_refs,
        updated_at: operation_now.clone(),
    };
    let mut projected_checkpoint = checkpoint.clone();
    for gap in &mut projected_checkpoint.gaps {
        if applications
            .iter()
            .any(|application| application.shaping_gap_id == gap.shaping_gap_id)
        {
            gap.status = ShapingGapStatus::Applied;
        }
    }
    let mut projected_task = task.clone();
    projected_task.summary = Some(result_summary.trim().to_owned());
    projected_task.lifecycle_phase = TaskLifecyclePhase::Ready;
    projected_task.close_basis_revision = close_basis.close_basis_revision;
    projected_task.close_basis = Some(close_basis.clone());
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
    let projected_project_state = project_state_header(
        project_state,
        planned_state_version,
        Some(task.task_id.clone()),
    );
    let close_plan = crate::close_readiness::plan_projected_close_readiness(
        store,
        &projected_project_state,
        &request.envelope.project_id,
        &request.task_id,
        crate::close_readiness::facts_from_projection(
            projected_task.clone(),
            Some(change_unit.clone()),
            Some(close_basis.clone()),
            Vec::new(),
            Vec::new(),
            None,
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
        evidence_gate: Some(close_plan.evidence_gate),
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers,
        guarantee_display: None,
    })?;
    let shaping_checkpoint = ShapingCheckpoint {
        shaping_checkpoint_id: shaping_checkpoint_id.clone(),
        predecessor_checkpoint_id: RequiredNullable::new(
            checkpoint
                .predecessor_shaping_checkpoint_id
                .as_deref()
                .map(volicord_types::ids::ShapingCheckpointId::new),
        ),
        project_id: request.envelope.project_id.clone(),
        task_id: request.task_id.clone(),
        scope_revision: checkpoint.scope_revision,
        baseline_ref: RequiredNullable::new(checkpoint.baseline_ref.clone()),
        summary: checkpoint.summary.clone(),
        implementation_boundary: RequiredNullable::new(checkpoint.implementation_boundary.clone()),
        readiness: checkpoint.readiness,
        source_refs: checkpoint.source_refs.clone(),
        evidence_refs: checkpoint.evidence_refs.clone(),
        created_at: checkpoint.created_at.clone(),
        superseded_at: RequiredNullable::new(checkpoint.superseded_at.clone()),
    };
    let mutations = vec![
        CoreStorageMutation::Shaping(ShapingCheckpointMutation::ApplyAdvisorFinalization(
            ShapingAdvanceApplication {
                task_id: task.task_id.clone(),
                shaping_checkpoint_id: checkpoint.shaping_checkpoint_id.clone(),
                change_unit_id: change_unit.change_unit_id.clone(),
                scope_revision: *scope_revision,
                baseline_ref: baseline_ref.clone(),
                applications: applications.clone(),
            },
        )),
        CoreStorageMutation::Task(TaskMutation::UpdateCloseBasis(TaskCloseBasisUpdate {
            task_id: task.task_id.clone(),
            close_basis_revision: close_basis.close_basis_revision,
            close_basis: Some(close_basis),
        })),
        CoreStorageMutation::Task(TaskMutation::UpdateScope(TaskScopeUpdate {
            task_id: task.task_id.clone(),
            work_phase: None,
            lifecycle_phase: Some(TaskLifecyclePhase::Ready),
            result: None,
            title: None,
            summary: Some(result_summary.trim().to_owned()),
            shaping: None,
            bounded_context: None,
            autonomy_boundary: None,
            close_summary: None,
        })),
    ];
    Ok(RecordShapingPlan {
        operation: OperationPlan::new(
            request.task_id.clone(),
            Some(change_unit_id.clone()),
            mutations,
            object_from_value(json!({
                "operation": "finalize_advice",
                "shaping_checkpoint_id": shaping_checkpoint_id,
                "change_unit_id": change_unit_id,
                "applied_shaping_gap_ids": applications
                    .iter()
                    .map(|application| application.shaping_gap_id.clone())
                    .collect::<Vec<_>>(),
                "close_basis_revision": projected_task.close_basis_revision,
            }))?,
        ),
        result_fields: RecordShapingResultFields {
            shaping_checkpoint,
            created_user_action_request_refs: Vec::new(),
            workflow,
            state,
        },
    })
}

fn validate_advisor_refs(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordShapingRequest,
    change_unit_id: &volicord_types::ids::ChangeUnitId,
    refs: &[StateRecordRef],
    evidence_only: bool,
) -> Result<(), PlanError> {
    let mut identities = BTreeSet::new();
    for reference in refs {
        if reference.project_id != request.envelope.project_id
            || reference.task_id.as_ref() != Some(&request.task_id)
            || !identities.insert((reference.record_kind as u8, reference.record_id.clone()))
        {
            return shaping_validation(
                request,
                project_state,
                "operation.result_refs",
                "advisor refs must be unique and owned by the exact Task",
            );
        }
        let exists = match reference.record_kind {
            StateRecordKind::ChangeUnit if !evidence_only => {
                reference.record_id.as_str() == change_unit_id.as_str()
            }
            StateRecordKind::Artifact => store
                .artifact_record(reference.record_id.as_str())
                .map_err(CorePipelineError::from)?
                .is_some_and(|record| record.task_id == request.task_id.as_str()),
            StateRecordKind::EvidenceSummary => store
                .evidence_summary_record(reference.record_id.as_str())
                .map_err(CorePipelineError::from)?
                .is_some_and(|record| {
                    record.task_id == request.task_id.as_str()
                        && record.change_unit_id.as_deref() == Some(change_unit_id.as_str())
                }),
            _ => false,
        };
        if !exists {
            return shaping_validation(
                request,
                project_state,
                "operation.result_refs",
                "advisor refs must identify current supported artifact, evidence, or Change Unit state",
            );
        }
    }
    Ok(())
}
