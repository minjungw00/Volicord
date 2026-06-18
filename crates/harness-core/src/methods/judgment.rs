use super::*;

impl CoreService {
    /// Executes `harness.request_user_judgment` through the shared Core mutation pipeline.
    pub fn request_user_judgment(
        &self,
        request: harness_types::RequestUserJudgmentRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = request.envelope.task_id.as_ref() {
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
            .as_ref()
            .cloned()
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
    let requested_at = utc_timestamp(service.now());
    validate_user_judgment_request_fields(UserJudgmentRequestValidation {
        dry_run: request.envelope.dry_run,
        state_version: Some(project_state.state_version),
        judgment_kind: request.judgment_kind,
        question: &request.question,
        options: &request.options,
        context: &request.context,
        affected_refs: &request.affected_refs,
        project_id: &request.envelope.project_id,
        task_id: &request.task_id,
        expires_at: request.expires_at.as_ref(),
        current_timestamp: requested_at.clone(),
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
    let requested_change_unit_id = if let Some(change_unit_id) = request.change_unit_id.as_ref() {
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
    let mut judgment_context = request.context.clone();
    let basis = build_request_judgment_basis(
        store,
        project_state,
        &request,
        &task,
        current_change_unit.as_ref(),
        requested_change_unit_id.as_ref(),
        &mut judgment_context,
    )?;
    let branch_change_unit_id = basis.change_unit_id.as_ref().cloned();
    let stored_sensitive_action_scope_json = basis
        .sensitive_action_scope
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?
        .unwrap_or_else(|| "{}".to_owned());

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
        change_unit_id: branch_change_unit_id.clone(),
        judgment_kind: request.judgment_kind,
        status: UserJudgmentStatus::Pending,
        presentation: request.presentation,
        question: request.question.clone(),
        options: request.options.clone(),
        context: judgment_context.clone(),
        affected_refs: request.affected_refs.clone(),
        basis: Some(basis.clone()),
        required_for: request.required_for,
        resolution: None,
        expires_at: request.expires_at.clone().into_option(),
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
        effective_authorization_now: None,
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
            change_unit_id: branch_change_unit_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            judgment_kind: storage_value(request.judgment_kind)?,
            request_json: serde_json::to_string(&json!({
                "presentation": request.presentation,
                "question": request.question,
                "required_for": request.required_for,
                "expires_at": request.expires_at
            }))?,
            context_json: serde_json::to_string(&judgment_context)?,
            options_json: serde_json::to_string(&request.options)?,
            affected_refs_json: serde_json::to_string(&request.affected_refs)?,
            artifact_refs_json: serde_json::to_string(&judgment_context.artifact_refs)?,
            sensitive_action_scope_json: stored_sensitive_action_scope_json,
            basis_json: Some(serde_json::to_string(&basis)?),
            basis_status: harness_types::JudgmentBasisCompatibilityStatus::Current,
            requested_by_surface_id: verified_surface.surface_id.as_str().to_owned(),
            requested_by_surface_instance_id: verified_surface
                .surface_instance_id
                .as_str()
                .to_owned(),
            requested_at: requested_at.to_string(),
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

fn build_request_judgment_basis(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &harness_types::RequestUserJudgmentRequest,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    requested_change_unit_id: Option<&ChangeUnitId>,
    context: &mut UserJudgmentContext,
) -> Result<JudgmentBasis, PlanError> {
    let current_scope = StoredScope::from_task(task)?;
    let current_baseline = current_scope.baseline_ref.clone().map(BaselineRef::new);
    let current_change_unit_id = current_change_unit
        .as_ref()
        .map(|record| ChangeUnitId::new(record.change_unit_id.clone()));

    let mut basis = JudgmentBasis {
        task_id: request.task_id.clone(),
        change_unit_id: current_change_unit_id.clone().into(),
        scope_revision: task.scope_revision,
        close_basis_revision: RequiredNullable::null(),
        baseline_ref: current_baseline.clone().into(),
        result_refs: Vec::new(),
        residual_risk_ids: Vec::new(),
        sensitive_action_scope: RequiredNullable::null(),
        created_at_state_version: project_state.state_version,
        compatibility_status: JudgmentBasisCompatibilityStatus::Current,
    };

    match request.judgment_kind {
        JudgmentKind::FinalAcceptance => {
            let close_basis = current_close_basis_for_judgment(
                store,
                project_state,
                request,
                task,
                current_change_unit,
            )?;
            basis.change_unit_id = Some(close_basis.change_unit_id.clone()).into();
            basis.close_basis_revision = Some(close_basis.close_basis_revision).into();
            basis.baseline_ref = close_basis.baseline_ref.clone();
            basis.result_refs = close_basis.result_refs.clone();
            basis.residual_risk_ids = close_basis
                .residual_risks
                .iter()
                .map(|risk| risk.risk_id.clone())
                .collect();
        }
        JudgmentKind::ResidualRiskAcceptance => {
            let close_basis = current_close_basis_for_judgment(
                store,
                project_state,
                request,
                task,
                current_change_unit,
            )?;
            let required_risk_ids = current_acceptance_required_risk_ids(&close_basis);
            if required_risk_ids.is_empty() {
                let response = rejected_pipeline_response(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    vec![tool_error(
                        ErrorCode::AcceptanceRequired,
                        "residual-risk acceptance requires at least one current acceptance-required risk",
                        false,
                        None,
                    )],
                )
                .map_err(PlanError::Core)?;
                return Err(PlanError::Response(Box::new(response)));
            }
            basis.change_unit_id = Some(close_basis.change_unit_id.clone()).into();
            basis.close_basis_revision = Some(close_basis.close_basis_revision).into();
            basis.baseline_ref = close_basis.baseline_ref.clone();
            basis.result_refs = close_basis.result_refs.clone();
            basis.residual_risk_ids = close_basis
                .residual_risks
                .iter()
                .filter(|risk| risk.acceptance_required)
                .map(|risk| risk.risk_id.clone())
                .collect();
            context.visible_risks = close_basis
                .residual_risks
                .iter()
                .filter(|risk| risk.acceptance_required)
                .map(|risk| harness_types::AcceptedRiskInput {
                    risk_id: risk.risk_id.clone(),
                    summary: risk.summary.clone(),
                    consequence: risk.consequence.clone(),
                    related_refs: risk.source_refs.clone(),
                    accepted_for_close: true,
                })
                .collect();
        }
        JudgmentKind::SensitiveApproval => {
            let Some(current_change_unit_id) = current_change_unit_id.as_ref() else {
                return Err(PlanError::Response(Box::new(
                    no_active_change_unit_response(
                        &request.envelope,
                        Some(project_state.state_version),
                        "sensitive approval requires a current Change Unit",
                    ),
                )));
            };
            if requested_change_unit_id.is_some_and(|requested| requested != current_change_unit_id)
            {
                return validation_plan_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "change_unit_id",
                    "sensitive approval must address the current Change Unit",
                )
                .map(|()| basis);
            }
            let Some(scope) = request.sensitive_action_scope.as_ref() else {
                return validation_plan_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "sensitive_action_scope",
                    "sensitive approval requests must include the exact SensitiveActionScope",
                )
                .map(|()| basis);
            };
            let normalized_scope =
                normalize_sensitive_action_scope(&store.project_record().repo_root, scope)
                    .map_err(|error| match error {
                        ProductPathError::Invalid => PlanError::Response(Box::new(
                            validation_rejected(
                                request.envelope.dry_run,
                                Some(project_state.state_version),
                                "sensitive_action_scope.intended_paths",
                                "sensitive action paths must be valid Product Repository paths",
                            )
                            .expect("validation response should serialize"),
                        )),
                        ProductPathError::LocalAccess => PlanError::Response(Box::new(
                            rejected_pipeline_response(
                                request.envelope.dry_run,
                                Some(project_state.state_version),
                                vec![tool_error(
                                    ErrorCode::LocalAccessMismatch,
                                    "sensitive action paths resolve outside the Product Repository",
                                    false,
                                    None,
                                )],
                            )
                            .expect("rejected response should serialize"),
                        )),
                    })?;
            basis.change_unit_id = Some(current_change_unit_id.clone()).into();
            basis.sensitive_action_scope = Some(normalized_scope).into();
        }
        JudgmentKind::ProductDecision
        | JudgmentKind::TechnicalDecision
        | JudgmentKind::ScopeDecision
        | JudgmentKind::Cancellation => {
            if request.sensitive_action_scope.is_some() {
                return validation_plan_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "sensitive_action_scope",
                    "sensitive_action_scope is only valid for sensitive_approval judgments",
                )
                .map(|()| basis);
            }
            if let Some(requested_change_unit_id) = requested_change_unit_id {
                basis.change_unit_id = Some(requested_change_unit_id.clone()).into();
            }
        }
    }

    Ok(basis)
}

fn current_close_basis_for_judgment(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &harness_types::RequestUserJudgmentRequest,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
) -> Result<CurrentCloseBasis, PlanError> {
    if current_change_unit.is_none() {
        return Err(PlanError::Response(Box::new(
            no_active_change_unit_response(
                &request.envelope,
                Some(project_state.state_version),
                "close-basis judgments require a current Change Unit",
            ),
        )));
    }
    let task_revision = store
        .task_revision_record(&request.task_id)
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
    let Some(close_basis) = task_revision.current_close_basis else {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "current close basis is required before requesting this judgment",
        ))));
    };
    let current_baseline = StoredScope::from_task(task)?.baseline_ref;
    if !close_basis_is_current(
        &close_basis,
        &request.task_id,
        current_change_unit.map(|record| record.change_unit_id.as_str()),
        task.scope_revision,
        task.close_basis_revision,
        current_baseline.as_deref(),
    ) {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "current close basis is stale for this judgment request",
        ))));
    }
    if request
        .change_unit_id
        .as_ref()
        .is_some_and(|change_unit_id| change_unit_id != &close_basis.change_unit_id)
    {
        return validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "change_unit_id",
            "close-basis judgments must address the current close-basis Change Unit",
        )
        .map(|()| close_basis);
    }
    Ok(close_basis)
}

