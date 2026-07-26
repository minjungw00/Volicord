use super::close_blockers::{normalize_close_blockers, open_write_ticket_close_blocker};
use super::close_readiness::{facts_from_projection, plan_projected_close_readiness};
use super::{
    acceptance_policy_storage, active_acceptance_criteria_for_task, allocate_write_ticket_id,
    baseline_matches, build_state_summary, change_unit_effect_contract, change_unit_ref,
    decode_required_json, guarantee_display_for_invocation, infallible_rejected_pipeline_response,
    matching_sensitive_approval, object_from_value, parse_acceptance_policy,
    parse_owner_storage_value, parse_task_mode, parse_work_phase, paths_match_current_change_unit,
    pending_user_action_authorities_for_plan, plan_error_response, prepare_or_response,
    project_state_projection, projected_close_basis, projected_evidence_summary,
    record_core_workflow_metric_best_effort, rejected_pipeline_response,
    resolve_prepare_write_task, response_committed_fresh_effect, state_ref, state_ref_from_stored,
    store_error_response, user_action_authority_from_record, validate_prepare_write_change_unit,
    validation_rejected, workspace_context_matches, write_ticket_summary_for_record,
    PersistedWriteTicketAttemptScope, PlanError, PrepareWritePlan, SensitiveApprovalSearch,
    SummaryBuild,
};
use crate::pipeline::{
    tool_error, CorePipelineError, CoreResult, CoreService, FreshnessPolicy, InvocationContext,
    MethodEffectPolicy, MethodPolicy, OwnerPipelineBranch, PipelineResponse, ReplayPolicy,
    TaskRequirement, VerifiedInvocationContext,
};
use crate::policy::effect_contract::{product_write_violations, EffectContractViolation};
use crate::policy::path::{normalize_product_paths, path_is_within, ProductPathError};
use crate::policy::user_action_relevance::{
    user_action_blocks_operation, UserActionOperation, UserActionOperationContext,
};
use crate::policy::workflow::{
    acceptance_policy_for_control, parse_task_control_level, project_workflow_policy,
    resolve_task_control_authority, ProjectWorkflowPolicy,
};
use crate::policy::write_ticket::{
    current_sensitive_approval, normalized_string_set, prepare_write_decision,
    prepare_write_dry_run_summary, write_decision_reason, write_ticket_is_idle_expired,
    SensitiveApprovalRequirement,
};
use chrono::Duration;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, CoreStorageMutation, ProjectStateHeader,
    TaskControlLevelUpdate, TaskMutation, TaskRecord, WriteTicketByIdInvalidation,
    WriteTicketInsert, WriteTicketMutation, WriteTicketRecord,
};
use volicord_store::diagnostics::{
    record_core_rejection_diagnostic, CoreRejectionDiagnostic, CoreRejectionReason,
    WorkflowMetricKind,
};
use volicord_store::error::StoreError;
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::{ChangeUnitId, TaskId, WriteTicketId};
use volicord_types::methods::{
    MethodOperationCategory, PrepareWriteRequest, PrepareWriteResultFields,
};
use volicord_types::schema::{
    DryRunSummary, GuaranteeDisplay, JsonObject, StateRecordRef, WriteDecisionReason, WriteTicket,
    WriteTicketAttemptScope, WriteTicketPathPatterns, WriteTicketScope, WriteTicketValidityBasis,
};
use volicord_types::values::{
    AcceptancePolicy, CloseState, ErrorCode, MethodName, PrepareWriteDecision, StateRecordKind,
    TaskControlLevel, TaskMode, UserActionKind, UserActionRequiredFor, UtcTimestamp, WorkPhase,
    WriteDecisionCategory, WriteTicketEffect, WriteTicketInvalidationReason, WriteTicketState,
};

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

        if request.envelope.dry_run {
            return self.execute_prepared_request::<PrepareWriteResultFields>(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: plan.dry_run_summary,
                },
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
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: plan.event_kind,
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: Some(plan.change_unit_id),
                storage_mutations: plan.storage_mutations,
            },
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

    if request.envelope.dry_run {
        MethodPolicy::exact(
            request.operation_category(),
            task,
            ReplayPolicy::None,
            FreshnessPolicy::IfPresent,
            MethodEffectPolicy::DryRunPreview,
        )
    } else {
        MethodPolicy::exact(
            request.operation_category(),
            task,
            ReplayPolicy::Committed,
            FreshnessPolicy::IfPresent,
            MethodEffectPolicy::CoreMutation,
        )
    }
}

struct PrepareWriteRawRequest {
    request: PrepareWriteRequest,
    plan_now: UtcTimestamp,
}

impl PrepareWriteRawRequest {
    fn new(request: PrepareWriteRequest, operation_now: &UtcTimestamp) -> Self {
        Self {
            request,
            plan_now: operation_now.clone(),
        }
    }
}

struct PrepareWriteNormalizedRequest {
    raw: PrepareWriteRawRequest,
    planned_state_version: u64,
    intended_operation: String,
    intended_paths: Vec<String>,
    sensitive_categories: Vec<String>,
}

struct PrepareWriteResolvedContext {
    normalized: PrepareWriteNormalizedRequest,
    task_id: TaskId,
    task: TaskRecord,
    change_unit: ChangeUnitRecord,
    reasons: Vec<WriteDecisionReason>,
}

