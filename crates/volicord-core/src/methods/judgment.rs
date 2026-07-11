use super::*;

impl CoreService {
    /// Executes `volicord.request_user_judgment` through the shared Core mutation pipeline.
    pub fn request_user_judgment(
        &self,
        request: volicord_types::RequestUserJudgmentRequest,
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
                request.operation_category(),
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
            &prepared.context.verified_invocation,
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

    /// Executes `volicord.record_user_judgment` through the shared Core mutation pipeline.
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
                request.operation_category(),
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
            &prepared.context.verified_invocation,
            &prepared.context.verified_actor,
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

    /// Records a local web consent answer and consumes its token in one Core commit.
    pub fn record_local_web_consent_judgment(
        &self,
        consent_request: LocalWebConsentJudgmentRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request = consent_request.request;
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
                request.operation_category(),
                task_requirement,
                request.envelope.dry_run,
            ),
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let token_now = match prepared.store.current_timestamp() {
            Ok(now) => now,
            Err(error) => {
                return Ok(store_error_response(
                    &request.envelope,
                    &prepared.context.project_state,
                    error,
                ))
            }
        };
        let token_record = match validate_local_web_consent_token(
            prepared.store.runtime_home(),
            LocalWebConsentTokenCheck {
                token: consent_request.token,
                expected_project_id: request.envelope.project_id.as_str().to_owned(),
                expected_connection_internal_id: consent_request.expected_connection_internal_id,
                now: token_now.clone(),
            },
        ) {
            Ok(LocalWebConsentTokenValidation::Valid(record)) => record,
            Ok(LocalWebConsentTokenValidation::Rejected(rejection)) => {
                return Ok(local_web_consent_token_rejected_response(
                    &request.envelope,
                    Some(prepared.context.project_state.state_version),
                    rejection,
                ))
            }
            Err(error) => {
                return Ok(store_error_response(
                    &request.envelope,
                    &prepared.context.project_state,
                    error,
                ))
            }
        };
        if token_record.judgment_id != request.user_judgment_id.as_str() {
            return Ok(decision_rejected_response(
                &request.envelope,
                Some(prepared.context.project_state.state_version),
                "local web consent token is not bound to this pending judgment",
            ));
        }
        if token_record.capture_basis != VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB {
            return Ok(decision_rejected_response(
                &request.envelope,
                Some(prepared.context.project_state.state_version),
                "local web consent token capture basis is not supported for this path",
            ));
        }

        let mut plan = match plan_record_user_judgment(
            self,
            &prepared.store,
            &prepared.context.project_state,
            request.clone(),
            &prepared.context.verified_invocation,
            &prepared.context.verified_actor,
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

        plan.storage_mutations.insert(
            0,
            CoreStorageMutation::ConsumeLocalWebConsentToken(LocalWebConsentTokenConsumption {
                token_hash: token_record.token_hash,
                connection_internal_id: token_record.connection_internal_id,
                judgment_id: token_record.judgment_id,
                consumed_at: token_now,
                completion_metadata_json: consent_request.completion_metadata_json,
            }),
        );
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
    request: volicord_types::RequestUserJudgmentRequest,
    verified_invocation: &VerifiedInvocationContext,
) -> Result<MethodPlan, PlanError> {
    let requested_at = utc_timestamp(service.now());
    let caller_options = request.options.as_ref().map(Vec::as_slice).unwrap_or(&[]);
    validate_user_judgment_request_fields(UserJudgmentRequestValidation {
        dry_run: request.envelope.dry_run,
        state_version: Some(project_state.state_version),
        judgment_kind: request.judgment_kind,
        question: &request.question,
        options: caller_options,
        context: &request.context,
        affected_refs: &request.affected_refs,
        required_for: &request.required_for,
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
    let options = current_options_for_request(
        request.judgment_kind,
        caller_options,
        request.envelope.locale.as_ref().map(String::as_str),
    );

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
        options: options.clone(),
        context: judgment_context.clone(),
        affected_refs: request.affected_refs.clone(),
        basis: basis.clone(),
        required_for: request.required_for.clone(),
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
        &request.task_id,
        planned_state_version,
        *requested_at.as_datetime(),
        Some(guarantee_display.clone()),
    )?;
    let evidence_summary = projected_evidence_summary(
        store,
        &request.envelope.project_id,
        planned_state_version,
        &task,
    )?;
    let mut pending_authorities = pending_judgment_authorities_for_plan(
        store,
        project_state,
        &request.envelope,
        &request.task_id,
    )?;
    pending_authorities.push(user_judgment_authority_from_state(&user_judgment, None));
    let lifecycle_phase = projected_judgment_lifecycle_phase(
        project_state,
        &task,
        current_change_unit.as_ref(),
        &pending_authorities,
    );
    let mut projected_task = task.clone();
    if let Some(lifecycle_phase) = lifecycle_phase {
        projected_task.lifecycle_phase = lifecycle_phase.to_owned();
    }
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
        verified_invocation,
        &request.envelope,
        &request.task_id,
        close_context_with_pending_authorities(
            close_context_from_projection(
                projected_task.clone(),
                current_change_unit.clone(),
                projected_close_basis(store, &request.task_id)?,
                pending_refs.clone(),
                blocker_refs.clone(),
                evidence_summary.clone(),
            ),
            pending_authorities,
        ),
        *requested_at.as_datetime(),
    )?;
    let inbox_item = judgment_inbox_item_from_judgment(
        &user_judgment,
        planned_state_version,
        UserChannelContext {
            guard_health: close_plan.guard_health.as_ref(),
            host_elicitation_available: verified_invocation.host_elicitation_available,
            local_web_consent_available: verified_invocation.local_web_consent_available,
        },
    )?;
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &projected_task,
        current_change_unit: current_change_unit.as_ref(),
        pending_user_judgment_refs: pending_refs,
        blocker_refs: blocker_refs.clone(),
        write_ticket_summary,
        evidence_summary,
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers,
        guard_health: close_plan.guard_health,
        guarantee_display: Some(guarantee_display),
    })?;
    let result = volicord_types::RequestUserJudgmentResult {
        base: placeholder_base(),
        user_judgment_ref: user_judgment_ref.clone(),
        user_judgment,
        inbox_item,
        blocker_refs,
        state,
    };
    let mut storage_mutations = vec![CoreStorageMutation::InsertUserJudgment(
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
            options_json: serde_json::to_string(&PersistedUserJudgmentOptions::current(
                options.clone(),
            ))?,
            affected_refs_json: serde_json::to_string(&request.affected_refs)?,
            artifact_refs_json: serde_json::to_string(&judgment_context.artifact_refs)?,
            sensitive_action_scope_json: stored_sensitive_action_scope_json,
            basis_json: serde_json::to_string(&basis)?,
            basis_status: volicord_types::JudgmentBasisCompatibilityStatus::Current,
            requested_by_actor_source: verified_invocation.actor_source.to_canonical_string(),
            requested_at: requested_at.to_string(),
            metadata_json: serde_json::to_string(&json!({}))?,
        },
    )];
    if let Some(lifecycle_phase) = lifecycle_phase {
        storage_mutations.push(task_lifecycle_mutation(&request.task_id, lifecycle_phase));
    }
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
    request: &volicord_types::RequestUserJudgmentRequest,
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
                .map(|risk| volicord_types::AcceptedRiskInput {
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
                                    ErrorCode::InvocationContextMismatch,
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
        | JudgmentKind::ScopeDecision => {
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
        JudgmentKind::Cancellation => {
            if request.sensitive_action_scope.is_some() {
                return validation_plan_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "sensitive_action_scope",
                    "sensitive_action_scope is only valid for sensitive_approval judgments",
                )
                .map(|()| basis);
            }
            let Some(current_change_unit_id) = current_change_unit_id.as_ref() else {
                return Err(PlanError::Response(Box::new(
                    no_active_change_unit_response(
                        &request.envelope,
                        Some(project_state.state_version),
                        "cancellation judgment requires a current Change Unit",
                    ),
                )));
            };
            if requested_change_unit_id.is_some_and(|requested| requested != current_change_unit_id)
            {
                return validation_plan_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "change_unit_id",
                    "cancellation judgment must address the current Change Unit",
                )
                .map(|()| basis);
            }
            basis.change_unit_id = Some(current_change_unit_id.clone()).into();
        }
    }

    Ok(basis)
}

