use super::close_readiness::{
    facts_from_projection, facts_with_projected_acceptance_criteria, plan_projected_close_readiness,
};
use super::{
    active_acceptance_criteria_for_task, allocate_acceptance_criterion_id, allocate_task_id,
    build_state_summary, decision_rejected_response, dry_run_summary,
    guarantee_display_for_invocation, initial_work_phase, mutation_method_policy,
    next_actions_for_state, normalize_display_text, normalize_source_refs,
    normalize_source_refs_with_carried_artifact_task, object_from_value, plan_error_response,
    prepare_or_response, project_continuity_ref, project_state_projection, projected_blocker_refs,
    projected_close_basis, projected_evidence_summary_for_criteria, projected_write_ticket_summary,
    resolve_requested_mode, state_ref, validation_rejected, MethodPlan, PlanError, StoredScope,
    SummaryBuild,
};
use crate::pipeline::{
    commit_mutation_branch, dry_run_preview_branch, CommitMutationBranch, CorePipelineError,
    CoreResult, CoreService, InvocationContext, PipelineResponse, TaskRequirement,
    VerifiedInvocationContext,
};
use crate::policy::evidence::unique_state_record_refs;
use crate::policy::workflow::{
    acceptance_policy_for_control, effective_control_level, project_workflow_policy,
    resolve_task_control_authority, ProjectWorkflowPolicy,
};
use crate::policy::write_ticket::normalized_string_set;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use volicord_store::core_pipeline::{
    AcceptanceCriteriaReplace, AcceptanceCriterionUpsert, ChangeUnitRecord, CoreProjectStore,
    CoreStorageMutation, ProjectStateHeader, TaskAutonomyBoundary, TaskControlLevelUpdate,
    TaskInsert, TaskMutation, TaskRecord, TaskShapingFacts, WriteTicketInvalidation,
    WriteTicketMutation,
};
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::{BaselineRef, TaskId};
use volicord_types::methods::{IntakeRequest, IntakeResultFields, MethodOperationCategory};
use volicord_types::schema::{
    AcceptanceCriterion, AcceptanceCriterionInput, CarryForwardDisposition, JsonObject,
    NextActionSummary, SourceRef, StateRecordRef,
};
use volicord_types::values::{
    AcceptancePolicy, CarryForwardDispositionStatus, CarryForwardKind, MethodName,
    PersistedCloseSummary, ProjectContinuityKind, ProjectContinuityStatus, RequestedControlLevel,
    ResumePolicy, StateRecordKind, TaskControlLevel, TaskLifecyclePhase, TaskLineageRelation,
    TaskMode, TaskResult, UtcTimestamp, WriteTicketInvalidationReason,
};
use volicord_user_action_service::projected_pending_user_action_refs;

