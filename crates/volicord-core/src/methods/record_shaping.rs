use std::collections::BTreeSet;

use serde_json::json;
use volicord_store::core_pipeline::{
    CoreProjectStore, CoreStorageMutation, ProjectStateHeader, ShapingCheckpointGapInsert,
    ShapingCheckpointInsert, ShapingCheckpointMutation, ShapingCheckpointRecord,
    ShapingCheckpointUserActionInsert, TaskCloseBasisUpdate, TaskMutation, TaskScopeUpdate,
};
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::methods::{
    MethodOperationCategory, RecordShapingRequest, RecordShapingResultFields,
};
use volicord_types::schema::{
    CurrentCloseBasis, RequiredNullable, ResidualRisk, ShapingCheckpoint,
    ShapingCheckpointOperation, WorkflowRejectionUserAction,
};
use volicord_types::values::{
    ErrorCode, MethodName, ShapingCheckpointReadiness, ShapingGapStatus, StateRecordKind,
    TaskLifecyclePhase, TaskMode, UserActionBasisStatus, UserActionStatus, WorkPhase,
};
use volicord_user_action_service::{
    construct_user_action, materialize_user_action_request, UserActionConstructionContext,
    UserActionConstructionInput, UserActionIntent, UserActionMaterializationInput,
    UserActionOrigin, UserActionPersistenceContext,
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
use crate::state_summary::{state_summary, StateSummaryInput};

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
        if request.envelope.dry_run.is_requested() {
            return self.execute_prepared_request(
                prepared,
                dry_run_preview_branch::<RecordShapingRequest>(dry_run_summary(
                    "shaping_checkpoint",
                    "commit",
                    "Shaping checkpoint and linked UserAction requests would be recorded atomically.",
                    Vec::new(),
                )),
            );
        }
        self.execute_prepared_request(
            prepared,
            commit_mutation_branch::<RecordShapingRequest>(
                plan.operation.into_commit_branch::<RecordShapingRequest>(
                    plan.result_fields,
                    "shaping_recorded",
                ),
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
    if task.scope_revision != request.scope_revision {
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
        match &request.checkpoint_operation {
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
                                    .decision_policy()
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
    if request.summary.trim().is_empty() {
        return shaping_validation(
            &request,
            project_state,
            "summary",
            "summary must not be empty",
        );
    }
    let task_baseline = task.shaping.baseline_ref.as_ref();
    if request.baseline_ref.as_ref() != task_baseline {
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
    for gap in &request.gaps {
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
        let decision_policy = gap.gap_kind.decision_policy();
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
    let readiness = if request.gaps.is_empty()
        && request.baseline_ref.is_some()
        && request
            .implementation_boundary
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
    {
        ShapingCheckpointReadiness::Ready
    } else {
        ShapingCheckpointReadiness::Blocked
    };
    let checkpoint_insert = ShapingCheckpointInsert {
        shaping_checkpoint_id: checkpoint_id.as_str().to_owned(),
        checkpoint_operation: request.checkpoint_operation.clone(),
        task_id: request.task_id.as_str().to_owned(),
        scope_revision: request.scope_revision,
        baseline_ref: request.baseline_ref.as_ref().cloned(),
        summary: request.summary.clone(),
        implementation_boundary: request.implementation_boundary.as_ref().cloned(),
        readiness,
        source_refs: request.source_refs.clone(),
        evidence_refs: request.evidence_refs.clone(),
        created_at: operation_now.clone(),
        gaps: gap_inserts,
    };
    mutations.push(CoreStorageMutation::Shaping(
        ShapingCheckpointMutation::Record(checkpoint_insert),
    ));
    let checkpoint_ref = crate::record_refs::state_ref(
        StateRecordKind::ShapingCheckpoint,
        checkpoint_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let advisor_close_basis = if task.mode == TaskMode::Advisor {
        if readiness == ShapingCheckpointReadiness::Ready {
            request.close_assessment.as_ref().map(|assessment| {
                let change_unit = current_change_unit.as_ref().ok_or_else(|| {
                    PlanError::Response(Box::new(
                        validation_rejected(
                            request.envelope.dry_run,
                            Some(project_state.state_version),
                            "close_assessment",
                            "a ready advisor close assessment requires a current non-write Change Unit boundary",
                        )
                        .expect("validation response serializes"),
                    ))
                })?;
                if !assessment.sensitive_categories.is_empty() {
                    return shaping_validation(
                        &request,
                        project_state,
                        "close_assessment.sensitive_categories",
                        "advisor shaping cannot establish sensitive write requirements",
                    );
                }
                let mut allocated_risk_ids = BTreeSet::new();
                let mut residual_risks = Vec::new();
                for risk in &assessment.residual_risks {
                    let risk_id = allocate_risk_id(
                        service.durable_id_generator(),
                        &allocated_risk_ids,
                    )
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
                let mut result_refs = assessment.result_refs.clone();
                if !result_refs.contains(&checkpoint_ref) {
                    result_refs.push(checkpoint_ref.clone());
                }
                Ok::<_, PlanError>(CurrentCloseBasis {
                    close_basis_revision: task.close_basis_revision + 1,
                    scope_revision: task.scope_revision,
                    task_id: request.task_id.clone(),
                    change_unit_id: volicord_types::ids::ChangeUnitId::new(
                        change_unit.change_unit_id.clone(),
                    ),
                    baseline_ref: request.baseline_ref.clone(),
                    result_summary: assessment.result_summary.trim().to_owned(),
                    result_refs,
                    evidence_summary_ref: RequiredNullable::null(),
                    residual_risks,
                    sensitive_categories: Vec::new(),
                    sensitive_action_requirements: Vec::new(),
                    recovery_constraints: assessment.recovery_constraints.clone(),
                    source_run_ref: checkpoint_ref.clone(),
                    updated_at: operation_now.clone(),
                })
            }).transpose()?
        } else {
            if request.close_assessment.is_some() {
                return shaping_validation(
                    &request,
                    project_state,
                    "close_assessment",
                    "an advisor close assessment requires a ready shaping checkpoint",
                );
            }
            None
        }
    } else {
        if request.close_assessment.is_some() {
            return shaping_validation(
                &request,
                project_state,
                "close_assessment",
                "a work shaping checkpoint cannot establish an implementation close basis",
            );
        }
        None
    };
    if let Some(close_basis) = advisor_close_basis.as_ref() {
        mutations.push(CoreStorageMutation::Task(TaskMutation::UpdateCloseBasis(
            TaskCloseBasisUpdate {
                task_id: task.task_id.clone(),
                close_basis_revision: close_basis.close_basis_revision,
                close_basis: Some(close_basis.clone()),
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
    if let Some(close_basis) = advisor_close_basis {
        projected_task.close_basis_revision = close_basis.close_basis_revision;
        projected_task.close_basis = Some(close_basis);
    }
    let projected_checkpoint = ShapingCheckpointRecord {
        project_id: request.envelope.project_id.as_str().to_owned(),
        shaping_checkpoint_id: checkpoint_id.as_str().to_owned(),
        predecessor_shaping_checkpoint_id: predecessor_checkpoint_id
            .as_ref()
            .map(|id| id.as_str().to_owned()),
        task_id: request.task_id.as_str().to_owned(),
        scope_revision: request.scope_revision,
        baseline_ref: request.baseline_ref.as_ref().cloned(),
        summary: request.summary.clone(),
        implementation_boundary: request.implementation_boundary.as_ref().cloned(),
        readiness,
        source_refs: request.source_refs.clone(),
        evidence_refs: request.evidence_refs.clone(),
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
        scope_revision: request.scope_revision,
        baseline_ref: request.baseline_ref.clone(),
        summary: request.summary.clone(),
        implementation_boundary: request.implementation_boundary.clone(),
        readiness,
        source_refs: request.source_refs.clone(),
        evidence_refs: request.evidence_refs.clone(),
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
