use super::*;

impl CoreService {
    /// Executes `volicord.update_scope` through the shared Core mutation pipeline.
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
            request.operation_category(),
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
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &UtcTimestamp,
) -> Result<MethodPlan, PlanError> {
    let planned_state_version = project_state.state_version + 1;
    let plan_now = operation_now.clone();
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
    validate_requested_effect_contract(store, project_state, &request)?;
    let linked_scope_decision_refs = validate_related_scope_decisions(
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
        project_state,
        &request.envelope,
        &plan_now,
        &operation_context,
    )?
    .is_empty()
    {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "a current pending user action must be resolved before this scope update",
        ))));
    }

    let current_scope = StoredScope::from_task(&task)?;
    let next_scope = current_scope.apply_request(&request);
    if request.change_unit.operation == ChangeUnitOperation::KeepCurrent
        && current_change_unit.is_some()
        && current_scope.baseline_ref != next_scope.baseline_ref
    {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "baseline_ref",
            "changing the Task baseline while a current Change Unit exists requires replace_current",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    let (acceptance_criteria, acceptance_criteria_mutation, acceptance_criteria_changed) =
        plan_acceptance_criteria_replacement(service, store, project_state, &request)?;
    let scope_changed = current_scope != next_scope
        || acceptance_criteria_changed
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

    let active_write_tickets = store
        .active_write_tickets(&request.task_id)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?;
    let stale_write_ticket_refs = if scope_changed {
        active_write_tickets
            .iter()
            .map(|record| write_ticket_ref(record, planned_state_version))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let task_mode = parse_task_mode(&task.mode)?;
    let mut storage_mutations = vec![CoreStorageMutation::UpdateTaskScope(TaskScopeUpdate {
        task_id: task.task_id.clone(),
        work_phase: matches!(
            request.change_unit.operation,
            ChangeUnitOperation::CreateCurrent | ChangeUnitOperation::ReplaceCurrent
        )
        .then(|| work_phase_storage(WorkPhase::Implementation).to_owned())
        .filter(|_| task_mode != TaskMode::Advisor),
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
    })];
    if let Some(mutation) = acceptance_criteria_mutation {
        storage_mutations.push(CoreStorageMutation::ReplaceAcceptanceCriteria(mutation));
    }
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
    synthetic_task.scope_revision = next_scope_revision;
    synthetic_task.close_basis_revision = next_close_basis_revision;
    if scope_changed {
        synthetic_task.close_basis_json = None;
    }
    synthetic_task.title = next_scope.goal_summary.clone();
    synthetic_task.summary = next_scope.goal_summary.clone();
    synthetic_task.shaping_summary_json = serde_json::to_string(&next_scope.to_json())?;
    synthetic_task.bounded_context_json = serde_json::to_string(&json!({
        "scope_update": request.scope_update.clone()
    }))?;
    synthetic_task.autonomy_boundary_json = serde_json::to_string(&json!({
        "autonomy_boundary": next_scope.autonomy_boundary
    }))?;
    if task_mode != TaskMode::Advisor
        && matches!(
            request.change_unit.operation,
            ChangeUnitOperation::CreateCurrent | ChangeUnitOperation::ReplaceCurrent
        )
    {
        synthetic_task.work_phase = work_phase_storage(WorkPhase::Implementation).to_owned();
    }

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
                let insert = change_unit_insert(&request, &change_unit_id, verified_invocation)?;
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
                let insert = change_unit_insert(&request, &change_unit_id, verified_invocation)?;
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

    if scope_changed && !active_write_tickets.is_empty() {
        storage_mutations.push(CoreStorageMutation::MarkActiveWriteTicketsStale {
            task_id: request.task_id.as_str().to_owned(),
        });
    }
    if scope_changed {
        storage_mutations.push(CoreStorageMutation::MarkUserActionsSupersededOrStale(
            UserActionInvalidation {
                task_id: request.task_id.as_str().to_owned(),
                action_kinds: Vec::new(),
            },
        ));
        if let Some(lifecycle_phase) = projected_user_action_lifecycle_phase(
            project_state,
            &task,
            synthetic_change_unit.as_ref(),
            &[],
        ) {
            synthetic_task.lifecycle_phase = lifecycle_phase.to_owned();
            storage_mutations.push(task_lifecycle_mutation(&request.task_id, lifecycle_phase));
        }
    }

    let pending_refs = if scope_changed {
        Vec::new()
    } else {
        store
            .pending_user_action_refs(&request.task_id, planned_state_version, &plan_now)
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
    let next_actions = next_actions_for_state(
        parse_task_mode(&synthetic_task.mode)?,
        &task_ref,
        change_unit_ref.as_ref(),
        planned_state_version,
    );
    let guarantee_display =
        guarantee_display_for_invocation(store, verified_invocation, planned_state_version)?;
    let write_ticket_summary = projected_write_ticket_summary(
        store,
        &request.task_id,
        planned_state_version,
        *plan_now.as_datetime(),
        Some(guarantee_display.clone()),
    )?;
    let projected_current_close_basis = if scope_changed {
        None
    } else {
        projected_close_basis(store, &request.task_id)?
    };
    let evidence_summary = projected_evidence_summary_for_criteria(
        store,
        &request.envelope.project_id,
        planned_state_version,
        &synthetic_task,
        &acceptance_criteria,
    )?
    .map(|summary| evidence_summary_for_display(summary, projected_current_close_basis.as_ref()));
    let close_evidence_summary = if scope_changed {
        evidence_summary_with_required_criteria(None, &acceptance_criteria)
    } else {
        evidence_summary.clone()
    };
    let projected_project_state = project_state_projection(
        project_state,
        planned_state_version,
        project_state
            .active_task_id
            .clone()
            .or_else(|| Some(request.task_id.as_str().to_owned())),
    );
    let close_context = close_context_with_projected_acceptance_criteria(
        close_context_from_projection(
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
        close_context_with_pending_authorities(close_context, Vec::new())
    } else {
        close_context
    };
    let close_plan = projected_close_check(
        store,
        &projected_project_state,
        verified_invocation,
        &request.envelope,
        &request.task_id,
        close_context,
        *plan_now.as_datetime(),
    )?;
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &synthetic_task,
        current_change_unit: synthetic_change_unit.as_ref(),
        acceptance_criteria,
        pending_user_action_refs: pending_refs,
        blocker_refs: blocker_refs.clone(),
        write_ticket_summary,
        evidence_summary,
        evidence_gate: Some(close_plan.evidence_gate),
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers,
        guard_health: close_plan.guard_health,
        guarantee_display: Some(guarantee_display),
    })?;
    let result = volicord_types::UpdateScopeResult {
        base: placeholder_base(),
        task_ref,
        change_unit_ref,
        linked_scope_decision_refs,
        stale_write_ticket_refs,
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
    let current = active_acceptance_criteria_for_task(store, &request.task_id)?;
    let Some(replacements) = request.acceptance_criteria.as_ref() else {
        return Ok((current, None, false));
    };

    let mut seen_ids = BTreeSet::new();
    let mut projected = Vec::with_capacity(replacements.len());
    let mut upserts = Vec::with_capacity(replacements.len());
    for (position, replacement) in replacements.iter().enumerate() {
        let statement = normalize_display_text(&replacement.statement);
        if statement.is_empty() {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "acceptance_criteria[].statement",
                "acceptance criterion statements must not be empty",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        let acceptance_criterion_id = match replacement.acceptance_criterion_id.as_ref() {
            Some(id) => {
                if !seen_ids.insert(id.as_str().to_owned()) {
                    validation_plan_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        "acceptance_criteria[].acceptance_criterion_id",
                        "acceptance criterion replacement IDs must not be duplicated",
                    )?;
                    unreachable!("validation_plan_error always returns Err");
                }
                let record = store
                    .acceptance_criterion_record(id.as_str())
                    .map_err(CorePipelineError::from)?;
                let Some(record) = record else {
                    validation_plan_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        "acceptance_criteria[].acceptance_criterion_id",
                        "acceptance criterion replacement ID is unknown",
                    )?;
                    unreachable!("validation_plan_error always returns Err");
                };
                if record.task_id != request.task_id.as_str() {
                    validation_plan_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        "acceptance_criteria[].acceptance_criterion_id",
                        "acceptance criterion replacement ID belongs to another Task",
                    )?;
                    unreachable!("validation_plan_error always returns Err");
                }
                if record.status != "active" {
                    validation_plan_error(
                        request.envelope.dry_run,
                        Some(project_state.state_version),
                        "acceptance_criteria[].acceptance_criterion_id",
                        "retired acceptance criterion IDs cannot be reused",
                    )?;
                    unreachable!("validation_plan_error always returns Err");
                }
                id.clone()
            }
            None => {
                let id = allocate_acceptance_criterion_id(service, store, &seen_ids)
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
            evidence_requirement: storage_value(replacement.evidence_requirement)?,
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
) -> Result<(), PlanError> {
    let Some(contract) = request.change_unit.effect_contract.as_ref() else {
        return Ok(());
    };
    match validate_effect_contract(contract) {
        Ok(()) => {}
        Err(EffectContractValidationError::ConflictingEffect(_)) => {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "change_unit.effect_contract",
                "effect_contract cannot list the same effect as both allowed and forbidden",
            )?;
        }
        Err(EffectContractValidationError::EmptyText(field)) => {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                field,
                "effect_contract string list entries must not be empty",
            )?;
        }
    }

    match validate_effect_contract_paths(&store.project_record().repo_root, contract) {
        Ok(()) => Ok(()),
        Err(ProductPathError::Invalid) => {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "change_unit.effect_contract.allowed_paths",
                "effect_contract.allowed_paths must be relative Product Repository paths that stay inside the repository",
            )?;
            unreachable!("validation_plan_error always returns Err")
        }
        Err(ProductPathError::LocalAccess) => {
            let response = rejected_pipeline_response(
                request.envelope.dry_run,
                Some(project_state.state_version),
                vec![tool_error(
                    ErrorCode::InvocationContextMismatch,
                    "effect_contract.allowed_paths resolve outside the Product Repository",
                    false,
                    None,
                )],
            )
            .map_err(PlanError::Core)?;
            Err(PlanError::Response(Box::new(response)))
        }
    }
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
        if related_ref.record_kind != StateRecordKind::UserActionResolution
            || related_ref.project_id != request.envelope.project_id
            || related_ref.task_id.as_ref() != Some(&request.task_id)
        {
            return validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "related_scope_decision_refs",
                "related scope decision refs must identify user-action resolutions for this Task",
            )
            .map(|()| Vec::new());
        }
        let resolution = store
            .user_action_resolution_record(related_ref.record_id.as_str())
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
                    "related scope decision resolution is missing",
                )))
            })?;
        let record = store
            .user_action_record(&resolution.user_action_request_id, now)
            .map_err(CorePipelineError::from)?
            .ok_or_else(|| {
                PlanError::Response(Box::new(decision_rejected_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "related scope decision request is missing",
                )))
            })?;
        let authority = user_action_authority_from_record(&record)?;
        if !accepted_current_scope_decision_authority(&authority, &requirement) {
            return Err(PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "related scope decision resolution is not current",
            ))));
        }
        linked_scope_decision_refs.push(related_ref.clone());
    }
    Ok(linked_scope_decision_refs)
}