impl CoreService {
    /// Executes `volicord.intake` through the shared Core mutation pipeline.
    pub fn intake(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        request: volicord_types::methods::IntakeRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        let policy = mutation_method_policy(
            MethodName::Intake,
            request.operation_category(),
            TaskRequirement::None,
            request.envelope.dry_run,
        );
        let prepared = match prepare_or_response(
            self,
            Some(context),
            MethodName::Intake,
            request.envelope.clone(),
            request_json,
            invocation,
            policy,
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let store = &prepared.store;
        let project_state = &prepared.context.project_state;
        if request.resume_policy == ResumePolicy::RejectIfActive
            && project_state.active_task_id.is_some()
        {
            return validation_rejected(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "resume_policy",
                "resume_policy=reject_if_active cannot proceed while a Task is active",
            );
        }

        let plan = match plan_intake(
            self,
            store,
            project_state,
            request.clone(),
            &prepared.context.verified_invocation,
            &prepared.operation_now,
        ) {
            Ok(plan) => plan,
            Err(error) => return plan_error_response(&request.envelope, project_state, error),
        };

        if request.envelope.dry_run.is_requested() {
            return self.execute_prepared_request(
                prepared,
                dry_run_preview_branch::<IntakeRequest>(dry_run_summary(
                    "task",
                    "commit",
                    "Intake would select or create a Task.",
                    plan.next_actions,
                )),
            );
        }

        self.execute_prepared_request(
            prepared,
            commit_mutation_branch::<IntakeRequest>(CommitMutationBranch {
                result_fields: plan.result_fields,
                event_kind: "task_intake".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: None,
                storage_mutations: plan.storage_mutations,
            }),
        )
    }
}

struct NormalizedIntakeRequest {
    request: volicord_types::methods::IntakeRequest,
    mode: TaskMode,
}

fn normalize_intake_request(
    request: volicord_types::methods::IntakeRequest,
) -> NormalizedIntakeRequest {
    let mode = resolve_requested_mode(request.requested_mode);
    NormalizedIntakeRequest { request, mode }
}

struct ResolvedIntakeContext {
    request: volicord_types::methods::IntakeRequest,
    mode: TaskMode,
    planned_state_version: u64,
    active_task: Option<TaskRecord>,
    create_new: bool,
    workflow_policy: ProjectWorkflowPolicy,
    planned_lineage: Option<PlannedTaskLineage>,
}

fn resolve_intake_context(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    normalized: NormalizedIntakeRequest,
) -> Result<ResolvedIntakeContext, PlanError> {
    let NormalizedIntakeRequest { mut request, mode } = normalized;
    let planned_state_version = project_state.state_version + 1;
    let workflow_policy = project_workflow_policy(store).map_err(CorePipelineError::from)?;
    let active_task = store
        .active_task_record()
        .map_err(CorePipelineError::from)?;
    let create_new = match request.resume_policy {
        ResumePolicy::ResumeActive => active_task.is_none(),
        ResumePolicy::CreateNew | ResumePolicy::RejectIfActive => true,
        ResumePolicy::SupersedeActive => true,
    };
    if !create_new && (request.acceptance_policy.is_some() || request.lineage.is_some()) {
        return intake_validation_rejection(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "acceptance_policy",
            "resume_active requires null acceptance_policy and lineage fields",
        );
    }
    if create_new
        && mode == TaskMode::Advisor
        && !matches!(
            request.requested_control_level,
            RequestedControlLevel::Auto | RequestedControlLevel::Observe
        )
    {
        return intake_validation_rejection(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "requested_control_level",
            "advisor mode accepts only auto or observe control",
        );
    }
    let planned_lineage = if create_new {
        plan_task_lineage(
            store,
            project_state,
            verified_invocation,
            &mut request,
            planned_state_version,
        )?
    } else {
        None
    };

    Ok(ResolvedIntakeContext {
        request,
        mode,
        planned_state_version,
        active_task,
        create_new,
        workflow_policy,
        planned_lineage,
    })
}

struct IntakePolicyDecision {
    request: volicord_types::methods::IntakeRequest,
    mode: TaskMode,
    planned_state_version: u64,
    active_task: Option<TaskRecord>,
    create_new: bool,
    planned_lineage: Option<PlannedTaskLineage>,
    requested_control_level: RequestedControlLevel,
    effective_control_level: TaskControlLevel,
    control_level_reason: String,
    acceptance_policy: AcceptancePolicy,
    acceptance_policy_reason: String,
    control_or_acceptance_raised: bool,
}

fn decide_intake_policy(
    resolved: ResolvedIntakeContext,
) -> Result<IntakePolicyDecision, PlanError> {
    let ResolvedIntakeContext {
        request,
        mode,
        planned_state_version,
        active_task,
        create_new,
        workflow_policy,
        planned_lineage,
    } = resolved;
    let (
        requested_control_level,
        effective_control_level,
        control_level_reason,
        acceptance_policy,
        acceptance_policy_reason,
        control_or_acceptance_raised,
    ) = if create_new {
        let (effective_control_level, control_level_reason) =
            effective_control_level(mode, request.requested_control_level, &workflow_policy);
        let (acceptance_policy, acceptance_policy_reason) = resolve_acceptance_policy(
            effective_control_level,
            request.acceptance_policy.as_ref().copied(),
            &workflow_policy,
            &request,
        )?;
        (
            request.requested_control_level,
            effective_control_level,
            control_level_reason,
            acceptance_policy,
            acceptance_policy_reason,
            false,
        )
    } else {
        let active = active_task
            .as_ref()
            .expect("active_task exists when resume selects an existing Task");
        let requested_control_level = active.requested_control_level;
        let resolved_control = resolve_task_control_authority(active, &workflow_policy)
            .map_err(CorePipelineError::from)?;
        (
            requested_control_level,
            resolved_control.effective_control_level,
            resolved_control.control_level_reason,
            resolved_control.acceptance_policy,
            resolved_control.acceptance_policy_reason,
            resolved_control.control_raised || resolved_control.acceptance_raised,
        )
    };

    Ok(IntakePolicyDecision {
        request,
        mode,
        planned_state_version,
        active_task,
        create_new,
        planned_lineage,
        requested_control_level,
        effective_control_level,
        control_level_reason,
        acceptance_policy,
        acceptance_policy_reason,
        control_or_acceptance_raised,
    })
}

struct PlannedIntakeMutations {
    request: volicord_types::methods::IntakeRequest,
    mode: TaskMode,
    planned_state_version: u64,
    create_new: bool,
    planned_lineage: Option<PlannedTaskLineage>,
    acceptance_policy: AcceptancePolicy,
    task_id: TaskId,
    task_record: TaskRecord,
    current_change_unit: Option<ChangeUnitRecord>,
    acceptance_criteria: Vec<AcceptanceCriterion>,
    storage_mutations: Vec<CoreStorageMutation>,
}

fn plan_intake_mutations(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    policy: IntakePolicyDecision,
) -> Result<PlannedIntakeMutations, PlanError> {
    let IntakePolicyDecision {
        request,
        mode,
        planned_state_version,
        active_task,
        create_new,
        planned_lineage,
        requested_control_level,
        effective_control_level,
        control_level_reason,
        acceptance_policy,
        acceptance_policy_reason,
        control_or_acceptance_raised,
    } = policy;
    let task_id = if create_new {
        match request.envelope.task_id.as_ref().cloned() {
            Some(task_id) => task_id,
            None => allocate_task_id(service, store)?,
        }
    } else {
        TaskId::new(
            active_task
                .as_ref()
                .expect("active_task exists when create_new is false")
                .task_id
                .clone(),
        )
    };
    if planned_lineage
        .as_ref()
        .is_some_and(|lineage| lineage.predecessor_task_id == task_id.as_str())
    {
        return intake_validation_rejection(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "lineage.predecessor_task_id",
            "a Task cannot name itself as its predecessor",
        );
    }

    let mut initial_source_refs = if create_new {
        normalize_source_refs(
            store,
            project_state,
            &request.envelope,
            &task_id,
            "initial_source_refs",
            &request.initial_source_refs,
        )?
    } else {
        Vec::new()
    };
    if let Some(lineage) = planned_lineage.as_ref() {
        let predecessor_task_id = TaskId::new(lineage.predecessor_task_id.clone());
        let carried_source_refs = normalize_source_refs_with_carried_artifact_task(
            store,
            project_state,
            &request.envelope,
            &task_id,
            "lineage.carry_forward.source_refs",
            &lineage.carried_source_refs,
            Some(&predecessor_task_id),
        )?;
        for source_ref in carried_source_refs {
            if !initial_source_refs.contains(&source_ref) {
                initial_source_refs.push(source_ref);
            }
        }
    }

    let mut storage_mutations = Vec::new();
    if create_new {
        if let Some(active) = &active_task {
            storage_mutations.push(CoreStorageMutation::WriteTicket(
                WriteTicketMutation::InvalidateActive(WriteTicketInvalidation {
                    task_id: active.task_id.clone(),
                    invalidation_reason: WriteTicketInvalidationReason::TaskClosed,
                }),
            ));
        }
    }
    if request.resume_policy == ResumePolicy::SupersedeActive {
        if let Some(active) = &active_task {
            storage_mutations.push(CoreStorageMutation::Task(TaskMutation::Supersede {
                task_id: active.task_id.clone(),
            }));
        }
    }
    if !create_new && control_or_acceptance_raised {
        storage_mutations.push(CoreStorageMutation::Task(TaskMutation::UpdateControlLevel(
            TaskControlLevelUpdate {
                task_id: task_id.as_str().to_owned(),
                effective_control_level,
                control_level_reason: control_level_reason.clone(),
                acceptance_policy: Some(acceptance_policy),
                acceptance_policy_reason: Some(acceptance_policy_reason.clone()),
            },
        )));
    }

    let acceptance_criteria = if create_new {
        let mut criteria = Vec::with_capacity(request.initial_scope.acceptance_criteria.len());
        let mut reserved_ids = BTreeSet::new();
        for input in &request.initial_scope.acceptance_criteria {
            let statement = normalize_display_text(&input.statement);
            if statement.is_empty() {
                return intake_validation_rejection(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "initial_scope.acceptance_criteria[].statement",
                    "acceptance criterion statements must not be empty",
                );
            }
            let acceptance_criterion_id =
                allocate_acceptance_criterion_id(service, store, &reserved_ids)
                    .map_err(PlanError::Core)?;
            reserved_ids.insert(acceptance_criterion_id.as_str().to_owned());
            criteria.push(AcceptanceCriterion {
                acceptance_criterion_id,
                statement,
                evidence_requirement: input.evidence_requirement,
            });
        }
        criteria
    } else {
        active_acceptance_criteria_for_task(store, &task_id)?
    };

    let task_record = if create_new {
        let mut shaping = TaskShapingFacts {
            goal_summary: Some(request.plain_language_request.clone()),
            scope_summary: Some(request.initial_scope.boundary.clone()),
            non_goals: request.initial_scope.non_goals.clone(),
            baseline_ref: None,
            autonomy_boundary: None,
            initial_context_refs: request.initial_context_refs.clone(),
            initial_source_refs: initial_source_refs.clone(),
        };
        if let Some(lineage) = planned_lineage.as_ref() {
            if let Some(baseline_ref) = lineage.carried_baseline_ref.as_ref() {
                shaping.baseline_ref = Some(baseline_ref.clone());
            }
        }
        let work_phase = initial_work_phase(mode);
        let task = TaskRecord {
            project_id: request.envelope.project_id.as_str().to_owned(),
            task_id: task_id.as_str().to_owned(),
            mode,
            requested_control_level,
            effective_control_level,
            control_level_reason: control_level_reason.clone(),
            work_phase,
            acceptance_policy,
            acceptance_policy_reason: acceptance_policy_reason.clone(),
            predecessor_task_id: planned_lineage
                .as_ref()
                .map(|lineage| lineage.predecessor_task_id.clone()),
            lineage_relation: planned_lineage.as_ref().map(|lineage| lineage.relation),
            lineage_reason: planned_lineage
                .as_ref()
                .map(|lineage| lineage.creation_reason.clone()),
            carry_forward: planned_lineage
                .as_ref()
                .map(|lineage| lineage.dispositions.clone())
                .unwrap_or_default(),
            lifecycle_phase: TaskLifecyclePhase::Shaping,
            result: Some(TaskResult::None),
            title: Some(request.plain_language_request.clone()),
            summary: Some(request.plain_language_request.clone()),
            shaping,
            bounded_context: object_from_value(json!({
                "initial_context_refs": request.initial_context_refs,
                "initial_source_refs": initial_source_refs
            }))?,
            autonomy_boundary: TaskAutonomyBoundary {
                autonomy_boundary: None,
            },
            scope_revision: 0,
            close_basis_revision: 0,
            close_basis: None,
            close_summary: PersistedCloseSummary::default(),
            current_change_unit_id: None,
            closed_at: None,
            metadata: JsonObject::new(),
        };
        storage_mutations.push(CoreStorageMutation::Task(TaskMutation::insert(
            TaskInsert {
                task_id: task.task_id.clone(),
                created_by_actor_source: verified_invocation.actor_source.clone(),
                mode: task.mode,
                requested_control_level: task.requested_control_level,
                effective_control_level: task.effective_control_level,
                control_level_reason: task.control_level_reason.clone(),
                work_phase: task.work_phase,
                acceptance_policy: task.acceptance_policy,
                acceptance_policy_reason: task.acceptance_policy_reason.clone(),
                predecessor_task_id: task.predecessor_task_id.clone(),
                lineage_relation: task.lineage_relation,
                lineage_reason: task.lineage_reason.clone(),
                carry_forward: task.carry_forward.clone(),
                lifecycle_phase: task.lifecycle_phase,
                result: task.result,
                title: task.title.clone(),
                summary: task.summary.clone(),
                shaping: task.shaping.clone(),
                bounded_context: task.bounded_context.clone(),
                autonomy_boundary: task.autonomy_boundary.clone(),
                close_summary: task.close_summary.clone(),
                current_change_unit_id: None,
            },
        )));
        storage_mutations.push(CoreStorageMutation::Task(
            TaskMutation::ReplaceAcceptanceCriteria(AcceptanceCriteriaReplace {
                task_id: task.task_id.clone(),
                criteria: acceptance_criteria
                    .iter()
                    .enumerate()
                    .map(|(position, criterion)| AcceptanceCriterionUpsert {
                        acceptance_criterion_id: criterion
                            .acceptance_criterion_id
                            .as_str()
                            .to_owned(),
                        statement: criterion.statement.clone(),
                        evidence_requirement: criterion.evidence_requirement,
                        position: position as u64,
                    })
                    .collect(),
            }),
        ));
        storage_mutations.push(CoreStorageMutation::Task(TaskMutation::SetActive {
            task_id: task.task_id.clone(),
        }));
        task
    } else {
        let mut active = active_task.expect("active_task exists when create_new is false");
        active.effective_control_level = effective_control_level;
        active.control_level_reason = control_level_reason;
        active.acceptance_policy = acceptance_policy;
        active.acceptance_policy_reason = acceptance_policy_reason;
        active
    };

    let current_change_unit = if create_new {
        None
    } else {
        store
            .current_change_unit(&task_id)
            .map_err(CorePipelineError::from)?
    };

    Ok(PlannedIntakeMutations {
        request,
        mode,
        planned_state_version,
        create_new,
        planned_lineage,
        acceptance_policy,
        task_id,
        task_record,
        current_change_unit,
        acceptance_criteria,
        storage_mutations,
    })
}

struct IntakeResponseProjection {
    task_id: TaskId,
    storage_mutations: Vec<CoreStorageMutation>,
    event_payload: JsonObject,
    result_fields: IntakeResultFields,
    next_actions: Vec<NextActionSummary>,
}

impl IntakeResponseProjection {
    fn into_method_plan(self) -> MethodPlan<IntakeResultFields> {
        MethodPlan {
            task_id: self.task_id,
            change_unit_id: None,
            storage_mutations: self.storage_mutations,
            event_payload: self.event_payload,
            result_fields: self.result_fields,
            next_actions: self.next_actions,
        }
    }
}

fn plan_intake(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: volicord_types::methods::IntakeRequest,
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &UtcTimestamp,
) -> Result<MethodPlan<IntakeResultFields>, PlanError> {
    let normalized = normalize_intake_request(request);
    let resolved = resolve_intake_context(store, project_state, verified_invocation, normalized)?;
    let policy = decide_intake_policy(resolved)?;
    let mutations =
        plan_intake_mutations(service, store, project_state, verified_invocation, policy)?;
    let projection = project_intake_response(
        store,
        project_state,
        verified_invocation,
        operation_now,
        mutations,
    )?;
    Ok(projection.into_method_plan())
}

fn project_intake_response(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &UtcTimestamp,
    mutations: PlannedIntakeMutations,
) -> Result<IntakeResponseProjection, PlanError> {
    let PlannedIntakeMutations {
        request,
        mode,
        planned_state_version,
        create_new,
        planned_lineage,
        acceptance_policy,
        task_id,
        task_record,
        current_change_unit,
        acceptance_criteria,
        storage_mutations,
    } = mutations;
    let plan_now = *operation_now.as_datetime();
    let user_action_now = operation_now.clone();
    let task_ref = state_ref(
        StateRecordKind::Task,
        &task_record.task_id,
        &request.envelope.project_id,
        Some(&task_id),
        Some(planned_state_version),
    );
    let change_unit_ref = current_change_unit.as_ref().map(|record| {
        state_ref(
            StateRecordKind::ChangeUnit,
            &record.change_unit_id,
            &request.envelope.project_id,
            Some(&task_id),
            Some(record.basis_state_version),
        )
    });
    let pending_refs = if create_new {
        Vec::new()
    } else {
        projected_pending_user_action_refs(
            store,
            &task_id,
            planned_state_version,
            &user_action_now,
        )?
    };
    let blocker_refs = if create_new {
        Vec::new()
    } else {
        projected_blocker_refs(store, &task_id, planned_state_version)?
    };
    let next_actions = next_actions_for_state(
        task_record.mode,
        &task_ref,
        change_unit_ref.as_ref(),
        planned_state_version,
    );
    let evidence_summary = projected_evidence_summary_for_criteria(
        store,
        &request.envelope.project_id,
        planned_state_version,
        &task_record,
        &acceptance_criteria,
    )?;
    let projected_project_state = project_state_projection(
        project_state,
        planned_state_version,
        Some(task_record.task_id.clone()),
    );
    let close_plan = plan_projected_close_readiness(
        store,
        &projected_project_state,
        &request.envelope,
        &task_id,
        facts_with_projected_acceptance_criteria(
            facts_from_projection(
                task_record.clone(),
                current_change_unit.clone(),
                if create_new {
                    None
                } else {
                    projected_close_basis(store, &task_id)?
                },
                pending_refs.clone(),
                blocker_refs.clone(),
                evidence_summary.clone(),
                user_action_now.clone(),
            ),
            &acceptance_criteria,
        ),
    )?;
    let guarantee_display =
        guarantee_display_for_invocation(store, verified_invocation, planned_state_version)?;
    let write_ticket_summary = if create_new {
        None
    } else {
        projected_write_ticket_summary(
            store,
            &task_id,
            planned_state_version,
            plan_now,
            Some(guarantee_display.clone()),
        )?
    };
    let state = build_state_summary(SummaryBuild {
        store,
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task_record,
        current_change_unit: current_change_unit.as_ref(),
        acceptance_criteria,
        pending_user_action_refs: pending_refs,
        blocker_refs,
        write_ticket_summary,
        evidence_summary,
        evidence_gate: Some(close_plan.evidence_gate),
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers,
        guarantee_display: Some(guarantee_display),
    })?;
    let result_fields = IntakeResultFields {
        task_ref: task_ref.clone(),
        change_unit_ref,
        state,
        next_actions: next_actions.clone(),
    };
    let event_payload = object_from_value(json!({
        "task_id": task_id,
        "resume_policy": request.resume_policy,
        "requested_mode": request.requested_mode,
        "resolved_mode": mode
        ,"acceptance_policy": acceptance_policy
        ,"lineage": planned_lineage.as_ref().map(|lineage| json!({
            "predecessor_task_id": lineage.predecessor_task_id,
            "relation": lineage.relation,
            "carry_forward": lineage.dispositions
        }))
    }))?;
    Ok(IntakeResponseProjection {
        task_id,
        storage_mutations,
        event_payload,
        result_fields,
        next_actions,
    })
}

#[derive(Debug, Clone)]
struct PlannedTaskLineage {
    predecessor_task_id: String,
    relation: TaskLineageRelation,
    creation_reason: String,
    dispositions: Vec<CarryForwardDisposition>,
    carried_baseline_ref: Option<BaselineRef>,
    carried_source_refs: Vec<SourceRef>,
}

fn intake_validation_rejection<T>(
    dry_run: volicord_types::schema::DryRunIntent,
    state_version: Option<u64>,
    field: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    let response =
        validation_rejected(dry_run, state_version, field, message).map_err(PlanError::Core)?;
    Err(PlanError::Response(Box::new(response)))
}

fn plan_task_lineage(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    request: &mut volicord_types::methods::IntakeRequest,
    planned_state_version: u64,
) -> Result<Option<PlannedTaskLineage>, PlanError> {
    let Some(mut lineage) = request.lineage.as_ref().cloned() else {
        return Ok(None);
    };
    lineage.creation_reason = normalize_display_text(&lineage.creation_reason);
    if lineage.creation_reason.is_empty() {
        return intake_validation_rejection(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "lineage.creation_reason",
            "lineage creation_reason must not be empty",
        );
    }
    let predecessor = store
        .task_record(&lineage.predecessor_task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "lineage predecessor must identify an existing same-project Task",
            )))
        })?;
    if lineage.relation == TaskLineageRelation::ImplementsAdviceFrom
        && !(predecessor.mode == TaskMode::Advisor
            && predecessor.lifecycle_phase == TaskLifecyclePhase::Completed
            && predecessor.result == Some(TaskResult::AdviceOnly))
    {
        return intake_validation_rejection(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "lineage.relation",
            "implements_advice_from requires a completed advisor advice_only predecessor",
        );
    }
    let selected = lineage
        .carry_forward
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if selected.len() != lineage.carry_forward.len() {
        return intake_validation_rejection(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "lineage.carry_forward",
            "carry_forward values must not contain duplicates",
        );
    }
    let predecessor_ref = state_ref(
        StateRecordKind::Task,
        &predecessor.task_id,
        &request.envelope.project_id,
        Some(&lineage.predecessor_task_id),
        Some(planned_state_version),
    );

    let predecessor_scope = StoredScope::from_task(&predecessor)?;
    if selected.contains(&CarryForwardKind::Scope) {
        let predecessor_criteria = store
            .active_acceptance_criteria(&lineage.predecessor_task_id)
            .map_err(CorePipelineError::from)?
            .into_iter()
            .map(|record| {
                Ok(AcceptanceCriterionInput {
                    statement: record.statement,
                    evidence_requirement: record.evidence_requirement,
                })
            })
            .collect::<CoreResult<Vec<_>>>()?;
        if predecessor_scope.scope_summary.is_none() && predecessor_criteria.is_empty() {
            return intake_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "lineage.carry_forward",
                "selected scope carry-forward has no predecessor scope material",
            );
        }
        if let Some(scope) = predecessor_scope.scope_summary.as_ref() {
            let submitted = normalize_display_text(&request.initial_scope.boundary);
            if submitted.is_empty() {
                request.initial_scope.boundary = scope.clone();
            } else if submitted != normalize_display_text(scope) {
                return intake_validation_rejection(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "initial_scope.boundary",
                    "carried scope must match an explicitly submitted new-Task boundary",
                );
            }
        }
        if request.initial_scope.acceptance_criteria.is_empty() {
            request.initial_scope.acceptance_criteria = predecessor_criteria;
        } else if request.initial_scope.acceptance_criteria != predecessor_criteria {
            return intake_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "initial_scope.acceptance_criteria",
                "carried criteria must match explicitly submitted criterion statements and requirements",
            );
        }
    }
    if selected.contains(&CarryForwardKind::NonGoals) {
        if predecessor_scope.non_goals.is_empty() {
            return intake_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "lineage.carry_forward",
                "selected non_goals carry-forward has no predecessor non-goals",
            );
        }
        if request.initial_scope.non_goals.is_empty() {
            request.initial_scope.non_goals = predecessor_scope.non_goals.clone();
        } else if normalized_string_set(&request.initial_scope.non_goals)
            != normalized_string_set(&predecessor_scope.non_goals)
        {
            return intake_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "initial_scope.non_goals",
                "carried non-goals must match explicitly submitted non-goals",
            );
        }
    }

    if selected.contains(&CarryForwardKind::ContextRefs) {
        let refs = predecessor.shaping.initial_context_refs.clone();
        if refs.is_empty() {
            return intake_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "lineage.carry_forward",
                "selected context_refs carry-forward has no predecessor context refs",
            );
        }
        request.initial_context_refs.extend(refs);
        request.initial_context_refs =
            unique_state_record_refs(request.initial_context_refs.clone());
    }
    let carried_source_refs = if selected.contains(&CarryForwardKind::SourceRefs) {
        let refs = predecessor.shaping.initial_source_refs.clone();
        if refs.is_empty() {
            return intake_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "lineage.carry_forward",
                "selected source_refs carry-forward has no predecessor source refs",
            );
        }
        refs
    } else {
        Vec::new()
    };
    let reference_only_sources = reference_only_carry_sources(
        store,
        project_state,
        request,
        &predecessor,
        &predecessor_scope,
        &selected,
        planned_state_version,
    )?;

    let carried_baseline_ref = if selected.contains(&CarryForwardKind::Baseline) {
        let baseline_ref = predecessor_scope.baseline_ref.as_ref().ok_or_else(|| {
            PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "selected baseline carry-forward has no predecessor baseline",
            )))
        })?;
        let change_unit = store
            .current_change_unit(&lineage.predecessor_task_id)
            .map_err(CorePipelineError::from)?
            .ok_or_else(|| {
                PlanError::Response(Box::new(decision_rejected_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "selected baseline carry-forward has no current predecessor Change Unit",
                )))
            })?;
        if change_unit
            .write_basis
            .baseline_ref
            .as_ref()
            .map(BaselineRef::as_str)
            != Some(baseline_ref.as_str())
        {
            return intake_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "lineage.carry_forward",
                "baseline carry-forward requires matching predecessor Task and Change Unit baselines",
            );
        }
        if !super::workspace_context_matches(&change_unit, verified_invocation)? {
            return intake_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "lineage.carry_forward",
                "baseline carry-forward requires the exact current compatible Git workspace context",
            );
        }
        Some(BaselineRef::new(baseline_ref.clone()))
    } else {
        None
    };

    let dispositions = lineage
        .carry_forward
        .iter()
        .copied()
        .map(|kind| CarryForwardDisposition {
            kind,
            status: if matches!(
                kind,
                CarryForwardKind::UserDecisions
                    | CarryForwardKind::KnownLimitations
                    | CarryForwardKind::UnresolvedObligations
                    | CarryForwardKind::ResidualRisks
            ) {
                CarryForwardDispositionStatus::ReferenceOnly
            } else {
                CarryForwardDispositionStatus::Applied
            },
            source_refs: reference_only_sources
                .get(&kind)
                .cloned()
                .unwrap_or_else(|| vec![predecessor_ref.clone()]),
        })
        .collect();
    Ok(Some(PlannedTaskLineage {
        predecessor_task_id: predecessor.task_id,
        relation: lineage.relation,
        creation_reason: lineage.creation_reason,
        dispositions,
        carried_baseline_ref,
        carried_source_refs,
    }))
}

