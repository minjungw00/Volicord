use super::*;

impl CoreService {
    /// Executes `harness.request_user_judgment` through the shared Core mutation pipeline.
    pub fn request_user_judgment(
        &self,
        request: harness_types::RequestUserJudgmentRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = &request.envelope.task_id {
            if envelope_task_id != &request.task_id {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match RequestUserJudgmentRequest.task_id",
                );
            }
        }
        let prepared = match prepare_or_response(
            self,
            MethodName::RequestUserJudgment,
            request.envelope.clone(),
            request_json,
            invocation,
            mutation_method_policy(
                request.requested_access_class(),
                TaskRequirement::Exact(request.task_id.clone()),
                request.envelope.dry_run,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan = match plan_request_user_judgment(
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
                        "user_judgment",
                        "create_pending",
                        "Request would create one pending user-owned judgment.",
                        plan.next_actions,
                    ),
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: "user_judgment_requested".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            },
        )
    }

    /// Executes `harness.record_user_judgment` through the shared Core mutation pipeline.
    pub fn record_user_judgment(
        &self,
        request: RecordUserJudgmentRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        let task_requirement = request
            .envelope
            .task_id
            .clone()
            .map(TaskRequirement::Exact)
            .unwrap_or(TaskRequirement::None);
        let prepared = match prepare_or_response(
            self,
            MethodName::RecordUserJudgment,
            request.envelope.clone(),
            request_json,
            invocation,
            mutation_method_policy(
                request.requested_access_class(),
                task_requirement,
                request.envelope.dry_run,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let plan = match plan_record_user_judgment(
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
                        "user_judgment",
                        "resolve_pending",
                        "Request would record the user's answer for one pending judgment.",
                        plan.next_actions,
                    ),
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: "user_judgment_recorded".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: plan.change_unit_id,
                storage_mutations: plan.storage_mutations,
            },
        )
    }
}

fn plan_request_user_judgment(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: harness_types::RequestUserJudgmentRequest,
    verified_surface: &VerifiedSurfaceContext,
) -> Result<MethodPlan, PlanError> {
    validate_user_judgment_request_fields(UserJudgmentRequestValidation {
        dry_run: request.envelope.dry_run,
        state_version: Some(project_state.state_version),
        question: &request.question,
        options: &request.options,
        context: &request.context,
        affected_refs: &request.affected_refs,
        project_id: &request.envelope.project_id,
        task_id: &request.task_id,
        expires_at: request.expires_at.as_deref(),
        current_timestamp: store.current_timestamp().map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?,
    })?;

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
    let branch_change_unit_id = if let Some(change_unit_id) = &request.change_unit_id {
        let existing = store
            .change_unit_record(&request.task_id, change_unit_id.as_str())
            .map_err(|error| {
                PlanError::Response(Box::new(store_error_response(
                    &request.envelope,
                    project_state,
                    error,
                )))
            })?;
        if existing.is_none() {
            let response = rejected_pipeline_response(
                request.envelope.dry_run,
                Some(project_state.state_version),
                vec![tool_error(
                    ErrorCode::NoActiveChangeUnit,
                    "change_unit_id does not identify a Change Unit for the Task",
                    false,
                    None,
                )],
            )
            .map_err(PlanError::Core)?;
            return Err(PlanError::Response(Box::new(response)));
        }
        Some(change_unit_id.clone())
    } else {
        None
    };

    let requested_at = store.current_timestamp().map_err(|error| {
        PlanError::Response(Box::new(store_error_response(
            &request.envelope,
            project_state,
            error,
        )))
    })?;
    let judgment_id = allocate_user_judgment_id(service, store).map_err(PlanError::Core)?;
    let user_judgment_ref = state_ref(
        StateRecordKind::UserJudgment,
        judgment_id.as_str(),
        &request.envelope.project_id,
        Some(&request.task_id),
        Some(planned_state_version),
    );
    let user_judgment = UserJudgment {
        judgment_id: judgment_id.clone(),
        project_id: request.envelope.project_id.clone(),
        task_id: request.task_id.clone(),
        change_unit_id: request.change_unit_id.clone(),
        judgment_kind: request.judgment_kind,
        status: UserJudgmentStatus::Pending,
        presentation: request.presentation,
        question: request.question.clone(),
        options: request.options.clone(),
        context: request.context.clone(),
        affected_refs: request.affected_refs.clone(),
        required_for: request.required_for,
        resolution: None,
        expires_at: request.expires_at.clone(),
        created_at: requested_at.clone(),
        resolved_at: None,
    };

    let mut pending_refs = store
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
    pending_refs.push(user_judgment_ref.clone());
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
    let change_unit_ref = current_change_unit.as_ref().map(|record| {
        state_ref(
            StateRecordKind::ChangeUnit,
            &record.change_unit_id,
            &request.envelope.project_id,
            Some(&request.task_id),
            Some(record.basis_state_version.unwrap_or(planned_state_version)),
        )
    });
    let next_actions = next_actions_for_state(&task_ref, change_unit_ref.as_ref());
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task,
        current_change_unit: current_change_unit.as_ref(),
        pending_user_judgment_refs: pending_refs,
        blocker_refs: blocker_refs.clone(),
        active_write_authorization: None,
        options: SummaryOptions::mutation(),
    })?;
    let result = harness_types::RequestUserJudgmentResult {
        base: placeholder_base(),
        user_judgment_ref: user_judgment_ref.clone(),
        user_judgment,
        blocker_refs,
        state,
    };
    let storage_mutations = vec![CoreStorageMutation::InsertUserJudgment(
        UserJudgmentInsert {
            judgment_id: judgment_id.as_str().to_owned(),
            task_id: request.task_id.as_str().to_owned(),
            change_unit_id: request
                .change_unit_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            judgment_kind: storage_value(request.judgment_kind)?,
            request_json: serde_json::to_string(&json!({
                "presentation": request.presentation,
                "question": request.question,
                "required_for": request.required_for,
                "expires_at": request.expires_at
            }))?,
            context_json: serde_json::to_string(&request.context)?,
            options_json: serde_json::to_string(&request.options)?,
            affected_refs_json: serde_json::to_string(&request.affected_refs)?,
            artifact_refs_json: serde_json::to_string(&request.context.artifact_refs)?,
            sensitive_action_scope_json: "{}".to_owned(),
            requested_by_surface_id: verified_surface.surface_id.as_str().to_owned(),
            requested_by_surface_instance_id: verified_surface
                .surface_instance_id
                .as_str()
                .to_owned(),
            requested_at,
            metadata_json: serde_json::to_string(&json!({
                "requested_by_actor_kind": request.envelope.actor_kind
            }))?,
        },
    )];
    let event_payload = object_from_value(json!({
        "task_id": request.task_id,
        "change_unit_id": request.change_unit_id,
        "judgment_id": judgment_id,
        "judgment_kind": request.judgment_kind,
        "required_for": request.required_for
    }))?;

    Ok(MethodPlan {
        task_id: user_judgment_ref
            .task_id
            .clone()
            .expect("user judgment refs are task-scoped"),
        change_unit_id: branch_change_unit_id,
        storage_mutations,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions,
    })
}

