use crate::acceptance_facts::active_acceptance_criteria;
use crate::close_readiness::{
    facts_from_projection, facts_with_pending_authorities, facts_with_resolved_authorities,
    plan_projected_close_readiness,
};
use crate::continuity::{plan_user_action_continuity_records, ContinuityPlanningError};
use crate::enforcement_facts::project_enforcement_profile;
use crate::error_boundary::{
    product_path::observe_request_product_paths, store::plan_error_response,
    user_action::user_action_service_plan_error,
};
use crate::evidence_facts::{
    load_current_evidence_summary_facts, load_required_evidence_criterion_ids,
};
use crate::evidence_projection::evidence_summary_for_display;
use crate::guarantee_projection::guarantee_display;
use crate::identity::{allocate_user_action_request_id, allocate_user_action_resolution_id};
use crate::json_object::object_from_value;
use crate::method_execution::{mutation_method_policy, prepare_or_response, PlanError};
use crate::method_rejection::{
    decision_rejected_response, dry_run_summary, no_active_task_response,
    rejected_pipeline_response, validation_plan_error, validation_rejected,
};
use crate::operation_plan::OperationPlan;
use crate::pipeline::{
    commit_mutation_branch, dry_run_preview_branch, tool_error, CorePipelineError, CoreResult,
    CoreService, InvocationContext, PipelineResponse, TaskRequirement, VerifiedActorContext,
    VerifiedInvocationContext,
};
use crate::policy::close_readiness_evidence::project_close_evidence_summary;
use crate::policy::workflow::project_workflow_policy;
use crate::record_refs::{state_ref, state_ref_from_stored};
use crate::state_summary::{project_state_header, state_summary, StateSummaryInput};
use crate::task_facts::{active_blocker_refs, current_close_basis};
use crate::task_policy::{plan_user_action_lifecycle_transition, TaskLifecycleFacts};
use crate::workflow_diagnostics::{
    record_core_workflow_metric_best_effort, response_committed_fresh_effect,
};
use crate::write_ticket::service::load_current_write_ticket_summary;
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
    validate_channel_submission_id, AgentSafeUserActionRequestSummary, StateRecordRef,
    StateSummary, ToolEnvelope, UserActionResolutionBody,
};
use volicord_types::values::{
    evaluate_shaping_decision_authority, ActorSource, ErrorCode, JudgmentResolutionOutcome,
    MethodName, ShapingDecisionAuthorityFacts, ShapingDecisionAuthorityState, ShapingGapStatus,
    StateRecordKind, UserActionBasisStatus, UserActionChannelKind, UserActionOptionAction,
    UserActionRequiredFor, UserActionStatus, UserActionVerificationBasis, UtcTimestamp, WorkPhase,
};

fn shaping_disposition(resolution: &UserActionResolutionBody) -> Option<ShapingGapStatus> {
    match resolution {
        UserActionResolutionBody::Choice {
            machine_action: UserActionOptionAction::Accept,
            resolution_outcome: JudgmentResolutionOutcome::Accepted,
            ..
        } => Some(ShapingGapStatus::Accepted),
        UserActionResolutionBody::Choice {
            machine_action: UserActionOptionAction::Reject,
            resolution_outcome: JudgmentResolutionOutcome::Rejected,
            ..
        } => Some(ShapingGapStatus::Rejected),
        UserActionResolutionBody::Choice {
            machine_action: UserActionOptionAction::Defer,
            resolution_outcome: JudgmentResolutionOutcome::Deferred,
            ..
        } => Some(ShapingGapStatus::Deferred),
        _ => None,
    }
}

