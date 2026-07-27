use super::close_readiness::{
    facts_from_projection, facts_with_pending_authorities, facts_with_resolved_authorities,
    plan_projected_close_readiness,
};
use super::{
    active_acceptance_criteria_for_task, allocate_user_action_resolution_id, build_state_summary,
    decision_rejected_response, decode_required_json, dry_run_summary,
    evidence_summary_for_display, guarantee_display_for_invocation, mutation_method_policy,
    next_actions_for_state, no_active_task_response, object_from_value, parse_task_mode,
    plan_error_response, prepare_or_response, project_state_projection, projected_blocker_refs,
    projected_close_basis, projected_evidence_summary, projected_write_ticket_summary,
    record_core_workflow_metric_best_effort, rejected_pipeline_response,
    response_committed_fresh_effect, state_ref, state_ref_from_stored, task_lifecycle_mutation,
    validation_plan_error, validation_rejected, MethodPlan, PlanError, SummaryBuild,
};
use crate::pipeline::{
    tool_error, CorePipelineError, CoreResult, CoreService, InvocationContext, OwnerPipelineBranch,
    PipelineResponse, TaskRequirement, VerifiedActorContext, VerifiedInvocationContext,
};
use crate::policy::close_readiness::UserActionAuthority;
use crate::user_action::authority::{user_action_authority_from_record, user_action_from_record};
use crate::user_action::continuity::plan_user_action_continuity_records;
use crate::user_action::identity::UserActionOrigin;
use crate::user_action::lifecycle::projected_user_action_lifecycle_phase;
use crate::user_action::materialization::{
    materialize_user_action_request, materialize_user_action_resolution,
    UserActionMaterializationInput, UserActionResolutionMaterializationInput,
};
use crate::user_action::model::{UserActionConstructionInput, UserActionIntent};
use crate::user_action::resolution::{
    channel_kind_from_verified_invocation,
    construct_user_action_resolution as construct_domain_user_action_resolution,
    resolution_input_matches_body as domain_resolution_input_matches_body,
    user_action_resolution_from_record as resolution_from_stored_record,
    validate_current_resolution_basis as validate_domain_resolution_basis,
};
use crate::user_action::service::{
    construct_user_action, pending_user_action_authorities_for_plan,
    resolved_user_action_authorities_for_all_kinds,
};
use serde_json::json;
use volicord_store::core_pipeline::{
    ChangeUnitRecord, CoreProjectStore, ProjectStateHeader, TaskRecord,
};
use volicord_store::diagnostics::WorkflowMetricKind;
use volicord_store::mutation::RuntimeHomeMutationContext;
use volicord_types::ids::{ChangeUnitId, IdempotencyKey, TaskId, UserActionRequestId};
use volicord_types::methods::{
    MethodOperationCategory, RequestUserActionRequest, RequestUserActionResultFields,
    ResolveUserActionRequest, ResolveUserActionResultFields,
};
use volicord_types::schema::{
    validate_channel_submission_id, AgentSafeUserActionRequestSummary, NextActionSummary,
    PersistedUserActionRequest, PersistedUserActionResolution, StateRecordRef, StateSummary,
    ToolEnvelope, UserActionBasis,
};
use volicord_types::values::{
    ActorSource, ErrorCode, MethodName, StateRecordKind, UserActionChannelKind, UserActionStatus,
    UtcTimestamp,
};

impl CoreService {
    /// Executes `volicord.request_user_action` through the shared Core mutation pipeline.
    pub fn request_user_action(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        request: RequestUserActionRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        execute_request_user_action(self, context, request, invocation)
    }

    /// Resolves one pending action from a verified User Channel invocation.
    pub fn resolve_user_action(
        &self,
        context: &RuntimeHomeMutationContext<'_>,
        request: ResolveUserActionRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        execute_resolve_user_action(self, context, request, invocation)
    }
}