fn plan_record_user_judgment(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: RecordUserJudgmentRequest,
) -> Result<MethodPlan, PlanError> {
    let planned_state_version = project_state.state_version + 1;
    let record = store
        .user_judgment_record(request.user_judgment_id.as_str())
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
                "user_judgment_id does not identify a pending user-owned judgment",
            )))
        })?;
    if let Some(envelope_task_id) = &request.envelope.task_id {
        if envelope_task_id.as_str() != record.task_id {
            let response = validation_rejected(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "task_id",
                "envelope.task_id must match the addressed UserJudgment task_id",
            )
            .map_err(PlanError::Core)?;
            return Err(PlanError::Response(Box::new(response)));
        }
    }
    if record.status != "pending" {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "user_judgment_id does not identify a pending user-owned judgment",
        ))));
    }

    let mut user_judgment = user_judgment_from_record(&record)?;
    let now = store.current_timestamp().map_err(|error| {
        PlanError::Response(Box::new(store_error_response(
            &request.envelope,
            project_state,
            error,
        )))
    })?;
    if user_judgment
        .expires_at
        .as_deref()
        .is_some_and(|expires_at| expires_at <= now.as_str())
    {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "pending user-owned judgment is expired",
        ))));
    }
    if request.judgment_kind != user_judgment.judgment_kind {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "judgment_kind is incompatible with the pending user-owned judgment",
        ))));
    }
    if !user_judgment
        .options
        .iter()
        .any(|option| option.option_id == request.selected_option_id)
    {
        let response = validation_rejected(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "selected_option_id",
            "selected_option_id is not one of the pending judgment options",
        )
        .map_err(PlanError::Core)?;
        return Err(PlanError::Response(Box::new(response)));
    }
    validate_answer_payload(
        request.envelope.dry_run,
        Some(project_state.state_version),
        request.judgment_kind,
        &request.answer,
    )?;

    let task_id = TaskId::new(record.task_id.clone());
    let task = store
        .task_record(&task_id)
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
    let current_change_unit = store.current_change_unit(&task_id).map_err(|error| {
        PlanError::Response(Box::new(store_error_response(
            &request.envelope,
            project_state,
            error,
        )))
    })?;
    let resolution = UserJudgmentResolution {
        selected_option_id: request.selected_option_id.clone(),
        answer: request.answer.clone(),
        note: request.note.clone(),
        accepted_risks: request.accepted_risks.clone(),
        resolved_by_actor_kind: request.envelope.actor_kind,
    };
    user_judgment.status = UserJudgmentStatus::Resolved;
    user_judgment.resolution = Some(resolution.clone());
    user_judgment.resolved_at = Some(now.clone());

    let user_judgment_ref = state_ref(
        StateRecordKind::UserJudgment,
        request.user_judgment_id.as_str(),
        &request.envelope.project_id,
        Some(&task_id),
        Some(planned_state_version),
    );
    let pending_refs = store
        .pending_user_judgment_refs(&task_id, planned_state_version)
        .map_err(|error| {
            PlanError::Response(Box::new(store_error_response(
                &request.envelope,
                project_state,
                error,
            )))
        })?
        .into_iter()
        .filter(|record_ref| record_ref.record_id != request.user_judgment_id.as_str())
        .map(state_ref_from_stored)
        .collect::<Vec<_>>();
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
    let task_ref = state_ref(
        StateRecordKind::Task,
        task_id.as_str(),
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
            Some(record.basis_state_version.unwrap_or(planned_state_version)),
        )
    });
    let next_actions = next_actions_for_state(&task_ref, change_unit_ref.as_ref());
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task,
        current_change_unit: current_change_unit.as_ref(),
        pending_user_judgment_refs: pending_refs,
        blocker_refs,
        active_write_authorization: None,
        options: SummaryOptions::mutation(),
    })?;
    let result = harness_types::RecordUserJudgmentResult {
        base: placeholder_base(),
        user_judgment_ref: user_judgment_ref.clone(),
        user_judgment,
        updated_refs: vec![user_judgment_ref],
        state,
        next_actions: next_actions.clone(),
    };
    let sensitive_action_scope_json = request
        .answer
        .sensitive_action_scope
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let storage_mutations = vec![CoreStorageMutation::ResolveUserJudgment(
        UserJudgmentResolutionUpdate {
            judgment_id: request.user_judgment_id.as_str().to_owned(),
            status: storage_value(UserJudgmentStatus::Resolved)?,
            resolution_json: serde_json::to_string(&resolution)?,
            sensitive_action_scope_json,
            resolved_at: now,
        },
    )];
    let event_payload = object_from_value(json!({
        "task_id": task_id,
        "change_unit_id": record.change_unit_id,
        "judgment_id": request.user_judgment_id,
        "judgment_kind": request.judgment_kind,
        "selected_option_id": request.selected_option_id
    }))?;

    Ok(MethodPlan {
        task_id,
        change_unit_id: record.change_unit_id.map(ChangeUnitId::new),
        storage_mutations,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions,
    })
}