fn projected_resolved_shaping_authority_state(
    resolution: &UserActionResolutionBody,
    disposition: ShapingGapStatus,
) -> ShapingDecisionAuthorityState {
    let (machine_action, resolution_outcome) = match resolution {
        UserActionResolutionBody::Choice {
            machine_action,
            resolution_outcome,
            ..
        } => (Some(*machine_action), Some(*resolution_outcome)),
        _ => (None, None),
    };
    evaluate_shaping_decision_authority(ShapingDecisionAuthorityFacts {
        effective_user_action_status: UserActionStatus::Resolved,
        resolution_present: true,
        machine_action,
        resolution_outcome,
        request_basis_status: UserActionBasisStatus::Current,
        basis_compatibility_status: UserActionBasisStatus::Current,
        checkpoint_identity_matches: true,
        gap_identity_matches: true,
        resolution_identity_matches: true,
        policy_matches: true,
        verified_user_channel: true,
        task_mode_matches: true,
        scope_revision_matches: true,
        baseline_matches: true,
        change_unit_matches: true,
        gap_status: disposition,
        application_present: false,
        application_authority_status: None,
        application_identity_matches: false,
        application_lineage_current: false,
    })
}
use volicord_user_action_service::{
    construct_user_action,
    construct_user_action_resolution as construct_domain_user_action_resolution,
    materialize_user_action_request, materialize_user_action_resolution,
    pending_user_action_authorities, projected_user_action_lifecycle_phase,
    resolution_input_matches_body as domain_resolution_input_matches_body,
    resolved_user_action_facts_for_all_kinds, user_action_authority_from_record,
    user_action_from_record, user_action_resolution_from_record as resolution_from_stored_record,
    validate_current_resolution_basis as validate_domain_resolution_basis, UserActionAuthority,
    UserActionConstructionContext, UserActionConstructionInput, UserActionIntent,
    UserActionMaterializationInput, UserActionOrigin, UserActionPersistenceContext,
    UserActionResolutionMaterializationInput,
};

fn channel_kind_from_verified_invocation(
    invocation: &VerifiedInvocationContext,
) -> Option<UserActionChannelKind> {
    UserActionVerificationBasis::parse(&invocation.verification_basis)
        .map(UserActionChannelKind::from_verification_basis)
}

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
            MethodName::RequestUserAction,
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
            let response =
                plan_error_response(&request.envelope, &prepared.context.project_state, error)?;
            return Ok(response.with_prepared_context(&prepared));
        }
    };
    if request.envelope.dry_run.is_requested() {
        return service.execute_prepared_request(
            prepared,
            dry_run_preview_branch::<RequestUserActionRequest>(dry_run_summary(
                "user_action_request",
                "create_pending",
                "Request would create one bounded pending user action.",
                Vec::new(),
            )),
        );
    }
    service.execute_prepared_request(
        prepared,
        commit_mutation_branch::<RequestUserActionRequest>(
            plan.operation
                .into_commit_branch::<RequestUserActionRequest>(
                    plan.result_fields,
                    "user_action_requested",
                ),
        ),
    )
}

