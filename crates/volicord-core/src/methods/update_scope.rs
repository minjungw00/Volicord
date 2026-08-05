use crate::acceptance_facts::active_acceptance_criteria;
use crate::change_unit_planning::plan_current_change_unit;
use crate::close_readiness::{
    facts_from_projection, facts_with_pending_authorities,
    facts_with_projected_acceptance_criteria, plan_projected_close_readiness,
};
use crate::enforcement_facts::project_enforcement_profile;
use crate::error_boundary::{
    product_path::observe_request_product_paths,
    store::{plan_error_response, store_error_plan},
    user_action::user_action_service_plan_error,
};
use crate::evidence_facts::load_current_evidence_summary_facts;
use crate::evidence_projection::evidence_summary_for_display;
use crate::guarantee_projection::guarantee_display;
use crate::identity::{allocate_acceptance_criterion_id, allocate_change_unit_id};
use crate::json_object::object_from_value;
use crate::method_execution::{mutation_method_policy, prepare_or_response, PlanError};
use crate::method_rejection::{
    authority_basis_mismatch_plan_error, decision_rejected_response, dry_run_summary,
    no_active_task_response, rejected_pipeline_response, validation_rejected,
    workflow_rejection_plan_error,
};
use crate::operation_plan::OperationPlan;
use crate::pipeline::{
    commit_mutation_branch, dry_run_preview_branch, tool_error, CorePipelineError, CoreResult,
    CoreService, InvocationContext, PipelineResponse, TaskRequirement, VerifiedInvocationContext,
};
use crate::policy::close_readiness::{
    accepted_current_scope_decision_authority, ScopeDecisionAuthorityRequirement,
};
use crate::policy::close_readiness_evidence::{
    evidence_summary_with_required_criteria, project_close_evidence_summary,
    required_acceptance_criterion_ids,
};
use crate::policy::effect_contract::{validate_effect_contract, EffectContractValidationError};
use crate::policy::workflow::{
    acceptance_policy_for_control, project_workflow_policy, resolve_task_control_authority,
    ProjectWorkflowPolicy,
};
use crate::record_refs::{change_unit_ref, state_ref, state_ref_from_stored, write_ticket_ref};
use crate::state_summary::{project_state_header, state_summary, StateSummaryInput};
use crate::task_facts::{active_blocker_refs, current_close_basis};
use crate::task_policy::{plan_user_action_lifecycle_transition, TaskLifecycleFacts};
use crate::task_state::{normalize_display_text, StoredScope};
use crate::write_ticket::service::load_current_write_ticket_summary;
use serde_json::json;
use std::collections::BTreeSet;
use volicord_store::core_pipeline::{
    AcceptanceCriteriaReplace, AcceptanceCriterionStatus, AcceptanceCriterionUpsert,
    ChangeUnitMutation, ChangeUnitRecord, CoreProjectStore, CoreStorageMutation,
    ProjectStateHeader, ShapingCheckpointMutation, ShapingGapApplication, TaskAutonomyBoundary,
    TaskCloseBasisUpdate, TaskControlLevelUpdate, TaskMutation, TaskRecord,
    TaskScopeRevisionUpdate, TaskScopeUpdate, TaskShapingFacts, UserActionBasisUpdate,
    UserActionInvalidation, UserActionMutation, WriteTicketInvalidation, WriteTicketMutation,
};
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::shaping_decision_application_id;
use volicord_types::ids::{ChangeUnitId, TaskId};
use volicord_types::methods::{
    MethodOperationCategory, UpdateScopeRequest, UpdateScopeResultFields,
};
use volicord_types::schema::{
    advisor_compatible_change_unit, AcceptanceCriterion, AuthorityBasisValue, JsonObject,
    PersistedUserActionRequestMetadata, StateRecordRef, UserActionBasis,
};
use volicord_types::values::{
    AcceptancePolicy, ChangeUnitEffectKind, ChangeUnitOperation, ErrorCode, MethodName,
    ShapingDecisionApplicationOwner, ShapingGapStatus, StateRecordKind, TaskControlLevel, TaskMode,
    UserActionBasisStatus, UtcTimestamp, WorkPhase, WriteTicketInvalidationReason,
};
use volicord_user_action_service::{
    pending_user_action_refs_for_operation, projected_user_action_lifecycle_phase,
    user_action_authority_from_record, UserActionOperation, UserActionOperationContext,
};

impl CoreService {
    /// Executes `volicord.update_scope` through the shared Core mutation pipeline.
    pub fn update_scope(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        request: UpdateScopeRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = request.envelope.task_id.as_ref() {
            if envelope_task_id != &request.task_id {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match UpdateScopeRequest.task_id",
                );
            }
        }
        let policy = mutation_method_policy(
            MethodName::UpdateScope,
            request.operation_category(),
            TaskRequirement::Exact(request.task_id.clone()),
            request.envelope.dry_run,
        );
        let prepared = match prepare_or_response(
            self,
            Some(context),
            MethodName::UpdateScope,
            request.envelope.clone(),
            request_json,
            invocation,
            policy,
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan = match plan_update_scope(
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
                dry_run_preview_branch::<UpdateScopeRequest>(dry_run_summary(
                    "scope",
                    "commit",
                    "Scope update would update current Task scope and Change Unit state.",
                    Vec::new(),
                )),
            );
        }

        self.execute_prepared_request(
            prepared,
            commit_mutation_branch::<UpdateScopeRequest>(
                plan.operation
                    .into_commit_branch::<UpdateScopeRequest>(plan.result_fields, "scope_updated"),
            ),
        )
    }
}

struct NormalizedUpdateScopeRequest {
    request: UpdateScopeRequest,
    sensitive_effect: bool,
}

fn normalize_update_scope_request(request: UpdateScopeRequest) -> NormalizedUpdateScopeRequest {
    let sensitive_effect = request
        .change_unit
        .effect_contract
        .as_ref()
        .is_some_and(|contract| {
            !contract.sensitive_action_expectations.is_empty()
                || contract.allowed_effects.iter().any(|effect| {
                    matches!(
                        effect,
                        ChangeUnitEffectKind::SensitiveAction
                            | ChangeUnitEffectKind::ExternalNetwork
                            | ChangeUnitEffectKind::SecretAccess
                    )
                })
        });
    NormalizedUpdateScopeRequest {
        request,
        sensitive_effect,
    }
}