struct PrepareWritePolicyDecision {
    normalized: PrepareWriteNormalizedRequest,
    task_id: TaskId,
    task: TaskRecord,
    change_unit: ChangeUnitRecord,
    reasons: Vec<WriteDecisionReason>,
    workflow_policy: ProjectWorkflowPolicy,
    sensitive_approval_required: bool,
    control_mutations: Vec<CoreStorageMutation>,
}

struct PrepareWritePlannedMutations {
    request: PrepareWriteRequest,
    planned_state_version: u64,
    plan_now: UtcTimestamp,
    task_id: TaskId,
    task: TaskRecord,
    change_unit: ChangeUnitRecord,
    reasons: Vec<WriteDecisionReason>,
    decision: PrepareWriteDecision,
    allowed: bool,
    pending_user_action_refs: Vec<StateRecordRef>,
    active_user_action_refs: Vec<StateRecordRef>,
    guarantee_display: Option<GuaranteeDisplay>,
    write_ticket_id: Option<WriteTicketId>,
    write_ticket_ref: Option<StateRecordRef>,
    planned_write_ticket_record: Option<WriteTicketRecord>,
    idle_expires_at: Option<UtcTimestamp>,
    write_ticket_effect: WriteTicketEffect,
    allowed_path_patterns: Vec<String>,
    denied_path_patterns: Vec<String>,
    storage_mutations: Vec<CoreStorageMutation>,
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

fn resolve_prepare_write_context(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    normalized: PrepareWriteNormalizedRequest,
) -> Result<PrepareWriteResolvedContext, PlanError> {
    let request = &normalized.raw.request;
    let (task_id, task, mut reasons) = resolve_prepare_write_task(store, project_state, request)?;
    let change_unit = match store.current_change_unit(&task_id).map_err(|error| {
        PlanError::Response(Box::new(store_error_response(
            &request.envelope,
            project_state,
            error,
        )))
    })? {
        Some(change_unit) => change_unit,
        None => {
            let _ = record_core_rejection_diagnostic(
                store
                    .mutation_context()
                    .expect("prepare_write planning retains a mutation context"),
                CoreRejectionDiagnostic {
                    project_id: request.envelope.project_id.as_str(),
                    task_id: task_id.as_str(),
                    method_name: MethodName::PrepareWrite,
                    reason: CoreRejectionReason::CurrentChangeUnitRequired,
                    occurred_at: &normalized.raw.plan_now,
                },
            );
            return Err(PlanError::Response(Box::new(
                prepare_write_change_unit_required_response(request, project_state, &task_id),
            )));
        }
    };
    validate_prepare_write_change_unit(request, &task_id, &change_unit, &mut reasons);

    Ok(PrepareWriteResolvedContext {
        normalized,
        task_id,
        task,
        change_unit,
        reasons,
    })
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
    let raw = PrepareWriteRawRequest::new(request, operation_now);
    let normalized = normalize_prepare_write_request(store, project_state, raw)?;
    let resolved = resolve_prepare_write_context(store, project_state, normalized)?;
    let policy = decide_prepare_write_policy(store, project_state, resolved)?;
    let mutations =
        plan_prepare_write_mutations(service, store, project_state, verified_invocation, policy)?;
    Ok(project_prepare_write_response(store, project_state, mutations)?.into_plan())
}

fn normalize_prepare_write_request(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    raw: PrepareWriteRawRequest,
) -> Result<PrepareWriteNormalizedRequest, PlanError> {
    if raw.request.intended_operation.trim().is_empty() {
        return prepare_write_validation_error(
            raw.request.envelope.dry_run,
            project_state.state_version,
            "intended_operation",
            "intended_operation must not be empty",
        );
    }
    let intended_operation = raw.request.intended_operation.trim().to_owned();
    let sensitive_categories = normalized_string_set(&raw.request.sensitive_categories);
    let intended_paths = match normalize_product_paths(
        &store.project_record().repo_root,
        &raw.request.intended_paths,
    ) {
        Ok(paths) => paths,
        Err(ProductPathError::Invalid) => {
            return prepare_write_validation_error(
                raw.request.envelope.dry_run,
                project_state.state_version,
                "intended_paths",
                "intended_paths must be relative Product Repository paths that stay inside the repository",
            );
        }
        Err(ProductPathError::LocalAccess) => {
            let response = rejected_pipeline_response(
                raw.request.envelope.dry_run,
                Some(project_state.state_version),
                vec![tool_error(
                    ErrorCode::InvocationContextMismatch,
                    "intended_paths resolve outside the Product Repository",
                    false,
                    None,
                )],
            )
            .map_err(PlanError::Core)?;
            return Err(PlanError::Response(Box::new(response)));
        }
    };
    Ok(PrepareWriteNormalizedRequest {
        raw,
        planned_state_version: project_state.state_version + 1,
        intended_operation,
        intended_paths,
        sensitive_categories,
    })
}

fn decide_prepare_write_policy(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    resolved: PrepareWriteResolvedContext,
) -> Result<PrepareWritePolicyDecision, PlanError> {
    let PrepareWriteResolvedContext {
        normalized,
        task_id,
        mut task,
        change_unit,
        reasons,
    } = resolved;
    let dry_run = normalized.raw.request.envelope.dry_run;
    if parse_task_mode(&task.mode)? == TaskMode::Advisor {
        return prepare_write_validation_error(
            dry_run,
            project_state.state_version,
            "task_id",
            "advisor Task mode does not support write preparation",
        );
    }
    let workflow_policy = project_workflow_policy(store).map_err(CorePipelineError::from)?;
    let current_control =
        parse_task_control_level(&task.effective_control_level).map_err(CorePipelineError::from)?;
    let current_acceptance = parse_acceptance_policy(&task.acceptance_policy)?;
    let resolved_control =
        resolve_task_control_authority(&task, &workflow_policy).map_err(CorePipelineError::from)?;
    let resolved_base_control = resolved_control.effective_control_level;
    let mut next_control = resolved_base_control;
    if next_control == TaskControlLevel::Observe {
        return prepare_write_validation_error(
            dry_run,
            project_state.state_version,
            "task_id",
            "observe control does not permit product write preparation",
        );
    }
    let has_policy_denied_path = workflow_policy.has_denied_path(&normalized.intended_paths);
    if next_control == TaskControlLevel::Light
        && !workflow_policy.light_paths_are_allowed(&normalized.intended_paths)
    {
        next_control = TaskControlLevel::Tracked;
    }
    if has_policy_denied_path || !normalized.sensitive_categories.is_empty() {
        next_control = TaskControlLevel::Sensitive;
    }
    let control_acceptance = acceptance_policy_for_control(next_control, &workflow_policy);
    let next_acceptance = if acceptance_policy_rank(resolved_control.acceptance_policy)
        >= acceptance_policy_rank(control_acceptance)
    {
        resolved_control.acceptance_policy
    } else {
        control_acceptance
    };
    let acceptance_raised =
        acceptance_policy_rank(next_acceptance) > acceptance_policy_rank(current_acceptance);
    let control_raised = next_control > current_control;
    let next_control_reason = if control_raised {
        if has_policy_denied_path {
            "Core raised control to `sensitive` because an intended path matches a denied project-policy prefix."
                .to_owned()
        } else if next_control == TaskControlLevel::Sensitive {
            "Core raised control to `sensitive` for declared sensitive write effects.".to_owned()
        } else if resolved_control.pending_policy_reevaluation
            && next_control == resolved_base_control
        {
            resolved_control.control_level_reason.clone()
        } else if current_control == TaskControlLevel::Light
            && next_control == TaskControlLevel::Tracked
        {
            "Core raised control to `tracked` because intended paths exceed the Light project policy."
                .to_owned()
        } else {
            resolved_control.control_level_reason.clone()
        }
    } else {
        task.control_level_reason.clone()
    };
    let next_acceptance_reason = if acceptance_raised
        && next_control == resolved_base_control
        && resolved_control.acceptance_raised
    {
        resolved_control.acceptance_policy_reason.clone()
    } else {
        format!(
            "Effective control `{}` requires final acceptance for the current close basis.",
            next_control.as_str()
        )
    };
    let mut control_mutations = Vec::new();
    if control_raised || acceptance_raised || resolved_control.policy_reevaluation_marked {
        control_mutations.push(CoreStorageMutation::Task(TaskMutation::UpdateControlLevel(
            TaskControlLevelUpdate {
                task_id: task.task_id.clone(),
                effective_control_level: next_control.as_str().to_owned(),
                control_level_reason: next_control_reason.clone(),
                acceptance_policy: acceptance_raised
                    .then(|| acceptance_policy_storage(next_acceptance).to_owned()),
                acceptance_policy_reason: acceptance_raised.then(|| next_acceptance_reason.clone()),
            },
        )));
        task.effective_control_level = next_control.as_str().to_owned();
        task.control_level_reason = next_control_reason;
        if acceptance_raised {
            task.acceptance_policy = acceptance_policy_storage(next_acceptance).to_owned();
            task.acceptance_policy_reason = next_acceptance_reason;
        }
    }
    if parse_work_phase(&task.work_phase)? != WorkPhase::Implementation {
        return prepare_write_validation_error(
            dry_run,
            project_state.state_version,
            "task_id",
            "write preparation requires work_phase=implementation",
        );
    }
    Ok(PrepareWritePolicyDecision {
        normalized,
        task_id,
        task,
        change_unit,
        reasons,
        workflow_policy,
        sensitive_approval_required: next_control == TaskControlLevel::Sensitive,
        control_mutations,
    })
}

fn plan_prepare_write_mutations(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    policy: PrepareWritePolicyDecision,
) -> Result<PrepareWritePlannedMutations, PlanError> {
    let PrepareWritePolicyDecision {
        normalized,
        task_id,
        task,
        change_unit,
        mut reasons,
        workflow_policy,
        sensitive_approval_required,
        control_mutations,
    } = policy;
    let PrepareWriteNormalizedRequest {
        raw,
        planned_state_version,
        intended_operation: normalized_operation,
        intended_paths: normalized_paths,
        sensitive_categories: normalized_sensitive_categories,
    } = normalized;
    let PrepareWriteRawRequest { request, plan_now } = raw;
    if request.product_file_write_intended == normalized_paths.is_empty() {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::WriteCompatibility,
            "product_write_flag_mismatch",
            "product_file_write_intended must match the intended Product Repository paths.",
            Vec::new(),
        ));
    }

    if !workspace_context_matches(&change_unit, verified_invocation)? {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Workspace,
            "workspace_context_mismatch",
            "The current Git workspace does not match the Change Unit baseline context.",
            vec![change_unit_ref(
                &request.envelope.project_id,
                &task_id,
                &change_unit,
                project_state.state_version,
            )],
        ));
    }
    if !baseline_matches(&change_unit, &task, &request.baseline_ref)? {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Baseline,
            "baseline_mismatch",
            "baseline_ref does not match the current write-compatibility basis.",
            vec![change_unit_ref(
                &request.envelope.project_id,
                &task_id,
                &change_unit,
                project_state.state_version,
            )],
        ));
    }

    if !paths_match_current_change_unit(
        &store.project_record().repo_root,
        &normalized_paths,
        &change_unit,
    )? {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::Scope,
            "path_out_of_scope",
            "One or more intended paths are outside the current Change Unit path scope.",
            vec![change_unit_ref(
                &request.envelope.project_id,
                &task_id,
                &change_unit,
                project_state.state_version,
            )],
        ));
    }

    if let Some(contract) = change_unit_effect_contract(&change_unit)? {
        let contract_violations = product_write_violations(
            &store.project_record().repo_root,
            &contract,
            request.product_file_write_intended,
            &normalized_paths,
        )
        .map_err(|_| {
            CorePipelineError::Store(StoreError::corrupt_owner_state_json(
                "change_units",
                change_unit.change_unit_id.clone(),
                "effect_contract_json",
            ))
        })?;
        for violation in contract_violations {
            reasons.push(effect_contract_reason(
                violation,
                change_unit_ref(
                    &request.envelope.project_id,
                    &task_id,
                    &change_unit,
                    project_state.state_version,
                ),
            ));
        }
    }

    let current_change_unit_id = ChangeUnitId::new(change_unit.change_unit_id.clone());
    let task_ref = state_ref(
        StateRecordKind::Task,
        task_id.as_str(),
        &request.envelope.project_id,
        Some(&task_id),
        Some(project_state.state_version),
    );
    let operation_refs = vec![
        task_ref.clone(),
        change_unit_ref(
            &request.envelope.project_id,
            &task_id,
            &change_unit,
            project_state.state_version,
        ),
    ];
    let sensitive_requirement = if !sensitive_approval_required {
        None
    } else {
        Some(SensitiveApprovalRequirement {
            task_id: &task_id,
            change_unit_id: &current_change_unit_id,
            scope_revision: task.scope_revision,
            operation: &normalized_operation,
            normalized_paths: &normalized_paths,
            sensitive_categories: &normalized_sensitive_categories,
            baseline_ref: Some(&request.baseline_ref),
            required_for: UserActionRequiredFor::PrepareWrite,
            now: &plan_now,
            repo_root: &store.project_record().repo_root,
        })
    };
    let pending_authorities = pending_user_action_authorities_for_plan(
        store,
        project_state,
        &request.envelope,
        &task_id,
        &plan_now,
    )?;
    let operation_context = UserActionOperationContext {
        operation: UserActionOperation::PrepareWrite,
        task_id: &task_id,
        change_unit_id: Some(&current_change_unit_id),
        scope_revision: task.scope_revision,
        close_basis: None,
        operation_refs: &operation_refs,
        sensitive_approval: sensitive_requirement.as_ref(),
    };
    let pending_user_action_refs = pending_authorities
        .iter()
        .filter(|authority| user_action_blocks_operation(authority, &operation_context))
        .map(|authority| {
            state_ref(
                StateRecordKind::UserActionRequest,
                &authority.user_action_request_id,
                &request.envelope.project_id,
                Some(&task_id),
                Some(project_state.state_version),
            )
        })
        .collect::<Vec<_>>();
    if !pending_user_action_refs.is_empty() {
        reasons.push(write_decision_reason(
            WriteDecisionCategory::UserAction,
            "user_action_unresolved",
            "A user action required before write preparation remains unresolved.",
            pending_user_action_refs.clone(),
        ));
    }

    let mut active_user_action_refs = Vec::new();
    let mut created_by_user_action_resolution_id = None;
    if sensitive_approval_required {
        let matching_sensitive_approval = matching_sensitive_approval(SensitiveApprovalSearch {
            store,
            project_state,
            request: &request,
            task_id: &task_id,
            task: &task,
            change_unit: &change_unit,
            intended_operation: &normalized_operation,
            normalized_paths: &normalized_paths,
            sensitive_categories: &normalized_sensitive_categories,
            now: &plan_now,
        })?;
        if let Some(record) = matching_sensitive_approval {
            if let Some(resolution) = record.resolution.as_ref() {
                created_by_user_action_resolution_id =
                    Some(resolution.user_action_resolution_id.clone());
                active_user_action_refs.push(state_ref(
                    StateRecordKind::UserActionResolution,
                    &resolution.user_action_resolution_id,
                    &request.envelope.project_id,
                    Some(&task_id),
                    Some(project_state.state_version),
                ));
            }
        } else {
            reasons.push(write_decision_reason(
                WriteDecisionCategory::SensitiveApproval,
                "sensitive_approval_missing",
                "A matching sensitive-action approval is required before write ticket issuance.",
                Vec::new(),
            ));
        }
    }

    let guarantee_display = Some(guarantee_display_for_invocation(
        store,
        verified_invocation,
        planned_state_version,
    )?);
    let change_unit_id = ChangeUnitId::new(change_unit.change_unit_id.clone());
    let attempt_scope = WriteTicketAttemptScope {
        task_id: task_id.clone(),
        change_unit_id: change_unit_id.clone(),
        intended_operation: normalized_operation,
        intended_paths: normalized_paths.clone(),
        product_file_write_intended: request.product_file_write_intended,
        sensitive_categories: normalized_sensitive_categories,
        baseline_ref: Some(request.baseline_ref.clone()),
    };
    let attempt_scope_json = serde_json::to_string(&attempt_scope)?;
    let created_at = plan_now.to_string();
    let validity_basis = WriteTicketValidityBasis {
        task_id: task_id.clone(),
        change_unit_id: change_unit_id.clone(),
        scope_revision: task.scope_revision,
        baseline_ref: Some(request.baseline_ref.clone()),
        workspace_context_sha256: verified_invocation
            .git_workspace_context
            .as_ref()
            .map(volicord_types::canonical::canonical_json_bare_sha256)
            .transpose()?,
        write_authority_fingerprint: workflow_policy.write_authority_fingerprint.clone(),
        approval_basis_refs: active_user_action_refs.clone(),
    };
    let active_ticket_selection = select_active_write_tickets(
        store,
        project_state,
        &request,
        &task,
        ActiveWriteTicketRequirements {
            validity_basis: &validity_basis,
            attempt_scope: &attempt_scope,
            sensitive_approval_required,
        },
        &plan_now,
    )?;
    if active_ticket_selection.compatible.is_some() {
        reasons.retain(|reason| reason.code != "sensitive_approval_missing");
    }
    let decision = prepare_write_decision(&reasons);
    let allowed = reasons.is_empty();
    let compatible_ticket = allowed
        .then_some(active_ticket_selection.compatible)
        .flatten();
    let reuse_write_ticket = compatible_ticket.is_some() && !request.envelope.dry_run;
    let issue_write_ticket = allowed && compatible_ticket.is_none() && !request.envelope.dry_run;
    let write_ticket_id = if let Some(record) = compatible_ticket.as_ref() {
        (!request.envelope.dry_run).then(|| WriteTicketId::new(record.write_ticket_id.clone()))
    } else if issue_write_ticket {
        Some(allocate_write_ticket_id(service, store).map_err(PlanError::Core)?)
    } else {
        None
    };
    let idle_expires_at_timestamp = if issue_write_ticket {
        workflow_policy
            .write_ticket_idle_timeout_minutes
            .map(|minutes| {
                let minutes = i64::try_from(minutes).map_err(|_| {
                    CorePipelineError::Store(StoreError::InvalidInput {
                        detail: "workflow write-ticket idle timeout is outside the supported range"
                            .to_owned(),
                    })
                })?;
                plan_now.checked_add(Duration::minutes(minutes)).map_err(|_| {
                    CorePipelineError::Store(StoreError::InvalidInput {
                        detail: "derived write-ticket idle timeout exceeds the supported timestamp range"
                            .to_owned(),
                    })
                })
            })
            .transpose()
            .map_err(PlanError::Core)?
    } else {
        compatible_ticket
            .as_ref()
            .and_then(|record| record.idle_expires_at.as_ref())
            .map(|value| {
                parse_owner_storage_value(
                    "write_tickets",
                    compatible_ticket
                        .as_ref()
                        .expect("idle expiration comes from a compatible ticket")
                        .write_ticket_id
                        .clone(),
                    "idle_expires_at",
                    value,
                )
            })
            .transpose()?
    };
    let write_ticket_ref = write_ticket_id.as_ref().map(|write_ticket_id| {
        state_ref(
            StateRecordKind::WriteTicket,
            write_ticket_id.as_str(),
            &request.envelope.project_id,
            Some(&task_id),
            Some(planned_state_version),
        )
    });
    let denied_path_patterns = if let Some(record) = compatible_ticket.as_ref() {
        decode_write_ticket_path_prefixes(record, false)?
    } else {
        denied_write_ticket_paths(&reasons, &normalized_paths)
    };
    let allowed_path_patterns = if let Some(record) = compatible_ticket.as_ref() {
        decode_write_ticket_path_prefixes(record, true)?
    } else {
        normalized_paths
            .iter()
            .filter(|path| !denied_path_patterns.iter().any(|denied| denied == *path))
            .cloned()
            .collect::<Vec<_>>()
    };
    let planned_write_ticket_record = if let Some(record) = compatible_ticket.as_ref() {
        (!request.envelope.dry_run).then(|| record.clone())
    } else {
        write_ticket_id
            .as_ref()
            .map(|write_ticket_id| WriteTicketRecord {
                project_id: request.envelope.project_id.as_str().to_owned(),
                write_ticket_id: write_ticket_id.as_str().to_owned(),
                task_id: task_id.as_str().to_owned(),
                change_unit_id: change_unit_id.as_str().to_owned(),
                basis_state_version: planned_state_version,
                status: "active".to_owned(),
                validity_basis_json: serde_json::to_string(&validity_basis)
                    .expect("write-ticket validity basis serializes"),
                allowed_path_prefixes_json: serde_json::to_string(&allowed_path_patterns)
                    .expect("write-ticket allowed paths serialize"),
                denied_path_prefixes_json: serde_json::to_string(&denied_path_patterns)
                    .expect("write-ticket denied paths serialize"),
                attempt_scope_json: attempt_scope_json.clone(),
                idle_expires_at: idle_expires_at_timestamp.as_ref().map(ToString::to_string),
                invalidation_reason: None,
                created_at: created_at.clone(),
                consumed_by_run_id: None,
                consumed_at: None,
            })
    };

    let write_ticket_effect = if reuse_write_ticket {
        WriteTicketEffect::Reused
    } else if issue_write_ticket {
        WriteTicketEffect::Issued
    } else {
        WriteTicketEffect::None
    };
    let mut storage_mutations = control_mutations;
    if !request.envelope.dry_run {
        for write_ticket_id in active_ticket_selection.stale_approval_ticket_ids {
            storage_mutations.push(CoreStorageMutation::WriteTicket(
                WriteTicketMutation::InvalidateById(WriteTicketByIdInvalidation {
                    write_ticket_id,
                    invalidation_reason: "approval_basis_changed".to_owned(),
                }),
            ));
        }
        for write_ticket_id in active_ticket_selection.stale_workspace_ticket_ids {
            storage_mutations.push(CoreStorageMutation::WriteTicket(
                WriteTicketMutation::InvalidateById(WriteTicketByIdInvalidation {
                    write_ticket_id,
                    invalidation_reason: WriteTicketInvalidationReason::WorkspaceChanged
                        .as_str()
                        .to_owned(),
                }),
            ));
        }
        for write_ticket_id in active_ticket_selection.stale_policy_ticket_ids {
            storage_mutations.push(CoreStorageMutation::WriteTicket(
                WriteTicketMutation::InvalidateById(WriteTicketByIdInvalidation {
                    write_ticket_id,
                    invalidation_reason: WriteTicketInvalidationReason::ExplicitRevoke
                        .as_str()
                        .to_owned(),
                }),
            ));
        }
    }
    if write_ticket_effect == WriteTicketEffect::Issued {
        let write_ticket_id = write_ticket_id
            .as_ref()
            .expect("new ticket issuance has an allocated ID");
        storage_mutations.push(CoreStorageMutation::WriteTicket(
            WriteTicketMutation::insert(WriteTicketInsert {
                write_ticket_id: write_ticket_id.as_str().to_owned(),
                task_id: task_id.as_str().to_owned(),
                change_unit_id: change_unit_id.as_str().to_owned(),
                validity_basis_json: serde_json::to_string(&validity_basis)?,
                allowed_path_prefixes_json: serde_json::to_string(&allowed_path_patterns)?,
                denied_path_prefixes_json: serde_json::to_string(&denied_path_patterns)?,
                attempt_scope_json,
                created_by_actor_source: verified_invocation.actor_source.to_canonical_string(),
                created_by_user_action_resolution_id,
                idle_expires_at: idle_expires_at_timestamp.as_ref().map(ToString::to_string),
                created_at,
                metadata_json: serde_json::to_string(&json!({
                    "verification_basis": verified_invocation.verification_basis.clone()
                }))?,
            }),
        ));
    }
    Ok(PrepareWritePlannedMutations {
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
        planned_write_ticket_record,
        idle_expires_at: idle_expires_at_timestamp,
        write_ticket_effect,
        allowed_path_patterns,
        denied_path_patterns,
        storage_mutations,
    })
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
        planned_write_ticket_record,
        idle_expires_at,
        write_ticket_effect,
        allowed_path_patterns,
        denied_path_patterns,
        storage_mutations,
    } = planned;
    let change_unit_id = ChangeUnitId::new(change_unit.change_unit_id.clone());
    let blocker_refs = store
        .active_blocker_refs(&task_id, planned_state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
    let evidence_summary = projected_evidence_summary(
        store,
        &request.envelope.project_id,
        planned_state_version,
        &task,
    )?;
    let projected_project_state = project_state_projection(
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
        &request.envelope,
        &task_id,
        facts_from_projection(
            task.clone(),
            Some(change_unit.clone()),
            projected_close_basis(store, &task_id)?,
            pending_user_action_refs.clone(),
            blocker_refs.clone(),
            evidence_summary.clone(),
            plan_now.clone(),
        ),
    )?;
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
        planned_write_ticket_record.as_ref(),
    ) {
        (Some(write_ticket_id), Some(write_ticket_ref), Some(record)) => {
            let selected_scope: WriteTicketAttemptScope =
                decode_required_json::<PersistedWriteTicketAttemptScope>(
                    "write_tickets",
                    record.write_ticket_id.clone(),
                    "attempt_scope_json",
                    Some(&record.attempt_scope_json),
                )?
                .into();
            let selected_validity_basis: WriteTicketValidityBasis = decode_required_json(
                "write_tickets",
                record.write_ticket_id.clone(),
                "validity_basis_json",
                Some(&record.validity_basis_json),
            )?;
            Some(WriteTicket {
                write_ticket_id: write_ticket_id.clone(),
                write_ticket_ref: write_ticket_ref.clone(),
                state: WriteTicketState::Open,
                scope: WriteTicketScope {
                    task_id: task_id.clone(),
                    change_unit_id: change_unit_id.clone(),
                    intended_operation: selected_scope.intended_operation,
                    product_file_write_intended: selected_scope.product_file_write_intended,
                    sensitive_categories: selected_scope.sensitive_categories,
                    baseline_ref: selected_scope.baseline_ref,
                },
                path_patterns: WriteTicketPathPatterns {
                    allowed: allowed_path_patterns.clone(),
                    denied: denied_path_patterns.clone(),
                },
                observed_paths: Vec::new(),
                basis_state_version: record.basis_state_version,
                validity_basis: selected_validity_basis,
                invalidation_reason: None,
                idle_expires_at: idle_expires_at.clone(),
                guarantee_display: guarantee_display.clone(),
            })
        }
        _ => None,
    };
    let state = build_state_summary(SummaryBuild {
        store,
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task,
        current_change_unit: Some(&change_unit),
        acceptance_criteria: active_acceptance_criteria_for_task(store, &task_id)?,
        pending_user_action_refs,
        blocker_refs,
        write_ticket_summary: planned_write_ticket_record
            .as_ref()
            .map(|record| {
                write_ticket_summary_for_record(
                    None,
                    record,
                    planned_state_version,
                    None,
                    None,
                    guarantee_display.clone(),
                )
            })
            .transpose()?,
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
        dry_run_summary: prepare_write_dry_run_summary(
            allowed,
            &reasons,
            write_ticket_ref,
            guarantee_display,
        ),
    })
}