fn plan_request_user_action(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: RequestUserActionRequest,
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &UtcTimestamp,
) -> Result<RequestUserActionPlan, PlanError> {
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
    let mut action = request.action.clone();
    if let volicord_types::schema::UserActionDraft::Choice(choice) = &mut action {
        if let Some(scope) = choice.sensitive_action_scope.as_mut() {
            scope.intended_paths = observe_request_product_paths(
                &store.project_record().repo_root,
                &scope.intended_paths,
                request.envelope.dry_run,
                Some(project_state.state_version),
                "action.sensitive_action_scope.intended_paths",
                "sensitive-action intended_paths must be normalized relative Product Repository paths",
                "sensitive-action intended_paths must resolve within the Product Repository",
            )?;
        }
    }
    let constructed = construct_user_action(UserActionConstructionInput {
        store,
        task: &task,
        current_change_unit: current_change_unit.as_ref(),
        context: UserActionConstructionContext {
            project_id: request.envelope.project_id.clone(),
            observed_state_version: project_state.state_version,
            observed_at: now.clone(),
            locale: request.envelope.locale.as_ref().cloned(),
        },
        intent: UserActionIntent {
            task_id: request.task_id.clone(),
            change_unit_id: request.change_unit_id.as_ref().cloned(),
            action,
            required_for: request.required_for.clone(),
            expires_at: request.expires_at.clone(),
        },
    })
    .map_err(|error| user_action_service_plan_error(&request.envelope, project_state, error))?;
    let coordinate_change_unit_id = constructed.coordinate_change_unit_id.clone();
    let planned_state_version = project_state.state_version + 1;
    let Some(operation_identity) = request.envelope.idempotency_key.as_ref().cloned() else {
        return validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "envelope.idempotency_key",
            "a user-action request requires an idempotency key",
        );
    };
    let user_action_request_id =
        allocate_user_action_request_id(service.durable_id_generator(), store)
            .map_err(PlanError::Core)?;
    let materialized = materialize_user_action_request(UserActionMaterializationInput {
        context: UserActionPersistenceContext {
            project_id: request.envelope.project_id.clone(),
            actor_source: verified_invocation.actor_source.clone(),
            operation_identity,
            planned_state_version,
            user_action_request_id,
        },
        origin: UserActionOrigin::DirectRequest,
        constructed,
    })
    .map_err(|error| user_action_service_plan_error(&request.envelope, project_state, error))?;
    let action_kind = materialized.public_request.action_kind;
    let request_id = materialized.public_request.user_action_request_id.clone();
    let request_ref = materialized.request_ref.clone();
    let effective = materialized.effective;
    let mut pending_authorities = pending_user_action_authorities(store, &request.task_id, &now)
        .map_err(|error| user_action_service_plan_error(&request.envelope, project_state, error))?;
    pending_authorities.push(
        user_action_authority_from_record(&effective).map_err(|error| {
            user_action_service_plan_error(&request.envelope, project_state, error)
        })?,
    );
    let lifecycle_phase = projected_user_action_lifecycle_phase(
        project_state,
        &task,
        current_change_unit.as_ref(),
        &pending_authorities,
    );
    let lifecycle_transition = lifecycle_phase
        .map(|target| {
            plan_user_action_lifecycle_transition(TaskLifecycleFacts::from(&task), target)
        })
        .transpose()?
        .flatten();
    let mut projected_task = task.clone();
    if let Some(transition) = lifecycle_transition.as_ref() {
        projected_task.lifecycle_phase = transition.target();
    }
    let (state, blocker_refs) = projected_user_action_state(
        store,
        project_state,
        verified_invocation,
        &request.envelope,
        &projected_task,
        current_change_unit.as_ref(),
        &now,
        planned_state_version,
        Some(
            user_action_authority_from_record(&effective).map_err(|error| {
                user_action_service_plan_error(&request.envelope, project_state, error)
            })?,
        ),
        Some(request_ref.clone()),
        None,
    )?;
    let result_fields = RequestUserActionResultFields {
        user_action_request_summary: AgentSafeUserActionRequestSummary::pending(request_id.clone()),
        blocker_refs,
        state,
    };
    let mut storage_mutations = vec![materialized.mutation];
    if let Some(transition) = lifecycle_transition {
        storage_mutations.push(transition.storage_mutation());
    }
    Ok(RequestUserActionPlan {
        operation: OperationPlan::new(
            request.task_id,
            coordinate_change_unit_id,
            storage_mutations,
            object_from_value(json!({
                "user_action_request_id": request_id,
                "action_kind": action_kind,
                "required_for": request.required_for,
            }))?,
        ),
        result_fields,
    })
}