fn reference_only_carry_sources(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &volicord_types::methods::IntakeRequest,
    predecessor: &TaskRecord,
    predecessor_scope: &StoredScope,
    selected: &BTreeSet<CarryForwardKind>,
    planned_state_version: u64,
) -> Result<BTreeMap<CarryForwardKind, Vec<StateRecordRef>>, PlanError> {
    let reference_only_kinds = [
        (
            CarryForwardKind::UserDecisions,
            ProjectContinuityKind::Decision,
        ),
        (
            CarryForwardKind::KnownLimitations,
            ProjectContinuityKind::KnownLimit,
        ),
        (
            CarryForwardKind::UnresolvedObligations,
            ProjectContinuityKind::Obligation,
        ),
        (
            CarryForwardKind::ResidualRisks,
            ProjectContinuityKind::AcceptedRisk,
        ),
    ];
    if !reference_only_kinds
        .iter()
        .any(|(kind, _)| selected.contains(kind))
    {
        return Ok(BTreeMap::new());
    }

    let continuity_records = store
        .project_continuity_records_for_task(&predecessor.task_id)
        .map_err(CorePipelineError::from)?;
    let needs_current_risks = selected.contains(&CarryForwardKind::KnownLimitations)
        || selected.contains(&CarryForwardKind::ResidualRisks);
    let current_close_basis = if needs_current_risks {
        projected_close_basis(store, &TaskId::new(predecessor.task_id.clone()))?
    } else {
        None
    };
    let current_change_unit = if current_close_basis.is_some() {
        store
            .current_change_unit(&TaskId::new(predecessor.task_id.clone()))
            .map_err(CorePipelineError::from)?
    } else {
        None
    };

    let mut result = BTreeMap::new();
    for (kind, continuity_kind) in reference_only_kinds {
        if !selected.contains(&kind) {
            continue;
        }
        let mut source_refs = continuity_records
            .iter()
            .filter(|record| {
                record.status == ProjectContinuityStatus::Active && record.kind == continuity_kind
            })
            .map(|record| project_continuity_ref(record, planned_state_version))
            .collect::<Vec<_>>();

        if matches!(
            kind,
            CarryForwardKind::KnownLimitations | CarryForwardKind::ResidualRisks
        ) {
            if let Some(close_basis) = current_close_basis.as_ref() {
                let relevant_risks = close_basis
                    .residual_risks
                    .iter()
                    .filter(|risk| {
                        kind == CarryForwardKind::ResidualRisks || !risk.acceptance_required
                    })
                    .collect::<Vec<_>>();
                if !relevant_risks.is_empty() {
                    let compatible = close_basis.task_id.as_str() == predecessor.task_id
                        && close_basis.scope_revision == predecessor.scope_revision
                        && close_basis.baseline_ref.as_ref().map(BaselineRef::as_str)
                            == predecessor_scope.baseline_ref.as_deref()
                        && current_change_unit.as_ref().is_some_and(|change_unit| {
                            change_unit.change_unit_id == close_basis.change_unit_id.as_str()
                        });
                    if !compatible {
                        return intake_validation_rejection(
                            request.envelope.dry_run,
                            Some(project_state.state_version),
                            "lineage.carry_forward",
                            "reference-only risk carry-forward requires a current compatible predecessor close basis",
                        );
                    }
                    source_refs.push(close_basis.source_run_ref.clone());
                    source_refs.extend(
                        relevant_risks
                            .into_iter()
                            .flat_map(|risk| risk.source_refs.clone()),
                    );
                }
            }
        }
        source_refs = unique_state_record_refs(source_refs);
        if source_refs.is_empty() {
            return intake_validation_rejection(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "lineage.carry_forward",
                "selected reference-only carry-forward has no active compatible predecessor record",
            );
        }
        result.insert(kind, source_refs);
    }
    Ok(result)
}

