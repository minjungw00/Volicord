use std::collections::BTreeSet;

use serde_json::json;
use volicord_store::core_pipeline::{
    CoreProjectStore, CoreStorageMutation, CurrentShapingApplicationAuthority, ProjectStateHeader,
    ShapingAdvanceApplication, ShapingCheckpointGapInsert, ShapingCheckpointInsert,
    ShapingCheckpointMutation, ShapingCheckpointRecord, ShapingCheckpointUserActionInsert,
    ShapingGapApplication, ShapingStaleAuthorityDisposition, TaskCloseBasisUpdate, TaskMutation,
    TaskScopeUpdate,
};
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::{
    shaping_authority_reauthorization_id, shaping_decision_application_id,
    ShapingDecisionApplicationId,
};
use volicord_types::methods::{
    FinalizeAdviceRequest, FinalizeAdviceResultFields, MethodOperationCategory,
    RecordShapingCheckpointRequest, RecordShapingCheckpointResultFields,
};
use volicord_types::schema::{
    advisor_compatible_change_unit, CurrentCloseBasis, PersistedUserActionRequestMetadata,
    RequiredNullable, ResidualRisk, ShapingCheckpoint, ShapingCheckpointOperation, ShapingGapInput,
    StaleShapingAuthorityAction, StateRecordRef, WorkflowActionKey,
};
use volicord_types::values::{
    ErrorCode, MethodName, ShapingAuthorityReauthorizationOutcome, ShapingCheckpointReadiness,
    ShapingDecisionApplicationAuthorityStatus, ShapingDecisionApplicationOwner, ShapingGapStatus,
    StateRecordKind, TaskLifecyclePhase, TaskMode, UserActionBasisStatus, UserActionStatus,
    WorkPhase,
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
};
use crate::operation_plan::OperationPlan;
use crate::pipeline::{
    commit_mutation_branch, dry_run_preview_branch, CorePipelineError, CoreResult, CoreService,
    InvocationContext, PipelineResponse, TaskRequirement, VerifiedInvocationContext,
};
use crate::policy::workflow::project_workflow_policy;
use crate::state_summary::{project_state_header, state_summary, StateSummaryInput};

impl CoreService {
    /// Executes `volicord.record_shaping_checkpoint` as an authority-bearing aggregate mutation.
    pub fn record_shaping_checkpoint(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        request: RecordShapingCheckpointRequest,
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
                "envelope.task_id must match RecordShapingCheckpointRequest.task_id",
            );
        }
        let request_json = serde_json::to_value(&request)?;
        let prepared = match prepare_or_response(
            self,
            Some(context),
            MethodName::RecordShapingCheckpoint,
            Some(request.checkpoint_operation.semantic_variant()),
            request.envelope.clone(),
            request_json,
            invocation,
            mutation_method_policy(
                MethodName::RecordShapingCheckpoint,
                request.operation_category(),
                TaskRequirement::Exact(request.task_id.clone()),
                request.envelope.dry_run,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan = match plan_record_shaping_checkpoint(
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
                return self.complete_prepared_response(response, &prepared);
            }
        };
        if request.envelope.dry_run.is_requested() {
            return self.execute_prepared_request(
                prepared,
                dry_run_preview_branch::<RecordShapingCheckpointRequest>(dry_run_summary(
                    "shaping_checkpoint",
                    "commit",
                    "Shaping checkpoint and linked UserAction requests would be recorded atomically.",
                    Vec::new(),
                )),
            );
        }
        self.execute_prepared_request(
            prepared,
            commit_mutation_branch::<RecordShapingCheckpointRequest>(
                plan.operation
                    .into_commit_branch::<RecordShapingCheckpointRequest>(
                        plan.result_fields,
                        "shaping_recorded",
                    ),
            ),
        )
    }

    /// Executes `volicord.finalize_advice` as an authority-bearing aggregate mutation.
    pub fn finalize_advice(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        request: FinalizeAdviceRequest,
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
                "envelope.task_id must match FinalizeAdviceRequest.task_id",
            );
        }
        let request_json = serde_json::to_value(&request)?;
        let prepared = match prepare_or_response(
            self,
            Some(context),
            MethodName::FinalizeAdvice,
            Some(volicord_types::values::WorkflowActionSemanticVariant::FinalizeAdvice),
            request.envelope.clone(),
            request_json,
            invocation,
            mutation_method_policy(
                MethodName::FinalizeAdvice,
                request.operation_category(),
                TaskRequirement::Exact(request.task_id.clone()),
                request.envelope.dry_run,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan = match plan_finalize_advice(
            self,
            &prepared.store,
            &prepared.context.project_state,
            &request,
            &prepared.operation_now,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                let response =
                    plan_error_response(&request.envelope, &prepared.context.project_state, error)?;
                return self.complete_prepared_response(response, &prepared);
            }
        };
        if request.envelope.dry_run.is_requested() {
            return self.execute_prepared_request(
                prepared,
                dry_run_preview_branch::<FinalizeAdviceRequest>(dry_run_summary(
                    "advisor_advice",
                    "commit",
                    "Advisor decisions, result, and checkpoint-backed close basis would be recorded atomically.",
                    Vec::new(),
                )),
            );
        }
        self.execute_prepared_request(
            prepared,
            commit_mutation_branch::<FinalizeAdviceRequest>(
                plan.operation.into_commit_branch::<FinalizeAdviceRequest>(
                    plan.result_fields,
                    "advisor_advice_finalized",
                ),
            ),
        )
    }
}