struct RequestUserActionPlan {
    operation: OperationPlan,
    result_fields: RequestUserActionResultFields,
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
) -> Result<(StateSummary, Vec<StateRecordRef>), PlanError> {
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
    let blocker_refs = active_blocker_refs(store, &task_id, planned_state_version)?;
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
        &task_id,
        planned_state_version,
        now,
        Some(guarantee_display.clone()),
    )?;
    let current_close_basis = current_close_basis(store, &task_id)?;
    let evidence_facts = load_current_evidence_summary_facts(
        store,
        task,
        &envelope.project_id,
        &task_id,
        planned_state_version,
    )?;
    let required_criteria = load_required_evidence_criterion_ids(store, &task_id)?;
    let evidence_summary = project_close_evidence_summary(evidence_facts, &required_criteria)
        .map(|summary| evidence_summary_for_display(summary, current_close_basis.as_ref()));
    let mut pending_authorities = pending_user_action_authorities(store, &task_id, now)
        .map_err(|error| user_action_service_plan_error(envelope, project_state, error))?;
    if let Some(resolved_request_id) = resolved_request_id {
        pending_authorities
            .retain(|authority| &authority.user_action_request_id != resolved_request_id);
    }
    let mut resolved_action_facts = resolved_user_action_facts_for_all_kinds(store, &task_id, now)
        .map_err(|error| user_action_service_plan_error(envelope, project_state, error))?;
    if let Some(authority) = projected_authority.as_ref() {
        match authority.status {
            UserActionStatus::Pending => {
                pending_authorities.retain(|existing| {
                    existing.user_action_request_id != authority.user_action_request_id
                });
                pending_authorities.push(authority.clone());
            }
            UserActionStatus::Resolved => {
                resolved_action_facts.retain(|existing| {
                    existing.user_action_request_id != authority.user_action_request_id
                });
                resolved_action_facts.push(authority.clone());
            }
            UserActionStatus::Stale | UserActionStatus::Superseded | UserActionStatus::Expired => {}
        }
    }
    let projected_project_state = project_state_header(
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
        resolved_action_facts,
    );
    let close_plan = plan_projected_close_readiness(
        store,
        &projected_project_state,
        &envelope.project_id,
        &task_id,
        close_context,
    )
    .map_err(|error| {
        crate::error_boundary::close_readiness::close_readiness_plan_error(
            envelope,
            &projected_project_state,
            error,
        )
    })?;
    let mut shaping_checkpoint = store
        .current_shaping_checkpoint(&task_id)
        .map_err(CorePipelineError::from)?;
    if let Some(authority) = projected_authority
        .as_ref()
        .filter(|authority| authority.status == UserActionStatus::Resolved)
    {
        let disposition = authority.resolution.as_ref().and_then(shaping_disposition);
        if let Some(checkpoint) = shaping_checkpoint.as_mut() {
            if let Some(gap) = checkpoint.gaps.iter_mut().find(|gap| {
                gap.user_action.as_ref().is_some_and(|link| {
                    link.user_action_request_id == authority.user_action_request_id.as_str()
                })
            }) {
                gap.status = disposition.unwrap_or(ShapingGapStatus::Current);
                if let Some(link) = gap.user_action.as_mut() {
                    link.user_action_resolution_id = authority
                        .user_action_resolution_id
                        .as_ref()
                        .map(|resolution_id| resolution_id.as_str().to_owned());
                    link.resolved_at = Some(now.clone());
                }
            }
            if checkpoint.baseline_ref.is_some()
                && checkpoint.implementation_boundary.is_some()
                && checkpoint
                    .gaps
                    .iter()
                    .all(|gap| gap.status != volicord_types::values::ShapingGapStatus::Current)
            {
                checkpoint.readiness = volicord_types::values::ShapingCheckpointReadiness::Ready;
            }
        }
    }
    let mut task_wide_shaping_authority = crate::workflow_projection::task_wide_shaping_authority(
        store,
        &envelope.project_id,
        planned_state_version,
        task,
        current_change_unit,
        shaping_checkpoint.as_ref(),
        now,
    )?;
    if let Some(authority) = projected_authority.as_ref().filter(|authority| {
        authority
            .required_for
            .contains(&UserActionRequiredFor::AdvanceTask)
    }) {
        let request_id = authority.user_action_request_id.as_str();
        for facts in [
            &mut task_wide_shaping_authority.awaiting_user,
            &mut task_wide_shaping_authority.accepted_unapplied,
            &mut task_wide_shaping_authority.recovery_required,
            &mut task_wide_shaping_authority.inconsistent,
        ] {
            facts.retain(|fact| fact.request_ref.record_id.as_str() != request_id);
        }
        let represented_gap = shaping_checkpoint.as_ref().and_then(|checkpoint| {
            checkpoint.gaps.iter().find(|gap| {
                gap.user_action
                    .as_ref()
                    .is_some_and(|link| link.user_action_request_id == request_id)
            })
        });
        let authority_state = match authority.status {
            UserActionStatus::Pending => ShapingDecisionAuthorityState::AwaitingUser,
            UserActionStatus::Expired if represented_gap.is_some() => {
                ShapingDecisionAuthorityState::Expired
            }
            UserActionStatus::Resolved => {
                represented_gap.zip(authority.resolution.as_ref()).map_or(
                    ShapingDecisionAuthorityState::Inconsistent,
                    |(gap, resolution)| {
                        projected_resolved_shaping_authority_state(resolution, gap.status)
                    },
                )
            }
            UserActionStatus::Stale => ShapingDecisionAuthorityState::Stale,
            UserActionStatus::Superseded => ShapingDecisionAuthorityState::Superseded,
            UserActionStatus::Expired => ShapingDecisionAuthorityState::Inconsistent,
        };
        let fact = crate::workflow_projection::WorkflowUserActionFact {
            request_ref: state_ref(
                StateRecordKind::UserActionRequest,
                request_id,
                &envelope.project_id,
                Some(&task_id),
                Some(planned_state_version),
            ),
            resolution_ref: authority
                .user_action_resolution_id
                .as_ref()
                .map(|resolution_id| {
                    state_ref(
                        StateRecordKind::UserActionResolution,
                        resolution_id.as_str(),
                        &envelope.project_id,
                        Some(&task_id),
                        Some(planned_state_version),
                    )
                }),
            status: authority.status,
            authority_state,
            required_owner_method: match authority_state {
                ShapingDecisionAuthorityState::AwaitingUser => MethodName::ResolveUserAction,
                ShapingDecisionAuthorityState::AcceptedUnapplied => represented_gap
                    .and_then(|gap| gap.gap_kind.decision_policy())
                    .map_or(MethodName::Status, |policy| {
                        policy.application_owner.method()
                    }),
                ShapingDecisionAuthorityState::Rejected
                | ShapingDecisionAuthorityState::Deferred
                | ShapingDecisionAuthorityState::Expired => MethodName::RecordShapingCheckpoint,
                ShapingDecisionAuthorityState::Stale => {
                    if task.work_phase == WorkPhase::Shaping {
                        MethodName::RecordShapingCheckpoint
                    } else {
                        MethodName::CloseTask
                    }
                }
                ShapingDecisionAuthorityState::Applied
                | ShapingDecisionAuthorityState::Superseded
                | ShapingDecisionAuthorityState::Inconsistent => MethodName::Status,
            },
        };
        match authority_state {
            ShapingDecisionAuthorityState::AwaitingUser => {
                task_wide_shaping_authority.awaiting_user.push(fact)
            }
            ShapingDecisionAuthorityState::AcceptedUnapplied => {
                task_wide_shaping_authority.accepted_unapplied.push(fact)
            }
            ShapingDecisionAuthorityState::Rejected
            | ShapingDecisionAuthorityState::Deferred
            | ShapingDecisionAuthorityState::Expired => {
                task_wide_shaping_authority.recovery_required.push(fact)
            }
            ShapingDecisionAuthorityState::Inconsistent => {
                task_wide_shaping_authority.inconsistent.push(fact)
            }
            ShapingDecisionAuthorityState::Applied
            | ShapingDecisionAuthorityState::Stale
            | ShapingDecisionAuthorityState::Superseded => {}
        }
    }
    let state = state_summary(StateSummaryInput {
        project_id: &envelope.project_id,
        state_version: planned_state_version,
        task,
        current_change_unit,
        shaping_checkpoint: shaping_checkpoint.as_ref(),
        task_wide_shaping_authority: &task_wide_shaping_authority,
        project_policy,
        acceptance_criteria: active_acceptance_criteria(store, &task_id)?,
        pending_user_action_refs: pending_refs,
        blocker_refs: blocker_refs.clone(),
        write_ticket_summary,
        evidence_summary,
        evidence_gate: Some(close_plan.evidence_gate),
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers,
        guarantee_display: Some(guarantee_display),
    })?;
    Ok((state, blocker_refs))
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
            MethodName::ResolveUserAction,
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
            let response =
                plan_error_response(&request.envelope, &prepared.context.project_state, error)?;
            return Ok(response.with_prepared_context(&prepared));
        }
    };
    if request.envelope.dry_run.is_requested() {
        return service.execute_prepared_request(
            prepared,
            dry_run_preview_branch::<ResolveUserActionRequest>(dry_run_summary(
                "user_action_resolution",
                "resolve_pending",
                "Request would immutably resolve one pending user action.",
                Vec::new(),
            )),
        );
    }
    let session_id = prepared.context.verified_invocation.session_id.clone();
    let response = service.execute_prepared_request(
        prepared,
        commit_mutation_branch::<ResolveUserActionRequest>(
            plan.method
                .operation
                .into_commit_branch::<ResolveUserActionRequest>(
                    plan.method.result_fields,
                    "user_action_resolved",
                ),
        ),
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
    method: ResolveUserActionMethodPlan,
}