struct ResolvedUpdateScopeContext {
    request: UpdateScopeRequest,
    sensitive_effect: bool,
    planned_state_version: u64,
    plan_now: UtcTimestamp,
    task: TaskRecord,
    current_change_unit: Option<ChangeUnitRecord>,
    workflow_policy: ProjectWorkflowPolicy,
}

fn resolve_update_scope_context(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    operation_now: &UtcTimestamp,
    normalized: NormalizedUpdateScopeRequest,
) -> Result<ResolvedUpdateScopeContext, PlanError> {
    let NormalizedUpdateScopeRequest {
        request,
        sensitive_effect,
    } = normalized;
    let planned_state_version = project_state.state_version + 1;
    let task = store
        .task_record(&request.task_id)
        .map_err(|error| store_error_plan(&request.envelope, project_state, error))?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    let current_change_unit = store
        .current_change_unit(&request.task_id)
        .map_err(|error| store_error_plan(&request.envelope, project_state, error))?;
    validate_requested_effect_contract(
        store,
        project_state,
        &request,
        &task,
        current_change_unit.as_ref(),
    )?;
    let workflow_policy = project_workflow_policy(store).map_err(CorePipelineError::from)?;

    Ok(ResolvedUpdateScopeContext {
        request,
        sensitive_effect,
        planned_state_version,
        plan_now: operation_now.clone(),
        task,
        current_change_unit,
        workflow_policy,
    })
}

struct ScopePolicyDecision {
    request: UpdateScopeRequest,
    planned_state_version: u64,
    plan_now: UtcTimestamp,
    task: TaskRecord,
    current_change_unit: Option<ChangeUnitRecord>,
    next_control: TaskControlLevel,
    next_acceptance: AcceptancePolicy,
    control_raised: bool,
    acceptance_raised: bool,
    control_level_reason: String,
    acceptance_policy_reason: String,
    linked_scope_decision_refs: Vec<StateRecordRef>,
    scope_gap_applications: Vec<ShapingGapApplication>,
}

fn decide_update_scope_policy(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    resolved: ResolvedUpdateScopeContext,
) -> Result<ScopePolicyDecision, PlanError> {
    let ResolvedUpdateScopeContext {
        request,
        sensitive_effect,
        planned_state_version,
        plan_now,
        task,
        current_change_unit,
        workflow_policy,
    } = resolved;
    let current_control = task.effective_control_level;
    let resolved_control =
        resolve_task_control_authority(&task, &workflow_policy).map_err(CorePipelineError::from)?;
    let next_control = if sensitive_effect {
        TaskControlLevel::Sensitive
    } else {
        resolved_control.effective_control_level
    };
    let current_acceptance = task.acceptance_policy;
    let control_acceptance = acceptance_policy_for_control(next_control, &workflow_policy);
    let next_acceptance = if acceptance_policy_rank_for_scope(resolved_control.acceptance_policy)
        >= acceptance_policy_rank_for_scope(control_acceptance)
    {
        resolved_control.acceptance_policy
    } else {
        control_acceptance
    };
    let acceptance_raised = acceptance_policy_rank_for_scope(next_acceptance)
        > acceptance_policy_rank_for_scope(current_acceptance);
    let control_raised = next_control > current_control;
    let control_level_reason = if sensitive_effect && control_raised {
        "Core raised control to `sensitive` for the proposed Change Unit effect contract."
            .to_owned()
    } else if control_raised {
        resolved_control.control_level_reason.clone()
    } else {
        task.control_level_reason.clone()
    };
    let acceptance_policy_reason = if acceptance_raised
        && next_control == resolved_control.effective_control_level
        && resolved_control.acceptance_raised
    {
        resolved_control.acceptance_policy_reason.clone()
    } else {
        format!(
            "Effective control `{}` requires final acceptance for the current close basis.",
            next_control.as_str()
        )
    };
    let checkpoint = store
        .current_shaping_checkpoint(&request.task_id)
        .map_err(CorePipelineError::from)?;
    let shaping_authority = crate::workflow_projection::task_wide_shaping_authority(
        store,
        &request.envelope.project_id,
        project_state.state_version,
        &task,
        current_change_unit.as_ref(),
        checkpoint.as_ref(),
        &plan_now,
    )?;
    if !shaping_authority.recovery_required.is_empty() {
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::UserDecisionUnresolved,
            "scope updates cannot apply a rejected, deferred, or expired shaping decision",
            MethodName::UpdateScope,
            None,
            Vec::new(),
            false,
            MethodName::RecordShapingCheckpoint,
        );
    }
    let validated_scope_decisions = validate_related_scope_decisions(
        store,
        project_state,
        &request,
        current_change_unit.as_ref(),
        task.scope_revision,
        &plan_now,
    )?;

    let current_change_unit_id = current_change_unit
        .as_ref()
        .map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let mut operation_refs = vec![state_ref(
        StateRecordKind::Task,
        request.task_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(project_state.state_version),
    )];
    if let Some(change_unit) = current_change_unit.as_ref() {
        operation_refs.push(change_unit_ref(
            &request.envelope.project_id,
            &request.task_id,
            change_unit,
            project_state.state_version,
        ));
    }
    let operation_context = UserActionOperationContext {
        operation: UserActionOperation::ScopeUpdate,
        task_id: &request.task_id,
        change_unit_id: current_change_unit_id.as_ref(),
        scope_revision: task.scope_revision,
        close_basis: None,
        operation_refs: &operation_refs,
        sensitive_approval: None,
    };
    if !pending_user_action_refs_for_operation(
        store,
        &request.envelope.project_id,
        project_state.state_version,
        &plan_now,
        &operation_context,
    )
    .map_err(|error| user_action_service_plan_error(&request.envelope, project_state, error))?
    .is_empty()
    {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "a current pending user action must be resolved before this scope update",
        ))));
    }

    Ok(ScopePolicyDecision {
        request,
        planned_state_version,
        plan_now,
        task,
        current_change_unit,
        next_control,
        next_acceptance,
        control_raised,
        acceptance_raised,
        control_level_reason,
        acceptance_policy_reason,
        linked_scope_decision_refs: validated_scope_decisions.resolution_refs,
        scope_gap_applications: validated_scope_decisions.applications,
    })
}