fn prepare_write_validation_error<T>(
    dry_run: bool,
    state_version: u64,
    field: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    let response = validation_rejected(dry_run, Some(state_version), field, message)
        .map_err(PlanError::Core)?;
    Err(PlanError::Response(Box::new(response)))
}

fn acceptance_policy_rank(policy: AcceptancePolicy) -> u8 {
    match policy {
        AcceptancePolicy::NotRequired => 0,
        AcceptancePolicy::PolicyDependent => 1,
        AcceptancePolicy::Required => 2,
    }
}

#[derive(Debug, Default)]
struct ActiveWriteTicketSelection {
    compatible: Option<WriteTicketRecord>,
    stale_approval_ticket_ids: Vec<String>,
    stale_workspace_ticket_ids: Vec<String>,
    stale_policy_ticket_ids: Vec<String>,
}

struct ActiveWriteTicketRequirements<'a> {
    validity_basis: &'a WriteTicketValidityBasis,
    attempt_scope: &'a WriteTicketAttemptScope,
    sensitive_approval_required: bool,
}

fn select_active_write_tickets(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &PrepareWriteRequest,
    task: &TaskRecord,
    requirements: ActiveWriteTicketRequirements<'_>,
    now: &UtcTimestamp,
) -> Result<ActiveWriteTicketSelection, PlanError> {
    let required_basis = requirements.validity_basis;
    let required_write_authority_fingerprint = &required_basis.write_authority_fingerprint;
    let required_scope = requirements.attempt_scope;
    let mut selection = ActiveWriteTicketSelection::default();
    for record in store
        .active_write_tickets(&required_basis.task_id)
        .map_err(CorePipelineError::from)?
    {
        if write_ticket_is_idle_expired(&record, *now.as_datetime())
            .map_err(CorePipelineError::from)?
        {
            continue;
        }
        let basis: WriteTicketValidityBasis = decode_required_json(
            "write_tickets",
            record.write_ticket_id.clone(),
            "validity_basis_json",
            Some(&record.validity_basis_json),
        )?;
        if basis.write_authority_fingerprint != *required_write_authority_fingerprint {
            selection
                .stale_policy_ticket_ids
                .push(record.write_ticket_id);
            continue;
        }
        let scope: WriteTicketAttemptScope =
            decode_required_json::<PersistedWriteTicketAttemptScope>(
                "write_tickets",
                record.write_ticket_id.clone(),
                "attempt_scope_json",
                Some(&record.attempt_scope_json),
            )?
            .into();
        if requirements.sensitive_approval_required
            && scope.intended_operation != required_scope.intended_operation
        {
            continue;
        }
        if basis.task_id != required_basis.task_id
            || basis.change_unit_id != required_basis.change_unit_id
            || basis.scope_revision != required_basis.scope_revision
            || basis.baseline_ref != required_basis.baseline_ref
        {
            continue;
        }
        if scope.task_id != required_scope.task_id
            || scope.change_unit_id != required_scope.change_unit_id
            || scope.product_file_write_intended != required_scope.product_file_write_intended
            || scope.baseline_ref != required_scope.baseline_ref
            || !category_set_for_reuse(&required_scope.sensitive_categories)
                .is_subset(&category_set_for_reuse(&scope.sensitive_categories))
        {
            continue;
        }
        let allowed = decode_write_ticket_path_prefixes(&record, true)?;
        let denied = decode_write_ticket_path_prefixes(&record, false)?;
        if !required_scope.intended_paths.iter().all(|path| {
            allowed.iter().any(|prefix| path_is_within(path, prefix))
                && !denied.iter().any(|prefix| path_is_within(path, prefix))
        }) {
            continue;
        }
        if basis.workspace_context_sha256 != required_basis.workspace_context_sha256 {
            selection
                .stale_workspace_ticket_ids
                .push(record.write_ticket_id);
            continue;
        }
        if !write_ticket_approval_basis_is_current_for_prepare(
            store,
            project_state,
            request,
            task,
            &scope,
            &basis,
            now,
        )? {
            selection
                .stale_approval_ticket_ids
                .push(record.write_ticket_id);
            continue;
        }
        if requirements.sensitive_approval_required
            && (required_basis.approval_basis_refs.is_empty()
                || basis.approval_basis_refs.is_empty()
                || !approval_basis_identity_matches(
                    &required_basis.approval_basis_refs,
                    &basis.approval_basis_refs,
                ))
        {
            continue;
        }
        if selection.compatible.is_none() {
            selection.compatible = Some(record);
        }
    }
    Ok(selection)
}

