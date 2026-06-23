use super::*;

impl CoreService {
    /// Executes `harness.update_scope` through the shared Core mutation pipeline.
    pub fn update_scope(
        &self,
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
            request.requested_access_class(),
            TaskRequirement::Exact(request.task_id.clone()),
            request.envelope.dry_run,
        );
        let prepared = match prepare_or_response(
            self,
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
            &prepared.context.verified_surface,
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
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: dry_run_summary(
                        "scope",
                        "commit",
                        "Scope update would update current Task scope and Change Unit state.",
                        plan.next_actions,
                    ),
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: "scope_updated".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            },
        )
    }
}

fn plan_update_scope(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: UpdateScopeRequest,
    verified_surface: &VerifiedSurfaceContext,
) -> Result<MethodPlan, PlanError> {
    let planned_state_version = project_state.state_version + 1;
    let plan_now = utc_timestamp(service.now());
    let task = store
        .task_record(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    let current_change_unit = store
        .current_change_unit(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let linked_scope_decision_refs = validate_related_scope_decisions(
        store,
        project_state,
        &request,
        current_change_unit.as_ref(),
        task.scope_revision,
        &plan_now,
    )?;

    let current_scope = StoredScope::from_task(&task)?;
    let next_scope = current_scope.apply_request(&request);
    let scope_changed = current_scope != next_scope
        || request.change_unit.operation == ChangeUnitOperation::CreateCurrent
        || request.change_unit.operation == ChangeUnitOperation::ReplaceCurrent;
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

    let active_write_authorizations = store
        .active_write_authorizations(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let stale_write_authorization_refs = if scope_changed {
        active_write_authorizations
            .iter()
            .map(|record| write_authorization_ref(record, planned_state_version))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let mut storage_mutations = vec![CoreStorageMutation::UpdateTaskScope(TaskScopeUpdate {
        task_id: task.task_id.clone(),
        lifecycle_phase: None,
        result: None,
        title: next_scope.goal_summary.clone(),
        summary: next_scope.goal_summary.clone(),
        shaping_summary_json: Some(serde_json::to_string(&next_scope.to_json())?),
        bounded_context_json: Some(serde_json::to_string(&json!({
            "scope_update": request.scope_update.clone()
        }))?),
        autonomy_boundary_json: Some(serde_json::to_string(&json!({
            "autonomy_boundary": next_scope.autonomy_boundary
        }))?),
        close_summary_json: None,
        completion_policy_json: None,
    })];
    if scope_changed {
        storage_mutations.push(CoreStorageMutation::UpdateTaskScopeRevision(
            TaskScopeRevisionUpdate {
                task_id: task.task_id.clone(),
                scope_revision: next_scope_revision,
            },
        ));
        storage_mutations.push(CoreStorageMutation::UpdateTaskCloseBasis(
            TaskCloseBasisUpdate {
                task_id: task.task_id.clone(),
                close_basis_revision: next_close_basis_revision,
                close_basis_json: None,
            },
        ));
    }

    let mut synthetic_task = task.clone();
    synthetic_task.title = next_scope.goal_summary.clone();
    synthetic_task.summary = next_scope.goal_summary.clone();
    synthetic_task.shaping_summary_json = serde_json::to_string(&next_scope.to_json())?;
    synthetic_task.bounded_context_json = serde_json::to_string(&json!({
        "scope_update": request.scope_update.clone()
    }))?;
    synthetic_task.autonomy_boundary_json = serde_json::to_string(&json!({
        "autonomy_boundary": next_scope.autonomy_boundary
    }))?;

    let (change_unit_ref, synthetic_change_unit, branch_change_unit_id) =
        match request.change_unit.operation {
            ChangeUnitOperation::KeepCurrent => {
                let change_unit_ref = current_change_unit.as_ref().map(|record| {
                    state_ref(
                        StateRecordKind::ChangeUnit,
                        &record.change_unit_id,
                        &request.envelope.project_id,
                        Some(&request.task_id),
                        Some(record.basis_state_version.unwrap_or(planned_state_version)),
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
                    let response = validation_rejected(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        "change_unit.operation",
                        "create_current requires no current Change Unit",
                    )
                    .map_err(PlanError::Core)?;
                    return Err(PlanError::Response(Box::new(response)));
                }
                let change_unit_id =
                    allocate_change_unit_id(service, store).map_err(PlanError::Core)?;
                let insert = change_unit_insert(&request, &change_unit_id)?;
                let record = synthetic_change_unit_record(
                    &request.envelope.project_id,
                    &request.task_id,
                    &insert,
                    planned_state_version,
                );
                storage_mutations.push(CoreStorageMutation::InsertCurrentChangeUnit(insert));
                synthetic_task.current_change_unit_id = Some(change_unit_id.as_str().to_owned());
                synthetic_task.lifecycle_phase = "ready".to_owned();
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
                let change_unit_id =
                    allocate_change_unit_id(service, store).map_err(PlanError::Core)?;
                let insert = change_unit_insert(&request, &change_unit_id)?;
                let record = synthetic_change_unit_record(
                    &request.envelope.project_id,
                    &request.task_id,
                    &insert,
                    planned_state_version,
                );
                storage_mutations.push(CoreStorageMutation::ReplaceCurrentChangeUnit(insert));
                synthetic_task.current_change_unit_id = Some(change_unit_id.as_str().to_owned());
                synthetic_task.lifecycle_phase = "ready".to_owned();
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

    if scope_changed && !active_write_authorizations.is_empty() {
        storage_mutations.push(CoreStorageMutation::MarkActiveWriteAuthorizationsStale {
            task_id: request.task_id.as_str().to_owned(),
        });
    }
    if scope_changed {
        storage_mutations.push(CoreStorageMutation::MarkUserJudgmentsSupersededOrStale(
            UserJudgmentInvalidation {
                task_id: request.task_id.as_str().to_owned(),
                judgment_kinds: Vec::new(),
            },
        ));
    }

    let pending_refs = if scope_changed {
        Vec::new()
    } else {
        store
            .pending_user_judgment_refs(&request.task_id, planned_state_version)
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?
            .into_iter()
            .map(state_ref_from_stored)
            .collect::<Vec<_>>()
    };
    let blocker_refs = store
        .active_blocker_refs(&request.task_id, planned_state_version)
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
    let task_ref = state_ref(
        StateRecordKind::Task,
        request.task_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let next_actions = next_actions_for_state(&task_ref, change_unit_ref.as_ref());
    let guarantee_display =
        guarantee_display_for_surface(store, verified_surface, planned_state_version)?;
    let write_authority_summary = projected_write_authority_summary(
        store,
        &request.task_id,
        planned_state_version,
        service.now(),
        Some(guarantee_display.clone()),
    )?;
    let evidence_summary = projected_evidence_summary(
        store,
        &request.envelope.project_id,
        planned_state_version,
        &synthetic_task,
    )?;
    let projected_project_state = project_state_projection(
        project_state,
        planned_state_version,
        project_state
            .active_task_id
            .clone()
            .or_else(|| Some(request.task_id.as_str().to_owned())),
    );
    let close_plan = projected_close_check(
        store,
        &projected_project_state,
        verified_surface,
        &request.envelope,
        &request.task_id,
        close_context_from_projection(
            synthetic_task.clone(),
            synthetic_change_unit.clone(),
            if scope_changed {
                None
            } else {
                projected_close_basis(store, &request.task_id)?
            },
            pending_refs.clone(),
            blocker_refs.clone(),
            evidence_summary.clone(),
        ),
        *plan_now.as_datetime(),
    )?;
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &synthetic_task,
        current_change_unit: synthetic_change_unit.as_ref(),
        pending_user_judgment_refs: pending_refs,
        blocker_refs: blocker_refs.clone(),
        write_authority_summary,
        evidence_summary,
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers,
        guarantee_display: Some(guarantee_display),
    })?;
    let result = volicord_types::UpdateScopeResult {
        base: placeholder_base(),
        task_ref,
        change_unit_ref,
        linked_scope_decision_refs,
        stale_write_authorization_refs,
        blocker_refs,
        state,
        next_actions: next_actions.clone(),
    };
    let event_payload = object_from_value(json!({
        "task_id": request.task_id.clone(),
        "change_unit_operation": request.change_unit.operation,
        "scope_changed": scope_changed,
        "scope_revision": next_scope_revision,
        "close_basis_revision": next_close_basis_revision
    }))?;

    Ok(MethodPlan {
        task_id: request.task_id,
        change_unit_id: branch_change_unit_id,
        storage_mutations,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions,
    })
}

fn validate_related_scope_decisions(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &UpdateScopeRequest,
    current_change_unit: Option<&ChangeUnitRecord>,
    scope_revision: u64,
    now: &UtcTimestamp,
) -> Result<Vec<StateRecordRef>, PlanError> {
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
            current_change_unit.basis_state_version,
        ));
    }
    let requirement = ScopeDecisionAuthorityRequirement {
        task_id: &request.task_id,
        scope_revision,
        current_change_unit_id: current_change_unit_id.as_ref(),
        affected_refs: &transition_refs,
        now,
    };
    let mut linked_scope_decision_refs = Vec::new();
    for related_ref in &request.related_scope_decision_refs {
        if related_ref.record_kind != StateRecordKind::UserJudgment
            || related_ref.project_id != request.envelope.project_id
            || related_ref.task_id.as_ref() != Some(&request.task_id)
        {
            return validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "related_scope_decision_refs",
                "related scope decision refs must identify user judgments for this Task",
            )
            .map(|()| Vec::new());
        }
        let record = store
            .user_judgment_record(related_ref.record_id.as_str())
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?
            .ok_or_else(|| {
                PlanError::Response(Box::new(decision_rejected_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "related scope decision judgment is missing",
                )))
            })?;
        let authority = user_judgment_authority_from_record(&record)?;
        if !accepted_current_scope_decision_authority(&authority, &requirement) {
            return Err(PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "related scope decision judgment is not current",
            ))));
        }
        linked_scope_decision_refs.push(related_ref.clone());
    }
    Ok(linked_scope_decision_refs)
}