struct PlannedScopeMutations {
    request: UpdateScopeRequest,
    planned_state_version: u64,
    plan_now: UtcTimestamp,
    synthetic_task: TaskRecord,
    synthetic_change_unit: Option<ChangeUnitRecord>,
    acceptance_criteria: Vec<AcceptanceCriterion>,
    scope_changed: bool,
    next_scope_revision: u64,
    next_close_basis_revision: u64,
    change_unit_ref: Option<StateRecordRef>,
    change_unit_id: Option<ChangeUnitId>,
    linked_scope_decision_refs: Vec<StateRecordRef>,
    scope_gap_applications: Vec<ShapingGapApplication>,
    checkpoint_preserved: bool,
    stale_write_ticket_refs: Vec<StateRecordRef>,
    storage_mutations: Vec<CoreStorageMutation>,
}

fn plan_update_scope_mutations(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    policy: ScopePolicyDecision,
) -> Result<PlannedScopeMutations, PlanError> {
    let ScopePolicyDecision {
        request,
        planned_state_version,
        plan_now,
        mut task,
        current_change_unit,
        next_control,
        next_acceptance,
        control_raised,
        acceptance_raised,
        control_level_reason,
        acceptance_policy_reason,
        linked_scope_decision_refs,
        scope_gap_applications,
    } = policy;
    let current_scope = StoredScope::from_task(&task)?;
    let next_scope = current_scope.apply_request(&request);
    if request.change_unit.operation == ChangeUnitOperation::KeepCurrent
        && current_change_unit.is_some()
        && current_scope.baseline_ref != next_scope.baseline_ref
    {
        let authority_value = |value: Option<&volicord_types::ids::BaselineRef>| {
            value.map_or(AuthorityBasisValue::Null(()), |value| {
                AuthorityBasisValue::String(value.as_str().to_owned())
            })
        };
        return authority_basis_mismatch_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "baseline_ref",
            authority_value(current_scope.baseline_ref.as_ref()),
            authority_value(next_scope.baseline_ref.as_ref()),
            "baseline retargeting requires replace_current",
        );
    }
    let (acceptance_criteria, acceptance_criteria_mutation, acceptance_criteria_changed) =
        plan_acceptance_criteria_replacement(service, store, project_state, &request)?;
    let authority_basis_changed = current_scope != next_scope
        || acceptance_criteria_changed
        || request.change_unit.operation == ChangeUnitOperation::ReplaceCurrent;
    let scope_changed = current_scope != next_scope
        || acceptance_criteria_changed
        || request.change_unit.operation == ChangeUnitOperation::CreateCurrent
        || request.change_unit.operation == ChangeUnitOperation::ReplaceCurrent
        || !scope_gap_applications.is_empty();
    let next_scope_revision = if scope_changed {
        task.scope_revision + 1
    } else {
        task.scope_revision
    };
    let next_close_basis_revision = if scope_changed {
        task.close_basis_revision + 1
    } else {
        task.close_basis_revision
    };

    if scope_changed && task.work_phase == WorkPhase::Implementation {
        let authority_graph = store
            .current_shaping_authority_graph(&request.task_id, &plan_now)
            .map_err(CorePipelineError::from)?;
        if !authority_graph.current_applications.is_empty() {
            return workflow_rejection_plan_error(
                store,
                project_state,
                &request.envelope,
                &request.task_id,
                ErrorCode::TaskPhaseTransitionRequired,
                "an implementation-phase scope update cannot stale current shaping authority; close or supersede the Task first",
                MethodName::UpdateScope,
                None,
                Vec::new(),
                false,
                MethodName::CloseTask,
            );
        }
    }

    let active_write_tickets = store
        .active_write_tickets(&request.task_id)
        .map_err(|error| store_error_plan(&request.envelope, project_state, error))?;
    let stale_write_ticket_refs = if scope_changed {
        active_write_tickets
            .iter()
            .map(|record| write_ticket_ref(record, planned_state_version))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let next_shaping = TaskShapingFacts {
        goal_summary: next_scope.goal_summary.clone(),
        scope_summary: next_scope.scope_summary.clone(),
        non_goals: next_scope.non_goals.clone(),
        baseline_ref: next_scope.baseline_ref.clone(),
        autonomy_boundary: next_scope.autonomy_boundary.clone(),
        initial_context_refs: task.shaping.initial_context_refs.clone(),
        initial_source_refs: task.shaping.initial_source_refs.clone(),
    };
    let next_bounded_context = object_from_value(json!({
        "scope_update": request.scope_update.clone()
    }))?;
    let next_autonomy_boundary = TaskAutonomyBoundary {
        autonomy_boundary: next_scope.autonomy_boundary.clone(),
    };
    let mut storage_mutations = vec![CoreStorageMutation::Task(TaskMutation::UpdateScope(
        TaskScopeUpdate {
            task_id: task.task_id.clone(),
            work_phase: None,
            lifecycle_phase: None,
            result: None,
            title: next_scope.goal_summary.clone(),
            summary: next_scope.goal_summary.clone(),
            shaping: Some(next_shaping.clone()),
            bounded_context: Some(next_bounded_context.clone()),
            autonomy_boundary: Some(next_autonomy_boundary.clone()),
            close_summary: None,
        },
    ))];
    if control_raised || acceptance_raised {
        storage_mutations.push(CoreStorageMutation::Task(TaskMutation::UpdateControlLevel(
            TaskControlLevelUpdate {
                task_id: task.task_id.clone(),
                effective_control_level: next_control,
                control_level_reason: control_level_reason.clone(),
                acceptance_policy: acceptance_raised.then_some(next_acceptance),
                acceptance_policy_reason: acceptance_raised
                    .then(|| acceptance_policy_reason.clone()),
            },
        )));
        task.effective_control_level = next_control;
        task.control_level_reason = control_level_reason;
        if acceptance_raised {
            task.acceptance_policy = next_acceptance;
            task.acceptance_policy_reason = acceptance_policy_reason;
        }
    }
    if let Some(mutation) = acceptance_criteria_mutation {
        storage_mutations.push(CoreStorageMutation::Task(
            TaskMutation::ReplaceAcceptanceCriteria(mutation),
        ));
    }
    let mut checkpoint_preserved = false;
    if scope_changed {
        storage_mutations.push(CoreStorageMutation::Task(
            TaskMutation::UpdateScopeRevision(TaskScopeRevisionUpdate {
                task_id: task.task_id.clone(),
                scope_revision: next_scope_revision,
            }),
        ));
        storage_mutations.push(CoreStorageMutation::Task(TaskMutation::UpdateCloseBasis(
            TaskCloseBasisUpdate {
                task_id: task.task_id.clone(),
                close_basis_revision: next_close_basis_revision,
                close_basis: None,
            },
        )));
    }

    let mut synthetic_task = task.clone();
    synthetic_task.scope_revision = next_scope_revision;
    synthetic_task.close_basis_revision = next_close_basis_revision;
    if scope_changed {
        synthetic_task.close_basis = None;
    }
    synthetic_task.title = next_scope.goal_summary.clone();
    synthetic_task.summary = next_scope.goal_summary.clone();
    synthetic_task.shaping = next_shaping;
    synthetic_task.bounded_context = next_bounded_context;
    synthetic_task.autonomy_boundary = next_autonomy_boundary;
    let (change_unit_ref, synthetic_change_unit, change_unit_id) =
        match request.change_unit.operation {
            ChangeUnitOperation::KeepCurrent => {
                let change_unit_ref = current_change_unit.as_ref().map(|record| {
                    state_ref(
                        StateRecordKind::ChangeUnit,
                        &record.change_unit_id,
                        &request.envelope.project_id,
                        Some(&request.task_id),
                        Some(record.basis_state_version),
                    )
                });
                (
                    change_unit_ref,
                    current_change_unit.clone(),
                    current_change_unit
                        .as_ref()
                        .map(|record| ChangeUnitId::new(record.change_unit_id.clone())),
                )
            }
            ChangeUnitOperation::CreateCurrent => {
                if current_change_unit.is_some() {
                    return scope_validation_rejection(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        "change_unit.operation",
                        "create_current requires no current Change Unit",
                    );
                }
                let change_unit_id = allocate_change_unit_id(service.durable_id_generator(), store)
                    .map_err(PlanError::Core)?;
                let change_unit_plan = plan_current_change_unit(
                    &request,
                    &change_unit_id,
                    verified_invocation,
                    planned_state_version,
                );
                let record = change_unit_plan.projected_record;
                let insert = change_unit_plan.insert;
                storage_mutations.push(CoreStorageMutation::ChangeUnit(
                    ChangeUnitMutation::InsertCurrent(insert),
                ));
                synthetic_task.current_change_unit_id = Some(change_unit_id.as_str().to_owned());
                let change_unit_ref = state_ref(
                    StateRecordKind::ChangeUnit,
                    change_unit_id.as_str(),
                    &request.envelope.project_id,
                    Some(&request.task_id),
                    Some(planned_state_version),
                );
                (Some(change_unit_ref), Some(record), Some(change_unit_id))
            }
            ChangeUnitOperation::ReplaceCurrent => {
                if current_change_unit.is_none() {
                    let response = rejected_pipeline_response(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        vec![tool_error(
                            ErrorCode::NoActiveChangeUnit,
                            "replace_current requires a current Change Unit",
                            false,
                            None,
                        )],
                    )
                    .map_err(PlanError::Core)?;
                    return Err(PlanError::Response(Box::new(response)));
                }
                let change_unit_id = allocate_change_unit_id(service.durable_id_generator(), store)
                    .map_err(PlanError::Core)?;
                let change_unit_plan = plan_current_change_unit(
                    &request,
                    &change_unit_id,
                    verified_invocation,
                    planned_state_version,
                );
                let record = change_unit_plan.projected_record;
                let insert = change_unit_plan.insert;
                storage_mutations.push(CoreStorageMutation::ChangeUnit(
                    ChangeUnitMutation::ReplaceCurrent(insert),
                ));
                synthetic_task.current_change_unit_id = Some(change_unit_id.as_str().to_owned());
                let change_unit_ref = state_ref(
                    StateRecordKind::ChangeUnit,
                    change_unit_id.as_str(),
                    &request.envelope.project_id,
                    Some(&request.task_id),
                    Some(planned_state_version),
                );
                (Some(change_unit_ref), Some(record), Some(change_unit_id))
            }
        };

    if scope_changed {
        let current_checkpoint = store
            .current_shaping_checkpoint(&request.task_id)
            .map_err(CorePipelineError::from)?;
        if let Some(checkpoint) = current_checkpoint.as_ref() {
            let preserve_checkpoint = shaping_checkpoint_can_rebase(
                checkpoint,
                next_scope.baseline_ref.as_ref(),
                &scope_gap_applications,
                authority_basis_changed,
            );
            if preserve_checkpoint {
                checkpoint_preserved = true;
                let rebased_change_unit_id = synthetic_change_unit
                    .as_ref()
                    .map(|change_unit| ChangeUnitId::new(change_unit.change_unit_id.clone()));
                let mut preserved_request_ids = Vec::new();
                for link in checkpoint
                    .gaps
                    .iter()
                    .filter_map(|gap| gap.user_action.as_ref())
                {
                    let record = store
                        .user_action_record(&link.user_action_request_id, &plan_now)
                        .map_err(CorePipelineError::from)?
                        .ok_or_else(|| CorePipelineError::Invariant {
                            detail: "a preserved shaping checkpoint references a missing UserAction request"
                                .to_owned(),
                        })?;
                    if record.request().basis_status() != UserActionBasisStatus::Current {
                        continue;
                    }
                    let basis = rebase_shaping_user_action_basis(
                        record.request().basis(),
                        next_scope_revision,
                        rebased_change_unit_id.as_ref(),
                        synthetic_task.shaping.baseline_ref.as_ref(),
                    );
                    preserved_request_ids.push(link.user_action_request_id.clone());
                    storage_mutations.push(CoreStorageMutation::UserAction(
                        UserActionMutation::UpdateBasis(UserActionBasisUpdate {
                            user_action_request_id: link.user_action_request_id.clone(),
                            basis,
                            basis_status: UserActionBasisStatus::Current,
                        }),
                    ));
                }
                storage_mutations.push(CoreStorageMutation::UserAction(
                    UserActionMutation::MarkSupersededOrStale(UserActionInvalidation {
                        task_id: request.task_id.as_str().to_owned(),
                        action_kinds: Vec::new(),
                        preserved_request_ids,
                    }),
                ));
                storage_mutations.push(CoreStorageMutation::Shaping(
                    ShapingCheckpointMutation::ApplyScopeAndRebaseCurrent {
                        task_id: task.task_id.clone(),
                        shaping_checkpoint_id: checkpoint.shaping_checkpoint_id.clone(),
                        scope_revision: next_scope_revision,
                        baseline_ref: synthetic_task.shaping.baseline_ref.clone(),
                        change_unit_id: rebased_change_unit_id
                            .as_ref()
                            .map(|id| id.as_str().to_owned()),
                        applications: scope_gap_applications.clone(),
                    },
                ));
            } else {
                storage_mutations.push(CoreStorageMutation::UserAction(
                    UserActionMutation::MarkSupersededOrStale(UserActionInvalidation {
                        task_id: request.task_id.as_str().to_owned(),
                        action_kinds: Vec::new(),
                        preserved_request_ids: Vec::new(),
                    }),
                ));
                storage_mutations.push(CoreStorageMutation::Shaping(
                    ShapingCheckpointMutation::SupersedeCurrent {
                        task_id: task.task_id.clone(),
                    },
                ));
            }
        } else {
            storage_mutations.push(CoreStorageMutation::UserAction(
                UserActionMutation::MarkSupersededOrStale(UserActionInvalidation {
                    task_id: request.task_id.as_str().to_owned(),
                    action_kinds: Vec::new(),
                    preserved_request_ids: Vec::new(),
                }),
            ));
        }
    }

    if scope_changed && !active_write_tickets.is_empty() {
        let invalidation_reason = if current_scope.baseline_ref != next_scope.baseline_ref {
            WriteTicketInvalidationReason::BaselineChanged
        } else if matches!(
            request.change_unit.operation,
            ChangeUnitOperation::CreateCurrent | ChangeUnitOperation::ReplaceCurrent
        ) {
            WriteTicketInvalidationReason::ChangeUnitChanged
        } else {
            WriteTicketInvalidationReason::ScopeRevisionChanged
        };
        storage_mutations.push(CoreStorageMutation::WriteTicket(
            WriteTicketMutation::InvalidateActive(WriteTicketInvalidation {
                task_id: request.task_id.as_str().to_owned(),
                invalidation_reason,
            }),
        ));
    }
    if scope_changed {
        if let Some(lifecycle_phase) = projected_user_action_lifecycle_phase(
            project_state,
            &task,
            synthetic_change_unit.as_ref(),
            &[],
        ) {
            if let Some(transition) = plan_user_action_lifecycle_transition(
                TaskLifecycleFacts::from(&task),
                lifecycle_phase,
            )? {
                synthetic_task.lifecycle_phase = transition.target();
                storage_mutations.push(transition.storage_mutation());
            }
        }
    }

    Ok(PlannedScopeMutations {
        request,
        planned_state_version,
        plan_now,
        synthetic_task,
        synthetic_change_unit,
        acceptance_criteria,
        scope_changed,
        next_scope_revision,
        next_close_basis_revision,
        change_unit_ref,
        change_unit_id,
        linked_scope_decision_refs,
        scope_gap_applications,
        checkpoint_preserved,
        stale_write_ticket_refs,
        storage_mutations,
    })
}