fn write_ticket_approval_basis_is_current_for_prepare(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &PrepareWriteRequest,
    task: &TaskRecord,
    scope: &WriteTicketAttemptScope,
    validity_basis: &WriteTicketValidityBasis,
    now: &UtcTimestamp,
) -> Result<bool, PlanError> {
    if validity_basis.approval_basis_refs.is_empty() {
        return Ok(scope.sensitive_categories.is_empty());
    }

    let requirement = SensitiveApprovalRequirement {
        task_id: &validity_basis.task_id,
        change_unit_id: &validity_basis.change_unit_id,
        scope_revision: task.scope_revision,
        operation: &scope.intended_operation,
        normalized_paths: &scope.intended_paths,
        sensitive_categories: &scope.sensitive_categories,
        baseline_ref: scope.baseline_ref.as_ref(),
        required_for: UserActionRequiredFor::PrepareWrite,
        now,
        repo_root: &store.project_record().repo_root,
    };
    let records = store
        .resolved_user_action_records(
            &validity_basis.task_id,
            UserActionKind::SensitiveApproval,
            now,
        )
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let mut current_resolution_refs = Vec::new();
    for record in records {
        let authority = user_action_authority_from_record(&record)?;
        if current_sensitive_approval(&authority, &requirement) {
            if let Some(resolution_id) = authority.user_action_resolution_id {
                current_resolution_refs.push(state_ref(
                    StateRecordKind::UserActionResolution,
                    &resolution_id,
                    &request.envelope.project_id,
                    Some(&validity_basis.task_id),
                    Some(project_state.state_version),
                ));
            }
        }
    }

    Ok(!current_resolution_refs.is_empty()
        && validity_basis.approval_basis_refs.iter().all(|stored| {
            current_resolution_refs
                .iter()
                .any(|current| state_ref_identity_matches(stored, current))
        }))
}