fn current_close_basis_for_judgment(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &volicord_types::RequestUserJudgmentRequest,
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
                                ErrorCode::InvocationContextMismatch,
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
                required_for: JudgmentRequiredFor::PrepareWrite,
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
    verified_invocation: &VerifiedInvocationContext,
    verified_actor: &VerifiedActorContext,
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

    let authority = user_judgment_authority_from_record(&record)?;
    if authority
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
    if request.judgment_kind != authority.judgment_kind {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "judgment_kind is incompatible with the pending user-owned judgment",
        ))));
    }
    let persisted_options = decode_required_json::<PersistedUserJudgmentOptions>(
        "user_judgments",
        record.judgment_id.clone(),
        "options_json",
        Some(&record.options_json),
    )?;
    let Some(persisted_selected_option) = persisted_options
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
    let selected_option = match persisted_selected_option.clone().into_current() {
        Ok(option) => option,
        Err(
            volicord_types::PersistedUserJudgmentCompatibilityError::MismatchedResolutionOutcome,
        ) => {
            return Err(PlanError::Core(CorePipelineError::Store(
                StoreError::corrupt_owner_state_value(
                    "user_judgments",
                    record.judgment_id.clone(),
                    "options_json",
                ),
            )));
        }
    };
    let machine_action = selected_option.machine_action;
    let resolution_outcome = machine_action.resolution_outcome();
    if selected_option.resolution_outcome != resolution_outcome {
        return Err(PlanError::Core(CorePipelineError::Store(
            StoreError::corrupt_owner_state_value(
                "user_judgments",
                record.judgment_id.clone(),
                "options_json",
            ),
        )));
    }
    if verified_actor.actor_source != ActorSource::LocalUser {
        let response = rejected_pipeline_response(
            request.envelope.dry_run,
            Some(project_state.state_version),
            vec![crate::policy::access::invocation_context_mismatch_error(
                "invocation.actor_source",
            )],
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
    validate_answer_outcome_agrees_with_option(
        request.envelope.dry_run,
        Some(project_state.state_version),
        &request.answer,
        resolution_outcome,
    )?;
    if let Err(error) = validate_judgment_rationale(
        &request.rationale,
        request.judgment_kind,
        resolution_outcome,
    ) {
        return validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            error.field(),
            error.message(),
        )
        .map(|()| unreachable!());
    }

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
    let mut user_judgment = user_judgment_from_record(&record)?;
    let resolution = UserJudgmentResolution {
        selected_option_id: request.selected_option_id.clone(),
        machine_action,
        resolution_outcome,
        answer,
        rationale: request.rationale.clone(),
        note: request.note.clone(),
        accepted_risks: request.accepted_risks.clone(),
        resolved_by_actor_source: verified_actor.actor_source.clone(),
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
    let evidence_summary = projected_evidence_summary(
        store,
        &request.envelope.project_id,
        planned_state_version,
        &task,
    )?;
    let pending_authorities =
        pending_judgment_authorities_for_plan(store, project_state, &request.envelope, &task_id)?
            .into_iter()
            .filter(|authority| authority.judgment_id != request.user_judgment_id.as_str())
            .collect::<Vec<_>>();
    let lifecycle_phase = projected_judgment_lifecycle_phase(
        project_state,
        &task,
        current_change_unit.as_ref(),
        &pending_authorities,
    );
    let mut projected_task = task.clone();
    if let Some(lifecycle_phase) = lifecycle_phase {
        projected_task.lifecycle_phase = lifecycle_phase.to_owned();
    }
    let mut resolved_authorities = resolved_judgment_authorities_for_all_kinds(
        store,
        project_state,
        &request.envelope,
        &task_id,
    )?;
    resolved_authorities.push(user_judgment_authority_from_state(
        &user_judgment,
        Some(verified_actor),
    ));
    let projected_project_state = project_state_projection(
        project_state,
        planned_state_version,
        project_state
            .active_task_id
            .clone()
            .or_else(|| Some(task_id.as_str().to_owned())),
    );
    let close_plan = projected_close_check(
        store,
        &projected_project_state,
        verified_invocation,
        &request.envelope,
        &task_id,
        close_context_with_resolved_authorities(
            close_context_with_pending_authorities(
                close_context_from_projection(
                    projected_task.clone(),
                    current_change_unit.clone(),
                    projected_close_basis(store, &task_id)?,
                    pending_refs.clone(),
                    blocker_refs.clone(),
                    evidence_summary.clone(),
                ),
                pending_authorities,
            ),
            resolved_authorities,
        ),
        *now.as_datetime(),
    )?;
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &projected_task,
        current_change_unit: current_change_unit.as_ref(),
        pending_user_judgment_refs: pending_refs,
        blocker_refs,
        write_ticket_summary,
        evidence_summary,
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers,
        guard_health: close_plan.guard_health,
        guarantee_display: Some(guarantee_display),
    })?;
    let continuity_plans = plan_judgment_continuity_records(JudgmentContinuityContext {
        service,
        store,
        project_state,
        request: &request,
        record: &record,
        task: &task,
        current_change_unit: current_change_unit.as_ref(),
        resolution: &resolution,
        user_judgment: &user_judgment,
        user_judgment_ref: &user_judgment_ref,
        planned_state_version,
        now: &now,
    })?;
    let continuity_record_refs = continuity_plans
        .iter()
        .map(|plan| plan.record_ref.clone())
        .collect::<Vec<_>>();
    let continuity_record_ids = continuity_record_refs
        .iter()
        .map(|record_ref| record_ref.record_id.as_str().to_owned())
        .collect::<Vec<_>>();
    let mut updated_refs = vec![user_judgment_ref.clone()];
    updated_refs.extend(continuity_record_refs);
    let result = volicord_types::RecordUserJudgmentResult {
        base: placeholder_base(),
        user_judgment_ref: user_judgment_ref.clone(),
        user_judgment,
        updated_refs,
        state,
        next_actions: next_actions.clone(),
    };
    let sensitive_action_scope_json = resolution
        .answer
        .sensitive_action_scope
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let mut storage_mutations = vec![CoreStorageMutation::ResolveUserJudgment(
        UserJudgmentResolutionUpdate {
            judgment_id: request.user_judgment_id.as_str().to_owned(),
            status: storage_value(UserJudgmentStatus::Resolved)?,
            resolution_outcome,
            resolution_machine_action: machine_action,
            resolution_json: serde_json::to_string(&PersistedUserJudgmentResolution::current(
                resolution.clone(),
            ))?,
            resolution_rationale_json: serde_json::to_string(&resolution.rationale)?,
            sensitive_action_scope_json,
            resolved_by_actor_source: verified_actor.actor_source.to_canonical_string(),
            resolved_verification_basis: verified_actor.verification_basis.clone(),
            resolved_assurance_level: verified_actor.assurance_level.clone(),
            resolved_at: now.to_string(),
        },
    )];
    if let Some(lifecycle_phase) = lifecycle_phase {
        storage_mutations.push(task_lifecycle_mutation(&task_id, lifecycle_phase));
    }
    storage_mutations.extend(continuity_plans.into_iter().map(|plan| plan.mutation));
    let event_payload = object_from_value(json!({
        "task_id": task_id,
        "change_unit_id": record.change_unit_id,
        "judgment_id": request.user_judgment_id,
        "judgment_kind": request.judgment_kind,
        "selected_option_id": request.selected_option_id,
        "machine_action": machine_action,
        "resolution_outcome": resolution_outcome,
        "continuity_record_ids": continuity_record_ids
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

#[derive(Clone, Copy)]
struct JudgmentContinuityContext<'a> {
    service: &'a CoreService,
    store: &'a CoreProjectStore,
    project_state: &'a ProjectStateHeader,
    request: &'a RecordUserJudgmentRequest,
    record: &'a UserJudgmentRecord,
    task: &'a TaskRecord,
    current_change_unit: Option<&'a ChangeUnitRecord>,
    resolution: &'a UserJudgmentResolution,
    user_judgment: &'a UserJudgment,
    user_judgment_ref: &'a StateRecordRef,
    planned_state_version: u64,
    now: &'a UtcTimestamp,
}

fn plan_judgment_continuity_records(
    context: JudgmentContinuityContext<'_>,
) -> Result<Vec<PlannedProjectContinuityRecord>, PlanError> {
    match judgment_continuity_kind(
        context.request.judgment_kind,
        context.resolution.resolution_outcome,
    ) {
        Some(ProjectContinuityKind::Decision) => {
            plan_decision_continuity_record(context).map(|record| vec![record])
        }
        Some(ProjectContinuityKind::AcceptedRisk) => plan_accepted_risk_continuity_records(context),
        _ => Ok(Vec::new()),
    }
}

fn plan_decision_continuity_record(
    context: JudgmentContinuityContext<'_>,
) -> Result<PlannedProjectContinuityRecord, PlanError> {
    let request = context.request;
    let user_judgment = context.user_judgment;
    let summary = continuity_decision_summary(request, user_judgment, context.resolution);
    let title = format!(
        "{}: {}",
        decision_title_prefix(request.judgment_kind),
        short_continuity_title(&summary)
    );
    let source_change_unit_id = context
        .record
        .change_unit_id
        .as_ref()
        .map(|id| ChangeUnitId::new(id.clone()));
    let source_refs = refs_with_context(
        vec![context.user_judgment_ref.clone()],
        refs_with_context(
            user_judgment.affected_refs.clone(),
            refs_with_context(
                user_judgment.context.related_refs.clone(),
                request.rationale.related_refs.clone(),
            ),
        ),
    );
    let applies_to_refs = refs_with_context(
        user_judgment.affected_refs.clone(),
        user_judgment.context.related_refs.clone(),
    );
    let artifact_refs = user_judgment
        .context
        .artifact_refs
        .iter()
        .cloned()
        .chain(request.rationale.artifact_refs.iter().cloned())
        .collect();
    let draft = ProjectContinuityDraft {
        kind: ProjectContinuityKind::Decision,
        title,
        summary,
        rationale: request
            .rationale
            .selected_reason
            .as_ref()
            .cloned()
            .or_else(|| Some(request.rationale.summary.clone())),
        applies_to_paths: bounded_paths_from_change_unit(context.current_change_unit)?,
        applies_to_refs,
        source_refs,
        artifact_refs,
        supersedes_refs: Vec::new(),
        review_triggers: request.rationale.review_triggers.clone(),
        metadata: json!({
            "source": "record_user_judgment",
            "judgment_kind": request.judgment_kind,
            "resolution_outcome": context.resolution.resolution_outcome
        }),
    };
    plan_project_continuity_record(
        ProjectContinuityPlanContext {
            service: context.service,
            store: context.store,
            project_id: &request.envelope.project_id,
            source_task_id: &user_judgment.task_id,
            source_change_unit_id: source_change_unit_id.as_ref(),
            planned_state_version: context.planned_state_version,
            now: context.now,
        },
        draft,
    )
    .map_err(PlanError::Core)
}

fn plan_accepted_risk_continuity_records(
    context: JudgmentContinuityContext<'_>,
) -> Result<Vec<PlannedProjectContinuityRecord>, PlanError> {
    let request = context.request;
    let resolution = context.resolution;
    let user_judgment = context.user_judgment;
    let accepted_risk_ids =
        accepted_risk_ids_from_answer(&resolution.answer, &resolution.accepted_risks);
    if accepted_risk_ids.is_empty() {
        return Ok(Vec::new());
    }
    let close_basis = current_close_basis_for_answer(
        context.store,
        context.project_state,
        request,
        context.task,
        context.current_change_unit,
    )?;
    let source_change_unit_id = Some(close_basis.change_unit_id.clone());
    let continuity_context = ProjectContinuityPlanContext {
        service: context.service,
        store: context.store,
        project_id: &request.envelope.project_id,
        source_task_id: &user_judgment.task_id,
        source_change_unit_id: source_change_unit_id.as_ref(),
        planned_state_version: context.planned_state_version,
        now: context.now,
    };
    let mut records = Vec::new();
    for risk in close_basis
        .residual_risks
        .iter()
        .filter(|risk| accepted_risk_ids.contains(&risk.risk_id))
    {
        let source_refs = refs_with_context(
            vec![context.user_judgment_ref.clone()],
            refs_with_context(
                risk.source_refs.clone(),
                request.rationale.related_refs.clone(),
            ),
        );
        let applies_to_refs =
            refs_with_context(close_basis.result_refs.clone(), risk.source_refs.clone());
        let artifact_refs = user_judgment
            .context
            .artifact_refs
            .iter()
            .cloned()
            .chain(request.rationale.artifact_refs.iter().cloned())
            .collect();
        let draft = ProjectContinuityDraft {
            kind: ProjectContinuityKind::AcceptedRisk,
            title: format!(
                "Accepted residual risk: {}",
                short_continuity_title(&risk.summary)
            ),
            summary: risk.summary.clone(),
            rationale: Some(format!(
                "{} Consequence: {}",
                request.rationale.summary, risk.consequence
            )),
            applies_to_paths: bounded_paths_from_change_unit(context.current_change_unit)?,
            applies_to_refs,
            source_refs,
            artifact_refs,
            supersedes_refs: Vec::new(),
            review_triggers: request.rationale.review_triggers.clone(),
            metadata: json!({
                "source": "record_user_judgment",
                "judgment_kind": request.judgment_kind,
                "risk_id": risk.risk_id,
                "close_basis_revision": close_basis.close_basis_revision
            }),
        };
        records.push(
            plan_project_continuity_record(continuity_context, draft).map_err(PlanError::Core)?,
        );
    }
    Ok(records)
}

fn continuity_decision_summary(
    request: &RecordUserJudgmentRequest,
    user_judgment: &UserJudgment,
    resolution: &UserJudgmentResolution,
) -> String {
    let payload_summary = match request.judgment_kind {
        JudgmentKind::ProductDecision => resolution
            .answer
            .product_decision
            .as_ref()
            .and_then(summary_from_json_object),
        JudgmentKind::TechnicalDecision => resolution
            .answer
            .technical_decision
            .as_ref()
            .and_then(summary_from_json_object),
        JudgmentKind::ScopeDecision => resolution
            .answer
            .scope_decision
            .as_ref()
            .and_then(summary_from_json_object),
        _ => None,
    };
    payload_summary
        .or_else(|| nonempty_string(request.rationale.summary.clone()))
        .or_else(|| nonempty_string(user_judgment.context.summary.clone()))
        .unwrap_or_else(|| decision_title_prefix(request.judgment_kind).to_owned())
}

fn summary_from_json_object(object: &JsonObject) -> Option<String> {
    ["summary", "decision", "selected_option", "selected_reason"]
        .into_iter()
        .filter_map(|key| object.get(key).and_then(Value::as_str))
        .find_map(|value| nonempty_string(value.to_owned()))
}

fn nonempty_string(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn short_continuity_title(value: &str) -> String {
    const MAX_CHARS: usize = 96;
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let short = chars.by_ref().take(MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{short}...")
    } else {
        short
    }
}

fn refs_with_context(
    mut refs: Vec<StateRecordRef>,
    additional_refs: Vec<StateRecordRef>,
) -> Vec<StateRecordRef> {
    refs.extend(additional_refs);
    refs
}

fn bounded_paths_from_change_unit(
    current_change_unit: Option<&ChangeUnitRecord>,
) -> Result<Vec<String>, PlanError> {
    current_change_unit
        .map(|record| {
            decode_required_json(
                "change_units",
                record.change_unit_id.clone(),
                "bounded_paths_json",
                Some(&record.bounded_paths_json),
            )
            .map_err(PlanError::Core)
        })
        .transpose()
        .map(Option::unwrap_or_default)
}

fn current_options_for_request(
    judgment_kind: JudgmentKind,
    caller_options: &[UserJudgmentOptionInput],
    locale: Option<&str>,
) -> Vec<UserJudgmentOption> {
    if is_authority_bearing_judgment(judgment_kind) {
        return canonical_authority_options(judgment_kind, locale);
    }

    caller_options
        .iter()
        .map(|option| UserJudgmentOption {
            option_id: option.option_id.clone(),
            label: option.label.clone(),
            description: option.description.clone(),
            consequence: option.consequence.clone(),
            machine_action: UserJudgmentOptionAction::Accept,
            resolution_outcome: JudgmentResolutionOutcome::Accepted,
            is_default: option.is_default,
        })
        .collect()
}

fn canonical_authority_options(
    judgment_kind: JudgmentKind,
    locale: Option<&str>,
) -> Vec<UserJudgmentOption> {
    let mut options = vec![
        canonical_authority_option(
            judgment_kind,
            UserJudgmentOptionAction::Accept,
            locale,
            true,
        ),
        canonical_authority_option(
            judgment_kind,
            UserJudgmentOptionAction::Reject,
            locale,
            false,
        ),
    ];
    if authority_defer_supported(judgment_kind) {
        options.push(canonical_authority_option(
            judgment_kind,
            UserJudgmentOptionAction::Defer,
            locale,
            false,
        ));
    }
    options
}

fn authority_defer_supported(_judgment_kind: JudgmentKind) -> bool {
    true
}

fn canonical_authority_option(
    judgment_kind: JudgmentKind,
    action: UserJudgmentOptionAction,
    locale: Option<&str>,
    is_default: bool,
) -> UserJudgmentOption {
    let template = authority_option_template(judgment_kind, action, locale);
    UserJudgmentOption {
        option_id: UserJudgmentOptionId::new(match action {
            UserJudgmentOptionAction::Accept => "accept",
            UserJudgmentOptionAction::Reject => "reject",
            UserJudgmentOptionAction::Defer => "defer",
        }),
        label: template.label.to_owned(),
        description: template.description.to_owned(),
        consequence: template.consequence.to_owned(),
        machine_action: action,
        resolution_outcome: action.resolution_outcome(),
        is_default,
    }
}

struct AuthorityOptionTemplate {
    label: &'static str,
    description: &'static str,
    consequence: &'static str,
}

fn authority_option_template(
    judgment_kind: JudgmentKind,
    action: UserJudgmentOptionAction,
    locale: Option<&str>,
) -> AuthorityOptionTemplate {
    match authority_option_locale(locale) {
        AuthorityOptionLocale::English => english_authority_option_template(judgment_kind, action),
        AuthorityOptionLocale::Korean => korean_authority_option_template(judgment_kind, action),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthorityOptionLocale {
    English,
    Korean,
}

fn authority_option_locale(locale: Option<&str>) -> AuthorityOptionLocale {
    let Some(locale) = locale.map(str::trim).filter(|locale| !locale.is_empty()) else {
        return AuthorityOptionLocale::English;
    };
    let normalized = locale.to_ascii_lowercase().replace('_', "-");
    if normalized == "ko" || normalized.starts_with("ko-") {
        AuthorityOptionLocale::Korean
    } else {
        AuthorityOptionLocale::English
    }
}

fn english_authority_option_template(
    judgment_kind: JudgmentKind,
    action: UserJudgmentOptionAction,
) -> AuthorityOptionTemplate {
    match (judgment_kind, action) {
        (JudgmentKind::ScopeDecision, UserJudgmentOptionAction::Accept) => AuthorityOptionTemplate {
            label: "Accept scope decision",
            description: "Record the user's accepted scope decision for this exact request.",
            consequence: "The accepted scope decision can be used only for the matching scope update.",
        },
        (JudgmentKind::ScopeDecision, UserJudgmentOptionAction::Reject) => AuthorityOptionTemplate {
            label: "Reject scope decision",
            description: "Record that the user rejected this scope decision.",
            consequence: "The requested scope update remains unauthorized.",
        },
        (JudgmentKind::ScopeDecision, UserJudgmentOptionAction::Defer) => AuthorityOptionTemplate {
            label: "Defer scope decision",
            description: "Record that the user deferred this scope decision.",
            consequence: "The requested scope update remains unauthorized until a current accepted decision exists.",
        },
        (JudgmentKind::SensitiveApproval, UserJudgmentOptionAction::Accept) => AuthorityOptionTemplate {
            label: "Approve sensitive action",
            description: "Record the user's approval for the named sensitive action only.",
            consequence: "The approval can satisfy only the matching sensitive-action requirement.",
        },
        (JudgmentKind::SensitiveApproval, UserJudgmentOptionAction::Reject) => AuthorityOptionTemplate {
            label: "Reject sensitive action",
            description: "Record that the user rejected the named sensitive action.",
            consequence: "The sensitive action remains unauthorized.",
        },
        (JudgmentKind::SensitiveApproval, UserJudgmentOptionAction::Defer) => AuthorityOptionTemplate {
            label: "Defer sensitive action",
            description: "Record that the user deferred the named sensitive action.",
            consequence: "The sensitive action remains unauthorized until a current approval exists.",
        },
        (JudgmentKind::FinalAcceptance, UserJudgmentOptionAction::Accept) => AuthorityOptionTemplate {
            label: "Accept result",
            description: "Record the user's final acceptance for the current close basis.",
            consequence: "Final acceptance can satisfy close only while the captured close basis remains current.",
        },
        (JudgmentKind::FinalAcceptance, UserJudgmentOptionAction::Reject) => AuthorityOptionTemplate {
            label: "Reject result",
            description: "Record that the user rejected final acceptance for the current close basis.",
            consequence: "The Task cannot close as complete from this judgment.",
        },
        (JudgmentKind::FinalAcceptance, UserJudgmentOptionAction::Defer) => AuthorityOptionTemplate {
            label: "Defer result",
            description: "Record that the user deferred final acceptance.",
            consequence: "The Task cannot close as complete until current final acceptance exists.",
        },
        (JudgmentKind::ResidualRiskAcceptance, UserJudgmentOptionAction::Accept) => AuthorityOptionTemplate {
            label: "Accept residual risk",
            description: "Record the user's acceptance of the named residual risks for this close basis.",
            consequence: "Residual-risk acceptance can satisfy close only for the matching current risks.",
        },
        (JudgmentKind::ResidualRiskAcceptance, UserJudgmentOptionAction::Reject) => AuthorityOptionTemplate {
            label: "Reject residual risk",
            description: "Record that the user rejected accepting the named residual risks.",
            consequence: "The Task cannot close with those residual risks accepted.",
        },
        (JudgmentKind::ResidualRiskAcceptance, UserJudgmentOptionAction::Defer) => AuthorityOptionTemplate {
            label: "Defer residual risk",
            description: "Record that the user deferred residual-risk acceptance.",
            consequence: "The Task cannot close with those residual risks until current acceptance exists.",
        },
        (JudgmentKind::Cancellation, UserJudgmentOptionAction::Accept) => AuthorityOptionTemplate {
            label: "Confirm cancellation",
            description: "Record the user's decision to cancel this Task.",
            consequence: "Cancellation can proceed only for the matching current Task and Change Unit.",
        },
        (JudgmentKind::Cancellation, UserJudgmentOptionAction::Reject) => AuthorityOptionTemplate {
            label: "Reject cancellation",
            description: "Record that the user rejected cancelling this Task.",
            consequence: "The Task remains open and cancellation is unauthorized.",
        },
        (JudgmentKind::Cancellation, UserJudgmentOptionAction::Defer) => AuthorityOptionTemplate {
            label: "Defer cancellation",
            description: "Record that the user deferred the cancellation decision.",
            consequence: "The Task remains open until current cancellation authority exists.",
        },
        (JudgmentKind::ProductDecision | JudgmentKind::TechnicalDecision, _) => {
            unreachable!("non-authority judgments do not use authority option templates")
        }
    }
}

fn korean_authority_option_template(
    judgment_kind: JudgmentKind,
    action: UserJudgmentOptionAction,
) -> AuthorityOptionTemplate {
    match (judgment_kind, action) {
        (JudgmentKind::ScopeDecision, UserJudgmentOptionAction::Accept) => {
            AuthorityOptionTemplate {
                label: "범위 결정 수락",
                description: "이 정확한 요청에 대한 사용자의 범위 결정 수락을 기록합니다.",
                consequence: "수락된 범위 결정은 일치하는 범위 업데이트에만 사용할 수 있습니다.",
            }
        }
        (JudgmentKind::ScopeDecision, UserJudgmentOptionAction::Reject) => {
            AuthorityOptionTemplate {
                label: "범위 결정 거부",
                description: "사용자가 이 범위 결정을 거부했음을 기록합니다.",
                consequence: "요청된 범위 업데이트는 계속 권한이 없습니다.",
            }
        }
        (JudgmentKind::ScopeDecision, UserJudgmentOptionAction::Defer) => AuthorityOptionTemplate {
            label: "범위 결정 보류",
            description: "사용자가 이 범위 결정을 보류했음을 기록합니다.",
            consequence: "현재 수락된 결정이 생길 때까지 요청된 범위 업데이트는 권한이 없습니다.",
        },
        (JudgmentKind::SensitiveApproval, UserJudgmentOptionAction::Accept) => {
            AuthorityOptionTemplate {
                label: "민감 작업 승인",
                description: "지정된 민감 작업에 대한 사용자의 승인만 기록합니다.",
                consequence: "이 승인은 일치하는 민감 작업 요구 사항만 충족할 수 있습니다.",
            }
        }
        (JudgmentKind::SensitiveApproval, UserJudgmentOptionAction::Reject) => {
            AuthorityOptionTemplate {
                label: "민감 작업 거부",
                description: "사용자가 지정된 민감 작업을 거부했음을 기록합니다.",
                consequence: "민감 작업은 계속 권한이 없습니다.",
            }
        }
        (JudgmentKind::SensitiveApproval, UserJudgmentOptionAction::Defer) => {
            AuthorityOptionTemplate {
                label: "민감 작업 보류",
                description: "사용자가 지정된 민감 작업을 보류했음을 기록합니다.",
                consequence: "현재 승인이 생길 때까지 민감 작업은 권한이 없습니다.",
            }
        }
        (JudgmentKind::FinalAcceptance, UserJudgmentOptionAction::Accept) => {
            AuthorityOptionTemplate {
                label: "결과 수락",
                description: "현재 닫기 기준에 대한 사용자의 최종 수락을 기록합니다.",
                consequence:
                    "캡처된 닫기 기준이 현재 상태일 때만 최종 수락이 닫기를 충족할 수 있습니다.",
            }
        }
        (JudgmentKind::FinalAcceptance, UserJudgmentOptionAction::Reject) => {
            AuthorityOptionTemplate {
                label: "결과 거부",
                description: "사용자가 현재 닫기 기준에 대한 최종 수락을 거부했음을 기록합니다.",
                consequence: "이 판단으로는 Task를 완료로 닫을 수 없습니다.",
            }
        }
        (JudgmentKind::FinalAcceptance, UserJudgmentOptionAction::Defer) => {
            AuthorityOptionTemplate {
                label: "결과 보류",
                description: "사용자가 최종 수락을 보류했음을 기록합니다.",
                consequence: "현재 최종 수락이 생길 때까지 Task를 완료로 닫을 수 없습니다.",
            }
        }
        (JudgmentKind::ResidualRiskAcceptance, UserJudgmentOptionAction::Accept) => {
            AuthorityOptionTemplate {
                label: "잔여 위험 수락",
                description: "이 닫기 기준에 명시된 잔여 위험에 대한 사용자의 수락을 기록합니다.",
                consequence:
                    "잔여 위험 수락은 일치하는 현재 위험에 대해서만 닫기를 충족할 수 있습니다.",
            }
        }
        (JudgmentKind::ResidualRiskAcceptance, UserJudgmentOptionAction::Reject) => {
            AuthorityOptionTemplate {
                label: "잔여 위험 거부",
                description: "사용자가 명시된 잔여 위험 수락을 거부했음을 기록합니다.",
                consequence: "Task는 해당 잔여 위험을 수락한 상태로 닫힐 수 없습니다.",
            }
        }
        (JudgmentKind::ResidualRiskAcceptance, UserJudgmentOptionAction::Defer) => {
            AuthorityOptionTemplate {
                label: "잔여 위험 보류",
                description: "사용자가 잔여 위험 수락을 보류했음을 기록합니다.",
                consequence:
                    "현재 수락이 생길 때까지 Task는 해당 잔여 위험을 포함해 닫힐 수 없습니다.",
            }
        }
        (JudgmentKind::Cancellation, UserJudgmentOptionAction::Accept) => AuthorityOptionTemplate {
            label: "취소 확정",
            description: "이 Task를 취소하려는 사용자의 결정을 기록합니다.",
            consequence: "취소는 일치하는 현재 Task와 Change Unit에 대해서만 진행될 수 있습니다.",
        },
        (JudgmentKind::Cancellation, UserJudgmentOptionAction::Reject) => AuthorityOptionTemplate {
            label: "취소 거부",
            description: "사용자가 이 Task 취소를 거부했음을 기록합니다.",
            consequence: "Task는 계속 열려 있으며 취소 권한은 없습니다.",
        },
        (JudgmentKind::Cancellation, UserJudgmentOptionAction::Defer) => AuthorityOptionTemplate {
            label: "취소 보류",
            description: "사용자가 취소 결정을 보류했음을 기록합니다.",
            consequence: "현재 취소 권한이 생길 때까지 Task는 계속 열려 있습니다.",
        },
        (JudgmentKind::ProductDecision | JudgmentKind::TechnicalDecision, _) => {
            unreachable!("non-authority judgments do not use authority option templates")
        }
    }
}

struct UserJudgmentRequestValidation<'a> {
    dry_run: bool,
    state_version: Option<u64>,
    judgment_kind: JudgmentKind,
    question: &'a str,
    options: &'a [UserJudgmentOptionInput],
    context: &'a UserJudgmentContext,
    affected_refs: &'a [StateRecordRef],
    required_for: &'a [JudgmentRequiredFor],
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
    if is_authority_bearing_judgment(input.judgment_kind) && !input.options.is_empty() {
        return validation_plan_error(
            input.dry_run,
            input.state_version,
            "options",
            "authority-bearing judgment options must be absent, null, or empty",
        );
    }
    if !is_authority_bearing_judgment(input.judgment_kind) && input.options.is_empty() {
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
        if option.description.trim().is_empty() {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "options.description",
                "option description must not be empty",
            );
        }
        if option.consequence.trim().is_empty() {
            return validation_plan_error(
                input.dry_run,
                input.state_version,
                "options.consequence",
                "option consequence must not be empty",
            );
        }
    }
    if input.required_for.is_empty() {
        return validation_plan_error(
            input.dry_run,
            input.state_version,
            "required_for",
            "required_for must include at least one operation target",
        );
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
    match answer_outcome_agreement(answer, selected_outcome)? {
        AnswerOutcomeAgreement::Agrees => Ok(()),
        AnswerOutcomeAgreement::UnsupportedClaim => validation_plan_error(
            dry_run,
            state_version,
            "answer",
            "answer outcome fields must use a supported judgment resolution_outcome",
        ),
        AnswerOutcomeAgreement::ConflictingClaim => validation_plan_error(
            dry_run,
            state_version,
            "answer",
            "answer outcome fields must agree with the selected option resolution_outcome",
        ),
    }
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

fn local_web_consent_token_rejected_response(
    envelope: &ToolEnvelope,
    state_version: Option<u64>,
    rejection: LocalWebConsentTokenRejection,
) -> PipelineResponse {
    let message = match rejection {
        LocalWebConsentTokenRejection::Invalid => "local web consent token is not valid",
        LocalWebConsentTokenRejection::Expired(_) => "local web consent token is expired",
        LocalWebConsentTokenRejection::Consumed(_) => {
            "local web consent token has already been used"
        }
        LocalWebConsentTokenRejection::WrongProject { .. } => {
            "local web consent token is not bound to this project"
        }
        LocalWebConsentTokenRejection::WrongConnection { .. } => {
            "local web consent token is not bound to this connection"
        }
    };
    decision_rejected_response(envelope, state_version, message)
}