struct UserJudgmentRequestValidation<'a> {
    dry_run: bool,
    state_version: Option<u64>,
    question: &'a str,
    options: &'a [UserJudgmentOption],
    context: &'a UserJudgmentContext,
    affected_refs: &'a [StateRecordRef],
    project_id: &'a ProjectId,
    task_id: &'a TaskId,
    expires_at: Option<&'a str>,
    current_timestamp: String,
}

fn validate_user_judgment_request_fields(
    input: UserJudgmentRequestValidation<'_>,
) -> Result<(), PlanError> {
    if input.question.trim().is_empty() {
        return validation_plan_error(
            input.dry_run,
            input.state_version,
            "question",
            "question must not be empty",
        );
    }
    if input.options.is_empty() {
        return validation_plan_error(
            input.dry_run,
            input.state_version,
            "options",
            "options must include at least one judgment option",
        );
    }
    let mut option_ids = BTreeSet::new();
    for option in input.options {
        if option.option_id.as_str().trim().is_empty() {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "options.option_id",
                "option_id must not be empty",
            );
        }
        if !option_ids.insert(option.option_id.as_str()) {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "options.option_id",
                "option_id values must be unique within the judgment",
            );
        }
        if option.label.trim().is_empty() {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "options.label",
                "option label must not be empty",
            );
        }
    }
    if input.context.summary.trim().is_empty() {
        return validation_plan_error(
            input.dry_run,
            input.state_version,
            "context.summary",
            "context.summary must not be empty",
        );
    }
    for affected_ref in input.affected_refs {
        if affected_ref.project_id != *input.project_id {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "affected_refs.project_id",
                "affected_refs must belong to the request project",
            );
        }
        if affected_ref
            .task_id
            .as_ref()
            .is_some_and(|ref_task_id| ref_task_id != input.task_id)
        {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "affected_refs.task_id",
                "task-scoped affected_refs must belong to the request Task",
            );
        }
    }
    if let Some(expires_at) = input.expires_at {
        if expires_at.trim().is_empty() {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "expires_at",
                "expires_at must be null or a non-empty timestamp string",
            );
        }
        if expires_at <= input.current_timestamp.as_str() {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "expires_at",
                "expires_at must be in the future for a pending judgment request",
            );
        }
    }
    Ok(())
}