fn resolve_acceptance_policy(
    control: TaskControlLevel,
    requested: Option<AcceptancePolicy>,
    workflow_policy: &ProjectWorkflowPolicy,
    request: &volicord_types::methods::IntakeRequest,
) -> Result<(AcceptancePolicy, String), PlanError> {
    let authoritative = acceptance_policy_for_control(control, workflow_policy);
    let selected = if control == TaskControlLevel::Light {
        requested
            .map(|requested| stronger_acceptance_policy(authoritative, requested))
            .unwrap_or(authoritative)
    } else {
        requested.unwrap_or(authoritative)
    };
    let valid = match control {
        TaskControlLevel::Observe => selected == AcceptancePolicy::NotRequired,
        TaskControlLevel::Light => match selected {
            AcceptancePolicy::Required | AcceptancePolicy::PolicyDependent => true,
            AcceptancePolicy::NotRequired => {
                workflow_policy.light.final_acceptance == AcceptancePolicy::NotRequired
            }
        },
        TaskControlLevel::Tracked | TaskControlLevel::Sensitive => {
            selected == AcceptancePolicy::Required
        }
    };
    if !valid {
        return intake_validation_rejection(
            request.envelope.dry_run,
            None,
            "acceptance_policy",
            "acceptance_policy is incompatible with the effective Task control level and project workflow policy",
        );
    }
    Ok((selected, acceptance_policy_reason(selected, control)))
}

fn acceptance_policy_reason(
    acceptance_policy: AcceptancePolicy,
    control: TaskControlLevel,
) -> String {
    match acceptance_policy {
        AcceptancePolicy::NotRequired => {
            format!("Effective control `{}` does not require final result acceptance.", control.as_str())
        }
        AcceptancePolicy::Required => {
            format!("Effective control `{}` requires final acceptance for the current close basis.", control.as_str())
        }
        AcceptancePolicy::PolicyDependent => {
            "Core evaluates final acceptance from the current low-risk completion conditions and residual-risk basis."
                .to_owned()
        }
    }
}

fn stronger_acceptance_policy(
    current: AcceptancePolicy,
    candidate: AcceptancePolicy,
) -> AcceptancePolicy {
    let rank = |policy| match policy {
        AcceptancePolicy::NotRequired => 0,
        AcceptancePolicy::PolicyDependent => 1,
        AcceptancePolicy::Required => 2,
    };
    if rank(candidate) > rank(current) {
        candidate
    } else {
        current
    }
}