fn validate_pending_judgment_basis_for_answer(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordUserJudgmentRequest,
    record: &UserJudgmentRecord,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    now: &UtcTimestamp,
) -> Result<RecordUserJudgmentPayload, PlanError> {
    let authority = user_judgment_authority_from_record(record)?;
    if !judgment_has_current_basis(&authority) {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "pending user-owned judgment basis is not current",
        ))));
    }
    if request.judgment_kind != JudgmentKind::ResidualRiskAcceptance
        && !request.accepted_risks.is_empty()
    {
        return validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "accepted_risks",
            "accepted_risks may only be supplied for residual_risk_acceptance judgments",
        )
        .map(|()| request.answer.clone());
    }

    let Some(basis) = authority.basis.as_ref() else {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "pending user-owned judgment basis is unavailable",
        ))));
    };

    match request.judgment_kind {
        JudgmentKind::FinalAcceptance => {
            let close_basis = current_close_basis_for_answer(
                store,
                project_state,
                request,
                task,
                current_change_unit,
            )?;
            let requirement = final_acceptance_requirement(&close_basis);
            if !final_acceptance_basis_matches_current(basis, &requirement) {
                return Err(PlanError::Response(Box::new(decision_rejected_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "pending final-acceptance judgment is stale against the current close basis",
                ))));
            }
            Ok(request.answer.clone())
        }
        JudgmentKind::ResidualRiskAcceptance => {
            let close_basis = current_close_basis_for_answer(
                store,
                project_state,
                request,
                task,
                current_change_unit,
            )?;
            if !residual_risk_basis_matches_current(basis, &close_basis) {
                return Err(PlanError::Response(Box::new(decision_rejected_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "pending residual-risk judgment is stale against the current close basis",
                ))));
            }
            if !accepted_risk_ids_within_basis(&request.answer, &request.accepted_risks, basis) {
                return validation_plan_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "accepted_risks",
                    "accepted risk IDs must be inside the pending judgment basis",
                )
                .map(|()| request.answer.clone());
            }
            Ok(request.answer.clone())
        }
        JudgmentKind::SensitiveApproval => {
            let Some(current_change_unit) = current_change_unit else {
                return Err(PlanError::Response(Box::new(
                    no_active_change_unit_response(
                        &request.envelope,
                        Some(project_state.state_version),
                        "sensitive approval requires a current Change Unit",
                    ),
                )));
            };
            let Some(stored_scope) = basis.sensitive_action_scope.as_ref() else {
                return Err(PlanError::Response(Box::new(decision_rejected_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "pending sensitive approval has no bound SensitiveActionScope",
                ))));
            };
            let Some(answer_scope) = request.answer.sensitive_action_scope.as_ref() else {
                return validation_plan_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "answer.sensitive_action_scope",
                    "sensitive approval answers must include SensitiveActionScope",
                )
                .map(|()| request.answer.clone());
            };
            let normalized_answer_scope =
                normalize_sensitive_action_scope(&store.project_record().repo_root, answer_scope)
                    .map_err(|error| match error {
                    ProductPathError::Invalid => PlanError::Response(Box::new(
                        validation_rejected(
                            request.envelope.dry_run,
                            Some(project_state.state_version),
                            "answer.sensitive_action_scope.intended_paths",
                            "sensitive action paths must be valid Product Repository paths",
                        )
                        .expect("validation response should serialize"),
                    )),
                    ProductPathError::LocalAccess => PlanError::Response(Box::new(
                        rejected_pipeline_response(
                            request.envelope.dry_run,
                            Some(project_state.state_version),
                            vec![tool_error(
                                ErrorCode::LocalAccessMismatch,
                                "sensitive action paths resolve outside the Product Repository",
                                false,
                                None,
                            )],
                        )
                        .expect("rejected response should serialize"),
                    )),
                })?;
            let current_change_unit_id =
                ChangeUnitId::new(current_change_unit.change_unit_id.clone());
            let requirement = SensitiveApprovalRequirement {
                task_id: &authority.task_id,
                change_unit_id: &current_change_unit_id,
                scope_revision: task.scope_revision,
                operation: &stored_scope.action_kind,
                normalized_paths: &stored_scope.intended_paths,
                sensitive_categories: &stored_scope.sensitive_categories,
                baseline_ref: basis.baseline_ref.as_ref(),
                now,
                repo_root: &store.project_record().repo_root,
            };
            if basis.task_id != authority.task_id
                || basis.change_unit_id.as_ref() != Some(&current_change_unit_id)
                || basis.scope_revision != task.scope_revision
                || !sensitive_action_scope_matches_requirement(stored_scope, &requirement)
                || normalized_answer_scope != *stored_scope
            {
                return validation_plan_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "answer.sensitive_action_scope",
                    "sensitive approval answer must match the stored judgment basis",
                )
                .map(|()| request.answer.clone());
            }
            let mut answer = request.answer.clone();
            answer.sensitive_action_scope = Some(stored_scope.clone()).into();
            Ok(answer)
        }
        JudgmentKind::ProductDecision
        | JudgmentKind::TechnicalDecision
        | JudgmentKind::ScopeDecision
        | JudgmentKind::Cancellation => {
            let current_change_unit_id =
                current_change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
            let basis_change_unit_is_current = basis.change_unit_id.as_ref().is_none()
                || basis.change_unit_id.as_ref() == current_change_unit_id.as_ref();
            if basis.task_id != authority.task_id
                || basis.scope_revision != task.scope_revision
                || !basis_change_unit_is_current
            {
                return Err(PlanError::Response(Box::new(decision_rejected_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "pending user-owned judgment is stale against current scope",
                ))));
            }
            Ok(request.answer.clone())
        }
    }
}