struct UpdateScopeResponseProjection {
    task_id: TaskId,
    change_unit_id: Option<ChangeUnitId>,
    storage_mutations: Vec<CoreStorageMutation>,
    event_payload: JsonObject,
    result_fields: UpdateScopeResultFields,
}

struct UpdateScopePlan {
    operation: OperationPlan,
    result_fields: UpdateScopeResultFields,
}

impl UpdateScopeResponseProjection {
    fn into_plan(self) -> UpdateScopePlan {
        UpdateScopePlan {
            operation: OperationPlan::new(
                self.task_id,
                self.change_unit_id,
                self.storage_mutations,
                self.event_payload,
            ),
            result_fields: self.result_fields,
        }
    }
}

fn plan_update_scope(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: UpdateScopeRequest,
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &UtcTimestamp,
) -> Result<UpdateScopePlan, PlanError> {
    let policy = decide_update_scope_policy(
        store,
        project_state,
        resolve_update_scope_context(
            store,
            project_state,
            operation_now,
            normalize_update_scope_request(request),
        )?,
    )?;
    let mutations =
        plan_update_scope_mutations(service, store, project_state, verified_invocation, policy)?;
    let projection =
        project_update_scope_response(store, project_state, verified_invocation, mutations)?;
    Ok(projection.into_plan())
}