fn approval_basis_identity_matches(left: &[StateRecordRef], right: &[StateRecordRef]) -> bool {
    left.len() == right.len()
        && left.iter().all(|reference| {
            right
                .iter()
                .any(|candidate| state_ref_identity_matches(reference, candidate))
        })
}

fn state_ref_identity_matches(left: &StateRecordRef, right: &StateRecordRef) -> bool {
    left.record_kind == right.record_kind
        && left.record_id == right.record_id
        && left.project_id == right.project_id
        && left.task_id == right.task_id
}

fn decode_write_ticket_path_prefixes(
    record: &WriteTicketRecord,
    allowed: bool,
) -> Result<Vec<String>, PlanError> {
    let (field, value) = if allowed {
        (
            "allowed_path_prefixes_json",
            &record.allowed_path_prefixes_json,
        )
    } else {
        (
            "denied_path_prefixes_json",
            &record.denied_path_prefixes_json,
        )
    };
    decode_required_json(
        "write_tickets",
        record.write_ticket_id.clone(),
        field,
        Some(value),
    )
    .map_err(PlanError::Core)
}

fn category_set_for_reuse(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

fn effect_contract_reason(
    violation: EffectContractViolation,
    change_unit_ref: StateRecordRef,
) -> WriteDecisionReason {
    match violation {
        EffectContractViolation::FileWriteForbidden => write_decision_reason(
            WriteDecisionCategory::EffectContract,
            "effect_contract_forbids_product_file_write",
            "The current Change Unit effect contract forbids product-file writes.",
            vec![change_unit_ref],
        ),
        EffectContractViolation::FileWriteNotAllowed => write_decision_reason(
            WriteDecisionCategory::EffectContract,
            "effect_contract_effect_not_allowed",
            "The current Change Unit effect contract does not allow product-file writes.",
            vec![change_unit_ref],
        ),
        EffectContractViolation::PathNotAllowed => write_decision_reason(
            WriteDecisionCategory::EffectContract,
            "effect_contract_path_not_allowed",
            "One or more intended paths are outside the current Change Unit effect contract allowed paths.",
            vec![change_unit_ref],
        ),
    }
}

fn denied_write_ticket_paths(
    reasons: &[WriteDecisionReason],
    normalized_paths: &[String],
) -> Vec<String> {
    let path_denied = reasons.iter().any(|reason| {
        matches!(
            reason.code.as_str(),
            "path_out_of_scope"
                | "effect_contract_path_not_allowed"
                | "effect_contract_forbids_product_file_write"
                | "effect_contract_effect_not_allowed"
        )
    });
    if path_denied {
        normalized_paths.to_vec()
    } else {
        Vec::new()
    }
}