fn validate_answer_payload(
    dry_run: bool,
    state_version: Option<u64>,
    judgment_kind: JudgmentKind,
    answer: &RecordUserJudgmentPayload,
) -> Result<(), PlanError> {
    if populated_answer_branch_count(answer) != 1 {
        return validation_plan_error(
            dry_run,
            state_version,
            "answer",
            "answer must populate exactly one decision-specific payload branch",
        );
    }
    if !answer_branch_matches_kind(judgment_kind, answer) {
        return validation_plan_error(
            dry_run,
            state_version,
            "answer",
            "answer payload branch must match the pending judgment_kind",
        );
    }
    Ok(())
}

fn populated_answer_branch_count(answer: &RecordUserJudgmentPayload) -> usize {
    usize::from(answer.product_decision.is_some())
        + usize::from(answer.technical_decision.is_some())
        + usize::from(answer.scope_decision.is_some())
        + usize::from(answer.sensitive_action_scope.is_some())
        + usize::from(answer.final_acceptance.is_some())
        + usize::from(answer.residual_risk_acceptance.is_some())
        + usize::from(answer.cancellation.is_some())
}

fn answer_branch_matches_kind(
    judgment_kind: JudgmentKind,
    answer: &RecordUserJudgmentPayload,
) -> bool {
    match judgment_kind {
        JudgmentKind::ProductDecision => answer.product_decision.is_some(),
        JudgmentKind::TechnicalDecision => answer.technical_decision.is_some(),
        JudgmentKind::ScopeDecision => answer.scope_decision.is_some(),
        JudgmentKind::SensitiveApproval => answer.sensitive_action_scope.is_some(),
        JudgmentKind::FinalAcceptance => answer.final_acceptance.is_some(),
        JudgmentKind::ResidualRiskAcceptance => answer.residual_risk_acceptance.is_some(),
        JudgmentKind::Cancellation => answer.cancellation.is_some(),
    }
}

fn user_judgment_from_record(record: &UserJudgmentRecord) -> CoreResult<UserJudgment> {
    let request = decode_required_json_object(
        "user_judgments",
        record.judgment_id.clone(),
        "request_json",
        Some(&record.request_json),
    )?;
    Ok(UserJudgment {
        judgment_id: harness_types::UserJudgmentId::new(record.judgment_id.clone()),
        project_id: ProjectId::new(record.project_id.clone()),
        task_id: TaskId::new(record.task_id.clone()),
        change_unit_id: record.change_unit_id.clone().map(ChangeUnitId::new),
        judgment_kind: parse_storage_value("user_judgments.judgment_kind", &record.judgment_kind)?,
        status: parse_storage_value("user_judgments.status", &record.status)?,
        presentation: request_member(
            "user_judgments.request_json.presentation",
            &request,
            "presentation",
        )?,
        question: string_member(&request, "question").ok_or_else(|| {
            CorePipelineError::InvalidDispatch {
                detail: "user_judgments.request_json.question missing".to_owned(),
            }
        })?,
        options: parse_json_text("user_judgments.options_json", &record.options_json)?,
        context: parse_json_text("user_judgments.context_json", &record.context_json)?,
        affected_refs: parse_json_text(
            "user_judgments.affected_refs_json",
            &record.affected_refs_json,
        )?,
        required_for: request_member(
            "user_judgments.request_json.required_for",
            &request,
            "required_for",
        )?,
        resolution: decode_optional_json(
            "user_judgments",
            record.judgment_id.clone(),
            "resolution_json",
            record.resolution_json.as_deref(),
        )?,
        expires_at: request
            .get("expires_at")
            .and_then(Value::as_str)
            .map(str::to_owned),
        created_at: record.requested_at.clone(),
        resolved_at: record.resolved_at.clone(),
    })
}

fn decision_rejected_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    message: &'static str,
) -> PipelineResponse {
    rejected_pipeline_response(
        envelope.dry_run,
        state_version,
        vec![tool_error(
            ErrorCode::DecisionUnresolved,
            message,
            false,
            None,
        )],
    )
    .expect("rejected response serialization should succeed")
}