fn execute_request_user_action(
    service: &CoreService,
    context: &RuntimeHomeMutationContext<'_>,
    request: RequestUserActionRequest,
    invocation: InvocationContext,
) -> CoreResult<PipelineResponse> {
    let request_json = serde_json::to_value(&request)?;
    if request.envelope.task_id.as_ref() != Some(&request.task_id) {
        return validation_rejected(
            request.envelope.dry_run,
            None,
            "envelope.task_id",
            "envelope.task_id must match RequestUserActionRequest.task_id",
        );
    }
    if let Err(error) = request.action.validate_bounds() {
        return validation_rejected(
            request.envelope.dry_run,
            None,
            error.field(),
            error.message(),
        );
    }
    let prepared = match prepare_or_response(
        service,
        Some(context),
        MethodName::RequestUserAction,
        request.envelope.clone(),
        request_json,
        invocation,
        mutation_method_policy(
            request.operation_category(),
            TaskRequirement::Exact(request.task_id.clone()),
            request.envelope.dry_run,
        ),
    )? {
        Ok(prepared) => prepared,
        Err(response) => return Ok(response),
    };
    let plan = match plan_request_user_action(
        service,
        &prepared.store,
        &prepared.context.project_state,
        request.clone(),
        &prepared.context.verified_invocation,
        &prepared.operation_now,
    ) {
        Ok(plan) => plan,
        Err(error) => {
            return plan_error_response(&request.envelope, &prepared.context.project_state, error)
        }
    };
    if request.envelope.dry_run {
        return service.execute_prepared_request::<RequestUserActionResultFields>(
            prepared,
            OwnerPipelineBranch::DryRunPreview {
                dry_run_summary: dry_run_summary(
                    "user_action_request",
                    "create_pending",
                    "Request would create one bounded pending user action.",
                    plan.next_actions,
                ),
            },
        );
    }
    service.execute_prepared_request(
        prepared,
        OwnerPipelineBranch::CommitMutation {
            result_fields: plan.result_fields,
            event_kind: "user_action_requested".to_owned(),
            event_payload: plan.event_payload,
            task_id: Some(plan.task_id),
            change_unit_id: plan.change_unit_id,
            storage_mutations: plan.storage_mutations,
        },
    )
}

