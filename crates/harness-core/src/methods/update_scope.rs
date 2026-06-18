use super::*;

impl CoreService {
    /// Executes `harness.update_scope` through the shared Core mutation pipeline.
    pub fn update_scope(
        &self,
        request: UpdateScopeRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = &request.envelope.task_id {
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
            AccessClass::CoreMutation,
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
) -> Result<MethodPlan, PlanError> {
    let planned_state_version = project_state.state_version + 1;
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

    let current_scope = StoredScope::from_task(&task);
    let next_scope = current_scope.apply_request(&request);
    let scope_changed = current_scope != next_scope
        || request.change_unit.operation == ChangeUnitOperation::CreateCurrent
        || request.change_unit.operation == ChangeUnitOperation::ReplaceCurrent;

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

    let pending_refs = store
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
        .collect::<Vec<_>>();
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
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &synthetic_task,
        current_change_unit: synthetic_change_unit.as_ref(),
        pending_user_judgment_refs: pending_refs,
        blocker_refs: blocker_refs.clone(),
        active_write_authorization: None,
        options: SummaryOptions::mutation(),
    })?;
    let result = harness_types::UpdateScopeResult {
        base: placeholder_base(),
        task_ref,
        change_unit_ref,
        linked_scope_decision_refs: request.related_scope_decision_refs.clone(),
        stale_write_authorization_refs,
        blocker_refs,
        state,
        next_actions: next_actions.clone(),
    };
    let event_payload = object_from_value(json!({
        "task_id": request.task_id.clone(),
        "change_unit_operation": request.change_unit.operation,
        "scope_changed": scope_changed
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