fn project_update_scope_response(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    mutations: PlannedScopeMutations,
) -> Result<UpdateScopeResponseProjection, PlanError> {
    let PlannedScopeMutations {
        request,
        planned_state_version,
        plan_now,
        synthetic_task,
        synthetic_change_unit,
        acceptance_criteria,
        scope_changed,
        next_scope_revision,
        next_close_basis_revision,
        change_unit_ref,
        change_unit_id: branch_change_unit_id,
        linked_scope_decision_refs,
        scope_gap_applications,
        checkpoint_preserved,
        stale_write_ticket_refs,
        storage_mutations,
    } = mutations;
    let pending_refs = if scope_changed {
        Vec::new()
    } else {
        store
            .pending_user_action_refs(&request.task_id, planned_state_version, &plan_now)
            .map_err(|error| store_error_plan(&request.envelope, project_state, error))?
            .into_iter()
            .map(state_ref_from_stored)
            .collect::<Vec<_>>()
    };
    let blocker_refs = active_blocker_refs(store, &request.task_id, planned_state_version)?;
    let mut projected_shaping_checkpoint = store
        .current_shaping_checkpoint(&request.task_id)
        .map_err(CorePipelineError::from)?;
    if scope_changed {
        if let Some(checkpoint) = projected_shaping_checkpoint.as_mut() {
            if checkpoint_preserved {
                let baseline_ref =
                    synthetic_task
                        .shaping
                        .baseline_ref
                        .as_ref()
                        .ok_or_else(|| CorePipelineError::Invariant {
                            detail: "a preserved shaping checkpoint requires a baseline".to_owned(),
                        })?;
                let projected_change_unit_id = synthetic_change_unit
                    .as_ref()
                    .map(|change_unit| ChangeUnitId::new(change_unit.change_unit_id.clone()));
                crate::workflow_projection::apply_projected_shaping_applications(
                    checkpoint,
                    &scope_gap_applications,
                    ShapingDecisionApplicationOwner::UpdateScope,
                    next_scope_revision,
                    baseline_ref,
                    projected_change_unit_id.as_ref(),
                    &plan_now,
                )?;
            } else {
                projected_shaping_checkpoint = None;
            }
        }
    }
    let task_ref = state_ref(
        StateRecordKind::Task,
        request.task_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let enforcement_profile = project_enforcement_profile(store)?;
    let guarantee_display = guarantee_display(
        &enforcement_profile,
        verified_invocation,
        planned_state_version,
    );
    let project_policy = project_workflow_policy(store)
        .map_err(CorePipelineError::from)?
        .summary;
    let write_ticket_summary = load_current_write_ticket_summary(
        store,
        &request.task_id,
        planned_state_version,
        &plan_now,
        Some(guarantee_display.clone()),
    )?;
    let projected_current_close_basis = if scope_changed {
        None
    } else {
        current_close_basis(store, &request.task_id)?
    };
    let evidence_facts = load_current_evidence_summary_facts(
        store,
        &synthetic_task,
        &request.envelope.project_id,
        &request.task_id,
        planned_state_version,
    )?;
    let required_criteria = required_acceptance_criterion_ids(&acceptance_criteria);
    let evidence_summary =
        project_close_evidence_summary(evidence_facts, &required_criteria).map(|summary| {
            evidence_summary_for_display(summary, projected_current_close_basis.as_ref())
        });
    let close_evidence_summary = if scope_changed {
        evidence_summary_with_required_criteria(None, &acceptance_criteria)
    } else {
        evidence_summary.clone()
    };
    let projected_project_state = project_state_header(
        project_state,
        planned_state_version,
        project_state
            .active_task_id
            .clone()
            .or_else(|| Some(request.task_id.as_str().to_owned())),
    );
    let close_context = facts_with_projected_acceptance_criteria(
        facts_from_projection(
            synthetic_task.clone(),
            synthetic_change_unit.clone(),
            projected_current_close_basis,
            pending_refs.clone(),
            blocker_refs.clone(),
            close_evidence_summary,
            plan_now.clone(),
        ),
        &acceptance_criteria,
    );
    let close_context = if scope_changed {
        facts_with_pending_authorities(close_context, Vec::new())
    } else {
        close_context
    };
    let close_plan = plan_projected_close_readiness(
        store,
        &projected_project_state,
        &request.envelope.project_id,
        &request.task_id,
        close_context,
    )
    .map_err(|error| {
        crate::error_boundary::close_readiness::close_readiness_plan_error(
            &request.envelope,
            &projected_project_state,
            error,
        )
    })?;
    let task_wide_shaping_authority = if projected_shaping_checkpoint.is_some() {
        crate::workflow_projection::task_wide_shaping_authority(
            store,
            &request.envelope.project_id,
            planned_state_version,
            &synthetic_task,
            synthetic_change_unit.as_ref(),
            projected_shaping_checkpoint.as_ref(),
            &plan_now,
        )?
    } else if scope_changed {
        let stored_task = store
            .task_record(&request.task_id)
            .map_err(CorePipelineError::from)?
            .ok_or_else(|| CorePipelineError::Invariant {
                detail: "scope projection lost its current Task".to_owned(),
            })?;
        let stored_change_unit = store
            .current_change_unit(&request.task_id)
            .map_err(CorePipelineError::from)?;
        let stored_checkpoint = store
            .current_shaping_checkpoint(&request.task_id)
            .map_err(CorePipelineError::from)?;
        let mut authority = crate::workflow_projection::task_wide_shaping_authority(
            store,
            &request.envelope.project_id,
            planned_state_version.saturating_sub(1),
            &stored_task,
            stored_change_unit.as_ref(),
            stored_checkpoint.as_ref(),
            &plan_now,
        )?;
        for mut fact in std::mem::take(&mut authority.applied) {
            fact.status = volicord_types::values::UserActionStatus::Stale;
            fact.authority_state = volicord_types::values::ShapingDecisionAuthorityState::Stale;
            authority.stale.push(fact);
        }
        authority
    } else {
        Default::default()
    };
    let state = state_summary(StateSummaryInput {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &synthetic_task,
        current_change_unit: synthetic_change_unit.as_ref(),
        shaping_checkpoint: projected_shaping_checkpoint.as_ref(),
        task_wide_shaping_authority: &task_wide_shaping_authority,
        project_policy,
        acceptance_criteria,
        pending_user_action_refs: pending_refs,
        blocker_refs: blocker_refs.clone(),
        write_ticket_summary,
        evidence_summary,
        evidence_gate: Some(close_plan.evidence_gate),
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers,
        guarantee_display: Some(guarantee_display),
    })?;
    let applied_shaping_gap_refs = scope_gap_applications
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
    let applied_shaping_decision_application_refs = scope_gap_applications
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
    let result_fields = UpdateScopeResultFields {
        task_ref,
        change_unit_ref,
        applied_shaping_gap_refs,
        applied_scope_decision_refs: linked_scope_decision_refs.clone(),
        applied_shaping_decision_application_refs: applied_shaping_decision_application_refs
            .clone(),
        stale_write_ticket_refs,
        blocker_refs,
        state,
    };
    let event_payload = object_from_value(json!({
        "task_id": request.task_id.clone(),
        "change_unit_operation": request.change_unit.operation,
        "scope_changed": scope_changed,
        "scope_revision": next_scope_revision,
        "close_basis_revision": next_close_basis_revision,
        "applied_shaping_gap_ids": scope_gap_applications
            .iter()
            .map(|application| application.shaping_gap_id.clone())
            .collect::<Vec<_>>(),
        "applied_scope_decision_resolution_ids": scope_gap_applications
            .iter()
            .map(|application| application.user_action_resolution_id.clone())
            .collect::<Vec<_>>(),
        "applied_shaping_decision_application_ids": scope_gap_applications
            .iter()
            .map(|application| application.shaping_decision_application_id.clone())
            .collect::<Vec<_>>(),
        "applied_shaping_decision_application_refs": applied_shaping_decision_application_refs,
    }))?;

    Ok(UpdateScopeResponseProjection {
        task_id: request.task_id,
        change_unit_id: branch_change_unit_id,
        storage_mutations,
        event_payload,
        result_fields,
    })
}

fn shaping_checkpoint_can_rebase(
    checkpoint: &volicord_store::core_pipeline::ShapingCheckpointRecord,
    next_baseline_ref: Option<&volicord_types::ids::BaselineRef>,
    scope_gap_applications: &[ShapingGapApplication],
    authority_basis_changed: bool,
) -> bool {
    if checkpoint.readiness == volicord_types::values::ShapingCheckpointReadiness::Superseded
        || next_baseline_ref.is_none()
        || checkpoint
            .implementation_boundary
            .as_deref()
            .is_none_or(|boundary| boundary.trim().is_empty())
        || (authority_basis_changed
            && scope_gap_applications.is_empty()
            && checkpoint
                .gaps
                .iter()
                .any(|gap| gap.status != ShapingGapStatus::Applied))
    {
        return false;
    }
    checkpoint.gaps.iter().all(|gap| {
        let Some(policy) = gap.gap_kind.decision_policy() else {
            return true;
        };
        if policy.application_owner != ShapingDecisionApplicationOwner::UpdateScope {
            return true;
        }
        gap.status == ShapingGapStatus::Applied
            || (gap.status == ShapingGapStatus::Accepted
                && scope_gap_applications.iter().any(|application| {
                    application.shaping_gap_id == gap.shaping_gap_id
                        && gap.user_action.as_ref().is_some_and(|link| {
                            link.user_action_resolution_id.as_deref()
                                == Some(application.user_action_resolution_id.as_str())
                        })
                }))
    })
}

fn rebase_shaping_user_action_basis(
    basis: &UserActionBasis,
    scope_revision: u64,
    change_unit_id: Option<&ChangeUnitId>,
    baseline_ref: Option<&volicord_types::ids::BaselineRef>,
) -> UserActionBasis {
    let mut basis = basis.clone();
    let coordinates = match &mut basis {
        UserActionBasis::Choice(choice) => &mut choice.coordinates,
        UserActionBasis::EvidenceObservation(observation) => &mut observation.coordinates,
    };
    coordinates.scope_revision = scope_revision;
    coordinates.change_unit_id = change_unit_id.cloned().into();
    coordinates.baseline_ref = baseline_ref.cloned().into();
    coordinates.compatibility_status = UserActionBasisStatus::Current;
    basis
}

fn scope_validation_rejection<T>(
    dry_run: volicord_types::schema::DryRunIntent,
    state_version: Option<u64>,
    field: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    let response =
        validation_rejected(dry_run, state_version, field, message).map_err(PlanError::Core)?;
    Err(PlanError::Response(Box::new(response)))
}

fn acceptance_policy_rank_for_scope(policy: AcceptancePolicy) -> u8 {
    match policy {
        AcceptancePolicy::NotRequired => 0,
        AcceptancePolicy::PolicyDependent => 1,
        AcceptancePolicy::Required => 2,
    }
}

fn plan_acceptance_criteria_replacement(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &UpdateScopeRequest,
) -> Result<
    (
        Vec<AcceptanceCriterion>,
        Option<AcceptanceCriteriaReplace>,
        bool,
    ),
    PlanError,
> {
    let current = active_acceptance_criteria(store, &request.task_id)?;
    let Some(replacements) = request.acceptance_criteria.as_ref() else {
        return Ok((current, None, false));
    };

    let mut seen_ids = BTreeSet::new();
    let mut projected = Vec::with_capacity(replacements.len());
    let mut upserts = Vec::with_capacity(replacements.len());
    for (position, replacement) in replacements.iter().enumerate() {
        let statement = normalize_display_text(&replacement.statement);
        if statement.is_empty() {
            return scope_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "acceptance_criteria[].statement",
                "acceptance criterion statements must not be empty",
            );
        }
        let acceptance_criterion_id = match replacement.acceptance_criterion_id.as_ref() {
            Some(id) => {
                if !seen_ids.insert(id.as_str().to_owned()) {
                    return scope_validation_rejection(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        "acceptance_criteria[].acceptance_criterion_id",
                        "acceptance criterion replacement IDs must not be duplicated",
                    );
                }
                let record = store
                    .acceptance_criterion_record(id.as_str())
                    .map_err(CorePipelineError::from)?;
                let Some(record) = record else {
                    return scope_validation_rejection(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        "acceptance_criteria[].acceptance_criterion_id",
                        "acceptance criterion replacement ID is unknown",
                    );
                };
                if record.task_id != request.task_id.as_str() {
                    return scope_validation_rejection(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        "acceptance_criteria[].acceptance_criterion_id",
                        "acceptance criterion replacement ID belongs to another Task",
                    );
                }
                if record.status != AcceptanceCriterionStatus::Active {
                    return scope_validation_rejection(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        "acceptance_criteria[].acceptance_criterion_id",
                        "retired acceptance criterion IDs cannot be reused",
                    );
                }
                id.clone()
            }
            None => {
                let id = allocate_acceptance_criterion_id(
                    service.durable_id_generator(),
                    store,
                    &seen_ids,
                )
                .map_err(PlanError::Core)?;
                seen_ids.insert(id.as_str().to_owned());
                id
            }
        };
        projected.push(AcceptanceCriterion {
            acceptance_criterion_id: acceptance_criterion_id.clone(),
            statement: statement.clone(),
            evidence_requirement: replacement.evidence_requirement,
        });
        upserts.push(AcceptanceCriterionUpsert {
            acceptance_criterion_id: acceptance_criterion_id.as_str().to_owned(),
            statement,
            evidence_requirement: replacement.evidence_requirement,
            position: position as u64,
        });
    }

    let changed = current != projected;
    Ok((
        projected,
        Some(AcceptanceCriteriaReplace {
            task_id: request.task_id.as_str().to_owned(),
            criteria: upserts,
        }),
        changed,
    ))
}