struct RecordShapingCheckpointPlan {
    operation: OperationPlan,
    result_fields: RecordShapingCheckpointResultFields,
}

struct FinalizeAdvicePlan {
    operation: OperationPlan,
    result_fields: FinalizeAdviceResultFields,
}

#[derive(Clone)]
struct StaleAuthorityPlan {
    authority: CurrentShapingApplicationAuthority,
    outcome: ShapingAuthorityReauthorizationOutcome,
    successor_gap: Option<ShapingGapInput>,
}

fn plan_record_shaping_checkpoint(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: RecordShapingCheckpointRequest,
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &volicord_types::values::UtcTimestamp,
) -> Result<RecordShapingCheckpointPlan, PlanError> {
    let checkpoint_operation = &request.checkpoint_operation;
    let attempted_action_key = WorkflowActionKey::new(
        MethodName::RecordShapingCheckpoint,
        checkpoint_operation.semantic_variant(),
    )
    .expect("checkpoint operations have a canonical RecordShapingCheckpoint action key");
    let scope_revision = &request.scope_revision;
    let baseline_ref = &request.baseline_ref;
    let summary = &request.summary;
    let implementation_boundary = &request.implementation_boundary;
    let gaps = &request.gaps;
    let source_refs = &request.source_refs;
    let evidence_refs = &request.evidence_refs;
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
            "record_shaping_checkpoint is not allowed for the current Task mode and work phase",
            attempted_action_key,
            None,
            Vec::new(),
            false,
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
    let authority_graph = store
        .current_shaping_authority_graph(&request.task_id, operation_now)
        .map_err(CorePipelineError::from)?;
    let mut carried_applications = Vec::new();
    let mut stale_authority_plans = Vec::<StaleAuthorityPlan>::new();
    let predecessor_checkpoint_id = match checkpoint_operation {
        ShapingCheckpointOperation::CreateInitial => {
            if current_checkpoint.is_some() {
                return workflow_rejection_plan_error(
                    store,
                    project_state,
                    &request.envelope,
                    &request.task_id,
                    ErrorCode::ShapingCheckpointStale,
                    "create_initial requires that the Task have no current shaping checkpoint",
                    attempted_action_key,
                    None,
                    Vec::new(),
                    true,
                );
            }
            if !authority_graph.stale_recovery_obligations.is_empty() {
                return workflow_rejection_plan_error(
                    store,
                    project_state,
                    &request.envelope,
                    &request.task_id,
                    ErrorCode::ShapingCheckpointStale,
                    "stale shaping authority requires exact checkpoint replacement",
                    attempted_action_key,
                    None,
                    Vec::new(),
                    false,
                );
            }
            None
        }
        ShapingCheckpointOperation::ReplaceCurrent {
            expected_current_checkpoint_id,
            retired_non_authorizing_request_refs,
            carry_forward_application_refs,
            stale_authority_actions,
        } => {
            let Some(current) = current_checkpoint.as_ref() else {
                return workflow_rejection_plan_error(
                    store,
                    project_state,
                    &request.envelope,
                    &request.task_id,
                    ErrorCode::ShapingCheckpointStale,
                    "replace_current requires an exact current shaping checkpoint",
                    attempted_action_key,
                    None,
                    Vec::new(),
                    true,
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
                    attempted_action_key,
                    None,
                    Vec::new(),
                    true,
                );
            }
            let expected_application_ids = current
                .applications
                .iter()
                .filter(|application| {
                    application.authority_status
                        == volicord_types::values::ShapingDecisionApplicationAuthorityStatus::Current
                        && application.linked_checkpoint_id.as_deref()
                            == Some(current.shaping_checkpoint_id.as_str())
                })
                .map(|application| application.shaping_decision_application_id.clone())
                .collect::<BTreeSet<_>>();
            let mut supplied_application_ids = BTreeSet::new();
            for application_ref in carry_forward_application_refs {
                if application_ref.record_kind != StateRecordKind::ShapingDecisionApplication
                    || application_ref.project_id != request.envelope.project_id
                    || application_ref.task_id.as_ref() != Some(&request.task_id)
                    || application_ref.produced_at_state_version.as_ref()
                        != Some(&project_state.state_version)
                    || !supplied_application_ids
                        .insert(application_ref.record_id.as_str().to_owned())
                {
                    return shaping_validation(
                        &request,
                        project_state,
                        "checkpoint_operation.carry_forward_application_refs",
                        "carry-forward refs must be unique exact current Task shaping decision application refs",
                    );
                }
            }
            if supplied_application_ids != expected_application_ids {
                return workflow_rejection_plan_error(
                    store,
                    project_state,
                    &request.envelope,
                    &request.task_id,
                    ErrorCode::UserDecisionUnresolved,
                    "carry-forward refs must exactly match every current compatible shaping decision application",
                    attempted_action_key,
                    None,
                    Vec::new(),
                    true,
                );
            }
            if current.applications.iter().any(|application| {
                expected_application_ids.contains(&application.shaping_decision_application_id)
                    && gaps
                        .iter()
                        .any(|gap| gap.gap_kind.judgment_kind() == Some(application.judgment_kind))
            }) {
                return shaping_validation(
                    &request,
                    project_state,
                    "gaps",
                    "a successor gap cannot conflict with carried application authority",
                );
            }
            carried_applications = current
                .applications
                .iter()
                .filter(|application| {
                    expected_application_ids.contains(&application.shaping_decision_application_id)
                })
                .cloned()
                .collect();
            let expected_stale_ids = authority_graph
                .stale_recovery_obligations
                .iter()
                .map(|authority| {
                    authority
                        .application
                        .shaping_decision_application_id
                        .clone()
                })
                .collect::<BTreeSet<_>>();
            let mut supplied_stale_ids = BTreeSet::new();
            for action in stale_authority_actions {
                let (reference, outcome, successor_gap) = match action {
                    StaleShapingAuthorityAction::Retire {
                        stale_application_ref,
                    } => (
                        stale_application_ref,
                        ShapingAuthorityReauthorizationOutcome::Retired,
                        None,
                    ),
                    StaleShapingAuthorityAction::Reauthorize {
                        stale_application_ref,
                        successor_gap,
                    } => (
                        stale_application_ref,
                        ShapingAuthorityReauthorizationOutcome::Reissued,
                        Some(successor_gap.clone()),
                    ),
                };
                if reference.record_kind != StateRecordKind::ShapingDecisionApplication
                    || reference.project_id != request.envelope.project_id
                    || reference.task_id.as_ref() != Some(&request.task_id)
                    || reference.produced_at_state_version.as_ref()
                        != Some(&project_state.state_version)
                    || !supplied_stale_ids.insert(reference.record_id.as_str().to_owned())
                {
                    return shaping_validation(
                        &request,
                        project_state,
                        "checkpoint_operation.stale_authority_actions",
                        "stale authority actions must use unique exact current-state Task application refs",
                    );
                }
                let authority =
                    authority_graph
                        .stale_recovery_obligations
                        .iter()
                        .find(|authority| {
                            authority.application.shaping_decision_application_id
                                == reference.record_id.as_str()
                        });
                let Some(authority) = authority else {
                    return shaping_validation(
                        &request,
                        project_state,
                        "checkpoint_operation.stale_authority_actions",
                        "each stale authority action must identify one exact stale application",
                    );
                };
                if let Some(successor_gap) = successor_gap.as_ref() {
                    let policy = successor_gap.gap_kind.decision_policy_for_mode(task.mode);
                    if !successor_gap.gap_kind.is_user_owned()
                        || successor_gap.user_action.as_ref().is_none()
                        || successor_gap.gap_kind.judgment_kind()
                            != Some(authority.application.judgment_kind)
                        || policy.is_none_or(|policy| {
                            policy.application_owner != authority.application.application_owner
                        })
                    {
                        return shaping_validation(
                            &request,
                            project_state,
                            "checkpoint_operation.stale_authority_actions",
                            "reauthorization must preserve the stale judgment kind and application owner through a user-owned successor gap",
                        );
                    }
                }
                stale_authority_plans.push(StaleAuthorityPlan {
                    authority: authority.clone(),
                    outcome,
                    successor_gap,
                });
            }
            if supplied_stale_ids != expected_stale_ids {
                return workflow_rejection_plan_error(
                    store,
                    project_state,
                    &request.envelope,
                    &request.task_id,
                    ErrorCode::UserDecisionUnresolved,
                    "stale authority actions must exactly consume every stale shaping application",
                    attempted_action_key,
                    None,
                    Vec::new(),
                    true,
                );
            }
            let mut has_live_linked_decision = false;
            let mut recoverable_request_ids = BTreeSet::new();
            for gap in current
                .gaps
                .iter()
                .filter(|gap| gap.status != ShapingGapStatus::Applied && gap.user_action.is_some())
            {
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
                if record.request().basis_status() != UserActionBasisStatus::Current
                    || record.request().basis().compatibility_status()
                        != UserActionBasisStatus::Current
                {
                    return workflow_rejection_plan_error(
                        store,
                        project_state,
                        &request.envelope,
                        &request.task_id,
                        ErrorCode::UserDecisionUnresolved,
                        "the current checkpoint contains stale or superseded shaping authority",
                        attempted_action_key,
                        None,
                        Vec::new(),
                        false,
                    );
                }
                let recoverable = matches!(
                    (gap.status, record.status()),
                    (ShapingGapStatus::Rejected, UserActionStatus::Resolved)
                        | (ShapingGapStatus::Deferred, UserActionStatus::Resolved)
                        | (ShapingGapStatus::Current, UserActionStatus::Expired)
                );
                if recoverable {
                    recoverable_request_ids.insert(link.user_action_request_id.clone());
                } else {
                    has_live_linked_decision = true;
                }
            }
            if has_live_linked_decision {
                return workflow_rejection_plan_error(
                    store,
                    project_state,
                    &request.envelope,
                    &request.task_id,
                    ErrorCode::UserDecisionUnresolved,
                    "the current shaping checkpoint has live linked UserAction authority",
                    attempted_action_key,
                    None,
                    Vec::new(),
                    false,
                );
            }
            let mut supplied_request_ids = BTreeSet::new();
            for retired_ref in retired_non_authorizing_request_refs {
                if retired_ref.record_kind != StateRecordKind::UserActionRequest
                    || retired_ref.project_id != request.envelope.project_id
                    || retired_ref.task_id.as_ref() != Some(&request.task_id)
                    || retired_ref.produced_at_state_version.as_ref()
                        != Some(&project_state.state_version)
                    || !supplied_request_ids.insert(retired_ref.record_id.as_str().to_owned())
                {
                    return shaping_validation(
                            &request,
                            project_state,
                            "checkpoint_operation.retired_non_authorizing_request_refs",
                            "retired request refs must be unique exact current Task UserAction request refs",
                        );
                }
            }
            if supplied_request_ids != recoverable_request_ids {
                return workflow_rejection_plan_error(
                        store,
                        project_state,
                        &request.envelope,
                        &request.task_id,
                        ErrorCode::UserDecisionUnresolved,
                        "retired request refs must exactly match every rejected, deferred, or expired predecessor decision",
                        attempted_action_key,
                        None,
                        Vec::new(),
                        true,
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
        let message = if task_baseline.is_none() && baseline_ref.is_some() {
            "Expected baseline_ref=null. Received a non-null BaselineRef. The Task state is valid; retry against the current action form."
                .to_owned()
        } else {
            "baseline_ref must match the current Task baseline; retry against the current action form"
                .to_owned()
        };
        return crate::method_rejection::authority_basis_mismatch_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "baseline_ref",
            task_baseline.map_or(
                volicord_types::schema::AuthorityBasisValue::Null(()),
                |baseline| {
                    volicord_types::schema::AuthorityBasisValue::String(
                        baseline.as_str().to_owned(),
                    )
                },
            ),
            baseline_ref.as_ref().map_or(
                volicord_types::schema::AuthorityBasisValue::Null(()),
                |baseline| {
                    volicord_types::schema::AuthorityBasisValue::String(
                        baseline.as_str().to_owned(),
                    )
                },
            ),
            message,
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
                    "record_shaping_checkpoint requires an idempotency key",
                )
                .expect("validation response serializes"),
            ))
        })?;

    let mut reserved_gap_ids = BTreeSet::new();
    let mut mutations = Vec::new();
    let mut gap_inserts = Vec::new();
    let mut projected_gaps = Vec::new();
    let mut created_request_refs = Vec::new();
    let mut materialized_stale_dispositions = stale_authority_plans
        .iter()
        .filter(|plan| plan.outcome == ShapingAuthorityReauthorizationOutcome::Retired)
        .map(|plan| ShapingStaleAuthorityDisposition {
            stale_application_id: plan
                .authority
                .application
                .shaping_decision_application_id
                .clone(),
            stale_user_action_request_id: plan.authority.application.user_action_request_id.clone(),
            outcome: plan.outcome,
            successor_gap_id: None,
            successor_user_action_request_id: None,
        })
        .collect::<Vec<_>>();
    let mut planned_gaps = gaps
        .iter()
        .cloned()
        .map(|gap| (gap, None))
        .collect::<Vec<_>>();
    planned_gaps.extend(stale_authority_plans.iter().filter_map(|plan| {
        plan.successor_gap.clone().map(|gap| {
            (
                gap,
                Some(
                    plan.authority
                        .application
                        .shaping_decision_application_id
                        .clone(),
                ),
            )
        })
    }));
    if carried_applications.iter().any(|application| {
        planned_gaps
            .iter()
            .any(|(gap, _)| gap.gap_kind.judgment_kind() == Some(application.judgment_kind))
    }) {
        return shaping_validation(
            &request,
            project_state,
            "gaps",
            "a successor gap cannot conflict with carried application authority",
        );
    }
    for (gap, reauthorizes_application_id) in &planned_gaps {
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
                    reauthorizes_application_id: reauthorizes_application_id
                        .as_ref()
                        .map(|id| ShapingDecisionApplicationId::new(id.clone())),
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
            if let Some(stale_application_id) = reauthorizes_application_id.as_ref() {
                materialized_stale_dispositions.push(ShapingStaleAuthorityDisposition {
                    stale_application_id: stale_application_id.clone(),
                    stale_user_action_request_id: stale_authority_plans
                        .iter()
                        .find(|plan| {
                            plan.authority.application.shaping_decision_application_id
                                == *stale_application_id
                        })
                        .map(|plan| plan.authority.application.user_action_request_id.clone())
                        .ok_or_else(|| CorePipelineError::Invariant {
                            detail: "a reauthorization gap lost its stale authority source"
                                .to_owned(),
                        })?,
                    outcome: ShapingAuthorityReauthorizationOutcome::Reissued,
                    successor_gap_id: Some(gap_id.as_str().to_owned()),
                    successor_user_action_request_id: Some(
                        materialized
                            .public_request
                            .user_action_request_id
                            .as_str()
                            .to_owned(),
                    ),
                });
            }
            mutations.push(materialized.mutation);
        }
        gap_inserts.push(ShapingCheckpointGapInsert {
            shaping_gap_id: gap_id.as_str().to_owned(),
            gap_kind: gap.gap_kind,
            summary: gap.summary.clone(),
            affected_refs: gap.affected_refs.clone(),
            reauthorizes_application_id: reauthorizes_application_id.clone(),
            user_action: user_action_insert,
        });
        projected_gaps.push(volicord_store::core_pipeline::ShapingCheckpointGapRecord {
            shaping_gap_id: gap_id.as_str().to_owned(),
            gap_kind: gap.gap_kind,
            summary: gap.summary.clone(),
            affected_refs: gap.affected_refs.clone(),
            status: ShapingGapStatus::Current,
            reauthorizes_application_id: reauthorizes_application_id.clone(),
            user_action: projected_user_action,
        });
    }
    let readiness = if planned_gaps.is_empty()
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
        retired_non_authorizing_request_ids: match checkpoint_operation {
            ShapingCheckpointOperation::CreateInitial => Vec::new(),
            ShapingCheckpointOperation::ReplaceCurrent {
                retired_non_authorizing_request_refs,
                ..
            } => retired_non_authorizing_request_refs
                .iter()
                .map(|reference| reference.record_id.as_str().to_owned())
                .collect(),
        },
        carry_forward_application_ids: match checkpoint_operation {
            ShapingCheckpointOperation::CreateInitial => Vec::new(),
            ShapingCheckpointOperation::ReplaceCurrent {
                carry_forward_application_refs,
                ..
            } => carry_forward_application_refs
                .iter()
                .map(|reference| reference.record_id.as_str().to_owned())
                .collect(),
        },
        stale_authority_dispositions: materialized_stale_dispositions.clone(),
        gaps: gap_inserts,
    };
    mutations.push(CoreStorageMutation::Shaping(
        ShapingCheckpointMutation::Record(Box::new(checkpoint_insert)),
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
    for application in &mut carried_applications {
        application.carried_from_checkpoint_id = application.linked_checkpoint_id.clone();
        application.linked_checkpoint_id = Some(checkpoint_id.as_str().to_owned());
    }
    let mut projected_applications = carried_applications.clone();
    projected_applications.extend(stale_authority_plans.iter().map(|plan| {
        let mut application = plan.authority.application.clone();
        application.authority_status = ShapingDecisionApplicationAuthorityStatus::Superseded;
        application.superseded_at = Some(operation_now.clone());
        application.linked_checkpoint_id = None;
        application.carried_from_checkpoint_id = None;
        application
    }));
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
        applications: projected_applications,
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
    )?;
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
    let carried_application_refs = carried_applications
        .iter()
        .map(|application| {
            crate::record_refs::state_ref(
                StateRecordKind::ShapingDecisionApplication,
                &application.shaping_decision_application_id,
                &request.envelope.project_id,
                Some(&request.task_id),
                Some(planned_state_version),
            )
        })
        .collect::<Vec<_>>();
    let shaping_authority_reauthorization_refs = stale_authority_plans
        .iter()
        .map(|plan| {
            let application_id = ShapingDecisionApplicationId::new(
                plan.authority
                    .application
                    .shaping_decision_application_id
                    .clone(),
            );
            let lineage_id =
                shaping_authority_reauthorization_id(&application_id).map_err(|_| {
                    CorePipelineError::Invariant {
                        detail: "stale authority lineage identity could not be derived".to_owned(),
                    }
                })?;
            Ok(crate::record_refs::state_ref(
                StateRecordKind::ShapingAuthorityReauthorization,
                lineage_id.as_str(),
                &request.envelope.project_id,
                Some(&request.task_id),
                Some(planned_state_version),
            ))
        })
        .collect::<CoreResult<Vec<_>>>()?;
    let result_fields = RecordShapingCheckpointResultFields {
        shaping_checkpoint,
        created_user_action_request_refs: created_request_refs,
        shaping_authority_reauthorization_refs: shaping_authority_reauthorization_refs.clone(),
        workflow,
        state,
    };
    Ok(RecordShapingCheckpointPlan {
        operation: OperationPlan::new(
            request.task_id,
            current_change_unit
                .as_ref()
                .map(|cu| volicord_types::ids::ChangeUnitId::new(cu.change_unit_id.clone())),
            mutations,
            object_from_value(json!({
                "shaping_checkpoint_id": checkpoint_id,
                "readiness": readiness,
                "carried_shaping_decision_application_ids": carried_applications
                    .iter()
                    .map(|application| application.shaping_decision_application_id.clone())
                    .collect::<Vec<_>>(),
                "carried_shaping_decision_application_refs": carried_application_refs,
                "shaping_authority_reauthorization_refs": shaping_authority_reauthorization_refs,
            }))?,
        ),
        result_fields,
    })
}