fn plan_request_user_action(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: RequestUserActionRequest,
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &UtcTimestamp,
) -> Result<MethodPlan<RequestUserActionResultFields>, PlanError> {
    let now = operation_now.clone();
    let task = store
        .task_record(&request.task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    let current_change_unit = store
        .current_change_unit(&request.task_id)
        .map_err(CorePipelineError::from)?;
    let constructed = construct_user_action(UserActionConstructionInput {
        store,
        project_state,
        envelope: &request.envelope,
        task: &task,
        current_change_unit: current_change_unit.as_ref(),
        operation_now: &now,
        intent: UserActionIntent {
            task_id: request.task_id.clone(),
            change_unit_id: request.change_unit_id.as_ref().cloned(),
            action: request.action.clone(),
            required_for: request.required_for.clone(),
            expires_at: request.expires_at.clone(),
        },
    })?;
    let coordinate_change_unit_id = constructed.coordinate_change_unit_id.clone();
    let planned_state_version = project_state.state_version + 1;
    let materialized = materialize_user_action_request(UserActionMaterializationInput {
        service,
        store,
        project_state,
        verified_invocation,
        envelope: &request.envelope,
        origin: UserActionOrigin::DirectRequest,
        constructed,
    })?;
    let action_kind = materialized.public_request.action_kind;
    let request_id = materialized.public_request.user_action_request_id.clone();
    let request_ref = materialized.request_ref.clone();
    let effective = materialized.effective;
    let mut pending_authorities = pending_user_action_authorities_for_plan(
        store,
        project_state,
        &request.envelope,
        &request.task_id,
        &now,
    )?;
    pending_authorities.push(user_action_authority_from_record(&effective)?);
    let lifecycle_phase = projected_user_action_lifecycle_phase(
        project_state,
        &task,
        current_change_unit.as_ref(),
        &pending_authorities,
    );
    let mut projected_task = task.clone();
    if let Some(lifecycle_phase) = lifecycle_phase {
        projected_task.lifecycle_phase = lifecycle_phase.to_owned();
    }
    let (state, blocker_refs, next_actions) = projected_user_action_state(
        store,
        project_state,
        verified_invocation,
        &request.envelope,
        &projected_task,
        current_change_unit.as_ref(),
        &now,
        planned_state_version,
        Some(user_action_authority_from_record(&effective)?),
        Some(request_ref.clone()),
        None,
    )?;
    let result_fields = RequestUserActionResultFields {
        user_action_request_summary: AgentSafeUserActionRequestSummary::pending(request_id.clone()),
        blocker_refs,
        state,
    };
    let mut storage_mutations = vec![materialized.mutation];
    if let Some(lifecycle_phase) = lifecycle_phase {
        storage_mutations.push(task_lifecycle_mutation(&request.task_id, lifecycle_phase));
    }
    Ok(MethodPlan {
        task_id: request.task_id,
        change_unit_id: coordinate_change_unit_id,
        storage_mutations,
        event_payload: object_from_value(json!({
            "user_action_request_id": request_id,
            "action_kind": action_kind,
            "required_for": request.required_for,
        }))?,
        result_fields,
        next_actions,
    })
}

#[allow(clippy::too_many_arguments)]
fn projected_user_action_state(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    envelope: &ToolEnvelope,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    now: &UtcTimestamp,
    target_state_version: u64,
    projected_authority: Option<UserActionAuthority>,
    added_pending_ref: Option<StateRecordRef>,
    resolved_request_id: Option<&UserActionRequestId>,
) -> Result<(StateSummary, Vec<StateRecordRef>, Vec<NextActionSummary>), PlanError> {
    let planned_state_version = target_state_version;
    let task_id = TaskId::new(task.task_id.clone());
    let mut pending_refs = store
        .pending_user_action_refs(&task_id, planned_state_version, now)
        .map_err(CorePipelineError::from)?
        .into_iter()
        .map(state_ref_from_stored)
        .filter(|record_ref| {
            resolved_request_id
                .is_none_or(|request_id| record_ref.record_id.as_str() != request_id.as_str())
        })
        .collect::<Vec<_>>();
    if let Some(added_pending_ref) = added_pending_ref {
        pending_refs.push(added_pending_ref);
    }
    let blocker_refs = projected_blocker_refs(store, &task_id, planned_state_version)?;
    let task_ref = state_ref(
        StateRecordKind::Task,
        task_id.as_str(),
        &envelope.project_id,
        Some(&task_id),
        Some(planned_state_version),
    );
    let change_unit_ref = current_change_unit.map(|record| {
        state_ref(
            StateRecordKind::ChangeUnit,
            &record.change_unit_id,
            &envelope.project_id,
            Some(&task_id),
            Some(record.basis_state_version),
        )
    });
    let next_actions = next_actions_for_state(
        parse_task_mode(&task.mode)?,
        &task_ref,
        change_unit_ref.as_ref(),
        planned_state_version,
    );
    let guarantee_display =
        guarantee_display_for_invocation(store, verified_invocation, planned_state_version)?;
    let write_ticket_summary = projected_write_ticket_summary(
        store,
        &task_id,
        planned_state_version,
        *now.as_datetime(),
        Some(guarantee_display.clone()),
    )?;
    let current_close_basis = projected_close_basis(store, &task_id)?;
    let evidence_summary =
        projected_evidence_summary(store, &envelope.project_id, planned_state_version, task)?
            .map(|summary| evidence_summary_for_display(summary, current_close_basis.as_ref()));
    let mut pending_authorities =
        pending_user_action_authorities_for_plan(store, project_state, envelope, &task_id, now)?;
    if let Some(resolved_request_id) = resolved_request_id {
        pending_authorities
            .retain(|authority| authority.user_action_request_id != resolved_request_id.as_str());
    }
    let mut resolved_authorities = resolved_user_action_authorities_for_all_kinds(
        store,
        project_state,
        envelope,
        &task_id,
        now,
    )?;
    if let Some(authority) = projected_authority {
        match authority.status {
            UserActionStatus::Pending => {
                pending_authorities.retain(|existing| {
                    existing.user_action_request_id != authority.user_action_request_id
                });
                pending_authorities.push(authority);
            }
            UserActionStatus::Resolved => {
                resolved_authorities.retain(|existing| {
                    existing.user_action_request_id != authority.user_action_request_id
                });
                resolved_authorities.push(authority);
            }
            UserActionStatus::Stale | UserActionStatus::Superseded | UserActionStatus::Expired => {}
        }
    }
    let projected_project_state = project_state_projection(
        project_state,
        planned_state_version,
        project_state
            .active_task_id
            .clone()
            .or_else(|| Some(task_id.as_str().to_owned())),
    );
    let close_context = facts_with_resolved_authorities(
        facts_with_pending_authorities(
            facts_from_projection(
                task.clone(),
                current_change_unit.cloned(),
                current_close_basis,
                pending_refs.clone(),
                blocker_refs.clone(),
                evidence_summary.clone(),
                now.clone(),
            ),
            pending_authorities,
        ),
        resolved_authorities,
    );
    let close_plan = plan_projected_close_readiness(
        store,
        &projected_project_state,
        envelope,
        &task_id,
        close_context,
    )?;
    let state = build_state_summary(SummaryBuild {
        store,
        project_id: &envelope.project_id,
        state_version: planned_state_version,
        task,
        current_change_unit,
        acceptance_criteria: active_acceptance_criteria_for_task(store, &task_id)?,
        pending_user_action_refs: pending_refs,
        blocker_refs: blocker_refs.clone(),
        write_ticket_summary,
        evidence_summary,
        evidence_gate: Some(close_plan.evidence_gate),
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers,
        guarantee_display: Some(guarantee_display),
    })?;
    Ok((state, blocker_refs, next_actions))
}

fn execute_resolve_user_action(
    service: &CoreService,
    context: &RuntimeHomeMutationContext<'_>,
    request: ResolveUserActionRequest,
    invocation: InvocationContext,
) -> CoreResult<PipelineResponse> {
    if let Err(error) = validate_channel_submission_id(&request.channel_submission_id) {
        return validation_rejected(
            request.envelope.dry_run,
            None,
            error.field(),
            error.message(),
        );
    }
    if let Err(error) = request.resolution.validate_bounds() {
        return validation_rejected(
            request.envelope.dry_run,
            None,
            error.field(),
            error.message(),
        );
    }
    if request.envelope.expected_state_version.is_some() {
        return validation_rejected(
            request.envelope.dry_run,
            None,
            "envelope.expected_state_version",
            "resolve_user_action requires expected_state_version to be null",
        );
    }
    if request
        .envelope
        .idempotency_key
        .as_ref()
        .map(IdempotencyKey::as_str)
        != Some(request.channel_submission_id.as_str())
    {
        return validation_rejected(
            request.envelope.dry_run,
            None,
            "envelope.idempotency_key",
            "idempotency_key must exactly match channel_submission_id",
        );
    }
    let prepared = match prepare_or_response(
        service,
        Some(context),
        MethodName::ResolveUserAction,
        request.envelope.clone(),
        serde_json::to_value(&request)?,
        invocation,
        mutation_method_policy(
            request.operation_category(),
            TaskRequirement::None,
            request.envelope.dry_run,
        )
        .with_current_state_default(),
    )? {
        Ok(prepared) => prepared,
        Err(response) => return Ok(response),
    };
    if prepared
        .context
        .verified_invocation
        .git_workspace_context
        .is_some()
    {
        return rejected_pipeline_response(
            request.envelope.dry_run,
            Some(prepared.context.project_state.state_version),
            vec![tool_error(
                ErrorCode::InvocationContextMismatch,
                "user-only resolution channels must not carry Git workspace context",
                false,
                None,
            )],
        );
    }
    let Some(channel_kind) =
        channel_kind_from_verified_invocation(&prepared.context.verified_invocation)
    else {
        return rejected_pipeline_response(
            request.envelope.dry_run,
            Some(prepared.context.project_state.state_version),
            vec![tool_error(
                ErrorCode::InvocationContextMismatch,
                "verified invocation is not a supported User Channel",
                false,
                None,
            )],
        );
    };
    let now = prepared.operation_now.clone();
    let plan = match plan_resolve_user_action(
        service,
        &prepared.store,
        &prepared.context.project_state,
        request.clone(),
        &prepared.context.verified_invocation,
        &prepared.context.verified_actor,
        channel_kind,
        now,
    ) {
        Ok(plan) => plan,
        Err(error) => {
            return plan_error_response(&request.envelope, &prepared.context.project_state, error)
        }
    };
    if request.envelope.dry_run {
        return service.execute_prepared_request::<ResolveUserActionResultFields>(
            prepared,
            OwnerPipelineBranch::DryRunPreview {
                dry_run_summary: dry_run_summary(
                    "user_action_resolution",
                    "resolve_pending",
                    "Request would immutably resolve one pending user action.",
                    plan.method.next_actions,
                ),
            },
        );
    }
    let session_id = prepared.context.verified_invocation.session_id.clone();
    let response = service.execute_prepared_request(
        prepared,
        OwnerPipelineBranch::CommitMutation {
            result_fields: plan.method.result_fields,
            event_kind: "user_action_resolved".to_owned(),
            event_payload: plan.method.event_payload,
            task_id: Some(plan.method.task_id),
            change_unit_id: plan.method.change_unit_id,
            storage_mutations: plan.method.storage_mutations,
        },
    )?;
    if response_committed_fresh_effect(&response) {
        record_core_workflow_metric_best_effort(
            context,
            session_id.as_deref(),
            WorkflowMetricKind::UserRoundtrip,
            1,
        );
    }
    Ok(response)
}

struct ResolveUserActionPlan {
    method: MethodPlan<ResolveUserActionResultFields>,
}

#[allow(clippy::too_many_arguments)]
fn plan_resolve_user_action(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: ResolveUserActionRequest,
    verified_invocation: &VerifiedInvocationContext,
    verified_actor: &VerifiedActorContext,
    channel_kind: UserActionChannelKind,
    now: UtcTimestamp,
) -> Result<ResolveUserActionPlan, PlanError> {
    if verified_actor.actor_source != ActorSource::LocalUser {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "user-action resolution requires verified local-user authority",
        ))));
    }
    if let Some(existing) = store
        .user_action_resolution_for_channel_submission(channel_kind, &request.channel_submission_id)
        .map_err(CorePipelineError::from)?
    {
        let body: PersistedUserActionResolution = decode_required_json(
            "user_action_resolutions",
            existing.user_action_resolution_id.clone(),
            "resolution_json",
            Some(&existing.resolution_json),
        )?;
        let exact = existing.user_action_request_id == request.user_action_request_id.as_str()
            && existing.resolved_by_actor_source
                == verified_actor.actor_source.to_canonical_string()
            && existing.resolved_verification_basis.as_str() == verified_actor.verification_basis
            && existing.resolved_assurance_level == verified_actor.assurance_level
            && domain_resolution_input_matches_body(&request.resolution, &body);
        if !exact {
            return Err(PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "channel_submission_id conflicts with an immutable stored resolution",
            ))));
        }
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "channel submission is already committed; pipeline replay must use its original request identity",
        ))));
    }
    let effective = store
        .user_action_record(request.user_action_request_id.as_str(), &now)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "user_action_request_id does not identify a current user action",
            )))
        })?;
    if effective.status != UserActionStatus::Pending {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            match effective.status {
                UserActionStatus::Expired => "user action expired at or before this resolution",
                UserActionStatus::Stale => "user action basis is stale",
                UserActionStatus::Superseded => "user action basis is superseded",
                UserActionStatus::Resolved => "user action is already resolved",
                UserActionStatus::Pending => unreachable!(),
            },
        ))));
    }
    if request
        .envelope
        .task_id
        .as_ref()
        .is_some_and(|task_id| task_id.as_str() != effective.request.task_id)
    {
        return validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "envelope.task_id",
            "envelope.task_id must match the addressed user action Task",
        );
    }
    let task_id = TaskId::new(effective.request.task_id.clone());
    let task = store
        .task_record(&task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    let current_change_unit = store
        .current_change_unit(&task_id)
        .map_err(CorePipelineError::from)?;
    let persisted: PersistedUserActionRequest = decode_required_json(
        "user_action_requests",
        effective.request.user_action_request_id.clone(),
        "request_json",
        Some(&effective.request.request_json),
    )?;
    let basis: UserActionBasis = decode_required_json(
        "user_action_requests",
        effective.request.user_action_request_id.clone(),
        "basis_json",
        Some(&effective.request.basis_json),
    )?;
    validate_domain_resolution_basis(
        store,
        project_state,
        &request,
        &task,
        current_change_unit.as_ref(),
        &basis,
    )?;
    let resolution_id =
        allocate_user_action_resolution_id(service, store).map_err(PlanError::Core)?;
    let (resolution_body, mut derived_refs) = construct_domain_user_action_resolution(
        store,
        project_state,
        &request,
        &persisted.body,
        &basis,
        &task_id,
        current_change_unit.as_ref(),
    )?;
    resolution_body.validate().map_err(|error| {
        PlanError::Response(Box::new(
            validation_rejected(
                request.envelope.dry_run,
                Some(project_state.state_version),
                error.field(),
                error.message(),
            )
            .expect("resolution validation response should serialize"),
        ))
    })?;
    let materialized_resolution =
        materialize_user_action_resolution(UserActionResolutionMaterializationInput {
            project_id: &request.envelope.project_id,
            user_action_resolution_id: resolution_id.clone(),
            user_action_request_id: &request.user_action_request_id,
            action_kind: effective.request.action_kind,
            channel_kind,
            channel_submission_id: &request.channel_submission_id,
            resolution: resolution_body.clone(),
            verified_actor,
            resolved_at: &now,
        })
        .map_err(PlanError::Core)?;
    let resolution_record = materialized_resolution.record;
    let mut projected_effective = effective.clone();
    projected_effective.status = UserActionStatus::Resolved;
    projected_effective.resolution = Some(resolution_record.clone());
    let planned_state_version = project_state.state_version + 1;
    let public_request = user_action_from_record(&projected_effective, planned_state_version)?;
    let public_resolution = resolution_from_stored_record(&resolution_record, &task_id)?;
    let request_ref = state_ref(
        StateRecordKind::UserActionRequest,
        request.user_action_request_id.as_str(),
        &request.envelope.project_id,
        Some(&task_id),
        Some(planned_state_version),
    );
    let resolution_ref = state_ref(
        StateRecordKind::UserActionResolution,
        resolution_id.as_str(),
        &request.envelope.project_id,
        Some(&task_id),
        Some(planned_state_version),
    );
    let continuity_plans = plan_user_action_continuity_records(
        service,
        store,
        project_state,
        &request.envelope,
        &task_id,
        current_change_unit.as_ref(),
        &persisted.body,
        &basis,
        &resolution_body,
        &resolution_ref,
        &now,
    )?;
    derived_refs.extend(continuity_plans.iter().map(|plan| plan.record_ref.clone()));
    let mut pending_authorities = pending_user_action_authorities_for_plan(
        store,
        project_state,
        &request.envelope,
        &task_id,
        &now,
    )?;
    pending_authorities.retain(|authority| {
        authority.user_action_request_id != request.user_action_request_id.as_str()
    });
    let lifecycle_phase = projected_user_action_lifecycle_phase(
        project_state,
        &task,
        current_change_unit.as_ref(),
        &pending_authorities,
    );
    let mut projected_task = task.clone();
    if let Some(lifecycle_phase) = lifecycle_phase {
        projected_task.lifecycle_phase = lifecycle_phase.to_owned();
    }
    let (state, _blocker_refs, next_actions) = projected_user_action_state(
        store,
        project_state,
        verified_invocation,
        &request.envelope,
        &projected_task,
        current_change_unit.as_ref(),
        &now,
        planned_state_version,
        Some(user_action_authority_from_record(&projected_effective)?),
        None,
        Some(&request.user_action_request_id),
    )?;
    let result_fields = ResolveUserActionResultFields {
        user_action_request_ref: request_ref,
        user_action_resolution_ref: resolution_ref,
        user_action_request: public_request,
        user_action_resolution: public_resolution,
        derived_refs,
        state,
        next_actions: next_actions.clone(),
    };
    let mut storage_mutations = vec![materialized_resolution.mutation];
    storage_mutations.extend(continuity_plans.into_iter().map(|plan| plan.mutation));
    if let Some(lifecycle_phase) = lifecycle_phase {
        storage_mutations.push(task_lifecycle_mutation(&task_id, lifecycle_phase));
    }
    Ok(ResolveUserActionPlan {
        method: MethodPlan {
            task_id,
            change_unit_id: current_change_unit
                .as_ref()
                .map(|record| ChangeUnitId::new(record.change_unit_id.clone())),
            storage_mutations,
            event_payload: object_from_value(json!({
                "user_action_request_id": request.user_action_request_id,
                "user_action_resolution_id": resolution_id,
                "action_kind": effective.request.action_kind,
                "channel_kind": channel_kind,
                "channel_submission_id": request.channel_submission_id,
            }))?,
            result_fields,
            next_actions,
        },
    })
}