fn validate_requested_effect_contract(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &UpdateScopeRequest,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
) -> Result<(), PlanError> {
    if task.mode == TaskMode::Advisor {
        let compatible = match request.change_unit.operation {
            ChangeUnitOperation::KeepCurrent => current_change_unit.is_some_and(|change_unit| {
                advisor_compatible_change_unit(
                    &change_unit.bounded_paths,
                    change_unit.effect_contract.as_ref(),
                )
            }),
            ChangeUnitOperation::CreateCurrent | ChangeUnitOperation::ReplaceCurrent => {
                let affected_paths = request.change_unit.affected_paths();
                advisor_compatible_change_unit(
                    &affected_paths,
                    request.change_unit.effect_contract.as_ref(),
                )
            }
        };
        if !compatible {
            return scope_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "change_unit",
                "advisor Change Units must be observe-only and authorize no Product Repository path, Run, sensitive action, external network, or secret access",
            );
        }
    }
    let Some(contract) = request.change_unit.effect_contract.as_ref() else {
        return Ok(());
    };
    match validate_effect_contract(contract) {
        Ok(()) => {}
        Err(EffectContractValidationError::ConflictingEffect(_)) => {
            return scope_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "change_unit.effect_contract",
                "effect_contract cannot list the same effect as both allowed and forbidden",
            );
        }
        Err(EffectContractValidationError::EmptyText(field)) => {
            return scope_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                field,
                "effect_contract string list entries must not be empty",
            );
        }
    }

    observe_request_product_paths(
        &store.project_record().repo_root,
        &contract.allowed_paths,
        request.envelope.dry_run,
        Some(project_state.state_version),
        "change_unit.effect_contract.allowed_paths",
        "effect_contract.allowed_paths must be normalized relative Product Repository paths",
        "effect_contract.allowed_paths resolve outside the Product Repository",
    )
    .map(|_| ())
}

