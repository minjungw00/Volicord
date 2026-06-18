use super::*;

impl CoreService {
    /// Executes `harness.intake` through the shared Core mutation pipeline.
    pub fn intake(
        &self,
        request: harness_types::IntakeRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        let policy = mutation_method_policy(
            AccessClass::CoreMutation,
            TaskRequirement::None,
            request.envelope.dry_run,
        );
        let prepared = match prepare_or_response(
            self,
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
            &prepared.context.verified_surface,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                return core_error_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    error,
                )
            }
        };

        if request.envelope.dry_run {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: dry_run_summary(
                        "task",
                        "commit",
                        "Intake would select or create a Task.",
                        plan.next_actions,
                    ),
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: "task_intake".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: None,
                storage_mutations: plan.storage_mutations,
            },
        )
    }
}

fn plan_intake(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: harness_types::IntakeRequest,
    verified_surface: &VerifiedSurfaceContext,
) -> CoreResult<MethodPlan> {
    let planned_state_version = project_state.state_version + 1;
    let mode = resolve_requested_mode(request.requested_mode);
    let active_task = store.active_task_record()?;

    let create_new = match request.resume_policy {
        ResumePolicy::ResumeActive => active_task.is_none(),
        ResumePolicy::CreateNew | ResumePolicy::RejectIfActive => true,
        ResumePolicy::SupersedeActive => true,
    };
    let task_id = if create_new {
        match request.envelope.task_id.clone() {
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

    let mut storage_mutations = Vec::new();
    if request.resume_policy == ResumePolicy::SupersedeActive {
        if let Some(active) = &active_task {
            storage_mutations.push(CoreStorageMutation::SupersedeTask {
                task_id: active.task_id.clone(),
            });
        }
    }

    let task_record = if create_new {
        let shaping_summary = task_shaping_json(
            Some(request.plain_language_request.clone()),
            Some(request.initial_scope.boundary.clone()),
            request.initial_scope.non_goals.clone(),
            request.initial_scope.acceptance_criteria.clone(),
            None,
            None,
            Some(serde_json::to_value(&request.initial_context_refs)?),
        );
        let task = TaskRecord {
            project_id: request.envelope.project_id.as_str().to_owned(),
            task_id: task_id.as_str().to_owned(),
            mode: task_mode_storage(mode).to_owned(),
            lifecycle_phase: "shaping".to_owned(),
            result: Some("none".to_owned()),
            title: Some(request.plain_language_request.clone()),
            summary: Some(request.plain_language_request.clone()),
            shaping_summary_json: serde_json::to_string(&shaping_summary)?,
            bounded_context_json: serde_json::to_string(&json!({
                "initial_context_refs": request.initial_context_refs
            }))?,
            autonomy_boundary_json: serde_json::to_string(&json!({
                "autonomy_boundary": Value::Null
            }))?,
            close_summary_json: serde_json::to_string(&json!({
                "close_reason": "none"
            }))?,
            completion_policy_json: "{}".to_owned(),
            current_change_unit_id: None,
            closed_at: None,
        };
        storage_mutations.push(CoreStorageMutation::InsertTask(TaskInsert {
            task_id: task.task_id.clone(),
            created_by_surface_id: verified_surface.surface_id.as_str().to_owned(),
            created_by_surface_instance_id: verified_surface
                .surface_instance_id
                .as_str()
                .to_owned(),
            mode: task.mode.clone(),
            lifecycle_phase: task.lifecycle_phase.clone(),
            result: task.result.clone(),
            title: task.title.clone(),
            summary: task.summary.clone(),
            shaping_summary_json: task.shaping_summary_json.clone(),
            bounded_context_json: task.bounded_context_json.clone(),
            autonomy_boundary_json: task.autonomy_boundary_json.clone(),
            close_summary_json: task.close_summary_json.clone(),
            completion_policy_json: task.completion_policy_json.clone(),
            current_change_unit_id: None,
        }));
        storage_mutations.push(CoreStorageMutation::SetActiveTask {
            task_id: task.task_id.clone(),
        });
        task
    } else {
        active_task.expect("active_task exists when create_new is false")
    };

    let task_ref = state_ref(
        StateRecordKind::Task,
        &task_record.task_id,
        &request.envelope.project_id,
        Some(&task_id),
        Some(planned_state_version),
    );
    let pending_refs = Vec::new();
    let blocker_refs = Vec::new();
    let next_actions = next_actions_for_state(&task_ref, None);
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task_record,
        current_change_unit: None,
        pending_user_judgment_refs: pending_refs,
        blocker_refs,
        active_write_authorization: None,
        options: SummaryOptions::mutation(),
    })?;
    let result = harness_types::IntakeResult {
        base: placeholder_base(),
        task_ref: task_ref.clone(),
        change_unit_ref: None,
        state,
        next_actions: next_actions.clone(),
    };
    let event_payload = object_from_value(json!({
        "task_id": task_id,
        "resume_policy": request.resume_policy,
        "requested_mode": request.requested_mode,
        "resolved_mode": mode
    }))?;
    Ok(MethodPlan {
        task_id,
        change_unit_id: None,
        storage_mutations,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions,
    })
}