trait ShapingValidationRequest {
    fn envelope(&self) -> &volicord_types::schema::ToolEnvelope;
}

impl ShapingValidationRequest for RecordShapingCheckpointRequest {
    fn envelope(&self) -> &volicord_types::schema::ToolEnvelope {
        &self.envelope
    }
}

impl ShapingValidationRequest for FinalizeAdviceRequest {
    fn envelope(&self) -> &volicord_types::schema::ToolEnvelope {
        &self.envelope
    }
}

fn shaping_validation<T>(
    request: &impl ShapingValidationRequest,
    project_state: &ProjectStateHeader,
    field: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    Err(PlanError::Response(Box::new(
        validation_rejected(
            request.envelope().dry_run,
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
    request: &FinalizeAdviceRequest,
    operation_now: &volicord_types::values::UtcTimestamp,
) -> Result<FinalizeAdvicePlan, PlanError> {
    let shaping_checkpoint_id = &request.shaping_checkpoint_id;
    let change_unit_id = &request.change_unit_id;
    let scope_revision = &request.scope_revision;
    let baseline_ref = &request.baseline_ref;
    let user_action_resolution_ids = &request.user_action_resolution_ids;
    let result_summary = &request.result_summary;
    let result_refs = &request.result_refs;
    let evidence_refs = &request.evidence_refs;
    let risk_inputs = &request.residual_risks;
    let recovery_constraints = &request.recovery_constraints;
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
            MethodName::FinalizeAdvice,
            None,
            Vec::new(),
            false,
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
            "result_summary",
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
            "recovery_constraints",
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
            MethodName::FinalizeAdvice,
            None,
            Vec::new(),
            true,
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
            MethodName::FinalizeAdvice,
            None,
            Vec::new(),
            true,
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
    if task_wide_authority.blocks_advance_application() {
        return workflow_rejection_plan_error(
            store,
            project_state,
            &request.envelope,
            &request.task_id,
            ErrorCode::UserDecisionUnresolved,
            "task-wide UserAction authority required for advisor finalization is not accepted",
            MethodName::FinalizeAdvice,
            None,
            Vec::new(),
            false,
        );
    }

    let mut applications = Vec::new();
    let mut expected_resolution_ids = checkpoint
        .applications
        .iter()
        .filter(|application| {
            application.authority_status
                == volicord_types::values::ShapingDecisionApplicationAuthorityStatus::Current
                && application.linked_checkpoint_id.as_deref()
                    == Some(checkpoint.shaping_checkpoint_id.as_str())
        })
        .map(|application| application.user_action_resolution_id.clone())
        .collect::<BTreeSet<_>>();
    let mut application_refs = checkpoint
        .applications
        .iter()
        .filter(|application| {
            application.authority_status
                == volicord_types::values::ShapingDecisionApplicationAuthorityStatus::Current
                && application.linked_checkpoint_id.as_deref()
                    == Some(checkpoint.shaping_checkpoint_id.as_str())
        })
        .map(|application| {
            crate::record_refs::state_ref(
                StateRecordKind::ShapingDecisionApplication,
                &application.shaping_decision_application_id,
                &request.envelope.project_id,
                Some(&request.task_id),
                Some(project_state.state_version + 1),
            )
        })
        .collect::<Vec<_>>();
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
                MethodName::FinalizeAdvice,
                None,
                Vec::new(),
                true,
            );
        }
        if policy.application_owner == ShapingDecisionApplicationOwner::FinalizeAdvice
            && !matches!(
                gap.status,
                ShapingGapStatus::Accepted | ShapingGapStatus::Applied
            )
        {
            return workflow_rejection_plan_error(
                store,
                project_state,
                &request.envelope,
                &request.task_id,
                ErrorCode::UserDecisionUnresolved,
                "every advisor-owned decision must be accepted before finalization",
                MethodName::FinalizeAdvice,
                None,
                Vec::new(),
                true,
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
                MethodName::FinalizeAdvice,
                None,
                Vec::new(),
                false,
            );
        }
        expected_resolution_ids.insert(resolution_id.clone());
        if policy.application_owner == ShapingDecisionApplicationOwner::FinalizeAdvice
            && gap.status == ShapingGapStatus::Accepted
        {
            let application_id = shaping_decision_application_id(
                &volicord_types::ids::UserActionResolutionId::new(resolution_id),
                ShapingDecisionApplicationOwner::FinalizeAdvice,
            )
            .map_err(CorePipelineError::from)?
            .into_inner();
            application_refs.push(crate::record_refs::state_ref(
                StateRecordKind::ShapingDecisionApplication,
                &application_id,
                &request.envelope.project_id,
                Some(&request.task_id),
                Some(project_state.state_version + 1),
            ));
            applications.push(ShapingGapApplication {
                shaping_decision_application_id: application_id,
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
            "user_action_resolution_ids",
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
                "residual_risks",
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
        shaping_decision_application_refs: application_refs.clone(),
        updated_at: operation_now.clone(),
    };
    let mut projected_checkpoint = checkpoint.clone();
    crate::workflow_projection::apply_projected_shaping_applications(
        &mut projected_checkpoint,
        &applications,
        ShapingDecisionApplicationOwner::FinalizeAdvice,
        *scope_revision,
        baseline_ref,
        Some(change_unit_id),
        operation_now,
    )?;
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
    )?;
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
    Ok(FinalizeAdvicePlan {
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
                "applied_shaping_decision_application_ids": applications
                    .iter()
                    .map(|application| application.shaping_decision_application_id.clone())
                    .collect::<Vec<_>>(),
                "applied_shaping_decision_application_refs": applications
                    .iter()
                    .map(|application| crate::record_refs::state_ref(
                        StateRecordKind::ShapingDecisionApplication,
                        &application.shaping_decision_application_id,
                        &request.envelope.project_id,
                        Some(&request.task_id),
                        Some(planned_state_version),
                    ))
                    .collect::<Vec<_>>(),
                "close_basis_revision": projected_task.close_basis_revision,
            }))?,
        ),
        result_fields: FinalizeAdviceResultFields {
            shaping_checkpoint,
            applied_shaping_decision_application_refs: applications
                .iter()
                .map(|application| {
                    crate::record_refs::state_ref(
                        StateRecordKind::ShapingDecisionApplication,
                        &application.shaping_decision_application_id,
                        &request.envelope.project_id,
                        Some(&request.task_id),
                        Some(planned_state_version),
                    )
                })
                .collect(),
            workflow,
            state,
        },
    })
}

fn validate_advisor_refs(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &FinalizeAdviceRequest,
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
                "result_refs",
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
                "result_refs",
                "advisor refs must identify current supported artifact, evidence, or Change Unit state",
            );
        }
    }
    Ok(())
}