struct ValidatedScopeDecisions {
    resolution_refs: Vec<StateRecordRef>,
    applications: Vec<ShapingGapApplication>,
}

fn validate_related_scope_decisions(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &UpdateScopeRequest,
    current_change_unit: Option<&ChangeUnitRecord>,
    scope_revision: u64,
    now: &UtcTimestamp,
) -> Result<ValidatedScopeDecisions, PlanError> {
    let current_change_unit_id =
        current_change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    let mut transition_refs = vec![state_ref(
        StateRecordKind::Task,
        request.task_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(project_state.state_version),
    )];
    if let Some(current_change_unit) = current_change_unit {
        transition_refs.push(state_ref(
            StateRecordKind::ChangeUnit,
            &current_change_unit.change_unit_id,
            &request.envelope.project_id,
            Some(&request.task_id),
            Some(current_change_unit.basis_state_version),
        ));
    }
    let requirement = ScopeDecisionAuthorityRequirement {
        task_id: &request.task_id,
        scope_revision,
        current_change_unit_id: current_change_unit_id.as_ref(),
        affected_refs: &transition_refs,
        now,
    };
    let checkpoint = store
        .current_shaping_checkpoint(&request.task_id)
        .map_err(CorePipelineError::from)?;
    let expected_scope_gaps = checkpoint
        .as_ref()
        .map(|checkpoint| {
            checkpoint
                .gaps
                .iter()
                .filter(|gap| {
                    gap.status == ShapingGapStatus::Accepted
                        && gap.gap_kind.decision_policy().is_some_and(|policy| {
                            policy.application_owner == ShapingDecisionApplicationOwner::UpdateScope
                        })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if request.related_scope_decision_refs.len() != expected_scope_gaps.len() {
        return scope_validation_rejection(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "related_scope_decision_refs",
            "related scope decision refs must exactly match every accepted current scope gap",
        );
    }
    let mut linked_scope_decision_refs = Vec::new();
    let mut applications = Vec::new();
    let mut supplied_resolution_ids = BTreeSet::new();
    for related_ref in &request.related_scope_decision_refs {
        if related_ref.record_kind != StateRecordKind::UserActionResolution
            || related_ref.project_id != request.envelope.project_id
            || related_ref.task_id.as_ref() != Some(&request.task_id)
        {
            return scope_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "related_scope_decision_refs",
                "related scope decision refs must identify user-action resolutions for this Task",
            );
        }
        if !supplied_resolution_ids.insert(related_ref.record_id.as_str()) {
            return scope_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "related_scope_decision_refs",
                "related scope decision refs must be unique",
            );
        }
        let resolution = store
            .user_action_resolution_record(related_ref.record_id.as_str())
            .map_err(|error| store_error_plan(&request.envelope, project_state, error))?
            .ok_or_else(|| {
                PlanError::Response(Box::new(decision_rejected_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "related scope decision resolution is missing",
                )))
            })?;
        let record = store
            .user_action_record(resolution.user_action_request_id(), now)
            .map_err(CorePipelineError::from)?
            .ok_or_else(|| {
                PlanError::Response(Box::new(decision_rejected_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "related scope decision request is missing",
                )))
            })?;
        let authority = user_action_authority_from_record(&record).map_err(|error| {
            user_action_service_plan_error(&request.envelope, project_state, error)
        })?;
        let Some(checkpoint) = checkpoint.as_ref() else {
            return scope_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "related_scope_decision_refs",
                "scope decisions require one current shaping checkpoint",
            );
        };
        let matching_gap = expected_scope_gaps.iter().find(|gap| {
            gap.user_action.as_ref().is_some_and(|link| {
                link.user_action_request_id == record.request().user_action_request_id()
                    && link.user_action_resolution_id.as_deref()
                        == Some(related_ref.record_id.as_str())
            })
        });
        let metadata_matches = matching_gap.is_some_and(|gap| {
            matches!(
                record.request().metadata(),
                PersistedUserActionRequestMetadata::Shaping(metadata)
                    if metadata.shaping_checkpoint_id.as_str()
                        == checkpoint.shaping_checkpoint_id
                        && metadata.shaping_gap_id.as_str() == gap.shaping_gap_id
            )
        });
        let policy_matches = matching_gap.is_some_and(|gap| {
            gap.gap_kind.decision_policy().is_some_and(|policy| {
                policy.application_owner == ShapingDecisionApplicationOwner::UpdateScope
                    && policy.changes_scope_revision
                    && record.request().required_for() == policy.required_for
            })
        });
        if !accepted_current_scope_decision_authority(&authority, &requirement)
            || !metadata_matches
            || !policy_matches
        {
            return Err(PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "related scope decision resolution is not current",
            ))));
        }
        let gap = matching_gap.ok_or_else(|| {
            PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "related scope decision resolution is not linked to a current accepted scope gap",
            )))
        })?;
        applications.push(ShapingGapApplication {
            shaping_decision_application_id: shaping_decision_application_id(
                &volicord_types::ids::UserActionResolutionId::new(related_ref.record_id.as_str()),
                ShapingDecisionApplicationOwner::UpdateScope,
            )
            .map_err(CorePipelineError::from)?
            .into_inner(),
            shaping_gap_id: gap.shaping_gap_id.clone(),
            user_action_resolution_id: related_ref.record_id.as_str().to_owned(),
        });
        linked_scope_decision_refs.push(related_ref.clone());
    }
    if applications.len() != expected_scope_gaps.len() {
        return scope_validation_rejection(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "related_scope_decision_refs",
            "related scope decision refs do not cover the exact current accepted scope gaps",
        );
    }
    Ok(ValidatedScopeDecisions {
        resolution_refs: linked_scope_decision_refs,
        applications,
    })
}