struct ResolveUserActionMethodPlan {
    operation: OperationPlan,
    result_fields: ResolveUserActionResultFields,
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
        let exact = existing.user_action_request_id() == request.user_action_request_id.as_str()
            && existing.resolved_by_actor_source() == &verified_actor.actor_source
            && existing.resolved_verification_basis().as_str() == verified_actor.verification_basis
            && existing.resolved_assurance_level() == verified_actor.assurance_level
            && domain_resolution_input_matches_body(&request.resolution, existing.resolution());
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
    if effective.status() != UserActionStatus::Pending {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            match effective.status() {
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
        .is_some_and(|task_id| task_id.as_str() != effective.request().task_id())
    {
        return validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "envelope.task_id",
            "envelope.task_id must match the addressed user action Task",
        );
    }
    let task_id = TaskId::new(effective.request().task_id());
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
    let persisted = effective.request().request();
    let basis = effective.request().basis();
    validate_domain_resolution_basis(
        store,
        project_state.state_version,
        &task,
        current_change_unit.as_ref(),
        basis,
    )
    .map_err(|error| user_action_service_plan_error(&request.envelope, project_state, error))?;
    let resolution_id = allocate_user_action_resolution_id(service.durable_id_generator(), store)
        .map_err(PlanError::Core)?;
    let (resolution_body, mut derived_refs) = construct_domain_user_action_resolution(
        store,
        &UserActionConstructionContext {
            project_id: request.envelope.project_id.clone(),
            observed_state_version: project_state.state_version,
            observed_at: now.clone(),
            locale: request.envelope.locale.as_ref().cloned(),
        },
        &request.resolution,
        &persisted.body,
        basis,
        &task_id,
        current_change_unit.as_ref(),
    )
    .map_err(|error| user_action_service_plan_error(&request.envelope, project_state, error))?;
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
    let projected_shaping_disposition = shaping_disposition(&resolution_body);
    let materialized_resolution =
        materialize_user_action_resolution(UserActionResolutionMaterializationInput {
            user_action_resolution_id: resolution_id.clone(),
            user_action_request_id: &request.user_action_request_id,
            action_kind: effective.request().action_kind(),
            channel_kind,
            channel_submission_id: &request.channel_submission_id,
            resolution: resolution_body.clone(),
            actor_source: verified_actor.actor_source.clone(),
            verification_basis: channel_kind.verification_basis(),
            assurance_level: verified_actor.assurance_level.clone(),
            resolved_at: &now,
        })
        .map_err(|error| user_action_service_plan_error(&request.envelope, project_state, error))?;
    let projected_effective = materialized_resolution
        .project_record_set(&effective, &now)
        .map_err(|error| user_action_service_plan_error(&request.envelope, project_state, error))?;
    let resolution_record = projected_effective
        .resolution()
        .expect("a projected resolution record set must contain its resolution");
    let planned_state_version = project_state.state_version + 1;
    let public_request = user_action_from_record(&projected_effective, planned_state_version)
        .map_err(|error| user_action_service_plan_error(&request.envelope, project_state, error))?;
    let public_resolution = resolution_from_stored_record(resolution_record, &task_id)
        .map_err(|error| user_action_service_plan_error(&request.envelope, project_state, error))?;
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
        service.durable_id_generator(),
        store,
        project_state,
        &request.envelope,
        &task_id,
        current_change_unit.as_ref(),
        &persisted.body,
        basis,
        &resolution_body,
        &resolution_ref,
        &now,
    )
    .map_err(|error| match error {
        ContinuityPlanningError::Core(error) => PlanError::Core(error),
        ContinuityPlanningError::UserAction(error) => {
            user_action_service_plan_error(&request.envelope, project_state, error)
        }
    })?;
    derived_refs.extend(continuity_plans.iter().map(|plan| plan.record_ref.clone()));
    let mut pending_authorities = pending_user_action_authorities(store, &task_id, &now)
        .map_err(|error| user_action_service_plan_error(&request.envelope, project_state, error))?;
    pending_authorities
        .retain(|authority| authority.user_action_request_id != request.user_action_request_id);
    let lifecycle_phase = projected_user_action_lifecycle_phase(
        project_state,
        &task,
        current_change_unit.as_ref(),
        &pending_authorities,
    );
    let lifecycle_transition = lifecycle_phase
        .map(|target| {
            plan_user_action_lifecycle_transition(TaskLifecycleFacts::from(&task), target)
        })
        .transpose()?
        .flatten();
    let mut projected_task = task.clone();
    if let Some(transition) = lifecycle_transition.as_ref() {
        projected_task.lifecycle_phase = transition.target();
    }
    let mut projected_checkpoint = store
        .current_shaping_checkpoint(&task_id)
        .map_err(CorePipelineError::from)?;
    let shaping_linked = projected_checkpoint.as_mut().is_some_and(|checkpoint| {
        let mut matched = false;
        for gap in &mut checkpoint.gaps {
            if gap.user_action.as_ref().is_some_and(|link| {
                link.user_action_request_id == request.user_action_request_id.as_str()
            }) {
                gap.status = projected_shaping_disposition.unwrap_or(ShapingGapStatus::Current);
                if let Some(link) = gap.user_action.as_mut() {
                    link.user_action_resolution_id = Some(resolution_id.as_str().to_owned());
                    link.resolved_at = Some(now.clone());
                }
                matched = true;
            }
        }
        if matched
            && !checkpoint
                .gaps
                .iter()
                .any(|gap| gap.status == volicord_types::values::ShapingGapStatus::Current)
        {
            checkpoint.readiness = volicord_types::values::ShapingCheckpointReadiness::Ready;
        }
        matched
    });
    let (mut state, _blocker_refs) = projected_user_action_state(
        store,
        project_state,
        verified_invocation,
        &request.envelope,
        &projected_task,
        current_change_unit.as_ref(),
        &now,
        planned_state_version,
        Some(
            user_action_authority_from_record(&projected_effective).map_err(|error| {
                user_action_service_plan_error(&request.envelope, project_state, error)
            })?,
        ),
        None,
        Some(&request.user_action_request_id),
    )?;
    if shaping_linked {
        let checkpoint = projected_checkpoint
            .as_ref()
            .expect("a shaping-linked resolution must retain its current checkpoint");
        let mut task_wide_authority = crate::workflow_projection::task_wide_shaping_authority(
            store,
            &request.envelope.project_id,
            planned_state_version,
            &projected_task,
            current_change_unit.as_ref(),
            Some(checkpoint),
            &now,
        )?;
        for facts in [
            &mut task_wide_authority.awaiting_user,
            &mut task_wide_authority.accepted_unapplied,
            &mut task_wide_authority.recovery_required,
            &mut task_wide_authority.inconsistent,
        ] {
            facts.retain(|fact| {
                fact.request_ref.record_id.as_str() != request.user_action_request_id.as_str()
            });
        }
        let resolved_gap = checkpoint
            .gaps
            .iter()
            .find(|gap| {
                gap.user_action.as_ref().is_some_and(|link| {
                    link.user_action_request_id == request.user_action_request_id.as_str()
                })
            })
            .ok_or_else(|| CorePipelineError::Invariant {
                detail: "a shaping-linked resolution lost its exact gap".to_owned(),
            })?;
        let authority_state =
            projected_resolved_shaping_authority_state(&resolution_body, resolved_gap.status);
        let required_owner_method = match authority_state {
            ShapingDecisionAuthorityState::AcceptedUnapplied => resolved_gap
                .gap_kind
                .decision_policy_for_mode(task.mode)
                .map_or(MethodName::Status, |policy| {
                    policy.application_owner.method()
                }),
            ShapingDecisionAuthorityState::Rejected
            | ShapingDecisionAuthorityState::Deferred
            | ShapingDecisionAuthorityState::Expired => MethodName::RecordShapingCheckpoint,
            ShapingDecisionAuthorityState::AwaitingUser => MethodName::ResolveUserAction,
            ShapingDecisionAuthorityState::Stale => {
                if projected_task.work_phase == WorkPhase::Shaping {
                    MethodName::RecordShapingCheckpoint
                } else {
                    MethodName::CloseTask
                }
            }
            ShapingDecisionAuthorityState::Applied
            | ShapingDecisionAuthorityState::Superseded
            | ShapingDecisionAuthorityState::Inconsistent => MethodName::Status,
        };
        let fact = crate::workflow_projection::WorkflowUserActionFact {
            request_ref: request_ref.clone(),
            resolution_ref: Some(resolution_ref.clone()),
            status: UserActionStatus::Resolved,
            authority_state,
            required_owner_method,
        };
        match authority_state {
            ShapingDecisionAuthorityState::AcceptedUnapplied => {
                task_wide_authority.accepted_unapplied.push(fact)
            }
            ShapingDecisionAuthorityState::Rejected
            | ShapingDecisionAuthorityState::Deferred
            | ShapingDecisionAuthorityState::Expired => {
                task_wide_authority.recovery_required.push(fact)
            }
            _ => task_wide_authority.inconsistent.push(fact),
        }
        state.workflow = crate::workflow_projection::workflow_projection(
            &request.envelope.project_id,
            planned_state_version,
            &projected_task,
            current_change_unit.as_ref(),
            Some(checkpoint),
            &task_wide_authority,
        );
    }
    let result_fields = ResolveUserActionResultFields {
        user_action_request_ref: request_ref,
        user_action_resolution_ref: resolution_ref,
        user_action_request: public_request,
        user_action_resolution: public_resolution,
        derived_refs,
        state,
    };
    let mut storage_mutations = vec![materialized_resolution.mutation];
    if shaping_linked {
        let disposition =
            projected_shaping_disposition.ok_or_else(|| CorePipelineError::Invariant {
                detail: "a shaping-linked UserAction requires a choice decision disposition"
                    .to_owned(),
            })?;
        storage_mutations.push(volicord_store::core_pipeline::CoreStorageMutation::Shaping(
            volicord_store::core_pipeline::ShapingCheckpointMutation::ResolveLinkedGap {
                user_action_request_id: request.user_action_request_id.as_str().to_owned(),
                user_action_resolution_id: resolution_id.as_str().to_owned(),
                disposition,
            },
        ));
    }
    storage_mutations.extend(continuity_plans.into_iter().map(|plan| plan.mutation));
    if let Some(transition) = lifecycle_transition {
        storage_mutations.push(transition.storage_mutation());
    }
    Ok(ResolveUserActionPlan {
        method: ResolveUserActionMethodPlan {
            operation: OperationPlan::new(
                task_id,
                current_change_unit
                    .as_ref()
                    .map(|record| ChangeUnitId::new(record.change_unit_id.clone())),
                storage_mutations,
                object_from_value(json!({
                    "user_action_request_id": request.user_action_request_id,
                    "user_action_resolution_id": resolution_id,
                    "action_kind": effective.request().action_kind(),
                    "channel_kind": channel_kind,
                    "channel_submission_id": request.channel_submission_id,
                }))?,
            ),
            result_fields,
        },
    })
}