fn current_close_basis_for_answer(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RecordUserJudgmentRequest,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
) -> Result<CurrentCloseBasis, PlanError> {
    let task_id = TaskId::new(task.task_id.clone());
    let task_revision = store
        .task_revision_record(&task_id)
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
    let Some(close_basis) = task_revision.current_close_basis else {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "current close basis is required for this judgment answer",
        ))));
    };
    let current_baseline = StoredScope::from_task(task)?.baseline_ref;
    if close_basis_is_current(
        &close_basis,
        &task_id,
        current_change_unit.map(|record| record.change_unit_id.as_str()),
        task.scope_revision,
        task.close_basis_revision,
        current_baseline.as_deref(),
    ) {
        Ok(close_basis)
    } else {
        Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "current close basis is stale for this judgment answer",
        ))))
    }
}

fn plan_record_user_judgment(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: RecordUserJudgmentRequest,
) -> Result<MethodPlan, PlanError> {
    let planned_state_version = project_state.state_version + 1;
    let now = utc_timestamp(service.now());
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
    if let Some(envelope_task_id) = request.envelope.task_id.as_ref() {
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
    if user_judgment
        .expires_at
        .as_ref()
        .is_some_and(|expires_at| &now >= expires_at)
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
    let Some(selected_option) = user_judgment
        .options
        .iter()
        .find(|option| option.option_id == request.selected_option_id)
    else {
        let response = validation_rejected(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "selected_option_id",
            "selected_option_id is not one of the pending judgment options",
        )
        .map_err(PlanError::Core)?;
        return Err(PlanError::Response(Box::new(response)));
    };
    let Some(resolution_outcome) = selected_option.resolution_outcome else {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "pending user-owned judgment option lacks a machine-readable resolution outcome",
        ))));
    };
    if is_authority_bearing_judgment(request.judgment_kind)
        && request.envelope.actor_kind != ActorKind::User
    {
        return validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "envelope.actor_kind",
            "authority-bearing judgments must be resolved by actor_kind=user",
        )
        .map(|()| unreachable!());
    }
    validate_answer_payload(
        request.envelope.dry_run,
        Some(project_state.state_version),
        request.judgment_kind,
        &request.answer,
    )?;
    validate_answer_outcome_agrees_with_option(
        request.envelope.dry_run,
        Some(project_state.state_version),
        &request.answer,
        resolution_outcome,
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
    let answer = validate_pending_judgment_basis_for_answer(
        store,
        project_state,
        &request,
        &record,
        &task,
        current_change_unit.as_ref(),
        &now,
    )?;
    let resolution = UserJudgmentResolution {
        selected_option_id: request.selected_option_id.clone(),
        resolution_outcome: Some(resolution_outcome),
        answer,
        note: request.note.clone().into_option(),
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
        effective_authorization_now: None,
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
    let sensitive_action_scope_json = resolution
        .answer
        .sensitive_action_scope
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let storage_mutations = vec![CoreStorageMutation::ResolveUserJudgment(
        UserJudgmentResolutionUpdate {
            judgment_id: request.user_judgment_id.as_str().to_owned(),
            status: storage_value(UserJudgmentStatus::Resolved)?,
            resolution_outcome,
            resolution_json: serde_json::to_string(&resolution)?,
            sensitive_action_scope_json,
            resolved_at: now.to_string(),
        },
    )];
    let event_payload = object_from_value(json!({
        "task_id": task_id,
        "change_unit_id": record.change_unit_id,
        "judgment_id": request.user_judgment_id,
        "judgment_kind": request.judgment_kind,
        "selected_option_id": request.selected_option_id,
        "resolution_outcome": resolution_outcome
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
    judgment_kind: JudgmentKind,
    question: &'a str,
    options: &'a [UserJudgmentOption],
    context: &'a UserJudgmentContext,
    affected_refs: &'a [StateRecordRef],
    project_id: &'a ProjectId,
    task_id: &'a TaskId,
    expires_at: Option<&'a UtcTimestamp>,
    current_timestamp: UtcTimestamp,
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
    let mut accepted_options = 0usize;
    let mut rejected_options = 0usize;
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
        let Some(outcome) = option.resolution_outcome else {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "options.resolution_outcome",
                "each judgment option must include a machine-readable resolution_outcome",
            );
        };
        match outcome {
            JudgmentResolutionOutcome::Accepted => accepted_options += 1,
            JudgmentResolutionOutcome::Rejected => rejected_options += 1,
            JudgmentResolutionOutcome::Deferred | JudgmentResolutionOutcome::Blocked => {
                if is_authority_bearing_judgment(input.judgment_kind) {
                    return validation_plan_error(
                        input.dry_run,
                        input.state_version,
                        "options.resolution_outcome",
                        "authority-bearing judgment options may only use accepted or rejected outcomes",
                    );
                }
            }
        }
    }
    if is_authority_bearing_judgment(input.judgment_kind) {
        if accepted_options == 0 || rejected_options == 0 {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "options.resolution_outcome",
                "authority-bearing judgment options must include accepted and rejected paths",
            );
        }
        if accepted_options > 1 || rejected_options > 1 {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "options.resolution_outcome",
                "authority-bearing judgment options must not duplicate accepted or rejected machine outcomes",
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
        if expires_at <= &input.current_timestamp {
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

fn validate_answer_outcome_agrees_with_option(
    dry_run: bool,
    state_version: Option<u64>,
    answer: &RecordUserJudgmentPayload,
    selected_outcome: JudgmentResolutionOutcome,
) -> Result<(), PlanError> {
    let answer_value = serde_json::to_value(answer)?;
    let mut claims = Vec::new();
    collect_answer_outcome_claims(&answer_value, &mut claims);
    if claims
        .iter()
        .any(|claimed_outcome| *claimed_outcome != selected_outcome)
    {
        return validation_plan_error(
            dry_run,
            state_version,
            "answer",
            "answer outcome fields must agree with the selected option resolution_outcome",
        );
    }
    Ok(())
}

fn collect_answer_outcome_claims(value: &Value, claims: &mut Vec<JudgmentResolutionOutcome>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if let Some(outcome) = answer_claimed_outcome(key, value) {
                    claims.push(outcome);
                }
                collect_answer_outcome_claims(value, claims);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_answer_outcome_claims(value, claims);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn answer_claimed_outcome(key: &str, value: &Value) -> Option<JudgmentResolutionOutcome> {
    match key {
        "resolution_outcome" | "outcome" | "decision" | "acceptance" => {
            outcome_from_json_value(value)
        }
        "accepted" | "approved" => outcome_from_boolean_or_string(value),
        _ => None,
    }
}

fn outcome_from_boolean_or_string(value: &Value) -> Option<JudgmentResolutionOutcome> {
    match value {
        Value::Bool(true) => Some(JudgmentResolutionOutcome::Accepted),
        Value::Bool(false) => Some(JudgmentResolutionOutcome::Rejected),
        Value::String(raw) => outcome_from_str(raw),
        _ => None,
    }
}

fn outcome_from_json_value(value: &Value) -> Option<JudgmentResolutionOutcome> {
    match value {
        Value::String(raw) => outcome_from_str(raw),
        Value::Bool(_) => outcome_from_boolean_or_string(value),
        _ => None,
    }
}

fn outcome_from_str(raw: &str) -> Option<JudgmentResolutionOutcome> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "accepted" | "accept" | "approve" | "approved" | "yes" | "true" => {
            Some(JudgmentResolutionOutcome::Accepted)
        }
        "rejected" | "reject" | "decline" | "declined" | "deny" | "denied" | "no" | "false" => {
            Some(JudgmentResolutionOutcome::Rejected)
        }
        "deferred" | "defer" => Some(JudgmentResolutionOutcome::Deferred),
        "blocked" | "block" => Some(JudgmentResolutionOutcome::Blocked),
        _ => None,
    }
}

fn is_authority_bearing_judgment(judgment_kind: JudgmentKind) -> bool {
    matches!(
        judgment_kind,
        JudgmentKind::FinalAcceptance
            | JudgmentKind::ResidualRiskAcceptance
            | JudgmentKind::SensitiveApproval
            | JudgmentKind::Cancellation
    )
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
    let request: PersistedUserJudgmentRequest = decode_required_json(
        "user_judgments",
        record.judgment_id.clone(),
        "request_json",
        Some(&record.request_json),
    )?;
    let _artifact_refs: Vec<ArtifactRef> = decode_required_json(
        "user_judgments",
        record.judgment_id.clone(),
        "artifact_refs_json",
        Some(&record.artifact_refs_json),
    )?;
    let authority = user_judgment_authority_from_record(record)?;
    let created_at = parse_owner_storage_value(
        "user_judgments",
        record.judgment_id.clone(),
        "requested_at",
        &record.requested_at,
    )?;
    let resolved_at = record
        .resolved_at
        .as_ref()
        .map(|resolved_at| {
            parse_owner_storage_value(
                "user_judgments",
                record.judgment_id.clone(),
                "resolved_at",
                resolved_at,
            )
        })
        .transpose()?;
    Ok(UserJudgment {
        judgment_id: harness_types::UserJudgmentId::new(record.judgment_id.clone()),
        project_id: ProjectId::new(record.project_id.clone()),
        task_id: TaskId::new(record.task_id.clone()),
        change_unit_id: record.change_unit_id.clone().map(ChangeUnitId::new),
        judgment_kind: authority.judgment_kind,
        status: authority.status,
        presentation: request.presentation,
        question: request.question,
        options: decode_required_json(
            "user_judgments",
            record.judgment_id.clone(),
            "options_json",
            Some(&record.options_json),
        )?,
        context: decode_required_json(
            "user_judgments",
            record.judgment_id.clone(),
            "context_json",
            Some(&record.context_json),
        )?,
        affected_refs: decode_required_json(
            "user_judgments",
            record.judgment_id.clone(),
            "affected_refs_json",
            Some(&record.affected_refs_json),
        )?,
        basis: authority.basis,
        required_for: request.required_for,
        resolution: authority.resolution,
        expires_at: request.expires_at.into_option(),
        created_at,
        resolved_at,
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
